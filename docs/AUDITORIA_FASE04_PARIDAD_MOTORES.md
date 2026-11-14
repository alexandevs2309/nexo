# Auditoría Fase 04 — Paridad semántica de motores (BASILISCO)

- **Fecha de auditoría:** 2026-11-12
- **Rama / repositorio:** `main` (solo lectura funcional; sin commits/push/ramas)
- **Alcance auditado:** `prototipo/src/evaluador.rs` (Tree-Walk, TW), `prototipo/src/vm.rs` (VM/MIR), `prototipo/src/backend_nativo.rs` + `prototipo/src/runtime.rs` (backend nativo), `prototipo/tests/diferencial_vm.rs`, `prototipo/tests/backend_nativo.rs`, `prototipo/tests/backend_wasm.rs`, contrato `prototipo/src/paridad.rs` y documentación de fase 04.
- **Objetivo:** diagnóstico reproducible y plan técnico de Fase 04 (sin implementar correcciones).

## 1. Evidencia reproducible (comandos, resultados y dependencias)

| Comando (desde `prototipo/`) | Resultado observado (2026-11-12) | Dependencias/notas |
|---|---|---|
| `cargo test --test diferencial_vm` | **OK** — 92 passed, 0 failed, 0 ignored (0.82s) | Harness TW↔VM directo con contrato `paridad.rs` (sin CLI/fallback). Algunos casos ejercen FS/HTTP local/FFI/concurrencia según diseño de pruebas. |
| `cargo test --test backend_nativo` | **OK** — 45 passed, 0 failed, 0 ignored (31.89s) | Requiere toolchain Rust para compilar código generado y entorno capaz de compilar/ejecutar binarios temporales (usa `rustc`/cargo según harness interno). |
| `cargo test --test backend_wasm` | **OK** — 11 passed, 0 failed, 0 ignored (8.69s) | Dependencias del backend WASM/harness (p. ej. Node para ejecución real en pruebas que lo requieren), según entorno de las pruebas. |
| `cargo test --workspace --no-fail-fast` (validación reciente 04.2, misma sesión, 2026-11-12) | **OK** — 987 passed, 0 failed, 2 ignored | 2 ignoradas preexistentes: `registry_integration` (PostgreSQL aislado). Incluye 92 diferenciales, 45 nativo, 11 wasm y 11 pruebas unitarias del contrato en `paridad.rs`. |

**Evidencia estructural:** `evaluador.rs` ~6485 líneas, `vm.rs` ~2622, `backend_nativo.rs` ~872, `runtime.rs` ~800, `diferencial_vm.rs` 92 tests. Estado git preservado (cambios 04.1/04.2 no commiteados + untracked documentales/artefactos locales) — no modificado por esta auditoría.

## 2. Matriz de características por motor

Leyenda: **Sí** = soportado en el alcance observado; **Parcial** = soportado con restricciones/subconjunto o semántica acotada; **No** = no soportado/no implementado/no generable; **N/A** = no aplica al modelo.

