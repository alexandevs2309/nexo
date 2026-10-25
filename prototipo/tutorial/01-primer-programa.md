# Primeros programas BASILISCO

Los ejemplos siguientes son archivos ejecutables del prototipo. Ejecuta los comandos
desde `prototipo/`; cada ejemplo también está cubierto por las pruebas del proyecto.

## 1. Variables, funciones y ciclos

En [`examples/tutorial_inicio.bsk`](../examples/tutorial_inicio.bsk), `doble` recibe
un número y devuelve el resultado. El ciclo acumula el doble de 1, 2 y 3.

```text
cargo run -- run examples/tutorial_inicio.bsk
```

Salida:

```text
12
```

## 2. Funciones y retorno

[`examples/funciones_basicas.bsk`](../examples/funciones_basicas.bsk) muestra una
función con dos parámetros, recursividad y una función sin parámetros.

```text
cargo run -- run examples/funciones_basicas.bsk
```

Salida:

```text
5
120
42
```

## 3. Arreglos, indexación y condiciones

[`examples/aceptacion_v04.bsk`](../examples/aceptacion_v04.bsk) suma los elementos
de un arreglo tipado, comprueba el total y construye una salida de texto.

```text
cargo run -- run examples/aceptacion_v04.bsk
```

Salida:

```text
total=9
```

## 4. Diagnóstico de ejecución

[`examples/aceptacion_error_v04.bsk`](../examples/aceptacion_error_v04.bsk) divide
por cero intencionalmente. La CLI termina con código distinto de cero e informa el
archivo y la ubicación del error.

```text
cargo run -- run examples/aceptacion_error_v04.bsk
```

Salida de error esperada:

```text
examples/aceptacion_error_v04.bsk: error de ejecución [2:1]: división entre cero
```

## Comandos de la CLI

`bsk` es un alias de `basilisco`. `--help` muestra los comandos disponibles y
`--version` la versión del paquete. Los archivos `.bsk` son la forma canónica;
`.nexo` se acepta por compatibilidad histórica.
