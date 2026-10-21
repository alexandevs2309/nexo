# Gramática, lexer y parser
## Pipeline conceptual
1. Decodificar fuente y validar codificación.
2. Lexer: caracteres a tokens con posición.
3. Parser: estructura gramatical y AST.
4. Análisis semántico: nombres, ámbitos, tipos y restricciones.
5. IR o forma ejecutable, si corresponde.
6. Ejecución o generación de código según la arquitectura elegida.

## Contratos
Definir tokens, precedencia y asociatividad; reportar ubicación; no descartar caracteres inválidos; especificar Unicode y finales de línea; recuperarse de errores de forma segura; probar entradas válidas, inválidas y adversariales.

Entregables: gramática normativa, tabla de tokens, precedencia, árboles de referencia y pruebas de regresión/fuzzing.
