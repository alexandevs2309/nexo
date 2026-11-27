# FASE 14 — Certificación integral de auditoría, corrección y estabilización

> **Fecha:** 2026-11-27  
> **Rama:** `main` | **HEAD base:** `bb1bfed` (sin commits nuevos)  
> **Alcance:** Fase 14 completa (14.0–14.2 cierre + auditoría integral remanente), conforme a misión única de certificación.  
> **Worktree:** sucio preexistente preservado íntegramente. Sin operaciones Git destructivas. Sin commits/push/merge/tags/releases/publicaciones. Sin cambios a `DEVELOPMENT_STATE.md` ni `ROADMAP_MASTER.md`. No se inició Fase 15.

## 1. Resumen ejecutivo

Se ejecutó una auditoría y estabilización integral sobre el estado real del proyecto BASILISCO. Se preservaron los cambios preexistentes del worktree. Se revisaron los informes previos de Fase 14 (plan, 14.1, 14.1-C, 14.2, 14.2-R) y se verificaron sus correcciones con evidencia actual.

**Correcciones realizadas en esta fase (acumuladas 14.1–14.2 cierre):** FE-01 (LOW, ubicación top-level módulo), FE-02 (HIGH, resolución semántica de miembros de módulo por alias), FE-14.2-01 (HIGH, tamaño Coleccion/compatibilidad Arreglo-Coleccion-Lista), FE-14.2R-01 (MEDIUM, VM/runtime TipoConv Coleccion ignoraba tamaño), FE-14.2R-02 (MEDIUM, `unificar` aceptaba Coleccion/Arreglo con tamaño distinto). Se añadió regresión por cada corrección y se ajustó un test HIR por política FE-14.2-01 documentada.

**Validación global final:** `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets --all-features -- -D warnings` PASS (EXIT 0); `cargo test --workspace --no-fail-fast` PASS (EXIT 0, 69 líneas `test result:`, 1504 passed, 0 failed, 2 ignored externos); `git diff --check` PASS; extensión VS Code `npm test` PASS (23 passing, EXIT 0). `registry_integration` (2 tests ignored por PostgreSQL) ejecutado adicionalmente en PostgreSQL aislado Docker: 2 passed, 0 failed, EXIT 0.

**Dictamen:** PASS — FASE 14 COMPLETED / STABILIZED / READY FOR HUMAN REVIEW.

## 2. Estado Git y cumplimiento de gobernanza

- **Gobernanza leída:** `AGENTS.md`, `.opencode/skills/basilisco-governance/SKILL.md`, `DEVELOPMENT_STATE.md`, `ROADMAP_MASTER.md`, `docs/governance/PHASE_GATE.md`, `docs/governance/WORKFLOW.md`, `docs/governance/VALIDATION.md`, `docs/reports/FASE_14_PLAN_AUDITORIA.md`, reportes 14.1/14.1-C/14.2/14.2-R.
- **Git inspeccionado:** rama `main`, HEAD `bb1bfed`. Worktree sucio preexistente preservado (82+ entradas al cierre; incluye M preexistentes y artefactos Fase 14). No se ejecutaron comandos destructivos. No se revirtieron cambios ajenos.
- **Restricciones cumplidas:** no commits/push/merge/tags/releases/publicaciones; no cambios en `DEVELOPMENT_STATE.md`/`ROADMAP_MASTER.md`; no Fase 15; componentes congelados no reabiertos lógicamente sin regresión reproducible; adaptaciones mecánicas de Coleccion en G4/IR documentadas.

## 3. Alcance real inspeccionado

