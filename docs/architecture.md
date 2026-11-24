# Arquitectura inicial

## Objetivo del MVP

Construir una herramienta de escritorio local que mida el uso real de servicios de IA y lo relacione con suscripciones, límites y coste efectivo.

## Capas

```text
CLI / GTK4 / GNOME extension
            |
      aplicación local
            |
  métricas y recomendaciones
            |
     ledger de uso SQLite
            |
   eventos de uso normalizados
            |
       collectors
```

## Decisiones iniciales

- Rust para el núcleo y las futuras aplicaciones.
- SQLite como almacenamiento local del histórico.
- Claude Code es una fuente de datos, no el host de la aplicación.
- Primero se valida el núcleo mediante CLI; la UI no se construye antes de tener datos confiables.
- El procesamiento es local-first: no se requieren credenciales ni red para analizar transcripts locales.

## Primer vertical slice completado

El comando `doctor` valida la disponibilidad de las rutas locales que utilizará el primer collector:

- `~/.claude/`
- `~/.claude/projects/`
- `~/.claude/sessions/`

El comando `scan` lee los transcripts JSONL de Claude Code línea por línea y convierte los registros `assistant` con `message.usage` en eventos normalizados. Deduplica mensajes repetidos por `message.id`, tolera líneas JSON inválidas y agrega tokens de entrada, salida y caché.

```bash
cargo run -- scan
cargo run -- scan /ruta/a/.claude/projects
```

Todavía no persiste datos en SQLite ni estima precios; esos son los siguientes límites deliberados del MVP.
