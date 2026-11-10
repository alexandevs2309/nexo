# Auditoría profunda de capacidades — BASILISCO (9 objetivos)

- **Repositorio:** `/home/auron/Escritorio/nexo-base-ultra-completa-0.2/nexo-base-ultra-completa-0.2`
- **Rama:** `main` (inspección local).
- **Fecha de ejecución (auditoría):** 2026-11 (sesión actual).
- **Modo:** **solo lectura** sobre código/pruebas/docs existentes. No se modificó código, pruebas ni documentación; no se crearon ramas; no commits/pushes; no publicación/cambio de versión; no se implementaron soluciones. Se generó únicamente este informe. Comprobaciones que escriben fuera del repo (p. ej. `/tmp`, `target/`, `cargo test` caches) se consideran artefactos de verificación, no cambios funcionales del lenguaje.
- **Criterio:** no se declara implementado por nombre/estructura; solo por comportamiento comprobado en código + ejecución/pruebas o, si no, se clasifica explícitamente. **Implementado ≠ probado ≠ funcional.**

**Categorías usadas:** (1) IMPLEMENTADA Y VERIFICADA; (2) IMPLEMENTADA, NO VERIFICADA; (3) PARCIAL; (4) EXPERIMENTAL; (5) AUSENTE; (6) BLOQUEADA POR DEPENDENCIAS.

## 1. Resumen ejecutivo

BASILISCO (crate `basilisco`, `prototipo/`, Rust edition 2024) tiene un núcleo funcional verificable: pipeline `Lexer → Parser/AST → Semántico → HIR → MIR → VM` (motor por defecto), evaluador **Tree-Walk**, y **backend nativo** (genera Rust embebido + compila con `rustc --edition 2024`) operativo en un **subconjunto real** (incluye numéricos/decimal estricto, colecciones, módulos `usar` alias/nominal/anidados, globals/`__init`, llamadas directas/indirectas, `leer_archivo`/`json_parsear`). Evidencia de regresión actual: `cargo fmt --check` OK (exit 0); `cargo clippy --all-targets --all-features -- -D warnings` OK; `cargo test --workspace --no-fail-fast` → **874 passed, 0 failed, 2 ignored** (53 líneas `test result`, `/tmp/lote1-tests2.txt`). Paridad TW=VM=nativo verificada en 5 ejemplos (Lote 1/auditoría previa, diff 0).

Hallazgos críticos de esta auditoría: (a) **WASM “5 passed” es engañoso bajo rustc 1.99.1**: el backend genera `#[no_mangle]` (`src/backend_wasm.rs`), que falla al compilar a `wasm32-unknown-unknown` (`error: unsafe attribute used without unsafe`), y los tests de compilación/ejecución **retornan temprano y el harness los marca `ok`** (skip silencioso) — **no hay ejecución WASM verificada** por esos tests; una prueba manual e2e (temp, atributo corregido solo en `/tmp`) sí produjo `.wasm` y ejecutó en Node devolviendo `55`. (b) **Concurrencia diferencial (canales) flaky**: `diferencial_vm` canal tests pasaron en la corrida workspace (81 passed en diferencial), pero ejecuciones repetidas filtradas mostraron **fallos intermitentes** (muestreo: 3/5 `FAILED 0 passed 2 failed`) con divergencia real capturada (p. ej. TW `42/62` vs VM `10/30`), indicando race/estado compartido. (c) **Registro remoto bloqueado**: 2 `registry_integration` `#[ignore]` fallan forzados por `DATABASE_URL` ausente (además requieren PostgreSQL aislado local, `psql`, `basilisco-registry`/red). (d) Abstracciones “de propósito general” **ausentes** como capacidad de usuario demostrable: genéricos de usuario, `struct/enum/trait` nominales, monomorfización de usuario, pattern matching ADT — no hallados en lexer/parser/AST/semántico/HIR/MIR/VM/backends como feature de usuario. (e) Closures/capturas/celdas **no soportadas en backend nativo** (rechazo explícito); FFI **no disponible en nativo/WASM**.

## 2. Auditoría detallada de los nueve objetivos

### Objetivo 1 — Sistema de tipos

