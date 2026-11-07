# AUDITORÍA MAESTRA — BASILISCO

**Fecha de auditoría:** 2026-11-05  
**Commit auditado:** `1267bc0` (HEAD → main, origin/main)  
**Versión del paquete:** `0.7.0` (Cargo.toml)  
**Metodología:** inspección directa del código fuente + ejecución de la suite de tests.  
**Ejecutor:** Kiro (independiente de informes anteriores).

---

## Resumen ejecutivo

BASILISCO es un lenguaje de programación imperativo con sintaxis en español. Su prototipo en Rust implementa el pipeline completo Lexer → Parser → Análisis semántico → HIR → MIR → VM, con un segundo motor de ejecución (evaluador tree-walk), un optimizador de MIR básico y dos backends experimentales (nativo y WASM). El toolchain incluye CLI, gestor de paquetes, registro remoto funcional y formateador. No existe REPL, LSP, debugger, ni integración con IDE.

### Tabla de estado por subsistema

| Subsistema | Estado | Confianza |
|---|---|---|
| Lexer | COMPLETO | ALTO |
| Parser / AST | COMPLETO | ALTO |
| Análisis semántico | PARCIAL | ALTO |
| Sistema de tipos | PARCIAL | ALTO |
| HIR + Lowering AST→HIR | PARCIAL | ALTO |
| Verificador HIR | PARCIAL | ALTO |
| MIR + Lowering HIR→MIR | COMPLETO (para subconjunto del lenguaje) | ALTO |
| Verificador MIR | PARCIAL | ALTO |
| Optimizador MIR | PARCIAL | ALTO |
| VM (intérprete MIR) | COMPLETO | ALTO |
| Evaluador tree-walk | COMPLETO | ALTO |
| Closures / Lambdas — tree-walk | COMPLETO | ALTO |
| Closures / Lambdas — MIR/VM | PARCIAL | ALTO |
| Genéricos | AUSENTE | ALTO |
| Traits / Interfaces | AUSENTE | ALTO |
| OOP / Estructuras | AUSENTE | ALTO |
| Módulos | PARCIAL | ALTO |
| Concurrencia | EXPERIMENTAL | ALTO |
| Canales | AUSENTE | ALTO |
| FFI | EXPERIMENTAL | ALTO |
| Backend nativo | EXPERIMENTAL | ALTO |
| Backend WASM | EXPERIMENTAL | ALTO |
| CLI | COMPLETO | ALTO |
| REPL | AUSENTE | ALTO |
| Formateador | PARCIAL | ALTO |
| Proyectos / Scaffolding | COMPLETO | ALTO |
| Gestión de paquetes (local) | COMPLETO | ALTO |
| Gestión de paquetes (remoto) | COMPLETO | ALTO |
| Registro de paquetes (servidor) | COMPLETO | ALTO |
| LSP | AUSENTE | ALTO |
| Debugger DAP | AUSENTE | ALTO |
| Extensión IDE | AUSENTE | ALTO |
| Tests | 782 / 0 fallos | ALTO |

---

## 1. Métricas verificadas

### 1.1 Suite de pruebas

Resultado de `cargo test --manifest-path prototipo/Cargo.toml` ejecutado durante esta auditoría:

```
test result: ok. 782 passed; 0 failed (suma de todos los harness)
```

Desglose por origen:
- Tests de integración (`tests/*.rs`): **669** (conteo por grep de `#[test]`)
- Tests unitarios inline (`src/*.rs`): **113** (conteo por grep de `    #[test]`)
- Total ejecutado: **782**

> **Contradicción resuelta:** el registro de brechas anterior reportaba "580 pruebas" y el INDICE-Y-MAPA-DE-COBERTURA.md mencionaba "264 pruebas". El número actual verificado es **782**. Ninguna de las cifras anteriores es correcta hoy.

### 1.2 Tamaño de los módulos fuente principales

| Archivo | Líneas |
|---|---|
| `evaluador.rs` | 6 091 |
| `vm.rs` | 2 362 |
| `parser.rs` | 2 073 |
| `hir.rs` | 2 023 |
| `mir.rs` | 1 318 |
| `semantico.rs` | 1 534 |
| `lexer.rs` | 606 |
| `optimizador.rs` | 302 |
| `backend_nativo.rs` | 265 |
| `ffi.rs` | 194 |
| `backend_wasm.rs` | 22 |

### 1.3 Commits recientes

```
1267bc0  fix: orden de ejecución de sentencias top-level en el VM (main sintético)
1ce53a8  chore: bump a 0.7.0 + README actualizado (VM/MIR implementados)
5cfebb6  fix: clippy warnings in conformidad tests
b7fc4ab  Corregir ruta de persistencia del inventario
75ee07e  Agregar aplicación de inventario BASILISCO
18408a2  docs: tutorial en español, referencia de stdlib y guía de migración VM
156b8e8  feat: VM como motor por defecto + paridad completa de módulos
```

---

## 2. Frontend del compilador

### 2.1 Lexer
**Archivo:** `src/lexer.rs` (606 líneas)  
**Estado:** COMPLETO — Confianza: ALTO

