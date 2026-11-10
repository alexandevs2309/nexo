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

El código es limpio (clippy/fmt en verde), bien estructurado y testeado dentro de su alcance, pero **la suite de conformidad (626 tests) no pasa por el nuevo pipeline** — 55 casos diferenciales lo validan (stdlib de texto/colecciones/JSON/archivos/conversiones, clausuras con capturas de variables, orden superior map/filtrar/reducir y modelo de excepciones). Esa es la brecha que debe cerrarse antes de considerar el pipeline "listo".

## Nota: módulos anidados (DT-12)

- Los módulos anidados (A importa de B, C importa de A) requieren que el
  importador declare explícitamente todas las dependencias transitivas:
  `usar "lib.bsk" { triple, doble }`. La inlining automática de dependencias
  transitivas queda como trabajo futuro.

## DT-06/07: backends nativo y WASM con soporte de texto (2026-11-07)

**Backend nativo (DT-06) — soporte de texto real:**

- Tipos soportados: `entero`, `lógico` y `texto` (parámetros anotados, locales
  inferidos y retorno). Los locales `Desconocido` se refinan con inferencia de
  tipos en dos niveles: (1) punto fijo a nivel de programa para los retornos de
  cada función, (2) punto fijo por función para los locales, incluyendo los
  destinos de `Llamada`.
- Concatenación texto+texto vía `format!` (`String + String` no compila en
  Rust). Comparaciones de texto (`==`, `!=`, `<`, …) usan `PartialEq`/`Ord`.
- E/S: `mostrar` → `println!`, `mostrar_sin_salto` → `print!`; el destino de
  la llamada queda en su valor nulo (0), igual que en la VM.
- Escape correcto de literales: `\`, `"`, `\n`, `\r`, `\t`.
- **Paridad estricta:** la suma mixta texto/entero NO se convierte
  silenciosamente. La VM la rechaza (error aritmético) y el análisis semántico
  la rechaza antes de llegar al backend; además el gate `sumas_homogeneas`
  rechaza la función como defensa en profundidad.
- **Nunca salida silenciosa:** si `main` no es generable (llamadas a nativas de
  stdlib como `longitud`, colecciones, excepciones, …), el `.rs` generado
  contiene `compile_error!(...)` — el build falla con diagnóstico explícito.
- El punto de entrada solo imprime el valor de retorno de `main` si retorna
  valor (declarado o inferido); si `main` no retorna, solo la ejecuta (igual
  que el CLI con la VM).
- Tests diferenciales nuevos (7): concatenación, parámetro texto, escape,
  condicional sobre texto, texto en bucle, salida de `mostrar`, rechazo de no
  generables.

**Backend WASM (DT-07):**

- `generar_rust_wasm` ahora usa el subconjunto **puro entero/lógico**
  (`generar_funciones_enteras`): las funciones con `texto` o E/S se omiten con
  comentario, porque `String` no cruza el ABI C de los exports WASM. Test
  nuevo: `backend_wasm_omite_funciones_texto`.

**Hallazgo (pendiente, front-end):** el análisis semántico rechaza `"texto" +
entero` con el código E1007 y mensaje "comparación incompatible entre texto y
entero" — el comportamiento es correcto (rechaza) pero el mensaje/código es
confuso para una suma; debería ser E1001 ("tipos incompatibles") o similar.
Queda como deuda menor de mensajes de error.

## G4: comandos de paquetes reales — add, update, publish con BPK (2026-11-07)

Se eliminaron los tres stubs del CLI de paquetes (antes solo imprimían
"pendiente de implementación completa"). El ecosistema de paquetes queda
operativo de extremo a extremo: `inicializar → add → instalar → update →
publish`, con verificación de integridad y firmas Ed25519 en cada paso.

**`bsk add <paquete> [versión] [--ruta=<ruta>]` (DT-21):**

- Valida nombre (`nombre_valido`) y versión (`version_valida`, ahora `pub`).
- Rechaza duplicadas consultando el manifiesto cargado.
- **Pre-verifica resolubilidad** con `resolver_dependencia` (ahora `pub`)
  *antes* de tocar el manifiesto: si la dependencia no se puede resolver, el
  `basilisco.toml` queda intacto (no hay estados rotos).
- Edita el manifiesto conservando el resto del archivo (comentarios incluidos)
  mediante `insertar_dependencia_en_toml`, que inserta al final de la sección
  `[dependencias]` (o crea la sección al final si no existe).
