# Auditoría general — IR, VM, backends, optimizador y formateador

**Fecha:** 2026-11-02  
**Alcance:** código real de `prototipo/src/{hir,mir,vm,optimizador,backend_nativo,backend_wasm,formateador}.rs` y sus pruebas.  
**Método:** inspección directa del código + ejecución de `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`.

---

## 1. Resumen ejecutivo

- **Estado de la cadena:** HIR → MIR → VM → optimizador → backend nativo → backend WASM está implementada y compila limpia. Formateador idempotente.
- **Pruebas:** `cargo test` → **737 passed, 0 failed**. Clippy y fmt en verde.
- **Alcance real:** la VM y los backends solo cubren un **subconjunto** del lenguaje (funciones con tipos `entero`/`lógico`, control de flujo básico, arreglos/listas por índice, texto por concatenación). El Tree-Walk (`evaluador.rs`, 6166 líneas) sigue siendo el único motor con cobertura completa.
- **Riesgo principal:** dos motores que deben mantenerse sincronizados; la suite diferencial (14 casos) es pequeña frente a la suite de conformidad (≈626 tests).

---

## 2. Inventario de módulos (LOC)

| Módulo | LOC | Rol |
|---|---|---|
| `evaluador.rs` | 6166 | Tree-Walk de referencia (sin cambios estructurales). |
| `parser.rs` | 2073 | Parser (sin cambios). |
| `semantico.rs` | 1512 | Análisis semántico (sin cambios). |
| `hir.rs` | 851 | HIR + lowering AST→HIR + `ResolutorHir`. |
| `mir.rs` | 938 | MIR + lowering HIR→MIR + `VerificadorMir`. |
| `vm.rs` | 639 | VM + FFI por cierre. |
| `optimizador.rs` | 285 | Pases de optimización. |
| `backend_nativo.rs` | 252 | Codegen Rust (nativo). |
| `backend_wasm.rs` | 34 | Codegen Rust (WASM, delega en nativo). |
| `formateador.rs` | 428 | Formateador AST→fuente. |
| `simbodo.rs` | 180 | IDs de símbodos. |
| `lib.rs` | 34 | Registro de módulos. |

---

## 3. Cobertura real por capa

### 3.1 HIR (`src/hir.rs`)

- **`lower_sentencia`**: maneja todos los 18 variantes de `Sentencia` del AST (`Declaracion`, `Asignacion`, `AsignacionIndice`, `Si`, `Mientras`, `Bloque`, `Expresion`, `Funcion`, `Retorno`, `Mostrar`, `MostrarSinSalto`, `ParaCada`, `Romper`, `Continuar`, `IntentarRescatar`, `Importar`, `Exportar`, `Usar`). Ninguno cae al catch-all en la práctica.
- **`lower_expresion`**: maneja 24 de 25 variantes de `Expresion`. Única caída: `Rango` (se maneja dentro de `ParaCada` vía `MirRvalue::Rango`, pero como expresión suelta baja a `Otro`).
- **Funciones anidadas (closures):** NO soportadas. `Sentencia::Funcion` dentro de otra función cae al catch-all `"sentencia AST no soportada en HIR"` (línea ~677). **Las closures no se bajan.**
- **`ResolutorHir`**: pila de ámbitos con nombres, asignación de `SimbodoId` desde base `1_000_000`. Pre-paso registra funciones de primer nivel (llamadas anticipadas/recursión).
- **`VerificadorHir`**: trivial (siempre `true`); no valida nada.

### 3.2 MIR (`src/mir.rs`)

- **`lower_stmt`**: maneja 12 de 18 `HirSentencia`: `Declaracion`, `Asignacion`, `AsignacionIndice`, `Bloque`, `Continuar`, `Expresion`, `IntentarRescatar`, `Mientras`, `ParaCada`, `Retornar`, `Romper`, `Si`. **Ignora:** `Exportar`, `Importar`, `Usar`, `Mostrar`, `MostrarSinSalto` (estos dos últimos ya son `Expresion` en HIR). Los módulos se descartan silenciosamente.
- **`lower_expr`**: maneja 19 variantes; faltan `MayorIgual`, `MenorIgual` (corregidos en esta sesión), `Acceso` (corregido), `Rango` (corregido). Ahora cubre todas excepto `Otro`.
- **`Lambda`**: baja a `MirConst::Nulo` (placeholder). **Las clausuras no se materializan.**
- **`Si`-expresión** (`HirExpresion::Si`): baja a `Nulo`.
- **`ParaCada`**: implementado como bucle por índice sobre el iterable (arreglo/lista/texto) con `MirRvalue::Longitud` y `MirRvalue::Indice`.
- **`Romper`/`Continuar`**: implementados (Goto al bloque de salida/condición del bucle).
- **`IntentarRescatar`**: **RESUELTO** — genera bloques intentar/rescatar/merge con `MirStmt::EntrarTry`/`SalirTry`; la VM captura cualquier error y ejecuta el bloque de rescate, vinculando el mensaje a la variable de captura.
- **`VerificadorMir`**: valida que haya bloques; no valida terminadores ni alcanzabilidad.

