# Auditoría técnica independiente — BASILISCO (NEXO)

- **Repositorio inspeccionado:** `/home/auron/Escritorio/nexo-base-ultra-completa-0.2/nexo-base-ultra-completa-0.2` (remote declarado en contexto previo: `https://github.com/alexandevs2309/nexo.git`, rama de trabajo: `main`).
- **Alcance:** inspección de código, ejecución de verificación solicitada, comprobación de ejemplos reales y comparación de motores cuando aplica.
- **Reglas respetadas:** sin creación de ramas; sin modificación de archivos fuente del proyecto salvo la generación de este informe; sin commits/pushes/publicación/cambio de versión; sin asumir documentación como prueba; sin ocultar errores; sin modificar pruebas para forzar resultados positivos. La documentación (`10-especificacion/`, `AUDITORIA_MAESTRA.md`, `README`, ADRs) se trató como referencia, **no** como evidencia de implementación.
- **Criterio de clasificación:** (a) *Implementada y verificada* = comportamiento observado en ejecución o cubierto por pruebas que pasan + inspección coherente del código; (b) *Implementada, pero no verificada* = código presente sin ejecución/prueba concluyente en esta auditoría; (c) *Parcial* = subconjunto funcional con límites claros; (d) *Experimental* = existe pero frágil/limitado/no apto para dependencia fuerte; (e) *Ausente* = no hallada capacidad equivalente; (f) *Bloqueada por dependencias* = requiere infraestructura/entorno externo no disponible. **Código implementado ≠ probado ≠ funcional.**

## 1. Resumen ejecutivo

BASILISCO es un lenguaje/prototipo implementado en Rust (`prototipo/`, crate `basilisco`, Rust 2024) con pipeline **fuente → Lexer → Parser (AST) → Análisis semántico → HIR → MIR → VM** (motor por defecto), más **evaluador Tree-Walk (árbol)** como ruta de ejecución alternativa/fallback y **backends de generación** (nativo, vía Rust+`rustc`; WASM, transpila a Rust para target wasm). El sistema es real y ejecuta programas: la suite automatizada de workspace reportó **874 passed, 0 failed, 2 ignored** (53 líneas `test result`, `cargo test --workspace --no-fail-fast`, `/tmp/audit-tests.txt`), `cargo clippy --all-targets --all-features -- -D warnings` finalizó limpio, y los 5 ejemplos solicitados produjeron salida **idéntica entre Tree-Walk, VM y backend nativo** en las condiciones probadas.

Sin embargo, **`cargo fmt --check` falló (exit 1, 7 diffs)** por formato de encadenamiento `.arg("--edition").arg("2024")` introducido en `prototipo/src/cli.rs`, `prototipo/tests/backend_nativo.rs` y `prototipo/tests/backend_wasm.rs` (hallazgo de higiene; no se corrigió). Además, las **2 pruebas `#[ignore]` fallan al forzarlas** (`cargo test -- --ignored` → `FAILED. 0 passed; 2 failed`), coherente con dependencia de infraestructura externa (integración de registro/paquetes). El lenguaje **no es de propósito general** en el sentido de Rust/Go/Java/C#: carece de genéricos de usuario, tipos de datos algebraicos/`struct`/`enum` definibles por usuario, traits/interfaces, macros de usuario, modelo de memoria propio robusto, concurrencia de propósito general segura/verificada más allá de primitivas acotadas, stdlib/ecosistema maduro, y backends de código máquina/WASM nativos completos. El backend nativo es **funcional para un subconjunto real creciente** (incluye colecciones, decimal estricto, módulos/`usar` con alias/nominal/anidados, globals/`__init`, llamadas indirectas, `leer_archivo`/`json_parsear`) pero **experimental/parcial** frente a la totalidad del lenguaje.

## 2. Arquitectura comprobada