| Aspecto | Estado | Evidencia (archivos/funciones) | Pruebas/comandos | Limitaciones / confianza |
|---|---|---|---|---|
| Representaciones (`TipoSem`, `TipoAnotacion`, MIR locals/globals) | **PARCIAL** | `src/semantico.rs` `enum TipoSem` (Entero/Decimal/Real/Logico/Texto/Arreglo/Lista/Coleccion/Funcion/Nulo/Desconocido), `VarInfo`, `FuncionSig`; `src/parser.rs` `TipoAnotacion`; `src/mir.rs` `MirFunction.locals/retorno/globals`; `src/valor.rs` decimal/real | Compilación/tests workspace verdes (874/0/2). `grep TipoSem` confirma variantes. | `Desconocido` frecuente → tipado incompleto. **Confianza alta** en existencia; **media** en solidez global. |
| Inferencia | **PARCIAL** | `semantico.rs` análisis propaga tipos parciales; MIR infiere locales/retornos en backend_nativo (`inferir_*`), pero semántica conserva `Desconocido` | Indirecta vía tests; sin suite dedicada a inferencia completa | No HM/madurada. **Confianza media**. |
| Unificación/compatibilidad | **PARCIAL** | `src/semantico.rs` `fn compatible(a,b)->bool` (≈L1389): `Desconocido` compatible con todo; numéricos con reglas (Entero→Decimal/Real implícito), colecciones/funciones estructurales; `comparables/ordenables/tipo_numerico_resultado` presentes | Diferenciales/conversiones (GAP-051) pasan; `compatible` no es unificador completo | Permisividad por `Desconocido` reduce garantías. **Confianza alta** en reglas leídas. |
| Conversiones implícitas/explícitas | **IMPLEMENTADA Y VERIFICADA (núcleo) / PARCIAL** | `semantico.rs` + `vm.rs::convertir_tipo` estricto; `valor.rs` aritmética/conversiones normativas (ADR-0005/GAP-051 ref. código) | GAP-051 diferenciales cubiertos en suite; `cargo test --workspace` verde | Cobertura no total de todos los contextos/motores. **Confianza alta** núcleo. |
| Tipos de retorno/funciones | **PARCIAL** | `FuncionSig{params,retorno}`, `TipoSem::Funcion{params,retorno}`, chequeos de retorno en semántico; MIR ret tipado | Suite pasa | Retornos/indirectas dinámicas degradan a `Desconocido` en rutas. **Confianza media**. |
| Verificador HIR | **IMPLEMENTADA, NO VERIFICADA (como gate fuerte)** | `src/hir.rs` `pub struct VerificadorHir; impl VerificadorHir { pub fn verificar(programa:&HirPrograma)->bool }` + `verificar_items/verificar_item/verificar_tipos_invariantes/verificar_expr` (≈L221,445,455,519,637) | Sin evidencia en esta sesión de invocación obligatoria en pipeline ni test dedicado fallando/pasando su contrato | Retorna `bool`; alcance real no demostrado end-to-end. **Confianza media-baja** en efectividad. |
| Verificador MIR | **IMPLEMENTADA, NO VERIFICADA (como gate fuerte)** | `src/mir.rs` `pub struct VerificadorMir; pub fn verificar(prog:&MirPrograma)->bool` (≈L1278) | Ídem | **Confianza media-baja**. |
| Tipos de colecciones | **PARCIAL** | `TipoSem::Arreglo(T,n)/Lista(T)/Coleccion(T)`, compatibilidades; runtime `Valor::Arreglo/Lista/Objeto` | Backend nativo + diferenciales colecciones/ejemplos pasan | Genéricos de colección son de lenguaje/builtin, **no** genéricos de usuario. **Confianza alta**. |
| Errores estáticos | **PARCIAL** | Diagnósticos tipados en semántico (`AsignacionIncompatible`, etc.), resolución de módulos/E1038 manejada en build | Tests/ejemplos; 874/0/2 | Cobertura incompleta; `Desconocido` oculta errores. **Confianza media**. |

### Objetivo 2 — Genéricos, estructuras, enumeraciones y abstracciones (de usuario)

| Capacidad | Estado | Evidencia | Pruebas/comandos | Limitaciones / confianza |
|---|---|---|---|---|
| Genéricos definidos por usuario | **AUSENTE** | Búsqueda `lexer/parser/semantico/hir/mir/vm/backend_nativo/backend_wasm`: sin parámetros genéricos de usuario/`<T>` como declaración; `lista<T>`/`arreglo<T,n>` son anotaciones builtin | `grep` (sin hallazgo equivalente) | `lista<T>` **no demuestra** genéricos de usuario. **Confianza alta**. |
| Monomorfización (usuario) / alternativa | **AUSENTE** | No hay pase de monomorfización de usuario en MIR/backends; backend_nativo genera código dinámico `Valor`/subset | N/A | **Confianza alta**. |
| Estructuras con nombre (usuario) | **AUSENTE** | Sin `struct` de usuario en AST/semántico como tipo nominal definido por usuario (búsqueda en parser/AST) | N/A | Objetos/mapas ≠ structs nominales. **Confianza alta**. |
| Enumeraciones (usuario) | **AUSENTE** | Sin `enum` de usuario en gramática/semántica | N/A | **Confianza alta**. |
| Tipos suma/producto (usuario) | **AUSENTE** | No ADTs algebraicos de usuario | N/A | **Confianza alta**. |
| Traits/interfaces | **AUSENTE** | Sin `trait/interface` en pipeline | N/A | **Confianza alta**. |
| Implementaciones/métodos/resolución | **PARCIAL (limitado, no traits)** | Llamadas/métodos/miembros existen en rutas (`LlamadaMetodo`/acceso/campo, dispatch indirecto VM/nativo); sin sistema de traits/impls de usuario | Diferenciales/ejemplos | Resolución no equivale a traits/interfaces. **Confianza media**. |
| Type aliases | **AUSENTE** | Sin `TypeAlias`/alias de tipo de usuario hallado | N/A | **Confianza media-alta**. |

### Objetivo 3 — Closures y funciones de primera clase en compilación nativa

**Evidencia de implementación (TW/VM):** `parser.rs` `Expresion::Lambda` + parse; `evaluador.rs` `Valor::FuncionClausura` (captura `Arc<Mutex<Entorno>>`), lambdas/anidadas; `hir.rs` `Lambda{params,capturas}`, `capturadas/capturas_externas/celdas`; `mir.rs` `CrearCelda/LeerCelda/Clausura`, `LlamadaIndirecta`; `vm.rs` `Valor::Celda/ClausuraMir`, manejo de clausuras/indirectas, invocación de valores función.

**Backend nativo:** `src/backend_nativo.rs` rechaza explícitamente `CrearCelda/LeerCelda/Clausura` y el subset excluye closures/celdas (soporta llamadas indirectas vía dispatch de `Valor::Funcion` nombre, no clausuras con entorno/celdas capturadas).

