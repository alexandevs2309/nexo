# Comparativa de BASILISCO con otros lenguajes

## Nota de Fase 1

BASILISCO 0.6.0 tiene funciones de primera clase parciales (`Valor::Funcion(String)`), sin closures ni lambdas. Funciones anidadas rechazadas por el parser. `Expresion::LlamadaIndirecta` no valida firma/argumentos/retorno. Inferencia de funciones no anotadas limitada. Sin IR/VM/nativo/WASM. Gestor de paquetes local.

# Comparativa de BASILISCO con otros lenguajes

| Capacidad | BASILISCO | Python/JS/TS | Rust/Go/Java/C#/Kotlin/Swift | Ruby/PHP/Lua/Dart/C++ |
|---|---|---|---|---|
| Funciones nombradas | Sí | Sí | Sí | Sí |
| Funciones de primera clase | Sí | Sí | Sí | Variable |
| Lambdas | Sí | Sí | Sí | Variable |
| Closures | Sí | Sí | Sí | Variable |
| Tipado estático | Parcial | Opcional/No | Sí | Variable |
| Inferencia | Parcial | Dinámico | Fuerte | Variable |
| OO | No | Sí | Sí | Variable |
| Concurrencia | Parcial | Sí | Sí | Variable |
| Módulos | Sí | Sí | Sí | Variable |
| Paquetes | Sí (local + R6 remoto) | Sí | Sí | Variable |
| Compilación nativa | No | No (CPython/Node) | Sí | Variable |
| WASM | No | Parcial | Parcial | Variable |
| FFI | Retirada deliberadamente (Fase 5) | Sí | Sí | Sí |
| Ecosistema | Incipiente | Maduro | Maduro | Variable |

## Conclusiones

BASILISCO posee un núcleo interpretado sólido, pruebas amplias, funciones de primera clase completas (closures, lambdas, captura de entorno) y registro remoto de paquetes (R6) implementado y verificado. La FFI/ABI C fue implementada y retirada deliberadamente; queda pendiente para Fase 5 junto con IR, compilación nativa y WASM. No conviene reimplementar FFI sin una arquitectura de IR estable.