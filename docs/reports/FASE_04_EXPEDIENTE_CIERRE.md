# Expediente único de cierre — Fase 04: Unificación semántica de motores

- **Fecha:** 2026-11-14
- **Misión:** MISIÓN 03 — Cierre técnico y consolidación de Fase 04 (preparación de cierre) + addendum de decisión humana
- **Rama:** `main` (trabajo local, sin commits/push/merge/tags/publicaciones)
- **Estado de fase:** **APROBADA** (autorización humana explícita 2026-11-14, opción 1) **con limitaciones documentadas**. **Fase 05 en progreso** (trabajo posterior conservado, no revertido). Este documento consolida la evidencia usada para la decisión y registra las brechas/limitaciones vigentes.

## 1. Alcance ejecutado y restricciones cumplidas

- Revisión de cambios locales 04.1–04.5 y 04.6 (contrato/harness, correcciones/mapeos, multi-backend, docs) preservando trabajo existente.
- Ampliación de evidencia diferencial y multi-backend dentro de lo expresable y de capacidades declaradas; investigación GAP-051; revisión de pruebas registry/CI; correcciones y alineación documental (`README`, `ROADMAP_MASTER`, `DEVELOPMENT_STATE`, reportes de fase).
- Restricciones respetadas: exclusivamente `main`, sin operaciones Git destructivas, sin commits/push/publicaciones, sin inventar equivalencias/capacidades/resultados, sin modificar componentes fuera de alcance, sin iniciar Fase 05.

## 2. Entregables revisados / consolidados

| Área | Entregable | Estado | Referencia |
|---|---|---|---|
| Contrato TW↔VM | Normalización, clasificación y mapeos explícitos (04.1/04.2/04.4/04.6) | Presente y verificado | `prototipo/src/paridad.rs`, `docs/semantica/CONTRATO_SEMANTICO.md` |
| Harness diferencial TW↔VM | Ejecución directa, comparación por contrato, batería 04.3 y casos dirigidos | Presente y verificado (118 pruebas) | `prototipo/tests/diferencial_vm.rs` |
| Correcciones/mapeos paridad | Cortocircuito, condiciones lógicas, inmutabilidad/errores, comparaciones, indexación, literales colección, mapeos de causa equivalentes | Presente y verificado | `prototipo/src/{vm,mir,runtime,backend_nativo,lib}.rs`, `docs/reports/FASE_04_4_6_CORRECCIONES_PARIDAD.md` |
| Contrato multi-backend 04.5 | Matriz de capacidades, clasificación `EQUIVALENT/DIVERGENCE/UNSUPPORTED/UNVERIFIABLE`, normalización de salida simple | Presente y verificado | `prototipo/src/paridad.rs`, `prototipo/tests/diferencial_multibackend.rs`, `docs/reports/FASE_04_5_CONTRATO_MULTIBACKEND.md` |
| Auditorías/reportes 04.x | Diagnóstico, consolidación evidencia, correcciones, multi-backend, auditoría integral | Presentes | `docs/AUDITORIA_FASE04_PARIDAD_MOTORES.md`, `docs/reports/FASE_04_3_*`, `docs/reports/FASE_04_4_6_*`, `docs/reports/AUDITORIA_INTEGRAL_BASILISCO.md` |
| Estado y gobernanza | Estado vivo y hoja de ruta alineados al estado real (sin cierre de fase) | Actualizados en MISIÓN 03 | `DEVELOPMENT_STATE.md`, `ROADMAP_MASTER.md`, `README.md` |

## 3. Cobertura alcanzada

### 3.1 TW↔VM (diferencial directo, contrato común)
- **Cobertura ejercitada:** aritmética/numéricos, control de flujo, funciones/recursión/closures/lambdas/HOF, colecciones/indexación/mutación y orden, texto/stdlib básica, comparaciones/igualdad/lógica con semántica alineada, try/rescatar y propagación de errores capturables, conversiones y asignaciones/retornos tipados (compatibles e incompatibles), efectos deterministas por estado/FS/HTTP local/FFI/fechas/tareas/canales según casos existentes, JSON, errores semánticos y mapeos de causa equivalentes declarados.
- **Ampliación MISIÓN 03:** validación determinista de stdout capturado en VM (`diff_mostrar_stdlib`, `diff_vm_stdout_mostrar_texto_determinista`) comparando valor semántico con TW (efecto stdout TW no homogéneo in-process), y evidencia adicional en rutas expresables try/rescatar (`diff_gap051_try_rescatar_invoke_ruta_expresable_equivalente`) y retornos/asignaciones tipadas incompatibles (`diff_gap051_retorno_tipado_rechaza_coleccion_incompatible`, `diff_gap051_retorno_entero_rechaza_texto_equivalente`, `diff_gap051_asignacion_texto_rechaza_lista_equivalente`).
- **Resultado:** `cargo test --test diferencial_vm` → **118 passed, 0 failed**.

