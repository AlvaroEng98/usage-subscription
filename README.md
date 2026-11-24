# usage-subscription

Herramienta local para monitorear, analizar y optimizar el uso de servicios y suscripciones de IA.

## Estado actual

Primer corte del MVP: aplicación base en Rust con un CLI independiente de Claude Code.

Incluye:

- comando `doctor` para detectar las fuentes locales de Claude Code;
- comando `scan` para leer transcripts JSONL y resumir tokens de uso;
- collector inicial con deduplicación por `message.id` y tolerancia a líneas inválidas;
- ejecución completamente local, sin credenciales ni llamadas de red.

## Ejecutar

```bash
cargo run -- --help
cargo run -- doctor
cargo test
```

## Dirección técnica

La aplicación principal no será un plugin de Claude Code. Claude Code será el primer collector de datos, junto con futuras fuentes como APIs y otros productos de IA.

El diseño previsto es:

```text
collectors -> eventos normalizados -> SQLite -> métricas -> recomendaciones
                                      |
                            CLI / app GTK4 / GNOME extension
```

La interfaz gráfica y la extensión de GNOME se construirán después de validar el núcleo de recolección y el modelo de datos.

## Referencia

`session-lens/` contiene el proyecto de referencia utilizado para estudiar la lectura de transcripts, tokens, costes, agentes y compactaciones de Claude Code. Se mantiene como repositorio independiente.
