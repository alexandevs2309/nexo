# Reporte 04.5 — Contrato diferencial multi-backend (TW/VM/Nativo/WASM)

- **Fecha:** 2026-11-14
- **Fase/Lote:** 04 / 04.5 (autorizado en MISIÓN 02)
- **Alcance:** contrato y clasificación multi-backend reutilizando `paridad.rs`, matriz de capacidades, pruebas/fixtures diferenciales y documentación. Sin cerrar Fase 04, sin iniciar Fase 05, sin commits/push.

## 1. Resumen

Se extendió el contrato existente (`prototipo/src/paridad.rs`) con un modelo multi-backend 04.5 que clasifica comparaciones entre Tree-Walk, VM, backend nativo y backend WASM como `EQUIVALENT`, `DIVERGENCE`, `UNSUPPORTED` o `UNVERIFIABLE`. La ausencia de soporte declarada se distingue de divergencia semántica. Se añadieron pruebas diferenciales multi-backend ejecutables y reproducibles.

## 2. Implementación

- **`prototipo/src/paridad.rs`** (reutilizado, sin arquitectura paralela): `Backend`, `Capacidad`, `SoporteBackend`, `capacidades_backend` (matriz declarada), `ClasificacionMulti {Equivalent,Divergence,Unsupported,Unverifiable}`, `clasificar_multibackend` (reusa `comparar_semantica` cuando ambos soportan), `resultado_desde_salida_simple`, `EjecucionBackend`.
- **`prototipo/src/lib.rs`**: re-export de la API 04.5.
- **`prototipo/tests/diferencial_multibackend.rs`**: harness que ejecuta VM y backends reales (rustc/node+wasm32 cuando disponibles), normaliza/compara y clasifica.

## 3. Matriz de capacidades (subconjuntos declarados)

| Capacidad | TW | VM | Nativo | WASM |
|---|:--:|:--:|:--:|:--:|
| Numéricos/lógicos básicos | ✓ | ✓ | ✓ | ✓ |
| Control de flujo básico | ✓ | ✓ | ✓ | ✓ |
| Funciones/llamadas directas | ✓ | ✓ | ✓ | ✓ |
| Recursión | ✓ | ✓ | ✓ | ✓ |
| Texto/colecciones básicas | ✓ | ✓ | ✓ | — |
| Try/rescatar | ✓ | ✓ | — | — |
| Módulos (alias/nominal) | ✓ | ✓ | ✓ | — |
| Closures/lambdas | ✓ | ✓ | — | — |
| Concurrencia/canales/tareas | ✓ | ✓ | — | — |
| FFI | ✓ | ✓ | — | — |
| IO archivos | ✓ | ✓ | — | — |
| JSON stdlib | ✓ | ✓ | ✓ | — |
| Efectos stdout deterministas | ✓ | ✓ | ✓ | — |
| Export entero WASM (`bsk_main`) | — | — | — | ✓ |

**Limitaciones:** Nativo requiere rustc y no soporta try/invoke/closures/celdas/rangos ni stdlib fuera del runtime embebido; sin declaración de soporte para concurrencia/FFI/IO archivos en este contrato. WASM es subconjunto entero/lógico sin texto/colecciones/try/módulos/closures/concurrencia/FFI/IO/JSON.

## 4. Clasificación y reglas

- `EQUIVALENT`: equivalente dentro del subconjunto soportado (incluye diferencias diagnósticas/representación absorbidas por contrato).
- `DIVERGENCE`: ambos declaran soporte y el resultado difiere semánticamente.
- `UNSUPPORTED`: al menos un backend no declara soporte/capacidad para el caso (no se confunde con divergencia).
- `UNVERIFIABLE`: evidencia insuficiente (p. ej. salida no normalizable de forma fiable, dependencia de entorno no disponible).

## 5. Evidencia de pruebas (ejecutada)

- `cargo test --test diferencial_multibackend` → **11 passed, 0 failed** (MISIÓN 03, ampliación): matriz declarada, equivalencias VM↔Nativo (aritmética, control/recursión, texto/colecciones básicas), equivalencias VM↔WASM (aritmética/loop entero, condicional/funciones), `UNSUPPORTED` texto y módulos en WASM, `UNSUPPORTED` try en Nativo por subconjunto, clasificaciones de divergencia/unverificable.
- Suites específicas preservadas (MISIÓN 03): `diferencial_vm` 118/118, `mir` 3/3, `vm` 11/11, `backend_nativo` 45/45, `backend_wasm` 11/11, `optimizador` 12/12. Validación integral workspace **1024 passed, 0 failed, 2 ignored**.

