# ADR-0012 — Iteración: `para cada`, rangos, `romper` y `continuar`

- Estado: **Aprobado** (2026-10-24)
- Fecha: 2026-10-24
- Responsable: proyecto BASILISCO
- Revisión prevista: al completar la implementación del incremento 7.
- Prerequisito: ADR-0011 aprobado (introduce `nulo` y el sistema de tipos gradual;
  la variable de iteración es inmutable dentro del bloque).
- Relacionado: ADR-0007 (bloques y ciclos — `romper`/`continuar` diferidos en §1.5),
  ADR-0008 (funciones), ADR-0011 (sistema de tipos gradual).
- Resuelve brechas: GAP-008 (colecciones — iteración), registro de diferimiento
  de `romper`/`continuar` en ADR-0007 §1.5.

---

## 1. Contexto y requisitos

ADR-0007 §1.5 establece explícitamente: "`romper` y `continuar` no forman parte del
incremento 4 documentado; no deben prometerse hasta especificarlos y probarlos." Este ADR
los especifica y los incorpora junto con una sentencia de iteración sobre colecciones.

El estado actual del intérprete:
- No existe forma de recorrer una `lista` o `arreglo` sin índices manuales y `mientras`.
- `romper` y `continuar` no están definidos.
- El `mientras` tiene un límite de 100 000 iteraciones (ADR-0007 §6, nota de implementación).

Requisitos de este ADR:
- R1: `para cada` sobre `arreglo<T, N>` y `lista<T>`.
- R2: `para cada` sobre rangos numéricos enteros.
- R3: `romper` y `continuar` en `para cada` **y** en `mientras`.
- R4: La variable de iteración es inmutable dentro del bloque.
- R5: La sintaxis es `para cada elemento en coleccion { ... }`.
- R6: `{` en la misma línea que `para` (coherente con ADR-0007 §1.6).
- R7: Sin cambio en el comportamiento existente de `mientras` excepto añadir
  soporte para `romper`/`continuar`.

---

## 2. Opciones consideradas

### O1 — `para elemento en coleccion` (sin `cada`)
Más breve. Pero `para` solo podría confundirse con otras construcciones futuras
(`para siempre`, `para n veces`). La ambigüedad aumenta con una sola palabra clave.

### O2 — `para cada elemento en coleccion` (propuesta seleccionada)
Dos palabras clave. Más verboso pero más legible en español. El precedente de `sino si`
(dos palabras clave) ya existe en ADR-0007. La secuencia `para cada` no es ambigua.
- **Seleccionada**: coherente con el principio de legibilidad del lenguaje (ADR-0003 §1).

### O3 — `foreach elemento en coleccion`
Anglicismo. Contradice ADR-0003 §1 (palabras clave en español).
- **Descartada**.

### O4 — Bucle de índice estilo C: `para i = 0; i < n; i = i + 1`
Familiar para programadores de C/Java. Introduce tres sentencias en la cabecera y
punto y coma, que ADR-0003 §4 no implementa. Añade complejidad sintáctica innecesaria
cuando `para cada` cubre el caso común.
- **Descartada** para este incremento.

---

## 3. Decisión

### 3.1 Sentencia `para cada`

```
"para" "cada" IDENT "en" expresion bloque
```

- `expresion` debe evaluar a un `arreglo`, una `lista` o un `rango` (ver §3.3).
  Cualquier otro tipo → error `IteracionSobreNoColeccion { obtenido }`.
- `IDENT` es el nombre de la variable de iteración. Nace en el ámbito del bloque,
  es **inmutable** (equivalente a `dato`), y vive solo durante ese bloque.
- No se puede asignar a la variable de iteración dentro del bloque:
  `elemento = 5` → error `AsignacionIncompatible { esperado: "variable inmutable" }`.
- El bloque se ejecuta una vez por elemento, en orden de índice creciente (0, 1, 2, …).
- El `{` debe ir en la misma línea que `para` (ADR-0007 §1.6 aplicado por extensión).

Ejemplo:
```
var numeros: lista<entero> = [10, 20, 30]
para cada n en numeros {
    mostrar n
}
```

### 3.2 `romper` y `continuar`

Ambas sentencias son válidas dentro de `para cada` **y** dentro de `mientras`.

- `romper`: termina inmediatamente el ciclo más cercano que lo contiene.
  La ejecución continúa en la primera sentencia después del bloque del ciclo.
- `continuar`: salta el resto del bloque de la iteración actual y pasa a la
  siguiente iteración (en `para cada`) o reevalúa la condición (en `mientras`).
