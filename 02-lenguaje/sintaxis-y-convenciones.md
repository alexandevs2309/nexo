# Sintaxis y convenciones
Estado: borrador, no gramática final.

## Preguntas abiertas
¿Palabras clave solo en español o alias? ¿Identificadores Unicode? ¿Reglas de cadenas, escapes y números? ¿Punto y coma o saltos de línea significativos? ¿Delimitación de bloques? ¿Operadores? ¿Convenciones de nombres y archivos?

## Ejemplo conceptual, no normativo
```text
funcion sumar(a, b) {
    retornar a + b
}
mostrar(sumar(10, 5))
```

## Estado de implementación
`funcion`, `retornar`, `mostrar` y los valores de texto están implementados en el prototipo (ADR-0008), aunque la gramática normativa de esta edición todavía debe actualizarse. Los ejemplos ejecutables usan `.bsk` y la CLI `basilisco`/`bsk` (ADR-0010).

La sintaxis debe ser consistente, fácil de leer y escribir, de baja ambigüedad, compatible con autocompletado y formateo, y capaz de producir errores con ubicación precisa.