Evidencia por inspección de `prototipo/src/` (tamaños aprox., `wc -l`): `evaluador.rs` 6196, `vm.rs` 2621, `hir.rs` 2290, `parser.rs` 2073, `cli.rs` 1865, `paquetes.rs` 1674, `semantico.rs` 1525, `mir.rs` 1477, `backend_nativo.rs` 872, `valor.rs` 814, `runtime.rs` 800, `lexer.rs` 606, `backend_entero.rs` 613, `cargador.rs` 441, `formateador.rs` 428, `ffi.rs` presente, `lsp.rs` 337, `optimizador.rs` 303, `backend_wasm.rs` 23; bin/servidor/cliente también presentes.

| Capa | Código (archivos/funciones clave) | Estado comprobado |
|---|---|---|
| Lexer + tokens | `lexer.rs` (tokenización, literales tipados `entero/decimal/real`, cadenas/escapes), `token.rs` | **Implementada y verificada** (suite/parser/lexer tests pasan; ejemplos ejecutan). |
| Parser + AST | `parser.rs` (`parsear_programa`/expresiones/sentencias; AST con `Expresion`, `Sentencia` — incluye `Lambda`, `Usar {ruta,alias,nombres}`, `LlamadaMetodo`/accesos, control de flujo, `intentar/rescatar`) | **Implementada y verificada** para la gramática cubierta; no implica cobertura total del lenguaje. |
| Análisis semántico | `semantico.rs` (`ContextoSemantico::analizar_programa`, `resolver_y_analizar_modulo`, `TipoSem`, reglas de conversión/asignación, resolución de módulos con cargador) | **Parcial→Implementada y verificada** en reglas cubiertas (p. ej. GAP-051 cerrada: conversiones de asignación alineadas; diagnóstico E1004). Inferencia/tipado incompletos frente a lenguajes estáticos maduros. |
| Tipos/valores/inferencia | `valor.rs` (`Valor`, aritmética entera/desbordamiento, decimal escala 9 / half-even / rango, real `f64`, conversiones normativas), `runtime.rs` (réplica embebida para nativo) | **Implementada y verificada** para numéricos/colecciones cubiertos (tests de valor y diferenciales). Decimal/Real con reglas específicas (no `Decimal↔Real`). Inferencia de usuario limitada (anotaciones + inferencia local/parcial). |
| Evaluador Tree-Walk | `evaluador.rs` (entorno/ámbitos, funciones, clausuras/lambdas, módulos/`Usar`, E/S, JSON, errores tipados `CausaError`) | **Implementada y verificada** (base funcional; usado como referencia de paridad y fallback). Motor “árbol”. |
| HIR | `hir.rs` (lowering con resolvedor, `lower_programa`, inlining de módulos `cargar_modulo_items`, clausuras, `ObjetoModulo`, globals/`__init`, síntesis `main`) | **Implementada y verificada** en rutas ejercidas (incl. módulos/alias/nominal/anidados de Etapa C). |
| MIR | `mir.rs` (`lower_programa`, CFG/bloques/terminadores, `Llamada`/`LlamadaIndirecta`, `Global`/`AsignacionGlobal`, `ObjetoModulo`, operandos/rvalues) | **Implementada y verificada** en subconjunto soportado; MIR existe y alimenta VM/nativo. |
| VM | `vm.rs` (interpreta MIR, stdlib/nativas, `ObjetoModulo`, globals, indirectas, tareas/canales básicos, JSON/archivo/FFI hooks según registro) | **Implementada y verificada** (motor por defecto; 874 tests workspace verdes; diferenciales). `convertir_tipo` estricto (GAP-051). |
| Backend nativo | `backend_nativo.rs` + `runtime.rs` embebido (genera Rust, compila con `rustc --edition 2024 -O`). Soporta subset amplio pero no total (ver §3) | **Parcial** (verificada para el subset: 45 tests backend_nativo, 5 ejemplos con paridad). No cubre todo el lenguaje. |
| Backend WASM | `backend_wasm.rs` (23 líneas) + `backend_entero.rs` reutilizado; genera Rust orientado a wasm32 (pruebas con `rustc --target wasm32-unknown-unknown` presentes) | **Experimental/Parcial**: transpila a Rust, **no** genera WAT/Wasm directo; cobertura acotada (entero/control básico según tests). |
| Runtime | `valor.rs`/`vm.rs` (intérprete) y `runtime.rs` (embebido nativo: `Valor`, aritmética/decimal, colecciones, JSON std-only, `leer_archivo`) | **Parcial** (suficiente para casos verificados; stdlib nativa no completa). |
| Biblioteca estándar | Nativas registradas en VM/evaluador (`mostrar`, `longitud`, `agregar`, `eliminar`, JSON, archivo, fechas/ffi/canales según módulos) | **Parcial**: núcleo usable; cobertura desigual entre motores/nativo. |
| Módulos/`usar` | `cargador.rs`, semántico `resolver_y_analizar_modulo`, HIR inlining, `ObjetoModulo`, `cli` build con cargador | **Implementada y verificada** para alias/nominal/anidados y los ejemplos objetivo; con limitaciones (doble inline, resolución HIR propia). |
| Funciones/closures | Funciones de primera clase (valores `Funcion`/clausura), `Lambda` en parser/evaluador/HIR/VM; backend nativo rechaza closures/celdas | **Parcial**: TW/VM soportan (tests diferenciales de lambdas/capturas); **nativo no** soporta closures/celdas. |
| Concurrencia | Primitivas en VM (`crear_canal`, `enviar`, `recibir`, `iniciar_tarea`, `esperar_tarea`, registro global de tareas) y tests diferenciales de canal/tareas | **Experimental/Parcial**: primitivas existen y diferenciales pasan; alcance y robustez (cancelación, scheduling, backpressure) no equivalen a modelos maduros. |
| CLI | `cli.rs`/`main.rs` (`run`/`ejecutar`, `build`/`compilar`, `analizar`, `formatear`, `probar`, `nuevo`, `instalar/add/update/publish`, `lsp`, `depurar`, `demo`, `--motor=arbol|vm`) | **Implementada y verificada** en comandos ejercidos (run/build/analizar paths observados). Superficie amplia con madurez desigual. |
| LSP y depurador | `lsp.rs` (337 líneas), `depurar` en CLI con traza VM | **Experimental/Parcial**: LSP existe como módulo (no se verificó integración IDE end-to-end aquí); depurador es traza básica. |
| Paquetes/herramientas | `paquetes.rs` (1674), `proyecto.rs`, BPK/hash/firma en código y tests BPK (2 tests ok); `registry_integration` `#[ignore]` | **Parcial**: empaquetado/manifiesto local con pruebas; integración remota/registro **bloqueada por dependencias** (2 ignoradas fallan forzadas). |
| FFI | `ffi.rs` (`cargar_ffi`/`llamar_ffi`/`descargar_ffi`, `libloading`/`libffi` según código) | **Implementada, pero no verificada** end-to-end en esta auditoría (presencia de código; no se ejecutó caso FFI real). Riesgo de portabilidad/seguridad. |