#### Matriz de paridad (conceptual, evidenciada; sin modificar archivos)

| Caso | Tree-Walk | VM | Backend nativo | Evidencia |
|---|---:|---:|---:|---|
| Lambdas | Verificado | Verificado | No soportado | `diferencial_vm`: `diff_lambda_en_map/filtrar/reducir` → **3 passed** (`cargo test --test diferencial_vm lambda`). |
| Funciones como valores / paso como argumento | Verificado | Verificado | Parcial (solo `Valor::Funcion` por nombre/indirecta simple; no clausura con captura) | `diff_closure_como_argumento` en conjunto `closure` → **7 passed** (`--test diferencial_vm closure`). |
| Captura inmutable | Verificado | Verificado | No soportado | `diff_closure_inmutable/captura_simple/dos_instancias`, `diff_map_con_clausura_capturada` (**7 passed**). |
| Captura mutable / celdas compartidas | Verificado | Verificado | No soportado | `diff_closure_mutable/_captura`, MIR celdas, nativo rechaza celdas. (**7 passed**). |
| Funciones anidadas | Verificado | Verificado | No soportado (clausuras) | evaluador/HIR soportan; nativo rechaza celdas/clausuras. |
| Retorno de funciones (clausura retornada) | Verificado | Verificado | No soportado | `diff_closure_retornada_anidada` (**7 passed**). |
| Llamadas indirectas | Verificado | Verificado | **Parcial/Verificado (subset)** | VM `LlamadaIndirecta`; nativo `bsk_llamar_indirecto` despacha `Valor::Funcion` sobre funciones soportadas (módulos/alias). |
| Entorno/vida útil | Gestionado (Arc/Mutex TW; celdas/valores VM) | Gestionado (MIR/valores) | No aplica a clausuras | Diseño observado; sin pruebas de fugas/estrés dedicadas aquí. **Confianza media**. |

**Nivel de confianza:** alto en paridad TW/VM para casos testeados; alto en no-soporte de clausuras/celdas en nativo (rechazo explícito en código).

### Objetivo 4 — Biblioteca estándar

**Superficie (evidencia):** VM registra nativas vía `registrar_nativo` en `src/vm.rs` (≈54 registros detectados por patrón) y `registrar_stdlib()`; evaluador expone nativas por dispatch propio (`src/evaluador.rs`); backend nativo/runtime embebido soporta subset explícito: `mostrar/mostrar_sin_salto`, `agregar`, `eliminar`, `longitud` (+ comparables en checks), wrappers `bsk_leer_archivo`, `bsk_json_parsear` (`src/backend_nativo.rs`, `src/runtime.rs`). `backend_wasm` = subconjunto entero/lógico sin `String`/E/S (según header/tests). No se halló `cargar_ffi/llamar_ffi/descargar_ffi` en nativo/runtime.

| Dominio | Estado global | Evidencia | Disp. TW | Disp. VM | Disp. nativo | Disp. WASM | Notas |
|---|---|---|---:|---:|---:|---:|---|
| E/S (`mostrar`, salto) | **IMPLEMENTADA Y VERIFICADA** | `vm.rs` nativas; `backend_nativo/runtime` generan print | Sí (observado ejemplos) | Sí | Sí (ejemplos diff 0) | **No** (subset sin E/S) | |
| Texto | **PARCIAL** | nativas VM/evaluador; nativo soporta texto en subset/runtime | Sí | Sí | Parcial | No | |
| Colecciones (`longitud`,`agregar`,`eliminar`, etc.) | **PARCIAL** | VM nativas; nativo soporta subset y runtime colecciones | Sí | Sí | Parcial (verificado) | No (entero/lógico) | |
| Matemáticas | **PARCIAL** | operadores/numéricos verificados; stdlib matemática ampliada no inventariada exhaustivamente | Parcial | Parcial | Parcial | Entero básico | Sin inventario completo de cada función. **Confianza media**. |
| Archivos (`leer/escribir/existe_archivo`) | **PARCIAL** | `vm.rs` registra archivo (observado en inspección previa); evaluador; nativo solo `leer_archivo` wrapper runtime | Sí (rutas VM) | Sí | **Solo `leer_archivo`** | No | Desigualdad entre motores. |
| Directorios | **IMPLEMENTADA, NO VERIFICADA** (alcance) | presencia esperable en nativas intérprete; no ejercitada end-to-end aquí | ? | ? | No | No | Requiere prueba dirigida. **Confianza baja**. |
| JSON (`json_parsear/serializar`) | **PARCIAL** | VM/evaluador (serde_json); nativo `json_parsear` via parser std-only en `runtime.rs` (mapeo int→Entero, float→Real, array→Lista, obj→Objeto) | Sí | Sí | **`json_parsear` sí** (inventario); serializar no evidenciado en runtime | No | |
| Fecha y hora | **IMPLEMENTADA, NO VERIFICADA** | `fechas.rs` en crate; nativas registradas en VM (patrón) | Probable | Probable | No evidenciado | No | No ejecutada prueba específica. **Confianza baja**. |
| Red/HTTP | **IMPLEMENTADA, NO VERIFICADA** | `http.rs`, dependencias (ureq etc. en tests/registry context) | Probable | Probable | No | No | No verificada. **Confianza baja**. |
| Procesos / SO | **IMPLEMENTADA, NO VERIFICADA** | No inventariadas exhaustivamente; CLI sí invoca procesos | ? | ? | No | No | **Confianza baja**. |
| Criptografía | **PARCIAL (uso interno BPK)** | SHA-256/Ed25519 en `paquetes`/BPK; como stdlib de usuario no inventariada | — | — | No | No | **Confianza media** en BPK; baja como stdlib general. |
| Concurrencia (`crear_canal/enviar/recibir/iniciar_tarea/esperar_tarea`) | **EXPERIMENTAL** | `vm.rs` (y soporte TW/canales en evaluador `CANALES`); tareas VM `TAREAS_VM` `LazyLock<Mutex<...>>` | Parcial (canales/evaluador) | Sí (primitivas) | No | No | **Flakiness observada** en diferenciales canal. |
| Conversión de datos | **PARCIAL** | conversiones tipadas/nativas puntuales; `a_texto` amplio no verificado en nativo | Parcial | Parcial | Parcial | No | |
| Entrada | **IMPLEMENTADA, NO VERIFICADA** (amplitud) | REPL/IO en CLI; nativas según motor | Parcial | Parcial | No | No | |

