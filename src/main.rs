mod config;
mod pty;
mod terminal;
mod renderer;

use config::Config;
use pty::Pty;
use terminal::Terminal;
use renderer::Renderer;

use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent, KeyEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
    keyboard::{PhysicalKey, Key, ModifiersState},
    dpi::LogicalSize,
};

fn main() {
    env_logger::init();
    let config = Config::load();

    let event_loop = EventLoop::new().unwrap();
    
    // Get primary monitor to center the window
    let monitor = event_loop.primary_monitor();
    let initial_width = 1881.0;
    let initial_height = 990.0;
    
    let window_builder = WindowBuilder::new()
        .with_title("Rusterm")
        .with_decorations(config.window_decorations)
        .with_transparent(true)
        .with_inner_size(LogicalSize::new(initial_width, initial_height));

    let window_builder = if let Some(monitor) = monitor {
        let monitor_size = monitor.size();
        let monitor_width = monitor_size.width as f64 / monitor.scale_factor();
        let monitor_height = monitor_size.height as f64 / monitor.scale_factor();
        // Adjust x to reduce 5% from the right (moving it slightly)
        let x = (monitor_width - initial_width) / 2.0;
        let y = (monitor_height - initial_height) / 2.0;
        window_builder.with_position(winit::dpi::LogicalPosition::new(x, y))
    } else {
        window_builder
    };

    let window = Arc::new(window_builder.build(&event_loop).unwrap());

    let mut renderer = pollster::block_on(Renderer::new(window.clone(), &config));

    let initial_size = window.inner_size();
    let font_size = config.font_size;
    let line_height = font_size * 1.2;
    let char_width = font_size * 0.6;
    let padding = 11.0; // ~3mm padding
    
    let initial_rows = ((initial_size.height as f32 - (padding * 2.0)) / line_height) as u16;
    let initial_cols = ((initial_size.width as f32 - (padding * 2.0)) / char_width) as u16;

    let (mut pty, rx) = Pty::new(&config.shell, initial_rows, initial_cols, window.clone());
    let mut terminal = Terminal::new(initial_rows, initial_cols);

    let mut modifiers = ModifiersState::default();
    let mut last_cursor_blink = std::time::Instant::now();
    let mut cursor_visible = true;
    let mut terminal_dirty = true;
    let mut needs_redraw = true;

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::ModifiersChanged(m) => modifiers = m.state(),
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(physical_size.width, physical_size.height);
                    let rows = ((physical_size.height as f32 - (padding * 2.0)) / line_height) as u16;
                    let cols = ((physical_size.width as f32 - (padding * 2.0)) / char_width) as u16;
                    if rows > 0 && cols > 0 {
                        pty.resize(rows, cols);
                        terminal.resize(rows, cols);
                        terminal_dirty = true;
                    }
                }
                WindowEvent::KeyboardInput { event: KeyEvent { physical_key, logical_key, text, state, .. }, .. } => {
                    if state.is_pressed() {
                        let app_cursor = terminal.is_app_cursor_mode();
                        if let Some(ansi) = key_to_ansi(physical_key, logical_key, text, modifiers, app_cursor) {
                            if ansi == "\x04" {
                                elwt.exit();
                                return;
                            }
                            pty.write(ansi.as_bytes());
                            // Marcar redibujado inmediato: al estar en modo Poll, el
                            // siguiente redibujo ocurre sin esperar a que el hilo del
                            // PTY despierte el bucle de eventos.
                            needs_redraw = true;
                        }
                    }
                }
                WindowEvent::RedrawRequested => {
                    let mut blink_changed = false;
                    if last_cursor_blink.elapsed() >= std::time::Duration::from_millis(500) {
                        cursor_visible = !cursor_visible;
                        last_cursor_blink = std::time::Instant::now();
                        blink_changed = true;
                    }
                    
                    let mut data_received = false;
                    while let Ok(data) = rx.try_recv() {
                        terminal.process_input(data);
                        data_received = true;
                    }

                    if data_received || blink_changed || terminal_dirty {
                        renderer.render(&terminal, config.background_opacity, config.rounded_corners, cursor_visible, font_size, padding, &config.font_family);
                        terminal_dirty = false;
                    }
                    needs_redraw = false;
                }
                _ => (),
            },
            Event::AboutToWait => {
                let blink_due = last_cursor_blink.elapsed() >= std::time::Duration::from_millis(500);
                if blink_due || needs_redraw {
                    window.request_redraw();
                    // Mientras haya trabajo pendiente usamos Poll para que los
                    // redibujos se entreguen de inmediato; al quedar inactivos
                    // volvemos a Wait y no consumimos CPU en un bucle ocupado.
                    elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);
                } else {
                    elwt.set_control_flow(winit::event_loop::ControlFlow::Wait);
                }
            }
            _ => (),
        }
    }).unwrap();
}

fn key_to_ansi(_physical_key: PhysicalKey, logical_key: Key, text: Option<smol_str::SmolStr>, modifiers: ModifiersState, app_cursor: bool) -> Option<String> {
    if modifiers.control_key() {
        match logical_key {
            Key::Character(ref s) if s == "c" || s == "C" => return Some("\x03".to_string()),
            Key::Character(ref s) if s == "d" || s == "D" => return Some("\x04".to_string()),
            Key::Character(ref s) if s == "l" || s == "L" => return Some("\x0c".to_string()),
            _ => {}
        }
    }

    match logical_key {
        Key::Named(winit::keyboard::NamedKey::Enter) => Some("\r".to_string()),
        Key::Named(winit::keyboard::NamedKey::Backspace) => Some("\x7f".to_string()),
        Key::Named(winit::keyboard::NamedKey::Tab) => Some("\t".to_string()),
        Key::Named(winit::keyboard::NamedKey::Escape) => Some("\x1b".to_string()),
        
        Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
            if app_cursor { Some("\x1bOA".to_string()) } else { Some("\x1b[A".to_string()) }
        }
        Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
            if app_cursor { Some("\x1bOB".to_string()) } else { Some("\x1b[B".to_string()) }
        }
        Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
            if app_cursor { Some("\x1bOC".to_string()) } else { Some("\x1b[C".to_string()) }
        }
        Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
            if app_cursor { Some("\x1bOD".to_string()) } else { Some("\x1b[D".to_string()) }
        }
        
        Key::Named(winit::keyboard::NamedKey::Home) => Some("\x1b[H".to_string()),
        Key::Named(winit::keyboard::NamedKey::End) => Some("\x1b[F".to_string()),
        Key::Named(winit::keyboard::NamedKey::Insert) => Some("\x1b[2~".to_string()),
        Key::Named(winit::keyboard::NamedKey::Delete) => Some("\x1b[3~".to_string()),
        Key::Named(winit::keyboard::NamedKey::PageUp) => Some("\x1b[5~".to_string()),
        Key::Named(winit::keyboard::NamedKey::PageDown) => Some("\x1b[6~".to_string()),
        
        Key::Character(s) => Some(s.to_string()),
        _ => text.map(|t| t.to_string()),
    }
}
