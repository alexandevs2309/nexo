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
- **Evidencias:** comandos, salidas, archivos `.wasm` generados en contexto de prueba, conteos.
- **Estado:** `PENDIENTE`.

## Fase 02 — Corrección de concurrencia y canales
- **Objetivo:** Eliminar no determinismo/races en canales/tareas (VM y donde aplique) y obtener pruebas deterministas y fiables.
- **Alcance permitido:** `prototipo/src/vm.rs`, `prototipo/src/evaluador.rs` (canales/TW), `prototipo/src/mir.rs/hir.rs` solo si necesario al fix, `tests/diferencial_vm.rs` u tests de concurrencia; sin nuevas features de concurrencia.
- **Dependencias:** Fase 00 `APROBADA`; idealmente tras Fase 01 solo por orden, pero dependencia técnica directa: baseline.
- **Tareas:** Reproducir flakiness (evidencia previa: divergencias TW/VM canal 42/62 vs 10/30); aislar estado global/IDs/colas por contexto de ejecución o prueba; corregir causa raíz; hacer diferenciales canal/tarea deterministas; re-ejecutar repetidamente para evidencia de estabilidad.
- **Archivos afectados previstos:** `vm.rs`, `evaluador.rs`, tests diferenciales/concurrencia.
- **Pruebas requeridas:** `cargo test --test diferencial_vm canal tarea` (y repeticiones); `workspace` completo; sin regresión de paridad.
- **Criterios de aceptación:** pruebas de canal/tarea deterministas (N repeticiones documentadas, 0 fallos); causa raíz explicada con evidencia.
- **Evidencias:** logs de repeticiones, diff antes/después, archivos/funciones.
- **Estado:** `PENDIENTE`.

## Fase 03 — Validadores semánticos HIR/MIR
- **Objetivo:** Verificadores HIR/MIR con contrato comprobable, integrados como gate donde corresponda, sin falsos negativos/positivos ocultos.
- **Alcance permitido:** `hir.rs` (`VerificadorHir`), `mir.rs` (`VerificadorMir`), pipeline de invocación, tests unitarios/integración de verificadores.
- **Dependencias:** Fase 00 `APROBADA` (recomendado tras 01-02 para baseline estable).
- **Tareas:** Definir invariantes reales (tipos de retorno, `LocalId`, coherencia de IDs/bloques/terminadores, etc.); implementar comprobaciones efectivas; añadir tests positivos/negativos; decidir y evidenciar integración en pipeline.
- **Archivos afectados previstos:** `hir.rs`, `mir.rs`, `cli.rs`/pipeline, `tests/`.
- **Pruebas requeridas:** tests nuevos/existentes de verificadores; `workspace` verde.
- **Criterios de aceptación:** verificadores detectan casos inválidos diseñados y aceptan válidos; comportamiento de gate documentado y probado.
- **Evidencias:** tests, comandos, ejemplos de rechazo.
- **Estado:** `PENDIENTE`.

## Fase 04 — Unificación semántica de motores
- **Objetivo:** Reducir divergencias de semántica entre TW, VM y nativo en las capacidades declaradas soportadas.
- **Alcance permitido:** `evaluador.rs`, `vm.rs`, `hir.rs`, `mir.rs`, `backend_nativo.rs`, `runtime.rs`, diferenciales; acotado a brechas identificadas en Fase 00.
- **Dependencias:** Fases 00–03 `APROBADAS` (según hallazgos).
- **Tareas:** Matriz de paridad por feature soportada; corregir divergencias con evidencia; ampliar diferenciales TW↔VM↔nativo.
- **Archivos afectados previstos:** motores, lowering, runtime, `tests/diferencial_*`, `tests/backend_nativo.rs`.
- **Pruebas requeridas:** diferenciales/paridad relevantes; `workspace`.
- **Criterios de aceptación:** features en alcance tienen comportamiento equivalente demostrado o exclusiones explícitas documentadas/probadas.
- **Evidencias:** matriz, tests, comandos.
- **Estado:** `PENDIENTE`.