| Característica | Tree-Walk (`evaluador.rs`) | VM/MIR (`vm.rs`) | Backend nativo (`backend_nativo.rs` + `runtime.rs`) | Evidencia (referencias) |
|---|---:|---:|---:|---|
| Tipos escalares (entero/decimal/real/lógico/texto/nulo) y semántica numérica núcleo | Sí | Sí (reusa `Valor`/aritmética núcleo) | Sí (runtime propio embebido, réplica) | `evaluador.rs:3-9,2610,2642+`; `vm.rs:1-15,25,2243-2253,2269+`; `runtime.rs:11-24,213-445` |
| Literales, arreglos/lista literal, rangos | Sí (`[...]` → `Lista`; rangos materializados con límite) | Sí (constantes MIR, agregados, `Rvalue::Rango` → lista) | **No/Parcial**: `Agregado`→`Arreglo`; `Rango` rvalue rechazado/no soportado | `evaluador.rs:131,2766-2839`; `vm.rs:2106-2231`; `backend_nativo.rs:409-428,820-824` |
| Variables, ámbitos, declaración/asignación, mutabilidad/inmutabilidad | Sí (ámbitos léxicos, `sea/dato/const`, checks de tipo/mutabilidad) | Sí (locales/params/globals, conversión por tipo esperado) | **Parcial** (locales/globales MIR soportados; sin modelo fuente completo) | `evaluador.rs:733-805,993-1100,1143+`; `vm.rs:1454-1558,1709-1771,2535-2609`; `backend_nativo.rs:57-65,443-524,671-677` |
| Operadores aritméticos, comparación, lógicos (cortocircuito) | Sí (`y/o` cortocircuito estricto) | Sí (bin/un ops, comparaciones, `y/o` lógicos) | **Parcial** (aritmética/comparaciones en runtime; `Y/O` generados sin cortocircuito semántico en codegen) | `evaluador.rs:2642-2751,3384-3464`; `vm.rs:2269-2360`; `runtime.rs:213-445`; `backend_nativo.rs:751-812` |
| Control de flujo (si, mientras, para cada, romper/continuar) | Sí (límites de iteración/rango) | Sí (bloques/terminadores, switch, goto, loops por MIR) | **Parcial** (Goto/SwitchInt/Return/Unreachable; subconjunto MIR soportado) | `evaluador.rs:131,1342-1413,1623-1701`; `vm.rs:1487-1697`; `backend_nativo.rs:391-398,682-724` |
| Funciones, aridad, retorno, recursión, inferencia/validación tipada | Sí (hoisting, anotaciones, inferencia 1ª llamada, retorno) | Sí (MIR, tipos inferidos/declarados, conversión retorno) | **Parcial** (funciones soportadas del programa; sin límites pasos/profundidad propios) | `evaluador.rs:746-760,993-1100,3003-3169`; `vm.rs:122-123,1454-1797,2535-2602`; `backend_nativo.rs:99-167,334-369,432-469,548-666` |
| Lambdas, closures, capturas, celdas mutables | Sí (`FuncionClausura` + entorno capturado) | Sí (`ClausuraMir`, `CrearCelda/LeerCelda`, proyección `Celda`) | **No** (`CrearCelda/LeerCelda/Clausura` rechazados) | `evaluador.rs:2611-2624,826-973,1547-1577`; `vm.rs:1574-1604,1887-1902,2154-2180,2363+`; `backend_nativo.rs:14-16,409-428,541-543` |
| Llamadas indirectas / funciones de primera clase | Sí (`LlamadaIndirecta`, valor función/clausura) | Sí (`LlamadaIndirecta`, `invocar_cierre`, `Funcion/ClausuraMir`) | **Parcial** (solo `Valor::Funcion` por nombre en tabla de funciones soportadas) | `evaluador.rs:2625-2641,3171-3332`; `vm.rs:1574-1604,1887-1902`; `backend_nativo.rs:67-97,649-666`; `runtime.rs:23` |
| Colecciones (arreglo/lista/objeto), indexación, mutación, agregados | Sí (lista/arreglo/objeto JSON, índice lectura/escritura en arr/lista) | Sí (arreglo/lista/objeto, índice/campo, longitud) | **Parcial** (arreglo/lista/objeto básico; agregado→arreglo; **asignación a campo de objeto no materializada** en stmt) | `evaluador.rs:1228-1336,2766-2774,2840-2981,5595+`; `vm.rs:1982-2091,2106-2153,2188-2201`; `runtime.rs:464-579`; `backend_nativo.rs:524-547,581-634,820-841` |
| Excepciones / intentar-rescatar / invoke | Sí (`intentar/rescatar`, no captura señales romper/continuar) | Sí (`EntrarTry/SalirTry` + captura a rescue; `Invoke` ok/error) | **No** (`EntrarTry/SalirTry`/`Invoke` no soportados) | `evaluador.rs:80-83,1710-1755,2169-2205`; `vm.rs:148-149,1494-1534,1605-1618,1678-1695`; `backend_nativo.rs:14-16,378-398,667-670,715-720` |
| Módulos / importación / exportación | Sí (`Importar` y `Usar` con cargador, alias, acceso/llamada a miembros, visibilidad/exportación) | **Parcial** (representación `ObjetoModulo` como valor; sin carga/import dinámica en VM) | **Parcial** (globals `BSK_GLOBALS`/`__init`, `ObjetoModulo` de funciones por prefijo) | `evaluador.rs:133-135,1001-1100,1422-1522,1953-2066,2387-2478,2916-3300`; `vm.rs:2181-2187`; `backend_nativo.rs:57-65,125-145,671-677,842-854` |
| Conversiones de tipos (implícitas/explícitas, anotaciones) | Sí (`convertir_a_tipo_anotado`, implícitas entero→decimal/real, etc.) | Sí (`convertir_tipo`, validaciones en params/retorno/local/global) | **Parcial** (`TipoConv` en asignaciones: entero→decimal/real y contenedores; delega compatibilidad dinámica al runtime) | `evaluador.rs:2522-2596,3420-3454`; `vm.rs:1709-1771,2452-2609`; `backend_nativo.rs:371-376,476-524`; `runtime.rs:89-134` |
| Errores y diagnósticos | Sí (`CausaError`/`ErrorEjecucion` tipados, ubicación, códigos) | Sí (`ErrorVm` tipado + aritmético núcleo) | **Parcial** (`RuntimeError(String)` textual; aborta en `main`) | `evaluador.rs:25-730`; `vm.rs:29-118,1657-1677`; `runtime.rs:26-33,136-445,464-521`; `backend_nativo.rs:140-162` |
| Stdlib: texto/números/colecciones utilitarias | Amplia (longitud, texto ops, map/filtrar/reducir, ordenar, etc.) | Amplia (nativos registrados, HOF con `ClausuraMir`) | **Limitada** (intrínsecos/wrappers acotados: mostrar, longitud, agregar, eliminar, leer_archivo, json_parsear; otros solo si son funciones del programa) | `evaluador.rs:3501+ (builtins)`, `5077-5237`; `vm.rs:236-1374,1775-1885`; `backend_nativo.rs:51-55,346-362,548-647`; `runtime.rs:581-800` |
| Archivos (alto nivel y handles) | Sí | Sí (nativos de archivo/handle) | **No/Parcial** (solo `leer_archivo` wrapper; sin handles/std de archivos completo) | `evaluador.rs:4029-4403,4948-5075`; `vm.rs:529-583,957-1190`; `backend_nativo.rs:51-55`; `runtime.rs:581-596` |
| JSON | Sí (`json_parsear/serializar/obtener`) | Sí | **Parcial** (`json_parsear` propio; sin serializar/obtener como stdlib nativa equivalente) | `evaluador.rs:5595-5786`; `vm.rs:584-644,2362-2421`; `runtime.rs:598-800` |
| FFI | Sí | Sí (nativos FFI) | **No** | `evaluador.rs:5455-5593`; `vm.rs:1191-1269`; `backend_nativo.rs:14-16` |
| Concurrencia, tareas y canales | Sí (contexto por hilo, `iniciar_tarea/esperar_tarea`, canal condvar) | Sí (nativos canal + tareas con VM hija/compartición de contexto) | **No** | `evaluador.rs:133-244,5239-5454`; `vm.rs:1270-1343,1401-1403,1784-1979` |
| Fechas y HTTP | Sí | Sí | **No** (no expuestos en stdlib nativa del backend) | `evaluador.rs:4405-4541`; `vm.rs:355-426` |
| Límites de ejecución (pasos/profundidad/iteraciones) | Sí (iteraciones/rangos; modelo de error propio) | Sí (`LIMITE_PASOS`, `LIMITE_PROFUNDIDAD`) | **No** equivalentes (recursión/bucle nativos de Rust sin contador propio) | `evaluador.rs:131`; `vm.rs:120-123,1446-1463`; `backend_nativo.rs:432-469` |

