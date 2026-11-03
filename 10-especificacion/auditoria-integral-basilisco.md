# Auditoría técnica integral de BASILISCO

## 1. Resumen ejecutivo (sincronizado 2026-11-01)

BASILISCO 0.6.0 cuenta con lexer, parser, evaluador, análisis semántico, módulos, CLI y biblioteca estándar básica. **R6 (registro remoto) CERRADA** con evidencia contra PostgreSQL 16 real: servidor Axum/SQLx, autenticación Bearer, namespaces, publicación BPK (SHA-256 + Ed25519), cadena de confianza, descarga, yank, test E2E automatizado y persistencia verificada. **FFI RETIRADA DELIBERADAMENTE** en el ciclo R6 (eliminadas libffi/libloading para simplificar el build); spec histórica conservada en `10-especificacion/ffi-abi.md`. Race condition en tests paralelos corregida (2026-11-01). Sin IR, VM, nativo ni WASM. Validación: 551+ pruebas aprobadas, 0 fallidas; `cargo fmt --check` y `cargo clippy --all-targets -- -D warnings` correctos.

## 1. Resumen ejecutivo (sincronizado 2026-10-30)

BASILISCO 0.6.0 cuenta con lexer, parser, evaluador, análisis semántico, módulos, CLI y biblioteca estándar básica. Validación: 551 pruebas aprobadas, 0 fallidas; `cargo fmt --check` y `cargo clippy --all-targets -- -D warnings` correctos. Funciones de primera clase con closures, lambdas y captura de entorno léxico; funciones anidadas soportadas por parser, evaluador y análisis semántico. `Expresion::LlamadaIndirecta` valida estáticamente firma, parámetros y retorno. El izado de funciones de nivel superior es consistente entre `analizar` y `run` (llamadas anticipadas, recursión mutua y detección de duplicados). Inferencia de funciones no anotadas limitada. Sin IR, VM, nativo ni WASM. Gestor de paquetes local.

## 1. Resumen ejecutivo (Fase 1, histórico)

Sesión de auditoría con 498 pruebas y funciones de primera clase parciales; los puntos "sin closures, lambdas" y "funciones anidada rechazadas por el parser" quedaron superados por los cambios del 2026-10-27 al 2026-10-30 (commits de funciones de primera clase y consolidación).

BASILISCO 0.6.0 es un lenguaje implementado en Rust con lexer, parser, AST, evaluador de árbol, sistema de tipos estático parcial, análisis semántico, módulos, CLI y biblioteca estándar básica. No existe IR, VM, compilación nativa ni WebAssembly. La ejecución real se realiza mediante evaluador de árbol en `prototipo/src/evaluador.rs`.

Estado general: **núcleo funcional con capacidades avanzadas parciales**. La implementación supera la documentación en algunas áreas (módulos, funciones de primera clase, stdlib) y queda por debajo en arquitectura de ejecución avanzada.

## 2. Arquitectura actual

- Lexer: `prototipo/src/lexer.rs`
- Parser: `prototipo/src/parser.rs`
- AST: `Expresion`, `Sentencia` en `parser.rs`
- Evaluador: `prototipo/src/evaluador.rs` (`evaluar_con`, `Entorno`)
- Semántico: `prototipo/src/semantico.rs` (`ContextoSemantico`)
- Módulos: `prototipo/src/cargador.rs` (`CargadorModulos`)
- CLI: `prototipo/src/cli.rs`
- Tipos: `TipoAnotacion` en parser, `TipoSem` en semantico, `Valor` en valor.rs

## 3. Fortalezas

- Suite amplia de pruebas (~498 tests).
- Análisis semántico integrado en `basilisco analizar`.
- Módulos con importaciones, alias, exports públicos/privados, ciclos.
- Funciones de primera clase básicas.
- Biblioteca estándar numérica, texto, colecciones, archivos.
- Seguridad de rutas en módulos y archivos.

## 4. Debilidades

- Evaluador de árbol como único backend.
- Inferencia de tipos limitada para funciones no anotadas.
- Clausuras no implementadas. *(superado en 2026-10-30: closures con captura de entorno implementadas)*
- Gestor de paquetes local, sin registro remoto. *(superado en 2026-10-31: R6 cerrada)*
- Sin IR/VM/native/WASM.
- FFI no implementado. *(superado transitoriamente en 2026-10-31, luego **retirada deliberadamente** en ciclo R6; spec histórica en `ffi-abi.md`)*

## 5. Limitaciones

- No hay compilación ahead-of-time.
- No hay concurrencia real robusta (tareas básicas).
- No hay orientación a objetos (clases/objetos).
- No hay genéricos formales.

## 6. Hallazgos por área

### A. Funciones
- Funciones nombradas, recursión, llamadas anticipadas: implementadas en parser, evaluador y semántico con la misma estructura (izado de nivel superior, E1003 en duplicados).
- Funciones de primera clase: implementadas (closures con captura de entorno, transferencia y devolución).
- Lambdas/anónimas: implementadas (`funcion (x) { ... }` con `retornar` válido y anotación de retorno).
- Callbacks: posibles por valor de función y por nombre.
- Funciones anidada: implementadas y verificadas en las tres capas; el uso anticipado dentro de una función es error E1002 coherente.

### B. Tipos
- Primitivos y compuestos básicos: implementados.
- Inferencia parcial: implementada.
- Colecciones tipadas: parcial.
- Tipos de función: representación estática básica.

### C. Parser
- Gramática amplia con precedencia documentada.
- Recuperación de errores limitada.

### D. Módulos
- Importaciones, alias, exports, ciclos: implementados.
- Aislamiento: parcial (ejecución en entorno propio).

### E. Paquetes
- Proyecto local, manifiesto, instalación básica: implementado.
- Registro remoto R6: **IMPLEMENTADO Y CERRADO** (servidor Axum/SQLx, auth, namespaces, BPK, Ed25519). Ver `prototipo/README-registry.md`.

### F. Arquitectura
- Solo tree-walk.
- IR/VM/native/WASM: ausentes.

### G. CLI
- Comandos: run, analizar, formatear, nuevo, probar, instalar, paquetes.
- Alias `bsk`.

### H. Calidad
- 551 pruebas (2026-10-30).
- Fuzzing básico.
- Portabilidad: Linux verificado; Windows/macOS no verificados.

## 7. Evidencias

- `prototipo/src/evaluador.rs`: evaluador y builtins.
- `prototipo/src/semantico.rs`: análisis semántico y módulos.
- `prototipo/src/cargador.rs`: cargador de módulos.
- `prototipo/src/cli.rs`: CLI.
- `prototipo/tests/`: suites de conformidad.

No se modificó código fuente durante esta auditoría.