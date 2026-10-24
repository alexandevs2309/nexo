# Primer programa BASILISCO

Este tutorial ejecutable recorre funciones, variables, un ciclo y una expresión final.
El archivo fuente es [`examples/tutorial_inicio.bsk`](../examples/tutorial_inicio.bsk).

Desde `prototipo/`, ejecútalo con:

```text
cargo run -- run examples/tutorial_inicio.bsk
```

El programa define `doble`, empieza `contador` y `total` en cero y suma el doble de
1, 2 y 3. La CLI muestra el valor de la última expresión:

```text
12
```

La misma fuente se ejecuta como parte de `cargo test`, para mantener el tutorial
sincronizado con el parser y el evaluador.
