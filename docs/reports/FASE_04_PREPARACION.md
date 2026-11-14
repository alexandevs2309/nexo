# BASILISCO — Preparación integral de la Fase 04
## Unificación semántica de motores

- **Repositorio:** `alexandevs2309/nexo`
- **Rama:** `main` (única, sin ramas creadas)
- **Fecha de preparación:** 2026-11-10
- **Alcance ejecutado:** auditoría, diagnóstico, matriz de paridad y planificación técnica. **No se implementaron correcciones de Fase 04.**
- **Estado de gobernanza:** Fase 03 **APROBADA** (autorización humana explícita). Campo `Fase autorizada (current)` en `DEVELOPMENT_STATE.md` permanece en Fase 03 y **Fase 04 permanece PENDIENTE, no autorizada para iniciar/implementar**. Esta tarea no modifica autorización ni aprobación de fases.
- **Restricciones respetadas:** sin commits, sin push, sin creación de ramas, sin cambios de versión, sin modificación/eliminación/debilitación de pruebas, sin sobrescritura/eliminación de cambios preexistentes (incl. artefactos no rastreados `ejemplos/**/{main,main.rs}` preservados).

---

## 1. Resumen ejecutivo

Se auditó el estado real de los motores Tree-Walk (TW), VM/MIR y backend nativo, junto con HIR/MIR/runtime, rutas de ejecución CLI y pruebas diferenciales/paridad existentes. La línea base actual es **verde y reproducida** (`cargo fmt --all -- --check` exit 0, `cargo clippy --all-targets --all-features -- -D warnings` exit 0, `cargo test --workspace --no-fail-fast` → **965 passed, 0 failed, 2 ignored**, 55 suites, exit 0). Los 2 ignorados son externos (**B-REG-001**, `tests/registry_integration.rs`) y no cuentan como aprobados.

**Hallazgos principales (evidencia inicial, no exhaustivos sin implementación/expansión de pruebas):**
- **TW y VM** comparten pipeline léxico/sintáctico/semántico/HIR/MIR en rutas de análisis/ejecución comparables y cuentan con **cobertura diferencial amplia** (`tests/diferencial_vm.rs`, 81 tests) que compara resultado/errores normalizados para múltiples capacidades (aritmética, control, funciones/recursión, colecciones, texto/stdlib, JSON/archivos/FFI/red/http, closures, try-catch, canales/tareas según soporte), con **normalizaciones de presentación** en el harness. Aun así, la paridad semántica total no está demostrada: existen diferencias de representación/modelo de errores y riesgos de divergencia por implementación dual (TW interpreta AST/entorno; VM interpreta MIR).
- **Backend nativo** es un **subconjunto** (MIR → Rust + runtime embebido `src/runtime.rs`) con **45 tests** (`tests/backend_nativo.rs`, comparación nativo↔VM por stdout/valor textual, depende de `rustc` y omite pruebas si no está disponible). Rechaza explícitamente capacidades no generables (p.ej. closures/celdas, try-catch/invoke, rangos, partes de stdlib no cubiertas); soporte de módulos/concurrencia/FFI no equivalente al de TW/VM. Su paridad debe evaluarse solo dentro del subconjunto soportado declarado/implementado.
- **HIR/MIR** incorporan gates estructurales/well-formedness/CFG-alcanzabilidad (Fase 03) integrados en `build/run/analizar/depurar/LSP` (`src/cli.rs`, `src/lsp.rs`, verificadores en `src/hir.rs`, `src/mir.rs`), sin fallback de verificación a Tree-Walk. Persisten limitaciones conservadoras y ausencia de spans IR en diagnósticos LSP (0,0).
- **Contrato semántico común no formalizado** entre motores (criterios de equivalencia de valor/salida/efectos/categoría de error y tratamiento de no-soportado/arquitectura). Fase 04 debe acotar paridad a **capacidades existentes** sin absorber alcance de Fase 05 (backend nativo y funciones de primera clase como trabajo específico).

**Conclusión de preparación:** existe base suficiente para planificar lotes de unificación semántica incremental, pero se requiere (1) contrato de equivalencia explícito, (2) ampliación/normalización rigurosa de pruebas diferenciales por categoría con casos mínimos reproducibles, y (3) clasificación causa-raíz antes de corregir. Este informe entrega inventario, matriz, divergencias/limitaciones identificadas, propuesta de contrato, estrategia diferencial y plan priorizado.

---

## 2. Estado de la línea base

Ejecución reproducida en `prototipo/` (2026-11-10). Logs en `/tmp/opencode/` (no versionados).

| Comando | Código de salida | Resultado observado | Log |
|---|---:|---|---|
| `cargo fmt --all -- --check` | 0 | Sin diferencias de formato | `/tmp/opencode/fase04_fmt.txt` (`---EXIT:0---`) |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | `Finished dev profile`, sin warnings promovidos a error | `/tmp/opencode/fase04_clippy.txt` (`---EXIT:0---`, 4.25s) |
| `cargo test --workspace --no-fail-fast` | 0 | **965 passed, 0 failed, 2 ignored**, **55 suites** | `/tmp/opencode/fase04_ws.txt` (`---CARGO_EXIT:0---`) |

**Evidencia focalizada (mismo run, extraída de `fase04_ws.txt`):**

| Suite | Resultado |
|---|---|
| `tests/diferencial_vm.rs` | `ok. 81 passed; 0 failed; 0 ignored` |
| `tests/backend_nativo.rs` | `ok. 45 passed; 0 failed; 0 ignored` |
| `tests/conformidad*.rs` (múltiples suites) | `ok` (0 failed, 0 ignored en suites observadas) |
| `tests/pipeline_3d5_integration.rs` | `ok. 6 passed; 0 failed; 0 ignored` |
| `tests/gate_3c_integration.rs` | `ok. 27 passed; 0 failed; 0 ignored` |
| `tests/registry_integration.rs` (**B-REG-001**) | `ok. 0 passed; 0 failed; 2 ignored` |

- **Ignorados (2, externos, no aprobados):** `registry_integration::registry_audit_event_persists_in_isolated_postgres`, `registry_integration::registry_dependencies_graph_is_persisted` (requieren PostgreSQL aislado + `DATABASE_URL`/infra; B-REG-001).
- La línea base previa reportada (965/0/2) **se confirmó vigente** en esta ejecución; no se asumió sin reejecutar.

