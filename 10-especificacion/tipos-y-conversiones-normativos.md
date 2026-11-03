# Tipos y conversiones normativos — BASILISCO Core 0.3

**Estado:** propuesta normativa para revisión (sincronizada con el prototipo tras el adelanto).  
**Fuentes:** ADR-0005, ADR-0007, ADR-0011, `prototipo/src/valor.rs`, `prototipo/src/evaluador.rs`.  
**Última sincronización:** 2026-10-25, commit `4b23ac2`.

## 1. Tipos del núcleo

| Tipo | Representación interna | Ámbito / notas | Estado |
|---|---|---|---|
| `entero` | `i64` con signo | Literales sin punto/exponente. Desbordamiento produce error (`D2`). | Normativo |
| `decimal` | `i128` escalonado, escala fija 9 (`DECIMAL(28,9)`) | Literales con punto decimal. Redondeo *half-even* (banquero). Exceso de precisión redondea; exceso de rango produce error (`D5`). | Normativo |
| `real` | `f64` (coma flotante binaria) | Literales con exponente (`1e10`, `2.5e-3`). Errores de división/conversión según casos. | Normativo |
| `lógico` | `bool` (`verdadero`/`falso`) | **No numérico**. No participa en aritmética ni se convierte con numéricos. `y`/`o` con cortocircuito, `no` unario. | Normativo (ADR-0007) |
| `texto` | `String` UTF-8 | Cadenas entre comillas dobles. Soporta Unicode. Escapes `\\`, `\"`, `\n`, `\t`. Concatenación con `+` permite conversión implícita del otro operando a su representación textual. | Implementado (prototipo); pendiente formalización completa (GAP-007) |
| `arreglo<T, N>` | `Vec<Valor>` de tamaño fijo `N` | Anotación `arreglo<T, N>`. Literal `[ ... ]`. Indexación `expr[expr]` de lectura. Mutación de elementos, iteración, longitud y anidamiento aún parciales. | Parcial (ADR-0011, GAP-008) |
| `lista<T>` | `Vec<Valor>` dinámica | Anotación `lista<T>`. Literal `[ ... ]`. Indexación de lectura parcial. Ver GAP-008. | Parcial (ADR-0011, GAP-008) |
| `nulo` | Valor especial `Nulo` | Distinto de `falso`, `0`, `""`. Presente en el valor; semántica completa pendiente de cierre (ADR-0011 §3.5). | Parcial |
| `archivo` | `Arc<Mutex<ArchivoInterno>>` opaco | Para E/S (ADR-0013). No constructible directamente desde BASILISCO. | Parcial (ADR-0013) |

## 2. Clasificación de literales numéricos
Según ADR-0005 §1.5:

| Forma | Tipo resultante | Notas |
|---|---|---|
| `123`, `-123` (signo como literal compuesto cuando es posible) | `entero` (`i64`) | Si el valor absoluto no cabe en `i64`, clasificación/construcción produce error. |
| `123.45`, `0.1` | `decimal` (`DECIMAL(28,9)`) | Requiere dígitos a ambos lados del punto. Exceso de decimales (>9) se redondea *half-even*. |
| `1e10`, `2.5e-3`, `1E+5` | `real` (`f64`) | Requiere exponente válido con dígitos (signo opcional). |

**Regla del signo:** el `-` unario es un operador aritmético aplicable a expresiones; cuando precede inmediatamente a un literal numérico, el parser intenta construir el literal negativo directamente (necesario para `i64::MIN`).

## 3. Conversiones

### 3.1 Conversiones implícitas (sin pérdida)
Solo se permiten las siguientes:

| Desde | Hacia | Cuándo | Notas |
|---|---|---|---|
| `entero` | `decimal` | En operaciones aritméticas binarias mixtas `entero` + `decimal`, `decimal` + `entero`, etc. | Sin pérdida. |
| `entero` | `real` | En operaciones aritméticas binarias mixtas `entero` + `real`, `real` + `entero`, etc. | Conversión numérica. |

### 3.2 Conversiones prohibidas / no implícitas
| Combinación | Resultado | Notas |
|---|---|---|
| `decimal` + `real` / `real` + `decimal` | **Error** (`ConversionIlegal` / tipos incompatibles según contexto) | No está permitida. Debe hacerse explícita si se desea (aún sin función de conversión normativa). |
| Cualquier numérico + `lógico` | **Error** (`TiposIncompatibles`) | `lógico` no es numérico (ADR-0007). |
| `lógico` en contexto aritmético | **Error** | No hay coerción a entero/0/1. |
| Conversión de `lógico` a numérico o viceversa | **Prohibida** | Sin conversión implícita. |

### 3.3 Conversión en concatenación de texto
| Operación | Resultado | Notas |
|---|---|---|
| `texto + texto` | Concatenación directa | Soportado. |
| `texto + T` donde `T` no es `texto` | `texto` | El operando se convierte a su representación textual mediante `Display`. |
| `T + texto` donde `T` no es `texto` | `texto` | Igual que el caso anterior. |

## 4. Aritmética

