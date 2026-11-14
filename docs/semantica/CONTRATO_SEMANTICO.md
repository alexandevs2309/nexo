# Contrato semántico común TW↔VM (04.1)

Estado: documentación de contrato y normalización diferencial (lote 04.1). No modifica la semántica de ejecución de Tree-Walk ni de VM.

## 1. Objetivo

Definir un modelo explícito para comparar resultados de ejecución entre motores (Tree-Walk y VM) sin depender de `Debug`, formato interno ni mensajes literales de error. Este contrato es la referencia de conformidad para las pruebas diferenciales TW↔VM y, posteriormente, para backends nativo y WASM dentro de sus subconjuntos soportados.

Implementación de referencia: `prototipo/src/paridad.rs` y harness `prototipo/tests/diferencial_vm.rs`.

## 2. Modelo de equivalencia

La comparación produce una clasificación:

- **Equivalente**: mismo resultado semántico según este contrato.
- **Divergencia semántica**: valor, causa de error o efecto observable determinista distinto.
- **Diferencia de diagnóstico**: misma causa/categoría semántica relevante pero datos/representación de diagnóstico distintos (p. ej. mensajes o campos `obtenido` con representación distinta). Las pruebas diferenciales de paridad la aceptan cuando la causa es equivalente.
- **Diferencia de representación**: distinta forma sin impacto semántico (normalmente absorbida por normalización).
- **Capacidad no soportada**: rechazo legítimo por característica no soportada en el contexto/motor evaluado; no se equipara a divergencia semántica ni a error semántico del programa.
- **No verificable**: evidencia insuficiente para afirmar conformidad/divergencia (p. ej. construcción no bajada a HIR en el contexto de la prueba).

## 3. Normalización de resultados

### 3.1 Resultado satisfactorio
- Se compara la **categoría/forma** y el **valor semántico** del valor de retorno (`Option<Valor>`).
- Valores soportados: entero (`i64`), decimal (escala fija interna `i128`), real (`f64` normalizado por bits, NaN canónico), lógico, texto, nulo, secuencias, objetos y fecha/hora.
- **Colecciones secuenciales** (`Arreglo` y `Lista`): se normalizan como secuencias comparando contenido y orden elemento a elemento (la clase concreta puede diferir en representación interna; la equivalencia diferencial se basa en contenido/orden).
- **Objetos**: comparación por claves y valores normalizados.
- **Opacos** (archivo, módulo, función/clausura, celda): comparación estructural limitada (tipo + detalle estable disponible); si no es comparable, debe tratarse como limitación.
- No se usa `Debug` como criterio de igualdad.

### 3.2 Numéricos
- No se introducen tolerancias numéricas arbitrarias.
- Enteros y decimales se comparan por valor exacto representado.
- Reales se comparan por representación de bits normalizada (NaN colapsado a patrón canónico); equivalencias numéricas siguen las reglas/tipos existentes de BASILISCO (ADR-0005).

### 3.3 Resultado de error
- Se compara **categoría** y **causa/subcausa** semántica normalizada, con datos relevantes cuando aplican.
- Se distingue (entre otras): aritmética (desbordamiento, división por cero, exceso de rango decimal, conversión ilegal, tipos incompatibles aritméticos), nombre no definido, declaración duplicada, asignación incompatible, reasignación inmutable, condición/operando no lógico, tipos incompatibles, límite de iteraciones, firma/retorno/parámetros incompatibles, indexación y rangos, errores de archivo/módulo/FFI, y estados de ejecución (límites, no soportado, etc. según motor).
- **No se exige igualdad literal de mensajes** si la causa semántica es la misma.
- **No se unifican errores semánticamente distintos** para forzar paridad.
- Para asignaciones/conversiones tipadas incompatibles, la clave semántica principal es la categoría y el tipo/valor esperado; el campo `obtenido` puede variar por representación (p. ej. literal colección como `Lista` vs `Arreglo`) y se admite como diferencia de diagnóstico cuando la causa es equivalente.

### 3.4 Efectos observables
- Modelo previsto: salida estándar y otros efectos deterministas comparables cuando la prueba lo aísla y ambos motores los exponen de forma comparable.
- En 04.1 el harness diferencial compara principalmente valor/error; la captura de stdout no es homogénea entre motores en todas las rutas (TW puede emitir a stdout del proceso, VM captura salida en algunos contextos), por lo que no se fuerza comparación de stdout salvo casos explícitamente instrumentados. Efectos de archivos/red/FFI/concurrencia se consideran solo en pruebas deterministas y aisladas.

## 4. Capacidades no soportadas

- Un rechazo por capacidad no soportada se clasifica como **Capacidad no soportada** (o no verificable si falta bajada/verificación), no como divergencia semántica.
- Un error semántico del programa (p. ej. división por cero, asignación incompatible, índice fuera de rango) **no** es “no soportado”.
- El contrato exige distinguir: resultado correcto, error semántico, capacidad no soportada, diferencia de diagnóstico, diferencia de representación y resultado no verificable.

## 5. Limitaciones conocidas

