# FASE 14.2-R — Auditoría residual de tipos, inferencia y consistencia entre capas

> **Fecha:** 2026-11-27  
> **Rama:** `main` | **HEAD:** `bb1bfed` (verificado)  
> **Worktree:** sucio preexistente preservado. Sin operaciones Git destructivas, sin commits/push/merge/tags/releases/publicaciones.  
> **Base:** `docs/reports/FASE_14_2_AUDITORIA_TIPOS_INFERENCIA_GENERICOS.md` y corrección FE-14.2-01.

## 1. Estado Git y gobernanza

- Gobernanza leída: `AGENTS.md`, `.opencode/skills/basilisco-governance/SKILL.md`, `docs/governance/*` aplicables, `DEVELOPMENT_STATE.md` (solo lectura), `ROADMAP_MASTER.md` (solo lectura).
- Rama `main`, HEAD `bb1bfed`. `git diff --check` PASS.
- No se modificó `DEVELOPMENT_STATE.md` ni `ROADMAP_MASTER.md`. No se inició Fase 14.3 ni Fase 15. No se reabrió G4/HIR020/E1010 lógicamente; solo adaptaciones mecánicas previas de Coleccion ya documentadas.

## 2. Matriz de cobertura

| Área | Auditoría 14.2 (previa) | Evidencia en 14.2-R | Estado |
|---|---|---|---|
| Representación de tipos (TipoSem, TipoAnotacion, TipoCanon) | Inspección + corrección FE-14.2-01 | Inspección de definición Coleccion con tamaño; revisión de usos en semantico/hir/mir/vm/runtime/backends/monomorf | Cubierto en alcance focal |
| Compatibilidad/conversiones | Inspección + FE-14.2-01 | Revisión `compatible`, `unificar`, VM `convertir_tipo`, runtime `TipoConv`; probe FE-14.2-01 6/7/8; endurecimiento VM/runtime tamaño Coleccion | Cubierto con hallazgo residual cerrado |
| Inferencia | Inspección focal + suites | Suites inferencia/generics/conformidad; sin cambio de lógica nuevo | Cobertura por suites focales |
| Unificación G3 | Inspección focal | Revisada forma Coleccion en unificar; suites g2.2/g3 | Cobertura por suites focales |
| Genéricos/monomorfización G4 | Inspección focal | G4 suite 33 PASS; canonicalizar/canon_a_sem revisados para Coleccion tamaño | Cubierto focalmente; G4 congelado |
| HIR/MIR/optimizaciones | Inspección focal | HIR/MIR suites PASS; optimizador sin refs Coleccion; verificación por patrones | Cobertura focal, no exhaustiva |
| VM/runtime/backends | Inspección parcial | VM/runtime revisados; backend_nativo emite TipoConv Coleccion con tamaño; backend_entero/wasm inspeccionados; WASM sin Coleccion refs | Cobertura parcial |
| Coherencia estática/runtime | Probe y conformidad | Probes sem vs rt para FE-14.2-01; suites conformidad | Cubierto focalmente |

**Clasificación de evidencia:** anterior = informe 14.2 y suites ya ejecutadas; esta misión = re-ejecución de 14 suites focales, inspección dirigida FE-14.2-01, probes, y endurecimiento VM/runtime.

## 3. Archivos y capas inspeccionados

- `prototipo/src/semantico.rs` — TipoSem::Coleccion, compatible, unificar, sustituir, inferencia literal.
- `prototipo/src/parser.rs` — TipoAnotacion (inspección previa).
- `prototipo/src/hir.rs` — tipos_compatibles_retorno, ArregloLiteral, Coleccion.
- `prototipo/src/mir.rs` — tipos_compatibles_mir, Coleccion.
- `prototipo/src/vm.rs` — convertir_tipo Coleccion.
- `prototipo/src/runtime.rs` — TipoConv::Coleccion.
- `prototipo/src/backend_nativo.rs` — emisión TipoConv Coleccion.
- `prototipo/src/backend_entero.rs` — patrón Coleccion.
- `prototipo/src/backend_wasm.rs` — ausencia de Coleccion (inspección por grep).
- `prototipo/src/optimizador.rs` — ausencia de Coleccion (inspección por grep).
- `prototipo/src/monomorfizacion.rs` — TipoCanon Coleccion tamaño, canonicalizar/canon_a_sem/codificar.
- Tests focales listados en §5.

## 4. Contratos verificados (FE-14.2-01)