---

## 3. Arquitectura de los motores y rutas de ejecución

### 3.1 Punto de entrada y selección de motor (CLI)
- **Entradas:** `prototipo/src/main.rs` → `basilisco::cli::main()` (`prototipo/src/cli.rs`).
- **Preferencia de motor:** `MOTOR_PREFERIDO` (`LazyLock<Mutex<String>>`) inicializado desde `BASILISCO_MOTOR` o flags `--motor=`/`--engine=` (default `"vm"`). Ayuda CLI documenta `--motor=arbol|vm`.
- **Ejecución de fuente (`ejecutar_fuente`, ~líneas 1137+):** si motor preferido es `vm`, intenta `ejecutar_fuente_vm(...)`.
  - Si falla con error de verificación (`PREFIJO_VERIFICACION` / gates), **no hace fallback** y retorna error.
  - Si falla por otras causas en pipeline/VM, emite aviso `aviso: motor VM falló (...); usando motor árbol (Tree-Walk).` y **cae a Tree-Walk** (comportamiento relevante para UX, pero **no válido como prueba de paridad**).
- **Tree-Walk (fallback/ruta directa):** lex (`Lexer::con_archivo`), parse (`Parser::...::parsear_programa`), construye `Entorno` (con/sin `CargadorModulos` y `archivo`), ejecuta `entorno.ejecutar_programa(&sentencias)`.
- **Ruta VM (`ejecutar_fuente_vm`):** lex/parse → `ContextoSemantico` (con cargador/archivo si aplica) → `ctx.analizar_programa` (errores semánticos bloquean) → `hir::lower_programa(&sentencias, &ctx)` (con manejo de error de módulo vía `hir::MODULO_ERROR`) → `gate_hir` → `mir::lower_programa(&hir)` → `gate_mir` → ejecución en `vm::MaquinaVirtual` (registro stdlib, búsqueda/ejecución de `main`/índice según contexto).
- **Gates estructurales compartidos:** `gate_hir` (`hir::VerificadorHir::verificar`) y `gate_mir` (`mir::VerificadorMir::verificar`) aplicados en rutas de build/run/analizar/depurar; LSP integra diagnósticos HIR/MIR (`src/lsp.rs`, 3D.5) sin spans IR (ubicación 0,0). Sin bypass ni fallback verificación→Tree-Walk.

### 3.2 Tree-Walk — `prototipo/src/evaluador.rs`
- **Modelo:** intérprete directo sobre AST/sentencias con `Entorno` (ámbitos, mutabilidad, bindings, contexto de archivo/cargador). Ejecuta vía `Entorno::ejecutar_programa` / evaluación de sentencias y expresiones.
- **Valores/semántica numérica:** `Valor` (`prototipo/src/valor.rs`, ADR-0005/0007 referenciados en cabecera) — `entero` (i64), `decimal` (DECIMAL 28,9, `FACTOR_ESCALA`), `real` (f64), `logico`, `texto`, `nulo`, etc.; errores tipados `CausaError`/`ErrorEjecucion` (aritmética `MotivoAritmetico`, tipos, condición no lógica, etc.).
- **Contextos de ejecución:** registros/contexto gestionado para canales y tareas (aislamiento por ejecución y herencia compartida en tareas hijas/nietas del mismo run), funciones auxiliares como `instalar_registros`, acceso a registros actuales; soporte de concurrencia/primitivas en evaluador (ver §4.11).
- **Módulos:** integración con `CargadorModulos`/símbolos exportados (`cargador.rs`) según ruta.

### 3.3 VM — `prototipo/src/vm.rs`
- **Modelo:** intérprete de MIR (`mir::MirPrograma`) bloques/terminadores con marcos (llamadas), `MaquinaVirtual::nuevo(mir)`, `registrar_stdlib()`, `indice_de("main")`, `ejecutar_indice(...)`.
- **Errores:** `ErrorVm` (ejecución/aritmética reutilizando motivos donde aplica; mensajes formateados distintos a `ErrorEjecucion` TW). Conversión MIR↔`Valor` directa; manejo de operaciones/stdlib nativos y estado de VM.
- **Capacidades MIR soportadas en ejecución (evidencia por símbolos/uso):** constantes/operaciones/unarios, llamadas directas/indirectas, `CrearCelda`/`Clausura`/`Valor::ClausuraMir` (closures/capturas), `EntrarTry`/`Invoke` (manejo de excepciones en flujo MIR), módulos-objeto según lowering, canales/tareas/primitivas registradas donde aplique, `__init`/globals según programa MIR.
- **Límites/seguridad operativa:** profundidad de marcos y contador de pasos (`limite_pasos`) para bucles/cómputo excesivo (según diseño del módulo).

### 3.4 HIR/MIR — `prototipo/src/hir.rs`, `prototipo/src/mir.rs`
- **HIR:** lowering AST+contexto semántico a representación tipada/resuelta (`lower_programa`), incluye expresiones/sentencias como funciones, lambdas/capturas, llamadas (directas/indirectas), objetos-módulo, control de flujo, try-catch, etc. (`HirExpresion`, `HirSentencia`). Verificador `VerificadorHir` (gates 3D.2+). Manejo de errores de módulo (`MODULO_ERROR`).
- **MIR:** lowering HIR→MIR con bloques/terminadores (`MirStmt`, `MirTerminator`, `MirRvalue`: `CrearCelda`, `Clausura`, `LlamadaIndirecta`, `EntrarTry`, `Invoke`, etc.), `__init`/globals/funciones. Verificador `VerificadorMir` (3D.4): CFG, alcanzabilidad, coherencia de terminadores/destinos/referencias, validaciones de try/rescue/merge y `MIR045` (retorno `None` alcanzable en funciones con retorno tipado concreto). Naturaleza **conservadora** ante información incompleta (`Desconocido`, callees no resueltos) y sin garantía de cobertura total de todos los retornos.

