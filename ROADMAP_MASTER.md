# ROADMAP_MASTER.md — BASILISCO

> **Roadmap secuencial obligatorio (15 fases: 00–14).** No asumir ninguna fase completada. La Fase 00 verifica el estado real del código. Una fase solo pasa a `APROBADA` cuando **todos** sus criterios obligatorios se satisfacen con evidencia y existe **autorización humana explícita**. Tests ignorados/bloqueados/no ejecutados **no** cuentan como aprobados.

## Convención de campos (toda fase)
- **Objetivo | Alcance permitido | Dependencias | Tareas | Archivos afectados previstos | Pruebas requeridas | Criterios de aceptación | Evidencias | Estado** (`PENDIENTE`/`EN PROGRESO`/`BLOQUEADA`/`APROBADA`).
- Evidencias mínimas: rutas de archivos, comandos ejecutados, exit code, contadores/resultados relevantes, fecha/contexto.

---

## Fase 00 — Auditoría y línea base actual
- **Objetivo:** Establecer línea base verificada del estado real (código, pruebas, paridad por motor, bloqueos) sin confiar en informes previos.
- **Alcance permitido:** Inspección de código, ejecución de pruebas/comandos existentes, generación/actualización de informes de auditoría y `DEVELOPMENT_STATE.md`. Sin fixes funcionales de compilador/runtime (salvo que se autorice explícitamente como lote dentro de Fase 00 y quede evidenciado).
- **Dependencias:** Ninguna.
- **Tareas:** (1) Inventariar arquitectura y capacidades por motor (TW/VM/nativo/WASM). (2) Ejecutar verificaciones reproducibles (`fmt --check`, `clippy -D warnings`, `cargo test --workspace --no-fail-fast`). (3) Ejecutar tests dirigidos por área crítica; registrar skips/flakiness. (4) Clasificar hallazgos con evidencia (no “verde” aparente). (5) Actualizar estado/bloqueos.
- **Archivos afectados previstos:** `DEVELOPMENT_STATE.md`, `AUDITORIA_*` (informes), `docs/governance/*` (si aclaración de evidencia); **sin** cambios a `prototipo/src`/tests salvo autorización explícita.
- **Pruebas requeridas:** `cargo fmt --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test --workspace --no-fail-fast`; pruebas dirigidas de áreas críticas (diferenciales, nativo, wasm, paquetes). Integración remota: solo si deps disponibles; si no, **BLOQUEADA** documentada.
- **Criterios de aceptación:** Línea base documentada con comandos+resultados; discrepancias/flakiness/skips identificados (p. ej. WASM skips, canales flaky, registry bloqueado) con causa; `DEVELOPMENT_STATE.md` consistente; ninguna capacidad marcada verificada sin ejecución.
- **Evidencias:** `/tmp` logs o rutas de resultados, conteos, outputs de fallos, referencias a archivos/funciones.
- **Estado:** `PENDIENTE` (inicial). *Ninguna fase aprobada por defecto.*

## Fase 01 — Corrección del backend WASM y sus pruebas
- **Objetivo:** Backend WASM honesto y verificable: generación compatible con toolchain soportada y tests que **no** pasen en silencio ante fallos de compilación/ejecución.
- **Alcance permitido:** `prototipo/src/backend_wasm.rs`, `prototipo/src/backend_entero.rs` (si aplica al subset WASM), `prototipo/tests/backend_wasm.rs`, docs técnicas WASM si existen; solo lo necesario al objetivo.
- **Dependencias:** Fase 00 `APROBADA`.
- **Tareas:** Alinear emisión (`#[unsafe(no_mangle)]`/forma válida) a rustc+edition soportados; hacer que tests fallen o marquen skip **explícito con razón verificable** cuando falte target/node/rustc, y **nunca** “ok” si la compilación/ejecución esperada falla; demostrar e2e real (`.wasm` + Node) para casos del subset; definir paridad declarada vs VM.
- **Archivos afectados previstos:** `backend_wasm.rs`, `backend_entero.rs`, `tests/backend_wasm.rs`, posiblemente `cli/build` docs.
- **Pruebas requeridas:** `cargo test --test backend_wasm` con ejecución real (no skip silencioso); prueba e2e programa real rustc→`.wasm`→Node con resultado correcto; `fmt`/`clippy`/`workspace` verdes.
- **Criterios de aceptación:** 0 falsos verdes WASM; e2e reproducible evidenciado; subset/paridad documentados con evidencia.
- **Evidencias:** `backend_wasm.rs`/`tests/backend_wasm.rs`; `cargo test --test backend_wasm` 11/0/0; fmt/clippy OK; workspace 880/0/2 (ver `DEVELOPMENT_STATE.md`).
- **Estado:** `APROBADA` (autorización humana explícita).

## Fase 02 — Corrección de concurrencia y canales
- **Objetivo:** Eliminar no determinismo/races en canales/tareas (VM y donde aplique) y obtener pruebas deterministas y fiables.
- **Alcance permitido:** `prototipo/src/vm.rs`, `prototipo/src/evaluador.rs` (canales/TW), `prototipo/src/mir.rs/hir.rs` solo si necesario al fix, `tests/diferencial_vm.rs` u tests de concurrencia; sin nuevas features de concurrencia.
- **Dependencias:** Fase 00–01 `APROBADAS`.
- **Tareas:** Reproducir flakiness (evidencia previa: divergencias TW/VM canal 42/62 vs 10/30); aislar estado global/IDs/colas por contexto de ejecución o prueba; corregir causa raíz; hacer diferenciales canal/tarea deterministas; re-ejecutar repetidamente para evidencia de estabilidad.
- **Archivos afectados previstos:** `vm.rs`, `evaluador.rs`, tests diferenciales/concurrencia.
- **Pruebas requeridas:** `cargo test --test diferencial_vm canal tarea` (y repeticiones); `workspace` completo; sin regresión de paridad.
- **Criterios de aceptación:** pruebas de canal/tarea deterministas (N repeticiones documentadas, 0 fallos); causa raíz explicada con evidencia.
- **Evidencias:** aislamiento por contexto en `evaluador.rs`/`vm.rs`; diferencial canal repetido (12×2/0 + 8×2/0), `diferencial_vm` 81/0; fmt/clippy OK; workspace 887/0/2 (B-REG-001 externo). GAP-051 preexistente preservado/no atribuido. Ver `DEVELOPMENT_STATE.md`.
- **Estado:** `APROBADA` (autorización humana explícita).

