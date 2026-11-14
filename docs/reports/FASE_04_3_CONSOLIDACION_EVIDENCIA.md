# Reporte 04.3 — Consolidación de evidencia TW↔VM

- **Fecha:** 2026-11-13
- **Fase/Lote:** 04 / 04.3 (autorizada)
- **Alcance:** batería diferencial sistemática dirigida TW↔VM (errores/categorías, indexación, inmutabilidad, mutación/colecciones, try/casos comparables, conversiones, operadores/lógica, efectos deterministas), clasificación por contrato y actualización de cobertura diferencial. **Sin cambios de semántica de TW/VM**, sin refactor de `runtime.rs`, sin correcciones de divergencias reales, sin tocar `DEVELOPMENT_STATE.md`, sin commits/push.

## 1. Archivos modificados / creados

- **Modificado:** `prototipo/tests/diferencial_vm.rs`
  - Helpers añadidos: `clasificar(...)` y `assert_divergencia(...)` para registrar evidencia reproducible de divergencias reales (clasificación contrato `DivergenciaSemantica`) sin alterar semántica de motores.
  - **+21 pruebas 04.3** (prefijo `diff_043_*`), clasificadas por contrato.
- **Creado:** `docs/reports/FASE_04_3_CONSOLIDACION_EVIDENCIA.md` (este reporte).

**No modificados:** `evaluador.rs`, `vm.rs`, `runtime.rs`, `backend_nativo.rs`, `cli.rs`, `paridad.rs` (contrato/mapeos), `DEVELOPMENT_STATE.md`, versiones. Trabajo preexistente/untracked preservado.

## 2. Cobertura diferencial 04.3 (clasificación por contrato)

Total diferencial TW↔VM tras 04.3: **113 pruebas** (92 previas + 21 nuevas), **0 fallidas**.

### 2.1 Equivalentes / aceptadas por contrato (assert_parity) — 6 casos
| Prueba | Eje | Clasificación observada |
|---|---|---|
| `diff_043_asignacion_compuesta_o_igual` | mutación/efecto determinista | Equivalente |
| `diff_043_conversion_explicita_texto_entero` | conversión | Equivalente |
| `diff_043_eliminar_elemento` | mutación de colección | Equivalente |
| `diff_043_modulo_por_cero_enteros` | error aritmético (causa) | Equivalente (misma causa/subcausa) |
| `diff_043_mutacion_coleccion_parametro` | mutación/aliasing por parámetro | Equivalente |
| `diff_043_objeto_json_lectura` | objeto (JSON) lectura/`json_obtener` | Equivalente |

### 2.2 Divergencias reales registradas como evidencia reproducible (assert_divergencia) — 15 casos
> Decisión por caso: **documentar; corrección diferida a lote posterior autorizado (04.4/04.6 según naturaleza)**. No se modificó semántica de motores.