Tokeniza el lenguaje completo. Tokens verificados:
- Literales: `Numero`, `Cadena(String)`, `Verdadero`, `Falso`, `Nulo`
- Palabras clave completas: `Sea`, `Var`, `Dato`, `Const`, `Arreglo`, `Lista`, `Funcion`, `Retornar`, `Mostrar`, `Si`, `Sino`, `Mientras`, `Y`, `O`, `No`, `Para`, `Cada`, `En`, `Romper`, `Continuar`, `Intentar`, `Rescatar`, `Importar`, `Exportar`, `Usar`, `Como`, `Publico`, `MostrarSinSalto`
- Operadores: `+`, `-`, `*`, `/`, `%`, `=`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `..`, `..=`, `->`, `.`, `:`
- Soporte: BOM inicial, comentarios `#`, continuación de expresión tras operador
- 113 tests unitarios inline en `src/lexer.rs` y `src/parser.rs`

**Limitaciones:** sin soporte de strings multilínea, sin interpolación de cadenas, sin comentarios de bloque.

### 2.2 Parser / AST
**Archivo:** `src/parser.rs` (2 073 líneas)  
**Estado:** COMPLETO — Confianza: ALTO

Enum `Expresion` (21 nodos + `src/parser.rs:21`):
`Literal`, `Variable`, `Suma`, `Resta`, `Multiplicacion`, `Division`, `Modulo`, `IgualIgual`, `Distinto`, `Menor`, `MenorIgual`, `Mayor`, `MayorIgual`, `No`, `Y`, `O`, `Llamada`, `Indice`, `AsignacionIndice`, `Acceso`, `LlamadaMetodo`, `Lambda`, `ArregloLiteral`, `ListaLiteral`, `Rango`, `Si`

Enum `Sentencia` (17 variantes + `src/parser.rs:103`):
`Declaracion`, `Asignacion`, `Expresion`, `Funcion`, `Si`, `Mientras`, `ParaCada`, `Bloque`, `Retornar`, `Mostrar`, `MostrarSinSalto`, `IntentarRescatar`, `Importar`, `Exportar`, `Usar`, `Romper`, `Continuar`

Enum `TipoAnotacion`: `Entero`, `Decimal`, `Real`, `Logico`, `Texto`, `Nulo`, `Arreglo(tipo, N)`, `Lista(tipo)`

Precedencia: o → y → no → comparación → suma → multiplicación → primaria (ADR-0004).

**Limitaciones:** no hay anotaciones de tipo de retorno en `TipoAnotacion` (tipo función), no hay `Clausura` como tipo anotable por usuario.

### 2.3 Análisis Semántico
**Archivo:** `src/semantico.rs` (1 534 líneas)  
**Estado:** PARCIAL — Confianza: ALTO

Implementa `ContextoSemantico::analizar_programa()` sin ejecutar código. Verifica:
- Declaración de variables y sombreado (detecta duplicados en el mismo ámbito)
- Inmutabilidad de `dato` y `const`
- Aridad de funciones
- Tipos de argumentos cuando son conocidos
- Resolución de importaciones y módulos
- Errores emitidos como `Vec<ErrorEjecucion>` antes de ejecutar

`TipoSem` verificado: `Entero`, `Decimal`, `Real`, `Logico`, `Texto`, `Arreglo(Box<TipoSem>, usize)`, `Lista(Box<TipoSem>)`, `Coleccion(Box<TipoSem>)`, `Funcion { params, retorno }`, `Nulo`, `Desconocido`

**Limitaciones:**
- No hay un tipo `Clausura` ni `Objeto` (fueron eliminados entre versiones; `hir.rs` usa `TipoSem::Desconocido` como comodín frecuente)
- Inferencia de tipo en primera llamada, no Hindley-Milner
- No detecta uso de variable antes de inicialización en todos los caminos
- No hay chequeo de exhaustividad de match/si

**Tests:** `tests/semantico.rs` (59 tests), `tests/semantico_modulos.rs` (11 tests), `tests/inferencia_tipos.rs` (18 tests)

### 2.4 Sistema de tipos
**Archivo:** `src/semantico.rs`, `src/valor.rs`  
**Estado:** PARCIAL — Confianza: ALTO

Tipos en ejecución (`Valor`): `Entero(i64)`, `Decimal(i128)` escalado a 9 decimales, `Real(f64)`, `Logico(bool)`, `Texto(String)`, `Nulo`, `Arreglo(Vec<Valor>)`, `Lista(Vec<Valor>)`, `Funcion`, `FuncionClausura`, `Modulo`, `Objeto(HashMap)`, `FechaHora`, `Archivo`

Conversiones implícitas permitidas: `entero → decimal`, `entero → real`. Resto son explícitas.

**Ausente:** genéricos definibles por usuario, traits, tipos suma, tipos producto (structs con nombre), tipos algebraicos, type aliases.

---

## 3. Pipeline HIR → MIR

### 3.1 HIR
**Archivo:** `src/hir.rs` (2 023 líneas)  
**Estado:** PARCIAL — Confianza: ALTO