### 3.5 Backend nativo — `prototipo/src/backend_nativo.rs`, `prototipo/src/runtime.rs`
- **Pipeline:** MIR → generación de código Rust (`generar_rust` / helpers) que embebe runtime (`runtime.rs`, `include_str!`) con `bsk_runtime::Valor`, `RuntimeError`, operaciones, colecciones, E/S y stdlib cubierta. Requiere `rustc` para compilar/ejecutar binarios generados (tests omiten con aviso si `rustc` no disponible).
- **Soporte declarado/implementado (subconjunto):** tipos básicos, texto, arreglos/listas/objetos/colecciones heterogéneas, indexación/asignación, agregar/eliminar, longitud, control de flujo, llamadas directas/indirectas (según generación), globals/`__init`, `ObjetoModulo` (alias inlinados por HIR), stdlib nativa parcial (`leer_archivo`, `json_parsear` en runtime std-only) y E/S (`mostrar`).
- **Rechazos/no soportado explícito (evidencia en tests/código):** closures/celdas (`CrearCelda`, `CerrarCelda`, `Clausura`), try-catch/invoke (`EntrarTry`/`Invoke`), rangos, y stdlib/capacidades no cubiertas por runtime embebido. Concurrencia (canales/tareas), FFI, módulos dinámicos/externos completos y otras primitivas de VM/TW **no deben asumirse soportadas** sin evidencia. Paridad evaluable solo dentro del subconjunto.

### 3.6 Resultados, salida y errores (producción/comparación observada)
- **TW:** `Ok(Option<Valor>)` / `Err(ErrorEjecucion{ causa: CausaError, ... })`; formato textual de error vía `to_string`/mensaje del error de ejecución.
- **VM:** `Result<Option<Valor>, ErrorVm>`; errores formateados como cadenas (`e.to_string()`), con prefijos/motivos que pueden diferir en presentación (ej. aritmético). Harness diferencial TW↔VM (`tests/diferencial_vm.rs`) compara con `tree_walk`/`vm_run` devolviendo `OK:{:?}` o `ERR:*` y **normaliza** mensajes para igualdad (`diff_*` usa `assert_parity` con normalizaciones específicas: desbordamiento, división por cero, razones aritméticas), además de validaciones dirigidas (tipo de valor, equivalencias).
- **Nativo:** comparación indirecta contra VM por ejecución de binario (`stdout`/resultado textual) en `tests/backend_nativo.rs`; dependiente de `rustc` y de representación emitida por runtime generado.
- **CLI fallback:** VM→TW ante fallo no-verificación puede enmascarar divergencias en uso interactivo; las pruebas de paridad deben invocar motores explícitamente y **sin fallback silencioso**.

---

## 4. Matriz de paridad semántica (por categoría)

Leyenda de estado: **Equivalente** = pruebas/evidencia directa de igualdad observada en motores comparados; **Divergente** = diferencia observada; **Parcial** = soporte/equivalencia solo en subconjunto o con restricciones; **No soportado** = rechazado/no implementado en motor; **Pendiente de verificar** = sin evidencia suficiente en esta auditoría para afirmar equivalencia.

> Nota: “Soporte declarado” se infiere de objetivos/roadmap/código visible; “Soporte implementado/resultado real” se basa en código y pruebas existentes ejecutadas o inspeccionadas. **No se marca equivalente sin prueba/evidencia.**