| Prueba | Eje | TW (observado) | VM (observado) | Clasificación | Nota |
|---|---|---|---|---|---|
| `diff_043_nombre_no_definido_divergencia` | nombre no definido | `Err(NombreNoDefinido)` | `Err(RetornoIncompatibleConAnotacion{esperado=entero,funcion=main,obtenido=nulo})` | Divergencia semántica (categorías distintas) | VM no reporta nombre no definido en este caso. |
| `diff_043_indice_lista_fuera_de_rango_divergencia` | indexación colección | `Err(AsignacionIncompatible{... índice 5 fuera de rango ...})` | `Err(IndiceFueraDeRango{indice=5,tamano=2})` | Divergencia semántica | Misma intención, categoría/mapeo distinto. |
| `diff_043_indice_lista_negativo_divergencia` | indexación colección | `Err(AsignacionIncompatible{...})` | `Err(IndiceFueraDeRango{indice=-1,tamano=2})` | Divergencia semántica | Idem. |
| `diff_043_iteracion_sobre_no_coleccion_divergencia` | iteración/error tipo | `Err(IteracionSobreNoColeccion{obtenido=entero})` | `Err(TipoEsperado{esperado="arreglo, lista o texto", encontrado="entero"})` | Divergencia semántica | Categorías distintas. |
| `diff_043_rango_tipo_invalido_divergencia` | rango/error tipo | `Err(RangoTipoInvalido{obtenido=lógico})` | `Err(TipoEsperado{...})` | Divergencia semántica | Categorías/diagnóstico distintos. |
| `diff_043_condicion_no_logica_si_divergencia` | condición no lógica | `Err(CondicionNoLogica{obtenido=entero})` | `Ok(Logico/Entero resultante: valor 1)` | Divergencia semántica (error vs ok) | VM acepta condición no booleana. |
| `diff_043_condicion_no_logica_mientras_divergencia` | condición no lógica | `Err(CondicionNoLogica{obtenido=entero})` | `Ok(Entero(0))` | Divergencia semántica (error vs ok) | Idem en bucle. |
| `diff_043_operando_no_logico_y_divergencia` | operador lógico/tipos | `Err(OperandoNoLogico{operador=y, obtenido=entero})` | `Err(TiposIncompatibles{izquierdo=entero,derecho=lógico})` | Divergencia semántica | Categorías distintas. |
| `diff_043_cortocircuito_y_no_evalua_rhs_divergencia` | cortocircuito/lógica | `Ok(Entero(0))` (RHS no evaluado) | `Err(Aritmetico{division_por_cero})` | Divergencia semántica (ok vs error) | **VM no cortocircuita `y`**: evalúa RHS con efecto/error. |
| `diff_043_cortocircuito_o_no_evalua_rhs_divergencia` | cortocircuito/lógica | `Ok(Entero(7))` (RHS no evaluado) | `Err(Aritmetico{division_por_cero})` | Divergencia semántica (ok vs error) | **VM no cortocircuita `o`**. |
| `diff_043_inmutabilidad_dato_reasignacion_divergencia` | inmutabilidad (`dato`) | `Err(ReasignacionInmutable{nombre=x})` | `Ok(Entero(2))` | Divergencia semántica (error vs ok) | VM permite reasignación de `dato` inmutable. |
| `diff_043_agregar_y_longitud_divergencia` | mutación colección/efecto | `Err(AsignacionIncompatible{... índice/tamaño inconsistente tras agregar ...})` | `Err(IndiceFueraDeRango{indice=2,tamano=1})` | Divergencia semántica | Comportamiento/mutación observable y categorización difieren. |
| `diff_043_conversion_texto_invalida_divergencia` | conversión/error | `Err(ConversionTextoInvalida{esperado=entero,valor="no"})` | `Err(ConversionInvalido{esperado=entero,valor="no"})` | Divergencia semántica | Misma intención, categoría distinta. |
| `diff_043_comparacion_mixta_enteros_logico_divergencia` | comparación/tipos | `Err(TiposIncompatibles{izquierdo=lógico,derecho=lógico})` | `Ok(Logico(true))` | Divergencia semántica (error vs ok) | Reglas de comparación/igualdad difieren. |
| `diff_043_igualdad_secuencias_divergencia` | comparación de colecciones | `Err(TiposIncompatibles{izquierdo=lista,derecho=lista})` | `Ok(Logico(true))` | Divergencia semántica (error vs ok) | Igualdad de secuencias aceptada en VM, rechazada en TW. |

## 3. Ejes cubiertos y brechas remanentes (04.3)

- **Cubiertos (evidencia reproducible):** categorías de error e indexación (fuera de rango/negativo, iteración/rango inválido, nombre no definido, conversión inválida, operando/condición no lógica), inmutabilidad (`dato`), mutación/aliasing básico y por parámetro, lectura de objetos vía JSON, conversiones y aritmética de error, rutas try comparables existentes, efectos deterministas por estado.
- **Parcialmente cubiertos / a profundizar:** try/invoke (se validaron rutas comparables simples; casos anidados/formatos específicos descartados por parseo en esta batería — ampliar con sintaxis confirmada), mutación de objetos/campos (solo lectura JSON), más escenarios de aliasing/colecciones.
- **No ejercitados en esta batería (limitación de comparabilidad con harness actual):** módulos `importar/usar` de archivo (requieren resolución/carga de módulos y setup de entorno no integrado al harness in-memory), stdout/efectos no modelados homogéneamente, FFI/HTTP/fechas/concurrencia determinista extendida (dependencias/entorno), rutas MIR `Invoke` no expresables directamente desde fuente comparable simple.
- **Hallazgo nuevo relevante (04.3):** falta de cortocircuito en `y/o` en **VM** (ok en TW) — divergencia semántica observable con RHS erróneo/efectuoso; distinto del riesgo H-03 auditado para backend nativo, debe registrarse como divergencia TW↔VM.