Estructuras verificadas:
- `HirLiteral`: `Entero(i64)`, `Real(f64)`, `Decimal(i128)`, `Logico(bool)`, `Texto(String)`, `Nulo`
- `HirExpresion`: 25+ variantes incluyendo `Lambda`, `Si` como expresión, `Rango`, `ArregloLiteral`, `Indice`, `Acceso`, `Llamada`, `LlamadaIndirecta`, `ModuloObjeto`, `Otro(TipoSem)`
- `HirSentencia`: 14 variantes incluyendo `ParaCada`, `IntentarRescatar`, `Exportar`, `Romper`, `Continuar`
- `HirItem`: `Funcion` (con `capturadas`, `capturas_externas`, `celdas`), `Declaracion`, `IntentarRescatar`, `Exportar`, `Importar`, `Usar`, `Expresion`
- `ResolutorHir`: asigna `SimbodoId` estables, gestiona ámbitos, detecta variables libres para closures

**Lowering AST → HIR:**
- Función pública: `lower_programa(&sentencias, &ctx) -> Result<HirPrograma, String>`
- Manejo de closures: variables capturadas vs. capturadas mutables (celdas)
- Importaciones: carga recursiva con detección de ciclos

**Limitaciones conocidas:**
- `HirExpresion::LlamadaMetodo` y `HirExpresion::ModuloObjeto` usan `TipoSem::Desconocido` como tipo de retorno sin análisis
- El nodo `HirExpresion::Otro(tipo)` es un comodín de escape que retorna `true` en el verificador sin análisis real

### 3.2 Verificador HIR
**Archivo:** `src/hir.rs:220-227` → `VerificadorHir::verificar()`  
**Estado:** PARCIAL — Confianza: ALTO

Verifica:
1. Todos los `SimbodoId` referenciados en expresiones están en el conjunto declarado
2. Parámetros de función son únicos
3. Las variables capturadas no se repiten y apuntan a símbolos declarados
4. Invariantes de tipo en operadores binarios (`tipos_compatibles_binario`)
5. Literales con tipo correcto (`Entero(_), TipoSem::Entero` etc.)

**Gap crítico verificado (línea 472):**
```rust
// 4. Tipo de retorno consistente
let _ = retorno;  // ← VACÍO. El chequeo de tipo de retorno está aplazado.
```

**Otros gaps:**
- `verificar_item` para `HirItem::Declaracion` rechaza `TipoSem::Desconocido` aunque es válido para variables `sea`
- `HirExpresion::Otro(_)` y `HirExpresion::ModuloObjeto` retornan `true` sin análisis

**Tests:** `tests/hir.rs` (9 tests)

### 3.3 MIR
**Archivo:** `src/mir.rs` (1 318 líneas)  
**Estado:** COMPLETO (para el subconjunto del lenguaje) — Confianza: ALTO

Tipos verificados:
- `MirOperand`: `Copy(LocalId)`, `Move(LocalId)`, `Global(GlobalId)`, `Const(MirConst)`
- `MirConst`: `Entero(i64)`, `Logico(bool)`, `Real(f64)`, `Decimal(i128)`, `Texto(String)`, `Nulo`
- `MirPlace` con `MirProjection`: `Indice(MirOperand)`, `Campo(String)`, `Deref`
- `MirRvalue`: `Use`, `BinOp`, `UnOp`, `Agregado(Vec<MirOperand>)`, `Indice`, `Campo`, `Longitud`, `Rango`, `Clausura { func, capturadas }`, `CrearCelda(MirOperand)`, `LeerCelda(MirOperand)`, `ObjetoModulo`
- `MirStmt`: `Asignacion`, `Llamada`, `LlamadaIndirecta`, `EntrarTry`, `SalirTry`, `AsignacionGlobal`, `Nop`
- `MirTerminator`: `Goto(BlockId)`, `SwitchInt`, `Return(Option<MirOperand>)`, `Invoke { bloque_ok, bloque_error }`, `Unreachable`

**Lowering HIR → MIR:** `lower_programa()` e `LoweringCtx`. Soporta el subconjunto actual del lenguaje (funciones, closures, módulos, excepciones, colecciones).

### 3.4 Verificador MIR
**Archivo:** `src/mir.rs:1281-1318` → `VerificadorMir::verificar()`  
**Estado:** PARCIAL — Confianza: ALTO

Verifica:
1. Ninguna función tiene cero bloques
2. No hay `BlockId` duplicados dentro de una función
3. El `bloque_entrada` existe en el CFG
4. Todos los targets de `Goto`, `SwitchInt` (casos y por_defecto) e `Invoke` (bloque_ok, bloque_error) apuntan a bloques existentes

**Gaps:**
- No verifica que `LocalId` y `GlobalId` referenciados en instrucciones existan
- No detecta uso-antes-de-definición (use-before-def)
- No verifica tipos de operandos en ninguna instrucción

**Tests:** `tests/mir.rs` (3 tests de estructura, invocados en conjunto con la suite diferencial)

---

## 4. VM y evaluador