**Conclusión Obj 4:** stdlib **no uniforme** entre motores; núcleo verificado en VM/TW/nativo-subset, pero muchas funciones **no verificadas** por dominio o **ausentes** en nativo/WASM. **Confianza media** (alta en filas verificadas).

### Objetivo 5 — Concurrencia robusta

| Aspecto | Estado | Evidencia | Comandos/resultados | Limitaciones / confianza |
|---|---|---|---|---|
| Creación/espera de tareas | **EXPERIMENTAL** | `vm.rs` `iniciar_tarea`/`esperar_tarea`; `TAREAS_VM: LazyLock<Mutex<Vec<TareaPendiente>>>` (L29) | `cargo test --test diferencial_vm tarea` → **2 passed, 0 failed** | Primitivas acotadas. **Confianza media**. |
| Modelo de hilos | **EXPERIMENTAL** | tareas en hilos (inspección previa de vm) + registro global | Tests tarea ok; sin spec de scheduling | No equivalente a goroutines/async maduro. **Confianza baja-media**. |
| Canales (prod/cons) | **EXPERIMENTAL (con fiabilidad no demostrada)** | `crear_canal/enviar/recibir` (vm) y canales evaluador (`CANALES` LazyLock); diferenciales `diff_canal_productor_consumidor`, `diff_dos_canales` | **Workspace completo:** diferencial 81 passed (incl. canal en esa corrida) → OK en `/tmp/lote1-tests2.txt`. **Ejecuciones filtradas repetidas:** `for i in 1..5: cargo test --test diferencial_vm canal` → resultados intermitentes (`FAILED 0/2`, `ok 2/0`, `FAILED 0/2`, `ok`, `ok`). Falla capturada (intento 1): TW `OK:Some(Entero(42))`/`62` vs VM `OK:Some(Entero(10))`/`30` (IDs/valores cruzados o race/estado). | **Hallazgo crítico:** no determinismo/reproducibilidad no garantizada. **Confianza alta** en existencia; **baja** en robustez. |
| Estado compartido / mutexes / seguridad memoria | **PARCIAL** | Rust `Arc/Mutex/Condvar` en canales/tareas; modelo de memoria propio del lenguaje no verificado | Revisión de código | Seguridad depende de implementación Rust; semántica de usuario no formalmente garantizada. **Confianza media**. |
| Cancelación, timeouts | **AUSENTE / NO VERIFICADA** | No halladas garantías de cancelación/timeout en primitivas inspeccionadas | N/A | **Confianza media-baja**. |
| Errores/propagación en tareas | **IMPLEMENTADA, NO VERIFICADA** | `esperar_tarea` maneja join/error (inspección vm) | Tarea tests básicos ok | Cobertura limitada. **Confianza baja**. |
| Deadlocks / condiciones de carrera / carga | **NO VERIFICADA (riesgo evidenciado)** | Flakiness canal sugiere race/estado global | Muestreo manual (ver arriba) | No hay tests de estrés/carga. **Confianza baja** en ausencia de carreras. |

**Conclusión Obj 5:** concurrencia **experimental**, no “segura/robusta” demostrada. No confundir con concurrencia madura.

### Objetivo 6 — Interoperabilidad (FFI)

| Aspecto | Estado | Evidencia | Disponibilidad por motor | Limitaciones / confianza |
|---|---|---|---|---|
| ABI C / carga dinámica / símbolos / llamada | **IMPLEMENTADA, NO VERIFICADA (e2e)** | `src/ffi.rs` (194 líneas): `cargar_libreria`/`descargar_libreria`/`llamar_funcion`, `libloading` + `libffi` (tipos i64/f64/puntero-cadena según inspección) | **TW:** nombres presentes en `evaluador.rs` (`"cargar_ffi","llamar_ffi","descargar_ffi"`). **VM:** registrados en `vm.rs` (`cargar_ffi/descargar_ffi/llamar_ffi`). **Nativo:** **no hallado** (`grep` en `backend_nativo.rs`,`runtime.rs` sin coincidencias). **WASM:** no. | Sin caso FFI real ejecutado. **Confianza media** en implementación; **baja** en corrección/portabilidad. |
| Tipos soportados, retornos, punteros/strings/structs/callbacks | **PARCIAL / NO VERIFICADA** | Código maneja marshalling básico (i64/f64/strings `CString` observados) | TW/VM potencial | Callbacks/structs complejos/gestión de memoria avanzada **no verificados**. **Confianza baja**. |
| Gestión de memoria/errores/seguridad | **EXPERIMENTAL** | unsafe/libloading/libffi implican riesgo | — | No auditada seguridad. **Confianza baja**. |
| Interop real con C/Rust/otros | **PARCIAL (potencial C ABI), NO VERIFICADA** | FFI vía C ABI | TW/VM | “Interfaz parcial”, no integración robusta demostrada con librerías reales. **Confianza baja**. |