## Fase 03 — Validadores semánticos HIR/MIR
- **Objetivo:** Verificadores HIR/MIR con contrato comprobable, integrados como gate donde corresponda, sin falsos negativos/positivos ocultos.
- **Alcance permitido:** `hir.rs` (`VerificadorHir`), `mir.rs` (`VerificadorMir`), pipeline de invocación, LSP/CLI según arquitectura, tests unitarios/integración.
- **Dependencias:** Fase 00–02 `APROBADAS`.
- **Tareas:** Definir/reforzar invariantes reales (tipado, IDs/bloques/terminadores, CFG/alcanzabilidad, retornos, try/invoke, coherencia), implementar comprobaciones efectivas, integrar gates en pipeline, añadir tests positivos/negativos e integración.
- **Archivos afectados (ejecutado 3A–3D.6):** `prototipo/src/{hir.rs,mir.rs,cli.rs,lsp.rs}`, `prototipo/tests/{gate_3c_integration.rs,pipeline_3d5_integration.rs}` (+ pruebas existentes asociadas), `DEVELOPMENT_STATE.md`, `ROADMAP_MASTER.md`.
- **Pruebas requeridas:** tests de verificadores/gates/pipeline; `fmt --check`; `clippy -D warnings`; `cargo test --workspace --no-fail-fast`.
- **Criterios de aceptación:** verificadores detectan inválidos diseñados y aceptan válidos; gates integrados y sin bypass/fallback silencioso de verificación; comportamiento documentado y probado; sin regresiones en workspace.
- **Evidencias:** ver `DEVELOPMENT_STATE.md` (3A–3D.6), logs `/tmp/opencode/{fmt,clippy,ws_fase03_cierre}.txt`, reporte integral de cierre. Validación final: fmt/clippy OK, workspace **965 passed, 0 failed, 2 ignored** (55 suites, exit 0); ignorados B-REG-001 externos/no aprobados. Limitaciones/deudas técnicas documentadas y mantenidas.
- **Estado:** `APROBADA` (autorización humana explícita, 2026-11-10, incl. 3D.4–3D.6). **Fase 04 NO iniciada** (pendiente auditoría/planificación y autorización).

## Fase 04 — Unificación semántica de motores
- **Objetivo:** Reducir divergencias de semántica entre TW, VM y nativo/WASM en las capacidades declaradas soportadas, con evidencia diferencial y clasificación explícita.
- **Alcance permitido:** `evaluador.rs`, `vm.rs`, `hir.rs`, `mir.rs`, `backend_nativo.rs`, `runtime.rs`, `paridad.rs`, diferenciales, documentación de fase; acotado a brechas identificadas y subconjuntos declarados.
- **Dependencias:** Fases 00–03 `APROBADAS`.
- **Tareas realizadas/verificadas (local, en progreso):** contrato y harness diferencial TW↔VM (04.1–04.3), correcciones/mapeos de paridad (04.4/04.6, ref. TW), contrato y matriz multi-backend TW/VM/Nativo/WASM con clasificación EQUIVALENT/DIVERGENCE/UNSUPPORTED/UNVERIFIABLE (04.5), ampliación de evidencia/casos y revisión de GAP-051/registry/docs (MISIÓN 03, expediente en preparación).
- **Archivos afectados (principales):** `prototipo/src/{paridad,vm,mir,runtime,backend_nativo,lib}.rs`, `prototipo/tests/{diferencial_vm,diferencial_multibackend,optimizador}.rs`, `docs/semantica/CONTRATO_SEMANTICO.md`, `docs/reports/FASE_04_*`, `docs/AUDITORIA_FASE04_PARIDAD_MOTORES.md`, `DEVELOPMENT_STATE.md`.
- **Pruebas requeridas:** diferenciales/paridad relevantes y multi-backend; `fmt`/`clippy`/`cargo test --workspace --no-fail-fast`.
- **Criterios de aceptación:** features en alcance con comportamiento equivalente demostrado o exclusiones explícitas documentadas/probadas; sin afirmar paridad fuera de subconjuntos soportados; evidencia reproducible. **Cierre formal pendiente de revisión/autorización humana.**
- **Evidencias (históricas/vigentes):** `diferencial_vm` 118/118, `diferencial_multibackend` 11/11 y suites específicas en verde en el hito de cierre; validación integral `fmt` OK, `clippy -D warnings` OK, `cargo test --workspace --no-fail-fast` **1024 passed, 0 failed, 2 ignored** (56 suites). Expediente: `docs/reports/FASE_04_EXPEDIENTE_CIERRE.md`. Formalización documental autorizada posterior: **GAP-051 CERRADO en alcance cubierto** (observaciones técnicas preservadas: MIR sin nombres de parámetros/índice 1-based, diferencias diagnósticas Lista/Arreglo, cobertura no exhaustiva, whitelist acotada justificada/restringida; sin equivalencia total) y **B-REG-001 CERRADO (impl./ejecución local)** con reejecución CI externa pendiente por billing (no atribuible a código/workflow), `#[ignore]` preservados, validación local diferenciada de CI.
- **Estado:** `APROBADA` (autorización humana explícita 2026-11-14, con limitaciones documentadas; formalización documental de GAP-051/B-REG-001 autorizada posteriormente, sin cerrar Fase 06 ni otras fases). Limitaciones aceptadas: módulos TW↔VM no comparables en harness directo (VM sin cargador file-based), stdout homogéneo in-process TW↔VM, mutación de campos de objeto no expresable, `Invoke` MIR indirecto, subconjuntos nativo/WASM, árbol local no commiteado. **Fase 05 no iniciada.** (nota histórica del hito)

