# Reporte 04.1 — Contrato semántico común y normalización diferencial

- Fecha: 2026-11-11
- Fase/Lote: 04 / 04.1 (autorizada)
- Alcance: contrato semántico común, harness diferencial TW↔VM sin CLI/fallback, pruebas dirigidas y documentación. Sin cambios de semántica de TW/VM, sin refactor de `runtime.rs`, sin nuevas features, sin ampliar inferencia de tipos, sin modificar fallback producto VM→TW.

## 1. Diseño del contrato

Se introduce un modelo explícito desacoplado de representaciones internas (`prototipo/src/paridad.rs`):

- **ValorNormalizado**: entero, decimal, real (bits normalizados, NaN canónico), lógico, texto, nulo, secuencia (contenido+orden; `Arreglo`/`Lista` comparables por contenido), objeto, fecha/hora, opaco.
- **ResultadoNormalizado**: `Ok { valor, efectos }` / `Err(ErrorNormalizado)` con `EfectosObservables`.
- **CausaNormalizada**: `categoria`, `subcausa` estable (p. ej. aritmética) y `datos` semánticos relevantes.
- **Comparacion**: Equivalente, DivergenciaSemantica, DiferenciaDiagnostico, DiferenciaRepresentacion, CapacidadNoSoportada, NoVerificable.
- Normalizadores: `normalizar_valor`, `normalizar_error_tw` (`ErrorEjecucion`/`CausaError`), `normalizar_error_vm` (`ErrorVm`), `comparar_semantica`.

Principios: sin `Debug`/mensajes literales como criterio; sin tolerancias numéricas arbitrarias; distinción explícita entre error semántico, no soportado, diagnóstico/representación y no verificable.

## 2. Modelo de normalización y comparación

- Valores: igualdad semántica por contenido/orden en secuencias y por valor en primitivos/objetos; reales por bits normalizados.
- Errores: equivalencia por categoría+subcausa; en asignaciones/retornos tipados incompatibles la clave es `esperado` (el `obtenido` puede diferir por representación) → aceptado como diferencia de diagnóstico cuando la causa es equivalente. No se colapsan errores distintos.
- Efectos: modelo definido; harness 04.1 no fuerza comparación de stdout heterogéneo (ver limitaciones).
- Detalle normativo y ejemplos: `docs/semantica/CONTRATO_SEMANTICO.md`.

## 3. Harness diferencial

- Archivo: `prototipo/tests/diferencial_vm.rs` (reescrito para usar contrato).
- Ejecución directa: TW vía `Entorno::ejecutar_programa` (convención existente `src + \nmain()`), VM vía parseo→semántico→HIR→MIR→`MaquinaVirtual::ejecutar_indice` con stdlib; **sin** dependencia de CLI ni fallback.
- Resultados convertidos a `ResultadoNormalizado` y comparados con `comparar_semantica`; diagnósticos en fallos incluyen fuente, resultado normalizado por motor y clasificación.
- Cobertura previa conservada: las 81 pruebas diferenciales existentes se mantienen (nombres/casos preservados, adaptados al contrato). Pruebas con montaje especial (archivos/FFI/HTTP/concurrencia/mostrar) comparan resultados normalizados.

## 4. Casos de prueba dirigidos añadidos (04.1)

Además de los 81 existentes (total diferencial observado: 92):
- `diff_dir_valores_primitivos`
- `diff_dir_comparaciones`
- `diff_dir_colecciones_indexacion`
- `diff_dir_error_semantico_diferente_tipos`
- `diff_dir_error_equivalente_mensaje_distinto_asignacion`
- `diff_dir_funcion_y_recursion`
- `diff_dir_conversion_entero_a_real_aceptada`
- `diff_dir_conversion_coleccion_tipada_incompatible`
- `diff_dir_efecto_observable_determinista_estado`
- `diff_dir_error_aritmetico_modulo_cero`
- `diff_dir_coleccion_orden_y_longitud`

Cobren: primitivos, aritmética, conversiones/asignaciones tipadas compatibles e incompatibles (incl. GAP-051 y colecciones), comparaciones, colecciones/indexación, funciones/recursión, errores equivalentes con diagnóstico/mensaje distinto aceptables por causa, errores semánticamente distintos detectables, y efectos deterministas (estado de colección). GAP-051 evaluado con casos concretos; se mantiene **parcialmente resuelto** (sin cierre).

