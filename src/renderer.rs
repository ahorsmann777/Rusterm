use wgpu::*;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::cell::Flags;
use glyphon::{
    FontSystem, SwashCache, TextAtlas, TextRenderer, Resolution,
    Viewport, TextArea, TextBounds, Color, Attrs, Metrics, Family,
    Buffer, Shaping, Cache, Wrap
};
use std::sync::Arc;
use crate::config::Config;
use crate::terminal::Terminal;

/// Mapea un nombre de familia de la configuración a la familia de fontdb.
/// Los nombres genéricos como "monospace" deben resolverse a la familia
/// genérica correspondiente; `Family::Name("monospace")` no coincide con
/// ninguna fuente instalada y cosmic-text cae a una fuente proporcionada,
/// lo que desalinea las columnas de la rejilla.
fn font_family_from_name(name: &str) -> Family<'_> {
    match name {
        "monospace" | "" => Family::Monospace,
        "serif" => Family::Serif,
        "sans-serif" | "sans" => Family::SansSerif,
        "cursive" => Family::Cursive,
        "fantasy" => Family::Fantasy,
        _ => Family::Name(name),
    }
}

pub struct Renderer {
    _instance: Instance,
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    pub font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    render_pipeline: RenderPipeline,
    viewport: Viewport,
    text_rows: Vec<RowCache>,
    font_size: f32,
}

/// Caché de una fila de la rejilla: buffer de texto ya compuesto y una clave
/// que resume su contenido para saber si cambió entre fotogramas.
struct RowCache {
    buffer: Buffer,
    key: Option<Vec<u8>>,
}

impl RowCache {
    fn new(font_system: &mut FontSystem, font_size: f32) -> Self {
        let mut buffer = Buffer::new(font_system, Metrics::new(font_size, font_size * 1.2));
        buffer.set_wrap(font_system, Wrap::None);
        Self { buffer, key: None }
    }
}

impl Renderer {
    async fn init_gpu(
        window: Arc<winit::window::Window>,
    ) -> (Instance, Surface<'static>, Device, Queue, SurfaceConfiguration, TextureFormat) {
        for backends in [Backends::VULKAN, Backends::GL] {
            let instance = Instance::new(InstanceDescriptor {
                backends,
                ..Default::default()
            });
            let surface = match instance.create_surface(window.clone()) {
                Ok(surface) => surface,
                Err(_) => continue,
            };
            let adapter = match instance
                .request_adapter(&RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await
            {
                Some(adapter) => adapter,
                None => continue,
            };
            let (device, queue) = match adapter.request_device(&DeviceDescriptor::default(), None).await {
                Ok(dq) => dq,
                Err(_) => continue,
            };

            let caps = surface.get_capabilities(&adapter);
            let format = caps.formats[0];
            let size = window.inner_size();

            // Immediate evita la espera de vsync/cola de frames (menor latencia);
            // Mailbox elimina el tearing; Fifo es el último recurso.
            let present_mode = if caps.present_modes.contains(&PresentMode::Immediate) {
                PresentMode::Immediate
            } else if caps.present_modes.contains(&PresentMode::Mailbox) {
                PresentMode::Mailbox
            } else {
                PresentMode::Fifo
            };

            let surface_config = SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width,
                height: size.height,
                present_mode,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 1,
            };
            surface.configure(&device, &surface_config);

            return (instance, surface, device, queue, surface_config, format);
        }

        panic!("no se pudo crear una GPU con ningún backend disponible");
    }

