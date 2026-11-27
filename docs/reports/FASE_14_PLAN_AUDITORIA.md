# FASE 14.0 — Inventario, línea base y plan de auditoría integral

> **Estado:** PLAN / INVENTARIO (sin correcciones ni implementaciones).  
> **Fecha de elaboración:** 2026-11-27 (evidencia local).  
> **Rama:** `main` | **HEAD:** `bb1bfed` (`feat(basilisco): complete phase 07 generics and monomorphization`)  
> **Worktree:** sucio (73 entradas en `git status --short` al inicio); **preservado íntegramente**.  
> **Fase 13:** `APROBADA / CLOSED` — expediente `docs/reports/FASE_13_EXPEDIENTE_CIERRE.md`.

## 1. Alcance

Preparar la ejecución de Fase 14 (auditoría integral y estabilización final) sin modificar código ni pruebas. Este documento define inventario, línea base, riesgos, orden de auditoría, misiones 14.1–14.9, criterios de aceptación, exclusiones, comandos y condiciones de cierre.

**Fuera de alcance en 14.0:** correcciones, refactors, nuevas funcionalidades, cambios de tests, commits/push/merge/tags/releases/publicaciones, modificación de `DEVELOPMENT_STATE.md` o `ROADMAP_MASTER.md`.

## 2. Inventario real del workspace

### 2.1 Raíz / estructura relevante

- `prototipo/` — crate Rust `basilisco` v0.7.0 (edition 2024), binarios `basilisco`, `bsk`, `basilisco-registry`.
- `editors/vscode-basilisco/` — extensión VS Code `alexandevs.basilisco` (local 0.4.0 no publicada; publicada verificada 0.3.2).
- `docs/` — documentación, gobernanza, reportes, semántica, stdlib.
- `10-especificacion/`, `02-lenguaje/`, `00-gobernanza/`, `08-plan/` — especificaciones/gobernanza/planificación (con modificaciones preexistentes en worktree).
- `.opencode/`, `.agents/` — agentes/skills/comandos.

### 2.2 Módulos fuente (`prototipo/src`)

| Área | Módulos | Rol observado |
|---|---|---|
| Frontend | `lexer.rs`, `token.rs`, `parser.rs`, `diagnostico.rs` | Tokenización, parsing/AST, diagnósticos. |
| Semántica/tipos/símbolos | `semantico.rs`, `simbodo.rs`, `valor.rs` | Análisis semántico, tipos, ámbitos/símbolos, valores. |
| IR/opt/generics | `hir.rs`, `mir.rs`, `optimizador.rs`, `monomorfizacion.rs` | HIR/MIR, verificadores, optimización, monomorfización G4. |
| Ejecución | `evaluador.rs`, `vm.rs`, `runtime.rs` | Tree-Walk, VM MIR, runtime/stdlib integrada. |
| Backends | `backend_nativo.rs`, `backend_entero.rs`, `backend_wasm.rs` | Nativo/WASM (subconjuntos). |
| Tooling | `cli.rs`, `lsp.rs`, `dap.rs`, `formateador.rs`, `depurador.rs`* | CLI, LSP, DAP, formateador, depuración. |
| Proyectos/módulos/paquetes | `proyecto.rs`, `cargador.rs`, `paquetes.rs`, `cliente_remoto.rs`, `servidor/*` | Proyecto, carga módulos, paquetes BPK, registro remoto/cliente. |
| Interop/IO/util | `ffi.rs`, `http.rs`, `fechas.rs`, `paridad.rs` | FFI C, HTTP, fechas, paridad. |
| Bins | `main.rs`, `bin/bsk.rs`, `bin/basilisco-registry.rs` | Entradas CLI/registry. |

\* `depurador.rs` existe como archivo de soporte/diagnóstico según inventario; verificar rol real en 14.3.

### 2.3 Suites de pruebas (`prototipo/tests`)

65 archivos `.rs` + directorios `fixtures/`, `fixtures-ffi/` (inventario por nombre):