| Categoría | Casos investigados | TW (impl./pruebas) | VM (impl./pruebas) | Nativo (impl./pruebas) | Diferencias observadas / evidencia | Estado |
|---|---|---|---|---|---|---|
| Valores | entero, decimal, real, lógico, texto, nulo | Implementado (`Valor`, evaluador). Cobertura en conformidad y diferencial TW↔VM. | Implementado (MIR↔`Valor`). Cobertura diferencial TW↔VM. | Subconjunto implementado en runtime embebido (`Valor` generado). Tests nativo↔VM para tipos básicos/numéricos/texto/colecciones soportadas. | Representación interna distinta en nativo (runtime propio) pero comparada por resultado observable en tests; fecha/archivo y otros tipos ricos: evidencia parcial/heterogénea. | **Parcial** (núcleo básico TW↔VM equivalente en casos probados; nativo parcial) |
| Operaciones | +, -, *, /, % | Implementado con semántica ADR-0005 (errores por desbordamiento/div0/reglas). Probado (`diferencial_vm`, conformidad). | Implementado reutilizando semántica de `Valor`/operaciones en VM. Probado. | Implementado para subconjunto numérico soportado. Probado (aritmética entero/real/decimal, casos borde como `i64::MIN`). | Mensajes/formato de error aritmético difieren entre TW y VM; diferencial normaliza (`diff_error_division_por_cero`, `diff_error_desbordamiento`, `assert_parity`). | **Parcial** (resultado numérico equivalente en casos probados; diagnóstico no idéntico textualmente) |
| Comparaciones | ==, !=, <, <=, >, >= | Implementado (incl. reglas de tipos/conversiones aprobadas). Probado. | Implementado. Probado. | Parcial en casos cubiertos por tests nativos. | Compatibilidad/coerción y combinaciones de tipos requieren contrato explícito; no todas combinaciones auditadas exhaustivamente. | **Parcial / Pendiente de verificar** (según tipo) |
| Conversión | compatibilidad y coerción entre tipos | Reglas definidas en semántica/valor (implícitas aprobadas, otras error). Cobertura en conformidad/conversiones y diferencial (`a_entero`, `a_texto`, etc.). | Soporte vía MIR/VM y stdlib/ops. Probado en diferencial/conformidad VM helper. | Subconjunto según runtime/stdlib generada. | Diferencias de representación diagnóstica en errores de conversión posibles; cobertura nativa limitada. | **Parcial** |
| Booleanos | verdad/falsedad, cortocircuito | Implementado (`logico`, condiciones). Probado. | Implementado. Probado. | Implementado en subconjunto. | Cortocircuito depende de lowering/evaluación; no se identificaron fallos en tests existentes, pero no exhaustivo. | **Equivalente** en casos probados TW↔VM; **Parcial** nativo |
| Variables | declaración, asignación, mutabilidad | Implementado (entornos/ámbitos). Probado. | Implementado (locals/places/globals según MIR). Probado. | Implementado en subconjunto (asignaciones tipadas en tests). | Modelo de almacenamiento difiere (AST/entorno vs MIR/locals/celdas). | **Parcial** (comportamiento observable probado en subconjuntos) |
| Ámbitos | locales, globales, shadowing | Implementado. Cobertura en suites de conformidad/semántica. | Implementado según MIR/globals (`__init`) y frames. | Soporte parcial (globals/init y funciones) en subconjunto. | Shadowing/casos límite no exhaustivamente comparados entre los tres. | **Pendiente de verificar / Parcial** |
| Funciones | parámetros, retorno, recursión | Implementado. Probado (`diff_recursion_factorial`, `diff_fibonacci_recursivo`, etc.). | Implementado (llamadas directas, frames). Probado. | Implementado (directas, recursión) en subconjunto. Probado. | Retornos tipados/`None` alcanzable sujeto a gates MIR (3D.4) y diferencias diagnósticas. | **Equivalente** en casos probados TW↔VM/nativo (subconjunto) |
| Funciones de primera clase | referencias, llamadas indirectas | Soportado en semántica TW (según capacidades del lenguaje). | Soportado en MIR/VM (`FuncionRef`, `LlamadaIndirecta`, `ClausuraMir`/valores función). | **No soportado/restringido** en nativo para casos generales (closures/celdas rechazadas; soporte de indirectas limitado al subconjunto generable). | Alcance Fase 05 incluye funciones de primera clase/backend nativo; no absorber en Fase 04. | **Parcial** TW↔VM; **No soportado/Parcial** nativo |
| Lambdas | capturas, cierres, mutables | Soportado en TW (closures). | Soportado en VM (celdas/capturas/clausuras MIR). Probado diferencial (`diff_closure_inmutable`, `diff_closure_mutable`). | **Rechazado/no generable** (`backend_nativo_rechaza_operaciones_no_generables`: closures/celdas/try/invoke). | Brecha estructural de capacidad en nativo. | **Equivalente** en casos probados TW↔VM; **No soportado** nativo |
| Colecciones | arreglos, listas, índices, modificaciones | Implementado. Probado (diferencial y conformidad arreglos/listas). | Implementado. Probado. | Implementado en subconjunto (heterogéneas, anidadas, indexación/asignación, agregar/eliminar, fuera de rango). | Casos límite/semántica exacta de mutación y errores de índice deben consolidarse por contrato. | **Parcial** (amplia cobertura TW↔VM/nativo en subconjunto) |
| Control de flujo | si, mientras, para, romper, continuar | Implementado. Probado (`diff_condicional`, `diff_mientras`, `diff_para_cada`, `diff_romper_continuar`). | Implementado vía MIR. Probado. | Implementado en subconjunto. Probado. | Equivalencia observable en casos probados. | **Equivalente** en casos probados TW↔VM; **Parcial** nativo |
| Errores | división por cero, tipos incompatibles, errores de ejecución | `ErrorEjecucion`/`CausaError` tipado. | `ErrorVm` con mensajes distintos. | `RuntimeError` en runtime embebido. | **Diferencia de representación**: tipo/mensaje no idénticos; diferencial TW↔VM normaliza categorías/motivos en comparaciones. | **Divergente (diagnóstico/representación)**, **Parcial** (categoría comparable en casos probados) |
| Excepciones | intentar, rescatar, propagación | Implementado en TW. Probado diferencial (`diff_intentar_rescatar*`). | Implementado en MIR/VM (`EntrarTry`/`Invoke`, rescue/merge). Probado diferencial. | **No soportado** (try-catch/invoke rechazados). | Paridad TW↔VM probada en casos; nativo fuera de alcance actual. | **Equivalente** en casos probados TW↔VM; **No soportado** nativo |
| Módulos | importaciones, alias, símbolos exportados | Soporte vía evaluador/cargador/semántica. | Soporte vía HIR/MIR (`ModuloObjeto`, objetos-módulo) y VM según lowering. | Soporte **parcial**: objetos-módulo/alias inlinados por HIR (`backend_nativo_modulo_*` tests). | Módulos dinámicos/carga externa y resolución completa no equivalente entre motores; diferenciales dedicados TW↔VM para módulos no identificados explícitamente en esta pasada. | **Parcial / Pendiente de verificar** |
| Concurrencia | primitivas soportadas (canales/tareas) | Implementado con contextos/registro gestionado y herencia (tests internos en evaluador; uso en diferencial). | Soportado en VM (primitivas/stdlib/estado) y probado diferencial (`diff_tarea_*`, `diff_canal_*`, `diff_dos_canales`). | **No soportado** en runtime nativo embebido actual. | Comportamiento observable probado TW↔VM en casos incluidos; determinismo/ordenamiento y casos límite requieren cautela. | **Equivalente** en casos probados TW↔VM; **No soportado** nativo |

**Resumen de cobertura diferencial existente (evidencia):**
- `prototipo/tests/diferencial_vm.rs` (81 tests): ejecuta mismo programa vía `tree_walk` (AST→Entorno, concatena `\nmain()` en helper) y `vm_run` (AST→Semántico→HIR→MIR→VM con `registrar_stdlib`), compara `OK:{:?}`/`ERR:*` con normalizaciones de mensajes aritméticos/errores y aserciones de valor; cubre aritmética, división/módulo, control, recursión/fibonacci, arreglos/listas, texto/longitud/stdlib, try-catch, closures, archivos/JSON, conversiones, canales/tareas, y otros casos listados en el archivo.
- `prototipo/tests/backend_nativo.rs` (45 tests): genera Rust desde MIR, compila con `rustc --edition 2024 -O` y compara salida/valor con ejecución VM (helper `vm_run`/equivalente); omite con aviso si `rustc` no disponible (`compilar_y_ejecutar` retorna `None`).
- `prototipo/tests/conformidad_vm_helper.rs` y suites `conformidad*`: proveen ejecución por VM y, en helper, comparación de paridad de valor (`assert_valor_parity` TW vs VM) para programas/expresiones; muchas suites de conformidad ejercitan semántica (predominantemente TW en helpers específicos) — cobertura cruzada útil pero no uniforme como diferencial sistemático por motor.
- `tests/pipeline_3d5_integration.rs`, `tests/gate_3c_integration.rs`: validan integración de gates/pipeline (no paridad de ejecución entre motores).

---

## 5. Divergencias comprobadas y hallazgos (auditoría)