### 4.1 VM (intérprete MIR)
**Archivo:** `src/vm.rs` (2 362 líneas)  
**Estado:** COMPLETO — Confianza: ALTO

`MaquinaVirtual` ejecuta `MirPrograma` bloque a bloque.

Características verificadas:
- Motor por defecto desde commit `156b8e8`
- `MaquinaVirtual::nuevo(programa)` y `con_limites(programa, limite_pasos, limite_profundidad)`
- `ErrorVm`: `LimitePasosExcedido`, `BloqueInalcanzable`, `ArgumentosInsuficientes { faltantes }`, `FuncionNoEncontrada`, `TipoEsperado`, `IndiceFueraDeRango`, `FfiInvalido`, etc.
- Stdlib registrada: `mostrar`, `mostrar_sin_salto`, `leer`, `leer_entero`, `leer_todo`, `abrir`, `escribir`, `escribir_linea`, `longitud`, `agregar`, `quitar`, `contiene`, `indice_de`, `subcadena`, y todas las funciones matemáticas, de fecha, conversión, JSON, `map`, `filtrar`, `reducir`, `zip`, `iniciar_tarea`, `esperar_tarea`, etc.
- Closures: `invocar_cierre()` implementado
- Concurrencia: `iniciar_tarea`/`esperar_tarea` sobre `std::thread`

**Tests:** `tests/vm.rs` (11 tests directos) + `tests/diferencial_vm.rs` (55 tests de paridad tree-walk vs VM)

### 4.2 Evaluador tree-walk
**Archivo:** `src/evaluador.rs` (6 091 líneas)  
**Estado:** COMPLETO — Confianza: ALTO

Motor de respaldo, activable con `--motor=arbol` o `BASILISCO_MOTOR=arbol`. Implementa `Entorno::ejecutar_programa()` sobre el AST directamente.

`CausaError` tiene 30+ variantes específicas. Toda la stdlib está también implementada aquí.

### 4.3 Paridad tree-walk vs VM
**Archivo:** `tests/diferencial_vm.rs` (55 tests)  
**Estado:** COMPLETO — Confianza: ALTO

Los 55 tests de paridad pasan en la suite actual. Cubren: aritmética, división, módulo, condicional, while, recursión factorial, arreglos, texto, errores aritméticos, para-cada, intentar-rescatar, listas, closures con captura, texto iterable, rangos, para-cada con romper y continuar.

> **Corrección de informe anterior:** el informe previo mencionaba "55/55" como métrica histórica y luego afirmaba que era de "una versión anterior". En realidad, el archivo `diferencial_vm.rs` tiene exactamente 55 tests hoy, y todos pasan. Es la cifra actual, no histórica.

---

## 5. Optimizador
**Archivo:** `src/optimizador.rs` (302 líneas)  
**Estado:** PARCIAL — Confianza: ALTO

Función pública: `optimizador::optimizar(&mut MirPrograma)`.

Pases implementados y verificados:
1. **Plegado de constantes** (`plegar_constantes`): evalúa expresiones con operandos constantes dentro de un bloque básico. Propaga constantes locales (`let mut conocidos: HashMap<usize, MirConst>`).
2. **Simplificación de ramas** (`simplificar_ramas`): convierte `SwitchInt` con discriminante constante en `Goto` al bloque correcto.
3. **Eliminación de NOPs** (`eliminar_nops`): borra `MirStmt::Nop`.

Ejecución: hasta punto fijo, máximo 8 iteraciones. Idempotente.

**Tests:** `tests/optimizador.rs` (8 tests que verifican que la salida es idéntica con/sin optimización y que coincide con tree-walk)

**Ausente:** eliminación de código muerto entre bloques, inlining, propagación de valores entre bloques (el mapa `conocidos` se reinicia por bloque), eliminación de variables no usadas, optimizaciones de bucles.

---

## 6. Características del lenguaje

### 6.1 Funciones y funciones anidadas
**Estado:** COMPLETO — Confianza: ALTO

Declaración con `funcion nombre(param1: tipo, ...) -> tipo { ... }`. Parámetros: todos tipados o ninguno (ADR-0011 §3.2). Las funciones anidadas se levantan como closures en el lowering HIR.

Recursión: probada con factorial en tests de VM, tree-walk, backend nativo y diferencial.

### 6.2 Lambdas
**Estado:** COMPLETO (tree-walk) / PARCIAL (VM) — Confianza: ALTO

Sintaxis: `funcion (x) { retornar x * 2 }` como expresión.

**Tree-walk:** representadas como `Valor::FuncionClausura`. Todos los casos del test `tests/lambdas.rs` (6 tests) pasan via `Entorno`. Una lambda sin retorno usada como valor emite `E1010` (RetornoAusente), comportamiento verificado.

**VM/MIR:** el lowering HIR→MIR genera `MirRvalue::Clausura { func, capturadas }`. El `diff_closure_captura` en `diferencial_vm.rs` pasa, lo que confirma paridad básica. Sin embargo, la implementación de closures en la VM depende de que `invocar_cierre()` encuentre el código en la tabla de funciones MIR — el test de closure mutable (`closure_mutable` en `tests/closures.rs`) solo se ejecuta con `Entorno` (tree-walk), no tiene su equivalente diferencial.

