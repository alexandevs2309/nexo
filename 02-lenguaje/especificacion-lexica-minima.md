# Especificación léxica mínima — borrador para el prototipo

Estado: Propuesto (pendiente de aprobación junto con ADR-0003).
Alcance: solo lo necesario para tokenizar el prototipo vertical (Fase 2). **No es gramática
normativa.**

## Codificación de archivo
- Fuente: texto **UTF-8**. Un archivo que no sea UTF-8 produce un error claro; no se adivina la codificación.
- BOM UTF-8 al inicio: se ignora si está presente.

## Espacios y tabulaciones
- Espacio (`U+0020`), tabulación (`U+009`) y el `\r` de un CRLF no son significativos entre tokens.
- No hay indentación significativa: la estructura de bloques usa llaves `{ }`.

## Reconocimiento de fin de línea
- Nueva línea = `\n` (LF) o `\r\n` (CRLF). Un `\r` suelto no se soporta.
- Regla de terminación contextual:
  1. Dentro de un grupo abierto `(` o `[`, el salto de línea no termina nada.
  2. Si la expresión queda incompleta (la línea termina en operador que requiere operando derecho:
     `+ - * / == < > …`), el salto de línea no termina la sentencia.
  3. En cualquier otro caso, el salto de línea termina la sentencia.
- Ejemplo resuelto:
  ```
  total = 10 +
      5
  ```
  → una sola sentencia: `total = 10 + 5`.

## Comentarios
- Comentario de línea: `#` hasta el final de la línea. No hay comentarios multi-línea en el prototipo.
- Un `#` dentro de una cadena no es comentario.
- Ejemplo: `total = 5  # impuesto`

## Cadenas de texto
- Delimitador: comilla doble. Ejemplo: `"hola"`.
- Escapes soportados: `\\`, `\"`, `\n`, `\t`. Cualquier otro `\x` produce error léxico con ubicación.
- Comilla simple `'` no soportada en el prototipo.
- Cadenas multi-línea no soportadas.
- Contenido: cualquier carácter Unicode (incluidos `ñ` y tildes).

## Números (suficiente para `10 + 5`)
- Enteros: `123`.
- Decimales: `123.45` — requiere dígito antes y después del punto; un solo punto.
- Sin exponentes, literales binarios/hexadecimales ni separadores de miles en el prototipo.
- El rango numérico se valida en la fase de tipos; el lexer solo tokeniza.

## Errores léxicos
- Todo error léxico lleva ubicación (línea, columna) y mensaje en español.
- El lexer no descarta caracteres inválidos en silencio (contrato de `gramatica-lexer-parser.md`).

## Fuera de alcance de este borrador
Inventario completo de palabras clave, precedencia de operadores y gramática de expresiones: se
definen en el ADR de gramática mínima siguiente, no aquí.