> No se aplicaron correcciones. Se listan divergencias/hallazgos observables o inferidos con evidencia directa; donde falta reproducción mínima dedicada, se marca **pendiente de caso mínimo**.

| ID | Hallazgo | Evidencia | Clasificación | Motores | Estado |
|---|---|---|---|---|---|
| D-01 | **Representación/formato de errores distinto** entre TW (`ErrorEjecucion`/`CausaError`) y VM (`ErrorVm` string), y nativo (`RuntimeError`/mensajes runtime). | Tipos y formateo en `src/evaluador.rs` (`CausaError`, `ErrorEjecucion`), `src/vm.rs` (`ErrorVm`); harness `tests/diferencial_vm.rs` normaliza prefijos/motivos (`error aritmético:`, desbordamiento, división por cero) para comparar. | **G. Diagnósticos** (y potencial **A** si categorías difieren) | TW↔VM↔Nativo | Comprobado (presentación); equivalencia de categoría **parcial/pendiente** de contrato |
| D-02 | **Subconjunto de capacidades del backend nativo**: rechaza closures/celdas, try-catch/invoke, rangos y stdlib no cubierta. | `src/backend_nativo.rs` (doc/comentarios y generación), `src/runtime.rs`; `tests/backend_nativo.rs::backend_nativo_rechaza_operaciones_no_generables` y ausencia de soporte de concurrencia/FFI. | **E. Backend nativo** (limitación de capacidad/generación) | Nativo vs TW/VM | Comprobado (no soportado) |
| D-03 | **Concurrencia y excepciones no disponibles en nativo** frente a TW/VM (casos probados). | `tests/diferencial_vm.rs` (`diff_tarea_*`, `diff_canal_*`, `diff_dos_canales`, `diff_intentar_rescatar*`) pasan TW↔VM; nativo no implementa estas primitivas/IR ops (`EntrarTry`/`Invoke`/canales/tareas) en runtime/generación. | **E. Backend nativo / F. Runtime** (capacidad) | Nativo vs TW/VM | Comprobado (brecha de alcance) |
| D-04 | **Fallback CLI VM→Tree-Walk** ante fallos no-verificación puede ocultar divergencias en ejecución interactiva. | `src/cli.rs`: `ejecutar_fuente` intenta VM y ante error no `PREFIJO_VERIFICACION` emite aviso y ejecuta Tree-Walk. | **G. Diagnósticos / Operacional** (riesgo metodológico) | CLI (TW/VM) | Comprobado (comportamiento); no es paridad |
| D-05 | **Pipelines de comparación en tests no idénticos al CLI completo** (helpers construyen contextos/mains sintéticos, pueden omitir pasos). | `tests/diferencial_vm.rs` (`tree_walk` añade `\nmain()`, `vm_run` hace semántica+lowering+stdlib+`main`), `tests/conformidad_vm_helper.rs` envuelve expresiones en `main`; `tests/backend_nativo.rs` compara contra VM helper. | **B. Lowering / Metodología de prueba** (riesgo de cobertura) | Tests | Comprobado (diseño de harness) |
| D-06 | **Gates/verificación conservadores y diagnósticos IR sin spans** (0,0) pueden aceptar/rechazar de forma conservadora sin localizar fuente fina; `Desconocido`/callees no resueltos limitan precisión. | `src/hir.rs`, `src/mir.rs` (verificadores), `DEVELOPMENT_STATE.md` (limitaciones 3D.4/3D.5: MIR045 solo alcanzable, conservadurismo, spans 0,0). | **B. Lowering / G. Diagnósticos** | HIR/MIR/LSP/CLI | Comprobado (limitación documentada/código) |
| D-07 | **Módulos: soporte no uniforme** (carga/cargador en TW/CLI vs `ModuloObjeto`/inlining en HIR y cobertura parcial nativa). | `src/cargador.rs`, `src/evaluador.rs`, `src/hir.rs` (`ModuloObjeto`), `src/backend_nativo.rs` (alias módulos inlinados) y tests `backend_nativo_modulo_*`; diferenciales TW↔VM específicos de importación no claramente identificados en esta auditoría. | **B. Lowering / D. Tree-Walk / C. VM / E. Nativo** (según caso) | TW/VM/Nativo | **Parcial/Pendiente** de casos mínimos |
| D-08 | **Dependencia de `rustc` para validar nativo**: ausencia de toolchain omite pruebas de backend nativo en ese entorno. | `tests/backend_nativo.rs::compilar_y_ejecutar` retorna `None` con aviso si `rustc` no disponible. | **Operacional** (no semántica) | Nativo (tests) | Comprobado (condición de ejecución) |

**No clasificado como divergencia real sin evidencia adicional:** diferencias de rendimiento, uso de memoria, orden interno de ejecución no observable, y capacidades fuera del subconjunto declarado de un motor.

---

## 6. Limitaciones y características no soportadas (inventario actual)

| Área | Tree-Walk | VM | Backend nativo | Nota |
|---|---|---|---|---|
| Closures/celdas/capturas mutables | Soportado (probado diferencial) | Soportado vía MIR (`CrearCelda`/`Clausura`/`ClausuraMir`) | **No soportado** (rechazado) | Fase 05 aborda funciones de primera clase/backend nativo |
| Try-catch / Invoke | Soportado (probado diferencial) | Soportado (`EntrarTry`/`Invoke`) | **No soportado** | Brecha de capacidad |
| Canales/tareas (concurrencia soportada) | Soportado (contextos gestionados/herencia; probado diferencial) | Soportado (probado diferencial) | **No soportado** | Verificar determinismo/casos límite en Fase 04 solo si en alcance existente |
| Módulos/imports/alias completos | Soportado (cargador/entorno) | Soportado según lowering/HIR-MIR | **Parcial** (objetos-módulo/alias inlinados) | Detalle de paridad pendiente |
| FFI / red / http / archivos / JSON / stdlib amplia | Amplio (según stdlib/entorno; diferencial cubre casos) | Amplio en casos cubiertos por diferencial/VM | **Parcial** (p.ej. `leer_archivo`, `json_parsear` y E/S/runtime cubierto) | Soporte real debe verificarse por caso |
| Rangos y otras ops no generables | Según lenguaje/TW | Según MIR/VM | **Rechazado** | Ver `backend_nativo_rechaza_operaciones_no_generables` |
| Diagnósticos con ubicación IR precisa | AST con ubicaciones según parser | Errores runtime/MIR; gates sin spans finos en LSP (0,0) | Errores compilación/runtime de binario generado | Mejora de spans fuera de objetivo inmediato salvo contrato de errores |
| Validación externa B-REG-001 | N/A | N/A | N/A | 2 tests ignorados (PostgreSQL/`DATABASE_URL`) |