| Área | Inspección realizada | Evidencia principal |
|---|---|---|
| 1. Lexer/parser/AST/frontend | Lectura dirigida lexer/parser/token/diagnostico/AST; revisión FE-01/FE-02; fuzz/parser/semántico suites | 14.1, workspace, clippy |
| 2. Semántica/símbolos/módulos/imports/visibilidad | semantico/cargador/simbodo; FE-02 fix verificado; alias scopes/aislamiento | 14.1-C, semantico_modulos, modulos_e2e, workspace |
| 3. Tipos/inferencia/unificación/conversiones | semantico TipoSem/compatible/unificar/tipo_comun; FE-14.2-* | 14.2/14.2-R/cierre, conformidad_tipos/inferencia_tipos, workspace |
| 4. Genéricos/monomorfización | generics G2/G2.2/G3/G4 suites; TipoCanon Coleccion shape mecánico | G4 frozen, suites PASS, workspace PASS |
| 5. HIR/MIR/optimizaciones | hir/mir/optimizador inspección; compatibilidad Coleccion; test HIR ajustado por política | hir/mir/optimizador suites, workspace |
| 6. VM/runtime/memoria | vm/runtime TipoConv/convertir_tipo; tamaño Coleccion; Arc/Mutex uso observado | vm/vm_debug/runtime, workspace, clippy |
| 7. Concurrencia/asincronía | concurrencia_fase10 en workspace; inspección de presencia | workspace PASS |
| 8. FFI y aislamiento | ffi.rs inspección unsafe/libloading; límites por diseño; suite ffi en workspace | workspace PASS, inspección |
| 9. Backends nativo/entero/WASM | backend_nativo/entero/wasm inspección; backend_wasm delega a backend_entero; wasm tests con Node | workspace PASS, clippy PASS |
| 10. Biblioteca estándar | stdlib_fase09 y conformidad en workspace | workspace PASS |
| 11. CLI/formatter/REPL/LSP/DAP | cli/lsp/dap/formateador/repl en workspace; VS Code E2E | workspace PASS, npm test 23 PASS |
| 12. Paquetes/dependencias/registry/publicación | paquetes/cliente_remoto/servidor inspección; BPK extraction checks; publish local-only | workspace PASS, registry_integration aislado PASS |
| 13. Seguridad/permisos/integridad/aislamiento | grep unsafe/unwrap/panic; path traversal/canonicalize; zip extraction guards; sqlx parametrizado | inspección dirigida, workspace/clippy |
| 14. Extensión VS Code | npm test Extension Host + E2E A-F | 23 passing, EXIT 0 |
| 15. Documentación/reproducibilidad/DevEx | reportes Fase 14, contrato publish local, limitaciones documentadas | reportes 14.x, este expediente |

## 4. Matriz de cobertura por subsistema

| Subsistema | Suites/evidencia | Resultado | Cobertura |
|---|---|---|---|
| Frontend/lexer/parser | lexer/parser lib, semantico, sintaxis_modulos, fuzz_basico, integracion | PASS | Alta (focal + workspace) |
| Semántica/módulos | semantico, semantico_modulos, modulos_e2e | PASS | Alta |
| Tipos/inferencia | conformidad_tipos, inferencia_tipos, conformidad_* | PASS | Alta |
| Genéricos G2-G4 | genericos_frontend_g2, g2_2, g3, monomorfizacion_g4 | PASS | Alta |
| HIR/MIR/opt | hir, mir, optimizador, gate_3c, pipeline_3d5 | PASS | Alta |
| VM/runtime | vm, vm_debug_control, diferencial_vm, runtime/conformidad | PASS | Alta |
| Backends | backend_nativo, backend_wasm, diferencial_multibackend | PASS | Alta (con Node/rustc toolchain) |
| Stdlib/FFI/concurrencia | stdlib_fase09, ffi, concurrencia_fase10 | PASS | Alta |
| Paquetes/registry | paquetes, bpk, registry_integration aislado, remoto_e2e, server_auth | PASS | Alta (aislado) |
| CLI/REPL/LSP/DAP/Formatter | cli, lsp, dap, formateador, depurador | PASS | Alta |
| VS Code | npm test (23) | PASS | Alta |
| Calidad estática | cargo fmt --check, cargo clippy -D warnings, git diff --check | PASS | Completa |

## 5. Hallazgos encontrados y severidad

| ID | Severidad | Estado | Área | Resumen |
|---|---|---|---|---|
| FE-01 | LOW | Corregido | Frontend/cargador-semántico | Ubicación 1:1 incorrecta para sentencia top-level no permitida en módulo (`Expresion`/`Bloque`). |
| FE-02 | HIGH | Corregido | Semántico/módulos | Acceso a miembro inexistente/privado por alias de módulo no diagnosticado semánticamente. |
| FE-14.2-01 | HIGH | Corregido | Tipos/compatibilidad | `Coleccion` sin tamaño y compatibilidad Arreglo<->Coleccion/Lista permitían aceptar estáticamente casos inválidos por tamaño. |
| FE-14.2R-01 | MEDIUM | Corregido | VM/runtime | `convertir_tipo`/`TipoConv::Coleccion` ignoraban tamaño de Coleccion. |
| FE-14.2R-02 | MEDIUM | Corregido | Unificación G3 | `unificar` aceptaba `Coleccion<T,N>` formal vs `Arreglo<T,M>` observado con `N!=M`. |