### Objetivo 7 — Sistema de paquetes

| Aspecto | Estado | Evidencia | Pruebas/comandos | Limitaciones / confianza |
|---|---|---|---|---|
| Manifiesto/proyecto/lockfile/resolución/semver/transitivas/conflictos/ciclos/BPK/SHA-256/Ed25519/namespaces/yank (local/parcial) | **PARCIAL** | `src/paquetes.rs` (1674), `proyecto.rs`, CLI `instalar/add/update/publish`, formato BPK, hashes/firmas en código | `cargo test --test bpk` → **2 passed**. `cargo test --test cargador` → **14 passed**. `cargo test paquetes` → **7 passed** (lib filter) | No todas las propiedades (reproducibilidad, seguridad BPK completa, resolución transitiva compleja) verificadas end-to-end. **Confianza media**. |
| Registro remoto, publicación/descarga remota autenticada, integración | **BLOQUEADA POR DEPENDENCIAS** | `tests/registry_integration.rs` (2 tests `#[ignore]`): requieren `DATABASE_URL` (PostgreSQL local aislado, no template/system DB), `psql`, `basilisco-registry` (`CARGO_BIN_EXE_basilisco-registry`), bind `127.0.0.1`, HTTP; hacen bootstrap admin, namespace, publish/download/install, reinicio, concurrencia | `cargo test --test registry_integration ciclo_publicacion_descarga_instalacion -- --ignored --nocapture` → **FAILED** (exit 101), `panicked at ...:123:19: DATABASE_URL no definida; no se puede ejecutar el E2E`. `concurrencia_misma_version` → **FAILED**, pánico `:312:19` misma causa. Entorno: `DATABASE_URL` vacío, `psql` no disponible, `basilisco-registry` presente en `target/debug/`. | Infraestructura externa no disponible; **no aprobadas**. **Confianza alta** en causa. |
| Gestor funcional para usuarios externos | **NO DEMOSTRADO (prototipo/parcial)** | combinación de lo anterior | — | Integración remota no verificable aquí. **Confianza media**. |

### Objetivo 8 — Backend WASM

| Aspecto | Estado | Evidencia | Comandos/resultados | Limitaciones / confianza |
|---|---|---|---|---|
| Qué genera (Rust intermedio vs Wasm directo) | **PARCIAL (transpilación a Rust intermedio)** | `src/backend_wasm.rs` (23 líneas): `generar_rust_wasm` envuelve salida de `backend_entero::generar_funciones_enteras` y reemplaza `fn bsk_` por `#[no_mangle]\npub extern "C" fn bsk_`; añade `#![no_main]`. **No** genera `.wasm`/WAT directamente. | Lectura de código; tests inspeccionados | **No es backend Wasm “completo/directo”**. **Confianza alta**. |
| Produce `.wasm` / necesita `rustc` | **IMPLEMENTADO (vía rustc), NO VERIFICADO por suite en toolchain actual** | Tests usan `rustc --target wasm32-unknown-unknown --edition 2024 -O` → `gen.wasm`; requieren `node` | `rustc 1.99.1`, `node v24.20.0`, target `wasm32-unknown-unknown` disponible. **`cargo test --test backend_wasm`** → `5 passed, 0 failed` (harness), PERO con `--nocapture` `backend_wasm_aritmetica` muestra `rustc stderr: error: unsafe attribute used without unsafe` sobre `#[no_mangle]` y el test hace `let Some(...) else { return; }` → **pasa vacuamente (skip silencioso)**. Confirmado aislado: compilar Rust con `#[no_mangle]` a wasm32 → **exit 1, sin `.wasm`**. | **Falsos positivos** en suite: 2 tests son string-only (válidos para su aserción), 3 de ejecución **no verifican** compilación/ejecución real en esta toolchain. **Confianza alta**. |
| Ejecución real end-to-end (programa real) | **DEMONSTRADA FUERA DEL REPO (temp), NO por backend actual sin ajuste** | Pasos registrados (todo en `/tmp`, sin tocar repo): (1) fuente Rust `bsk_main` suma 1..10; (2) `rustc --target wasm32-unknown-unknown --edition 2024 -O /tmp/audit_wasm_real.rs -o /tmp/audit_wasm_real.wasm` con `#[no_mangle]` → **FALLA** (`unsafe attribute`, exit 1). (3) misma fuente con `#[unsafe(no_mangle)]` (ajuste solo en `/tmp/audit_wasm_real_fix.rs`, **no en repo**) → **OK** (exit 0), genera `/tmp/audit_wasm_real_fix.wasm` (~1.6M). (4) Node `v24.20.0` carga `WebAssembly.Module/Instance` y llama `inst.exports.bsk_main()` → imprime **`55`** (exit 0). | Comandos/resultados como arriba | Demuestra viabilidad del **pipeline** (rustc→wasm+Node) para el subset; **el código generado por el backend del repo no compila tal cual** en esta toolchain por la forma del atributo. **Confianza alta**. |
| Instrucciones/tipos/strings/colecciones/indirectas/memoria dinámica/errores | **EXPERIMENTAL (subset mínimo)** | `backend_entero` (613), sin `String`/E/S por ABI; solo entero/lógico/control/llamadas directas del subset; sin colecciones/strings/memoria dinámica/errores ricos | 5 tests (intención) cubren aritmética/recursión/loop/cond; ejecución real no verificada por suite actual | **No** llamar completo. **Confianza media**. |
| Node.js / navegador | **Node: PARCIAL (demostrado en e2e manual temp)** / **Navegador: NO VERIFICADO** | Host Node en tests/e2e manual | Node devolvió `55` | Navegador no probado. **Confianza media (Node)** / **baja (browser)**. |
| Paridad con VM | **NO DEMOSTRADA (en estado actual)** | Tests pretenden comparar con VM pero se saltan ante fallo de compilación | — | Paridad reclamada no verificada por ejecución. **Confianza baja**. |

