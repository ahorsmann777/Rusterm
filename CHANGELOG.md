# Changelog

Todos los cambios notables en este proyecto se documentan en este archivo.

El formato se basa en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/),
y este proyecto adhiere al [SemVer](https://semver.org/lang/es/).

## [0.1.0] - Unreleased

Primera versión pública. Emulador de terminal renderizado por GPU.

### Añadido

- Ventana propia con `winit` 0.29: centrado en el monitor principal, modo borderless y fondo transparente.
- Renderizado GPU con `wgpu` 22.1 (Vulkan con fallback a OpenGL) y presentación `Immediate`/`Mailbox` de baja latencia.
- Shader WGSL propio (`src/shader.wgsl`) para el fondo: transparencia y esquinas redondeadas.
- Renderizado de texto con `glyphon` 0.6, incluido soporte de glifos anchos/emoji y familias de fuentes genéricas (`monospace`, `serif`, etc.).
- Paleta de colores ANSI de 16/256/truecolor (estilo Nord) con soporte de colores 24-bit y colores indexados.
- Caché de composición por fila: solo se recomponen las filas cuya clave de contenido cambió.
- PTY nativa con `portable-pty`, hilo lector con buffer de 64 KB y canal `crossbeam-channel`.
- Integración con `alacritty_terminal` 0.26: parser VT/ANSI, rejilla y modo cursor de aplicación (`CSI`/`SS3`).
- Entrada de teclado mapeada a secuencias ANSI (Enter, Backspace, Tab, Escape, flechas, Home/End, Insert/Delete, PageUp/PageDown, `Ctrl+C`/`Ctrl+D`/`Ctrl+L`).
- Redimensionamiento dinámico de la ventana con resize coordinado de PTY y rejilla.
- Parpadeo de cursor cada 500 ms con bucle `Wait`/`Poll` que evita el consumo de CPU en reposo.
- Configuración TOML (`rusterm.conf`) autogenerada con valores por defecto vía `serde` + `toml`.
- Logs con `env_logger` (`RUST_LOG=info|debug`).

### Conocido

- `background_opacity`, `rounded_corners`, `cursor_blink` y `scrollback_lines` se declaran en la configuración pero aún no alteran el comportamiento (valores fijos en el shader / defaults de `alacritty_terminal`).
- Sin scroll con ratón, selección de texto ni copiado al portapapeles.
- Sin pestañas, splits ni búsqueda.
- Dirigido a Linux (X11/Wayland).

[0.1.0]: https://github.com/ahorsmann777/rusterm/releases/tag/v0.1.0