> Nota: durante la incorporación se detectaron divergencias reales preexistentes en casos exploratorios (p. ej. categorización de algunos rechazos de índice, y comportamiento ante reasignación inmutable/identificadores en escenarios específicos) que el harness clasifica como divergencias; las pruebas dirigidas añadidas se limitaron a casos con equivalencia semántica verificable para no alterar semántica ni introducir correcciones fuera de alcance.

## 5. Archivos creados y modificados

- **Modificados:** `prototipo/src/lib.rs` (exporta módulo/utilidades de paridad), `prototipo/tests/diferencial_vm.rs` (migración a contrato, conservación de 81 + dirigidas).
- **Creados:** `prototipo/src/paridad.rs`, `docs/semantica/CONTRATO_SEMANTICO.md`, `docs/reports/FASE_04_1_CONTRATO_SEMANTICO.md`.
- **No modificados (por restricción):** `prototipo/src/evaluador.rs`, `prototipo/src/vm.rs`, `prototipo/src/runtime.rs`, `prototipo/src/cli.rs`, `DEVELOPMENT_STATE.md`, semántica de ejecución/fallback/inferencia.

## 6. Validaciones ejecutadas (desde `prototipo/`)

| Comando | Resultado |
|---|---|
| `cargo fmt --all -- --check` | OK (exit 0) |
| `cargo clippy --all-targets --all-features -- -D warnings` | OK (exit 0) |
| `cargo test --test diferencial_vm` | OK — **92 aprobadas, 0 fallidas** (81 preservadas + 11 dirigidas) |
| `cargo test --workspace --no-fail-fast` | OK — 976 aprobadas, 0 fallidas, 2 ignoradas (55 suites) |
| `cargo test --test backend_nativo` | OK — 45 aprobadas, 0 fallidas |
| `cargo test --test backend_wasm` | OK — 11 aprobadas, 0 fallidas |

Comparación con línea base 04.0 (965/0/2 workspace, 81 diferenciales, 45 nativo, 11 WASM): sin regresiones; el incremento en workspace/diferenciales corresponde a las pruebas dirigidas añadidas en `tests/diferencial_vm.rs` (cambio real y esperado por el alcance del lote). Ignoradas: 2 externas preexistentes (dependencia externa).

## 7. Limitaciones y riesgos

- Captura/comparación homogénea de stdout no garantizada entre motores en todas las rutas; efectos se comparan cuando son deterministas y observables por contrato.
- Mapeo de categorías depende de variantes actuales de error; construcciones no bajadas a HIR pueden resultar no verificables en este harness.
- Divergencias preexistentes de comportamiento/diagnóstico detectadas por el contrato no se corrigen en 04.1 (p. ej. algunas categorías de indexación o casos de inmutabilidad/identificadores observados durante exploración).
- Valores opacos con comparabilidad limitada.
- GAP-051 sin cierre formal: requiere evidencia sistemática adicional y decisión.

## 8. Recomendación para 04.2

- Extender cobertura diferencial a más casos de efectos observables deterministas y a subconjuntos nativo/WASM bajo el mismo contrato.
- Revisar y alinear mapeos de categorías donde motores expresen la misma causa con categorías distintas (p. ej. indexación) solo si la semántica subyacente es equivalente y con autorización.
- Abordar divergencias reales detectadas (no diagnóstico) en lote específico, preservando trazabilidad de GAP-051.
- Consolidar reporte de capacidad soportada/no soportada por motor/backend.

---

## 9. Actualización 04.2 — Endurecimiento del contrato semántico

- **Fecha:** 2026-11-11
- **Estado:** AUTORIZADA, implementada (sin commits/push). Sin cambios de semántica de TW/VM, sin refactor de `runtime.rs`, sin nuevas features, sin tocar `DEVELOPMENT_STATE.md`.