## Fase 05 — Backend nativo y funciones de primera clase
- **Objetivo:** Definir e implementar (o delimitar con rechazo claro y probado) soporte nativo para closures/celdas/funciones de primera clase y brechas críticas relacionadas.
- **Alcance permitido:** `backend_nativo.rs`, `runtime.rs`, `mir.rs/hir.rs` si requerido, tests nativos.
- **Dependencias:** Fase 04 `APROBADA`.
- **Tareas:** Diseño de representación (closures/capturas/celdas) o decisión explícita; implementación con paridad o diagnósticos consistentes; resolver riesgos conocidos (p. ej. doble inline) si en alcance.
- **Archivos afectados previstos:** backend/runtime/lowering, tests.
- **Pruebas requeridas:** casos closure/lambda/indirecta en nativo vs VM; `workspace`.
- **Criterios de aceptación:** soporte demostrado con paridad o no-soporte explícito, estable y probado (sin comportamiento indefinido silencioso).
- **Evidencias:** `prototipo/src/{runtime.rs,backend_nativo.rs}`, `prototipo/tests/backend_nativo.rs`; `docs/reports/FASE_05_BACKEND_NATIVO_FUNCIONES_PRIMERA_CLASE.md`. 12 pruebas de paridad nativa↔VM (closures/celdas/funciones de primera clase, incl. `para cada`) y 2 rechazos explícitos (try/catch, rango). Validación (MISIÓN 05): fmt/clippy OK, `backend_nativo` 59/0/0, workspace **1038/0/2** (56 suites). Ignorados: 2 `registry_integration` (`#[ignore]`, B-REG-001). Límites propios documentados (sin try/invoke/rangos, stdlib/HOF acotada, sin límites de pasos/profundidad equivalentes a VM, errores runtime textuales). Estado heredado actualizado por formalización autorizada: GAP-051 **CERRADO en alcance cubierto** (observaciones preservadas, sin equivalencia total); B-REG-001 **CERRADO (impl./ejecución local)** con reejecución CI externa pendiente por billing.
- **Estado:** `APROBADA` (autorización humana explícita 2026-11-14, con limitaciones documentadas). Fase 06 **EN PROGRESO** (no cerrada) según estado vivo actual.

## Fase 06 — Sistema de tipos avanzado
- **Objetivo:** Fortalecer tipado (inferencia útil, compatibilidad/unificación coherente, reducción de `Desconocido` crítico, chequeos estáticos efectivos).
- **Alcance permitido:** `semantico.rs`, `parser/ast` si necesario, `hir/mir`, verificadores, tests.
- **Dependencias:** Fases 00–05 `APROBADAS`.
- **Tareas:** Reglas tipadas con especificación/evidencia; mejorar diagnósticos; tests positivos/negativos (ejecución incremental por etapas/lotes según planificación autorizada). Etapa B completada/cerrada. Etapa C completada/cerrada (C.0 diseño aprobado; C.1 firmas/valores-función; C.2 inferencia local conservadora de retornos por identidad de declaración, canal separado de C.1, D0/D3–D6, sin inferencia interprocedimental ni ampliación a lambdas/closures, sin cambios en runtime/`tipos_parametros_inferidos`/diagnósticos/`run`/VM/backends/HIR/MIR), con evidencia y aprobación humana formal (MISIÓN 06.18–06.21). Etapa D completada/cerrada (D.1–D.4, investigación de viabilidad/riesgos, cierre formal autorizado por el usuario el 2026-11-16; sin implementación de mejoras avanzadas). D.3.1-A permanece únicamente candidata futura, sin implementación autorizada. **Cierre formal de Fase 06 autorizado (2026-11-17)** con fundamento en auditoría **APTO PARA CIERRE FORMAL** (CA-1/CA-2) y evidencias verificadas; sin alterar objetivos/criterios originales ni alcance de fases futuras.
- **Archivos afectados previstos:** semántico, lowering, verificadores, tests.
- **Pruebas requeridas:** suite de tipos/conversiones/inferencia; `workspace`.
- **Criterios de aceptación:** reglas implementadas comprobadas; regresiones cero.
- **Evidencias (Etapa B, conservadas):**
  - **B.1 (CLOSED, 2026-11-15):** `prototipo/src/semantico.rs`, `prototipo/tests/semantico.rs` (inferencia de elemento en `para cada` e indexación para `Arreglo/Lista/Coleccion`; 11 pruebas nuevas). Validación (informe B.1, `main@0e41fb2`, árbol local): `cargo fmt --check` exit 0; `cargo clippy --all-targets --all-features -- -D warnings` exit 0; `cargo test --test semantico` **70 passed, 0 failed, 0 ignored**; `cargo test --workspace --no-fail-fast` **1049 passed, 0 failed, 2 ignored** (56 suites).
  - **B.2 (CLOSED, 2026-11-15, MISIÓN 06.3):** inferencia local de colecciones y refinamiento de variables inicialmente `Desconocido` (evidencia MISIÓN 06.2, `prototipo/src/semantico.rs`, `prototipo/tests/semantico.rs`).
  - **B.3 (CLOSED, 2026-11-15, MISIÓN 06.3):** validación estática de asignaciones por índice, conservadora ante `Desconocido` (evidencia MISIÓN 06.2).
  - **Validación conjunta B.2+B.3 (MISIÓN 06.2, `main@0e41fb2`, árbol local):** 20 pruebas nuevas; `cargo fmt --check` **PASS**; `cargo clippy --all-targets --all-features -- -D warnings` **PASS**; `cargo test --test semantico` **90 passed**; `cargo test --test conformidad_tipos` **7 passed**; `cargo test --test inferencia_tipos` **18 passed**; `cargo test --workspace --no-fail-fast` **1069 passed, 0 failed, 2 ignored** (56 suites). Ignoradas: 2 `registry_integration` (B-REG-001, PostgreSQL aislado).