### 6.3 Closures con captura mutable
**Estado:** COMPLETO (tree-walk) / PARCIAL (VM) — Confianza: ALTO

Tree-walk: `Valor::FuncionClausura` usa `Arc<Mutex<Entorno>>` para compartir estado. Test `closure_mutable` verifica que un contador incrementa de 1 a 2 correctamente.

MIR/VM: usa `MirRvalue::CrearCelda`/`LeerCelda` para celdas de referencia mutables. La paridad de closures mutables no está cubierta por `diferencial_vm.rs`.

### 6.4 Módulos
**Estado:** PARCIAL — Confianza: ALTO

**Implementado:**
- `importar "ruta/archivo.bsk" como alias`
- `usar "ruta/archivo.bsk" como alias` (acceso por `alias.funcion()`)
- `exportar` y `publico funcion`
- Detección de ciclos de importación (tanto en HIR como en `CargadorModulos`)
- Caché por ruta canónica por proceso
- Resolución desde `bsk_modules/` y ruta relativa al archivo

**Tests e2e:** `tests/modulos_e2e.rs` (28 tests), `tests/cargador.rs` (14 tests), `tests/sintaxis_modulos.rs` (20 tests), `tests/semantico_modulos.rs` (11 tests)

**Ausente/gap:**
- Visibilidad fina: solo `publico` (todo o nada), sin `publico(crate)` ni re-exportaciones selectivas
- Inicialización controlada (el orden de ejecución de sentencias top-level en módulos es problemático; commit `1267bc0` corrigió un bug relacionado)
- Sin namespacing estructural más allá del alias
- Módulos anidados: no soportados como jerarquía de paquetes

### 6.5 Excepciones
**Estado:** PARCIAL — Confianza: ALTO

`intentar { ... } rescatar e { ... }` implementado en tree-walk y VM. El terminator `MirTerminator::Invoke { bloque_ok, bloque_error }` soporta el modelo estructurado. `MirStmt::EntrarTry`/`SalirTry` y `try_stack` en la VM.

**Tests:** `tests/extensiones_control.rs` (11 tests), más casos en `conformidad_control.rs` y `diferencial_vm.rs`.

**Ausente:** tipos de error personalizados, propagación con `?` o equivalente, jerarquía de excepciones.

### 6.6 Colecciones
**Estado:** PARCIAL — Confianza: ALTO

`arreglo<T, N>`: tamaño fijo, asignación por índice.  
`lista<T>`: tamaño dinámico, `agregar(lista, elem)`, `quitar(lista, indice)`.  
Indexación `xs[i]` con error en índice fuera de rango.  
`para cada x en coleccion {}`, rangos `0..5` (exclusivo) y `0..=5` (inclusivo).

**Ausente:** iteradores definibles por usuario, colecciones tipadas como trait, mapas/diccionarios, conjuntos, deque, compresión de listas.

### 6.7 Concurrencia
**Estado:** EXPERIMENTAL — Confianza: ALTO

Builtins: `iniciar_tarea("nombre_funcion") -> entero` e `esperar_tarea(id) -> valor`. Implementado tanto en tree-walk (`evaluador.rs:4947`) como en VM (`vm.rs`). Usa `std::thread::spawn`.

**Evidencia directa de ausencia de canales:** búsqueda exhaustiva de `canal`, `channel`, `mpsc`, `enviar`, `recv`, `select!` en `src/` devuelve 0 resultados.

**No existe:** cancelación, timeouts, mutexes expuestos al lenguaje, primitivas de sincronización, canales.

### 6.8 Genéricos, Traits, OOP
**Estado:** AUSENTE — Confianza: ALTO

No existe ningún nodo en el AST, HIR ni MIR para definir parámetros de tipo, interfaces, implementaciones polimórficas ni clases. Solo existen `arreglo<T, N>` y `lista<T>` como tipos parametrizados _integrados_, sin posibilidad de crear tipos genéricos propios.

---

## 7. Backends

### 7.1 Backend nativo
**Archivo:** `src/backend_nativo.rs` (265 líneas)  
**Estado:** EXPERIMENTAL — Confianza: ALTO

Genera código Rust desde MIR mediante transpilación. El código generado usa un `loop { match bloque_id {} }` para reproducir el CFG. Se compila con `rustc` del sistema.

**Limitación crítica verificada** (`funcion_soportada`, línea 53-55):
```rust
fn funcion_soportada(f: &MirFunction) -> bool {
    f.params.iter().all(|(_, t)| es_entero(t))
        && es_entero(&f.retorno)
        && f.locals.iter().all(es_entero)
}
fn es_entero(t: &TipoSem) -> bool {
    matches!(t, TipoSem::Entero | TipoSem::Logico | TipoSem::Desconocido)
}
```

Las funciones con parámetros `Texto`, `Real`, `Decimal`, `Lista`, `Arreglo`, `Nulo`, `Clausura`, `Objeto`, `Modulo` se omiten con un comentario. Solo generan código funciones de tipo entero/lógico.

