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

## Primer vertical slice

El comando `doctor` valida la disponibilidad de las rutas locales que utilizará el primer collector:

- `~/.claude/`
- `~/.claude/projects/`
- `~/.claude/sessions/`

El siguiente slice debe implementar la lectura incremental de transcripts y convertirlos a eventos normalizados, sin introducir todavía una base de datos o una UI.