## 3. Inventario de funcionalidades

| Funcionalidad | Clasificación | Evidencia (archivos/funciones/pruebas/comandos) | Notas |
|---|---|---|---|
| Tokenización/literales (entero/decimal/real/texto/lógico) | IMPLEMENTADA Y VERIFICADA | `lexer.rs`,`token.rs`,`valor.rs`; suite 874/0/2; ejemplos ejecutan | Decimal escala 9, half-even, rango acotado. |
| Parseo AST (sentencias/expresiones, control, funciones, `usar`, lambdas) | IMPLEMENTADA Y VERIFICADA | `parser.rs` (`Sentencia::Usar`, `Expresion::Lambda`); parser tests en suite | Cobertura de gramática limitada a lo implementado. |
| Análisis semántico + resolución módulos | PARCIAL | `semantico.rs` (`analizar_programa`, `resolver_y_analizar_modulo`); E1038 corregido en `build`; GAP-051 cerrada | Inferencia/tipos no completos; depende de cargador/contexto. |
| Sistema de tipos (primitivos, colecciones, conversiones normativas) | IMPLEMENTADA Y VERIFICADA (núcleo) / PARCIAL (global) | `valor.rs`,`semantico.rs`,`vm.rs::convertir_tipo`; tests valor + 12 diferenciales GAP-051; `tipos-y-conversiones-normativos.md` (ref., no prueba) | Sin tipos de usuario/ADTs/genéricos. `Decimal↔Real` prohibido. |
| Inferencia de tipos | PARCIAL | `semantico.rs`/lowering (`TipoSem::Desconocido` frecuente), anotaciones soportadas | No equiparable a Hindley-Milner/Rust inference madura. |
| Evaluador Tree-Walk | IMPLEMENTADA Y VERIFICADA | `evaluador.rs`; paridad TW-VM en ejemplos y diferenciales | Fallback; semántica de referencia en muchos casos. |
| HIR/MIR lowering | IMPLEMENTADA Y VERIFICADA (subset) | `hir.rs`,`mir.rs`; pipeline usado por VM/nativo/tests | Subset; inlining de módulos con limitaciones. |
| VM (MIR, default) | IMPLEMENTADA Y VERIFICADA | `vm.rs`; 874 tests; `--motor=vm` en ejemplos = TW | Incluye globals, indirectas, `ObjetoModulo`, canales/tareas básicos. |
| Backend nativo (Rust+rustc) | PARCIAL | `backend_nativo.rs`,`runtime.rs`; 45 tests `backend_nativo` ok; 5 ejemplos build+run ≡ VM | Soporta: numéricos/decimal estricto, colecciones, globals/`__init`, `ObjetoModulo`, llamadas directas/indirectas, `leer_archivo`/`json_parsear`, módulos `usar` (alias/nominal/anidados). **No**: closures/celdas, try/catch/invoke, rangos, stdlib completa. `sumas_homogeneas` delega al runtime. |
| Backend WASM | EXPERIMENTAL | `backend_wasm.rs` (23), `backend_entero.rs`; 5 tests `backend_wasm` ok (transpila a Rust/wasm32) | **No** genera Wasm directo; cobertura mínima. |
| Módulos `usar` (alias, nominal, anidados) | IMPLEMENTADA Y VERIFICADA (objetivo) | `cargador.rs`,`semantico.rs`,`hir.rs` inlining,`vm.rs`,`backend_nativo.rs`,`cli.rs`; 4 tests módulo nativo; `inventario`/`punto_venta` ≡ VM | Doble-inline posible (duplicación símbolos/globals) en casos no objetivo; resolución HIR parcialmente propia. |
| Funciones/1ª clase/closures (TW/VM) | PARCIAL | `parser` `Lambda`, `evaluador` clausuras, HIR/MIR/VM clausuras-celdas; diferenciales lambda/map/filtrar/reducir | **Nativo no verificado/soportado** para closures/celdas. |
| Genéricos definibles por usuario | AUSENTE | Sin nodos/parámetros de tipo de usuario hallados en la capacidad equivalente; coherente con deuda DT-10 documentada (ref.) | No hay monomorfización de usuario verificada. |
| Structs/enums/ADTs/traits/interfaces de usuario | AUSENTE (como tipos nominales de usuario equivalentes) | No se verificó un sistema de `struct/enum/trait` de usuario equiparable a Rust/Go/Java/C# en el AST/semántica ejercida | Objetos/mapas y variantes internas existen; no sustituyen ADTs/traits de usuario. |
| Manejo de errores `intentar/rescatar` | PARCIAL | AST/parser/evaluador/VM tienen soporte en rutas; backend nativo **rechaza** try/catch | Cobertura y paridad nativa incompletas. |
| Concurrencia (canales/tareas) | EXPERIMENTAL | `vm.rs` `crear_canal/enviar/recibir/iniciar_tarea/esperar_tarea`; diferenciales `diff_canal_*` ok | Primitivas acotadas; no modelo maduro (sin cancelación general verificada, etc.). |
| FFI | IMPLEMENTADA, PERO NO VERIFICADA | `ffi.rs` (`cargar_ffi`,`llamar_ffi`,`descargar_ffi`, libloading/libffi) | Sin ejecución FFI real en esta auditoría. |
| CLI (`run/build/analizar/formatear/probar/lsp/depurar/pkg`) | PARCIAL | `cli.rs`; comandos usados: `run --motor=arbol|vm`, `build` (genera+rustc), suite; ejemplos build/run | Madurez desigual por subcomando. |
| LSP | EXPERIMENTAL | `lsp.rs` (337) | No verificada integración cliente/editor aquí. |
| Depurador | EXPERIMENTAL | `depurar` en CLI, traza VM (`vm.activar_traza`) | Traza básica, no DAP completo verificado. |
| Paquetes/BPK/manifiesto | PARCIAL | `paquetes.rs`,`proyecto.rs`; `cargo test --test bpk` 2 ok | Flujo local parcial. |
| Integración registro remoto | BLOQUEADA POR DEPENDENCIAS | `registry_integration` `#[ignore]` (2 pruebas) — `cargo test -- --ignored` → **2 failed** | Requiere infraestructura externa no disponible; no aprobada. |
| Formateador | PARCIAL | `formateador.rs`,`cli formatear`; formato del repo **no conforme** (`fmt --check` exit 1) | Funcionalidad existe; estado de formato actual falla verificación. |