---

## 7. Contrato semántico común propuesto

> Propuesta técnica para revisión/autorización; **no implementada**.

### 7.1 Definición de equivalencia semántica (para motores que declaran soportar la característica)
Dos ejecuciones del **mismo programa fuente canónico** (misma versión de lenguaje, misma configuración relevante, mismas entradas/entorno controlado) son semánticamente equivalentes si:
1. **Resultado final:** mismo valor observable (`Valor` equivalente por igualdad semántica definida) o misma ausencia de valor, según contrato de la construcción.
2. **Salida estándar:** mismo contenido observable normalizable solo por diferencias irrelevantes de presentación explícitamente permitidas (p.ej. fin de línea de plataforma) — **no** diferencias de contenido/orden significativo.
3. **Efectos secundarios observables:** mismo estado observable acordado (archivos/salidas/canales) cuando la prueba los controla y la capacidad está soportada.
4. **Errores:** misma **categoría/tipo semántico de error** y mismo punto lógico de fallo cuando sea observable/determinista. La **representación textual exacta** puede diferir solo si se define un mapeo canónico de categorías (p.ej. `DivisionPorCero`, `Desbordamiento`, `TiposIncompatibles`, `CondicionNoLogica`, `IndiceFueraDeRango`, etc.).
5. **Propagación:** misma decisión de propagación/captura (ej. try-catch) y mismo resultado tras manejo.
6. **Funciones/cierres:** mismo comportamiento observable de llamadas, retornos, capturas y mutaciones visibles.

### 7.2 Diferencias de representación aceptables (a normalizar solo en comparadores)
- Formato de mensajes/`to_string`/prefijos de error, siempre que la categoría sea equivalente.
- Diferencias de `Debug`/formateo interno de `Valor` si el valor semántico es igual (preferir comparación estructural/normalizada explícita).
- Rutas/formatos dependientes de plataforma **solo** cuando el caso no los hace semánticamente relevantes y se documenta.

### 7.3 Características no soportadas
- Un motor **no declara paridad** en una característica si no la soporta realmente: el resultado esperado es un **rechazo/diagnóstico de no soportado** definido (en compilación/lowering/generación o ejecución temprana), no un fallback silencioso a otro motor.
- Las pruebas diferenciales deben etiquetar casos como **soportados por {motores}**; la ausencia de soporte se verifica con aserción explícita de no-soporte cuando aplique (especialmente nativo).

### 7.4 Diferencias intencionales de arquitectura
- Se permiten solo si están **documentadas**, **delimitadas** (capacidad/límite), y **fuera del conjunto de equivalencia** declarado (ej. límites de pasos/profundidad con diagnóstico definido, o subconjunto nativo). No pueden usarse para justificar resultados distintos en casos soportados.

### 7.5 Evitar soporte declarado sin implementación real
- Requerir evidencia: prueba positiva/negativa ejecutada + referencia a código (lowering/runtime/generación).
- Matriz de paridad debe distinguir **soporte declarado / implementado / probado** y estado; CI/pruebas diferenciales por categoría para capacidades reclamadas.

### 7.6 Centralización vs implementaciones independientes (recomendación)
- **Recomendación inicial:** mantener implementaciones independientes (TW/VM/nativo-runtime) pero **consolidar contratos y pruebas comunes** primero; introducir centralización/refactorización compartida **solo** donde la evidencia muestre duplicación defectuosa de reglas (ej. helpers semánticos puros, clasificación de errores, operaciones numéricas ya centralizadas en `Valor`/módulos comunes) y con riesgo controlado. Evitar refactorización global sin causa raíz demostrada por lote.

---

## 8. Estrategia de pruebas diferenciales (propuesta)

### 8.1 Principios
- Ejecutar el **mismo programa/caso** en todos los motores que **soporten** la característica, sin fallback silencioso entre motores.
- Comparar, según aplique: valor final, stdout, errores (categoría/mapeo canónico), efectos secundarios controlados, y comportamiento de funciones/cierres.
- **Normalizar solo presentación irrelevante** (fin de línea, prefijos textuales mapeados a categoría acordada). **No normalizar diferencias semánticas**.
- Cada divergencia/corrección futura debe acompañarse de **caso mínimo reproducible**.

### 8.2 Tipos de pruebas y etiquetado
- **Positivas:** resultado/efecto esperado idéntico (según contrato) en motores soportados.
- **Negativas:** mismo error/categoría (y rechazo equivalente) en motores soportados.
- **Casos límite:** bordes numéricos, colecciones vacías/fuera de rango, recursión/profundidad, Unicode/texto, cierres/mutación, concurrencia observable determinista.
- **No soportadas:** aserción explícita de no-soporte/rechazo en motor que no declara capacidad (p.ej. nativo para closures/try-canales).
- **Bloqueadas:** dependencias externas (infra/toolchain no disponible, B-REG-001); registrar causa y no contabilizar como aprobadas.

### 8.3 Diseño técnico sugerido (para implementación futura autorizada)
- Harness diferencial unificado (extensión de `diferencial_vm.rs`/helpers) con: identificación de motores aplicables por caso, comparador estructural de `Valor`, comparador de salida, y clasificador/mapeo de errores a categorías canónicas.
- Matriz de casos por categoría (§4) con metadatos: `engines=[tw,vm,native]`, `kind=pos|neg|edge|unsupported|blocked`, evidencia/referencia.
- Separar suites de **paridad soportada** de suites de **capacidad/no-soporte** para evitar falsos positivos.
- Mantener ejecuciones explícitas por motor (`--motor`/APIs de test) y registrar logs/comandos.