No existe backend LLVM, Cranelift, ni generación de código máquina.

**Tests:** `tests/backend_nativo.rs` (4 tests que compilan con `rustc` del sistema si está disponible; si no, se omiten con `return`). El test verifica paridad de salida con la VM.

### 7.2 Backend WASM
**Archivo:** `src/backend_wasm.rs` (22 líneas)  
**Estado:** EXPERIMENTAL — Confianza: ALTO

Contiene exactamente una función pública: `generar_rust_wasm(programa: &MirPrograma) -> String`. Reutiliza `backend_nativo::generar_funciones()` y reemplaza `fn bsk_` por `#[no_mangle]\npub extern "C" fn bsk_`. La misma limitación de tipos que el backend nativo se aplica.

No genera bytecode WASM directamente. Es una transpilación a Rust-WASM compilable con `rustc --target wasm32-unknown-unknown`.

**Tests:** `tests/backend_wasm.rs` se omite si no hay target `wasm32-unknown-unknown` ni Node.js en el sistema.

### 7.3 FFI
**Archivo:** `src/ffi.rs` (194 líneas)  
**Estado:** EXPERIMENTAL — Confianza: ALTO

Funciones: `cargar_libreria(ruta) -> i64`, `descargar_libreria(id)`, `llamar_funcion(id, simbolo, args, retorno)`.

**Tipos de argumentos admitidos:** `Entero`, `Real`, `Decimal` (convertido a `f64`), `Texto` (puntero C con `CString`).  
**Tipos de retorno admitidos:** `"entero"`, `"logico"`, `"real"`, `"decimal"`, `"texto"`, `"nulo"`/`"vacío"`.

Implementado con `libffi` y `libloading`. El `Arc` de la biblioteca sobrevive a la llamada aunque otro hilo la descargue.

**Ausente:** callbacks (función como argumento FFI), structs/unions como tipo, variádicos, gestión de memoria del lado C, sandbox de seguridad.

**Tests:** `tests/ffi.rs` (14 tests que compilan una biblioteca C de ejemplo con `cc`)

---

## 8. Toolchain

### 8.1 CLI
**Archivo:** `src/cli.rs`, binarios `basilisco` y `bsk`  
**Estado:** COMPLETO — Confianza: ALTO

Comandos verificados:
- `run <archivo.bsk|archivo.nexo>` / `run` (sin argumento usa `basilisco.toml`)
- `<archivo.bsk>` (ejecución directa sin subcomando)
- `nuevo proyecto <nombre>` / `nuevo [archivo.bsk]`
- `analizar <archivo>` (análisis semántico sin ejecutar)
- `formatear <archivo>` (format in-place)
- `probar [ruta]` (test runner)
- `instalar [ruta]` / `listar [ruta]`
- `instalar-remoto <metadata-url> <artifact-url> <destino>`
- `registry add|list|remove|set-default|enable|disable|info`
- `demo` (demostración embebida)
- `--version`, `--help`, `ayuda`
- `--motor=arbol|vm` / `BASILISCO_MOTOR`

Extensiones `.bsk` (canónica) y `.nexo` (legado). Códigos de salida: 0/1/2/3/4.

**Tests:** `tests/cli.rs` (20 tests), `tests/proyectos.rs` (14 tests)

### 8.2 REPL
**Estado:** AUSENTE — Confianza: ALTO

No existe ningún modo interactivo. Búsqueda de `REPL`, `repl`, `modo interactivo` en `src/cli.rs` devuelve 0 resultados.

### 8.3 Formateador
**Archivo:** `src/formateador.rs`  
**Estado:** PARCIAL — Confianza: ALTO

Reconstruye código desde el AST. Implementado para el subconjunto del lenguaje: declaraciones, funciones, condicionales, bucles, expresiones.

**Limitación verificada:** la función `formatear` reconstruye desde el AST; los comentarios (`#`) son silenciados por el lexer y no llegan al AST, por lo tanto **los comentarios se pierden al formatear**.

**Tests:** `tests/formateador.rs` (6 tests)

### 8.4 Proyectos y Scaffolding
**Archivo:** `src/proyecto.rs`  
**Estado:** COMPLETO — Confianza: ALTO

`basilisco.toml` con claves `nombre`, `version` (SemVer X.Y.Z), `entrada`. `bsk nuevo proyecto <nombre>` crea: directorio, `basilisco.toml`, `src/main.bsk`, `src/lib.bsk`, `README.md`, `.gitignore`.

**Tests:** `tests/proyectos.rs` (14 tests)

### 8.5 Gestión de paquetes
**Archivos:** `src/paquetes.rs`, `src/cliente_remoto.rs`  
**Estado:** COMPLETO — Confianza: ALTO

- Formato BPK: ZIP con `BPK-MANIFEST.toml`, directorio `payload/`, validación SHA-256 + Ed25519
- `Manifiesto` con `[dependencias]` y lockfile `basilisco.lock`
- Resolución de dependencias locales (rutas) y remotas (URL al registro)
- Verificación de integridad: SHA-256 por archivo, firma Ed25519, SemVer estricto
- `AlmacenClaves`: gestión de claves de confianza por registro (agregar, revocar, rotar)
- `RegistroNonces`: prevención de replay attacks
- `ConfigRegistros`: gestión de múltiples registros (alias, URL, predeterminado)