## 4. Evidencias de ejecución

Comandos ejecutados (desde `prototipo/` salvo ejemplos):

1. `cargo fmt --check` → **exit 1**. 7 diffs: `src/cli.rs` (1), `tests/backend_nativo.rs` (5), `tests/backend_wasm.rs` (1), todos por expansión multilínea de `.arg("--edition").arg("2024")`. Captura: `/tmp/fmt-check.txt`.
2. `cargo clippy --all-targets --all-features -- -D warnings` → **limpio** (`Finished 'dev' profile`, sin errores/warnings en el resultado final). (Re-ejecución de auditoría tras contención de locks).
3. `cargo test --workspace --no-fail-fast` → **874 passed; 0 failed; 2 ignored** (53 `test result`), `/tmp/audit-tests.txt`.
4. `cargo test -- --ignored` (prototipo) → `FAILED. 0 passed; 2 failed` (2 pruebas ignoradas fallan al forzar ejecución) — **bloqueadas por dependencias/no aprobadas**.

Ejemplos (binario `prototipo/target/debug/basilisco`, verificados con paridad por `diff` de salidas, `$?=0` = idéntico):
- `ejemplos/consola`: `--motor=arbol`/`--motor=vm`/`build`+`./main` → **TW-VM:0, VM-NAT:0** (factorial/fibonacci).
- `ejemplos/calculadora`: TW/VM/native → **0/0** (aritmética y división por cero manejada en programa).
- `ejemplos/gestor_tareas`: TW/VM/native → **0/0** (colecciones/`agregar`/`longitud`).
- `ejemplos/inventario` (ejecutado desde su directorio, requiere `datos/inventario.json` relativo a CWD): TW/VM/`build`+`./main` → **TW-VM:0, VM-NAT:0**.
- `ejemplos/punto_venta`: VM/`build`+`./main` → **DIFF:0** (y coherente con corridas previas de sesión).