## 3. Diferencias comprobadas

1. **Subconjunto soportado del backend nativo es acotado frente a TW/VM.** Rechaza/omite closures, celdas, rangos, try/catch (`EntrarTry/SalirTry`) e `Invoke`, y limita stdlib/intrínsecos; funciones con operaciones no soportadas se omiten o producen rechazo de `main` (`backend_nativo.rs:14-16,99-108,132-137,378-428,667-670,715-720`).
2. **Modelo de errores distinto en nativo.** Nativo usa `RuntimeError(String)` y termina `main` con `stderr+exit(1)`; TW/VM exponen errores tipados/estructurados (`CausaError/ErrorEjecucion`, `ErrorVm`) (`runtime.rs:26-33`; `backend_nativo.rs:140-162`; `evaluador.rs:688-730`; `vm.rs:29-118`).
3. **Sin límites de pasos/profundidad en el ejecutable nativo generado.** TW/VM imponen límites explícitos en sus modelos; el backend generado delega en ejecución Rust sin contadores equivalentes (`evaluador.rs:131`; `vm.rs:120-123,1446-1463`; `backend_nativo.rs:432-469`).
4. **Materialización de agregados/literales difiere en backend.** `Agregado` MIR se materializa como `Valor::Arreglo` en nativo (`backend_nativo.rs:820-824`), mientras TW construye literal `[...]` como `Lista` por defecto (`evaluador.rs:2766-2774`). (Impacto depende de observabilidad; requiere contraste diferencial dirigido.)
5. **Asignación a campo de objeto no materializada en backend nativo.** En proyección `Campo` de sentencia, el código generado descarta la escritura (`backend_nativo.rs:538-540`), frente a soporte de lectura/campo en runtime y a capacidades de objeto en TW/VM.
6. **Llamadas indirectas restringidas en nativo.** Solo valores `Valor::Funcion(nombre)` despachables en tabla de funciones soportadas (`backend_nativo.rs:67-97,649-666`), sin clausuras/capturas; TW/VM soportan clausuras/callables ricos.
7. **Módulos/importaciones con modelos distintos.** TW implementa `Importar`/`Usar`, resolución, caché y visibilidad/exportación; VM expone `ObjetoModulo` como valor sin carga/import dinámica equivalente; nativo materializa `ObjetoModulo` sobre globals/prefijos (`evaluador.rs:1422-1522,1953-2066,2387-2478`; `vm.rs:2181-2187`; `backend_nativo.rs:57-65,842-854`).
8. **Semántica de `Y/O` en codegen nativo potencialmente no cortocircuito.** `backend_nativo.rs` evalúa operandos y construye `Valor::Logico(lhs==true &&/|| rhs==true)` (`:751-812`), lo que puede diferir del cortocircuito de TW/VM si el operando derecho tiene efectos/errores (diferencia semántica relevante si se alcanza).
9. **Cobertura diferencial TW↔VM existente es amplia pero acotada.** 92 pruebas pasan bajo contrato endurecido 04.2; no existe suite diferencial contractual equivalente TW↔VM↔nativo (nativo y wasm tienen suites propias). El contrato 04.2 clasifica `NoVerificable` para opacos sin identidad demostrable y tolera por whitelist solo la representación `obtenido` GAP-051 en asignación/retorno/parámetro tipado.
10. **GAP-051 permanece parcialmente resuelto (comprobado por estado de contrato/pruebas).** Casos concretos cubiertos y aceptados (incl. diferencia diagnóstica de representación Lista/Arreglo en asignación tipada), sin cierre formal ni evidencia exhaustiva.