    pub async fn new(window: Arc<winit::window::Window>, config: &Config) -> Self {
        let (instance, surface, device, queue, surface_config, format) = Self::init_gpu(window.clone()).await;

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, format);
        let text_renderer = TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);
        let viewport = Viewport::new(&device, &cache);

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Background Shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: surface_config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            _instance: instance,
            device,
            queue,
            surface,
            config: surface_config,
            font_system,
            swash_cache,
            atlas,
            text_renderer,
            render_pipeline,
            viewport,
            text_rows: Vec::new(),
            font_size: config.font_size,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.viewport.update(&self.queue, Resolution { width, height });
        }
    }

    fn update_row(&mut self, row: usize, spans: &[(String, Attrs)], key: &[u8], font_family: Family) {
        let cache = &mut self.text_rows[row];
        if cache.key.as_deref() == Some(key) {
            return;
        }
        cache.buffer.set_rich_text(
            &mut self.font_system,
            spans.iter().map(|(s, a)| (s.as_str(), *a)),
            Attrs::new().family(font_family),
            Shaping::Advanced,
        );
        cache.buffer.shape_until_scroll(&mut self.font_system, false);
        cache.key = Some(key.to_vec());
    }

    pub fn render(&mut self, terminal: &Terminal, _opacity: f32, _rounded: bool, cursor_visible: bool, font_size: f32, padding: f32, font_family_name: &str) {
        let frame = self.surface.get_current_texture().unwrap();
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());

        let grid = terminal.term.grid();
        let font_family = font_family_from_name(font_family_name);
        let rows = grid.screen_lines();
        let line_h = font_size * 1.2;

        // (Re)crear los buffers de fila si cambió el tamaño/métrica.
        if self.font_size != font_size || self.text_rows.len() != rows {
            self.text_rows.clear();
            for _ in 0..rows {
                self.text_rows.push(RowCache::new(&mut self.font_system, font_size));
            }
            self.font_size = font_size;
        }

        let cursor = grid.cursor.point;
        let mut buf = [0u8; 4];

        let mut last_row: Option<usize> = None;
        let mut spans: Vec<(String, Attrs)> = Vec::new();
        let mut key: Vec<u8> = Vec::new();
        let mut run = String::new();
        let mut run_attrs = Attrs::new().family(font_family);
        let mut first = true;

        for cell in grid.display_iter() {
            let row = cell.point.line.0 as usize;
            if let Some(prev) = last_row {
                if row != prev {
                    // Fin de fila: componer la fila anterior y reiniciar acumuladores.
                    if !run.is_empty() {
                        spans.push((run.clone(), run_attrs));
                    }
                    self.update_row(prev, &spans, &key, font_family);
                    spans.clear();
                    key.clear();
                    run.clear();
                    run_attrs = Attrs::new().family(font_family);
                    first = true;
                }
            }
            last_row = Some(row);

            // La celda de continuación de un carácter ancho no debe producir
            // un glifo propio; el emoji/glifo ancho ya avanza ~2 celdas.
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }

            let fg = match cell.fg {
                alacritty_terminal::vte::ansi::Color::Named(n) => match n {
                    alacritty_terminal::vte::ansi::NamedColor::Black => Color::rgb(40, 44, 52),
                    alacritty_terminal::vte::ansi::NamedColor::Red => Color::rgb(224, 108, 117),
                    alacritty_terminal::vte::ansi::NamedColor::Green => Color::rgb(152, 195, 121),
                    alacritty_terminal::vte::ansi::NamedColor::Yellow => Color::rgb(229, 192, 123),
                    alacritty_terminal::vte::ansi::NamedColor::Blue => Color::rgb(97, 175, 239),
                    alacritty_terminal::vte::ansi::NamedColor::Magenta => Color::rgb(198, 120, 221),
                    alacritty_terminal::vte::ansi::NamedColor::Cyan => Color::rgb(86, 182, 194),
                    alacritty_terminal::vte::ansi::NamedColor::White => Color::rgb(171, 178, 191),
                    alacritty_terminal::vte::ansi::NamedColor::BrightBlack => Color::rgb(92, 99, 112),
                    alacritty_terminal::vte::ansi::NamedColor::BrightRed => Color::rgb(224, 108, 117),
                    alacritty_terminal::vte::ansi::NamedColor::BrightGreen => Color::rgb(152, 195, 121),
                    alacritty_terminal::vte::ansi::NamedColor::BrightYellow => Color::rgb(229, 192, 123),
                    alacritty_terminal::vte::ansi::NamedColor::BrightBlue => Color::rgb(97, 175, 239),
                    alacritty_terminal::vte::ansi::NamedColor::BrightMagenta => Color::rgb(198, 120, 221),
                    alacritty_terminal::vte::ansi::NamedColor::BrightCyan => Color::rgb(86, 182, 194),
                    alacritty_terminal::vte::ansi::NamedColor::BrightWhite => Color::rgb(220, 223, 228),
                    _ => Color::rgb(255, 255, 255),
                },
                alacritty_terminal::vte::ansi::Color::Spec(rgb) => Color::rgb(rgb.r, rgb.g, rgb.b),
                alacritty_terminal::vte::ansi::Color::Indexed(idx) => {
                    if idx < 16 {
                        match idx {
                            0 => Color::rgb(40, 44, 52),
                            1 => Color::rgb(224, 108, 117),
                            2 => Color::rgb(152, 195, 121),
                            3 => Color::rgb(229, 192, 123),
                            4 => Color::rgb(97, 175, 239),
                            5 => Color::rgb(198, 120, 221),
                            6 => Color::rgb(86, 182, 194),
                            7 => Color::rgb(171, 178, 191),
                            8 => Color::rgb(92, 99, 112),
                            9 => Color::rgb(224, 108, 117),
                            10 => Color::rgb(152, 195, 121),
                            11 => Color::rgb(229, 192, 123),
                            12 => Color::rgb(97, 175, 239),
                            13 => Color::rgb(198, 120, 221),
                            14 => Color::rgb(86, 182, 194),
                            15 => Color::rgb(220, 223, 228),
                            _ => Color::rgb(255, 255, 255),
                        }
                    } else if idx < 232 {
                        let idx = idx - 16;
                        let r = (idx / 36) * 51;
                        let g = ((idx % 36) / 6) * 51;
                        let b = (idx % 6) * 51;
                        Color::rgb(r, g, b)
                    } else {
                        let gray = (idx - 232) * 10 + 8;
                        Color::rgb(gray, gray, gray)
                    }
                }
            };

            let mut attrs = Attrs::new().family(font_family).color(fg);
            let on_cursor = cursor_visible && cell.point.line == cursor.line && cell.point.column == cursor.column;
            let c = if on_cursor && !cell.flags.contains(Flags::WIDE_CHAR) {
                attrs = attrs.color(Color::rgb(255, 255, 255));
                '█'
            } else {
                if on_cursor {
                    attrs = attrs.color(Color::rgb(255, 255, 255));
                }
                cell.c
            };

            // Clave compacta (bytes del carácter + color) para detectar filas cambiadas.
            key.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            key.push(fg.r() as u8);
            key.push(fg.g() as u8);
            key.push(fg.b() as u8);

            if first {
                run_attrs = attrs;
                run.push(c);
                first = false;
            } else if attrs == run_attrs {
                run.push(c);
            } else {
                spans.push((run.clone(), run_attrs));
                run.clear();
                run.push(c);
                run_attrs = attrs;
            }
        }

        // Componer la última fila visible.
        if let Some(row) = last_row {
            if !run.is_empty() {
                spans.push((run.clone(), run_attrs));
            }
            self.update_row(row, &spans, &key, font_family);
        }

        // Un TextArea por fila, posicionado en su coordenada exacta de la rejilla.
        let mut areas: Vec<TextArea> = Vec::with_capacity(rows);
        for (i, cache) in self.text_rows.iter().enumerate() {
            let top = padding + i as f32 * line_h;
            areas.push(TextArea {
                buffer: &cache.buffer,
                left: padding,
                top,
                scale: 1.0,
                bounds: TextBounds {
                    left: padding as i32,
                    top: top as i32,
                    right: (self.config.width as f32 - padding) as i32,
                    bottom: (top + line_h) as i32,
                },
                default_color: Color::rgb(255, 255, 255),
                custom_glyphs: &[],
            });
        }

        self.text_renderer.prepare(
            &self.device,
            &self.queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash_cache,
        ).unwrap();

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.render_pipeline);
            pass.draw(0..6, 0..1);

            self.text_renderer.render(&self.atlas, &self.viewport, &mut pass).unwrap();
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