- `--ruta` fuera del proyecto → `autorizado = true` con aviso visible (el
  resolver sigue limitando al directorio padre).
- Al final ejecuta la instalación completa (resolver + bloqueo).

**`bsk update [ruta]` (DT-22):**

- Ignora el bloqueo previo: limpia `.basilisco/cache` (la fuente manda sobre
  la caché), re-resuelve todo y reescribe `basilisco.lock`.
- Útil exactamente cuando `instalar` falla por "integridad comprometida"
  (hash alterado intencionalmente): `update` acepta el cambio local y
  re-sincroniza.

**`bsk publish [ruta] [--clave=<hex32>] [--salida=<archivo>]` (DT-23):**

- Recopila todos los `.bsk` del proyecto (`recoger_fuentes_bsk`): raíz y
  subdirectorios recursivos, excluyendo `.basilisco/`, `paquetes/`, `vendor/`,
  `dist/` y directorios ocultos; límites 1000 archivos / 100 MB total /
  10 MB por archivo; rutas normalizadas NFC y validadas.
- Verifica SemVer estricto (la versión `*` o `dev` no es publicable).
- Genera el artefacto BPK R2 con el nuevo constructor
  `cliente_remoto::crear_bpk`: ZIP con `BPK-MANIFEST.toml` en la raíz y los
  archivos bajo `payload/`, con sha256 por archivo en el manifiesto. El
  constructor valida rutas (traversal, `\0`, `\`, NFC, duplicados) y límites
  antes de escribir el ZIP.
- **Autocomprobación doble:** el artefacto generado pasa por `validar_bpk`
  antes de escribirse, y las firmas generadas pasan por
  `verificar_firma_metadatos` antes de devolverse. Nunca se publica un
  artefacto o firma que no verifique.
- `--clave` (semilla Ed25519 en hex, 32 bytes) firma los metadatos y escribe
  el sidecar `.json` junto al artefacto (`MetadatosPaquete` serializado, con
  `sha256` del artefacto completo, `registry_id = "local"`, `key_id = "key1"`
  — coincide con el almacén de confianza del resolver — y `nonce` =
  timestamp en nanos, anti-replay).
- Sin `--clave` no se escribe sidecar (publicación anónima explícita).

**Tests nuevos (10, en `tests/paquetes.rs`, total 39):** add agrega+y-bloquea,
add rechaza duplicada, add rechaza inexistente sin tocar manifiesto, add con
ruta externa y autorización, update acepta cambios y reescribe bloqueo,
update falla sin manifiesto, publish BPK válido sin firma (+ extracción
round-trip), publish firmado con verificación de firma y sha256, publish
rechaza no-SemVer, publish rechaza clave inválida.

**Estado:** 802 tests pasando, 0 fallos; `cargo fmt --check` y
`cargo clippy --all-targets --all-features -- -D warnings` limpios.

## DT-26: inventario y punto de venta con modelos reales + dos bugs de motor corregidos (2026-11-07)

Los ejemplos `inventario` y `punto_venta` tenían stubs (`valor_total` devolvía
`0`, `agregar_producto` devolvía el inventario sin tocarlo) y duplicaban las
funciones de `lib.bsk` dentro del `main`. Ahora son proyectos reales:

**`inventario/`** — `lib.bsk` con funciones puras sobre el modelo
`[nombre, cantidad, precio]`: alta validada (sin duplicados ni negativos),
búsqueda, posición, actualización de cantidad, venta con control de stock
(rechaza sobregiros), eliminación, valor total, producto más caro, conteo de
agotados y listado. `main.bsk` carga el catálogo base desde
`datos/inventario.json` (`leer_archivo` + `json_parsear`) y ejecuta el flujo
completo (ventas válidas e inválidas, reposición, retirada). Los precios del
JSON son **céntimos enteros**: los decimales de JSON no se mezclan con enteros
en la aritmética, así que el valor total (2610.0 → 2270.0 céntimos) se expresa
en esa unidad.

**`punto_venta/`** — `lib.bsk` con crear/agregar línea (validada)/total/
descuento por porcentaje/descuento por volumen (10% si total > 100)/ticket;
`main.bsk` ejecuta una venta de 114.0 → 102.6 con la línea inválida ignorada.

Ambos usan `usar "lib.bsk" como <alias>` (auténtica importación de módulos)
y sentencias de nivel superior. **Convención de ejemplos fijada:** las
sentencias de nivel superior corren idénticas en ambos motores (HIR sintetiza
`main` para la VM); `funcion main()` sin llamada explícita era silencioso en
Tree-Walk. Por coherencia se desenrolló `main()` en `calculadora` (que además
tenía el bug E1020: redefinía la nativa `dividir`, renombrada a
`dividir_num`) y en `gestor_tareas`.

**Bug de motor 1 — HIR/MIR no reescribía llamadas internas de módulos con alias.**
Al inlinear un módulo con prefijo `alias_`, los cuerpos conservaban los nombres
sin prefijo (`triplo` llamaba a `doble`), así que la VM fallaba con "función
no encontrada" y el CLI caía a Tree-Walk con aviso. Los e2e existentes no lo
detectaban (solo funciones hoja). Corrección en `src/hir.rs`: al prefijar, se
reescribe recursivamente toda `HirExpresion::Llamada` de los cuerpos y
declaraciones del módulo con el mapa nombre-original → prefijado
(`renombrar_llamadas_sentencia` / `renombrar_llamadas_expresion`; las llamadas
indirectas no se tocan). Ahora la VM ejecuta módulos con llamadas internas sin
fallback.

**Bug de motor 2 — el guard de inferencia de tipos de la VM rechazaba lo que
Tree-Walk acepta.** La VM comparaba por discriminante exacto; Tree-Walk
*convierte* (`convertir_para_tipo` / `convertir_a_tipo_anotado`). Divergencias
verificadas con sondas mínimas antes de corregir: `[]` (Arreglo en VM,
Lista en Tree-Walk) frente a `json_parsear` (Lista en ambos) → "se esperaba
«lista», se encontró «arreglo»"; inferido `f(1.5)` luego `f(2)` → rechazo en
VM, `2.0` en Tree-Walk; anotado `f(x: decimal); f(2)` → rechazo en VM, `2.0`
en Tree-Walk. Corrección en `src/vm.rs`: el guard ahora convierte igual que
Tree-Walk (Entero→Decimal/Real; Arreglo↔Lista compatibles como ya hace
`tipo_compat_valor` para anotaciones) y reenvía los argumentos convertidos;
el rechazo entero→texto se mantiene (test `primera_llamada_entero_segunda_texto_falla`).
Deuda documentada: la conversión elemento a elemento de secuencias *anotadas*
(TW sí la hace) queda sin replicar; no la usa ningún ejemplo.

**Estado:** los 5 ejemplos (`inventario`, `punto_venta`, `calculadora`,
`gestor_tareas`, `consola`) ejecutan con salida idéntica y exit 0 en
`--motor=vm` y `--motor=arbol`, sin avisos. README de los ejemplos corregidos
(`ejemplos/`, no `examples/`) con instrucciones de ejecución reales.

## G6: LSP y depurador (DT-27..29, 2026-11-08)

**DT-27 — Servidor de idioma (`src/lsp.rs`, comando `bsk lsp`):**
- JSON-RPC 2.0 sobre stdio con encuadre `Content-Length` (lectura/escritura
  en `leer_mensaje`/`escribir_mensaje`, testables por separado).
- `initialize` anuncia `textDocumentSync` (full), `hoverProvider` y
  `completionProvider`; `initialized`, `shutdown`, `exit` (el servidor
  permanece vivo hasta `exit`, como exige el protocolo).
- `textDocument/didOpen`/`didChange`: almacena el documento y publica
  diagnósticos (`textDocument/publishDiagnostics`) calculados con el mismo
  pipeline que `bsk analizar` (lexer → parser → semántico → HIR; los errores
  de módulos se recogen de `MODULO_ERROR`).
- `textDocument/hover`: devuelve la línea actual como bloque de código
  (best effort, sin inferencia de tipos por posición).
- `textDocument/completion`: lista de palabras clave y nativas del lenguaje.

**DT-28 — Depurador (`src/vm.rs` + comando `bsk depurar`):**
- La VM gana `traza: bool` + `traza_salida` (buffer capturable) y los
  métodos `activar_traza()`/`traza()`; `ejecutar_indice` registra entrada
  (`→ nombre(args)`) y salida (`← nombre = valor` o `! error`) de cada
  función, saltando `__init` para no ensuciar la traza. La traza es
  capturable, no solo stderr, lo que la hace testeable.
- `depurar <archivo> [--punto-funcion=<nombre>]` compila con el pipeline
  completo (AST → HIR → MIR → VM), activa la traza y ejecuta `main`;
  `--punto-funcion` filtra la salida a una función concreta. Los errores de
  ejecución imprimen la traza parcial antes del mensaje de error.

**DT-29 — Integración y pruebas:**
- Comandos `lsp` y `depurar`/`debug` en el despacho y en `ayuda()`.
- `tests/lsp.rs` (3 tests): round-trip initialize→shutdown→exit, publicación
  de diagnósticos con código inválido, completion con `mostrar`.
- `tests/depurador.rs` (3 tests): traza completa, filtro por función, reporte
  de error con traza parcial.

**Estado:** 814 tests pasando, 0 fallos; `cargo fmt --check` y
`cargo clippy --all-targets --all-features -- -D warnings` limpios.

## DT-10..13: genéricos, closures, módulos robustos, concurrencia (2026-11-08)

**DT-11 — Closures (funciones de primer orden):**
- Lambdas y clausuras con captura (incluida captura mutable) funcionan en VM
  y árbol (tests `closures.rs`, `lambdas.rs`).
- **Bug corregido:** un parámetro de función que recibía un valor función
  (p. ej. `aplicar(doble, 4)`) no se registraba en `res.variables`, así que
  la VM lo trataba como llamada directa (`función no encontrada: f`) y caía a
  Tree-Walk. Ahora los parámetros (de funciones y lambdas) se registran en
  `res.variables`, y un identificador que nombre una función se baja como
  `HirExpresion::FuncionRef` → `MirConst::Funcion` → `Valor::Funcion`, con
  paridad real. Backend nativo: `MirConst::Funcion` se materializa como `0`
  (las funciones de primer orden no se soportan en el backend nativo, consistente
  con su alcance actual).
- Tests de paridad nuevos: `diff_closure_captura_simple`, `diff_closure_dos_instancias`,
  `diff_closure_mutable_captura`, `diff_closure_como_argumento`,
  `diff_closure_retornada_anidada`, `diff_lambda_en_map`, `diff_lambda_en_filtrar`,
  `diff_lambda_en_reducir`.

**DT-12 — Módulos robustos:**
- Importación por alias y nominal, visibilidad `publico`, inicialización de
  top-level, llamadas internas entre funciones del módulo (reescritura HIR,
  DT-26) y módulos anidados (A→B→C, `modulos_e2e::e2e_transitiva`).
- Tests de paridad: los módulos con llamadas internas se ejercitan vía
  `modulos_e2e` (30 tests), incluyendo `e2e_llamadas_internas_del_modulo` y
  `e2e_llamadas_internas_en_cadena` añadidos en DT-26.

**DT-13 — Concurrencia (canales) y REPL:**
- `crear_canal`/`enviar`/`recibir` compartidos entre motores (estado global
  `CANALES`); `iniciar_tarea`/`esperar_tarea` en VM (hilo propio con globals
  clonados y `inicializado = true`) y en Tree-Walk. Ejemplo vivo:
  `ejemplos/concurrencia` (productor/consumidor) con salida idéntica en ambos
  motores.
- REPL ya existente (`bsk repl`/`repl()`): acumulación multilínea, reporte
  de errores léxicos/sintácticos/de ejecución sin abortar.
- Tests de paridad: `diff_canal_productor_consumidor`, `diff_dos_canales`,
  `diff_tarea_iniciar_y_esperar`, `diff_tarea_dos_tareas`.

**DT-10 — Genéricos definibles por usuario:**
- **Estado:** AUSENTE (deuda futura acotada, no implementado en este ciclo).
- **Motivo:** implementarlo con type erasure sería una simulación (prohibido
  por la misión); la monomorphization completa es un feature de semanas que
  toca parser, semántico, HIR, MIR y ambos motores, y arriesgaría los 814 tests.
- **Plan propuesto:** sintaxis `funcion f<T>(x: T) -> T`; monomorphization en
  HIR (generar una copia por tipo concreto usado) en lugar de type erasure;
  propagación de los parámetros de tipo a `TipoSem`/`TipoAnotacion`; replicar
  en VM y árbol; extender backends nativo/WASM. Queda como deuda documentada
  con plan, no como "parcial".

**Estado:** 823 tests pasando, 0 fallos; `cargo fmt --check` y
`cargo clippy --all-targets --all-features -- -D warnings` limpios.

## DT-03: robustecimiento del lowering HIR (2026-11-08)

**Diagnóstico:** la migración a `Result` ya estaba hecha (`lower_expresion` y
`lower_sentencia` devuelven `Result` y propagan con `?`; `lower_programa`
agrega errores en `Vec<String>`). Los únicos `.unwrap()` en `src/hir.rs` son
3 locks de mutex (invariante válida) y 1 `.expect("siempre hay ámbito raíz")`
(invariante válida: `scopes` siempre tiene el ámbito raíz). Los `panic!` del
parser están todos en `#[cfg(test)]`.

**Riesgo real corregido:** en `lower_sentencia`, rama `Asignacion`, se usaba
`res.resolver(nombre).unwrap_or_else(|| res.insertar(nombre, None))`, creando
una variable fantasma para asignaciones a variables no declaradas (el semántico
ya reportaba `VariableNoDefinida`). Esto ocultaba el error y podía producir
programas HIR que fallaban de forma confusa en runtime.

**Cambio:** si `res.resolver(nombre)` es `None`, se devuelve
`Err("asignación a variable no declarada en lowering: '...' (el análisis
semántico debería haberla rechazado)")` en lugar de crear un fantasma. El error
se agrega a `errores` (patrón existente), sin abortar el lowering abruptamente,
y se propaga a `ejecutar_fuente_vm` vía `lower_programa(...).map_err(...)?`.
El diagnóstico complementa (no duplica exactamente) el error semántico: el
semántico reporta `VariableNoDefinida` (E1002) con ubicación; el lowering
reporta "no declarada en lowering" como defensa en profundidad.

**Criterios verificados:**
1. `x = 5` sin declaración → error claro, sin variable fantasma. ✅
2. Varios errores en un programa → reportados sin panic ni pérdida. ✅
3. Sin regresiones: asignaciones válidas, funciones, módulos, closures conservan
   comportamiento (828 tests, 0 fallos). ✅
4. Paridad TW/VM verificada en los 6 ejemplos vivos (salida idéntica, exit 0,
   stderr vacío). ✅

**Tests nuevos (4, en `tests/hir.rs`):** `hir_lower_asignacion_no_declarada_falla`,
`hir_lower_programa_con_varios_errores_no_panica`,
`hir_lower_estructura_incompleta_no_panica`, `hir_lower_valido_sigue_ok`.

**Estado:** 828 tests pasando, 0 fallos; `cargo fmt --check` y
`cargo clippy --all-targets --all-features -- -D warnings` limpios.
GAP-030 marcado CERRADA en `registro-de-brechas.md`.

## Etapa A: backend nativo con colecciones y runtime embebido (2026-11-08)

**Implementado:**
- Nuevo módulo `src/runtime.rs` (runtime embebido, solo `std`): enum `Valor` uniforme
  (Entero, Decimal, Real, Logico, Texto, Arreglo, Lista, Objeto, Nulo, Funcion) con
  Display, aritmética (suma/resta/mul/div/modulo/neg/not), comparaciones
  (eq/lt), colecciones (indice, indice_set, agregar, eliminar, longitud,
  discriminante, campo). Errores vía `RuntimeError` (sin panics de Rust).
- `src/backend_nativo.rs` reescrito: genera código Rust que incluye el runtime
  (`mod bsk_runtime { ... }`) y opera sobre `Valor`. Funciones retornan
  `Result<Valor, RuntimeError>`; `main()` maneja el resultado (imprime valor o
  error + exit 1). Soporta colecciones homogéneas y heterogéneas, indexación,
  modificación por índice, longitud, agregar, eliminar, anidamiento, para cada.
- `generar_funciones_enteras` (WASM) preservado en `src/backend_entero.rs`
  (subconjunto entero/lógico, sin cambios).
- `src/semantico.rs`: los literales de arreglo heterogéneos ya no se rechazan
  (el tipo común cae a `Desconocido` sin error), alineando el semántico con
  el runtime que acepta cualquier `Valor`.
- `src/lib.rs`: registra `runtime` y `backend_entero`.

**Tests nuevos (8, en `tests/backend_nativo.rs`):** colección de enteros,
texto, heterogénea, indexación/asignación, agregar/eliminar, anidada,
fuera de rango (error limpio, sin panic), para cada anidado. Todos comparan
salida nativa vs VM.

**Verificación:**
- `consola`, `calculadora`, `gestor_tareas` compilan nativamente con salida
  idéntica a VM (exit 0, stderr vacío).
- `hetero.bsk` (arreglo heterogéneo) compila y ejecuta idéntico a VM.
- `oob.bsk` (índice fuera de rango) imprime `error: índice 10 fuera de rango
  (tamaño 3)` y exit 1, sin panic de Rust.
- `cargo fmt --check` ✅, `cargo clippy --all-targets --all-features -- -D
  warnings` ✅, `cargo test --workspace` → **836 passed, 0 failed**.
- `backend_wasm` sigue verde (5 tests).

**Limitaciones que permanecen (Etapa A):**
- Módulos (`usar`) no compilan nativamente: el análisis semántico requiere un
  cargador (`ContextoSemantico::con_cargador`) y `bsk build` usa
  `ContextoSemantico::nuevo()`. Los ejemplos `inventario` y `punto_venta` (que
  usan módulos) no compilan nativamente todavía → Etapa C.
- `Real`/`Decimal`: el runtime los soporta, pero `es_soportado` los acepta y
  el código generado los trata como `Valor` (uniforme), así que deberían
  compilar. Pendiente de test explícito → Etapa B.
- Closures, first-class funcs, globals, try/catch, invoke: rechazados
  explícitamente (compile_error! o comentario) → Etapas D/E.
- stdlib nativa (json_parsear, etc.): no generada (sin serde en std).
  → Etapa F.
- WASM: sigue siendo subconjunto entero/lógico (sin texto ni colecciones).
  → Etapa G (opcional).
- El código generado es grande (~16 KB por programa) por el runtime embebido.

**Arquitectura:** el runtime embebido (`mod bsk_runtime`) hace el backend
evolutivo: añadir un tipo nuevo = añadir una variante a `Valor` + arms en los
métodos, sin tocar el generador. Monomorphization (genéricos) puede recuperar
performance después.

## Etapa B: Real y Decimal completos en el backend nativo (2026-11-08)

**Diagnóstico previo:** el runtime de Etapa A tenía variantes `Decimal(i128)` y
`Real(f64)` pero con semántica incorrecta: multiplicación y división decimal
usaban truncamiento en lugar de half-even, sin validación de rango, y las
mezclas Decimal↔Real convertían a f64 (justo lo que prohíbe ADR-0005).

**Corregido en `src/runtime.rs` (réplica exacta de `valor.rs`/vm.rs):**
- `redondear_half_even` (banquero) y `validar_rango_decimal` (límite 10^28-1)
  copiados de `valor.rs` (sin `TryFrom`, que no está en el prelude de la
  edición con la que `rustc` compila el código generado).
- `mul`: `round_half_even(a·b)/10^9` + validación de rango.
- `div`: `division_decimal(a,b)` con ajuste half-even por residuo.
- `suma`/`resta` decimales: exactas con `checked_add/sub` + validación de rango.
- `Decimal↔Real`: todos los operadores aritméticos y comparaciones → error
  `conversión ilegal` / `tipos incompatibles: decimal y real` (antes mezclaba
  vía f64). Cero conversiones Decimal→f64 en operaciones.
- `modulo`: ENTERO%ENTERO únicamente; resto → `conversión ilegal`;
  `i64::MIN % -1` = 0; `i64::MIN / -1` y `i64::MIN * -1` → `desbordamiento
  entero`; mensajes normalizados al `Display` de `MotivoAritmetico`
  (`división entre cero`, `desbordamiento entero`, `exceso de rango decimal`).
- `neg`: `checked_neg` para entero (i64::MIN).
- `lt`: mensaje espejo de `ErrorVm::TiposIncompatibles`; comparaciones en el
  código generado ahora **propagan** el error en vez de degradar a `falso`.
- Nuevo `TipoConv` + `Valor::convertir_a`: réplica de `convertir_tipo`
  (vm.rs) — la VM convierte el valor al tipo declarado de la local en cada
  `escribir_place`; el backend nativo ahora hace lo mismo en asignaciones
  sin proyección. Error previsible en Etapa A que no era
  visible entonces: `var r: real = 7` debe almacenar Real(7.0) (y `r/2` = 3.5, no 3).

**Pruebas nuevas (22 → total backend_nativo 41):** half-even con negativos y
empates (2.675·100, 2.5, 3.5, ±0.5·3, ±1/6, ±1/3), monetario (19.99·3 + IVA
21%), precisión (0.1+0.2=0.3, 1/7, −7/3), conversiones (entero+decimal),
comparaciones, división por cero (entero), desbordamiento entero,
i64::MIN/tablas arriba, decimales/real ilegal, comparación ilegal rango
decimal (1.3e10²), asignaciones tipadas (real, decimal, lista<entero>).

**Verificación manual (árbol/VM/nativo idénticos):** todos los casos
anteriores + `x·x` con desborde — árbol imprime y luego reporta E1001 con el
mismo texto; nativo imprime la salida parcial y sale 1 con `error: …`.
Ejemplos `consola`, `calculadora`, `gestor_tareas`: salida idéntica nativo vs
VM tras el cambio de conversiones (diff vacío).

**Limitaciones que permanecen:** `a_real`/`a_decimal`/`a_entero` son nativas
de la stdlib del intérprete (parser de texto) y aún no se generan en el
backend (rechazo limpio) → Etapa F.

## GAP-051 cerrada: conversiones de asignación estrictas en la VM (2026-11-08)

**Especificación (verificada):** `tipos-y-conversiones-normativos.md` §3.1 —
conversiones implícitas solo `entero→decimal` y `entero→real` (únicamente en
aritmética mixta). §3.2 — `decimal↔real` prohibida. Tree-Walk
(`evaluador::convertir_a_tipo_anotado`) y semántico rechazan p. ej.
`var r: real = 1.5` (literal decimal → destino real) con E1004
«asignación incompatible: se esperaba real y se obtuvo decimal» (ADR-0005).

**Causa raíz (VM):** `vm::convertir_tipo` era permisiva (`_ => v`), aceptando
conversiones prohibidas (decimal→real, real→decimal, numérico→entero,
texto/lógico cruzados, etc.). El camino CLI VM (`cli::ejecutar_fuente_vm`)
además descarta errores semánticos (`let _ = ctx.analizar_programa`), por lo
que la VM era la única barrera efectiva y no la aplicaba.

**Corrección (`prototipo/src/vm.rs`, sin alterar conversiones válidas):**
- `convertir_tipo` → `Result<Valor, ErrorVm>`, réplica estricta de
  `convertir_a_tipo_anotado`: Entero→{Entero}; Decimal→{Decimal|Entero};
  Real→{Real|Entero}; Lógico/Texto/Nulo exactos; Lista/Arreglo(len)/Coleccion
  con conversión recursiva de elementos (error reportado a nivel contenedor,
  paridad TW); `Funcion`/`Desconocido` pasan sin restricción.
- Nuevos `ErrorVm::AsignacionIncompatible {esperado,obtenido}` (mensaje
  idéntico a E1004) y `ErrorVm::RetornoIncompatible {funcion,esperado,obtenido}`
  (mensaje idéntico a `RetornoIncompatibleConAnotacion`).
- Call sites actualizados: asignación global (1536) y `escribir_place`/locales
  → `AsignacionIncompatible`; `MirTerminator::Return` → `RetornoIncompatible`
  con nombre de función. `convertir_elementos` → `Result`.

**Pruebas (cond. 4):** 12 tests `diff_gap051_*` en `tests/diferencial_vm.rs`
(paridad TW↔VM): rechazo decimal→real (var y retorno), entero←decimal
(var/reasignación/retorno), texto←entero, lógico←entero, entero←texto,
elemento decimal en `lista<entero>`; y conversiones válidas entero→real,
entero→decimal, decimal→decimal.

**Divergencias adicionales revisadas (cond. 5):** (a) retornos anotados
incompatibles (p. ej. `-> real` devolviendo decimal, `-> entero` devolviendo
decimal) — la VM los aceptaba; corregidos con `RetornoIncompatible` espejo.
(b) reasignación de local tipado con valor incompatible — cubierta y corregida
vía `escribir_place`. (c) literales `[...]` reportan `obtenido` distinto en el
diagnóstico (TW evalúa a `Lista` → «obtuvo lista», VM a `Arreglo` → «obtuvo
arreglo»): divergencia de representación *preexistente* del valor, no de la
regla de conversión; el rechazo es correcto en ambos (test de lista usa
aserción de rechazo en ambos motores).

**Efecto colateral documentado (cond. 7, sin eliminar pruebas):**
`tests/backend_nativo.rs::backend_nativo_real_aritmetica` usaba
`funcion main() -> real` retornando el resultado de literales fraccionarios
(`2.5`, `3.0`) — es decir, un `decimal` — lo cual era aceptado solo por la
conversión permisiva. Se ajustó la anotación de retorno a `-> decimal`
(con comentario) para cumplir ADR-0005; el cuerpo/aritmética/valor esperado
(`7.333333333`) y la aserción de paridad VM↔nativo se mantienen intactos.

**Validación:** `cargo fmt --check` OK; `cargo clippy --all-targets
--all-features -- -D warnings` OK; `cargo test --workspace` →
**870 passed, 0 failed, 2 ignored** (+12 GAP-051). `registro-de-brechas.md`
GAP-051 → CERRADA.

## Etapa C: compilación nativa de módulos y `usar` (2026-11-08)

**Objetivo:** `bsk build` compila y ejecuta nativamente programas con `usar`;
ejemplos de referencia `inventario` y `punto_venta` con salida idéntica a VM.

**Investigación (1–6):** (1) `cargador.rs` resuelve/canonicaliza rutas
relativas al importador (o root), normaliza extensión, cachea por ruta
canónica, detecta ciclos (`en_carga`), valida nivel superior
(`Declaracion/Funcion/Usar`) y en runtime-TW ejecuta el módulo recolectando
exportaciones `publico`. (2) Resolución relativa al padre del importador con
fallback a root y contención `starts_with(root)`. (3) HIR inlining vía
`cargar_modulo_items`: lee/lex/parse/analiza/baja con resolvedor compartido
(ciclo `MODULOS_EN_CARGA`), prefija funciones aliadas `alias_nombre`,
reescribe llamadas internas, convierte decls top-level del módulo a
`HirItem::Declaracion` (init global), crea `ObjetoModulo{prefijo,nombres}` →
MIR `MirRvalue::ObjetoModulo` (VM: `Objeto` de `Valor::Funcion(prefijo_nombre)`);
`alias.func()` → `Campo` sobre `Global` del alias + `LlamadaIndirecta`.
Nominal `usar {n}` filtra/registra sin prefijo. MIR maneja globals/`__init`.
(4) Init en `__init` preservando orden de inlining. (5) `cli.rs::build`
usaba `ContextoSemantico::nuevo()` sin cargador/archivo → E1038
«importaciones no disponibles». (6) Integrado cargador en build (patrón
`depurar`): root = padre del fuente, `con_cargador` + `set_archivo_actual` +
chequeo `MODULO_ERROR`.

**Cambios (backend nativo / runtime / CLI):** `backend_nativo.rs` soporta
`MirOperand::Global`, `MirStmt::AsignacionGlobal` (almacén
`OnceLock<Mutex<Vec<Valor>>>` + accessor `bsk_g()`), `MirRvalue::ObjetoModulo`,
`MirStmt::LlamadaIndirecta` (dispatch `Valor::Funcion(nombre)` → `bsk_*` sobre
funciones soportadas); `__init` invocado con propagación de error; wrappers
stdlib `leer_archivo`/`json_parsear`; `sumas_homogeneas` relajada (tipos
dinámicos/Desconocido delegados al runtime). `runtime.rs` añade
`bsk_leer_archivo` y `bsk_json_parsear` (parser JSON recursivo std-only;
mapeo VM-paridad: int→`Entero`, float→`Real`, array→`Lista`, object→`Objeto`).
`cli.rs` build con cargador+contexto de módulo; rustc invocado con
`--edition 2024` (necesario por el runtime embebido).

**Verificación:** `punto_venta` (`bsk build` + ejecución desde su dir) salida
idéntica a `bsk run` (diff vacío). `inventario` (usa `leer_archivo` +
`json_parsear`, ejecutado desde su dir) salida idéntica a `bsk run` (diff
vacío). Pruebas nativas nuevas `tests/backend_nativo.rs` (VM↔nativo, módulos
en tmpdir con cargador): `backend_nativo_modulo_alias`, `_nominal`,
`_anidado`, `_con_estado_init` → 4/4 OK. `cargo fmt` OK, `clippy -D warnings`
OK. Suite completa workspace: ver resultado final adjunto al informe de cierre.

**Limitaciones que permanecen:** doble inline del mismo módulo en
alias/contextos distintos puede duplicar símbolos/globals en el binario
nativo (HIR inlining sin caché de "ya emitido" por identidad de módulo;
sin impacto en los casos objetivo). Soporte stdlib nativa parcial
(`leer_archivo`/`json_parsear`/`longitud`/`agregar`/`eliminar`/E/S;
resto según runtime). Sin closures/celdas, try/catch/invoke, rangos en
nativo. `punto_venta`/`inventario` dependen de CWD para rutas relativas de
datos (igual que VM). Resolución de rutas en HIR `cargar_modulo_items` usa
su propia lógica de candidatos (consistente con `ctx.archivo_actual`) además
de la validación semántica vía cargador.