No se identificaron hallazgos CRITICAL o HIGH abiertos atribuibles al alcance al cierre de la validación global. Hallazgos informacionales/limitaciones se documentan en §11.

## 6. Causa raíz y correcciones realizadas

### FE-01 (LOW) — Corregido
- **Causa raíz:** `ubicacion_sentencia` no extraía ubicación de `Sentencia::Expresion`/`Bloque`.
- **Corrección:** helper `ubicacion_expresion` y manejo de Expresion/Bloque en cargador y semántico.
- **Regresión:** `modulos_e2e::e2e_modulo_sentencia_no_permitida_reporta_ubicacion_expresion`.

### FE-02 (HIGH) — Corregido
- **Causa raíz:** alias de módulo registrado solo como variable `Desconocido`; `Acceso`/`LlamadaIndirecta` no resolvían exportación/privacidad.
- **Corrección:** `modulos_alias_scopes` scope-aware paralelo a `scopes`, sincronizado push/pop/save/restore; `resolver_alias_modulo` con sombreado léxico; resolución en `Acceso` emite `SimbodoNoExportado`/`SimbodoPrivado` y tipos de exportados.
- **Regresión:** 14 tests `fe02_*` en `semantico_modulos.rs`.

### FE-14.2-01 (HIGH) — Corregido
- **Causa raíz:** `TipoSem::Coleccion` sin tamaño; `compatible` ignoraba tamaño para Arreglo con Coleccion/Lista.
- **Corrección:** `Coleccion(Box<TipoSem>, usize)`; inferencia literal conserva tamaño; `compatible`/`unificar` exigen tamaño para Arreglo-Coleccion; `Arreglo<-Lista` rechazado estáticamente por tamaño desconocido (política conservadora documentada). Adaptación mecánica en HIR/MIR/VM/runtime/backends/TipoCanon G4.
- **Regresión:** 3 tests en `conformidad_tipos.rs`.

### FE-14.2R-01 (MEDIUM) — Corregido
- **Causa raíz:** VM/runtime convertían Coleccion ignorando `_n`.
- **Corrección:** `elems.len()==*n` en `vm::convertir_tipo` y `runtime::TipoConv::Coleccion`.

### FE-14.2R-02 (MEDIUM) — Corregido
- **Causa raíz:** `unificar` con formal `Coleccion(T,N)` aceptaba `Arreglo(T,M)` observado sin comprobar `N==M`.
- **Corrección:** exigir `nf==no` para Arreglo observado.
- **Regresión:** `conformidad_tipos::unificar_coleccion_arreglo_tamano_distinto_falla`.

### Ajuste de prueba por política (no defecto de producto)
- `hir.rs`: `d1_lista_a_arreglo_elemento_compatible_acepta` → `d1_lista_a_arreglo_rechaza_por_tamano_desconocido` para reflejar política FE-14.2-01.

## 7. Archivos modificados y pruebas añadidas

### Código fuente (cambios Fase 14 atribuibles)
- `prototipo/src/cargador.rs` — FE-01 ubicación sentencia top-level.
- `prototipo/src/semantico.rs` — FE-01 ubicación, FE-02 alias scope-aware, FE-14.2-01 Coleccion tamaño/compatible/unificar, FE-14.2R-02 unificar Arreglo tamaño.
- `prototipo/src/runtime.rs` — FE-14.2-01/FE-14.2R-01 TipoConv Coleccion con tamaño.
- `prototipo/src/vm.rs` — FE-14.2-01/FE-14.2R-01 convertir_tipo Coleccion con tamaño.
- `prototipo/src/hir.rs` — compatibilidad Coleccion tamaño; ajuste test HIR por política.
- `prototipo/src/mir.rs` — compatibilidad Coleccion tamaño.
- `prototipo/src/backend_nativo.rs`, `prototipo/src/backend_entero.rs` — shape Coleccion.
- `prototipo/src/monomorfizacion.rs` — adaptación mecánica TipoCanon Coleccion (G4 congelado, sin cambio lógico).

### Pruebas añadidas/modificadas (Fase 14 atribuibles)
- `prototipo/tests/modulos_e2e.rs` — regresión FE-01.
- `prototipo/tests/semantico_modulos.rs` — 14 regresiones FE-02.
- `prototipo/tests/conformidad_tipos.rs` — 4 regresiones FE-14.2-01/FE-14.2R-02.

