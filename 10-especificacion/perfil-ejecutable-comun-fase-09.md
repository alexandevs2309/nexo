# Perfil Ejecutable Común (PEC) — Fase 09

Estado: **BORRADOR TÉCNICO / READY FOR HUMAN REVIEW** (no APROBADA, no CLOSED).
Fecha: 2026-11-23. Rama: `main`.

## Propósito

Matriz verificable de capacidades por capa/backend para el subconjunto consolidado en Fase 09.
Clasificación:
- **OBL** (obligatoria en alcance PEC): capacidad soportada y exigida en esa capa/backend para el subconjunto declarado; debe tener evidencia positiva.
- **LIM** (limitada): soportada parcialmente en el alcance declarado por la capa/backend; no se amplía artificialmente; límites documentados.
- **NO** (no soportada): fuera de alcance o no implementada en esa capa/backend; debe producir rechazo explícito/estable (BAS-7124 u otro diagnóstico normativo) y no generar artefactos inválidos.

## Matriz

| Capacidad | Semántica | HIR | MIR | TW | VM | Nativo | WASM | Evidencia / nota |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| Núcleo escalar/control/funciones | OBL | OBL | OBL | OBL | OBL | OBL | LIM | `diferencial_vm` 153/0; `backend_nativo` tests existentes; WASM mantiene alcance declarado |
| Colecciones base (arreglo/lista/objeto/índice) | OBL | OBL | OBL | OBL | OBL | OBL | LIM | suites diferenciales/backend_nativo existentes; WASM no ampliado |
| Cierres/capturas mutables (Celda) | OBL | OBL | OBL | OBL | OBL | NO | NO | Fase 09 A: `diff_f09_*` 8/8, `diff_closure_mutable*` 2/2, `diferencial_vm` 153/0. Nativo: no soportado (rechazo capacidad/BAS-7124 por ruta de no-soportado); WASM: fuera de alcance declarado |
| Struct nominal (decl/constr/campo/C2) | OBL | OBL | OBL | OBL | OBL | OBL | NO | `s8_structs_nominales`, `s8_b7124_backend_nativo_soporta_nominal_basico`, smoke Fase 09 B; WASM BAS-7124 |
| Enum nominal unitario/payload posicional | OBL | OBL | OBL | OBL | OBL | OBL | NO | `s8_enums_nominales`, smoke Fase 09 B enum+coincidir; WASM BAS-7124 |
| `coincidir` v1 nominal | OBL | OBL | OBL | OBL | OBL | OBL | NO | `s8_pattern_matching`/s8 enums; smoke nativo enum payload+coincidir PASS |
| Métodos inherentes | OBL | OBL | OBL | OBL | OBL | OBL | NO | `s8_traits_metodos`; smoke nativo PASS |
| Dispatch estático trait/método (alcance aprobado) | OBL | OBL | OBL | OBL | OBL | OBL | NO | `s8_traits_metodos`; smoke nativo PASS |
| Genéricos monomorfizados (G4, no nominales) | OBL | OBL | OBL | OBL | OBL | LIM | LIM | Semántica/HIR/MIR/TW/VM cubiertos por suites G4/diferenciales existentes. Nativo/WASM: limitados al subconjunto que ya soportan tras monomorfización; capacidades no soportadas deben rechazar con BAS-7124/no-soportado, sin artefactos inválidos. |
| Try/catch/invoke | OBL | OBL | OBL | OBL | OBL | NO | NO | OBL en Semántica/HIR/MIR/TW/VM (cobertura diferencial/VM existente). NO en Nativo/WASM: rechazo explícito/estable; `backend_nativo` emite `BAS-7124` para main con operaciones no soportadas, y rutas no soportadas no deben materializar semántica falsa. |
| Genéricos nominales / traits avanzados / dyn | NO | NO | NO | NO | NO | NO | NO | Fuera de alcance Fase 09/R3; rechazo semántico/capacidad según capa |
| Igualdad nominal automática `==/!=` (R3) | NO | NO | NO | NO | NO | NO | NO | Rechazada semánticamente (BAS-7126 / `IgualdadNominalNoSoportada`) en TW/VM/semántica; no habilitada en backend nativo. |
| Registry integración PostgreSQL | N/A | N/A | N/A | N/A | N/A | N/A | N/A | `registry_integration.rs` 2 tests ignorados; dependencia psql CLI eliminada (sqlx); CI job `e2e-postgres`; harness `scripts/test-registry-integration.sh` |

## Rechazos normativos

- Nativo: BAS-7124 para capacidades nominales/capacidades no soportadas fuera del subconjunto (`backend_nativo.rs`).
- WASM: mantiene rechazos BAS-7124 existentes para nominal/coincidir no soportado.

## Evidencia de rechazos no soportados

| Caso | Capa/backend | Esperado | Evidencia |
|---|---|---|---|
| Nominal en WASM | WASM | BAS-7124 / rechazo explícito | `s8_structs_nominales`/`s8_enums_nominales` rechazos WASM existentes |
| Rango en nativo | Nativo | BAS-7124, sin Valor::Nulo silencioso | `backend_nativo.rs` genera error explícito BAS-7124; `funcion_soportada` marca no soportado |
| try/catch/invoke en nativo | Nativo | BAS-7124 | `generar_rust` emite `compile_error!("BAS-7124: ... operaciones no soportadas (try/catch/invoke...)"` |
| Igualdad nominal `==/!=` | Semántica/TW/VM | BAS-7126 / `IgualdadNominalNoSoportada` | `semantico.rs` BAS-7126, `evaluador.rs`/`vm.rs` rechazan; regresión Fase 09 |
| Escape Celda interna (retorno/global) | VM | rechazo `NoSoportado` interno | `vm.rs` rechaza Celda en retorno y asignación global; pruebas diferenciales closures/celdas verdes |

## Limitaciones preservadas

- WASM no ampliado en Fase 09.
- Nativo no soporta closures/celdas, try/catch/invoke, rangos, ni capacidades fuera del subconjunto nominal aprobado.
- Registry E2E requiere PostgreSQL (CI service/harness Docker); no se ejecuta en `cargo test --workspace` por defecto.