## 4. Validaciones

Ejecutadas desde `prototipo/`:

| Comando | Resultado |
|---|---|
| `cargo fmt --all` / `cargo fmt --all -- --check` | **OK** |
| `cargo clippy --all-targets --all-features -- -D warnings` | **OK** |
| `cargo test --test diferencial_vm` | **OK** — **113 passed, 0 failed, 0 ignored** (antes 92; +21 casos 04.3) |

**Variación de conteo:** +21 pruebas diferenciales (6 equivalentes, 15 divergencias reales registradas). Se espera que `cargo test --workspace --no-fail-fast` refleje el mismo incremento (+21) sobre la base reciente (987 → ~1008), sujeto a las 2 ignoradas preexistentes de `registry_integration`.

## 5. Riesgos residuales

- Divergencias reales TW↔VM identificadas quedan **sin corregir** (por alcance 04.3); requieren decisión de semántica de referencia y corrección autorizada (04.6) o ajuste de mapeo solo si la causa es equivalente (04.4).
- Cortocircuito de VM, aceptación de condiciones no lógicas, inmutabilidad de `dato`, igualdad/comparaciones y categorización de errores son inconsistencias de alto impacto funcional.
- Cobertura de módulos/efectos stdout/invoke-MIR y casos dependientes de entorno sigue incompleta.
- GAP-051 permanece **parcialmente resuelto** (sin cierre).

## 6. Recomendación para 04.4

1. **Triage y normalización/mapeo (solo donde la causa sea semánticamente equivalente):** indexación (AsignacionIncompatible↔IndiceFueraDeRango), iteración/rango (↔TipoEsperado) y conversión texto inválida (↔ConversionInvalido) — documentar datos relevantes y añadir pruebas meta sin relajar reglas.
2. **Separar divergencias semánticas reales (no mapeables):** cortocircuito `y/o` en VM, inmutabilidad `dato` en VM, condiciones no lógicas aceptadas, igualdad/comparación de secuencias y lógicos, y manejo de nombre no definido — derivar a **04.6** con semántica de referencia definida.
3. **GAP-051:** consolidar evidencia sistemática de colecciones tipadas/asignaciones/conversiones y preparar decisión formal (cierre o alcance explícito).
4. **Ampliar evidencia:** módulos comparables (si se habilita harness con fixtures), más try/invoke expresables, y efectos observables homogéneos.

## 7. Actualización post-corrección 04.4/04.6 (2026-11-14)

- **Estado:** los 21 casos `diff_043_*` pasan como equivalentes/paridad por contrato tras correcciones autorizadas (referencia TW) y mapeos explícitos de causas equivalentes.
- **Resuelto:** cortocircuito `y`/`o` en VM, enforcement de condiciones lógicas e inmutabilidad de `dato`, alineación de comparaciones/igualdad e indexación, nombre no definido, y categorización/mapeo para conversión de texto inválida, iteración sobre no colección y rango de tipo inválido.
- **Mapeos aplicados (04.4):** `ConversionTextoInvalida`↔`ConversionInvalido`, `IteracionSobreNoColeccion`↔`TipoEsperado`, `RangoTipoInvalido`↔`TipoEsperado`.
- **Evidencia vigente:** `cargo test --test diferencial_vm` → **113 passed, 0 failed**. Ver `docs/reports/FASE_04_4_6_CORRECCIONES_PARIDAD.md` para detalle completo.
