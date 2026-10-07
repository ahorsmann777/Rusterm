# Contribuir a Rusterm

¡Gracias por tu interés en mejorar Rusterm! Este documento explica cómo participar.

## Cómo empezar

1. Haz un fork y clona el repositorio:

   ```bash
   git clone https://github.com/ahorsmann777/rusterm.git
   cd rusterm
   ```

2. Crea una rama para tu cambio:

   ```bash
   git checkout -b feat/mi-cambio
   ```

3. Compila y prueba:

   ```bash
   cargo build
   cargo run --release
   ```

## Flujo de trabajo

- Abre una **issue** primero para cambios grandes (nuevas funcionalidades, cambios de arquitectura) y comenta tu intención antes de invertir tiempo.
- Los cambios pequeños (correcciones, documentación) pueden ir directo en un **pull request**.
- Un PR debe tocar un solo tema; si tienes varios cambios, abre varios PRs.
- Asegúrate de que todos los commits pasan las verificaciones antes de pedir la revisión.

## Verificaciones obligatorias

Antes de enviar tu PR ejecuta:

```bash
cargo fmt --all -- --check   # formato de código
cargo clippy --all-targets   # lints
cargo build                  # que compile sin errores
```

Si `cargo fmt` o `cargo clippy` reportan problemas, corrígelos:

```bash
cargo fmt --all
cargo clippy --all-targets --fix
```

## Estilo de código

- Sigue el estilo por defecto de `rustfmt`.
- Nombres en inglés para identificadores y APIs; los comentarios en el código pueden ir en español o inglés, pero sé consistente dentro de un módulo.
- Documenta los módulos y funciones públicas con `///` cuando la intención no sea obvia.
- Sin `unwrap()` en rutas que puedan fallar por entrada del usuario o del sistema; maneja el error o justifica el pánico con un mensaje claro.
- Evita dependencias nuevas sin justificación; discútela en la issue/PR.

## Guía para issues

- **Bugs:** incluye pasos para reproducir, resultado esperado, resultado real, versión de Rust (`rustc --version`), distribución y backend gráfico (Vulkan/OpenGL, X11/Wayland). Logs con `RUST_LOG=debug` ayudan mucho.
- **Funcionalidad:** describe el problema que resuelve, no solo la solución propuesta.
- Usa etiquetas claras: `bug`, `enhancement`, `documentation`, `good first issue`.

## Commits

- Mensajes cortos e imperativos: `fix: alinear columnas con fuentes monoespaciadas`.
- Prefijo de tipo al inicio: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `chore:`.

## Licencia

Al contribuir, aceptas que tus aportaciones se distribuyan bajo la [licencia MIT](LICENSE) del proyecto.