- **Conformidad/base:** `conformidad*.rs`, `control_flujo.rs`, `variables.rs`, `extensiones_control.rs`, `lambdas.rs`, `closures.rs`, `registros.rs`, `integracion.rs`, `fuzz_basico.rs`, `tutorial.rs`.
- **Semántica/tipos/generics:** `semantico.rs`, `semantico_modulos.rs`, `inferencia_tipos.rs`, `genericos_frontend_g2.rs`, `genericos_semantica_g2_2.rs`, `genericos_semantica_g3.rs`, `genericos_monomorfizacion_g4.rs`.
- **IR/VM:** `hir.rs`, `mir.rs`, `vm.rs`, `vm_debug_control.rs`, `optimizador.rs`, `puerta_c.rs`, `gate_3c_integration.rs`, `pipeline_3d5_integration.rs`.
- **Paridad/diferencial:** `diferencial_vm.rs`, `diferencial_multibackend.rs`.
- **Backends:** `backend_nativo.rs`, `backend_wasm.rs`.
- **Tooling:** `cli.rs`, `lsp.rs`, `dap.rs`, `formateador.rs`, `depurador.rs`.
- **Módulos/proyectos/paquetes/registry:** `modulos_e2e.rs`, `sintaxis_modulos.rs`, `proyectos.rs`, `paquetes.rs`, `bpk.rs`, `registry_integration.rs` (2 tests `#[ignore]`, PostgreSQL aislado), `remoto_e2e.rs`, `server_auth.rs`, `cargador.rs`.
- **Fase 08 nominales:** `s8_structs_nominales.rs`, `s8_enums_nominales.rs`, `s8_traits_metodos.rs`, `s8_pattern_matching.rs`.
- **Stdlib/FFI/concurrencia:** `stdlib_fase09.rs`, `ffi.rs`, `concurrencia_fase10.rs`.
- **Higiene:** `_tmp_dbg_sols.rs` (artefacto temporal/debug presente; revisar en 14.9, no eliminar en 14.0).

### 2.4 Extensión VS Code

- `editors/vscode-basilisco/`: `package.json`, `extension.js`, gramática, snippets, language config, README/CHANGELOG, tests Extension Host (`@vscode/test-electron` + mocha).
- Comando verificado en Fase 13: `npm test` (23 passing).

## 3. Arquitectura (visión operativa)

```text
Fuente (.bsk/.nexo)
  -> lexer/token
  -> parser/AST
  -> semantico + simbodo/tipos
  -> HIR + verificador HIR
  -> monomorfizacion G4 (cuando aplica)
  -> MIR + verificador MIR
  -> optimizador
  -> ejecución: VM (default) / Tree-Walk (fallback --motor=arbol)
  -> backends experimentales: nativo (Rust/rustc), WASM (subconjunto)
Tooling: CLI, LSP stdio, DAP, formateador, REPL, paquetes/registry
Extensión VS Code: LanguageClient LSP + DebugAdapterExecutable DAP + providers locales
```

## 4. Componentes y dependencias

| Componente | Depende de | Evidencia/suites principales | Estado base conocido |
|---|---|---|---|
| Parser/AST | lexer/token | conformidad, parser/sintaxis, cli/lsp | Fase 13 verificado; fix 5.1D span función. |
| Semántica/tipos | parser, simbodo | semantico, inferencia, conformidad tipos, generics | Verificado. |
| G4 monomorfización | semántica, HIR/MIR | genericos_monomorfizacion_g4 (33) | FROZEN salvo regresión reproducible. |
| HIR/MIR/verificadores | semántica, IR | hir, mir, gate_3c, pipeline_3d5 | Verificado; HIR020/E1010 tratados congelados. |
| VM | MIR/runtime | vm, diferencial_vm, vm_debug_control | Verificado. |
| Tree-Walk | AST/runtime | conformidad, diferencial_vm | Verificado. |
| Backends nativo/WASM | MIR/runtime/toolchain | backend_nativo, backend_wasm, diferencial_multibackend | Parcial/depende toolchain Node/rustc/wasm32. |
| Stdlib/runtime | evaluador/vm/runtime | stdlib_fase09, conformidad_*, diferencial | Verificado en alcance. |
| Concurrencia | evaluador/vm | concurrencia_fase10, diferencial | Verificado alcance Fase 10. |
| FFI | evaluador/vm/libffi | ffi, diferencial | Verificado alcance Fase 11. |
| Paquetes/registry | cliente/servidor/bpk | paquetes, bpk, registry_integration, remoto_e2e, server_auth | Local verificado; registry E2E depende PostgreSQL aislado. |
| LSP | parser/semántica/diagnóstico | lsp (18), Extension Host 5.1D | Verificado; frozen salvo regresión. |
| DAP/VM Debug | vm, cli/dap | vm_debug_control (9), dap, Extension Host B | Verificado; frozen salvo regresión. |
| Formatter | parser/formateador/cli | formateador (13), E2E D | Verificado; frozen salvo regresión. |
| CLI/REPL | todos | cli (47), E2E C/E | Verificado; REPL frozen salvo regresión. |
| Extensión VS Code | LSP/DAP/CLI | npm test 23 | Verificado; local 0.4.0 no publicada. |

## 5. Pruebas existentes y comandos

### 5.1 Verificación global (base Fase 13 M6)

```bash
export PATH="/home/auron/.nvm/versions/node/v24.20.0/bin:$PATH"
cd prototipo
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --no-fail-fast   # invocación única, con Node en PATH
git diff --check
cd ../editors/vscode-basilisco
npm test
```

### 5.2 Suites focales de referencia