## 6. GAP-051 (estado formalizado en alcance cubierto)

- **Situación (histórica 04.5/MISIÓN 03):** GAP-051 (conversiones/asignaciones tipadas estrictas, ADR-0005) cubierto por correcciones y whitelist acotada en `paridad.rs`; casos diferenciales dirigidos equivalentes/aceptados según contrato.
- **Formalización documental autorizada:** GAP-051 **CERRADO en el alcance cubierto por pruebas diferenciales**. Se mantienen reglas normativas de conversión/causa alineadas en contextos cubiertos y la whitelist diferencial **justificada y restringida** (diferencias diagnósticas acotadas: representación `obtenido` Lista/Arreglo y, cuando aplica, identificador de parámetro no disponible en MIR/VM con índice 1-based), sin autorizar diferencias en `esperado`/`funcion`.
- **Observaciones técnicas preservadas:** cobertura no exhaustiva de todas las rutas de ejecución; no se afirma equivalencia total entre motores. Evidencia técnica adicional registrada en estado vivo/registro de brechas (validación local diferenciada de CI). La reejecución/fortalecimiento futuro, si aplica, requiere autorización y evidencia separadas.

## 7. Cobertura ampliada y viabilidad (04.5/alcance)

- **Módulos `importar/usar`:** fixtures comparables existentes en `backend_nativo` (alias/nominal/anidado/init) con paridad frente a VM; harness TW↔VM directo no extendido a módulos con cargador en este lote (coste de instrumentación). Clasificable como capacidad soportada en nativo según matriz; evidencia TW↔VM de módulos requiere fixtures/harness dedicado.
- **Rutas `Invoke` expresables en MIR:** ejercitadas indirectamente vía semántica try/rescatar en paridad TW↔VM (casos `diff_intentar_rescatar*`, 04.3). No se añadió ruta MIR directa adicional aislada.
- **Efectos stdout homogéneos:** VM captura salida; TW emite a stdout de proceso en rutas generales (captura in-process no homogénea/determinista en el harness actual). No forzados; clasificados fuera de comparación homogénea salvo casos instrumentados.
- **Mutación de objetos/campos:** no se identificó sintaxis/soporte estable de asignación de campo mutable de objeto comparable en los motores para este alcance; no inventado. Lectura de objetos JSON cubierta (`json_obtener`).
- **Determinismo de entorno:** casos deterministas existentes (archivos temporales, HTTP local, FFI con compilación controlada) se mantienen en diferencial TW↔VM.

## 8. Pendientes (vigentes tras MISIÓN 03)

- 04.8 cierre formal de Fase 04 (requiere revisión/autorización humana). Expediente preparado: `docs/reports/FASE_04_EXPEDIENTE_CIERRE.md`.
- Módulos TW↔VM con fixtures comparables en harness diferencial directo: **limitación arquitectónica** (VM sin cargador file-based `importar/usar`); evidencia de módulos existe en subconjunto nativo por suites específicas, no como paridad TW↔VM in-process homogénea.
- Stdout homogéneo TW↔VM in-process: **limitado** (TW imprime a proceso; VM captura `salida()`). MISIÓN 03 añadió validación determinista de stdout en VM y comparación por valor; comparación de efectos stdout TW↔VM requiere instrumentación/captura fuera del harness actual.
- Mutación/asignación de campos de objeto: **no expresable** con sintaxis/soporte identificado; lectura JSON cubierta.
- `Invoke` MIR: ejercitado indirectamente vía try/rescatar expresable (evidencia diferencial); ruta MIR directa aislada desde fuente no identificada como expresable.
- GAP-051: **CERRADO en alcance cubierto por pruebas diferenciales** con whitelist acotada justificada/restringida y observaciones técnicas preservadas (MIR sin nombres de parámetros, diferencias diagnósticas Lista/Arreglo, cobertura no exhaustiva; sin equivalencia total). Cualquier ampliación futura requiere autorización/evidencia separada.
- `DEVELOPMENT_STATE.md`: Fase 04 no marcada como cerrada.