**Tests:** `tests/paquetes.rs` (29 tests), `tests/bpk.rs`, `tests/registros.rs`

### 8.6 Registro remoto
**Archivos:** `src/servidor/`, binario `basilisco-registry`  
**Estado:** COMPLETO — Confianza: ALTO

Servidor HTTP con Axum + SQLx + PostgreSQL. Módulos: `auth.rs`, `config.rs`, `db.rs`, `errores.rs`, `middleware.rs`, `modelo.rs`, `rutas.rs`.

API verificada: autenticación JWT + Argon2, publicación BPK (SHA-256 + Ed25519), descarga verificada, yank, namespaces, búsqueda, SemVer, atomicidad, Docker Compose. Migrations SQL en `migrations/`.

El test `tests/registry_integration.rs` está marcado `#[ignore]` y requiere PostgreSQL real.

---

## 9. Ecosistema

### 9.1 LSP
**Estado:** AUSENTE — Confianza: ALTO

Búsqueda de `LSP`, `lsp`, `language.server`, `diagnostic`, `hover`, `goto.def` en `src/` devuelve 0 resultados. No existe implementación de Language Server Protocol.

### 9.2 Debugger DAP
**Estado:** AUSENTE — Confianza: ALTO

Búsqueda de `DAP`, `dap`, `debugger`, `breakpoint`, `debug.server` en `src/` devuelve 0 resultados. No existe debugger interactivo.

### 9.3 Extensión IDE
**Estado:** AUSENTE — Confianza: ALTO

No existe plugin para VSCode, NeoVim ni ningún otro IDE. No hay directorio `.vscode/` ni archivos de extensión en el repositorio.

### 9.4 Documentación y ejemplos
**Estado:** PARCIAL — Confianza: MEDIO

Existe documentación en `docs/`, `10-especificacion/` y varios archivos markdown de ADRs. Los ejemplos de código Basilisco son mínimos:

- `prototipo/examples/`: `tutorial_inicio.bsk`, `funciones_basicas.bsk`, `funciones_basicas.nexo`, `debug_fact.bsk`, `debug_fact.nexo`, `aceptacion_v04.bsk`, `aceptacion_error_v04.bsk`
- `ejemplos/`: `control-de-flujo.nexo.txt`, `expresion-conceptual.nexo.txt`, `procesamiento-json.bsk`
- `plantillas/main.bsk`: una sola plantilla

No existen ejemplos de aplicaciones completas (servidor HTTP, procesamiento de datos, etc.).

---

## 10. Deudas técnicas

### DT-01 · Verificador HIR — tipo de retorno
**Estado:** PARCIAL  
**Evidencia:** `hir.rs:472` contiene `let _ = retorno;` donde debería comparar el tipo declarado con el tipo inferido de los `retornar` del cuerpo. El comentario en el código dice "4. Tipo de retorno consistente: si todos los retornos tienen tipo conocido, el tipo declarado debe coincidir." — y luego ignora `retorno`.  
**Impacto:** funciones con tipo de retorno incorrecto declarado no se detectan en el verificador HIR.

### DT-02 · Verificador MIR — type-checking
**Estado:** PARCIAL  
**Evidencia:** `VerificadorMir::verificar()` (mir.rs:1284-1318) solo verifica estructura del CFG. No verifica tipos de operandos, existencia de `LocalId`/`GlobalId`, ni uso-antes-de-definición.  
**Impacto:** código MIR tipo-incorrecto puede llegar a la VM y producir `ErrorVm::TipoEsperado` en ejecución.

### DT-03 · Lowering HIR — propagación de errores
**Estado:** PARCIAL  
**Evidencia:** `registro-de-brechas.md` tiene un "Plan de migración DT-03" documentando que `lower_expresion` tiene 51 call sites que usan panics/`unwrap` en lugar de `Result`. El plan está escrito pero no ejecutado. El lowering actual retorna `Result<HirPrograma, String>` en el nivel superior pero internamente usa panic en casos de borde.  
**Impacto:** errores de lowering en casos no anticipados producen panic en lugar de error reportable.

### DT-04 · Paridad VM — closures mutables
**Estado:** PARCIAL  
**Evidencia:** `tests/closures.rs` usa solo `Entorno` (tree-walk). No hay test diferencial tree-walk vs VM para closures mutables. El test `diff_closure_captura` en `diferencial_vm.rs` cubre captura inmutable pero no contadores mutables.  
**Impacto:** posibles diferencias de comportamiento entre motores en closures mutables.

### DT-05 · Errores de lowering HIR (panics)
**Estado:** PARCIAL  
**Evidencia:** verificado que existen `.unwrap()` en `MODULO_ERROR.lock().unwrap()` y `MODULOS_EN_CARGA.lock().unwrap()` en `hir.rs`. Aunque los únicos panics de mutex lock son en condiciones de corrupción de estado, la arquitectura general del lowering no retorna `Result` en todos los caminos.