- **Evidencias (Etapa C):**
  - **C.0 (CLOSED, MISIÓN 06.6, ADR-0018):** cierre formal de diseño (Opción A conservadora) aprobado.
  - **C.1 (CLOSED, aprobado/cerrado formalmente posterior a MISIÓN 06.8):** unificación semántica conservadora de firmas de valores-función (commit `229feb6`, `feat(semantico): etapa B y C.1 de tipos en análisis estático`, publicado en `origin/main`; `prototipo/src/semantico.rs`, `prototipo/tests/semantico.rs`). Validación: `cargo fmt --check` **PASS**; Clippy `-D warnings` **PASS**; `--test semantico` **109 passed**; `--test conformidad_tipos` **7 passed**; `--test inferencia_tipos` **18 passed**; `--test lambdas` **6 passed**; `--test closures` **5 passed**; `cargo test --workspace --no-fail-fast` **1088 passed, 0 failed, 2 ignored** (56 suites). Ignorados: 2 `registry_integration` (B-REG-001, PostgreSQL aislado).
  - **C.2 (CLOSED, aprobado/cerrado formalmente 2026-11-16, MISIÓN 06.18–06.21, IMPLEMENTED/VERIFIED):** inferencia estática local conservadora de retornos (Opción A, D0/D3–D6) en `prototipo/src/semantico.rs`, con canal separado de C.1, identidad por declaración (`SimbodoId`), aislamiento de homónimos/ámbitos y consulta interna `consultar_c2_retorno` (ausente/desconocido/conocido), sin consumidor productivo artificial ni cambios en C.1, diagnósticos/E1010, `run`/runtime/VM/backends/HIR/MIR/parser/AST/CLI/LSP. Validación (06.21): `cargo fmt --check` **PASS**; `cargo clippy --workspace --all-targets --all-features -- -D warnings` **PASS**; `cargo test --lib` **240 passed, 0 failed, 0 ignored**; `--test semantico` **109 passed**; `--test conformidad_tipos` **7 passed**; `--test inferencia_tipos` **18 passed**; `--test lambdas` **6 passed**; `--test closures` **5 passed**; `--test conformidad_funciones` **23 passed**; `--test conformidad_funciones_extra` **8 passed**; `cargo test --workspace --no-fail-fast` **PASS**. Ignorados: 2 `registry_integration` (B-REG-001, PostgreSQL aislado).
- **Limitaciones documentadas (conservadas):** sin unificación general/análisis interprocedimental completo; C.2 implementada en alcance local conservador (sin inferencia interprocedimental de cuerpos/punto fijo, sin consumidores productivos adicionales, sin ampliación a lambdas/closures); colecciones vacías/heterogéneas/con elementos desconocidos conservadoras; sin modificar runtime/VM/backends/HIR/MIR por B/C. **GAP-051 CERRADO en alcance cubierto** (observaciones técnicas preservadas: MIR sin nombres de parámetros/índice 1-based, diferencias diagnósticas Lista/Arreglo, cobertura no exhaustiva, whitelist acotada justificada/restringida; sin equivalencia total). **B-REG-001 CERRADO (impl./ejecución local)** con observación externa pendiente: reejecución CI no iniciada por billing/límite de gasto (no atribuible a código/workflow), `#[ignore]` preservados, validación local diferenciada de CI.
- **Estado:** `APROBADA / CLOSED` (cierre formal autorizado 2026-11-17, con limitaciones/observaciones documentadas). Fundamento: auditoría de cierre **APTO PARA CIERRE FORMAL** frente a criterios originales (**CA-1** reglas implementadas comprobadas; **CA-2** regresiones cero) con evidencias verificadas de Etapas B/C/D, GAP-051 (alcance cubierto) y B-REG-001 (impl./ejecución local; CI externa pendiente por billing, no atribuible a código/workflow). **Etapa B = COMPLETADA / CLOSED** (**B.1/B.2/B.3**). **Etapa C = COMPLETADA / CLOSED** (**C.0/C.1/C.2**). **Etapa D = COMPLETADA / CLOSED** (D.1–D.4). D.3.1-A permanece candidata futura, sin implementación autorizada. Fase 07: **APROBADA / CLOSED** (2026-11-21; G2.1–G6 CLOSED; commit `bb1bfedefeec6a1bc3eef2b93336abe36a25bfc4`).

## Fase 07 — Genéricos y monomorfización
- **Objetivo:** Soporte real de genéricos **de usuario** con estrategia de implementación (monomorfización u otra) demostrada end-to-end.
- **Alcance permitido:** lexer/parser/AST, semántico, HIR/MIR, VM, nativo/WASM (según diseño), tests, spec.
- **Dependencias:** Fase 06 `APROBADA/CLOSED` (satisfecha).
- **Tareas:** Diseño/spec; implementación sintaxis+tipado+lowering+ejecución/backends en alcance; `lista<T>` builtin no cuenta como genéricos de usuario. Completado en subetapas G2.1–G6 dentro del alcance aprobado.
- **Archivos afectados previstos:** pipeline completo + tests + docs de spec.
- **Pruebas requeridas:** genéricos de usuario en casos reales por motor soportado; `workspace`.
- **Criterios de aceptación:** programas con genéricos de usuario compilan/ejecutan con evidencia en el alcance aprobado; sin regresión. Satisfechos con evidencia G6.
- **Evidencias:** G2.1 23/23, G2.2 20/20, G3 55/55, G4 33/33, diferencial TW/VM 145/145, workspace 1284 passed / 0 failed / 2 ignored; fmt/clippy/diff-check OK. Commit: `bb1bfedefeec6a1bc3eef2b93336abe36a25bfc4` (`origin/main`, 19 archivos).
- **Cierre humano:** 2026-11-21. **G2.1, G2.2, G3, G4, G5, G6: CLOSED.**
- **Estado:** `APROBADA` (cierre humano 2026-11-21, publicación oficial verificada; limitaciones G4 v1 y nativo/WASM documentadas).

