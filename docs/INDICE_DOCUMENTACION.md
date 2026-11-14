# Índice de documentación local (BASILISCO)

> **Propósito:** mapa de navegación para reducir la fragmentación documental. No reemplaza ni invalida los documentos existentes; indica **dónde está cada cosa** y cuál es la **fuente de verdad** según el tema.
>
> **Regla práctica:** cuando documentación y código difieran, **el código y las validaciones reproducibles** describen el comportamiento real; las especificaciones/ADRs/planes expresan intención, contrato o decisión (según su estado). Verifica siempre el estado (`APROBADO/Propuesto/Pendiente/Exploratorio`) y la evidencia asociada.

## 1) Fuentes de verdad rápidas

| Tema | Fuente principal | Complemento / evidencia |
|---|---|---|
| Identidad, estado declarado, uso básico, limitaciones actuales | `README.md` | `prototipo/Cargo.toml` (versión de manifiesto), `prototipo/README.md` (si aplica) |
| Estado de desarrollo gobernado, fase autorizada, bloqueos, validaciones | `DEVELOPMENT_STATE.md` | `ROADMAP_MASTER.md`, `docs/reports/*` |
| Roadmap y criterios por fase (00–14) | `ROADMAP_MASTER.md` | `08-plan/*`, `DEVELOPMENT_STATE.md` |
| Gobernanza y decisiones (ADRs/estado/decisiones) | `00-gobernanza/*`, `docs/governance/*` | `AGENTS.md`, `.opencode/skills/basilisco-governance/SKILL.md` |
| Especificación del lenguaje y contratos normativos/propuestos | `10-especificacion/*`, `02-lenguaje/*` | `07-validacion/*`, `09-glosario/glosario.md` |
| Arquitectura, modelos de ejecución, plataformas | `03-arquitectura/*`, `04-plataformas/*`, `docs/architecture/*`, `docs/ARQUITECTURA-MAESTRA-BASILISCO.md` | Código en `prototipo/src/*` |
| Calidad, seguridad, pruebas y puertas | `05-calidad-seguridad/*`, `07-validacion/*` | Resultados de `cargo test/clippy/fmt` y reportes en `docs/reports/*` |
| Herramientas, CLI, paquetes, interoperabilidad | `06-herramientas-ecosistema/*`, `docs/PROJECT_SYSTEM.md`, `docs/stdlib.md`, `10-especificacion/*` (paquetes/FFI/CLI) | Implementación en `prototipo/src/*` (`cli.rs`, `paquetes.rs`, `ffi.rs`, etc.) |
| Visión, alcance, usuarios, plan | `01-vision/*`, `08-plan/*`, `docs/VISION.md` | — |
| Comportamiento real implementado | **`prototipo/src/**`** y **`prototipo/tests/**`** | Ejemplos ejecutables `prototipo/examples/*.bsk`, tutorial `prototipo/tutorial/*` |

## 2) Mapa por carpetas

- **`/` (raíz):** `README.md`, `DEVELOPMENT_STATE.md`, `ROADMAP_MASTER.md`, `AGENTS.md`, `LICENSE`, manifiestos/configuración.
- **`00-gobernanza/` y `docs/governance/`:** estado del proyecto, ADRs, registro/plantillas de decisiones, procesos.
- **`01-vision/` a `09-glosario/`:** visión/requisitos, lenguaje, arquitectura, plataformas, calidad/seguridad, herramientas/ecosistema, validación, plan y glosario (documentación estructurada por dominio).
- **`10-especificacion/`:** especificaciones, perfiles, gramática, catálogos, auditorías de coherencia, brechas y contratos técnicos (normativos o propuestos según documento).
- **`docs/`:** documentación operativa/técnica transversal (arquitectura, convenciones, stdlib, sistema de proyectos/templates, tutorial, migración VM, reportes en `docs/reports/`).
- **`prototipo/`:** implementación Rust (código, tests, ejemplos, tutorial). **Referencia definitiva del comportamiento observable.**
- **`ejemplos/`:** ejemplos conceptuales (no necesariamente ejecutables en el prototipo actual).
- **`editors/`, `basilisco-vscode/`, `.vscode/`:** soporte/editor y tooling asociado.

## 3) Lecturas recomendadas (según objetivo)

- **Entender qué es y qué estado tiene hoy:** `README.md` → `DEVELOPMENT_STATE.md` → `ROADMAP_MASTER.md`.
- **Saber qué está garantizado vs experimental:** `10-especificacion/perfil-de-conformidad-v0.4.md`, `10-especificacion/registro-de-brechas.md`, `README.md` (limitaciones), `DEVELOPMENT_STATE.md`.
- **Consultar lenguaje/contratos:** `02-lenguaje/*`, `10-especificacion/*` (núcleo, gramática, tipos, errores, FFI, paquetes) contrastado con `prototipo/src` y `prototipo/tests`.
- **Contribuir/cambiar con gobernanza:** `AGENTS.md`, `00-gobernanza/*`, `docs/governance/*`, `.opencode/skills/basilisco-governance/SKILL.md`.
- **Ver evidencia de validación:** `docs/reports/*`, `07-validacion/*`, comandos y resultados registrados en `DEVELOPMENT_STATE.md`.

## 4) Nota sobre fragmentación

La documentación está distribuida por dominios y etapas (gobernanza, especificación, plan, validación, reportes, código). Este índice no unifica el contenido para no perder trazabilidad; sirve como punto de entrada único. Si se detecta contradicción:
1. Comprobar la **fecha/corte** y el **estado** del documento.
2. Contrastar con **código, tests y validaciones** recientes.
3. Actualizar el documento afectado con evidencia (sin inventar aprobaciones ni estados).