**Efectos colaterales de verificación (no commit):** `bsk build` generó `main.rs`/binario `main` en directorios de ejemplos (observado en `punto_venta`/`inventario`/otros). No se modificaron fuentes del lenguaje; estos artefactos no se confirmaron en git por esta auditoría.

## 5. Pruebas y resultados

| Verificación | Resultado | Detalle |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **APROBADA** | 874 passed, 0 failed, 2 ignored (53 líneas). Incluye unitarios, integración, diferenciales TW↔VM, backend_nativo (45), backend_wasm (5), bpk (2), cargador (14), etc. |
| `cargo clippy --all-targets --all-features -- -D warnings` | **APROBADA** | Sin warnings/error final. |
| `cargo fmt --check` | **NO APROBADA** | Exit 1, 7 diffs (formato `--edition 2024`). |
| Pruebas `#[ignore]` forzadas | **NO APROBADAS / BLOQUEADAS** | `cargo test -- --ignored` → 0 passed, 2 failed. Infraestructura externa requerida. |
| Consola/Calculadora/Gestor/Inventario/Punto de venta | **APROBADAS** (condiciones probadas) | Paridad TW=VM=nativo (diff 0). Inventario dependió de CWD correcto. |

## 6. Comparación con otros lenguajes

Comparación por **capacidades funcionales demostrables**, no por funcionalidades declaradas. BASILISCO es un prototipo/intérprete+VM con backend nativo parcial; Rust/Go/Python/JavaScript/Java/C# son lenguajes de producción con ecosistemas maduros.