### G2.1 — Frontend de genéricos (CLOSED, 2026-11-18)
- **Autorización:** cierre formal autorizado por el usuario; incluye correcciones **G2.1.1** incorporadas al cierre. **Auditoría final:** **APTO PARA CIERRE FORMAL**.
- **Alcance completado:**
  - Parámetros genéricos en declaraciones de funciones.
  - Múltiples parámetros de tipo.
  - Referencias sintácticas a parámetros declarados.
  - Tipos compuestos con parámetros genéricos.
  - Turbofish en llamadas directas por identificador.
  - Formateador actualizado.
  - Catálogo BAS-7001–7005 reservado y alineado.
  - Suite específica de 23 pruebas (`genericos_frontend_g2`).
  - Correcciones de los hallazgos de auditoría G2.1.1.
- **Evidencias de validación (documentadas):** `cargo fmt --check` **PASS**; `cargo clippy --all-targets --all-features -- -D warnings` **PASS**; `genericos_frontend_g2` **23/23**; `conformidad_tipos` **7/7**; `conformidad_arreglos_listas` **21/21**; `cargo test --workspace --no-fail-fast` **PASS, 0 fallos**; `git diff --check` **PASS**. El workspace conserva **2 pruebas de integración ignoradas preexistentes**.
- **Limitaciones y deuda preservada (no defectos del alcance cerrado; no capacidades implementadas):** sin inferencia de tipos genéricos; sin `TipoSem::Param`; sin sustitución ni instanciación; sin monomorfización; sin integración funcional en HIR/MIR/VM/backends; lambdas genéricas no soportadas; turbofish restringido a llamadas directas por identificador; reglas finas de visibilidad y sombreado en funciones genéricas anidadas pendientes de definición.

### G2.2 — Fundación semántica de genéricos (CLOSED, 2026-11-20, con observaciones)
- **Autorización:** cierre formal autorizado expresamente por el usuario (2026-11-20), con observaciones. **Implementación:** IMPLEMENTED y VERIFIED. **Auditoría independiente G2.2.1:** APTO CON OBSERVACIONES.
- **Alcance completado (conforme a especificación normativa G2.2/G3/G4 v1.0, 2026-11-18):** `TipoParamId`/identidad por declaración-propietario (cualificación por módulo compatible), `TipoSem::Param(TipoParamId)` ≠ `Desconocido`, registro y firmas genéricas (`FuncionSig.parametros_tipo`, metadatos/`TipoSem::Funcion.genericos`), resolución contextual de `TipoAnotacion::Parametro`, validación de declaraciones/ámbitos por declaración sin herencia implícita (§3.3), validación de turbofish (existencia/genericidad, forma/aridad exacta/resolubilidad/ligadura en contexto), rechazo temporal de inferencia no disponible en llamadas genéricas sin args de tipo explícitos, forwarding explícito con preservación de identidad, opacidad de `Param` sin capacidades implícitas, deferral conservador sin sustitución/compatibilidad completa post-sustitución en llamadas explícitas válidas; adaptaciones downstream estrictamente de compatibilidad estructural. Diagnósticos BAS-7001–7005 preservados; BAS-7006–7009 asignados/documentados; BAS-7010–7015 reservados. Suite específica `genericos_semantica_g2_2` (20 pruebas).
- **Evidencias de validación (documentadas):** `cargo fmt --check` PASS; `cargo clippy --all-targets --all-features -- -D warnings` PASS; `genericos_frontend_g2` 23/23; `genericos_semantica_g2_2` 20/20; `conformidad_tipos` 7/7; `conformidad_arreglos_listas` 21/21; `cargo test --workspace --no-fail-fast` PASS, 0 fallos; `git diff --check` PASS (workspace conserva 2 pruebas de integración ignoradas preexistentes).
- **Observaciones de auditoría preservadas (no corregidas, no reclasificadas):**
  - **MEDIO:** cobertura de pruebas insuficiente para subcontratos específicos: BAS-7009/E1044 semántico no ejercitado explícitamente; deferral ante incompatibilidad ordinaria en llamada genérica explícita válida no demostrado con caso incompatible; demostraciones adicionales de identidad/opacidad limitadas (lógica existente). Ref.: auditoría independiente G2.2.1 §4, §7, §8.
  - **BAJO:** endurecimiento de resolución de parámetros de tipo en parser al ámbito propio de cada función (alineado a G2.2 §3.3, sin regresión observada en contratos G2.1 cerrados). Ref.: auditoría G2.2.1 §3, §8.
  - **BAJO:** `tipo_anotacion_a_sem` (context-free) conserva mapeo conservador Param→Desconocido. Ref.: auditoría G2.2.1 §8.
- **Exclusiones preservadas:** sin inferencia, sin sustitución estructural, sin compatibilidad completa post-sustitución, sin instanciación/monomorfización, sin worklist/`InstanceKey`/mangling, sin integración funcional de genéricos en HIR/MIR, sin ejecución genérica especializada en Tree-Walk/VM/nativo/WASM; sin soporte inter-módulo parcial/ambiguo. G3/G4 no implementados.

### G3 — Inferencia y resolución semántica de genéricos (CLOSED, 2026-11-20)
- **Autorización:** cierre formal aprobado por decisión humana (acta de cierre registrada, 2026-11-20). **Estados:** IMPLEMENTED aprobado, VERIFIED aprobado, CLOSED aprobado. G3 no requiere nuevas auditorías rutinarias sin evidencia nueva que justifique reabrirla.
- **Alcance completado:** inferencia local por unificación estructural de igualdad (σ única/completa, sin inferencia global/retorno/heurísticas), sustitución estructural por `TipoParamId` (total/determinista/idempotente), validaciones V1/V2/V3 (opacidad estricta, compatibilidad concreta tras sustitución vía vista semántica sin instancias ejecutables), forwarding genérico directo/múltiple/transitivo, recolección determinista de dependencias sin materialización, diagnósticos diferenciados (BAS-7001–7009 preservados), aislamiento del estado semántico durante V3.
- **Evidencias:** informe **G3-FINAL** aceptado (ubicación de archivo no verificada en el repositorio; referencia conservada sin invención de ruta), con verificación de suites específicas y `cargo test --workspace --no-fail-fast` satisfactoria (2 pruebas `registry_integration` ignoradas por requerir PostgreSQL aislado, limitación conocida, no fallo de G3).
- **Ratificaciones normativas:** **BAS-7011/E1046** (`InferenciaArgumentosTipoAmbigua`) reservado sin emisión en G3; **BAS-7016/E1051** (`ForwardingGenericoInvalido`) reservado sin emisión en G3; **BAS-7017/E1052** (`DependenciaGenericaNoResoluble`) reservado para etapa posterior, fuera de G3. Trazabilidad contractual G2.2 → G3 ratificada según evidencia disponible.
- **Limitaciones/exclusiones preservadas:** limitaciones documentales de G3-FINAL conservadas como observaciones históricas. Sin monomorfización, instanciación/clonado/especialización ejecutable, worklist/`InstanceKey`/mangling, HIR/MIR/gate/codegen/ejecución especializada; sin responsabilidades G4. G2.2 preservada.

