# Comparativa de BASILISCO con otros lenguajes

## Nota de Fase 1

BASILISCO 0.6.0 tiene funciones de primera clase parciales (`Valor::Funcion(String)`), sin closures ni lambdas. Funciones anidadas rechazadas por el parser. `Expresion::LlamadaIndirecta` no valida firma/argumentos/retorno. Inferencia de funciones no anotadas limitada. Sin IR/VM/nativo/WASM. Gestor de paquetes local.

# Comparativa de BASILISCO con otros lenguajes

| Capacidad | BASILISCO | Python/JS/TS | Rust/Go/Java/C#/Kotlin/Swift | Ruby/PHP/Lua/Dart/C++ |
|---|---|---|---|---|
| Funciones nombradas | Sí | Sí | Sí | Sí |
| Funciones de primera clase | Parcial | Sí | Sí | Variable |
| Lambdas | No | Sí | Sí | Variable |
| Closures | No | Sí | Sí | Variable |
| Tipado estático | Parcial | Opcional/No | Sí | Variable |
| Inferencia | Parcial | Dinámico | Fuerte | Variable |
| OO | No | Sí | Sí | Variable |
| Concurrencia | Parcial | Sí | Sí | Variable |
| Módulos | Sí | Sí | Sí | Variable |
| Paquetes | Parcial | Sí | Sí | Variable |
| Compilación nativa | No | No (CPython/Node) | Sí | Variable |
| WASM | No | Parcial | Parcial | Variable |
| FFI | No | Sí | Sí | Sí |
| Ecosistema | Incipiente | Maduro | Maduro | Variable |

## Conclusiones

BASILISCO posee un núcleo interpretado sólido y pruebas amplias, pero necesita lambdas, clausuras, OO opcional, mejor inferencia y backends de ejecución para ser competitivo. No conviene implementar todavía compilación nativa o FFI sin una arquitectura de IR estable.