### 8.4 Brechas actuales de cobertura (a priorizar)
- Casos mínimos dedicados por divergencia D-01/D-03/D-05/D-07.
- Paridad de módulos/imports/exports y resolución entre TW↔VM (y límites nativos).
- Comparación sistemática de **categorías de error** (no solo strings normalizados ad-hoc).
- Efectos secundarios/IO y concurrencia determinista más allá de escenarios felices actuales.
- Combinaciones de tipos/conversiones/comparaciones y shadowing/ámbitos no exhaustivos.

---

## 9. Plan de ejecución por lotes (propuesta, Fase 04)

> Lotes pequeños y verificables, limitados a **paridad de capacidades existentes** (sin absorber Fase 05: backend nativo avanzado y funciones de primera clase como trabajo específico). Orden sugerido por impacto/dependencia; sujeto a autorización y ajuste por evidencia.

| Lote | Objetivo | Divergencia/área | Causa raíz (hipótesis a confirmar) | Archivos previstos | Cambios propuestos | Tests | Validaciones | Riesgos | Criterios de aceptación |
|---|---|---|---|---|---|---|---|---|---|
| L0 (preparación formal) | Fijar contrato semántico y taxonomía de errores/capacidades | D-01, D-04, método | Ausencia de contrato/mapeo canónico | `docs/reports/FASE_04_*` (o doc de contrato), helpers de test (si autorizado) | Definir categorías, comparadores y política de no-soporte/fallback | Diseño/revisión de casos (sin debilitar existentes) | fmt/clippy/ws + revisión | Ambigüedad contractual | Contrato aprobado; mapeo de categorías consensuado; sin cambio de comportamiento |
| L1 | Unificar comparación de errores TW↔VM (categorías) | D-01 | Representaciones distintas `CausaError` vs `ErrorVm` | `src/evaluador.rs`, `src/vm.rs`, `tests/diferencial_vm.rs`, posibles helpers | Introducir clasificación/mapeo canónico (o adaptadores) preservando semántica | Nuevos casos negativos mínimos + existentes diferenciales verdes | fmt/clippy/ws + diferencial | Regresión diagnóstica/UX | Misma categoría en casos soportados; sin fallback; tests previos intactos |
| L2 | Alinear semántica de valores/operaciones/comparaciones/conversiones en casos divergentes detectados | Categ. 1–4, A/B/C/D | Reglas duales o lowering inconsistente | `src/valor.rs`, `src/evaluador.rs`, `src/vm.rs`, `src/hir.rs`/`src/mir.rs` (según causa) | Corrección puntual por caso mínimo con evidencia | Positivos/negativos/límites por tipo | fmt/clippy/ws + suites conformidad/diferencial | Cambios sutiles de coerción | Casos objetivo equivalentes; sin alterar casos ya válidos |
| L3 | Paridad control de flujo, ámbitos, variables y funciones/retornos (TW↔VM) | Categ. 5–7, 8 (excl. 1ª clase avanzada) | Diferencias de evaluación/lowering | `src/evaluador.rs`, `src/vm.rs`, `src/mir.rs`/`src/hir.rs` | Correcciones dirigidas por evidencia | Casos de shadowing/retorno/recursión/bordes | fmt/clippy/ws + diferencial | Efectos en CFG/gates | Equivalencia observable en matriz actualizada |
| L4 | Paridad colecciones, texto/stdlib observable y mutación (TW↔VM) | Categ. 11 y soporte stdlib común | Implementaciones duplicadas/índices/errores | `src/evaluador.rs`, `src/vm.rs`, stdlib/ops relevantes | Alinear comportamiento y errores de índice/operación | Pos/neg/límites de colecciones y strings | fmt/clippy/ws + conformidad/diferencial | Regresiones en API stdlib | Resultados y errores-categoría equivalentes en casos soportados |
| L5 | Paridad excepciones (try-catch/propagación) TW↔VM | Categ. 14, D-03 (parcial) | Manejo distinto en MIR/VM vs TW | `src/evaluador.rs`, `src/vm.rs`, `src/mir.rs` | Ajustes de propagación/captura según contrato | Casos mínimos de anidación/propagación | fmt/clippy/ws + diferencial | Cambios de control-flow | Mismo comportamiento en casos soportados |
| L6 | Paridad módulos/imports/alias/exports (TW↔VM) dentro de capacidades existentes | Categ. 15, D-07 | Lowering/carga/resolución diferente | `src/cargador.rs`, `src/hir.rs`, `src/mir.rs`, `src/evaluador.rs`, `src/vm.rs` | Alinear resolución y errores observables | Casos mínimos de import/alias/export | fmt/clippy/ws + e2e módulos existentes | Complejidad de resolución | Comportamiento equivalente o no-soporte explícito documentado |
| L7 | Concurrencia soportada (canales/tareas) TW↔VM: contrato de observables y determinismo de pruebas | Categ. 16 | Aislamiento/orden/errores distintos | `src/evaluador.rs`, `src/vm.rs`, runtime de VM | Ajustes solo si divergencia observable y determinista; pruebas robustas | Casos deterministas, negativos y límites | fmt/clippy/ws + diferencial | Flakiness/no-determinismo | Resultados observables definidos y estables en casos soportados |
| L8 | Delimitación explícita de paridad nativa (subconjunto) y harness de no-soporte | D-02, D-03, D-08 | Alcance intencional/limitado del backend | `tests/backend_nativo.rs`, docs/contrato, (código solo si necesario) | Etiquetado de capacidades soportadas/no soportadas; no ampliar alcance | Reforzar aserciones de rechazo/no-soporte y paridad dentro de subconjunto | fmt/clippy/ws + backend_nativo (si `rustc`) | Confundir no-soporte con bug | Matriz clara; sin reclamar paridad fuera de subconjunto |

**Fuera de alcance de Fase 04 (recordatorio):** nuevas capacidades de lenguaje; implementación de funciones de primera clase avanzadas/closures en nativo o expansión mayor del backend nativo propia de Fase 05; cambios de versión/publicación.

---

## 10. Pruebas y evidencias (recopiladas)