### Objetivo 9 — Herramientas de desarrollo

| Herramienta/capacidad | Estado | Evidencia | Prueba end-to-end / comando | Limitaciones / confianza |
|---|---|---|---|---|
| CLI (`run/analizar/formatear/probar/depurar/build/nuevo/lsp/pkg/...`, `--motor`) | **PARCIAL (usable en comandos clave)** | `src/cli.rs` (1865), `--help` | `basilisco --help` OK (lista comandos/opciones). `run --motor=arbol|vm` y `build` verificados en ejemplos (diff 0). `analizar`/otros existen (no todos ejercitados aquí) | Madurez heterogénea. **Confianza alta** en help/run/build; **media** global. |
| LSP (diagnósticos, completion, hover, goto def, refs, rename) | **EXPERIMENTAL / IMPLEMENTADA NO VERIFICADA e2e** | `src/lsp.rs` (337 líneas); comando `basilisco lsp` (stdio) | **No** se realizó sesión LSP JSON-RPC completa en esta auditoría (protocolo stdio no trivial en un paso); inspección de módulo indica presencia parcial | Distinguir “existe” de “utilizable en IDE”: **no demostrado** end-to-end. **Confianza baja-media**. |
| Diagnósticos (CLI/análisis) | **PARCIAL** | semántico/parser/errores tipados; `analizar`/`run` reportan | Ejemplos/errores observados en sesiones | Cobertura no total. **Confianza media**. |
| Formateador | **IMPLEMENTADA Y VERIFICADA (smoke)** | `src/formateador.rs` (428), `cli formatear` | `cp ejemplos/consola/main.bsk /tmp/fmt_test.bsk && basilisco formatear /tmp/fmt_test.bsk` → `formateado: /tmp/fmt_test.bsk`, exit 0 (repo no modificado) | No verificada idempotencia completa ni todos los nodos. **Confianza media**. |
| Linter | **AUSENTE / NO VERIFICADO como producto separado** | clippy aplica al compilador Rust; linter de BASILISCO no identificado como herramienta de usuario equivalente | — | **Confianza media**. |
| Depurador (traza) | **EXPERIMENTAL (traza básica, verificada smoke)** | `depurar` en CLI; traza VM/funciones (`vm.activar_traza`/salida) | `timeout 5 basilisco depurar ../ejemplos/consola/main.bsk` → imprime resultado + traza `→ main()/factorial(...)` y retornos (verificado) | **No** breakpoints, step over/into/out, inspección interactiva, DAP. **Confianza alta** en traza básica; ausente en resto. |
| Breakpoints / stepping / inspección variables | **AUSENTE (demostrado)** | no hallado flujo interactivo equivalente | depurador es traza | **Confianza alta**. |
| Extensión VS Code | **IMPLEMENTADA, NO VERIFICADA** | posible existencia fuera de `prototipo/src`/no auditada e2e (no ejercitada) | N/A | **Confianza baja**. |
| Documentación | **PARCIAL** | `README`, `docs/`, `10-especificacion/` abundantes; pueden desfasarse (histórico) | Se usó como referencia, no como prueba | **Confianza media**. |
| Instalación/actualización | **PARCIAL** | build/install scripts en árbol (no verificados e2e aquí); CLI `instalar/update` | `bpk`/paquetes tests parciales; update/install no ejecutados contra red | **Confianza baja**. |

## 3. Matriz de capacidades por motor

| Capacidad | Tree-Walk | VM (MIR, default) | Backend nativo | Backend WASM | Notas clave |
|---|---:|---:|---:|---:|---|
| Núcleo lenguaje (control, funciones básicas, numéricos/decimal estricto) | Verificado | Verificado | Parcial (verificado subset) | Experimental (subset entero; ejecución suite no verificada) | 874 tests workspace; 5 ejemplos TW=VM=nativo diff 0. |
| Módulos `usar` (alias/nominal/anidados) | Verificado | Verificado | Verificado (objetivo) | No | Nativo: doble inline riesgo. |
| Closures/capturas/celdas | Verificado | Verificado | **No soportado** | No | 7 closure + 3 lambda diferenciales TW↔VM ok. |
| Llamadas indirectas | Verificado | Verificado | Parcial (`Valor::Funcion` dispatch) | No | |
| try/catch | Parcial | Parcial | No | No | Nativo rechaza. |
| Concurrencia (canal/tarea) | Parcial (canales) | Experimental (flaky observada) | No | No | Canal diferencial intermitente. |
| FFI | Impl. no verificada e2e | Impl. no verificada e2e | No | No | `ffi.rs` + registros TW/VM. |
| Stdlib (amplitud) | Parcial-amplia | Parcial-amplia | Subset | Mínimo | Nativo: leer_archivo/json_parsear/E/S/colec básicas. |
| Salida binaria | N/A | N/A (interpreta) | Sí (Rust→rustc) | `.wasm` vía rustc (pipeline demostrado en temp; backend repo no compila tal cual aquí) | Nativo/wasm dependen de rustc. |