### 3.3 VM (`src/vm.rs`)

- Maneja todos los `MirStmt` y `MirTerminator`.
- `MirRvalue`: `Use`, `BinOp`, `UnOp`, `Agregado`, `Indice`, `Campo`, `Longitud`, `Rango` (implementados). 
- **Proyecciones anidadas** en `MirPlace`: solo un nivel (`[]`, `[Campo]`, `[Indice]`); las anidadas devuelven error.
- **FFI**: `registrar_nativo` por cierre; prioridad MIR > nativo. No integra ABI C.
- **Límites:** `LIMITE_PASOS_DEFAULT = 1_000_000`, `LIMITE_PROFUNDIDAD_DEFAULT = 1_000`.

### 3.4 Optimizador (`src/optimizador.rs`)

- Plegado de constantes (BinOp/UnOp) con propagación local por bloque.
- Simplificación de `SwitchInt` con discriminante constante → `Goto`.
- Eliminación de `Nop`.
- Punto fijo (máx. 8 iteraciones).
- **No implementa:** DCE real, inlining, propagación inter-bloque, SSA.

### 3.5 Backends (`backend_nativo.rs`, `backend_wasm.rs`)

- Soportan solo funciones con tipos `entero`/`lógico`/`Desconocido`.
- `MirRvalue::Agregado`, `Indice`, `Campo`, `Longitud`, `Rango` → `"0"` (placeholder).
- `MirStmt::LlamadaIndirecta` → comentario (no soportado).
- `MirTerminator::Invoke` → `__ret = 0; break`.
- WASM delega en nativo (anota `#[no_mangle] pub extern "C"`).

### 3.6 Formateador (`src/formateador.rs`)

- Idempotente. Maneja todos los `Sentencia`. Soporta `publico`.
- Ignora comentarios (no los preserva, como es esperable).
- `tipo_str` usa `"logico"` (sin acento) — consistente con el keyword del parser.

---

## 4. Pruebas

| Suite | Tests | Cobertura |
|---|---|---|
| `tests/hir.rs` | 8 | Lowering HIR básico. |
| `tests/mir.rs` | 3 | Estructura de bloques. |
| `tests/vm.rs` | 11 | Ejecución VM + FFI por cierre. |
| `tests/optimizador.rs` | 8 | Paridad con/sin optimización. |
| `tests/diferencial_vm.rs` | 14 | Paridad Tree-Walk vs VM. |
| `tests/backend_nativo.rs` | 4 | Codegen nativo + compilación. |
| `tests/backend_wasm.rs` | 4 | Codegen WASM + compilación. |
| `tests/formateador.rs` | 6 | Idempotencia + publico. |
| **Nuevas (esta sesión)** | **46** | — |
| **Total proyecto** | **737** | Incluye ~626 tests preexistentes. |

La suite diferencial cubre: aritmética, control de flujo, recursión, arreglos (lectura/escritura por índice), texto, lógicos, errores (división por cero, desbordamiento), `para cada`, `intentar/rescatar`. **No cubre:** closures, módulos, archivos, FFI, fechas, objetos, ubicaciones de error, orden observable de operaciones, recursos.

---

## 5. Defectos conocidos (con evidencia)

| # | Defecto | Evidencia |
|---|---|---|
| 1 | **Closures: solo capturas inmutables por valor.** Las clausuras que mutan una variable capturada (`x = x + 1`) no propagan la mutación al entorno exterior porque las capturas se copian al crear la clausura. Requiere celdas de referencia (`RefCell`/`Arc<Mutex>`) en `Valor`. | `src/hir.rs` (variables_libres, lowering de Lambda/Funcion anidada); `src/vm.rs` (LlamadaIndirecta con ClausuraMir). Test `diff_closure_mutable` marcado `#[ignore]`. |
| 2 | ~~**Módulos ignorados en MIR.**~~ — **PARCIALMENTE RESUELTO.** Inlining en tiempo de compilación. `importar ... como alias` con acceso `alias.func` no soportado. | `src/hir.rs`: `cargar_modulo_items`. |
| 3 | ~~**`Romper`/`Continuar` son Nop.**~~ — **RESUELTO.** Ahora generan `Goto` al bloque de salida/condición del bucle actual. | `src/mir.rs`: arms `HirSentencia::Romper` / `Continuar`. |
| 4 | ~~**`IntentarRescatar` ejecuta solo el bloque `intentar`.**~~ — **RESUELTO** mediante marco de excepción y salto a bloque de rescate. | `src/mir.rs`, `src/vm.rs`: `MirStmt::EntrarTry`/`SalirTry`. |
| 5 | **`Si`-expresión baja a Nulo.** | `src/mir.rs` lower_expr: `HirExpresion::Si { ... } => MirOperand::Const(MirConst::Nulo)`. |
| 6 | **`Lambda` baja a Nulo.** | `src/mir.rs` lower_expr: `HirExpresion::Lambda => MirOperand::Const(MirConst::Nulo)`. |
| 7 | **Proyecciones anidadas no soportadas** en VM. | `src/vm.rs` escribir_place: `_ => Err(... "proyección anidada")`. |
| 8 | **Backends no soportan tipos ricos.** Agregado/Indice/Campo/Longitud/Rango → `"0"`. | `src/backend_nativo.rs` generar_rvalue. |
| 9 | **`VerificadorMir`/`VerificadorHir` triviales.** Siempre `true`. | `src/mir.rs` ~786, `src/hir.rs` ~147. |
| 10 | **FFI de la VM no integra ABI C.** Solo cierres Rust. | `src/vm.rs`: `registrar_nativo` toma `FuncionNativa` (Arc de cierre). |
| 11 | **`unwrap()` en código de producción** (vm.rs líneas 352, 413) — protegidos por guards, pero frágiles. | `src/vm.rs`. |
| 12 | **`HirExpresion::Acceso` (acceso a miembro)** manejado en MIR pero el VM solo resuelve objetos; arreglos no tienen campos. | `src/vm.rs` evaluar_rvalue `Campo`. |