### G4 — Monomorfización (CLOSED, 2026-11-21)
- **Autorización/cierre:** humano 2026-11-21. **Alcance:** Arquitectura C, identidad canónica (`GenericFnId`, `InstanceKey`, `SpecializedName`), dedup/cache/worklist transaccional, cierre transitivo, sustitución estructural total, V3 post-sustitución, gate integridad salida, reescritura, gates HIR/MIR, integración CLI/TW/VM, límites M1–M3 (provisionales). **Evidencia:** G4 33/33, diferencial 145/145, workspace 1284/0/2. **Limitaciones aceptadas:** G4 v1 mismo-módulo; sin traits/bounds/genéricos structs-ADTs-lambdas/inferencia avanzada-retorno/resolución dinámica/instancias parciales/especialización por motor/inter-módulo completa/soporte universal nativo-WASM.

### G5 — Integración de motores y paridad (CLOSED, 2026-11-21)
- **Autorización/cierre:** humano 2026-11-21. **Alcance:** TW↔VM sobre misma monomorfización canónica, sin resolución genérica independiente por motor, paridad resultados/errores en subconjunto soportado; corrección CLI TW auto-main; rechazo coherente BAS-7020/E1055 para genérica usada como valor. **Evidencia:** 15 diff_g5 en diferencial_vm (145/145 total), workspace 1284/0/2. **Nativo/WASM:** compatibilidad parcial acotada, no obligatoria.

### G6 — Consolidación integral (CLOSED, 2026-11-21)
- **Dictamen técnico:** CONFORME PARA PROPUESTA DE CIERRE. **Cierre humano:** 2026-11-21. **Publicación:** `bb1bfedefeec6a1bc3eef2b93336abe36a25bfc4` en `origin/main` (19 archivos).

- **Estado:** `APROBADA` (cierre humano 2026-11-21). G2.1–G6 CLOSED. Limitaciones G4 v1 y nativo/WASM documentadas; no se amplían requisitos de Fase 08.

## Fase 08 — Structs, enums, ADTs, traits y pattern matching
- **Objetivo:** Abstracciones de datos y comportamiento de usuario (tipos nominales/ADTs/traits/pattern matching) según spec aprobada.
- **Alcance permitido:** pipeline completo, runtime/backends, stdlib mínima asociada, tests.
- **Dependencias:** Fase 07 `APROBADA` (y 06).
- **Tareas:** Diseño incremental; implementación y verificación por constructo (S8.1–S8.5 según plan aprobado).
- **Archivos afectados previstos:** parser/semántico/HIR/MIR/motores/backends/tests.
- **Pruebas requeridas:** conformidad por constructo y paridad donde aplique.
- **Criterios de aceptación:** constructos especificados funcionan con evidencia; límites declarados.
- **Evidencias:** spec+tests+ejecución.
- **Estado:** `APROBADA / CLOSED` (aprobación humana 2026-11-23).
  - **S8.0 (spec/contrato):** APROBADA POR REVISIÓN HUMANA (2026-11-22). R3: spec normativa, D8-01..D8-14, matrices capacidades/aceptación, diagnósticos BAS-71xx, plan S8.1–S8.5; C1–C3.
  - **S8.1 (structs nominales):** APROBADA (2026-11-22). Evidencia: structs nominales C2, C3 módulos/visibilidad, TipoNominalId coherente semántica/HIR/MIR/TW/VM, verifiers nominales, BAS-71xx/BAS-7124, 24 tests S8.1 PASS.
  - **S8.2 (enums/ADTs nominales):** APROBADA (2026-11-23, cierre formal Fase 08). Evidencia: enums/variantes/payload posicional, BAS-7108..7111, TW↔VM, 11 tests S8.2 PASS.
  - **S8.3 (pattern matching/exhaustividad):** APROBADA (2026-11-23, cierre formal Fase 08). Evidencia: `coincidir` v1, patrones/binding/exhaustividad/unreachable, BAS-7112..7116, HIR/MIR/TW↔VM, 11 tests S8.3 PASS.
  - **S8.4 (traits/métodos):** APROBADA (2026-11-23, cierre formal Fase 08). Evidencia: `rasgo`/`implementar`/métodos inherentes+trait, resolución estática, coherencia/visibilidad, BAS-7117..7122/7128, HIR/MIR/TW↔VM, 18 tests S8.4 PASS.
  - **S8.5 (integración/validación):** COMPLETADA (2026-11-23, cierre formal Fase 08). Evidencia: fmt/clippy/git diff --check PASS; HIR 13/13, MIR 3/3; G2 23/23, G2.2 20/20, G3 55/55, G4 33/33 (`RUST_MIN_STACK=16777216`), modulos_e2e 30/30; workspace `RUST_MIN_STACK=16777216 cargo test --workspace --no-fail-fast` PASS salvo 2 fallos preexistentes `diferencial_vm` (`diff_closure_mutable`, `diff_closure_mutable_captura`, 143/145, TW Ok Entero(2), VM TipoParametroIncompatible esperado entero obtenido celda), no atribuibles a Fase 08.
  - **Observaciones independientes preservadas:** 2 fallos conocidos closures mutables en `diferencial_vm`; sensibilidad stack G4 pasa con `RUST_MIN_STACK=16777216`; limitaciones nominales Nativo/WASM rechazadas mediante BAS-7124.

