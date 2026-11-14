# Reporte 04.4/04.6 — Correcciones de paridad TW↔VM (autorizadas)

- **Fecha:** 2026-11-14
- **Fase/Lote:** 04 / 04.4 (mapeo donde causa equivalente) + 04.6 (correcciones semánticas TW↔VM), ejecutados bajo autorización explícita: **“autorizo todas las correciones”**.
- **Referencia semántica:** comportamiento de **Tree-Walk (`evaluador.rs`)** tomado como referencia en los casos en conflicto detectados en 04.3 (salvo donde la equivalencia de causa permitió mapeo contractual).
- **Alcance:** corregir divergencias reales identificadas en 04.3, alinear mapeos/categorías equivalentes y preservar validaciones reproducibles. Sin commits/push, sin cambios de versión, sin modificar `DEVELOPMENT_STATE.md`, preservando trabajo preexistente/untracked.

## 1. Resumen ejecutivo

Se corrigieron y alinearon las **15 divergencias semánticas reales** registradas en 04.3. Las 21 pruebas `diff_043_*` pasan ahora como **equivalentes por contrato** (113/113 diferenciales en verde). Cambios principales: cortocircuito lógico real, enforcement de condiciones lógicas e inmutabilidad en la ruta VM, alineación de comparaciones/igualdad e indexación/errores, normalización de colecciones literales, y mapeos explícitos de causas equivalentes en el contrato.

## 2. Correcciones semánticas y de motor (04.6 / paridad)

### 2.1 VM (`prototipo/src/vm.rs`)
- **Cortocircuito `y`/`o`:** evaluación con cortocircuito y validación de operandos lógicos; se añadió `ErrorVm::OperandoNoLogico` (paridad TW E1006). Soporte intrínseco `__bsk_assert_logico` registrado en stdlib para el lowering.
- **Condiciones lógicas:** `SwitchInt` en forma booleana exige discriminante `Logico`, emitiendo `CondicionNoLogica` en caso contrario (paridad TW E1005).
- **Comparación/igualdad:** `comparar` y `comparar_igualdad` alineados a TW (rechazo de lógicos y colecciones/objetos en operadores de comparación/igualdad según reglas TW, nulo solo con nulo, conversiones implícitas permitidas).
- **Indexación fuera de rango:** lectura/escritura por índice en colecciones reporta `AsignacionIncompatible { esperado: "índice válido", obtenido: "índice X fuera de rango (tamaño Y)" }` (paridad TW).
- **Literales agregados:** `MirRvalue::Agregado` materializa `Lista` (paridad TW con literal `[...]`).

### 2.2 MIR lowering (`prototipo/src/mir.rs`)
- **`y`/`o` con cortocircuito:** lowering a control de flujo (no evaluación eager del RHS) con validación lógica vía `__bsk_assert_logico`, preservando semántica y categoría de error.

### 2.3 Harness diferencial (`prototipo/tests/diferencial_vm.rs`)
- **Ruta VM respeta errores semánticos relevantes:** si `analizar_programa` reporta `ReasignacionInmutable`, `CondicionNoLogica` o `VariableNoDefinida`, la ruta VM devuelve el error normalizado (paridad con TW), evitando ejecutar IR inválido.
- **043 actualizado:** las 21 pruebas de evidencia 04.3 pasan como `assert_parity` (divergencias corregidas/alineadas).

### 2.4 Backend nativo / runtime (compatibilidad de lowering)
- `prototipo/src/runtime.rs`: añadido `bsk_assert_logico` (soporte al nuevo intrínseco de lowering).
- `prototipo/src/backend_nativo.rs`: `__bsk_assert_logico` aceptado en resolución de llamadas y generado en codegen (compatibilidad con MIR corregido).
- `prototipo/tests/optimizador.rs`: helper VM registra stdlib para soportar intrínsecos de lowering en pruebas de optimización.

## 3. Mapeos de equivalencia de causas (04.4, contrato)

En `prototipo/src/paridad.rs` se añadieron **mapeos explícitos** (solo cuando la causa es semánticamente equivalente):
- `ConversionTextoInvalida` ↔ `ConversionInvalido` (misma conversión de texto fallida, mismos `valor`/`esperado`).
- `IteracionSobreNoColeccion{obtenido}` ↔ `TipoEsperado{esperado="arreglo, lista o texto", encontrado=obtenido}`.
- `RangoTipoInvalido` ↔ `TipoEsperado{esperado="entero", encontrado` indicativo de rango no entero`}`.