- **Línea base:** `/tmp/opencode/fase04_fmt.txt`, `/tmp/opencode/fase04_clippy.txt`, `/tmp/opencode/fase04_ws.txt` (comandos y resultados en §2).
- **Inventario de tests:** `prototipo/tests/` (55 suites en workspace). Diferenciales/paridad clave: `diferencial_vm.rs` (81), `backend_nativo.rs` (45), `conformidad_vm_helper.rs` (`assert_valor_parity`/helpers), múltiples `conformidad*.rs`, `pipeline_3d5_integration.rs` (6), `gate_3c_integration.rs` (27).
- **Código auditado (evidencia primaria):** `src/evaluador.rs` (TW, `CausaError`/`ErrorEjecucion`, contextos canales/tareas), `src/vm.rs` (`MaquinaVirtual`, `ErrorVm`, ejecución MIR), `src/hir.rs`/`src/mir.rs` (lowering, verificadores, IR ops: `LlamadaIndirecta`, `Clausura`/celdas, `EntrarTry`/`Invoke`, `ModuloObjeto`), `src/backend_nativo.rs`/`src/runtime.rs` (generación, runtime embebido, rechazos), `src/cli.rs` (selección motor, fallback, gates, rutas `run/build/analizar/depurar`), `src/cargador.rs`.
- **Limitaciones/bloqueos:** 2 ignorados B-REG-001 (`registry_integration.rs`) externos; nativo depende de `rustc` para ejecución de pruebas generadas.

---

## 11. Riesgos y dependencias

| Riesgo/Dependencia | Impacto | Mitigación propuesta |
|---|---|---|
| Contrato semántico ambiguo (valor/error/efectos) | Correcciones inconsistentes o falsas equivalencias | Aprobar contrato y taxonomía de categorías antes de lotes correctivos (L0) |
| Diferencias legítimas de capacidad (nativo/TW/VM) confundidas con bugs | Cambios fuera de alcance o regresiones | Etiquetar soporte por motor; tratar no-soporte como esperado donde corresponda; respetar límite Fase 05 |
| No-determinismo en concurrencia/canales-tareas | Tests flaky, diagnósticos erróneos | Diseñar casos deterministas y observables; aislar contexto por ejecución |
| Fallback CLI VM→TW y harness no representativos | Paridad aparente no real | Prohibir fallback en pruebas diferenciales; ejercicios explícitos por motor y casos end-to-end cuando sea necesario |
| Conservadurismo/gates HIR-MIR y spans 0,0 | Ruido o bloqueos en diagnóstico, no en semántica runtime | No relajar gates; separar problemas de diagnóstico (Fase 13/LSP) de semántica de ejecución |
| Dependencias externas (PostgreSQL/B-REG-001, `rustc`) | Cobertura incompleta de validación | Registrar como bloqueado/no aprobado; no condicionar paridad TW↔VM a infra externa |
| Refactorización amplia prematura | Regresiones amplias | Cambios mínimos por causa raíz demostrada; centralización solo con evidencia |

---

## 12. Criterios de aceptación (preparación y lotes futuros)

### 12.1 Preparación Fase 04 (esta entrega) — cumplidos
- Inventario real de motores y rutas de ejecución documentado con evidencia de archivos/funciones.
- Matriz de paridad por categoría con soporte/resultado/estado y distinción de equivalencia probada vs pendiente.
- Divergencias y limitaciones identificadas con clasificación y referencias.
- Contrato semántico común propuesto (equivalencia, representación aceptable, errores, no-soporte, diferencias intencionales, evidencia de soporte).
- Estrategia de pruebas diferenciales y brechas de cobertura definidas.
- Plan de lotes ordenado con objetivo/causa/archivos/cambios/tests/validaciones/riesgos/criterios.
- Riesgos, dependencias y decisiones para autorización humana explicitadas.
- Línea base reproducida y registrada (sin inventar porcentajes).

### 12.2 Criterios medibles por lote (para ejecución futura autorizada)
- **Correctitud:** todos los casos objetivo del lote pasan en motores declarados soportados; casos de no-soporte se rechazan explícitamente donde aplique.
- **No regresión:** `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace --no-fail-fast` en verde respecto a línea base (salvo variaciones justificadas y autorizadas de conteos por tests nuevos añadidos, nunca por debilitamiento/eliminación).
- **Evidencia:** caso mínimo reproducible por divergencia, comando/log, y actualización de matriz/estado.
- **Alcance:** sin introducir capacidades de fases posteriores ni absorber Fase 05.
- **Gobernanza:** cambios solo en `main`, sin commit/push salvo autorización explícita, y sin alterar aprobación/autorización de fases sin decisión humana.

---

## 13. Decisiones que requieren autorización humana

1. **Autorizar inicio/ejecución de Fase 04** y (si corresponde) actualizar explícitamente `Fase autorizada (current)` en `DEVELOPMENT_STATE.md`/`ROADMAP_MASTER.md` (esta preparación no lo realiza).
2. **Aprobar el contrato semántico propuesto** (§7), en particular: definición de equivalencia de errores (categoría vs mensaje), normalizaciones permitidas, y política de no-soporte/fallback.
3. **Aprobar alcance y orden de lotes** (§9), confirmando que L1–L7 se limitan a capacidades existentes y que el subconjunto nativo (L8) no se expande hacia objetivos de Fase 05.
4. **Aprobar estrategia de pruebas diferenciales** (§8), incluyendo eventual adición de tests/harness (sin modificar/debilitar pruebas existentes) y manejo de casos bloqueados/ignorados.
5. **Definir tratamiento de divergencias de diagnóstico/UX** (mensajes, spans LSP 0,0) cuando entren en tensión con paridad de categoría semántica.
6. **Validar dependencias operativas** para lotes que toquen nativo/concurrencia/infra (disponibilidad de `rustc`, entorno controlado; B-REG-001 permanece externo/no aprobado).

---

## 14. Condición final

Esta tarea concluye con la **preparación técnica y este informe** (`docs/reports/FASE_04_PREPARACION.md`).

- **NO** se implementaron correcciones.
- **NO** se inició la Fase 04.
- **NO** se inició la Fase 05.
- **NO** se cambió de fase ni se modificó la autorización/aprobación vigente.
- **NO** se crearon ramas.
- **NO** se realizaron commits ni push.

**Pendiente:** revisión del informe y **autorización humana explícita** para cualquier acción posterior (actualización de estado autorizado, inicio de lotes de Fase 04 o implementación).