- Fuera de un ciclo → error sintáctico `RomperFueraDeCiclo` /
  `ContinuarFueraDeCiclo` (detectado por el parser).
- El límite de 100 000 iteraciones de `mientras` se mantiene y es independiente
  de `romper`/`continuar`.

```
var total: entero = 0
para cada n en numeros {
    si n == 20 {
        continuar
    }
    si n > 25 {
        romper
    }
    total = total + n
}
```

### 3.3 Rango numérico entero

Se introduce el literal de rango como expresión temporal (no almacenable en variable):

| Sintaxis | Semántica | Ejemplo |
|----------|-----------|---------|
| `inicio..fin` | `[inicio, fin)` — el fin queda excluido | `0..5` → 0, 1, 2, 3, 4 |
| `inicio..=fin` | `[inicio, fin]` — el fin queda incluido | `1..=3` → 1, 2, 3 |

- `inicio` y `fin` deben ser expresiones de tipo `entero`. Cualquier otro tipo →
  error `RangoTipoInvalido { obtenido }`.
- Si `inicio >= fin` en `..`, el rango está vacío (el cuerpo no se ejecuta).
- Si `inicio > fin` en `..=`, el rango está vacío.
- Los rangos con valores negativos son válidos: `-3..=3`.
- Un rango **no** es un valor del tipo `Valor`; es una construcción sintáctica solo
  válida como expresión de `para cada`. No se puede guardar en variable ni pasar como
  argumento a función.

```
para cada i en 0..5 {
    mostrar i
}
# imprime: 0, 1, 2, 3, 4

para cada i en 1..=3 {
    mostrar i
}
# imprime: 1, 2, 3
```

### 3.4 Relación con el límite de iteraciones de `mientras`

El límite de 100 000 iteraciones documentado en ADR-0007 §6 es una **protección del
prototipo**, no una regla normativa del lenguaje. La especificación normativa lo declara
como límite de implementación provisional, revisable con evidencia.

`para cada` **no tiene límite de iteraciones** ya que el número de iteraciones está
acotado por el tamaño finito de la colección o del rango. El límite solo aplica a
`mientras`.

---

## 4. Reglas semánticas

### 4.1 Ámbito de la variable de iteración

La variable `elemento` en `para cada elemento en coleccion { ... }` es local al bloque.
Si existe una variable con el mismo nombre en un ámbito exterior, la variable de
iteración la **sombrea** (coherente con ADR-0006 §1.6 y ADR-0007 §1.6).

### 4.2 Modificación de la colección durante la iteración

La colección referenciada en `para cada` **no puede modificarse durante la iteración**.
Un intento de asignar por índice a la colección iterada durante el bloque produce
error `ModificacionDuranteIteracion`. Esta regla se implementa verificando si la
variable origen es la misma que se itera.

### 4.3 Orden de evaluación

La expresión de la colección/rango se evalúa **una sola vez**, antes de la primera
iteración. Cambios en la variable entre iteraciones no afectan el recorrido.

### 4.4 Nuevos errores (`CausaError`)

```
IteracionSobreNoColeccion { obtenido: &'static str }
RangoTipoInvalido { obtenido: &'static str }
ModificacionDuranteIteracion { variable: String }
```

Errores sintácticos (detectados por el parser, no en ejecución):
```
RomperFueraDeCiclo
ContinuarFueraDeCiclo
```

---

## 5. Gramática ampliada

```ebnf
sentencia = ...
          | iteracion
          | "romper"
          | "continuar" ;

iteracion = "para" , "cada" , IDENT , "en" , expr_iteracion , bloque ;

expr_iteracion = expresion
               | rango ;

rango      = expresion , ".." , expresion        (* exclusivo *)
           | expresion , "..=" , expresion ;     (* inclusivo *)
```

Nuevos tokens léxicos:

```
"para"     →  TipoToken::Para
"cada"     →  TipoToken::Cada
"en"       →  TipoToken::En
"romper"   →  TipoToken::Romper
"continuar"→  TipoToken::Continuar
".."       →  TipoToken::RangoDos
"..="      →  TipoToken::RangoDosCerrado
```

**Nota sobre `en`**: la palabra clave `en` es de uso contextual (solo válida en la
posición de `para cada IDENT en`). Como el lexer convierte todos los identificadores
coincidentes en palabras clave, `en` deja de ser un identificador válido. Si hay código
existente con una variable `en`, se produce un error léxico. [VERIFIED: ningún test ni
ejemplo usa `en` como identificador].