```bash
cargo test --test lsp
cargo test --test cli
cargo test --test formateador
cargo test --test vm_debug_control
cargo test --test dap
cargo test --test genericos_monomorfizacion_g4
cargo test --test diferencial_vm
cargo test --test diferencial_multibackend
cargo test --test backend_nativo
cargo test --test backend_wasm
cargo test --test paquetes
cargo test --test bpk
cargo test --test ffi
cargo test --test concurrencia_fase10
cargo test --test stdlib_fase09
cargo test --test registry_integration -- --ignored --nocapture  # requiere PostgreSQL aislado
```

### 5.3 Condiciones de entorno

- Node v24.20.0 detectado en `/home/auron/.nvm/versions/node/v24.20.0/bin/node`; no siempre en PATH por defecto.
- `backend_wasm` / `diferencial_multibackend` sensibles a Node/toolchain.
- `registry_integration` tests marcados `#[ignore]`: requieren PostgreSQL aislado y `DATABASE_URL`.

## 6. Historial de evidencias (evitar duplicación)

| Evidencia | Alcance | Uso en Fase 14 |
|---|---|---|
| `docs/reports/FASE_13_EXPEDIENTE_CIERRE.md` | Cierre Fase 13, matriz y verificación global | Línea base inicial; no repetir sin razón. |
| `docs/reports/AUDITORIA_INTEGRAL_BASILISCO.md` | Auditoría amplia histórica | Referencia de estructura; datos pueden estar desactualizados, contrastar con estado actual. |
| `docs/reports/AUDITORIA_ORDEN_Y_TRAZABILIDAD_BASILISCO.md` | Trazabilidad histórica | Usar para detectar contradicciones documentales, no como verdad actual sin contraste. |
| Reportes Fase 04/05/06 | Paridad, nativo, tipos | Contexto histórico; reabrir solo con regresión reproducible. |
| `DEVELOPMENT_STATE.md` | Estado formal por fase | Fuente operativa; no modificar en 14.0. |
| `ROADMAP_MASTER.md` | Definición Fase 14 | Criterios base de fase. |

## 7. Riesgos y contradicciones conocidas

1. **Worktree sucio grande** (73 entradas): riesgo de mezclar alcance; toda misión debe preservar cambios ajenos.
2. **Dependencia Node en PATH**: falsos negativos WASM/diferencial si no se normaliza entorno.
3. **Registry PostgreSQL**: tests `#[ignore]` no se ejecutan en workspace normal; no declararlos PASS sin ejecución.
4. **Extensión no publicada**: 0.4.0 local no publicada; publicada 0.3.2. Riesgo documental/comercial fuera de alcance técnico.
5. **Capacidades locales VS Code vs LSP contractual**: rename/signature/semantic tokens locales no equivalen a soporte LSP completo.
6. **`bsk publish`**: contrato local BPK, sin HTTP remoto. Riesgo de documentación ambigua.
7. **Artefacto temporal** `prototipo/tests/_tmp_dbg_sols.rs`: higiene, posible ruido; no borrar sin autorización.
8. **Documentación histórica desalineada**: auditorías antiguas contienen estados/contadores antiguos; Fase 14 debe contrastar con ejecución actual.
9. **Componentes congelados**: G4, LSP, DAP/VM Debug, REPL, Formatter, HIR020/E1010; reabrir solo con regresión reproducible.

## 8. Misiones propuestas 14.1–14.9

| Misión | Objetivo | Dependencias | Entregable principal | Criterio de aceptación |
|---|---|---|---|---|
| 14.1 | Línea base reproducible y normalización de entorno | 14.0 | Reporte de ejecución base | fmt/clippy/workspace/npm ejecutados con entorno documentado; resultados completos; sin correcciones de código. |
| 14.2 | Auditoría frontend (lexer/parser/AST/diagnósticos) | 14.1 | Hallazgos y cobertura | Casos positivos/negativos revisados; discrepancias documentadas con evidencia; sin cambios de código. |
| 14.3 | Auditoría semántica/tipos/generics/G4 | 14.1 | Hallazgos por contrato | G4 tratado frozen salvo regresión; invariantes semánticas verificadas; regresiones con reproducción. |
| 14.4 | Auditoría HIR/MIR/VM/optimizador/paridad TW↔VM | 14.1, 14.3 | Matriz de paridad/IR | diferencial_vm/IR/gates revisados; divergencias clasificadas; sin falsos PASS. |
| 14.5 | Auditoría backends nativo/WASM y toolchains | 14.1, 14.4 | Reporte de subconjunto/limitaciones | backend_nativo/wasm/diferencial_multibackend ejecutados con toolchain documentada; UNVERIFIABLE/UNSUPPORTED separados de PASS. |
| 14.6 | Auditoría stdlib/concurrencia/FFI | 14.1, 14.4 | Matriz de capacidades runtime | stdlib_fase09, concurrencia_fase10, ffi verificados; límites documentados. |
| 14.7 | Auditoría paquetes/registry/CLI/publish | 14.1 | Reporte de contrato | cli, paquetes, bpk, registry con entorno adecuado; `publish` confirmado local-only. |
| 14.8 | Auditoría tooling LSP/DAP/REPL/Formatter/VS Code | 14.1 | Reporte E2E/protocolo | lsp/dap/vm_debug/formateador/cli/npm test; componentes frozen solo reabiertos por regresión reproducible. |
| 14.9 | Consolidación, hardening documentado y expediente final Fase 14 | 14.1–14.8 | Expediente Fase 14 | Matriz global, defectos abiertos/cerrados, riesgos residuales, dictamen final; sin iniciar Fase 15. |

