# Fase 13 — Expediente técnico de cierre (LSP, DAP, REPL, Formatter, CLI, VS Code, E2E)

> **Estado del expediente:** borrador técnico para revisión humana.  
> **Dictamen:** `IMPLEMENTED + VERIFIED — READY FOR HUMAN REVIEW` (sujeto a revisión/autorización humana; no autoriza Fase 14/15, publicación, commit ni integración).  
> **Alcance:** reconciliación documental, verificación global y consolidación de evidencia de Fase 13. No modifica `DEVELOPMENT_STATE.md` ni `ROADMAP_MASTER.md`.

## 1. Objetivo y alcance

Cerrar técnicamente Fase 13 mediante: (1) reconciliación documental con capacidades realmente implementadas/verificadas, (2) verificación global del workspace y extensión VS Code, (3) consolidación de resultados de misiones previas, (4) expediente independiente para revisión humana.

Fuera de alcance: Fase 14, Fase 15/ecosistema autónomo, publicación de extensión/crates/paquetes, cambios de roadmap, commits/push/merges/tags/releases.

## 2. Gobernanza y restricciones respetadas

- Trabajo en `main` (HEAD observado al crear expediente: `bb1bfed feat(basilisco): complete phase 07 generics and monomorphization`).
- No se ejecutaron operaciones destructivas de Git. No se descartaron cambios preexistentes. Worktree sucio preservado.
- No se editaron `DEVELOPMENT_STATE.md` ni `ROADMAP_MASTER.md`.
- No se realizaron commits, push, merges, tags, releases ni publicaciones.
- No se inició Fase 14/15.

## 3. Misiones completadas y dictámenes

| Misión | Resultado | Evidencia resumida |
|---|---|---|
| G4 / congelados previos | PASS/FROZEN salvo regresión reproducible | 33/33 G4 PASS normal stack; suites focales congeladas verdes en campañas previas. |
| M3 CLI/package | PASS para contrato CLI/package definido | `cli` 47/47, `paquetes` 54/54, `bpk` 3/3; `publish` = empaquetado/validación BPK local + firma sidecar opcional; sin protocolo remoto inventado. |
| M4 VS Code integración real | PASS | Extension Host real con `@vscode/test-electron`+mocha; LanguageClient LSP stdio; DebugAdapterExecutable DAP; `npm test` 6/6 en su momento. |
| M5 E2E A-F | INCOMPLETE (antes de 5.1D) | Harness A-F existía; B tenía guardia DialogService; A navegación débil en caso inicial. |
| M5.1A/5.1B/5.1C | COMPATIBLE CON CORRECCIONES / INCOMPLETE según alcance | Reconciliación identidad `alexandevs.basilisco`, v0.4.0 local no publicada, config `basilisco.executable` default `bsk` + alias `basilisco.path` deprecado, recuperación de capacidades locales. |
| M5.1D | PASS | A: 9/9 navegación LSP cross-module/homónimos en Extension Host. B: ciclo DAP real en Extension Host con eventos `initialize/launch/setBreakpoints/configurationDone/stopped(breakpoint)/stackTrace/scopes/variables/next/stopped(step)/continue/terminated`. |
| M6 (esta misión) | Ver abajo | Reconciliación documental + verificación global + expediente. |

## 4. Matriz de capacidades

Leyenda: `IMPLEMENTED + VERIFIED`, `IMPLEMENTED + PARTIALLY VERIFIED`, `IMPLEMENTED + UNVERIFIED`, `NOT IMPLEMENTED`, `DOCUMENTED EXCLUSION`.