## Fase 05 — Backend nativo y funciones de primera clase
- **Objetivo:** Definir e implementar (o delimitar con rechazo claro y probado) soporte nativo para closures/celdas/funciones de primera clase y brechas críticas relacionadas.
- **Alcance permitido:** `backend_nativo.rs`, `runtime.rs`, `mir.rs/hir.rs` si requerido, tests nativos.
- **Dependencias:** Fase 04 `APROBADA`.
- **Tareas:** Diseño de representación (closures/capturas/celdas) o decisión explícita; implementación con paridad o diagnósticos consistentes; resolver riesgos conocidos (p. ej. doble inline) si en alcance.
- **Archivos afectados previstos:** backend/runtime/lowering, tests.
- **Pruebas requeridas:** casos closure/lambda/indirecta en nativo vs VM; `workspace`.
- **Criterios de aceptación:** soporte demostrado con paridad o no-soporte explícito, estable y probado (sin comportamiento indefinido silencioso).
- **Evidencias:** tests, e2e, matriz.
- **Estado:** `PENDIENTE`.

## Fase 06 — Sistema de tipos avanzado
- **Objetivo:** Fortalecer tipado (inferencia útil, compatibilidad/unificación coherente, reducción de `Desconocido` crítico, chequeos estáticos efectivos).
- **Alcance permitido:** `semantico.rs`, `parser/ast` si necesario, `hir/mir`, verificadores, tests.
- **Dependencias:** Fases 03–04 `APROBADAS`.
- **Tareas:** Reglas tipadas con especificación/evidencia; mejorar diagnósticos; tests positivos/negativos.
- **Archivos afectados previstos:** semántico, lowering, verificadores, tests.
- **Pruebas requeridas:** suite de tipos/conversiones/inferencia; `workspace`.
- **Criterios de aceptación:** reglas implementadas comprobadas; regresiones cero.
- **Evidencias:** tests, comandos.
- **Estado:** `PENDIENTE`.

## Fase 07 — Genéricos y monomorfización
- **Objetivo:** Soporte real de genéricos **de usuario** con estrategia de implementación (monomorfización u otra) demostrada end-to-end.
- **Alcance permitido:** lexer/parser/AST, semántico, HIR/MIR, VM, nativo/WASM (según diseño), tests, spec.
- **Dependencias:** Fase 06 `APROBADA`.
- **Tareas:** Diseño/spec; implementación sintaxis+tipado+lowering+ejecución/backends en alcance; `lista<T>` builtin no cuenta como genéricos de usuario.
- **Archivos afectados previstos:** pipeline completo + tests + docs de spec.
- **Pruebas requeridas:** genéricos de usuario en casos reales por motor soportado; `workspace`.
- **Criterios de aceptación:** programas con genéricos de usuario compilan/ejecutan con evidencia; sin regresión.
- **Evidencias:** programas, tests, comandos.
- **Estado:** `PENDIENTE`.

## Fase 08 — Structs, enums, ADTs, traits y pattern matching
- **Objetivo:** Abstracciones de datos y comportamiento de usuario (tipos nominales/ADTs/traits/pattern matching) según spec aprobada.
- **Alcance permitido:** pipeline completo, runtime/backends, stdlib mínima asociada, tests.
- **Dependencias:** Fase 07 `APROBADA` (y 06).
- **Tareas:** Diseño incremental; implementación y verificación por constructo.
- **Archivos afectados previstos:** parser/semántico/HIR/MIR/motores/backends/tests.
- **Pruebas requeridas:** conformidad por constructo y paridad donde aplique.
- **Criterios de aceptación:** constructos especificados funcionan con evidencia; límites declarados.
- **Evidencias:** spec+tests+ejecución.
- **Estado:** `PENDIENTE`.

## Fase 09 — Biblioteca estándar
- **Objetivo:** Stdlib coherente, inventariada y con paridad declarada entre motores para dominios priorizados.
- **Alcance permitido:** stdlib VM/TW, `runtime.rs`/nativo, WASM subset, docs, tests.
- **Dependencias:** Fases 04–06 `APROBADAS` (mínimo estabilidad semántica/motores).
- **Tareas:** Inventario real por función/dominio/motor; implementar/brechas priorizadas; documentar disponibilidad.
- **Archivos afectados previstos:** `vm.rs`, `evaluador.rs`, `runtime.rs`, `backend_nativo.rs`, tests, `docs/stdlib.md`.
- **Pruebas requeridas:** tests por dominio/función y paridad declarada.
- **Criterios de aceptación:** matriz stdlib verificada con evidencia; sin afirmaciones no comprobadas.
- **Evidencias:** matriz, tests.
- **Estado:** `PENDIENTE`.