- La normalización depende de la información expuesta por `Valor`, `ErrorEjecucion` (TW) y `ErrorVm` (VM); categorías/causas no modeladas pueden caer en mapeos genéricos.
- Algunas divergencias preexistentes de diagnóstico/representación o de comportamiento entre motores pueden ser detectadas por el harness (el contrato las clasifica; no corrige semántica).
- Valores opacos tienen comparabilidad limitada.
- Equivalencia de reales: normalización por bits/NaN canónico; no hay tolerancia epsilon.
- No sustituye una unificación de runtime (fuera de alcance de 04.1).

## 6. Ejemplos de comparación

- **Equivalente (valor)**: `2 + 3 * 4 - 1` → ambos retornan entero `13`.
- **Equivalente (error por causa)**: división por cero → ambos reportan error aritmético con subcausa `division_por_cero` (mensajes literales pueden diferir).
- **Equivalente con diferencia de diagnóstico**: `var xs: lista<entero> = [1, 2.5]` → ambos rechazan por asignación incompatible con esperado `lista<entero>`; el `obtenido` puede representarse distinto (Lista/Arreglo). Caso GAP-051 cubierto.
- **Divergencia semántica**: un motor retorna valor y el otro error, o categorías/subcausas de error incompatibles.

## 7. Relación con pruebas diferenciales

- Harness: `prototipo/tests/diferencial_vm.rs`.
- Ejecución **directa** de TW y VM (sin CLI, sin fallback de producto VM→TW).
- Se conserva la cobertura diferencial previa (81 pruebas) y se añaden pruebas dirigidas (primitivos, aritmética, conversiones/asignaciones tipadas compatibles e incompatibles, comparaciones, colecciones/indexación, funciones/recursión, errores equivalentes con mensajes/diagnósticos distintos, errores semánticamente diferentes cuando aplica, efectos deterministas, rechazos).
- GAP-051: evaluado mediante casos concretos (conversiones/asignaciones/retornos y colecciones tipadas); permanece **parcialmente resuelto** hasta evidencia suficiente y decisión formal de cierre (no se fuerza cierre).

## 8. Uso como referencia para backends

Nativo y WASM deberán compararse contra este contrato dentro de los subconjuntos que declaren soportar; capacidades fuera de subconjunto deben clasificarse como no soportadas/no verificables, no como divergencia semántica del lenguaje.

## 10. Contrato multi-backend 04.5 (TW/VM/Nativo/WASM)

Extensión del contrato común (reutilizando `prototipo/src/paridad.rs`, sin arquitectura paralela):

- **Clasificaciones obligatorias:** `EQUIVALENT` (equivalente dentro del subconjunto soportado), `DIVERGENCE` (diferencia con soporte declarado en ambos), `UNSUPPORTED` (funcionalidad no soportada por al menos un backend; no es divergencia), `UNVERIFIABLE` (evidencia insuficiente).
- **Subconjuntos de capacidades declarados explícitamente:** matriz en `capacidades_backend(Backend)` (TW/VM referencia; Nativo: numéricos/lógicos, control, funciones/recursión, texto/colecciones básicas, módulos, JSON stdlib, stdout determinista — requiere rustc, sin try/invoke/closures/celdas/rangos ni concurrencia/FFI/IO archivos declarados; WASM: subconjunto entero/lógico + export `bsk_main`, sin texto/colecciones/try/módulos/closures/concurrencia/FFI/IO/JSON).
- **Reglas:** no confundir ausencia de soporte con divergencia; no convertir errores de backend en éxitos; comparación determinista y normalizada (sin `Debug`); limitaciones estructurales documentadas.
- **Harness/pruebas:** `prototipo/tests/diferencial_multibackend.rs` (positivas, negativas/clasificación, fixtures reproducibles por ejecución real cuando hay dependencias).
- **Estado:** GAP-051 mantiene whitelist acotada y queda **parcialmente resuelto, sin cierre formal** (decisión técnica: no eliminar whitelist sin evidencia sistemática suficiente). Ver `docs/reports/FASE_04_5_CONTRATO_MULTIBACKEND.md`.

---

## 9. Endurecimiento 04.2 (política estricta)

Actualización del contrato para evitar falsos positivos. Se preserva el contenido y alcance 04.1; las reglas siguientes lo endurecen.

### 9.1 Política estricta de equivalencia de errores
1. **Categorías diferentes** nunca son equivalentes.
2. **Subcausas diferentes** nunca son equivalentes.
3. **Datos semánticamente relevantes** deben coincidir según la regla explícita de la categoría.
4. **Diferencias de representación** solo se aceptan mediante listas blancas explícitas (categoría + campos).
5. **Diferencia no contemplada** (misma categoría/subcausa pero datos no iguales y sin lista blanca aplicable) produce **Divergencia semántica** (nunca equivalencia automática). `No soportado` frente a otra causa no comparable se clasifica como **No verificable**.
6. **`DiferenciaDiagnostico`** solo se emite en casos declarados y justificados por lista blanca; el harness diferencial solo la acepta como tal resultado del contrato.