## 4. Matriz de pruebas

| Suite / prueba | Comando | Resultado | Cobertura/observación |
|---|---|---|---|
| Workspace completo | `cargo test --workspace --no-fail-fast` (`prototipo/`, `/tmp/lote1-tests2.txt`) | **874 passed, 0 failed, 2 ignored** (53) | Regresión general; incluye diferencial 81, backend_nativo 45, backend_wasm 5 (ver obj 8), bpk 2, cargador 14. |
| Formato | `cargo fmt --check` | **OK** (exit 0) | Estado actual. |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | **OK** | |
| Integración registro (forzada) | `cargo test --test registry_integration <test> -- --ignored --nocapture` | **2 FAILED** (0/1 cada uno) | Causa: `DATABASE_URL` no definida; + deps psql/PostgreSQL aislado/registry. **Bloqueada**. |
| BPK | `cargo test --test bpk` | **2 passed** | Formato/paquete local. |
| Cargador | `cargo test --test cargador` | **14 passed** | Resolución/carga módulos. |
| Paquetes (lib filter) | `cargo test paquetes` | **7 passed** (filtro relevante) | Subconjunto. |
| Diferencial closures | `cargo test --test diferencial_vm closure` | **7 passed** | TW↔VM. |
| Diferencial lambdas | `cargo test --test diferencial_vm lambda` | **3 passed** | TW↔VM. |
| Diferencial tareas | `cargo test --test diferencial_vm tarea` | **2 passed** | TW↔VM. |
| Diferencial canales (filtrado, repetido) | `cargo test --test diferencial_vm canal` (5 corridas) | **Intermitente: 3 FAILED / 2 OK**; falla capturada con divergencia TW vs VM (42/62 vs 10/30) | **Flakiness crítica**; workspace pasó en su corrida. |
| Backend WASM (suite) | `cargo test --test backend_wasm` / con `--nocapture` | Harness: **5 ok**; pero 3 tests de ejecución **saltan por fallo de rustc** (`#[no_mangle]` vs rustc 1.99.1) y aún marcan ok | **Verificación real no lograda** por suite en este estado. |
| WASM e2e manual (temp) | rustc wasm32→`.wasm` + `node` export `bsk_main` | **OK con ajuste atributo solo en `/tmp`** → salida `55`; **FALLA** con atributo generado por repo | Viabilidad pipeline; incompatibilidad actual del generador. |
| Herramientas smoke | `formatear /tmp`, `depurar consola`, `--help` | **OK** | Sin LSP e2e IDE. |

## 5. Dependencias arquitectónicas entre objetivos

1. **Obj 1 (tipos) → 2,3,4,8:** `TipoSem`/inferencia/verificadores alimentan lowering y codegen; `Desconocido`/permisividad propaga incertidumbre a nativo/WASM.
2. **Obj 2 (abstracciones de usuario) → 1,4,7,9:** ausencia de genéricos/ADTs/traits limita expresividad, modelado de stdlib, APIs de paquetes y tooling (completion/rename semántico).
3. **Obj 3 (closures) → 5,8,nativo:** celdas/clausuras dependen de representación de entorno; nativo/WASM carecen de ese soporte → paridad incompleta.
4. **Obj 4 (stdlib) → 6,7,8:** I/O, JSON, HTTP, cripto condicionan FFI/paquetes/WASM; desuniformidad por motor complica portabilidad.
5. **Obj 5 (concurrencia) → 1,4, runtime:** requiere modelo de memoria/errores claro; estado global (`TAREAS_VM`/canales) impacta determinismo de pruebas.
6. **Obj 6 (FFI) → 4,8,seguridad:** ABI/marshalling depende de tipos/runtime; no portable a WASM/nativo actual.
7. **Obj 7 (paquetes) → 4,6,9,infra:** resolución/registro dependen de stdlib red/cripto y de infraestructura (DB/servicio) — hoy bloquea verificación remota.
8. **Obj 8 (WASM) → 1,3,4,toolchain:** subset entero vía `backend_entero`; closures/strings/colecciones bloqueadas; depende de rustc/Node y de validez del código generado.
9. **Obj 9 (tooling) → 1,2:** LSP/rename/refactor precisos dependen de semántica/tipos/abstracciones estables.

## 6. Hallazgos críticos

1. **WASM: suite con skips silenciosos + generador incompatible con rustc 1.99.1** (`#[no_mangle]` vs atributo unsafe-edition) → ejecución/paridad WASM **no verificada**; riesgo de falsa confianza en CI/local. Evidencia: `src/backend_wasm.rs`, `tests/backend_wasm.rs`, nocapture, compilación aislada.
2. **Concurrencia (canales) no determinista:** diferenciales canal intermitentes (3/5 fallos en muestreo) con valores VM incorrectos frente a TW (42/62 vs 10/30) → posible race/estado global. Evidencia: re-ejecuciones y traza de pánico capturada.
3. **Verificación de formato es sensible pero actualmente OK**; historial reciente mostró divergencias (resueltas en Lote 1) — mantener gate. (Contexto: `fmt --check` OK ahora.)
4. **Integración de registro no verificable sin infra:** 2 tests ignorados fallan forzados por falta de `DATABASE_URL`/PostgreSQL aislado/`psql`; no cuentan como aprobados.
5. **Brecha de “propósito general”:** sin genéricos/ADTs/traits de usuario demostrables; backend nativo/WASM parciales; cierres no portables a nativo.

