# Casos de prueba y puertas de calidad
## Casos iniciales
Expresiones válidas e inválidas; precedencia; paréntesis; números fuera de rango; nombres no definidos; duplicación y sombreado; tipos incompatibles; cadenas sin cerrar; Unicode; archivos vacíos y entradas muy largas.

El analizador no debe colapsar ante entradas arbitrarias. Los errores deben indicar ubicación cuando sea posible.

## Puertas
A. Diseño: gramática, semántica, tipos y ADR revisadas.
B. Prototipo: lexer/parser probados y expresión sencilla evaluada.
C. Núcleo: variables, funciones y control de flujo implementados con conformidad.
D. Experimental: compatibilidad, límites, seguridad e instalación documentados.
E. Estable: política de versiones, migración, regresión y documentación.

Documentos por sí solos no superan una puerta de calidad.

## Evidencia de ejecución (incremento 2, 2026-10-20)

Comandos ejecutados en `prototipo/`:

- `cargo test` → **103 pruebas, 0 fallos** (59 unidad + 29 conformidad + 15 integración).
- `cargo clippy --all-targets` → **0 advertencias, 0 errores**.
- `cargo run` → demo funcional con valores y errores correctos.

Suites nuevas:

- `tests/integracion.rs` (15): flujo completo cadena → léxico → sintaxis → evaluación;
  errores con ubicación real (línea/columna) y motivo exacto; contrato "sin valor por defecto".
- `tests/conformidad.rs` (29): organizadas por secciones del ADR-0005 (§1.1 enteros,
  §1.2 decimales, §1.3 reales, §1.4 conversiones, §1.5 literales, §1.6 errores) con
  casos límite explícitos: límites de i64, half-even (empates exactos y sticky),
  desbordamiento i128 intermedio, rango DECIMAL(28,9) en extremos.

Defectos encontrados y corregidos por las pruebas de este incremento (todos cubiertos):

1. Redondeo half-even de la división decimal nunca se activaba (comparaba |resto| con 2|b|)
   → la división truncaba silenciosamente.
2. `mul_dec` devolvía el múltiplo de 10^9 redondeado en vez del cociente: toda
   multiplicación decimal inflaba el resultado en un factor 10^9.
3. Literales decimales con >9 decimales se truncaban en vez de redondear half-even.
4. El límite de rango aceptaba |mantisa| = 10^28 (20 dígitos enteros, fuera de DECIMAL(28,9)).
5. La ubicación de los errores de ejecución era siempre `[1:1]` (el AST no guardaba posiciones).

## Evidencia de ejecución (incremento 3, 2026-10-21)

Comandos ejecutados en `prototipo/`:

- `cargo test` → **132 pruebas, 0 fallos** (74 unidad + 29 conformidad + 15 integración
  + 14 variables).
- `cargo clippy --all-targets` → **0 advertencias, 0 errores**.
- `cargo run` → programa de aceptación ejecutándose:
  `sea precio = 10.5 / sea total = precio * 3 / total = total - 1.5 / total` → `30.0`;
  error real con ubicación: `error de ejecución [2:5]: variable no definida: y`.

Reglas del incremento: ADR-0006 (declaraciones, asignación, mutabilidad, ámbitos).

Suites nuevas:

- Tests unitarios en lexer (`sea`, `=`), parser (sentencias, límites de `parsear_expresion`)
  y evaluador (entorno, causas tipadas, ubicaciones).
- `tests/variables.rs` (14): programa de aceptación completo, conversiones implícitas en
  asignación (ADR-0005 §1.4 reutilizado), los tres errores tipados del ADR-0006 con
  ubicación exacta, ausencia de declaración implícita, y la semántica numérica aprobada
  funcionando a través de variables sin cambios.

## Evidencia de ejecución (incremento 4, 2026-10-21)

Comandos ejecutados en `prototipo/`:

- `cargo test` → **211 pruebas, 0 fallos** (95 unidad + 29 conformidad numérica +
  39 conformidad de control + 19 integración de control + 15 integración + 14 variables).
- `cargo clippy --all-targets` → **0 advertencias, 0 errores**.
- `cargo run` → demo de control de flujo: programa de aceptación
  `sea total = 0 / sea contador = 0 / mientras contador < 5 { contador = contador + 1 /
  si contador % 2 == 0 { total = total + contador } sino { total = total + 1 } } / total` →
  `9`; literales lógicos `1 < 2` → `verdadero`, `verdadero y falso` → `falso`,
  `no falso` → `verdadero`; error tipado real: `si 5 { ... }` →
  `error de ejecución [1:4]: la condición debe ser lógica, se obtuvo entero`.

Reglas del incremento: ADR-0007 (bloques, condiciones, ciclos y tipos lógicos), sobre
ADR-0005 (tipos numéricos) y ADR-0006 (variables y ámbitos).

Suites nuevas:

- Tests unitarios en parser (AST de `si`/`sino`/`mientras`/bloque, precedencia reflejada en
  el árbol, llave en la misma línea, `sino` huérfano, comparaciones encadenadas, palabras
  clave no reutilizables como identificadores) y evaluador (comparaciones, operadores
  lógicos, cortocircuito, tipos de condición, ámbitos, guard de iteraciones, errores dentro
  de bloques con ubicación).
- `tests/conformidad_control.rs` (39): organizada por secciones del ADR-0007 (§1.1 tipo
  lógico, §1.2 comparaciones y lógicos, §1.3 precedencia, §1.4 `si`/`sino`, §1.5 ciclos,
  §1.6 bloques y ámbitos, §1.7 errores) más gramática y léxico nuevos.
- `tests/control_flujo.rs` (19): flujo completo con programas que combinan variables,
  condiciones, ciclos y anidamiento; errores tipados con ubicación exacta; compatibilidad
  con los programas del incremento 3.

Defectos encontrados y corregidos por las pruebas de este incremento (todos cubiertos):

1. `sino si` terminaba la sentencia **dos veces** (la sentencia `si` interna consumía el fin
   de línea y la externa volvía a consumirlo) → cualquier programa con `sino si` fallaba con
   «texto inesperado después de la sentencia». Causa raíz: `sentencia_si` consumía el
   terminador; ahora la terminación la decide el llamador y `sentencia_si` no la consume.
2. La regla de «`{` en la misma línea» (ADR-0007 §1.6) se cumplía **por efecto secundario**
   del salto de línea, sin comprobación explícita: una condición multilínea podía eludirla.
   Ahora hay comparación explícita de la línea del token `{` con la de la palabra clave.
3. `sino` seguido de salto de línea daba un mensaje genérico («se esperaba '{' o 'si' tras
   'sino'») en vez del diagnóstico de misma línea previsto por el ADR.
4. La causa de error se llamaba `TiposIncompatiblesComparacion`, nombre distinto del
   `TiposIncompatibles` fijado en el ADR §1.7 → reconciliada con la norma.
5. `y`/`o` pasaron a ser palabras clave: las pruebas que usaban `y` como identificador
   (`sea y = 2`, `x + y`) fallaban → migradas a nombres válidos (`z`, `zeta`), según lo
   previsto en el ADR §4.

Cambio de comportamiento registrado (no es defecto, es consecuencia de una decisión
aprobada): `y`/`o` son palabras clave, por lo que `sea y = 2` es ahora error léxico/sintáctico.