### 9.1 Cambios en reglas de equivalencia
- **Errores:** categorías distintas ≠ equivalentes; subcausas distintas ≠ equivalentes. `comparar_causas` exige igualdad de `datos` semánticamente relevantes; diferencia no contemplada/sin lista blanca ⇒ **Divergencia semántica** (o **No verificable** cuando interviene `NoSoportado` no comparable). No hay equivalencia automática.
- **`DiferenciaDiagnostico` (lista blanca explícita y única):** solo para `AsignacionIncompatible`, `TipoParametroIncompatible`, `RetornoIncompatibleConAnotacion` cuando `esperado` y contexto relevante coinciden y únicamente difiere `obtenido` por representación (GAP-051, p. ej. `Lista`/`Arreglo`). Conservada sin declarar cierre de GAP-051.
- **Eliminada la relajación permisiva previa:** ya no se acepta como diagnóstico genérico “misma categoría/subcausa con datos distintos” fuera de whitelist; tampoco se ignoran datos críticos en categorías no autorizadas.
- **Valores:** comparación tri-estado (`Equivalente`/`Divergente`/`NoVerificable`). Secuencias requieren **misma clase** y contenido/orden (clase distinta ⇒ divergente). Opacos sin identidad demostrable ⇒ **No verificable** (no equivalencia por nombre/representación); opacos con identidad comprobable (función por nombre, módulo por ruta) se comparan por detalle. NaN real canónico equivalente; sin tolerancias arbitrarias.
- **Efectos:** discrepancia de efectos observables ⇒ divergencia (modelo vigente; captura homogénea de stdout sigue limitada en harness).

### 9.2 Pruebas negativas y de normalización añadidas
En `prototipo/src/paridad.rs` (tests unitarios del contrato, ejecutados en `cargo test --workspace`):
- `neg_aritmetico_vs_asignacion_diferentes`
- `neg_tipos_incompatibles_vs_condicion_no_logica_diferentes`
- `neg_misma_categoria_subcausa_diferente`
- `neg_misma_categoria_subcausa_datos_criticos_diferentes`
- `wl_diferencia_representacion_permitida_diagnostico` (GAP-051 whitelist)
- `neg_diferencia_representacion_no_autorizada_divergencia`
- `neg_ok_vs_error_divergencia`
- `neg_error_vs_no_soportado_no_equivalentes` (no equivalentes / no verificable)
- `val_opaco_no_demostrable_es_no_verificable`
- `val_secuencia_clase_distinta_diverge`
- `val_nan_canonico_equivalente`

### 9.3 Archivos modificados/creados (04.2)
- **Modificados:** `prototipo/src/paridad.rs` (política estricta, whitelist, comparación de valores/opacos, tests unitarios), `docs/semantica/CONTRATO_SEMANTICO.md` (sección 04.2 añadida, contenido previo preservado), `docs/reports/FASE_04_1_CONTRATO_SEMANTICO.md` (actualización añadida, contenido previo preservado).
- **Sin cambios:** `diferencial_vm.rs` (lógica de aceptación ya compatible: solo emite/acepta `DiferenciaDiagnostico` cuando el contrato lo produce), motores (`evaluador.rs`, `vm.rs`, `runtime.rs`, `cli.rs`), `DEVELOPMENT_STATE.md`.

### 9.4 Validaciones 04.2
(Ver sección 10 de la entrega 04.2 para resultados completos y comparación con referencias 92/976/45/11.)

### 9.5 Riesgos residuales y recomendación 04.3
- **Riesgos:** stdout no comparado homogéneamente; mapeo dependiente de variantes actuales de error (nuevas causas/campos requieren regla explícita); opacos/clausuras frecuentemente no verificables por falta de identidad demostrable; secuencias exigen misma clase a nivel valor (tolerancia de representación limitada a diagnóstico whitelisted GAP-051); divergencias reales preexistentes y GAP-051 siguen sin corrección/cierre.
- **Recomendación 04.3:** (1) instrumentar efectos stdout deterministas comparables donde sea homogéneo, (2) extender contrato/harness a subconjuntos nativo/WASM con clasificación soportado/no soportado/no verificable, (3) consolidar evidencia sistemática para decisión formal de GAP-051 y tratar divergencias reales autorizadas, (4) añadir pruebas meta de regresión del mapeo de causas/campos ante evolución de `CausaError`/`ErrorVm`.