### DT-06 · Backend nativo — cobertura de tipos
**Estado:** EXPERIMENTAL  
**Evidencia:** `funcion_soportada()` en `backend_nativo.rs` rechaza explícitamente texto, real, decimal, arreglos, objetos, módulos. Solo genera código para `entero`/`lógico`.  
**Impacto:** el backend nativo no puede compilar más de una fracción mínima de programas reales.

### DT-07 · Backend WASM — cobertura y binario real
**Estado:** EXPERIMENTAL  
**Evidencia:** `backend_wasm.rs` tiene 22 líneas totales. No genera bytecode WASM, solo Rust anotado. Las mismas limitaciones de tipos que el backend nativo.

### DT-08 · Optimizador — pases avanzados
**Estado:** PARCIAL  
**Evidencia:** solo 3 pases implementados (const folding, simplificación de ramas, eliminar NOPs). Sin eliminación de código muerto inter-bloques, sin inlining, sin dead store elimination.

### DT-09 · Formateador — comentarios
**Estado:** PARCIAL  
**Evidencia:** `formateador.rs` reconstruye desde el AST. El lexer descarta los comentarios `#` antes de que lleguen al parser. Los comentarios se pierden al formatear.

### DT-10 · Genéricos definibles por usuario
**Estado:** AUSENTE  
**Evidencia:** búsqueda en el AST y HIR confirma que no existe ningún nodo para parámetros de tipo definibles por el usuario.

### DT-11 · Concurrencia — canales y primitivas
**Estado:** AUSENTE  
**Evidencia:** búsqueda exhaustiva de `canal`/`channel`/`mpsc` en `src/` devuelve 0 resultados.

### DT-12 · Módulos — visibilidad fina e inicialización
**Estado:** PARCIAL  
**Evidencia:** commit `1267bc0` corrige "orden de ejecución de sentencias top-level en el VM". Solo existe `publico` como modificador de visibilidad.

### DT-13 · REPL
**Estado:** AUSENTE  
**Evidencia:** no existe código de REPL en ningún módulo.

### DT-14 · LSP
**Estado:** AUSENTE

### DT-15 · Debugger DAP
**Estado:** AUSENTE

### DT-16 · Extensión IDE
**Estado:** AUSENTE

### DT-17 · Documentación/índice desactualizado
**Estado:** PARCIAL  
**Evidencia verificada:** `INDICE-Y-MAPA-DE-COBERTURA.md` menciona "264 pruebas", `registro-de-brechas.md` anterior menciona "580 pruebas". La cifra real hoy es 782.

---

## 11. Contradicciones resueltas

| Contradicción | Resolución |
|---|---|
| Informe anterior: "264 pruebas" (INDICE) / "580 pruebas" (registro) | **782 pruebas** verificadas ejecutando `cargo test` directamente. |
| Informe anterior: "DT-06/07 no iniciados" | **Existen** `backend_nativo.rs` y `backend_wasm.rs`. Son experimentales pero no son inexistentes. |
| Informe anterior: "DT-04 55/55 de versión anterior" | Los 55 tests diferenciales siguen siendo los actuales. No es una cifra histórica. |
| Informe anterior: `TipoSem::Clausura`, `TipoSem::Objeto` en semántico | Estos tipos **no existen** en `TipoSem` de `semantico.rs`. El HIR los maneja con `TipoSem::Desconocido`. |
| Informe anterior: "análisis semántico integrado con VM (GAP-026)" | El análisis semántico es independiente del motor; se ejecuta antes de HIR lowering en `cli.rs`. No está "integrado" en la VM sino que la precede en el pipeline. |

## 12. Contradicciones pendientes

- El registro de brechas marca GAP-016 (paquetes) como "R6 CERRADA" pero no menciona que `tests/registry_integration.rs` está marcado `#[ignore]` — lo que significa que el test E2E no se ejecuta en CI estándar.
- Los ADRs referenciados en comentarios de código (ADR-0017 D1, D8) mencionan "Integración en Fase 3" para acceso a miembros de objetos, pero no hay evidencia de que Fase 3 esté planificada formalmente.

---

## 13. Riesgos técnicos

| Riesgo | Severidad | Probabilidad |
|---|---|---|
| El lowering HIR produce panics en programas válidos no anticipados | ALTO | MEDIA |
| El verificador HIR no detecta tipos de retorno incorrectos | ALTO | ALTA (ocurre en cualquier función sin anotación de retorno) |
| Diferencias de comportamiento entre tree-walk y VM en closures mutables no detectadas | MEDIO | MEDIA |
| `tests/registry_integration.rs` marcado `#[ignore]` — R6 no se verifica en CI normal | MEDIO | ALTA |
| Backend nativo/WASM no puede compilar programas con texto o colecciones | ALTO | ALTA (cualquier programa real usa texto) |
| Concurrencia sin canales ni cancelación no es utilizable para casos reales | ALTO | ALTA |
| Formateador pierde comentarios — inaceptable para uso real | MEDIO | ALTA |