---

## 6. Decisiones de diseño relevantes

- **Nombres fuente en MIR** (cambio de `f_{id}` a nombre real): mejora legibilidad y FFI, pero introduce riesgo de colisión si hay funciones homónimas en ámbitos anidados. El índice de la VM usa "la primera gana" (`or_insert`). **Limitación documentada, no resuelta.**
- **`ResolutorHir` con base `1_000_000`**: evita colisión con IDs de `TablaSimbodo`, pero los IDs no son estables entre compilaciones si cambia el orden del AST. Aceptable para prototipo.
- **MIR no es SSA.** Variables se reasignan; el optimizador es conservador (propagación solo intra-bloque).
- **VM interpreta MIR directamente** (sin bytecode). No hay formato serializable ni caching.
- **Tree-Walk sigue siendo la referencia.** La VM y los backends son prototipos; no se ha sustituido nada.

---

## 7. Dependencias y toolchain

- Rust 1.99.1 (cargo 1.99.1). Sin dependencias nuevas introducidas en esta sesión (solo `std`).
- Tests de backend requieren `rustc` y `node` en PATH (para WASM) y `cc` (para FFI preexistente).
- PostgreSQL accesible vía Docker (`basilisco_test`) — no usado en esta sesión.

---

## 8. Estado de validación

| Comando | Resultado |
|---|---|
| `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅ |
| `cargo test` | ✅ 737 passed, 0 failed |

---

## 9. Recomendaciones priorizadas (post-auditoría)

1. ~~**Closures mutables**~~ — **RESUELTO.** Implementado mediante celdas de referencia (`Valor::Celda`, `MirRvalue::CrearCelda`/`LeerCelda`, proyección `MirProjection::Celda`). Las capturas mutables ahora se comparten y propagan. `diff_closure_mutable` en verde.
2. **Ampliar suite diferencial** a closures mutables, módulos, archivos, FFI, fechas, objetos, ubicaciones de error. Sin esto, la paridad es una afirmación, no una evidencia.
3. ~~**`Romper`/`Continuar`**~~ — **RESUELTO.** Implementado con pila de bucles (`ctx.bucles`) en el lowering: `Romper` → `Goto(salida)`, `Continuar` → `Goto(condición)`. `diff_romper_continuar` en verde.
4. ~~**Modelo de excepciones**~~ — **RESUELTO** (`MirStmt::EntrarTry`/`SalirTry`, marco de excepción en la VM; `rescatar` se ejecuta con la variable de error vinculada).
5. ~~**Módulos en MIR**~~ — **PARCIALMENTE RESUELTO.** Inlining en tiempo de compilación (`cargar_modulo_items`): `usar "ruta" { nombres }` importa las funciones nombradas; `importar` inlina todas. Limitación: `importar ... como alias` + acceso `alias.funcion()` no crea objeto de módulo.
6. **Tipos ricos en backends** solo cuando la VM tenga paridad completa.
7. **Benchmarks** (GAP-021) para justificar el pipeline.

---

## 10. Conclusión

El trabajo de esta sesión convierte a BASILISCO de "lenguaje interpretado con IR diseñado" a "lenguaje con cadena de compilación funcional para un subconjunto". **No es todavía un compilador.** La VM y los backends son prototipos que cubren enteros, control de flujo, arreglos por índice, texto y clausuras (inmutables y mutables). La inversión es estratégicamente correcta, pero el **valor inmediato para el usuario es cero** hasta que se cierre la paridad funcional (módulos, archivos, FFI, fechas, objetos, ubicaciones de error).

El código es limpio (clippy/fmt en verde), bien estructurado y testeado dentro de su alcance, pero **la suite de conformidad (626 tests) no pasa por el nuevo pipeline** — 48 casos diferenciales lo validan (stdlib de texto/colecciones/JSON/archivos/conversiones, clausuras con capturas de variables, orden superior map/filtrar/reducir y modelo de excepciones). Esa es la brecha que debe cerrarse antes de considerar el pipeline "listo".