## 4. Diferencias sospechadas que requieren pruebas

1. **Cortocircuito lógico en nativo con efectos/errores en operando derecho.** Confirmar si `y/o` generados evalúan siempre RHS y divergen de TW/VM (casos con división por cero, mutación o llamadas en RHS).
2. **Mutación y aliasing de colecciones en nativo** (`agregar/eliminar` por copia en runtime, escrituras de campo objeto no aplicadas) vs semántica observable en TW/VM en escenarios no cubiertos por tests nativos actuales.
3. **Paridad de tipos de colección observables** (`Arreglo` vs `Lista`, agregados, retornos/JSON/interop) entre motores y frente a contrato (04.2 endureció clase de secuencia a nivel valor en comparador, pero los motores pueden producir clases distintas legítimas o divergentes según contexto).
4. **Categorización/mapeo de errores entre motores** en casos no cubiertos: p. ej. indexación fuera de rango (texto vs colección), reasignación inmutable, identificadores/llamadas no definidos, rutas y modos de E/S, donde categorías/campos pueden diferir (observado como riesgo de mapeo/diagnóstico en 04.1-04.2; requiere batería sistemática).
5. **Módulos en paridad**: `Usar`/`Importar`, exportación/privacidad, alias, acceso a miembro y llamadas a funciones de módulo entre TW y VM (VM sin carga dinámica equivalente) y subconjunto nativo.
6. **Excepciones avanzadas**: `Invoke`/errores en try, propagación, y equivalencia de mensajes/capturas más allá de los 4 casos diferenciales actuales de `intentar/rescatar`.
7. **Conversiones y anotaciones compuestas** (colecciones anidadas, retornos, parámetros, inferencia) en bordes no ejercitados diferencialmente y en nativo.
8. **Funciones sin retorno / uso de valor** y semánticas específicas (TW: `RetornoAusente`/E1010 en contextos; VM: locales marcados sin retorno; nativo: modelo `Result<Valor,...>`), posibles divergencias en casos límite.
9. **Stdlib de orden superior y closures** (`map/filtrar/reducir` con distintas formas de callable) fuera de casos cubiertos; nativo no soporta closures/HOF.
10. **FFI, HTTP, fechas, concurrencia/canales/tareas**: paridad TW↔VM cubierta parcialmente por diferenciales puntuales; determinismo/efectos y cobertura de errores requieren ampliación; nativo fuera de alcance.