## 9. Criterios de aceptación por misión

- **14.1:** Una ejecución de línea base con comandos, entorno, exit codes y resumen; discrepancias ambientales separadas de fallos.
- **14.2:** Lista de áreas auditadas, tests ejecutados, hallazgos con archivo/línea/comando/reproducción; cero correcciones aplicadas.
- **14.3:** Confirmación de invariantes G4/semántica; cualquier hallazgo incluye reproducción mínima y clasificación de severidad.
- **14.4:** Resultados de diferencial_vm/IR/gates; divergencias clasificadas como bug/regresión/límite documentado.
- **14.5:** Resultados por backend con toolchain presente/ausente; no confundir UNVERIFIABLE con PASS.
- **14.6:** Cobertura por API/capacidad probada; límites FFI/concurrencia/stdlib explícitos.
- **14.7:** Contrato CLI/packages/registry validado; pruebas ignoradas por dependencia externa claramente marcadas.
- **14.8:** Evidencia de protocolo/E2E para LSP/DAP/REPL/Formatter/VS Code; regresiones congeladas solo si reproducibles.
- **14.9:** Expediente final con matriz completa, evidencia reproducible, riesgos residuales, y dictamen; sin modificaciones no autorizadas.

## 10. Exclusiones

- No iniciar Fase 15 ni ecosistema autónomo.
- No publicar extensión/crates/paquetes.
- No hacer commits/push/merge/tags/releases.
- No modificar `DEVELOPMENT_STATE.md` ni `ROADMAP_MASTER.md` salvo misión autorizada explícitamente.
- No reabrir componentes congelados sin evidencia reproducible.
- No agregar funcionalidades nuevas durante auditoría/estabilización.

## 11. Comandos de verificación (referencia)

```bash
# Entorno
git branch --show-current
git rev-parse --short HEAD
git status --short
command -v node || true
/home/auron/.nvm/versions/node/v24.20.0/bin/node --version
cargo --version

# Base (prototipo)
export PATH="/home/auron/.nvm/versions/node/v24.20.0/bin:$PATH"
cd prototipo
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --no-fail-fast
cd ..
git diff --check

# Extensión
cd editors/vscode-basilisco
export PATH="/home/auron/.nvm/versions/node/v24.20.0/bin:$PATH"
npm test

# Focales (según misión)
cd ../prototipo
cargo test --test lsp
cargo test --test dap
cargo test --test vm_debug_control
cargo test --test cli
cargo test --test formateador
cargo test --test genericos_monomorfizacion_g4
cargo test --test diferencial_vm
cargo test --test diferencial_multibackend
cargo test --test backend_nativo
cargo test --test backend_wasm
cargo test --test paquetes
cargo test --test bpk
cargo test --test ffi
cargo test --test concurrencia_fase10
cargo test --test stdlib_fase09

# Registry (solo con PostgreSQL aislado)
# DATABASE_URL="postgres://..." cargo test --test registry_integration -- --ignored --nocapture
```

## 12. Condiciones de cierre de Fase 14

Fase 14 solo puede declararse cerrada cuando:

1. Existe matriz completa 00–13 con evidencia vigente o limitación externa documentada.
2. `fmt`, `clippy -D warnings`, `cargo test --workspace --no-fail-fast` (invocación única, entorno correcto) y `git diff --check` pasan, o cualquier fallo está clasificado con evidencia y resolución/autorización humana.
3. `npm test` de extensión pasa o los fallos están clasificados con evidencia.
4. No hay regresiones críticas sin explicar.
5. Skips/ignores/bloqueos externos no se presentan como PASS.
6. Los componentes congelados no se reabrieron sin regresión reproducible.
7. Existe expediente final Fase 14 independiente, reproducible y revisable.
8. Revisión/autorización humana explícita registrada según gobernanza.

## 13. Detención

Este documento es el entregable de Fase 14.0. No se inició 14.1. No se inició Fase 15. No se modificó código ni pruebas. No se modificaron `DEVELOPMENT_STATE.md` ni `ROADMAP_MASTER.md`.