### 3.2 Multi-backend (TW/VM/Nativo/WASM, 04.5)
- **Cobertura ejercitada:** matriz declarada, equivalencias VM↔Nativo (aritmética, control/recursión, texto/colecciones básicas), equivalencias VM↔WASM (aritmética/loop entero, condicional/funciones), casos `UNSUPPORTED` por subconjunto (texto y módulos en WASM, try en Nativo), clasificación de divergencia/unverificable por contrato.
- **Resultado:** `cargo test --test diferencial_multibackend` → **11 passed, 0 failed**.

### 3.3 Cobertura no completada / limitada (evidencia)
- **Módulos TW↔VM con fixtures comparables en harness diferencial:** no instrumentado de forma homogénea; **VM no implementa cargador file-based `importar/usar`** (evidencia: ausencia de soporte en `prototipo/src/vm.rs`; TW usa `CargadorModulos`/`Entorno::con_cargador`). Evidencia de módulos existe en suites específicas (p.ej. backend nativo) pero no como paridad TW↔VM in-process equivalente.
- **Stdout/efectos observables homogéneos TW↔VM:** **limitado**. TW emite a stdout de proceso (captura global in-process no determinista en tests paralelos, sin buffer en `Entorno`); VM captura `salida()`. Se validó stdout determinista en VM y paridad de valor TW↔VM; comparación de efectos stdout entre ambos no es comparable en el harness actual.
- **`Invoke` MIR:** ejercitado **indirectamente** por rutas expresables de `intentar/rescatar` (lowering/ejecución VM, casos diferenciales existentes y añadidos). No se identificó ruta MIR `Invoke` directa aislada expresable desde fuente comparable adicional.
- **Mutación/asignación de campos de objeto:** **no expresable** con sintaxis/soporte estable identificado en parser/AST/HIR/MIR/evaluadores para asignación de campo (`obj.campo = ...` / equivalente). Lectura de objetos (JSON) y mutación de colecciones indexadas sí cubiertas.

## 4. Matriz de capacidades multi-backend (subconjuntos declarados)

| Capacidad | TW | VM | Nativo | WASM | Notas |
|---|:--:|:--:|:--:|:--:|---|
| Numéricos/lógicos básicos | ✓ | ✓ | ✓ | ✓ | Comparables en subconjunto |
| Control de flujo básico | ✓ | ✓ | ✓ | ✓ |  |
| Funciones/llamadas directas | ✓ | ✓ | ✓ | ✓ |  |
| Recursión | ✓ | ✓ | ✓ | ✓ |  |
| Texto/colecciones básicas | ✓ | ✓ | ✓ | — | Nativo: subconjunto runtime; WASM no soporta |
| Try/rescatar | ✓ | ✓ | — | — | Nativo/WASM `UNSUPPORTED` |
| Módulos (alias/nominal) | ✓ | ✓ | ✓ | — | TW↔VM file-based no comparable en harness; WASM `UNSUPPORTED` |
| Closures/lambdas | ✓ | ✓ | — | — |  |
| Concurrencia/canales/tareas | ✓ | ✓ | — | — |  |
| FFI | ✓ | ✓ | — | — |  |
| IO archivos | ✓ | ✓ | — | — |  |
| JSON stdlib | ✓ | ✓ | ✓ | — | Nativo: soporte parcial (p.ej. `json_parsear`; `json_obtener` no expuesto como intrínseco equivalente en runtime/harness) |
| Efectos stdout deterministas | ✓ | ✓ | ✓ | — | TW↔VM no homogéneo in-process; VM/Nativo comparables según normalización |
| Export entero WASM (`bsk_main`) | — | — | — | ✓ |  |

**Regla:** ausencia de soporte declarada ⇒ `UNSUPPORTED` (no divergencia). Evidencia insuficiente/herramienta o salida no normalizable ⇒ `UNVERIFIABLE`.

## 5. Resultados de pruebas y validaciones