## 7. Riesgos de diseño

- **Paridad multi-motor divergente:** subsets distintos (nativo/wasm) y estado global en concurrencia aumentan semánticas no equivalentes.
- **`TipoSem::Desconocido` permisivo + verificadores `bool` no demostrados como gates** pueden dejar errores sin detectar hasta runtime.
- **Estado global (`TAREAS_VM`, canales)** y diseño de IDs/colas: fuente probable de flakiness/races.
- **Dependencia de toolchains externas** (rustc para nativo/WASM, Node para wasm host) acopla “portabilidad” a versiones de herramientas.
- **FFI unsafe/libffi**: riesgos de seguridad, UB y portabilidad sin suite e2e.
- **Escalabilidad del inlining de módulos** (duplicación potencial) y concentración de lógica en archivos grandes (`evaluador`, `vm`, `hir`).

## 8. Capacidades demostradas

- Ejecución verificada del núcleo en workspace: **874 passed, 0 failed, 2 ignored**; `clippy` limpio; `fmt` OK (estado actual).
- Paridad **TW=VM** en diferenciales cubiertos (aritmética, control, tipos/conversiones GAP-051, funciones, 7 closures, 3 lambdas, 2 tareas) — con salvedad de **canales flaky**.
- Paridad **TW=VM=nativo** en 5 ejemplos reales (consola, calculadora, gestor_tareas, inventario, punto_venta) bajo condiciones probadas.
- Módulos `usar` (alias/nominal/anidados) + globals/`__init`/indirectas en nativo (subset objetivo) con tests de módulo nativo (4) pasando en suite.
- BPK/cargador/paquetes (subset local): 2 + 14 + 7 tests pasando.
- Herramientas: `--help`, `formatear` (smoke), `depurar` traza básica (smoke).
- Pipeline WASM factible (demostración controlada en `/tmp`): rustc→`.wasm` + Node → resultado correcto `55`.

## 9. Capacidades no demostradas

- Genéricos de usuario, structs/enums ADT, traits/interfaces, type aliases de usuario, monomorfización de usuario.
- Closures/celdas, try/catch, rangos, stdlib amplia y FFI en **backend nativo**; WASM más allá de subset entero mínimo.
- **Ejecución/paridad WASM real** con el generador actual del repo en esta toolchain (suite se salta).
- Concurrencia **robusta/determinista** (canales flaky; sin cancelación/timeout/carga verificados).
- FFI end-to-end con librerías reales; disponibilidad en nativo; seguridad.
- Registro remoto/publicación-descarga autenticada end-to-end (bloqueado por infra).
- LSP completo en IDE (completion/hover/goto/refs/rename) y depurador interactivo (breakpoints/stepping/inspección).
- Comportamiento en navegador; directorios/fecha-hora/red/procesos/SO exhaustivos por motor.

## 10. Prioridades técnicas

1. **(P0) Fiabilidad de pruebas WASM:** eliminar skips silenciosos (fallar si compilación esperada falla o marcar skip explícito con razón/dependencia) y alinear generador de atributos con toolchain soportada; verificar ejecución `.wasm`+Node real.
2. **(P0) Determinismo de concurrencia:** aislar estado global de canales/tareas por prueba/VM, revisar IDs/colas; convertir diferenciales canal en deterministas o diagnosticar race; añadir pruebas de estrés.
3. **(P1) Paridad y gates de tipos:** hacer verificadores HIR/MIR comprobables (tests de contrato, integración opcional en pipeline) y reducir zonas `Desconocido` críticas.
4. **(P1) Nativo:** roadmap explícito para closures/celdas o rechazo diagnóstico uniforme; ampliar stdlib con matriz por función; resolver doble-inline de módulos.
5. **(P1) Diseño de abstracciones (Obj 2):** especificación e implementación incremental (ADT/`match`/tipos nominales, luego genéricos/traits) con suite de conformidad.
6. **(P2) Paquetes/registro:** harness reproducible (contenedor/servicio efímero o mocks) para ejecutar integración sin depender de entorno ad-hoc; mantener separadas de unit tests.
7. **(P2) FFI:** casos e2e controlados, matriz de tipos/ABI, hardening.
8. **(P2) Tooling:** LSP e2e (protocol tests), capacidades reales documentadas; depurador más allá de traza si se requiere.

## 11. Propuesta de fases de desarrollo

- **Fase A — Higiene y veracidad (corto):** arreglar WASM test-harness (no false-green), reproducir/fijar flakiness de canales, fijar gates `fmt/clippy/test` y documentar matriz de soporte por motor.
- **Fase B — Núcleo semántico sólido (corto-medio):** contratos de verificadores, endurecer tipado/conversiones, matriz stdlib por motor, paridad diferencial ampliada (módulos complejos, errores).
- **Fase C — Compilación nativa útil (medio):** cerrar brechas prioritarias (decisión sobre closures/try/rangos), dedup módulos, ampliar runtime embebido.
- **Fase D — Abstracciones de usuario (medio-largo):** ADTs/`struct/enum` nominales + `match`, luego genéricos/traits con especificación y pruebas.
- **Fase E — Plataforma (largo):** concurrencia robusta, WASM directo o subset bien definido y verificado, FFI segura, registro de paquetes con CI de integración, LSP/debugger maduro.

---
*Informe generado en modo solo lectura. Comandos de verificación ejecutados en `prototipo/` (salvo indicación); artefactos temporales en `/tmp` y compilación en `target/`.*