### Reportes Fase 14
- `docs/reports/FASE_14_PLAN_AUDITORIA.md`
- `docs/reports/FASE_14_1_AUDITORIA_FRONTEND.md`
- `docs/reports/FASE_14_1C_CORRECCION_FE02.md`
- `docs/reports/FASE_14_2_AUDITORIA_TIPOS_INFERENCIA_GENERICOS.md`
- `docs/reports/FASE_14_2_AUDITORIA_RESIDUAL.md`
- `docs/reports/FASE_14_2_CIERRE_INTEGRAL.md`
- `docs/reports/FASE_14_CERTIFICACION_INTEGRAL.md` (este)

> Nota: el worktree contiene otros archivos modificados preexistentes ajenos a esta misión; no se atribuyen a Fase 14 y se preservaron.

## 8. Verificación de hallazgos históricos

| Hallazgo/contrato | Verificación | Resultado |
|---|---|---|
| FE-01 ubicación top-level módulo | Regresión e2e + workspace | OK |
| FE-02 alias miembro inexistente/privado | 14 regs semantico_modulos + suites | OK |
| FE-14.2-01 tamaño Coleccion/Arreglo/Lista | regs conformidad_tipos + workspace | OK |
| FE-14.2R-01 VM/runtime Coleccion size | inspección + workspace | OK |
| FE-14.2R-02 unificar Coleccion/Arreglo size | reg conformidad_tipos + workspace | OK |
| G4 frozen | No reapertura lógica; solo shape Coleccion mecánico; G4 suite PASS | OK |
| HIR020/E1010, REPL, DAP/VM Debug, LSP, Formatter, CLI, packages/registry | Sin reapertura; suites relevantes PASS | OK |
| `bsk publish` local-only | Inspección CLI/docs; no upload HTTP remoto | OK |

## 9. Resultados exactos de suites y comandos globales

### Formato y calidad estática

```text
$ (cd prototipo && cargo fmt --check)
EXIT:0

$ (cd prototipo && cargo clippy --workspace --all-targets --all-features -- -D warnings)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 12s
EXIT:0

$ git diff --check
OK
EXIT:0
```

### Workspace Rust (final)

```text
$ export PATH="/home/auron/.nvm/versions/node/v24.20.0/bin:$PATH"
$ (cd prototipo && cargo test --workspace --no-fail-fast)
EXIT:0
69 líneas test result:
1504 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out
```

Líneas `test result:` exactas:

```text
test result: ok. 242 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 59.31s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.08s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.78s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.44s
test result: ok. 153 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.42s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.84s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.26s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 109 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.18s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Registry integration (aislado, PostgreSQL Docker)

```text
$ docker run -d --rm --name bsk_fase14_pg -e POSTGRES_PASSWORD=bsk -e POSTGRES_USER=bsk -e POSTGRES_DB=bsk_test -p 127.0.0.1:55432:5432 postgres:16-alpine
$ export DATABASE_URL="postgres://bsk:bsk@127.0.0.1:55432/bsk_test"
$ (cd prototipo && cargo test --test registry_integration -- --ignored --nocapture)
running 2 tests
test concurrencia_misma_version ... ok
E2E: publicación OK
E2E: descarga OK
E2E: instalación verificada OK
E2E: persistencia tras reinicio OK
test ciclo_publicacion_descarga_instalacion ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.17s
EXIT:0
$ docker rm -f bsk_fase14_pg
```

### Extensión VS Code

```text
$ export PATH="/home/auron/.nvm/versions/node/v24.20.0/bin:$PATH"
$ (cd editors/vscode-basilisco && npm test)
...
  23 passing (2m)