## Fase 09 — Biblioteca estándar
- **Objetivo:** Stdlib coherente, inventariada y con paridad declarada entre motores para dominios priorizados.
- **Alcance permitido:** stdlib VM/TW, `runtime.rs`/nativo, WASM subset, docs, tests.
- **Dependencias:** Fases 04–06 `APROBADAS/CLOSED` (estabilidad semántica/motores verificada).
- **Tareas:** Inventario real por función/dominio/motor; implementar/brechas priorizadas; documentar disponibilidad.
- **Archivos afectados previstos:** `vm.rs`, `evaluador.rs`, `runtime.rs`, `backend_nativo.rs`, tests, `docs/stdlib.md`.
- **Pruebas requeridas:** tests por dominio/función y paridad declarada.
- **Criterios de aceptación:** matriz stdlib verificada con evidencia; sin afirmaciones no comprobadas.
- **Evidencias:** Inventario stdlib y matriz de disponibilidad TW/VM/Nativo/WASM en `docs/stdlib.md`. Correcciones: `piso/techo/redondear` con Decimal en aritmética entera escalada (sin `f64`) y Real con guardas IEEE-754 explícitas (`[-2^63, 2^63)`, NaN/∞ rechazados, sin conversión saturada silenciosa); contratos documentados de `piso`, `techo`, `redondear` (half away from zero) y `posicion` (índices Unicode por caracteres, subcadena vacía=0, ausente=-1); `texto` disponible en TW/VM. Pruebas: 12 específicas stdlib PASS (`prototipo/tests/stdlib_fase09.rs`), 153 diferenciales TW↔VM PASS (`diferencial_vm`). Workspace completo: 1412 passed, 0 failed, 2 ignored (`RUST_MIN_STACK=16777216 cargo test --workspace --no-fail-fast`). `cargo fmt --check` PASS, `cargo clippy --workspace --all-targets --all-features -- -D warnings` PASS, `git diff --check` PASS.
- **Limitaciones residuales:** Nativo/WASM sin paridad stdlib total (LIM/NO según matriz, sin ampliación artificial); `Real` sin exactitud decimal fuera de precisión entera exacta IEEE-754; cobertura stdlib no exhaustiva formal.
- **Estado:** `APROBADA / CLOSED` (autorización humana explícita, 2026-11-24).

## Fase 10 — Concurrencia robusta
- **Objetivo:** Modelo de concurrencia fiable (más allá de primitiva experimental): corrección, manejo de errores, y garantías definidas (cancelación/timeout si en alcance).
- **Alcance permitido:** runtime/VM/TW/nativo (según diseño), tests de concurrencia/determinismo/estrés razonable.
- **Dependencias:** Fase 02 `APROBADA`; Fases 04–05 recomendadas.
- **Tareas:** Especificar garantías; implementar; pruebas deterministas y de carga acotada.
- **Archivos afectados previstos:** `vm.rs`, `evaluador.rs`, `runtime/backends`, tests.
- **Pruebas requeridas:** determinismo (repeticiones), propagación de errores, escenarios prod/cons; `workspace`.
- **Criterios de aceptación:** garantías documentadas y probadas; 0 flakiness conocida en alcance.
- **Evidencias:** 20 pruebas específicas Fase 10 PASS (`prototipo/tests/concurrencia_fase10.rs`); 153 pruebas diferenciales TW↔VM PASS (`diferencial_vm`); pruebas de supervivencia de tareas activas al terminar main TW y VM PASS (`evaluador::tests::f10_tarea_sobrevive_fin_main_tw`, `..._vm`); repeticiones de determinismo satisfactorias (12/8/8/6 en suite Fase 10); workspace completo PASS (`RUST_MIN_STACK=16777216 cargo test --workspace --no-fail-fast`, sin FAILED); `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` PASS. Especificación normativa: `10-especificacion/concurrencia-fase-10.md`. Informe de evidencia: registrado en conversación de trabajo (Fase 10) y revisión focalizada.
- **Limitación aceptada:** No existe join global al terminar main. Las tareas bloqueadas o de larga duración pueden mantener hilos y recursos vivos hasta la terminación del proceso. No se implementan cancelación ni timeout en esta fase.
- **Estado:** `APROBADA / CLOSED` (autorización humana explícita, 2026-11-23).

## Fase 11 — FFI e interoperabilidad
- **Objetivo:** FFI verificada, segura en lo razonable y con matriz de tipos/ABI/plataformas soportadas.
- **Alcance permitido:** `ffi.rs`, integración VM/TW/nativo (si en alcance), tests e2e controlados, docs.
- **Dependencias:** Fases 04, 06 y 09 `APROBADAS/CLOSED` (cierre Fase 11 autorizado explícitamente por mantenedor humano con limitaciones documentadas).
- **Tareas:** Casos e2e reales (libs controladas), marshalling, errores, memoria; definir soporte por motor.
- **Archivos afectados previstos:** `ffi.rs`, motores, tests/fixtures.
- **Pruebas requeridas:** e2e FFI reproducible en entorno disponible; manejo de fallos.
- **Criterios de aceptación:** capacidades FFI demostradas con evidencia y límites explícitos.
- **Evidencias:** ABI C subconjunto implementado/verificado en Linux x86_64. TW: SUPPORTED (subconjunto). VM: SUPPORTED (subconjunto), paridad TW/VM verificada. Backend nativo: UNSUPPORTED, rechazo explícito BAS-7124. WASM: UNSUPPORTED, rechazo explícito BAS-7124. Pruebas FFI E2E 27 aprobadas (`prototipo/tests/ffi.rs`, fixture C real). Workspace completo PASS (`RUST_MIN_STACK=16777216 cargo test --workspace --no-fail-fast`, sin FAILED). `cargo fmt --check` PASS. `cargo clippy --workspace --all-targets --all-features -- -D warnings` PASS. `git diff --check` PASS. Artefactos: `10-especificacion/ffi-abi.md`, catálogos de diagnósticos, `README.md`.
- **Limitaciones aceptadas en el cierre (conservadas, no tareas obligatorias de reapertura):** sin callbacks, structs C, variádicas, punteros opacos ni buffers raw; sin sandbox para código nativo externo; sin validación automática de la firma C real; plataformas distintas de Linux x86_64 no verificadas; registro FFI global por proceso.
- **Estado:** `APROBADA / CLOSED` (autorización humana explícita del mantenedor, 2026-11-24).