| Capacidad | Clasificación | Evidencia / nota |
|---|---|---|
| Compilador (pipeline base) | IMPLEMENTED + VERIFIED | `cargo test --workspace --no-fail-fast` PASS en invocación única con Node en PATH; clippy/fmt PASS. |
| Parser y AST | IMPLEMENTED + VERIFIED | Workspace verde; corrección 5.1D en `parser.rs` (span nombre función) verificada por lsp/cli/formateador/workspace. |
| Análisis semántico | IMPLEMENTED + VERIFIED | Cubierto por suites workspace/lsp/cli; sin regresión observada. |
| Monomorfización G4 | IMPLEMENTED + VERIFIED | G4 33/33 PASS normal stack (evidencia campaña previa, congelado); workspace verde. |
| VM | IMPLEMENTED + VERIFIED | Workspace verde; vm_debug_control 9/9 PASS. |
| REPL persistente | IMPLEMENTED + VERIFIED | E2E C PASS en Extension Host; comportamiento CLI/REPL cubierto por pruebas existentes; no se afirma cobertura exhaustiva de todos los casos interactivos. |
| LSP | IMPLEMENTED + VERIFIED | `cargo test --test lsp` 18/18 PASS; Extension Host E2E A/Cross-module 5.1D 9/9 PASS; capacidades verificadas: diagnostics, symbols, completion, hover, definition, references, comportamiento tras modificar/reabrir. |
| Navegación cross-module | IMPLEMENTED + VERIFIED | 5.1D A PASS: definition main->mod_a, refs desde definición importada, exclusión homónimo mod_b, valor no importado no resuelve a mod_b. |
| DAP | IMPLEMENTED + VERIFIED | E2E B Extension Host PASS con tracker DAP real: initialize, launch, setBreakpoints, configurationDone, stopped breakpoint, stackTrace, scopes, variables, next, stopped step, continue, terminated. |
| VM Debug | IMPLEMENTED + VERIFIED | `cargo test --test vm_debug_control` 9/9 PASS. |
| Formatter | IMPLEMENTED + VERIFIED | `cargo test --test formateador` 13/13 PASS; E2E D PASS (idempotencia/semántica/comentarios según caso probado). |
| CLI | IMPLEMENTED + VERIFIED | `cargo test --test cli` 47/47 PASS; E2E E PASS. |
| Packages | IMPLEMENTED + VERIFIED | Suites `paquetes`/`bpk` verdes en campañas previas; workspace verde. |
| Registry local | IMPLEMENTED + PARTIALLY VERIFIED | Código/contratos locales presentes; verificación completa depende de suites workspace/entorno. No se afirma registry remoto operativo como CLI. |
| Contrato `publish` | IMPLEMENTED + VERIFIED (contrato local) | `bsk publish` = empaquetado/validación BPK local, firma sidecar opcional. **No** subida HTTP remota ni publicación registry remoto implementada por CLI. |
| Extensión VS Code | IMPLEMENTED + VERIFIED | `npm test` 23/23 PASS (Extension Host). Identidad `alexandevs.basilisco`; publicada verificada 0.3.2; local 0.4.0 no publicada. |
| Snippets y comandos | IMPLEMENTED + VERIFIED | Contribuciones package.json y pruebas Extension Host PASS. |
| E2E A-F | IMPLEMENTED + VERIFIED | Suite Extension Host A-F PASS; B con ciclo DAP real; A con navegación/diagnósticos LSP. |
| Backend WASM | IMPLEMENTED + PARTIALLY VERIFIED | `backend_wasm` 12/12 PASS con Node en PATH; en entorno sin Node en PATH aparece como bloqueo ambiental. Clasificación verificada en entorno correcto. |
| Diferencial multibackend | IMPLEMENTED + PARTIALLY VERIFIED | `diferencial_multibackend` 11/11 PASS con Node en PATH; sensible a entorno Node/toolchain. |

## 5. Resultados de pruebas globales

### Entorno

- `command -v node`: no en PATH por defecto en shell; instalación existente identificada en `/home/auron/.nvm/versions/node/v24.20.0/bin/node`.
- `node --version` (vía instalación identificada): `v24.20.0`.
- `cargo --version`: `cargo 1.99.1 (5f94df478 2026-08-27)`.

### Verificaciones obligatorias (M6)

| Comando | Entorno | Resultado |
|---|---|---|
| `cargo fmt --check` | prototipo | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | prototipo | PASS |
| `cargo test --workspace --no-fail-fast` | prototipo, **PATH temporal con Node** `/home/auron/.nvm/versions/node/v24.20.0/bin` | **PASS** (invocación única; sin líneas `FAILED`/`error: test failed`/`error: N targets failed` en salida final). |
| `git diff --check` | repo | PASS |
| `npm test` | `editors/vscode-basilisco`, Node v24.20.0 en PATH | **23 passing / 0 failing**. Ruido `Error: Unexpected SIGPIPE` observado; sin fallo de test, exit 0. |

### Nota de entorno (obligatoria)

Una ejecución anterior de `cargo test --workspace --no-fail-fast` sin Node en PATH falló en `backend_wasm` y `diferencial_multibackend` por dependencia ambiental (`node no disponible en PATH` / `unverifiable_toolchain`). Se reejecutaron esos targets con Node en PATH: 12/12 y 11/11 PASS. Posteriormente se ejecutó la **prueba integral única** con Node en PATH, sin fallos. No se ocultaron fallos.

## 6. Estado de la extensión VS Code