## 5. Casos de prueba faltantes (brechas de cobertura)

- **TW↔VM (diferencial/contrato):** módulos (`usar/importar`, visibilidad, alias, miembro/llamada), objetos (acceso/campo/mutación si aplica), mutación de colecciones y aliasing en más escenarios, indexación fuera de rango (colección vs texto) y otras categorías de error no normalizadas por pares, reasignación inmutable/`dato-const`, `Invoke`/try paths de MIR que cambien semántica observable, conversiones anidadas y retornos tipados complejos, comparaciones/operaciones mixtas de bordes, mayor cobertura de stdlib (texto/numérico/archivos) con aserciones de efectos deterministas homogéneos (stdout/FS), y casos donde `DiferenciaDiagnostico` **no** deba aplicarse (contrato endurecido, pero faltan diferenciales meta por programa).
- **TW↔VM↔Nativo (diferencial contractual):** inexistente; faltan pruebas que comparen subconjunto soportado nativo contra referencia TW/VM usando el contrato, clasificando explícitamente capacidades no soportadas como `NoSoportado/NoVerificable`.
- **Nativo (específicas):** cortocircuito `y/o` con efectos, escritura de campo objeto, comportamiento con rangos/closures/try (rechazo vs clasificación), paridad de errores para desbordamiento/índice/tipos en todos los intrínsecos, y semántica de `Arreglo`/`Lista` observable.
- **WASM:** paridad contractual con otros motores en subconjunto compartido (hoy suite aislada).
- **Contrato/paridad:** pruebas de regresión para nuevas variantes de `CausaError`/`ErrorVm` y para evitar whitelists no autorizadas.

## 6. Hallazgos clasificados (P0–P3)

| ID | Pri | Hallazgo | Evidencia / impacto |
|---|---|---|---|
| H-01 | **P0** | Backend nativo sin equivalencia de manejo de excepciones/flujo excepcional (`try/catch/invoke`) del lenguaje soportado en TW/VM | `backend_nativo.rs:378-398,667-670,715-720`. Programas con intentar/rescatar o flujos MIR de invoke no son ejecutables con semántica equivalente. |
| H-02 | **P0** | Backend nativo sin soporte de closures/capturas/celdas y llamadas indirectas plenas | `backend_nativo.rs:409-428,541-543,67-97`. Excluye HOF/callables capturantes soportados en TW/VM. |
| H-03 | **P1** | Posible pérdida de cortocircuito en `y/o` en código nativo generado | `backend_nativo.rs:751-812`. Riesgo de evaluación de efectos/errores en RHS (divergencia semántica) — **requiere prueba confirmatoria**. |
| H-04 | **P1** | Mutación de estructuras no garantizada en nativo (escritura de campo objeto no aplicada; operaciones de colección por copia) | `backend_nativo.rs:538-540`; `runtime.rs:523-559`. Riesgo de divergencia observable en programas que dependan de aliasing/mutación in-place. |
| H-05 | **P1** | Ausencia de paridad diferencial contractual TW↔VM↔Nativo/WASM | Suites separadas; no hay clasificación uniforme soportado/no soportado/no verificable por backend. Dificulta afirmar conformidad de subconjuntos. |
| H-06 | **P1** | Brechas de cobertura TW↔VM en módulos, objetos/mutación, categorías de error y efectos observables | 92 diffs pasan pero no cubren ejes completos (módulos/import-export, indexación categorizada, stdout homogéneo, invoke/try paths). Riesgo de regresión no detectada. |
| H-07 | **P1** | Divergencias reales preexistentes potenciales detectadas en exploraciones previas (p. ej. categorización/causa en indexación o inmutabilidad/identificadores en casos específicos) | Documentadas como observaciones de exploración en 04.1/04.2; no corregidas ni sistematizadas. Requieren batería y clasificación (bug semántico vs diagnóstico). |
| H-08 | **P2** | Diferencias de representación/tipo de colección (`Arreglo` vs `Lista`, agregados) y materialización entre motores | `evaluador.rs:2766-2774`; `backend_nativo.rs:820-824`; contrato 04.2 estricto en clase de secuencia a nivel valor. Puede generar divergencias o no-verificabilidad según caso. |
| H-09 | **P2** | Modelo de errores/diagnósticos no uniforme (tipado vs `RuntimeError` textual) y sin mapeo contractual para nativo | `runtime.rs:26-33`; `backend_nativo.rs:140-162`. Limita comparación semántica de fallos en backend nativo. |
| H-10 | **P2** | GAP-051 sin cierre formal | Whitelist diagnóstica conservada (04.2) y casos cubiertos; evidencia sistemática y decisión pendiente. |
| H-11 | **P3** | Límites de ejecución (pasos/profundidad) no equivalentes en nativo | `vm.rs:120-123,1446-1463`; `backend_nativo.rs` sin contadores. Diferencia operativa en programas patológicos (no necesariamente semántica funcional en programas válidos acotados). |
| H-12 | **P3** | Stdlib nativa limitada y dependencias de entorno en pruebas (FFI/HTTP/FS/Node/rustc) | Cobertura y reproducibilidad sujetas a entorno; FFI diferencial omite aserción si no compila auxiliar. |