| Capacidad | BASILISCO (demostrado) | Rust | Go | Python | JavaScript | Java | C# |
|---|---|---|---|---|---|---|---|
| Tipado | Dinámico con anotaciones y chequeos parciales; conversiones normativas acotadas | Estático fuerte, inferencia robusta | Estático, simple | Dinámico (hints opc.) | Dinámico (TS aparte) | Estático | Estático |
| Inferencia | Parcial/local; `Desconocido` frecuente | Avanzada | Limitada/local | N/A (dinámico) | N/A (TS sí) | Local (var) | `var`/mejorada |
| Funciones 1ª clase | Sí (TW/VM) | Sí | Sí | Sí | Sí | Sí (lambdas/refs) | Sí (delegates/lambdas) |
| Closures | Sí TW/VM (capturas/celdas); **nativo no** | Sí, ownership claro | Sí | Sí | Sí | Sí (efectivamente final) | Sí |
| Genéricos de usuario | **Ausente** | Sí (monomorfización) | Sí (desde 1.18) | Sí (typing/runtime) | TS sí | Sí (borrado) | Sí (reified parcial) |
| Structs/enums/ADTs | **Ausente** como tipos nominales de usuario equivalentes | `struct/enum`+pattern | `struct`/interfaces | clases/dataclasses | objetos/prototipos | clases/enums | clases/struct/enums |
| Traits/interfaces | **Ausente** equivalente verificado | Traits | Interfaces implícitas | Protocolos/ABC | Duck/TS | Interfaces | Interfaces |
| Concurrencia | Primitivas canal/tarea (VM, experimental) | async/threads/Send-Sync | goroutines/channels | threads/async (GIL matices) | event loop/workers | threads/virtual (reciente) | async/Tasks/threads |
| Manejo de errores | `intentar/rescatar` parcial; nativo sin try/catch | `Result`/`?` | errores como valores/panic | excepciones | excepciones/promesas | excepciones | excepciones |
| Memoria/seguridad | Valores administrados por runtime Rust; sin modelo propio verificado | Ownership/borrow, sin GC | GC | GC/refcount | GC | GC | GC |
| Compilación | Intérpretes TW/VM + backend nativo (genera Rust→rustc) **parcial**; WASM experimental | AOT nativo | AOT nativo | Bytecode/interpretado | JIT/interpretado | Bytecode/JVM/JIT/AOT | IL/JIT/AOT |
| Interop | FFI presente pero **no verificada** | C FFI robusto | cgo/C | C/API extensiva | N-API/WASM/host | JNI/FFM | P/Invoke |
| Biblioteca estándar | Núcleo parcial (I/O, JSON, colecciones, archivo); desigual por motor/nativo | Amplia | Amplia | Muy amplia | Host + npm | Muy amplia | Muy amplia |
| Ecosistema/herramientas | CLI/LSP/debug/pkg **parciales/experimentales**; 2 tests registro bloqueadas | cargo/rust-analyzer maduro | go tool maduro | pip/venv maduro | npm maduro | Maven/Gradle maduro | dotnet/NuGet maduro |

**Conclusión comparativa:** BASILISCO demuestra un núcleo de lenguaje funcional (sintaxis, control, funciones, colecciones, módulos, numéricos con decimal estricto, VM+TW con paridad en casos probados, y compilación nativa de un subconjunto real). **No** demuestra equivalencia funcional con lenguajes de propósito general en abstracciones de tipos (genéricos/ADTs/traits), robustez de concurrencia/errores en todos los backends, stdlib/ecosistema, ni independencia de toolchains externas (nativo depende de `rustc`; WASM de toolchain Rust/target).