| Validación | Resultado (MISIÓN 03) | Base MISIÓN 02 | Diferencia |
|---|---|---:|---:|
| `cargo fmt --all -- --check` | **OK** | OK | — |
| `cargo clippy --all-targets --all-features -- -D warnings` | **OK** | OK | — |
| `cargo test --test diferencial_vm` | **118 passed, 0 failed** | 113/0 | +5 |
| `cargo test --test diferencial_multibackend` | **11 passed, 0 failed** | 8/0 | +3 |
| `cargo test --workspace --no-fail-fast` | **1024 passed, 0 failed, 2 ignored** (56 suites) | 1016/0/2 | +8 passed |
| Suites específicas | `mir` 3/3, `vm` 11/11, `backend_nativo` 45/45, `backend_wasm` 11/11, `optimizador` 12/12 | iguales | sin regresión |

- **Ignorados (no aprobados, B-REG-001):** 2 en `tests/registry_integration.rs` (`ciclo_publicacion_descarga_instalacion`, `concurrencia_misma_version`) — requieren PostgreSQL aislado/`DATABASE_URL`/`psql`/entorno controlado.
- **Entorno observado:** dependencias según caso (rustc, Node+wasm32, compilador C/FFI, FS/HTTP local). Clasificaciones multi-backend tratan indisponibilidad/no normalización como `UNVERIFIABLE` o `UNSUPPORTED` según corresponda.

## 6. Decisión fundamentada — GAP-051

- **Definición:** GAP-051 refiere a conversiones/asignaciones (y retornos/parámetros) tipadas estrictas (ADR-0005) y divergencia diagnóstica de representación `obtenido` (p.ej. `Lista` vs `Arreglo`) manteniendo la misma causa semántica.
- **Evidencia:** contrato endurecido 04.2 con whitelist acotada y mapeos 04.4; casos diferenciales existentes y ampliados (`diff_gap051_*`, retornos/asignaciones tipadas incompatibles, colecciones tipadas, reasignaciones). En los casos cubiertos por la whitelist, comparación produce `Equivalente`/`DiferenciaDiagnostico` solo cuando categoría/`esperado`/contexto coinciden y difiere únicamente `obtenido` por representación permitida.
- **Investigación adicional (MISIÓN 03):** pruebas exploratorias de parámetros tipados incompatibles mostraron divergencias no absorbibles por la whitelist actual (categorías/comportamiento distintos TW vs VM en casos concretos). Estas divergencias quedan fuera del alcance seguro de cierre de GAP-051 y evidencian necesidad de alineación/evidencia adicional.
- **Decisión técnica:** **mantener la whitelist diagnóstica acotada existente y NO cerrar GAP-051**. Estado: **parcialmente resuelto, sin cierre formal**.
- **Justificación:** (1) retirar la whitelist sin evidencia sistemática completa puede introducir falsos rojos/verdes; (2) existen brechas observadas en tipado/paridad de parámetros que requieren tratamiento específico; (3) la tolerancia actual es mínima, explícita y no unifica causas distintas.
- **Trabajo futuro recomendado (autorizado por separado):** batería sistemática de asignación/retorno/parámetro tipado en todas las formas expresables, corrección de divergencias semánticas reales detectadas, y reducción/eliminación de whitelist solo con demostración completa.

## 7. Pruebas de registry (B-REG-001) y validación reproducible en CI

- **Revisión:** `prototipo/tests/registry_integration.rs` contiene 2 pruebas `#[ignore]`: `ciclo_publicacion_descarga_instalacion` y `concurrencia_misma_version`. Ambas validan `DATABASE_URL`, exigen host local y DB no de sistema (`is_isolated`), requieren `psql`, crean esquema temporal único, usan directorios/artefactos temporales, ejecutan `basilisco-registry` en puerto efímero y limpian vía RAII + `DROP SCHEMA ... CASCADE`.
- **Bloqueo local:** ejecución no verificable sin PostgreSQL aislado y dependencias externas (B-REG-001). No se forzó ejecución sin entorno.
- **CI reproducible preparado:** `.github/workflows/ci.yml` → job `e2e-postgres` (ubuntu-latest) con servicio `postgres:16`, healthcheck, `DATABASE_URL` local (`postgres://basilisco:basilisco_dev_password@localhost:5432/basilisco`), build de `basilisco-registry` y ejecución `cargo test --test registry_integration -- --ignored --nocapture` en `prototipo/`. Esto proporciona ruta reproducible en CI para validación cuando el workflow se ejecute.

## 8. Inconsistencias documentales corregidas / alineación