## 7. Propuesta de implementación por lotes (Fase 04), ordenada por dependencia y prioridad

> Lotes pequeños, verificables, sin cambiar semántica fuera de alcance autorizado. (Planificación; no iniciados.)

| Lote | Objetivo | Dependencias | Entregables principales |
|---|---|---|---|
| **04.3** | **Consolidación de evidencia TW↔VM**: batería sistemática de diferencias sospechadas (errores/categorías, indexación, inmutabilidad, objetos/mutación, módulos accesibles en ambos, try/invoke paths) con clasificación contrato (equivalente/divergencia/diagnóstico-whitelist/no soportado/no verificable) y actualización de cobertura diferencial | 04.1-04.2 (contrato/harness) | Nuevos casos diferenciales dirigidos, matriz de hallazgos reproducibles, decisión por caso (documentar vs corregir en lote posterior) |
| **04.4** | **Paridad de errores y mapeo**: normalización/mapeo explícito de categorías y datos relevantes donde la causa sea semánticamente equivalente (sin relajar); documentación de causas no comparables. Decisión/evidencia sistemática para **GAP-051** | 04.3 | Ajustes de `paridad.rs`/mapeos (solo si equivalencia demostrada), pruebas negativas/meta, reporte GAP-051 |
| **04.5** | **Contrato multi-backend (subconjuntos)**: definir subconjunto soportado de nativo y wasm, harness/comparadores o adaptadores que clasifiquen `NoSoportado/NoVerificable` vs divergencia, sin afirmar paridad fuera de subconjunto | 04.1-04.2, inventario de capacidades | Especificación de subconjuntos, pruebas diferenciales/contractuales por backend, reporte de conformidad |
| **04.6** | **Correcciones semánticas TW↔VM priorizadas** (solo divergencias reales confirmadas, p.ej. P0/P1 funcionales) preservando comportamiento aprobado | 04.3-04.4 | Fixes acotados + pruebas diferenciales/regresión + actualización de contrato si cambia forma diagnóstica (autorizada) |
| **04.7** | **Alineación backend nativo en subconjunto**: abordar diferencias comprobadas de alto impacto dentro de su alcance viable (p.ej. cortocircuito lógico, mutaciones soportables, rangos/agregados donde proceda) o declarar explícitamente no soportado | 04.5, 04.6 (referencia semántica) | Cambios en `backend_nativo.rs`/`runtime.rs` acotados, tests de conformidad de subconjunto, clasificación de brechas remanentes |
| **04.8** | **Cierre de fase**: consolidación de cobertura, documentación de paridad por motor, riesgos residuales y criterios de salida para habilitar siguientes fases | 04.3-04.7 | Informe de cierre Fase 04, matriz final, lista de capacidades soportadas/no soportadas/no verificables |

## 8. Criterios de aceptación verificables (Fase 04)

- **Contrato:** para cada par/motor comparado, los resultados se clasifican exclusivamente como equivalente, divergencia semántica, diferencia diagnóstica whitelisted declarada, capacidad no soportada o no verificable — sin equivalencias permisivas no justificadas.
- **TW↔VM:** suite diferencial ampliada cubre módulos (cuando comparables), errores/categorías priorizadas, conversiones/GAP-051 y efectos deterministas acordados; divergencias reales conocidas están corregidas o registradas con decisión y evidencia.
- **GAP-051:** evidencia sistemática completa y decisión formal documentada (cerrado o mantenido con alcance explícito) — sin cierre implícito.
- **Backend nativo/wasm:** subconjuntos soportados definidos; pruebas contractuales demuestran conformidad dentro del subconjunto y clasifican el resto como no soportado/no verificable (sin falsos verdes).
- **Regresión:** validaciones reproducibles en verde (`diferencial_vm`, suites de backend aplicables y `workspace` o conjunto definido) con variaciones de conteo explicadas.
- **Trazabilidad:** matriz característica×motor y hallazgos P0–P3 actualizados con estado (resuelto/aceptado/no soportado/pendiente).