**Nota sobre `..` y `..=`**: el lexer actual no emite `.`. Cuando encuentra `.` dentro
de un número produce un decimal. En posición que no sea número, `.` actualmente causa
error léxico (`carácter inesperado`). Los nuevos tokens `..` y `..=` se producen solo
cuando el punto aparece en posición que no es parte de un número. La regla: si el
carácter anterior es un dígito, `.` sigue siendo parte del literal decimal; si no, `.`
seguido de `.` forma `..` (o `..=` si el siguiente es `=`).

---

## 6. Compatibilidad y migración

- **Sin rotura de código existente** [VERIFIED]: ningún test ni ejemplo usa `para`,
  `cada`, `en`, `romper` ni `continuar` como identificadores.
- **`mientras` existente no cambia** salvo añadir soporte para `romper`/`continuar`.
- **El límite de 100 000 iteraciones se mantiene** en `mientras`.

---

## 7. Criterios de aceptación

### `para cada` sobre colecciones
- Iterar sobre `lista<entero>` de 3 elementos → 3 ejecuciones del cuerpo, en orden.
- Iterar sobre `arreglo<texto, 2>` → 2 ejecuciones.
- Lista vacía → 0 ejecuciones (sin error).
- Variable de iteración inmutable: asignación dentro del bloque → error.
- Colección no válida (`para cada x en 5`) → error `IteracionSobreNoColeccion`.

### Rangos
- `0..5` → itera 0, 1, 2, 3, 4.
- `1..=3` → itera 1, 2, 3.
- `5..5` → rango vacío, cero iteraciones.
- Rango negativo `−2..=2` → itera −2, −1, 0, 1, 2.
- Rango con tipo no entero → error `RangoTipoInvalido`.

### `romper` y `continuar`
- `romper` en `para cada` termina el ciclo; la ejecución continúa después del bloque.
- `continuar` en `para cada` salta al siguiente elemento.
- `romper` en `mientras` termina el ciclo.
- `continuar` en `mientras` reevalúa la condición.
- `romper` fuera de ciclo → error sintáctico.
- `continuar` fuera de ciclo → error sintáctico.
- `romper` dentro de `si` anidado en `para cada` → termina el `para cada`.

### Regresión
- Las 268 pruebas existentes siguen en verde.
- El comportamiento de `mientras` sin `romper`/`continuar` es idéntico al actual.

---

## 8. Archivos a modificar (plan; sin cambios ahora)

| Archivo | Cambio |
|---------|--------|
| `prototipo/src/token.rs` | Añadir `Para`, `Cada`, `En`, `Romper`, `Continuar`, `RangoDos`, `RangoDosCerrado` |
| `prototipo/src/lexer.rs` | Reconocer nuevas palabras clave y tokens `..`/`..=` |
| `prototipo/src/parser.rs` | Sentencia `para cada`; detección de `romper`/`continuar` fuera de ciclo |
| `prototipo/src/evaluador.rs` | Lógica de iteración; señales `Romper`/`Continuar`; nuevas causas de error |
| `prototipo/tests/conformidad_iteracion.rs` | Nueva suite |
| `10-especificacion/gramatica-de-referencia-0.3.md` | Actualizar |

---

## 9. Riesgos y condiciones de revisión

| Riesgo | Probabilidad | Mitigación |
|--------|-------------|------------|
| `en` como palabra clave rompe código con variable `en` | Baja | Verificado: no existe |
| Rangos con `..` confunden al lexer al lado de decimales | Media | Regla de posición clara (§5); prueba léxica dedicada |
| `romper`/`continuar` en ciclos anidados rompen el ciclo equivocado | Baja | Semántica explícita: ciclo más cercano; prueba de anidamiento |

**Condiciones de revisión:**
- Si surge necesidad de acceder al índice durante `para cada`, se puede añadir
  `para cada elemento, indice en coleccion` en un ADR posterior.
- Si surge necesidad de rangos de decimales o reales, se decide en un ADR posterior.
- El límite de 100 000 iteraciones en `mientras` se puede volver configurable o
  eliminar con evidencia de que no es necesario como protección del intérprete.

---

## 10. Lo que este ADR no decide

- Bucle de índice estilo C (`para i = 0; i < n; i = i + 1`).
- Iteradores perezosos / generadores.
- `para cada` con desestructuración (`para cada (clave, valor) en mapa`).
- Rangos como tipo persistente (guardar en variable, pasar como argumento).
- Comprensiones de lista (`[x * 2 para cada x en lista]`).