| # | Contrato | Evidencia | Resultado |
|---|---|---|---|
| 1 | `Coleccion<T,N>` conserva N | Definición TipoSem/TipoCanon, construcción literal `elementos.len()` | OK |
| 2 | Tamaño no se pierde en AST/HIR/MIR | Inspección ArregloLiteral lowering; tipos portan Coleccion(T,N); suites HIR/MIR PASS | OK focal |
| 3 | VM respeta contrato | `vm.rs` convertir_tipo Coleccion exige `elems.len()==*n` (endurecido en 14.2-R); probe 7 rechaza | OK |
| 4 | Conversiones mantienen restricción tamaño | runtime `TipoConv::Coleccion` ahora exige tamaño; VM igual; backend_nativo emite tamaño | OK |
| 5 | Backends no aceptan silenciosamente asignaciones rechazadas por semántico | Semántico rechaza; backends reciben tipos; no se halló aceptación silenciosa en inspección focal | OK focal |
| 6 | `Arreglo<T,N> <- Coleccion<T,N>` funciona | Probe: sem=[] rt=Ok | OK |
| 7 | `Arreglo<T,N> <- Coleccion<T,M>` falla | Probe: sem error tamaño, rt error | OK |
| 8 | `Arreglo<T,N> <- Lista<T>` conservador si longitud desconocida | Probe: sem rechaza, rt acepta (longitud runtime coincide); política conservadora documentada | OK (política) |
| 9 | Optimizaciones no eliminan/alteran checks necesarios | optimizador sin refs Coleccion; no se observó eliminación de checks | OK focal |
| 10 | Monomorfización mantiene tamaño correcto | TipoCanon::Coleccion(te,n), canonicalizar/canon_a_sem/codificar preservan n; G4 33 PASS | OK |

## 5. Pruebas ejecutadas y resultados exactos

Ejecutadas en `prototipo/`:

```text
$ cargo test --test conformidad_tipos --test inferencia_tipos --test genericos_frontend_g2 --test genericos_semantica_g2_2 --test genericos_semantica_g3 --test genericos_monomorfizacion_g4 --test hir --test mir --test semantico --test conformidad_arreglos_listas --test conformidad_funciones --test conformidad_semantica --test conformidad_conversiones --test conformidad_control
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 109 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --check
(no output, exit 0)

$ git diff --check
OK (exit 0)
```

No se ejecutó `cargo test --workspace --no-fail-fast`. No se declara workspace aprobado.

## 6. Hallazgos nuevos

| ID | Severidad | Estado | Resumen |
|---|---|---|---|
| FE-14.2R-01 | MEDIUM | **CORREGIDO** | `vm::convertir_tipo` y `runtime::TipoConv::Coleccion` ignoraban el tamaño `_n`, permitiendo conversión runtime permisiva para Coleccion aunque el tipo portara tamaño. |

- **Causa raíz:** adaptación FE-14.2-01 dejó enforcement de tamaño en semántico/HIR/MIR pero mantuvo conversión Coleccion permisiva en VM/runtime.
- **Corrección:** exigir `elems.len() == *n` en VM y runtime para Coleccion, alineado con Arreglo.
- **Regresión:** cubierta por comportamiento existente/probe 6-7; no se añadió prueba nueva separada por ser ajuste defensivo dentro del mismo contrato FE-14.2-01. Riesgo residual bajo.
- **Archivos:** `prototipo/src/vm.rs`, `prototipo/src/runtime.rs`.

## 7. Correcciones realizadas y regresiones

- FE-14.2R-01 corregido (VM/runtime). Re-ejecución de 14 suites focales: PASS.

## 8. Riesgos y limitaciones pendientes

- **Workspace no ejecutado:** no se afirma aprobación global.
- **Cobertura no exhaustiva:** no se auditó cada camino de inferencia/unificación/generics; se priorizó FE-14.2-01 y contratos de tamaño.
- **WASM:** backend_wasm sin referencias Coleccion; si Coleccion llega a WASM, podría requerir manejo explícito. No se observó regresión en esta misión.
- **Lista->Arreglo estático:** rechazo conservador ya documentado; programas que dependían de longitud runtime coincidente ahora fallan en semántico.

## 9. Evaluación de FE-14.2-01 después de auditoría residual

FE-14.2-01 mantiene coherencia en las capas examinadas. La brecha defensiva residual en VM/runtime conversion (FE-14.2R-01) fue identificada y corregida. No se hallaron regresiones reproducibles en suites focales.

## 10. Dictamen final

**INCOMPLETE — RESIDUAL FINDINGS**

FE-14.2-01 y la brecha residual FE-14.2R-01 quedan corregidas con evidencia focal, pero la cobertura residual no es exhaustiva y el workspace completo no fue ejecutado. No se certifica el sistema de tipos globalmente. Detenido para revisión humana; no se inició Fase 14.3 ni Fase 15.