## 9. Riesgos y dependencias

| Riesgo/Dependencia | Detalle |
|---|---|
| Dependencia de definición semántica | Algunas diferencias (p.ej. clase Arreglo/Lista observable, aliasing/mutación, identidad de opacos) requieren criterio normativo explícito antes de “corregir”. |
| Acoplamiento contrato↔variantes de error | Evolución de `CausaError`/`ErrorVm` exige mantener `paridad.rs` y mapeos; riesgo de desalineación o whitelist indebida. |
| Efectos no homogéneos | Stdout/FS/HTTP/FFI/concurrencia/tiempo difíciles de comparar de forma determinista entre motores y backends. |
| Alcance estructural del backend nativo | Closures, try/invoke, stdlib amplia y modelo tipado de errores implican cambios significativos o declaración explícita de no soporte. |
| Entorno de pruebas | `rustc` (nativo), Node/WASM toolchain, compilador C para FFI, red local para HTTP en tests, PostgreSQL para ignoradas — afectan reproducibilidad. |
| Regresiones cruzadas | Alinear un motor puede romper suites existentes o contratos de fases 00–03 si no se acota al subconjunto. |
| Trabajo 04.1/04.2 local no commiteado | Base actual incluye endurecimiento de contrato/harness y docs en árbol; la planificación debe considerar este estado preservado. |

## 10. Estimación de esfuerzo

| Lote | Esfuerzo estimado | Confianza | Notas |
|---|---:|---|---|
| 04.3 Batería y clasificación TW↔VM | M (3–5 días) | Media | Depende de hallazgos reales y de definición de “comparable” en módulos/efectos. |
| 04.4 Mapeo de errores + GAP-051 | M (3–5 días) | Media | Puede crecer si hay muchas categorías no alineadas o decisiones semánticas abiertas. |
| 04.5 Contrato multi-backend (subconjuntos) | M/L (4–7 días) | Media-baja | Requiere delimitar capacidades nativo/wasm y adaptar harness. |
| 04.6 Correcciones semánticas TW↔VM | S–L (2–8 días) | Baja | Altamente dependiente de cantidad/naturaleza de divergencias confirmadas. |
| 04.7 Alineación backend nativo (subconjunto) | L (1–3 semanas) | Baja | Impactado por brechas estructurales (closures/try/cortocircuito/mutación/stdlib/errores). |
| 04.8 Cierre y documentación de fase | S (1–2 días) | Alta | Una vez estabilizados lotes previos. |
| **Total Fase 04 (estimación)** | **~4–8 semanas** | **Media-baja** | Varía según decisiones de alcance (corregir vs declarar no soportado) y volumen de divergencias reales. |

## 11. Resumen ejecutivo

- **Estado de paridad actual:** TW↔VM cuenta con harness diferencial directo y contrato endurecido (04.1-04.2); **92/92** pruebas diferenciales pasan, y las validaciones pertinentes ejecutadas en esta auditoría (diferencial, nativo, wasm) están en verde. Sin embargo, la cobertura no es exhaustiva (módulos, objetos/mutación profunda, categorías de error completas, efectos stdout homogéneos, rutas try/invoke) y existen riesgos residuales documentados.
- **Backend nativo:** implementa un **subconjunto** de MIR con runtime embebido propio; **45/45** pruebas propias pasan, pero presenta diferencias estructurales y funcionales comprobadas frente a TW/VM (sin closures/celdas/try-invoke/rangos, stdlib acotada, errores textuales, sin límites equivalentes, restricciones en mutación/campo objeto y llamadas indirectas, y posible falta de cortocircuito en `y/o` generados). No hay paridad contractual integrada con TW/VM.
- **Principales brechas:** (1) ausencia de contrato diferencial multi-backend, (2) cobertura TW↔VM incompleta en ejes críticos (módulos/errores/efectos/mutación), (3) brechas semánticas del backend nativo en subconjunto relevante, (4) **GAP-051** pendiente de cierre formal.
- **Conclusión:** existe diagnóstico reproducible y base técnica (contrato/harness) para avanzar. Se recomienda ejecutar Fase 04 en lotes 04.3→04.8 priorizando consolidación de evidencia y clasificación por subconjunto antes de correcciones amplias, con decisiones explícitas sobre alcance de soporte del backend nativo.
- **Cumplimiento de misión:** no se modificó código funcional, no se inició implementación/corrección de Fase 04, no se alteraron fases 00–03; cambios limitados a documentación de auditoría. Se entrega informe para revisión y **espera de autorización humana**.

