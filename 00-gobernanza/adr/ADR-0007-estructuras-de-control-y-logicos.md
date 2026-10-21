# ADR-0007 — Bloques, condiciones, ciclos y tipos lógicos (incremento 4)

- Estado: **Aprobado** (autorización de incremento 4, 2026-10-21). Los cuatro puntos propuestos han sido aceptados explícitamente: `y`/`o` reservados como palabras clave; comparaciones `decimal` vs `real` → error hasta conversión explícita; `{` en la misma línea que `si`/`sino`/`mientras`; `romper`/`continuar` fuera del incremento.
- Alcance: sentencias y operadores nuevos del prototipo; no modifica ADR-0003, ADR-0005 ni
  ADR-0006. **Ejerce la condición de revisión de ADR-0004 §2** ("revisar al añadir operadores
  nuevos"): amplía la tabla de precedencia añadiendo niveles por debajo de los existentes;
  los niveles de ADR-0004 (`( )`, `* / %`, `+ -`) quedan intactos.

## 1. Decisión

### 1.1 Tipo lógico y literales
Nuevo valor `lógico` (booleano) con literales `verdadero` y `falso` (palabras clave). No es un
tipo numérico: **no hay conversión** entre `lógico` y `entero`/`decimal`/`real` en ningún
sentido; cualquier mezcla es error con ubicación. ADR-0005 sigue rigiendo solo el sistema
numérico; este ADR lo extiende con un tipo adicional, sin reabrirlo.

### 1.2 Operadores nuevos
- **Comparaciones**: `==`, `!=`, `<`, `<=`, `>`, `>=` sobre operandos numéricos. Las reglas de
  conversión son **únicamente las implícitas aprobadas en ADR-0005 §1.4** (`entero → decimal`,
  `entero → real`); un par sin conversión aprobada (p. ej. `decimal` vs `real`) → error
  `TiposIncompatibles { izquierdo, derecho }` con ubicación (coherente con ADR-0006 §1.4;
  revisable cuando exista conversión explícita). `==`/`!=` sobre decimales es comparación
  exacta (decimales exactos, ADR-0005). Comparaciones encadenadas (`a < b < c`) → **error
  sintáctico explícito**; evita el confuso `(a < b) < c` entre `lógico` y numérico.
- **Lógicos**: palabras clave `y`, `o` (asociativos a izquierda, **cortocircuito**) y `no`
  (unaria, asociativa a derecha). Operandos **estrictamente `lógico`**; cualquier otro tipo →
  error `OperandoNoLogico { operador, obtenido }` con ubicación.
  - *Precisión de implementación (no altera la decisión)*: por cortocircuito, el operando
    derecho de `y`/`o` **solo se valida si llega a evaluarse**. `falso o 1` es error
    `OperandoNoLogico` (el derecho se evalúa); `verdadero o 1` devuelve `verdadero` sin error
    (el derecho no se evalúa). Esto es consecuencia directa del cortocircuito aprobado, no una
    excepción al requisito de operandos estrictamente lógicos.

### 1.3 Tabla de precedencia (enmienda a ADR-0004 §2, por su condición de revisión)

| Nivel | Operadores | Asociatividad |
|---|---|---|
| 1 (mayor) | `( )` agrupación | — |
| 2 | `*`, `/`, `%` | izquierda |
| 3 | `+`, `-` | izquierda |
| 4 | `==`, `!=`, `<`, `<=`, `>`, `>=` (sin encadenar) | izquierda |
| 5 | `no` | derecha |
| 6 | `y` | izquierda |
| 7 (menor) | `o` | izquierda |

Ejemplos normativos: `no a y b` → `(no a) y b`; `a > 0 o a < -10 y b == 2` →
`(a > 0) o ((a < -10) y (b == 2))`.

### 1.4 Condiciones: `si` / `sino`
La condición se evalúa y **debe resultar `lógico`**; si resulta numérico u otro tipo → error
`CondicionNoLogica { obtenido }` con ubicación (nunca interpretar `0`/`1` ni números como
verdad, consistente con ADR-0005 §1.6: sin coerciones silenciosas ni valores por defecto).

```text
"si" expresión bloque ( "sino" ( "si" expresión bloque | bloque ) )?
```
`else if` se escribe `sino si`.

### 1.5 Ciclos: `mientras`
```text
"mientras" expresión bloque
```
Reevalúa la condición antes de cada iteración; requiere `lógico` estricto (mismo error
`CondicionNoLogica`). El incremento 4 **no incluye `romper` ni `continuar`** (diferidos: la
única salida es agotar la condición).

### 1.6 Bloques y ámbitos
```text
bloque → "{" sentencia* "}"
```
- El bloque es también una sentencia (`| bloque` en la gramática de sentencias).
- Ejecuta lo previsto en ADR-0006 §1.6: al entrar **empuja un ámbito**, al salir lo retira y
  descarta sus variables. La lectura va de dentro hacia fuera; el **sombreado está permitido**
  y la duplicidad solo falla dentro del mismo nivel (confirma la recomendación de ADR-0006).
- El `{` de `si`/`sino`/`mientras` **debe ir en la misma línea** que la palabra clave
  (regla provisional, diagnóstico inmediato: "se esperaba '{' en la misma línea"; condición de
  revisión por usabilidad NEX-001).
- Los bloques **no producen valor**: el valor de `ejecutar_programa` sigue siendo la última
  sentencia de expresión del nivel superior. La "expresión-bloque" con valor propio se difiere.

### 1.7 Errores nuevos (extensión de `CausaError`)
- `CondicionNoLogica { obtenido }`
- `OperandoNoLogico { operador, obtenido }`
- `TiposIncompatibles { izquierdo, derecho }` (comparaciones sin conversión implícita aprobada)
Mismo tratamiento que los anteriores: mensaje derivado, ubicación real, cero valores por
defecto.

## 2. Gramática añadida

```text
sentencia → "sea" IDENT "=" expresión
          | IDENT "=" expresión
          | "si" expresión bloque ( "sino" ( "si" expresión bloque | bloque ) )?
          | "mientras" expresión bloque
          | bloque
          | expresión
bloque    → "{" sentencia* "}"
```

Léxico nuevo: palabras clave `si`, `sino`, `mientras`, `y`, `o`, `no`, `verdadero`, `falso`
(minúsculas exactas, como `sea`); operadores `==`, `!=`, `<`, `<=`, `>`, `>=` (el lexer
distingue `=` de `==` y `!` de `!=` por el carácter siguiente).

## 3. Alternativas descartadas
- **`&&` / `||` / `!`**: símbolos internacionales; se prefieren palabras para NEX-001
  (legibilidad en español: `x > 0 y x < 10`).
- **`Y` / `O` / `NO` en mayúsculas** (estilo Pseint): sensibilidad a mayúsculas inconsistente
  con las palabras clave minúsculas ya aprobadas (`sea`, `funcion`).
- **Condición numérica estilo C** (cualquier número = verdad): contradice los tipos estrictos
  y la prohibición de coerciones silenciosas (ADR-0005 §1.6).
- **`romper`/`continuar` en el incremento 4**: ampliaría el alcance; se propone aparte.
- **Bloque como expresión con valor**: pendiente de decisión propia sobre el valor de bloque.

## 4. Consecuencias
- **Impacto en pruebas existentes**: al implementar, `y` pasa a ser palabra clave; las pruebas
  actuales que usan `y` como identificador (p. ej. `sea y = 2`) deben migrarse a otro nombre.
  Se registrará en el informe de implementación; ninguna decisión aprobada cambia.
- Reservados quedan: `sea`, `funcion`, `retornar`, `mostrar` (hoy) más `si`, `sino`,
  `mientras`, `y`, `o`, `no`, `verdadero`, `falso` (este ADR).
- El parser crece con un nivel de precedencia nuevo (4 niveles de comparación/lógicos por
  debajo de la aritmética existente) y sentencias compuestas; `parsear_expresion` y la
  semántica numérica aprobada no cambian.

## 5. Puntos que requieren confirmación al aprobar
1. **Reservar `y`/`o`** como operadores lógicos (renumbra el espacio de identificadores).
2. **`decimal` vs `real` en comparaciones → error** hasta que exista conversión explícita.
3. **`{` en la misma línea** obligatorio para `si`/`sino`/`mientras`.
4. **Sin `romper`/`continuar`** en este incremento.

Los cuatro puntos fueron aceptados explícitamente por la persona responsable del proyecto
(2026-10-21); con ello el ADR pasa de *Propuesta* a **Aprobado** y el incremento 4 queda
autorizado para implementación.

## 6. Notas de implementación (no normativas)
Estas notas registran decisiones de implementación que **no modifican** la semántica aprobada
y que deben ser visibles para quien use el lenguaje:
- **Guard de iteraciones**: `mientras` aborta con `LimiteIteraciones { limite }` al superar
  100 000 iteraciones, para que un ciclo infinito termine en error en vez de colgar el proceso.
  Es una protección del anfitrión, no una forma de salir del ciclo: el ADR §1.5 sigue siendo
  la única salida semántica. Constante pública: `LIMITE_ITERACIONES`.
- **Nombre de la causa de error**: la causa se llama `TiposIncompatibles { izquierda, derecha }`,
  tal como figura en §1.7 del ADR.
- **`sino` en la línea siguiente**: se admite que `sino` aparezca en la línea posterior al `}`
  de la rama `si` (el `{` de `sino` sí debe ir en la misma línea que `sino`).
- **Comparación entre lógicos**: `==`/`!=` sobre operandos lógicos es error
  `TiposIncompatibles`; el ADR §1.2 solo autoriza comparaciones sobre operandos numéricos.
- **Ámbito por iteración**: cada iteración de `mientras` ejecuta el cuerpo en un ámbito nuevo
  (consecuencia de §1.6: el bloque empuja y retira un ámbito al entrar y salir).

## 7. Aceptación
Aceptado con las notas de §6 incorporadas. Verificación registrada en
`07-validacion/casos-de-prueba-y-puertas.md` (evidencia del incremento 4).