### 9.2 Listas blancas de diferencias diagnósticas (declaradas)
- **GAP-051 (asignación/retorno/parámetro tipado incompatible)** — categorías: `AsignacionIncompatible`, `TipoParametroIncompatible`, `RetornoIncompatibleConAnotacion`.
  - **Campos que deben coincidir:** `esperado` y todo el resto de datos de contexto presentes (`funcion`, `parametro`, según aplique).
  - **Campo tolerado como representación:** `obtenido` puede diferir **únicamente** cuando la causa semántica es la misma y el resto coincide (p. ej. representación de colección `Lista` vs `Arreglo` en diagnóstico de asignación tipada).
  - **Justificación:** divergencia preexistente de representación diagnóstica; la regla de conversión/compatibilidad (GAP-051) es la misma. No implica cierre de GAP-051.
  - **No autorizado:** diferencia en `esperado`, en contexto relevante o en cualquier otro campo/categoría.

No existen otras listas blancas activas en 04.2.

### 9.3 Categorías y campos semánticos relevantes (resumen)
- **Aritmético:** categoría + `subcausa` relevante (desbordamiento, división por cero, exceso de rango decimal, conversión ilegal, tipos incompatibles aritméticos); datos iguales o cubiertos por la subcausa.
- **Asignación/Tipo parámetro/Retorno incompatible (anotado):** `esperado` y contexto (`funcion`, `parametro`) semánticamente relevantes; `obtenido` solo por lista blanca GAP-051.
- **Índices/rangos** (`IndiceFueraDeRango`, `IndiceTextoFueraDeRango`): datos completos relevantes (p. ej. índice/tamaño o longitud) deben coincidir.
- **Nombre/identificador/ámbito:** `nombre` (y módulo/contexto cuando aplique) relevante.
- **Tipos/lógicos/condición/operando/iteración/rango:** campos tipados u obtenidos según categoría relevantes y deben coincidir.
- **Firma/retorno ausente/límites/archivo/módulo/FFI/ejecución:** datos estructurales expuestos por la categoría relevantes y deben coincidir.
- **No soportado:** no equivalente a error semántico; frente a resultado/causa no comparable → No verificable.

### 9.4 Tratamiento de valores opacos y colecciones
- **Opacos (archivo, celda, clausuras no identificables, etc.):** si no puede demostrarse equivalencia semántica, se clasifican como **No verificable** (no equivalentes por nombre/representación textual). Solo opacos con identidad comprobable y coincidente (p. ej. función por nombre, módulo por ruta canónica coincidente) pueden considerarse equivalentes.
- **Secuencias (`Arreglo`/`Lista`):** la comparación de valores exige misma clase y mismo contenido/orden; clases distintas no se asumen equivalentes a nivel de valor. La tolerancia de representación `Lista`/`Arreglo` se limita al diagnóstico whitelisted (GAP-051) cuando la causa semántica es demostrable.
- **No se implementa identidad completa de clausuras en 04.2.**

### 9.5 Pruebas negativas incorporadas (contrato)
Pruebas unitarias en `prototipo/src/paridad.rs` (`mod tests`) verifican: aritmético vs asignación (diferentes), tipos incompatibles vs condición no lógica (diferentes), misma categoría con subcausa diferente (diferentes), misma categoría/subcausa con datos críticos diferentes (diferentes), diferencia de representación permitida por lista blanca GAP-051 (diagnóstico), diferencia de representación no autorizada (divergencia), resultado correcto vs error (divergencia), error vs capacidad no soportada (no equivalentes/no verificable), y casos de normalización/comparación de valores (opaco no demostrable → no verificable, secuencia de clase distinta → divergente, NaN canónico → equivalente).

### 9.6 Limitaciones conocidas (vigentes)
- Stdout/efectos heterogéneos no comparados homogéneamente por el harness (modelo definido).
- Mapeo dependiente de variantes actuales de `CausaError`/`ErrorVm`; categorías no contempladas o datos no modelados pueden requerir regla explícita.
- Opacos y clausuras sin identidad completa → frecuentemente no verificables.
- Divergencias reales preexistentes de motores no corregidas en este lote.
- GAP-051 permanece **parcialmente resuelto** (sin cierre).

### 9.7 Mapeos de equivalencia semántica explícitos (04.4, TW↔VM)
Aplican **solo** cuando la causa subyacente y los datos relevantes coinciden; en caso contrario sigue la regla estricta:
- `ConversionTextoInvalida` ↔ `ConversionInvalido`: conversión de texto fallida, igualdad en `valor` y `esperado`.
- `IteracionSobreNoColeccion{obtenido}` ↔ `TipoEsperado{esperado="arreglo, lista o texto", encontrado=obtenido}`.
- `RangoTipoInvalido` ↔ `TipoEsperado{esperado="entero", encontrado` indicativo de rango no entero`}`.

Además, para paridad con TW: literales `[...]` se normalizan como `Lista`; condiciones de control booleanas requieren `Logico`; indexación fuera de rango se representa como `AsignacionIncompatible` con `esperado="índice válido"`.