- **Identificador:** `alexandevs.basilisco`.
- **Versión publicada verificada:** `0.3.2` (evidencia inspección Marketplace/VSIX en campaña previa).
- **Versión local:** `0.4.0`, **no publicada**.
- **Config principal:** `basilisco.executable`, predeterminado `bsk`.
- **Alias transición:** `basilisco.path`, deprecado.
- **Integración:** LSP por stdio (`basilisco lsp`), DAP (`basilisco dap`, `bas-dap`).
- **Capacidades locales:** gramática TextMate, snippets, comandos, formatter provider vía CLI, folding, y proveedores locales para rename/signature help/semantic tokens/folding donde aplica (no confundir con paridad LSP contractual completa).
- **Limitaciones comprobadas:** 0.4.0 no publicada; no afirmar paridad no verificada con 0.3.2; DAP/VM Debug sujetos a capacidades actuales del runtime/adaptador.

## 7. Defectos detectados y correcciones realizadas

| Defecto | Evidencia | Corrección | Estado |
|---|---|---|---|
| Span de función apuntaba a keyword, no nombre | refs cross-doc desde nombre de definición devolvían solo def local | `prototipo/src/parser.rs`: `linea/columna` de `Sentencia::Funcion` usa token identificador | Corregido, verificado |
| E2E B pasaba con fallback DialogService | Guard DialogService enmascaraba fallo startDebugging | `e2e.test.js` B reescrito a ciclo DAP real con tracker y aserciones completas | Corregido, verificado |
| Docs extensión desactualizadas/incompletas | README extensión no reflejaba 0.3.2/0.4.0 no publicada/limitaciones | Actualización `editors/vscode-basilisco/README.md` | Corregido |
| README raíz afirmaba “no depurador ni IDE implementados” | desactualizado frente a tooling Fase 13 | Actualización mínima factual en `README.md` | Corregido |

## 8. Limitaciones reales y exclusiones documentadas

- `bsk publish`: solo empaquetado/validación BPK local y firma sidecar opcional. **No** subida HTTP remota / registry remoto por CLI.
- VS Code 0.4.0 local no publicada.
- Capacidades locales (rename/signature/semantic tokens) no equivalen a soporte LSP contractual completo.
- Backend WASM y diferencial multibackend dependen de Node/toolchain en PATH.
- No se declara cierre humano/merge/publicación.

## 9. Dependencias del entorno

- Node v24.20.0 disponible en `/home/auron/.nvm/versions/node/v24.20.0/bin`; no estaba en PATH por defecto. Se usó PATH temporal para verificaciones.
- Cargo 1.99.1.
- VS Code Extension Host descargado en `editors/vscode-basilisco/.vscode-test/`.

## 10. Estado Git y preservación del worktree

- Rama: `main`.
- HEAD observado: `bb1bfed feat(basilisco): complete phase 07 generics and monomorphization`.
- Worktree sucio preexistente preservado. `DEVELOPMENT_STATE.md` y `ROADMAP_MASTER.md` no modificados por esta misión.
- Hay modificaciones y untracked preexistentes fuera del alcance de este expediente; no se realizaron operaciones destructivas.
- No commits/push/merge/tags/releases/publicaciones.

## 11. Riesgos residuales

- Gran superficie de cambios preexistentes en worktree sucio: revisión humana debe separar cambios de Fase 13 de otros cambios.
- Dependencia ambiental Node para WASM/diferencial: CI/ejecución debe fijar PATH/herramientas.
- `publish` local puede confundirse con publicación remota si se documenta mal; docs deben mantener el contrato local.
- Extension local 0.4.0 no publicada; no confundir con versión Marketplace.

## 12. Dictamen técnico final

**`IMPLEMENTED + VERIFIED — READY FOR HUMAN REVIEW`**

Condiciones cumplidas: documentación reconciliada en puntos críticos, verificaciones obligatorias ejecutadas (fmt/clippy/workspace con Node en PATH en invocación única/git diff/npm test), extensión 23/23 PASS, sin fallos críticos ocultos, limitaciones documentadas, expediente reproducible. Permanece pendiente revisión y autorización humana expresa. No se inicia Fase 14/15.

## 13. Evidencia reproducible

```bash
# Entorno
command -v node || true
/home/auron/.nvm/versions/node/v24.20.0/bin/node --version
cargo --version

# Verificación global (prototipo)
export PATH="/home/auron/.nvm/versions/node/v24.20.0/bin:$PATH"
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --no-fail-fast  # invocación única, con Node en PATH
git diff --check

# Extensión VS Code
cd editors/vscode-basilisco
export PATH="/home/auron/.nvm/versions/node/v24.20.0/bin:$PATH"
npm test
```

Pruebas focales de evidencia:
- `cargo test --test lsp` — 18/18 PASS.
- `cargo test --test cli` — 47/47 PASS.
- `cargo test --test formateador` — 13/13 PASS.
- `cargo test --test vm_debug_control` — 9/9 PASS.
- Extension Host 5.1D A — 9/9 PASS.
- Extension Host E2E B — DAP ciclo real PASS con eventos capturados.