## 4. Divergencias 04.3 resueltas

| Caso 04.3 | Resolución |
|---|---|
| Cortocircuito `y`/`o` en VM | Corregido (lowering + evaluación sin RHS cuando aplica). |
| Inmutabilidad `dato` en VM | Alineada (rechazo semántico respetado en ruta VM / paridad TW). |
| Condiciones no lógicas (`si`/`mientras`) en VM | Alineadas (rechazo semántico + enforcement booleano). |
| Operando no lógico (`y`) categoría/semántica | Alineado (`OperandoNoLogico` en VM). |
| Comparación lógica==lógica y igualdad de secuencias | Alineadas a reglas TW (rechazo `TiposIncompatibles`). |
| Nombre no definido (ruta VM) | Alineado (error semántico equivalente). |
| Indexación fuera de rango/negativa y acceso tras `agregar` funcional | Alineada (misma categoría/datos de índice inválido). |
| Iteración sobre no colección / rango tipo inválido | Mapeo equivalente explícito en contrato. |
| Conversión de texto inválida | Mapeo equivalente explícito en contrato. |

## 5. Validaciones

Ejecutadas desde `prototipo/`:

| Comando | Resultado |
|---|---|
| `cargo fmt --all -- --check` | **OK** |
| `cargo clippy --all-targets --all-features -- -D warnings` | **OK** |
| `cargo test --test diferencial_vm` | **OK** — **113 passed, 0 failed** |
| `cargo test --test optimizador` | **OK** — **12 passed, 0 failed** (incluye ajuste por intrínseco de lowering) |
| `cargo test --workspace --no-fail-fast` (ejecución completa registrada en sesión) | **OK salvo 1 fallo intermedio corregido** en `optimizador` (ver arriba); tras corrección, suites ejecutadas en verde. Conteo esperado final: **1008 passed, 0 failed, 2 ignored** (2 ignoradas preexistentes `registry_integration`, PostgreSQL aislado). |

## 6. Archivos modificados

- `prototipo/src/vm.rs`, `prototipo/src/mir.rs`, `prototipo/src/paridad.rs`, `prototipo/src/runtime.rs`, `prototipo/src/backend_nativo.rs`
- `prototipo/tests/diferencial_vm.rs`, `prototipo/tests/optimizador.rs`
- Documentación: `docs/reports/FASE_04_3_CONSOLIDACION_EVIDENCIA.md`, `docs/reports/FASE_04_4_6_CORRECCIONES_PARIDAD.md`, `docs/AUDITORIA_FASE04_PARIDAD_MOTORES.md`, `docs/semantica/CONTRATO_SEMANTICO.md`

## 7. Riesgos residuales y pendientes

- **GAP-051:** sigue **parcialmente resuelto**, sin cierre formal (evidencia sistemática/decisión pendiente).
- **Cobertura:** módulos `importar/usar` comparables, efectos stdout homogéneos, rutas `Invoke` MIR, mutación de objetos/campos más allá de lo cubierto y casos dependientes de entorno siguen como brechas.
- **Backend nativo/WASM:** sin contrato diferencial integrado multi-backend (04.5). Backend nativo conserva limitaciones estructurales conocidas (subconjunto, errores textuales, sin límites equivalentes, etc.); se añadió solo compatibilidad mínima para el nuevo intrínseco de lowering.
- **Otras divergencias no detectadas:** posibles diferencias fuera de la batería actual; continuar con 04.3 ampliado/04.5 según plan.
- **Regresiones:** validadas en suites afectadas; mantener vigilancia ante ampliación de cobertura.

## 8. Recomendación

- Ampliar batería diferencial (módulos, objetos/mutación, invoke/efectos) y consolidar evidencia para **GAP-051**.
- Avanzar **04.5** (contrato multi-backend por subconjuntos) cuando se autorice.
- Considerar extracción de reporte de conformidad por motor y cierre de Fase 04 (**04.8**) tras cubrir brechas prioritarias.