## Fase 10 — Concurrencia robusta
- **Objetivo:** Modelo de concurrencia fiable (más allá de primitiva experimental): corrección, manejo de errores, y garantías definidas (cancelación/timeout si en alcance).
- **Alcance permitido:** runtime/VM/TW/nativo (según diseño), tests de concurrencia/determinismo/estrés razonable.
- **Dependencias:** Fase 02 `APROBADA`; Fases 04–05 recomendadas.
- **Tareas:** Especificar garantías; implementar; pruebas deterministas y de carga acotada.
- **Archivos afectados previstos:** `vm.rs`, `evaluador.rs`, `runtime/backends`, tests.
- **Pruebas requeridas:** determinismo (repeticiones), propagación de errores, escenarios prod/cons; `workspace`.
- **Criterios de aceptación:** garantías documentadas y probadas; 0 flakiness conocida en alcance.
- **Evidencias:** tests repetidos, informes.
- **Estado:** `PENDIENTE`.

## Fase 11 — FFI e interoperabilidad
- **Objetivo:** FFI verificada, segura en lo razonable y con matriz de tipos/ABI/plataformas soportadas.
- **Alcance permitido:** `ffi.rs`, integración VM/TW/nativo (si en alcance), tests e2e controlados, docs.
- **Dependencias:** Fases 04, 06, 09 (parcial) `APROBADAS` según alcance.
- **Tareas:** Casos e2e reales (libs controladas), marshalling, errores, memoria; definir soporte por motor.
- **Archivos afectados previstos:** `ffi.rs`, motores, tests/fixtures.
- **Pruebas requeridas:** e2e FFI reproducible en entorno disponible; manejo de fallos.
- **Criterios de aceptación:** capacidades FFI demostradas con evidencia y límites explícitos.
- **Evidencias:** libs de prueba, comandos, resultados.
- **Estado:** `PENDIENTE`.

## Fase 12 — Paquetes y registro remoto
- **Objetivo:** Gestor de paquetes confiable; integración de registro verificada cuando la infraestructura esté disponible.
- **Alcance permitido:** `paquetes.rs`, `proyecto.rs`, `cli` pkg, registro/servidor, tests (unitarias + integración con deps), formato BPK.
- **Dependencias:** Fase 00; coordinación con Fase 09 (si aplica).
- **Tareas:** Consolidar resolución/lock/integridad/firmas/namespaces; habilitar/verificar integración con PostgreSQL aislado/`psql`/servicio en entorno reproducible o mocks controlados (sin fingir aprobación).
- **Archivos afectados previstos:** paquetes/servidor/cli/tests/CI docs.
- **Pruebas requeridas:** `bpk`/paquetes locales; `registry_integration` solo con deps → si no, **BLOQUEADA** (no aprobada).
- **Criterios de aceptación:** flujo local verificado; flujo remoto verificado con evidencia **o** fase marcada BLOQUEADA en ese sub-criterio con causa externa (no APROBADA total si criterio obligatorio depende de ello).
- **Evidencias:** tests, infra usada, logs.
- **Estado:** `PENDIENTE`.

## Fase 13 — LSP, DAP, REPL y herramientas
- **Objetivo:** Herramientas utilizables end-to-end (no solo módulos presentes): LSP, depuración real (o alcance definido), REPL/CLI coherentes.
- **Alcance permitido:** `lsp.rs`, `cli.rs`, formateador/linter si existen, soporte DAP/depurador, extensión VS Code, docs.
- **Dependencias:** Fases 01–06 (estabilidad), 09 parcial.
- **Tareas:** Pruebas de protocolo LSP (initialize, diagnostics, completion, hover, definition, refs, rename según soporte real); depurador: breakpoints/stepping/inspección o límites explícitos; verificación en editor si en alcance.
- **Archivos afectados previstos:** `lsp.rs`, CLI, tooling, tests, extensión.
- **Pruebas requeridas:** tests automatizados de protocolo/comandos; smoke e2e.
- **Criterios de aceptación:** capacidades declaradas demostradas; las no soportadas marcadas explícitamente.
- **Evidencias:** transcripciones/protocol logs, tests.
- **Estado:** `PENDIENTE`.

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
