# Mutabilidad, inicialización y sombreado — BASILISCO Core 0.3

**Estado:** propuesta normativa para revisión (sincronizada con el prototipo).  
**Fuentes:** ADR-0006, ADR-0007, ADR-0008, `prototipo/src/parser.rs`, `prototipo/src/evaluador.rs`.  
**Última sincronización:** 2026-10-25, commit `4b23ac2`.

## 1. Declaraciones y mutabilidad
El parser acepta cuatro formas de declaración con las siguientes reglas:

| Palabra clave | Anotación de tipo | Mutable | `es_const` | Uso | Notas |
|---|---|---|---|---|---|
| `sea` | Opcional | `true` | `false` | `sea x = 5` | Forma legacy/compacta. Sin anotación de tipo obligatoria. |
| `var` | **Obligatoria** (`:`) | `true` | `false` | `var x: entero = 5` | Variable mutable con tipo declarado. |
| `dato` | **Obligatoria** (`:`) | `false` | `false` | `dato pi: real = 3.14` | Variable inmutable con tipo declarado. |
| `const` | **Obligatoria** (`:`) | `false` | `true` | `const MAX: entero = 100` | Constante. Semántica de "constante de compilación" aún en refinamiento. |

**Reglas:**
- Para `var`/`dato`/`const` la anotación `: tipo` es obligatoria. El parser rechaza su ausencia.
- Para `sea` la anotación es opcional. Si se omite, el tipo se infiere/resuelve en evaluación.
- Todas las declaraciones requieren `=` y una expresión inicializadora.
- Los nombres de variables siguen las reglas de identificador ASCII (`[A-Za-z_][A-Za-z0-9_]*`).
- `y` y `o` son palabras clave y **no pueden** usarse como identificadores.

## 2. Ámbitos (scoping)
Basilisco utiliza **ámbitos léxicos**.

| Contexto | Comportamiento |
|---|---|
| Nivel superior (programa) | Ámbito global. |
| Bloque `{ ... }` | Introduce un nuevo ámbito. Las variables declaradas dentro no son visibles fuera. |
| `si` / `sino` / `sino si` | Las ramas (`rama_si`, `rama_sino`) son bloques con sus propios ámbitos. |
| `mientras` | El cuerpo es un bloque con su propio ámbito. |
| `funcion` | Introduce un nuevo ámbito. Los parámetros se declaran en el ámbito de la función. **No se permiten funciones anidadas**. |
| Llamadas a función | Cada llamada crea un entorno nuevo para esa invocación (marco de ejecución). |

## 3. Inicialización antes del uso
- **Toda variable debe estar inicializada al declararse.** La sintaxis obliga `=` con una expresión, por lo que la declaración siempre inicializa.
- **Uso de variable no declarada:** produce error `variable no definida` (`BAS-3001`) con ubicación.
- **Acceso antes de inicialización:** al no existir sintaxis para declarar sin inicializar, este caso no es posible con la gramática actual. Si en el futuro se añade, deberá ser un error explícito.
- **Parámetros de función:** se inicializan con los valores de los argumentos al momento de la llamada.

## 4. Declaraciones duplicadas
- **En el mismo ámbito:** declarar una variable con el mismo nombre que otra ya existente en ese ámbito es un error. Se detecta en evaluación como `declaración duplicada` (`BAS-3002`).
- **En ámbitos distintos:** está permitido **sombrear** (ver sección 5). La variable del ámbito más interno oculta a la del ámbito externo mientras está en alcance.

## 5. Sombreado (shadowing)
- **Permitido.** Una declaración en un ámbito interno puede reutilizar el nombre de una variable del ámbito externo.
- **Alcance:** el sombreado solo afecta al ámbito interno y a sus descendientes. Al salir del ámbito interno, la variable externa vuelve a estar visible.
- **Resolución:** la búsqueda de identificadores es **léxica de adentro hacia afuera** (se encuentra primero en el ámbito más interno).
- **Mutabilidad independiente:** la variable sombreada es una entidad distinta con sus propios metadatos (`mutable`, `tipo_declarado`, `es_const`).

## 6. Asignación y mutabilidad
- **Solo variables declaradas:** asignar a un identificador no declarado produce `variable no definida`.
- **Solo variables mutables:** asignar a `dato` o `const` produce `asignación incompatible` con mensaje `"variable inmutable (dato/const)"` (`BAS-3003`).
- **Comprobación de tipo en asignación:** si la variable tiene `tipo_declarado` (declaraciones `var`/`dato`/`const`), la evaluación comprueba compatibilidad de tipos. Las incompatibilidades producen `asignación incompatible: se esperaba <esperado> y se obtuvo <obtenido>`.
- **`sea` sin anotación:** el comportamiento de tipado en asignación posterior sigue las reglas del evaluador actual (se valida cuando hay anotación de tipo).

## 7. Parámetros de función
- **Declarados en el ámbito de la función:** cada parámetro se añade al entorno de la invocación.
- **Mutables por defecto:** los parámetros se tratan como mutables (`mutable: true`) dentro de la función (ver `evaluador.rs`), similar a `var`.
- **Sombreado permitido:** un parámetro puede sombrear una variable del ámbito externo.
- **Número de argumentos:** debe coincidir exactamente con el número de parámetros. Si no coincide, se produce `firma inválida` (`BAS-3013`).

## 8. Casos especiales y limitaciones
| Caso | Estado | Notas |
|---|---|---|
| Asignación a elementos de arreglo/lista (`a[0] = 5`) | **No implementado** | Parser/evaluador aún no soportan asignación a `Indice`. Pendiente en GAP-008. |
| `const` como constante de compilación con evaluación en tiempo de compilación | **Parcial** | Marcado con `es_const`, pero las garantías formales aún no están completas. |
| Inicialización cíclica | **No especificado** | No hay reglas formales aún; evitar en código actual. |
| Captura de variables en cierres | **No implementado** | Las funciones no son cierres. Funciones anidadas ya están prohibidas. |
| `retornar` fuera de función | **Pendiente** | Debe producir error con código `BAS-3021` (ver catálogo). |

## 9. Reglas normativas (resumen)
1. **Declaración obliga inicialización.**
2. **Ámbitos léxicos con sombreado permitido.**
3. **Búsqueda de adentro hacia afuera.**
4. **Inmutabilidad se respeta en asignación.** `dato`/`const` no son reasignables.
5. **`var`/`dato`/`const` requieren anotación de tipo.** `sea` es opcional.
6. **Declaración duplicada en mismo ámbito = error.**
7. **Variable no declarada = error.**
8. **Parámetros son mutables dentro de la función.**

## 10. Estado de cierre (GAP-006)
- **Implementado:** declaraciones con `sea`/`var`/`dato`/`const`, mutabilidad, sombreado léxico, detección de duplicados, inmutabilidad en asignación, parámetros mutables.
- **Pendiente de formalizar:** garantías completas de `const`, errores con códigos estables para inicialización/asignación inmutable, y reglas más estrictas si se añaden nuevas formas de declaración.
- **Acción para cerrar:** añadir tests de conformidad específicos para sombreado, inicialización, duplicados e inmutabilidad; asignar códigos `BAS-3021`/`BAS-3022`/`BAS-3023` cuando se formalicen.