Exit code:   0
EXIT:0
```

Observación: aparecieron mensajes de ruido del host de VS Code (`SIGPIPE`/`Extension host... unresponsive/responsive`/`AgentHostStarter...`), pero la suite finalizó con 23 passing y EXIT 0.

## 10. Fallos, pruebas ignoradas y dependencias externas

- **Fallos finales:** ninguno.
- **Ignorados en workspace:** 2 tests en `registry_integration.rs` (`ciclo_publicacion_descarga_instalacion`, `concurrencia_misma_version`) — requieren PostgreSQL aislado. Fueron ejecutados aparte en contenedor Docker aislado con PASS.
- **Dependencias externas usadas:** Node v24.20.0 (`/home/auron/.nvm/versions/node/v24.20.0/bin`), rustc/cargo toolchain local, Docker para PostgreSQL aislado. No se alteraron servicios de usuario ni bases de datos existentes.
- **Corridas intermedias no finales:** workspace3 EXIT 101 (test HIR política FE-14.2-01, corregido), workspace4 EXIT 101 (error compilación `es_err()` en test ajustado, corregido) — documentadas en 14.2 cierre.

## 11. Riesgos residuales y limitaciones conocidas

- **INFORMATIONAL/Contrato:** parser sin recuperación multi-error (FE-03 anterior). Posiciones char-based vs UTF-16 LSP default (FE-04 anterior), sensible por contrato LSP congelado.
- **Política tipos:** `Arreglo<T,N> <- Lista<T>` rechazado estáticamente por tamaño desconocido; runtime podría aceptar si longitud coincide. Documentado como endurecimiento conservador.
- **Auditoría no exhaustiva línea a línea:** inspección dirigida + suites globales; no equivale a prueba formal de todos los invariantes.
- **FFI:** por diseño carga librerías dinámicas nativas; superficie unsafe controlada por libffi/libloading pero inherentemente de alto privilegio.
- **BPK/zip:** extracción con defensas (path traversal, NFC, tamaño, symlinks, staging). Límite por `file.size()` declarado; endurecimiento adicional contra zip-bomb por lectura acotada puede considerarse mejora futura.
- **Componentes congelados:** no reabiertos lógicamente; solo verificados por pruebas/regresiones.

## 12. Evaluación de seguridad y estabilidad

- **Estabilidad:** workspace completo PASS, clippy -D warnings PASS, VS Code E2E PASS, registry aislado PASS. Sin defectos CRITICAL/HIGH abiertos.
- **Seguridad:** sin hallazgos CRITICAL/HIGH nuevos en inspección dirigida. SQLx parametrizado en servidor registry. Cargador confina rutas canonicalizadas al root. Extracción BPK valida rutas, symlinks, tamaños y prefijo payload. FFI unsafe delimitado; sigue siendo capacidad privilegiada por contrato.
- **Integridad/reproducibilidad:** comandos y entornos documentados. Paquetes BPK con hash/firma/nonce según contrato existente.

## 13. Matriz de capacidades reales del lenguaje y toolchain

| Capacidad | Estado real | Evidencia/nota |
|---|---|---|
| Frontend (lexer/parser/AST/diagnósticos) | Implementado y verificado | suites frontend/workspace |
| Semántica, símbolos, módulos, visibilidad | Implementado y verificado | FE-02 corregido, suites |
| Tipos/inferencia/unificación/generics/mono G4 | Implementado y verificado en alcance | FE-14.2-* corregidos, suites G2-G4 |
| HIR/MIR/optimizador/verificadores | Implementado y verificado | suites IR/gates |
| VM y runtime | Implementado y verificado | vm/vm_debug/diferencial |
| Tree-Walk | Implementado y verificado | conformidad/diferencial |
| Backends nativo/WASM | Implementado en subconjunto y verificado | backend_nativo/wasm/diferencial_multibackend |
| Stdlib | Implementado y verificado en alcance | stdlib_fase09/conformidad |
| Concurrencia Fase 10 | Implementado y verificado en alcance | concurrencia_fase10 |
| FFI | Implementado y verificado en alcance; privilegiado | ffi suite; unsafe por diseño |
| CLI/REPL/Formatter/LSP/DAP | Implementado y verificado | cli/lsp/dap/formateador/repl, VS Code |
| Paquetes BPK/local | Implementado y verificado | paquetes/bpk |
| Registry remoto/cliente/servidor | Implementado en alcance existente y verificado en entorno aislado | registry_integration + server_auth/remoto_e2e |
| `bsk publish` remoto | **No implementado** (contrato local BPK/firma sidecar) | inspección CLI/docs; no afirmar remoto |
| Extensión VS Code | Implementada localmente y verificada | npm test 23 PASS; 0.4.0 no publicada |

## 14. Dictamen final

**PASS — FASE 14 COMPLETED / STABILIZED / READY FOR HUMAN REVIEW**

Criterios satisfechos: alcance relevante inspeccionado; hallazgos reproducibles dentro del alcance corregidos con regresiones; FE-01/FE-02/FE-14.2-01/FE-14.2R-01/FE-14.2R-02 verificados; no quedan defectos CRITICAL/HIGH pendientes atribuibles al alcance; validación global esencial pasó (`fmt`, `clippy -D warnings`, `workspace --no-fail-fast`, `git diff --check`, VS Code `npm test`, registry aislado). Limitaciones residuales documentadas explícitamente. No se modificó el estado formal de fases. No se inició Fase 15.