## Fase 12 — Paquetes y registro remoto
- **Objetivo:** Gestor de paquetes confiable; integración de registro verificada cuando la infraestructura esté disponible.
- **Alcance permitido:** `paquetes.rs`, `proyecto.rs`, `cli` pkg, registro/servidor, tests (unitarias + integración con deps), formato BPK.
- **Dependencias:** Fase 00; coordinación con Fase 09 (si aplica).
- **Tareas:** Consolidar resolución/lock/integridad/firmas/namespaces; habilitar/verificar integración con PostgreSQL aislado/`psql`/servicio en entorno reproducible o mocks controlados (sin fingir aprobación).
- **Archivos afectados previstos:** paquetes/servidor/cli/tests/CI docs.
- **Pruebas requeridas:** `bpk`/paquetes locales; `registry_integration` solo con deps → si no, **BLOQUEADA** (no aprobada).
- **Criterios de aceptación:** flujo local verificado; flujo remoto verificado con evidencia **o** fase marcada BLOQUEADA en ese sub-criterio con causa externa (no APROBADA total si criterio obligatorio depende de ello).
- **Evidencias:** Autenticidad remota fail-closed en rutas reales (`resolver_dependencia_remota`, `instalar_paquete_remoto_completo`, CLI `instalar-remoto`, materialización desde `basilisco.lock`) y reproducibilidad de instalación desde lock (sin re-resolución/reescritura en instalación normal; `update` deliberado) verificadas por inspección y ejecución. Pruebas: paquetes 54 PASS, BPK 3 PASS, registry PostgreSQL aislado 2 PASS (`postgres:16-alpine`, desechable), workspace completo 1428 passed / 0 failed / 2 ignored (`RUST_MIN_STACK=16777216 cargo test --workspace --no-fail-fast`). `cargo fmt --check` PASS, `cargo clippy --workspace --all-targets --all-features -- -D warnings` PASS, `git diff --check` PASS. Verificación independiente: satisfactoria, sin bypasses bloqueantes demostrados.
- **Limitaciones técnicas residuales documentadas (no defectos bloqueantes):** R3 distribución/rotación/revocación avanzada de claves; R1 estados formales de verificación del lock; dependencia práctica de CWD; TOCTOU y atomicidad residual; cobertura E2E parcial del registro. No se afirma publicación en registro real ni garantías no probadas por implementación+pruebas.
- **Estado:** `APROBADA / CLOSED` (autorización humana explícita 2026-11-24).

## Fase 13 — LSP, DAP, REPL y herramientas
- **Objetivo:** Herramientas utilizables end-to-end (no solo módulos presentes): LSP, depuración real (o alcance definido), REPL/CLI coherentes.
- **Alcance permitido:** `lsp.rs`, `cli.rs`, formateador/linter si existen, soporte DAP/depurador, extensión VS Code, docs.
- **Dependencias:** Fases 01–06 (estabilidad), 09 parcial.
- **Tareas:** Pruebas de protocolo LSP (initialize, diagnostics, completion, hover, definition, refs, rename según soporte real); depurador: breakpoints/stepping/inspección o límites explícitos; verificación en editor si en alcance.
- **Archivos afectados previstos:** `lsp.rs`, CLI, tooling, tests, extensión.
- **Pruebas requeridas:** tests automatizados de protocolo/comandos; smoke e2e.
- **Criterios de aceptación:** capacidades declaradas demostradas; las no soportadas marcadas explícitamente.
- **Evidencias:** Expediente `docs/reports/FASE_13_EXPEDIENTE_CIERRE.md`. M6 (invocación única, Node v24.20.0 en PATH temporal): `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets --all-features -- -D warnings` PASS; `cargo test --workspace --no-fail-fast` PASS; `git diff --check` PASS; `npm test` (editors/vscode-basilisco) 23/23 PASS. Focales: lsp 18/18, cli 47/47, formateador 13/13, vm_debug_control 9/9. E2E A-F PASS; 5.1D A cross-module 9/9; B DAP ciclo real Extension Host.
- **Estado:** `APROBADA / CLOSED` (autorización humana explícita 2026-11-27; clasificación IMPLEMENTED + VERIFIED; dictamen `IMPLEMENTED + VERIFIED — READY FOR HUMAN REVIEW`). Limitaciones: `bsk publish` local BPK sin HTTP remoto; VS Code local 0.4.0 no publicada (publicada verificada 0.3.2); WASM/diferencial sensibles a Node en PATH; capacidades locales extensión no equivalen a soporte LSP contractual completo.

## Fase 14 — Auditoría integral y estabilización final
- **Objetivo:** Verificación integral de todas las fases aprobadas, regresión cero, documentación de estado real y límites.
- **Alcance permitido:** auditoría transversal, hardening, fixes de regresión dentro de fases ya aprobadas (con evidencia), informes finales.
- **Dependencias:** Fases 00–13 `APROBADAS` (o bloqueos formales documentados y aceptados por humano).
- **Tareas:** Re-ejecutar matriz completa; resolver regresiones; confirmar no falsos verdes/skips ocultos; publicar informe final de estado (sin cambiar versión del lenguaje si no autorizado).
- **Archivos afectados previstos:** los necesarios para regresiones justificadas + informes.
- **Pruebas requeridas:** `fmt`/`clippy`/`workspace` + suites dirigidas por fase; repetición de flaky targets.
- **Criterios de aceptación:** todas las fases previas con evidencia vigente; bloqueos externos explícitos; informe integral.
- **Evidencias:** informe final, logs, matriz.
- **Estado:** `PENDIENTE`.
