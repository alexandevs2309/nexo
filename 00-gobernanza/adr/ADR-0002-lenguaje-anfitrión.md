# ADR-0002 — Lenguaje anfitrión de la primera implementación de Nexo

- Estado: Aprobado
- Fecha: 2026-10-20
- Responsable: por asignar
- Revisión prevista: al completar el prototipo vertical (Puerta B de `07-validacion`)

## Contexto y requisitos
Nexo aún no tiene implementación. Antes de gramática, lexer o parser hay que elegir el lenguaje
en el que se escribe la primera implementación (lenguaje anfitrión / *host*). Por ADR-0001 el objetivo
es multipropósito, lo que implica requisitos futuros de rendimiento, FFI, compilación a WASM/nativo
y distribución de binarios por CLI. El prototipo inicial, en cambio, exige iteración rápida.

## Opciones consideradas

| Opción | Ventajas | Costes y riesgos | Reversibilidad |
|---|---|---|---|
| **A. Python** | Iteración más rápida del mercado; Unicode excelente; ecosistema de parsers (Lark, pyparsing, ANTLR); barrera de contribución mínima | Rendimiento bajo; distribución de binarios incómoda; para el objetivo multipropósito casi con certeza habría que reescribir el núcleo más adelante | Baja: reescritura posterior probable |
| **B. Rust** | Rendimiento y seguridad de memoria sin GC; FFI a C y WASM desde el día 1; `cargo` aporta build, paquetes y CI sin inventar nada; precedente directo de toolchains de análisis modernas (SWC, Biome, Rspack, Rolldown — todos en Rust); tree-sitter con bindings Rust y soporte WASM permite integración con editores después | Curva de aprendizaje; iteración más lenta que Python; tiempos de compilación del propio host | Alta: sirve tanto para prototipo como para el núcleo definitivo |
| **C. Go** | Compilación rápida; CLI simple; buen soporte de Unicode | Menor expresividad para ASTs e IR (sin ADTs algebraicos ni pattern matching); GC; peor encaje con metaprogramación de compiladores | Media |
| **D. TypeScript/Node** | Iteración rápida; buen tooling de parsers | Distribución pobre para CLI de sistema; rendimiento y memoria limitados; encaje débil con sistemas/bajo nivel | Media |
| **E. Zig** | Control de bajo nivel; sin GC | Ecosistema inmaduro; bindings y build menos estables; mayor riesgo de mantenimiento | Media-alta |

## Criterios y ponderación
1. Camino sin reescritura hacia el objetivo multipropósito (FFI, WASM, rendimiento) — alto peso.
2. Velocidad de iteración del prototipo — alto peso.
3. Tooling de proyecto y paquetes — medio.
4. Curva de aprendizaje y tamaño de comunidad — medio.
5. Distribución de binarios por plataforma — medio.

## Evidencia
- Rust 1.99.0 es la versión estable actual (rust-lang.org, verificado 2026-10-20).
- Toolchains de análisis y transformación de código modernas están escritas en Rust:
  SWC, Biome, Rspack, Rolldown (artículo de 2026-09-22 sobre coste de mantenimiento del toolchain JS).
- tree-sitter expone bindings Rust con soporte de gramáticas WASM (docs.rs/tree-sitter), lo que
  permite, en el futuro, resaltar sintaxis Nexo en editores.
- El propio compilador de Rust se autoalimenta (stage0 → stage1 → stage2, bootstrap de rust-lang/rust),
  precedente de que un lenguaje moderno puede implementarse en su propio anfitrión desde fases tempranas.

## Decisión
**Propuesto: Rust (opción B)** como lenguaje anfitrión de la primera implementación.
Motivo: es la única opción que sirve igualmente para el prototipo de la Fase 2 y para el núcleo
multipropósito futuro, evitando la reescritura completa que Python haría probable.
Plan B explícito: si el prototipo avanza demasiado lento (ver condiciones de revisión), se migra el
frontend a Python sin perder el diseño (lexer/parser/AST son piezas portables por definición).

**Aprobado por el propietario el 2026-10-20**, tras verificarse la instalación del toolchain en la
máquina de desarrollo (rustc/cargo 1.99.1 + prueba de compilación y ejecución). La validación de la
Puerta B (prototipo vertical) queda como **condición de revisión**, no de aprobación.

## Consecuencias y migración
- No decide el modelo de ejecución (AST/bytecode/nativo) — ese es un ADR aparte; aquí solo se elige el anfitrión.
- El repositorio inicial será un proyecto `cargo` con CI mínima desde el día 1.
- Riesgo de curva de aprendizaje registrado; se mitiga acotando el prototipo a lexer + parser + evaluación de expresiones.

## Condiciones de revisión
- Revisar si tras ~2-3 semanas de trabajo el prototipo no evalúa `10 + 5` con diagnósticos en español.
- Revisar si aparece evidencia de que el rendimiento del host limita el diseño del runtime de Nexo.