## 7. Hallazgos críticos

1. **`cargo fmt --check` falla (exit 1, 7 diffs):** `prototipo/src/cli.rs`, `prototipo/tests/backend_nativo.rs`, `prototipo/tests/backend_wasm.rs` — formato de `.arg("--edition").arg("2024")`. Incumple la verificación de estilo solicitada; no corregido por regla de auditoría.
2. **2 pruebas `#[ignore]` fallan al forzarlas** (`registry_integration`): `cargo test -- --ignored` → 0 passed, 2 failed. Indica dependencia externa real; no deben contarse como aprobadas.
3. **Backend nativo parcial (no totalidad del lenguaje):** rechaza closures/celdas, try/catch/invoke, rangos y gran parte de stdlib. Código válido en TW/VM puede no compilar nativamente.
4. **WASM no es binario directo:** `backend_wasm.rs` (23 líneas) transpila a Rust; no hay generación WAT/Wasm propia verificada como backend completo.
5. **Riesgo de doble inlining de módulos (nativo/HIR):** sin deduplicación de símbolos/globals por identidad de módulo en todos los contextos; benigno en objetivos, potencialmente roto en grafos de imports más complejos.
6. **LSP/debugger/FFI no verificados end-to-end:** código presente, madurez/funcionalidad real no demostrada integralmente aquí.

## 8. Limitaciones

- Sin genéricos de usuario, ADTs/`struct`/`enum` nominales de usuario, traits/interfaces, pattern matching algebraico completo equivalente a lenguajes maduros (no verificado como capacidad de usuario).
- Backend nativo depende de `rustc` (`--edition 2024`) y de `rustc` en PATH; `leer_archivo`/rutas de datos dependen de CWD (paridad con VM, pero sensible al contexto).
- Stdlib nativa embebida limitada (`leer_archivo`, `json_parsear`, colección/E/S básica); muchas nativas del intérprete no están en `runtime.rs`.
- `try/catch`, closures/celdas, rangos no soportados en nativo.
- Concurrencia = primitivas VM experimentales; sin garantías de propósito general.
- Formato del árbol no pasa `fmt --check` en el estado auditado.
- Pruebas de registro remoto bloqueadas por infraestructura.

## 9. Riesgos arquitectónicos

- **Divergencia de semántica entre motores/backends:** mitigada por diferenciales (TW↔VM) y pruebas de paridad, pero persiste en áreas no cubiertas (p. ej. representación `[...]` Lista/Arreglo en diagnósticos observada; backends con subsets distintos).
- **Inlining de módulos sin caché global de emisión:** puede escalar mal o duplicar estado/símbolos.
- **Crecimiento monolítico:** `evaluador.rs` (6196), `vm.rs` (2621), `hir.rs` (2290) concentran complejidad; aumenta riesgo de regresión.
- **Dependencia de toolchain externa para “nativo”:** el “binario nativo” es un artefacto mediado por Rust; no es un compilador nativo independiente de BASILISCO.
- **Superficie FFI/unsafe indirecto (vía libloading/libffi):** riesgo de seguridad/portabilidad si se usa.
- **Higiene CI local degradada:** `fmt --check` rojo indica deriva entre cambios recientes y formato canónico.

## 10. Porcentaje de avance por componente (metodología)

**Metodología (cualitativa→cuantitativa acotada, solo orientativa):** se asigna 0–100% por componente como `(0.5 * Implementación observada en código + 0.5 * Verificación ejecutada/pasada relevante)`, ajustado a la baja por límites de alcance (subset, bloqueos, paridad incompleta). **No** es una medida de completitud del lenguaje; componentes distintos no son sumables linealmente. Se justifica con la evidencia anterior.