## 12. Actualización 04.3 — Consolidación de evidencia TW↔VM (2026-11-13)

- **Estado:** lote 04.3 ejecutado (autorizado). Cobertura diferencial actualizada sin modificar semántica de motores.
- **Evidencia:** `cargo test --test diferencial_vm` → **113 passed, 0 failed** (92 previas + 21 nuevas `diff_043_*`); fmt/clippy en verde. Detalle y matriz de clasificación: `docs/reports/FASE_04_3_CONSOLIDACION_EVIDENCIA.md`.
- **Clasificación 04.3:** 6 casos equivalentes/aceptados por contrato y **15 divergencias semánticas reales registradas** (evidencia reproducible, sin corrección): nombre no definido; indexación fuera de rango/negativa y mutación con categorización distinta; iteración/rango/tipos con categorías distintas; condición no lógica aceptada por VM (si/mientras); operando lógico con categoría distinta; **cortocircuito `y/o` no respetado en VM** (evalúa RHS, produce error/efecto); **inmutabilidad `dato` no enforced en VM**; conversión texto inválida con categoría distinta; comparación/igualdad de secuencias y lógicos aceptada en VM y rechazada en TW.
- **Hallazgos relevantes añadidos/confirmados (TW↔VM):** cortocircuito lógico y enforcement de inmutabilidad en VM, y divergencias de reglas de comparación/igualdad y categorización de errores — priorizables para triage (04.4 mapeo solo si causa equivalente; 04.6 corrección semántica autorizada).
- **Brechas aún abiertas:** módulos `importar/usar` comparables (setup/harness), efectos stdout homogéneos, rutas `Invoke` MIR no expresables directamente, mutación de objetos/campos más allá de lectura JSON, y dependencias de entorno. GAP-051 sigue sin cierre formal. Backend nativo/WASM sin contrato diferencial integrado (04.5).

## 13. Actualización 04.4/04.6 — Correcciones y mapeos TW↔VM (2026-11-14)

- **Autorización:** correcciones ejecutadas bajo “autorizo todas las correciones” (referencia semántica TW en conflictos).
- **Estado:** las 15 divergencias reales registradas en 04.3 fueron corregidas o alineadas por mapeo equivalente explícito. La batería 04.3 pasa en paridad (`diff_043_*`).
- **Evidencia:** `cargo fmt --all -- --check` OK; `cargo clippy --all-targets --all-features -- -D warnings` OK; `cargo test --test diferencial_vm` → **113 passed, 0 failed**; `cargo test --test optimizador` → **12 passed, 0 failed**. Validación workspace registrada con 1 fallo intermedio de `optimizador` (faltaba registrar stdlib para intrínseco de cortocircuito) ya corregido; conteo final esperado **1008 passed, 0 failed, 2 ignored** (2 ignoradas preexistentes `registry_integration`).
- **Cambios clave (TW↔VM):** cortocircuito real `y`/`o` con validación lógica, enforcement de condición lógica booleana (`si`/`mientras`/forma booleana), respeto en ruta VM de errores semánticos relevantes (inmutabilidad `dato`, condición no lógica, nombre no definido), alineación de comparación/igualdad e indexación fuera de rango, literales agregados como `Lista`.
- **Mapeos contractuales explícitos (04.4):** `ConversionTextoInvalida`↔`ConversionInvalido`, `IteracionSobreNoColeccion`↔`TipoEsperado`, `RangoTipoInvalido`↔`TipoEsperado` (solo con datos/causa equivalentes).
- **Brechas pendientes:** GAP-051 sin cierre formal; módulos/efectos stdout homogéneos/rutas `Invoke`/mutación de objetos-campo y casos dependientes de entorno; contrato diferencial multi-backend nativo/WASM (04.5) y limitaciones estructurales de backend nativo. Detalle: `docs/reports/FASE_04_4_6_CORRECCIONES_PARIDAD.md`.