- `README.md`: versión del manifiesto alineada a `0.7.0` (consistente con `prototipo/Cargo.toml`) y nota de que Fase 04 no implica cierre automático.
- `ROADMAP_MASTER.md`: Fase 04 actualizada a **EN PROGRESO (no cerrada)** con dependencias satisfechas, tareas/evidencias/limitaciones y referencia al estado vivo.
- `DEVELOPMENT_STATE.md`: actualizado con avance MISIÓN 03, validación **1024/0/2**, detalle de Fase 04, decisión GAP-051, registry/CI y historial; mantiene Fase 04 no cerrada y Fase 05 no iniciada.
- `docs/reports/FASE_04_5_CONTRATO_MULTIBACKEND.md`: actualizado a resultados **11/11** y pendientes vigentes tras MISIÓN 03.
- Expediente único creado: `docs/reports/FASE_04_EXPEDIENTE_CIERRE.md`.

## 9. Limitaciones restantes y pendientes

| Tema | Limitación / pendiente | Impacto |
|---|---|---|
| Módulos TW↔VM | Sin harness diferencial file-based comparable (VM sin `importar/usar` con cargador) | Brecha de cobertura TW↔VM en módulos |
| Stdout TW↔VM | Captura homogénea in-process no viable; VM validada determinísticamente, TW no comparable en mismo harness | Efectos stdout comparables limitados |
| Objetos/campos | Mutación de campo de objeto no expresable/identificada | Sin casos diferenciales de mutación de campo |
| Invoke MIR directo | Solo indirecto vía try/rescatar expresable | Cobertura MIR específica acotada |
| GAP-051 | Whitelist conservada, evidencia sistemática incompleta, divergencias exploradas en parámetros tipados | Sin cierre formal |
| Multi-backend | Subconjuntos acotados (nativo/wasm), algunos intrínsecos/formatos no normalizables | `UNSUPPORTED/UNVERIFIABLE` esperables |
| Registry E2E | 2 pruebas ignoradas localmente por dependencia externa | Validación depende de entorno/CI |
| Ámbito | Cambios locales no commiteados; revisión humana pendiente | No hay baseline commit de Fase 04 en repo |

## 10. Recomendación y decisión de cierre

- **Recomendación técnica (MISIÓN 03, previa a decisión):** no cerrar sin aceptación explícita de limitaciones/GAP-051 o trabajo adicional autorizado; consolidar evidencia y revisar diffs.
- **Decisión humana registrada (addendum):** **APROBADA/CERRADA formalmente** (2026-11-14, opción 1) aceptando limitaciones documentadas y la decisión GAP-051. GAP-051 permanece **parcialmente resuelto, sin cierre formal**.
- **Condiciones residuales (post-cierre, si se desea endurecer):** (a) trabajo adicional autorizado para brechas técnicas abiertas (ver §9); (b) validación reproducible en CI incluyendo `e2e-postgres` según política/disponibilidad; (c) consolidación/commit solo si es autorizado posteriormente.
- **Siguiente fase (roadmap):** Fase 05 — Backend nativo y funciones de primera clase (**EN PROGRESO**, autorizada; trabajo conservado).

## 11. Trazabilidad de comandos ejecutados (MISIÓN 03)

- `cargo fmt --all -- --check` (prototipo) → OK
- `cargo clippy --all-targets --all-features -- -D warnings` (prototipo) → OK
- `cargo test --test diferencial_vm` (prototipo) → 118 passed, 0 failed
- `cargo test --test diferencial_multibackend` (prototipo) → 11 passed, 0 failed
- `cargo test --workspace --no-fail-fast` (prototipo) → **1024 passed, 0 failed, 2 ignored** (56 suites), exit 0
- Log agregado: `/tmp/fase04_mision03_ws.txt`

---
*Expediente generado para revisión y decisión humana. No implica aprobación ni cierre de fase.*

## 12. Addendum — Cierre formal autorizado
- **Fecha/hora de decisión:** 2026-11-14
- **Decisión humana explícita:** autorización de **cierre/aprobación formal de Fase 04** (opción 1) aceptando las limitaciones documentadas y la decisión de GAP-051.
- **Registro:** `DEVELOPMENT_STATE.md` y `ROADMAP_MASTER.md` actualizados a **Fase 04 APROBADA**; GAP-051 permanece **parcialmente resuelto, sin cierre formal**; B-REG-001 y demás limitaciones documentadas. Sin commits/push/merge/tags/publicaciones. **Fase 05 no iniciada (pendiente de autorización).**
