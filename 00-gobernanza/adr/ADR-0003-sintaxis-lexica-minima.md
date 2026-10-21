# ADR-0003 — Sintaxis léxica mínima del prototipo

- Estado: Aprobado
- Fecha: 2026-10-20
- Responsable: por asignar
- Revisión prevista: antes de cerrar la Puerta B (prototipo vertical)
- Relacionado: ADR-0002; `02-lenguaje/sintaxis-y-convenciones.md`; `02-lenguaje/especificacion-lexica-minima.md`

## Contexto y requisitos
Con el anfitrión aprobado (ADR-0002), el siguiente bloqueo antes del lexer es cerrar las preguntas
de `02-lenguaje/sintaxis-y-convenciones.md` lo suficiente para tokenizar el prototipo sin supuestos
ocultos.

**Principio rector acordado:** las restricciones iniciales deben reducir complejidad, no convertirse
por accidente en limitaciones permanentes del lenguaje. Toda restricción de este ADR se declara
provisional y lleva condición de revisión.

## Decisiones propuestas

### 1. Palabras clave: exclusivamente español
Opciones: solo español / español + alias en inglés / solo inglés.
**Propuesto: solo español** (`funcion`, `retornar`, `mostrar`, …). Coherente con NEX-001; los alias
en inglés duplican gramática y diagnósticos sin aportar al prototipo.

### 2. Identificadores: ASCII provisional, con reglas explícitas
- Conjunto: `[A-Za-z_][A-Za-z0-9_]*` — letras, dígitos y guion bajo; **no pueden empezar por dígito**.
- Válidos: `numero`, `total_ventas`, `contador2`.
- **Provisional, no permanente.** ASCII simplifica el lexer y evita por ahora la pareja `año`/`ano`.
  Pero ASCII no elimina todas las confusiones visuales (`l` vs `I`): la salvaguarda completa
  (identificadores Unicode + normalización NFC + reglas anti-confusión) se estudia en un ADR
  futuro. Aquí se **difiere**, no se prohíbe.

### 3. Final de sentencia: salto de línea como terminador contextual
Opciones: (a) `;` obligatorio; (b) salto de línea siempre terminador; (c) terminador contextual.
**Propuesto: (c).** Regla:
- El fin de línea termina la sentencia cuando la expresión está completa según la gramática.
- El fin de línea **no** termina la sentencia si:
  1. está dentro de un grupo abierto `(` o `[`; o
  2. la expresión queda incompleta porque la línea termina en un operador que requiere operando
     derecho (p. ej. `total = 10 +`).
- Esto resuelve la ambigüedad de
  ```
  total = 10 +
      5
  ```
  → una sola sentencia.

### 4. Punto y coma: no implementado en el prototipo
Ni obligatorio ni opcional. El lexer y el parser del prototipo **no** soportan `;`. Se reevaluará
solo con evidencia de necesidad real (p. ej. múltiples sentencias en una línea).

### 5. Unicode en cadenas de texto: permitido desde ya
Los identificadores son ASCII, pero las **cadenas de texto** admiten cualquier carácter Unicode.
Un lenguaje orientado al español debe poder contener `ñ`, tildes y acentos dentro de cadenas sin
arrastrar el problema de los identificadores. El archivo se decodifica como UTF-8.

## Especificación léxica mínima (borrador asociado)
Comentarios, cadenas, espacios/tabulaciones, números y reconocimiento exacto de fin de línea se
detallan en `02-lenguaje/especificacion-lexica-minima.md` (estado: Propuesto, mismo ciclo de
aprobación que este ADR).

## Consecuencias
- El lexer del prototipo se implementará contra estas reglas y contra ese borrador léxico.
- Nada de esto es normativo hasta que este ADR pase a *Aprobado*.
- Restricciones provisionales registradas: identificadores ASCII, sin `;`.

## Condiciones de revisión
- Revisar los identificadores ASCII al empezar la biblioteca estándar (aparecerán nombres con ñ/tildes deseados).
- Revisar `;` solo si surge necesidad real de múltiples sentencias por línea.
- Revisar la regla contextual de saltos de línea con pruebas de usabilidad (NEX-001).

## Aceptación
Aprobado por el propietario el 2026-10-20. Las decisiones 1-5 quedan cerradas para el prototipo;
el borrador `02-lenguaje/especificacion-lexica-minima.md` queda aprobado junto con este ADR y sirve
de contrato de implementación del lexer.