| Componente | % (orientativo) | Justificación breve |
|---|---:|---|
| Lexer | 85% | Implementado y cubierto por suite/ejemplos. |
| Parser/AST | 80% | Amplio, verificado; gramática no total. |
| Semántico/tipos | 55% | Núcleo+conversiones verificadas; inferencia/usuario incompletos. |
| Tree-Walk | 80% | Robusto en lo ejercido; referencia/paridad. |
| HIR/MIR | 70% | Pipeline funcional; módulos con limitaciones de inlining. |
| VM | 75% | Motor default verificado, stdlib parcial, concurrencia experimental. |
| Backend nativo | 45% | Subset real verificado (5 ejemplos, 45 tests) pero excluye features clave. |
| Backend WASM | 15% | Transpilación mínima, no Wasm directo. |
| Runtime/stdlib | 45% | Núcleo útil; cobertura desigual; nativo embebido parcial. |
| Módulos/`usar` | 65% | Casos objetivo verificados; riesgos de duplicación/unificación. |
| CLI | 55% | Comandos clave funcionan; superficie heterogénea. |
| Paquetes/registro | 35% | Local/BPK parcial; registro remoto bloqueado (2 ignored fallan). |
| LSP/depurador | 20% | Presentes/experimentales, sin verificación integral. |
| FFI | 25% | Código presente, no verificado end-to-end. |

## 11. Nivel de madurez general y justificación

**Nivel: Prototipo avanzado / pre-alfa técnico (núcleo funcional, no de propósito general).**

Justificación: (1) ejecución real y repetible de un núcleo coherente (874 tests verdes, clippy limpio, 5 ejemplos con paridad TW/VM/nativo); (2) arquitectura de compilación moderna en forma (AST→semántico→HIR→MIR→VM + backend nativo parcial) operativa en subconjuntos; (3) carencias estructurales frente a un lenguaje general (abstracciones de tipos de usuario, genéricos, ecosistema, stdlib completa, backends totales, concurrencia/errores uniformes); (4) señales de inmadurez operativa (`fmt --check` rojo, 2 tests ignoradas fallando forzadas, backends parciales/experimentales). Es adecuado para experimentación, docencia del diseño y programas dentro del subset soportado, **no** para reemplazar lenguajes de propósito general en producción.

## 12. Plan de trabajo priorizado

1. **Restaurar higiene de verificación (P0):** corregir `cargo fmt` (7 diffs `--edition 2024`) y mantener `fmt`/`clippy`/tests verdes en CI local; tratar `fmt` como gate.
2. **Backend nativo — cerrar brechas del subset (P0/P1):** deduplicación de módulos inlinados por identidad canónica (símbolos/globals/init), ampliar stdlib embebida (`a_texto`/coerciones útiles, JSON completo según necesidad), y definir matriz explícita de soporte (closures/celdas/try-catch/rangos: implementar o rechazar con diagnóstico claro y probado).
3. **Paridad y robustez semántica (P1):** extender diferenciales TW↔VM↔nativo a módulos complejos/grafos de imports, errores de carga, y representación de literales/diagnósticos; unificar resolución de rutas entre HIR/cargador/semántico.
4. **Tipos/abstracciones (P1, diseño primero):** ADR+especificación para ADTs/`struct`/`enum`/`match` útil y genéricos de usuario (o alternativa explícita); implementar con tests de conformidad. Hoy es el mayor gap de “propósito general”.
5. **Concurrencia y errores (P1/P2):** formalizar modelo (cancelación, errores de tareas, aislamiento), completar `intentar/rescatar` con paridad en nativo o delimitar soporte.
6. **WASM realista (P2):** decidir si se mantiene transpile-a-Rust o se invierte en generación Wasm directa; acotar alcance y medir contra el subset numérico/colecciones.
7. **Ecosistema/herramientas (P2):** registro/paquetes con tests de integración ejecutables (entorno reproducible o mocks), LSP verificado en editor, depurador más allá de traza.
8. **Seguridad/FFI (P2):** auditoría de `ffi.rs` (unsafe, ABI, librerías), pruebas end-to-end controladas.

---
*Informe generado en modo solo lectura del código/estado (salvo escritura de este archivo). Las ejecuciones de build de ejemplos pudieron generar artefactos locales en `ejemplos/*/(main,main.rs)`.*