| Operador | Tipos válidos | Tipo resultado | Reglas |
|---|---|---|---|
| `+` | `entero`/`decimal`/`real` con conversiones implícitas permitidas; `texto` con conversión implícita del otro lado | Mismo tipo numérico tras conversión, o `texto` en caso de concatenación | Desbordamiento entero → error. Decimal sigue reglas de escala 9. |
| `-` | Igual que `+` (numéricos) | Igual que operandos tras conversión | Desbordamiento entero → error. |
| `*` | Igual que `+` (numéricos) | Igual que operandos tras conversión | Decimal: multiplicación con redondeo *half-even* a escala 9. Desbordamiento entero → error. |
| `/` | Numéricos con conversiones permitidas | `entero/entero` → **división entera truncada hacia cero** (resultado `entero`). `decimal/decimal`, `real/real`, mixtos permitidos → división numérica (resultado `decimal` o `real` según tipos). | **División por cero** → error en todos los tipos numéricos. `i64::MIN / -1` → error por desbordamiento entero. `decimal`: redondeo *half-even*. |
| `%` | **Solo `entero % entero`** | `entero` | **Resto truncado**, signo del dividendo. `i64::MIN % -1` → `0` (caso especial). Divisor cero → error. Otros tipos → error de conversión/incompatibilidad. |
| `-` unario | Numéricos | Mismo tipo | Aplica negación aritmética. |

## 5. Comparaciones
Según ADR-0007 §1.3:

| Operador | Reglas | Notas |
|---|---|---|
| `==`, `!=` | Requieren tipos comparables. Igualdad exacta entre valores. | Sin conversión implícita entre tipos distintos (salvo casos ya definidos). |
| `<`, `<=`, `>`, `>=` | Comparación de orden. | **No se permiten comparaciones encadenadas** (ej. `1 < x < 10` es error sintáctico). |
| `decimal` vs `real` | **No permitidas** | Prohibidas sin conversión explícita. |
| Numéricos vs `lógico`/`texto`/`nulo`/colecciones | **No permitidas** | Sin coerción implícita. |

## 6. Lógica y control
Según ADR-0007 §1.4:

| Operador/constructo | Reglas |
|---|---|
| `no` | Negación lógica unaria. Requiere operando de tipo `lógico`. |
| `y`, `o` | Lógicos binarios con **cortocircuito** obligatorio (evaluación izquierda a derecha). Ambos operandos deben ser `lógico`. |
| `si`/`mientras` | La condición **debe** ser de tipo `lógico`. No se permite convertir números/cadenas/valores ausentes a verdadero/falso implícitamente. |

## 7. Rangos, límites y errores numéricos

| Situación | Comportamiento | Código/motivo |
|---|---|---|
| Desbordamiento en `+ - *` entero | Error de ejecución | `DesbordamientoEntero` |
| División por cero (`entero`/`decimal`/`real`) | Error de ejecución | `DivisionPorCero` |
| `i64::MIN / -1` | Error de ejecución | `DesbordamientoEntero` |
| `i64::MIN % -1` | `0` (valor correcto) | Caso especial |
| Exceso de rango decimal (`DECIMAL(28,9)`) | Error de ejecución | `ExcesoRangoDecimal` |
| Exceso de precisión decimal (>9 decimales) | Redondeo *half-even* | Sin error |
| `decimal` + `real` | Error de ejecución | `ConversionIlegal` / tipos incompatibles |
| Comparaciones encadenadas | **Error sintáctico** | Parser |
| Mezcla numérico/lógico | **Error de ejecución** | `TiposIncompatibles` / `OperandoNoLogico` |
| Límite de iteraciones `mientras` | Error de ejecución (protección) | `LimiteIteraciones` (límite configurable en prototipo: `100000`) |

## 8. Reglas de ámbito y mutabilidad (estado actual)

| Constructo | Mutabilidad | Notas |
|---|---|---|
| `sea nombre = expr` | Mutable | Anotación de tipo opcional. |
| `var nombre: tipo = expr` | Mutable | Anotación de tipo **obligatoria**. |
| `dato nombre: tipo = expr` | Inmutable | Anotación de tipo **obligatoria**. |
| `const NOMBRE: tipo = expr` | Inmutable/constante | Anotación de tipo **obligatoria**. Semántica de `const` como constante de compilación aún en refinamiento. |

**Pendiente (GAP-006):** reglas formales de inicialización antes del uso, declaraciones duplicadas en mismo ámbito, sombreado, y garantías de inmutabilidad.

## 9. Notas de implementación
- La fuente de verdad para el comportamiento numérico es `prototipo/src/valor.rs`. Cualquier discrepancia entre este documento y el código debe resolverse a favor de la implementación verificada por tests hasta cerrar la norma.
- Los tests de conformidad relevantes: `prototipo/tests/conformidad.rs`, `conformidad_arreglos_listas.rs`, `conformidad_texto.rs`, `conformidad_control.rs`, `conformidad_funciones.rs`.
- Este documento debe mantenerse sincronizado con ADR-0005, ADR-0007 y ADR-0011.
