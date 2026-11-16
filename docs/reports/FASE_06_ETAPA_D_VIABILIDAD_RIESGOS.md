# Fase 06 — Etapa D: Documento de viabilidad y riesgos
## Sistema de inferencia de tipos de BASILISCO (D.1–D.4)

- **Fase:** 06 — Sistema de tipos avanzado
- **Etapa:** D (investigación avanzada más allá de C.2)
- **Subetapas consolidadas:** D.1 (encuadre), D.2 (línea base/brechas), D.3 (viabilidad/riesgos), D.4 (síntesis y recomendación)
- **Fecha de síntesis:** 2026-11-16
- **Alcance:** investigación documental/técnica de solo lectura sobre el repositorio en `main` (HEAD `66a2966` al momento del análisis). **No se implementaron mejoras de inferencia.** No se modificaron código, `DEVELOPMENT_STATE.md`, `ROADMAP_MASTER.md`, ADRs existentes ni otros documentos del repositorio.
- **Autoridad y fuentes:** `ROADMAP_MASTER.md` (Fase 06), `00-gobernanza/adr/ADR-0018-evolucion-inferencia-tipos.md` (D0/D3–D6, Etapas B–D), `00-gobernanza/planes/PLAN-MAESTRO-ADR-0018.md`, `00-gobernanza/adr/ADR-0011-sistema-de-tipos-gradual.md`, `00-gobernanza/adr/ADR-0017-analisis-semantico-completo.md`, `DEVELOPMENT_STATE.md`, e inspección de `prototipo/src/*` y `prototipo/tests/*` (especialmente `semantico.rs`, `simbodo.rs`, `hir.rs`, `mir.rs`, `evaluador.rs`, `vm.rs`).

---

## 1. Resumen ejecutivo

- **Objetivo de Etapa D (original):** investigación de inferencia **avanzada** de cuerpo/retornos **más allá del alcance acotado de C.2** (p. ej. interprocedimental avanzada, flujos complejos u otras técnicas), **sin implementación obligatoria**, con entregable de viabilidad/riesgos y, si corresponde, propuesta de decisión/ADR futura (ADR-0018 §7, plan maestro).
- **Estado técnico actual:** B/C.0/C.1/C.2 están cerrados. C.2 proporciona inferencia local conservadora de retornos por declaración (`SimbodoId`), en canal separado de C.1, con consulta interna `consultar_c2_retorno` y sin consumo productivo externo, preservando D0/D3–D6 y sin cambios en C.1, diagnósticos/E1010, `run`/runtime/`tipos_parametros_inferidos`, VM/backends/HIR/MIR.
- **Hallazgo central:** no existe capacidad implementada más allá de C.2 para propagación interprocedimental de inferidos, convergencia por punto fijo, análisis bidireccional, ni tratamiento avanzado de flujos/ciclos más allá de la regla estructural conservadora vigente. Las brechas identificadas son reales, pero su viabilidad segura bajo las restricciones vigentes es limitada o no demostrada.
- **Conclusión de viabilidad:** 
  - **D.3.1-A (refinamiento estructural conservador mínimo):** **viable con restricciones** solo como hipótesis de futura propuesta acotada (beneficio potencial bajo–medio, riesgo medio, requiere especificación formal y evidencia exhaustiva). **No aprobada para implementación.**
  - **D.3.3-A (propagación de resultados C.2 por `SimbodoId`):** **no recomendable bajo restricciones vigentes** (tensión directa con D5); solo podría considerarse como hipótesis sujeta a decisión normativa/arquitectónica independiente.
  - **D.3.4-A (uso acotado de tipo esperado local):** **indeterminada** (beneficio incierto, sensibilidad a D0).
  - Resto de alternativas evaluadas (CFG intra-función amplio, punto fijo, interprocedimental avanzada general, bidireccional desde usos/llamadas, y detección de ciclos sin mejora de precisión): **no recomendables** o **marginales/indeterminadas**.
- **Recomendación contrastada con evidencia:** **no implementar todavía ninguna mejora de inferencia avanzada**. Mantener intactos B, C.0, C.1 y C.2, así como D5 y D0/D3/D4/D6. Conservar **D.3.1-A** únicamente como **candidata a evaluación en una futura decisión independiente**, supeditada a especificación previa, justificación de beneficio y evidencia de no regresión. Cualquier otra vía que implique relajar D5, introducir punto fijo/CFG/interprocedimental/bidireccional sustantivos, o afectar `run`/runtime/diagnósticos/IR/motores no se recomienda en el estado actual.

---

## 2. Objetivo y alcance

### 2.1 Objetivo
Consolidar D.1–D.3 para emitir una conclusión técnica fundamentada sobre la viabilidad, riesgos y posibles líneas arquitectónicas futuras del sistema de inferencia de tipos de BASILISCO, sin autorizar ni iniciar implementación.

### 2.2 Alcance incluido
- Delimitación del espacio “más allá de C.2” (D.1).
- Línea base técnica y brechas en flujos de control, recursividad/ciclos, interprocedimental acotada y bidireccional parcial (D.2).
- Evaluación de viabilidad/riesgo frente a criterios técnicos y normativos (D.3).
- Clasificación final de alternativas y recomendación arquitectónica (D.4).

### 2.3 Fuera de alcance
- Implementación de código, cambios en tests, en runtime/VM/backends/HIR/MIR/parser/AST/CLI/LSP, o en diagnósticos/comportamiento.
- Modificación de `DEVELOPMENT_STATE.md`, `ROADMAP_MASTER.md`, ADRs existentes u otros documentos existentes.
- Reapertura de B, C.0, C.1 o C.2; cierre automático de Etapa D/Fase 06; inicio de nuevas fases.
- Resolución de GAP-051 y B-REG-001 (independientes).

---

## 3. Estado técnico actual

| Área | Estado (hecho comprobado) | Evidencia principal |
|---|---|---|
| Etapas cerradas | B, C.0, C.1, C.2 cerrados; Etapa C cerrada; Fase 06 `IN PROGRESS`; Etapa D `NOT STARTED` (antes de esta síntesis documental) | `ROADMAP_MASTER.md`, `DEVELOPMENT_STATE.md`, `ADR-0018` |
| Modelo semántico | `TipoSem` (incl. `Funcion`, `Desconocido`), compatibilidad/unificación local (`tipo_comun`), análisis estático sobre AST con ámbitos/símbolos | `prototipo/src/semantico.rs`, `prototipo/src/simbodo.rs` |
| C.1 | Firmas desde anotaciones explícitas, resolución léxica conservadora, preservación de propagación/fallback legado separado de C.2 | `semantico.rs` (registro/resolución de funciones y llamadas) |
| C.2 | Canal separado por `SimbodoId` (`c2_resultados`), activación en candidatas no anotadas, recolección/admisibilidad conservadora (`c2_expr_admisible`, `c2_llamada_admisible`), finalización estructural (`bloque_definitivamente_retorna`, `c2_finalizar`, `c2_unificar`), consulta `consultar_c2_retorno` | `semantico.rs` (~100–129, ~533–649, ~1695–1838, pruebas internas ~1899–2423) |
| Restricciones efectivas en C.2 | D5: llamadas admisibles solo con retornos anotados explícitos/visibles; indirectas/accesos inadmisibles. D3/D4: `retornar` vacío/caída/insuficiencia → `Desconocido`. Caminos no “definitivos” (si sin sino, bucles, try/rescatar, etc.) degradan. Recursión/ciclos sin evidencia admisible → `Desconocido` | `semantico.rs` (~1795–1838, ~1748–1789); tests `c2_callee_sin_anotacion_desconocido_d5`, `c2_recursion_sin_evidencia_independiente`, `c2_dependencia_circular_desconocido`, `c2_camino_incompleto_si_sin_sino`, `c2_retorno_vacio_desconocido` |
| Integración de C.2 | Sin consumidor productivo en HIR/MIR/evaluador/VM/backends (consulta interna/pruebas) | búsquedas sin `consultar_c2_retorno`/`c2_resultados` fuera de `semantico.rs` (en `src`) |
| Separación estático/dinámico | `analizar` sin compartir estado con evaluador; `run`/runtime/`tipos_parametros_inferidos` preservados; D0/D6 vigentes | `ADR-0018`, inspección de flujo CLI/LSP/semántica |

---

## 4. Hallazgos consolidados de D.1–D.3

### 4.1 D.1 — Encuadre y delimitación
- **Hecho:** Etapa D es investigación avanzada más allá de C.2, sin implementación obligatoria; entregable esperado: viabilidad/riesgos y, si procede, propuesta de decisión/ADR futura.
- **Delimitación:** candidatos a investigar (retornos/flujos/recursión más allá de C.2, interprocedimental acotada, bidireccional parcial) vs. exclusiones explícitas (genéricos, bidireccional completa, cambios runtime/IR/diagnósticos/`run`, relajación implícita de D0/D3–D6, reapertura de etapas cerradas).
- **Condición:** toda continuación requiere priorización humana del foco de investigación y respeto estricto de restricciones normativas.

### 4.2 D.2 — Línea base técnica y brechas
- **Flujos complejos:** precisión limitada por criterio estructural de “retorno definitivo” (solo retorno directo/bloque/si completo con ambas ramas); bucles, try/rescatar, si sin sino y caminos abiertos → `Desconocido` (hecho).
- **Recursividad/ciclos:** no hay punto fijo ni modelado de dependencias; llamadas a no anotados inadmisibles en C.2 → convergencia no resuelta, resultado conservador (hecho).
- **Interprocedimental:** D5 impide usar retornos inferidos C.2 de callees; resultados C.2 no se propagan ni consumen productivamente (hecho).
- **Bidireccional:** no hay uso de tipos esperados/contextuales para guiar inferencia C.2 (hecho por ausencia en mecanismo).
- **Brecha global:** ausencia de mecanismo avanzado implementado y ausencia del entregable documental de viabilidad previo a D.4 (hecho).

### 4.3 D.3 — Evaluación de viabilidad y riesgos
- **Evaluación:** la mayoría de alternativas presentan complejidad/riesgo altos con beneficio incierto o acotado bajo D0/D3–D6 y preservación C.1/C.2.
- **Única alternativa “viable con restricciones”:** D.3.1-A (refinamiento estructural conservador mínimo), sujeta a demostrabilidad estricta, especificación formal y evidencia; beneficio acotado.
- **Tensión normativa clave:** D.3.3-A es incompatible con D5 vigente; solo concebible como hipótesis supeditada a decisión normativa independiente.
- **Indeterminaciones:** D.3.4-A (valor neto no demostrado, sensibilidad a D0) y detección de ciclos sin mejora de precisión (marginal).

### 4.4 Coherencia entre conclusiones y evidencia
- **Consistente:** las conclusiones de D.3 se derivan de mecanismos observados en `semantico.rs` (admisibilidad, finalización, ausencia de punto fijo/propagación/bidireccional) y de las restricciones documentadas (D0/D3–D6, separación de canales, `analizar`/`run`).
- **No contradicho por evidencia disponible:** no se hallaron implementaciones ocultas o consumidores de C.2 en IR/motores que alteren la evaluación.
- **Límite explícito:** viabilidad de mejoras específicas y cuantificación de beneficio no están demostradas (requieren especificación y, en su caso, evidencia experimental futura autorizada).

---

## 5. Matriz final de viabilidad

| ID | Alternativa | Clasificación final | Justificación resumida | Condiciones / notas |
|---|---|---|---|---|
| D.3.1-A | Refinamiento estructural conservador mínimo de “retorno definitivo” | **Recomendada para futura propuesta** (condicionada) | Única opción con balance potencialmente aceptable si se limita a casos estructuralmente demostrables, manteniendo `Desconocido` como fallback y sin tocar D0/D3/D4/D6 ni C.1/C.2. | **No es tarea aprobada.** Requiere autorización independiente, especificación formal de reglas/casos, justificación de beneficio y evidencia de no regresión antes de cualquier implementación. |
| D.3.1-B | Análisis CFG intra-función para convergencia de retornos | **No recomendable** | Complejidad y riesgo altos (modelado de alcanzabilidad/semántica), beneficio incierto, posible desalineación con pipeline/IR y riesgo de falsos positivos. | Solo reconsiderable con diseño y autorización específicos, no en el estado actual. |
| D.3.2-A | Punto fijo / iteración para recursión y ciclos | **No recomendable** | Garantías de terminación/determinismo no demostradas, complejidad alta, riesgo de regresiones y tensión con D0/D5/D6. | Rechazada como vía práctica en el estado actual. |
| D.3.2-B | Detección conservadora de ciclos/recursión sin mejora de precisión | **Indeterminada** (marginal) | No mejora inferencia; posible valor limitado de mantenibilidad/claridad interna, sin prioridad para Etapa D. | No justificaría una propuesta de evolución de inferencia. |
| D.3.3-A | Propagación de resultados C.2 conocidos por `SimbodoId` | **Descartada bajo las restricciones actuales** (hipótesis condicionada a decisión normativa futura) | Beneficio potencial acotado pero **incompatible con D5 vigente** (admisibilidad limitada a retornos anotados explícitos/visibles); riesgos de orden/consistencia y de contaminación de canales. | Solo podría pasar a “viable con restricciones” como hipótesis si existiera autorización y redefinición normativa explícita de D5 y garantías formales; no asumido. |
| D.3.3-B | Interprocedimental avanzada general (call-graph/propagación amplia) | **No recomendable** | Complejidad/riesgo muy altos, infraestructura inexistente, fuera del alcance acotado y sin garantías de terminación/determinismo. | Descartada en práctica. |
| D.3.4-A | Uso acotado de tipo esperado local (sin bidireccional general) | **Indeterminada** | Beneficio neto no demostrado; sensibilidad a D0 y riesgo de efectos colaterales si modifica inferencia observable. | Requiere definición concreta y evidencia de valor sin violar D0/D6. |
| D.3.4-B | Bidireccional desde sitios de llamada/tipos esperados hacia no anotados | **No recomendable** | Ambigüedad, dependencia contextual, riesgo alto de falsos positivos y conflictos con D0/D6 y modelo gradual. | No recomendar como propuesta. |

---

## 6. Riesgos y restricciones

### 6.1 Riesgos transversales
- **Falsos positivos / pérdida de conservadurismo:** inferir sin demostrabilidad suficiente.
- **Regresión de contratos cerrados:** contaminación o alteración indirecta de C.1/C.2 (canales, `FuncionSig`, fallback, resultados por identidad).
- **Violación normativa:** incumplir D0 (diagnósticos/códigos/E1010/`run`), D3/D4 (retornos), D5 (evidencia interprocedimental), D6 (separación estático-dinámico y alcance de ejecución).
- **Complejidad no gobernable:** terminación, determinismo y rendimiento en análisis de flujos/ciclos/grafos.
- **Incoherencia de pipeline/motores:** dependencias futuras con HIR/MIR/VM/backends o divergencias TW↔VM↔nativo↔WASM.

### 6.2 Restricciones innegociables vigentes
- Preservar B, C.0, C.1 y C.2 y sus resultados/contratos.
- Mantener D0, D3, D4, D5 y D6.
- No alterar `run`, runtime, `tipos_parametros_inferidos`, diagnósticos existentes, E1010, HIR/MIR, VM o backends como efecto de inferencia.
- No reabrir etapas cerradas ni asumir cambios normativos sin autorización/ADR independiente.

### 6.3 Dependencias independientes
- **GAP-051** (paridad, parcialmente resuelto, whitelist conservada, sin cierre formal) y **B-REG-001** (pruebas de registro ignoradas por dependencia externa PostgreSQL): sin relación directa ni prerrequisito obligatorio para Etapa D.

---

## 7. Alternativas descartadas

**Descartadas bajo las restricciones actuales (no recomendables para propuesta):**
- D.3.1-B (CFG intra-función amplio).
- D.3.2-A (punto fijo/convergencia iterativa).
- D.3.3-B (interprocedimental avanzada general).
- D.3.4-B (bidireccional desde sitios de llamada/esperados).
- Cualquier alternativa que requiera modificar D0/D3/D4/D5/D6, alterar C.1/C.2, `run`/runtime/diagnósticos, HIR/MIR o backends, o reabrir etapas cerradas.

**Descartada como candidata de inferencia (pero marginalmente neutra):** D.3.2-B, por no aportar mejora de precisión.

---

## 8. Recomendación arquitectónica

### 8.1 Recomendación principal (contrastada con evidencia)
1. **No implementar todavía ninguna mejora de inferencia avanzada.** La evidencia (D.2/D.3) muestra beneficio incierto o acotado y riesgos relevantes para la mayoría de vías bajo las restricciones vigentes.
2. **Mantener intactos** los contratos y cierres de **B, C.0, C.1 y C.2**.
3. **Conservar** **D5** y las restricciones **D0/D3/D4/D6**.
4. **Mantener D.3.1-A** como **única candidata para una futura decisión independiente**, sin convertirla en tarea aprobada.

Esta recomendación es coherente con los hallazgos: D.3.1-A es la sola alternativa evaluada como “viable con restricciones” y aun así exige acotación estricta y evidencia.

### 8.2 Evaluación específica de D.3.1-A

| Aspecto | Evaluación |
|---|---|
| **Beneficio real** | Potencial **bajo–medio** y acotado: cubrir un subconjunto adicional de casos estructuralmente demostrables hoy degradados a `Desconocido`. No demostrado como impacto significativo sin inventario de patrones concretos. |
| **Límites exactos (requeridos en especificación)** | Solo extensiones donde **todos los caminos** sean demostrablemente retornantes bajo reglas estructurales formales y equivalentes en semántica; exclusión explícita de bucles, try/rescatar, efectos/casos no modelados, y cualquier situación ambigua; `Desconocido` como resultado obligatorio ante duda; sin cambio en D3/D4/D0. |
| **Riesgos** | **Medio:** falsos positivos por definición imprecisa de completitud de caminos, regresiones en C.1/C.2, y riesgo de introducir expectativas no alineadas con semántica/diagnósticos existentes. |
| **Requisitos de especificación (previos a cualquier implementación)** | (a) Catálogo exhaustivo de formas sintáctico-semánticas admitidas y excluidas; (b) pruebas formales/informales de demostrabilidad por forma; (c) interacción explícita con C.2 (`c2_finalizar`, admisibilidad, unificación) y con C.1; (d) confirmación de no impacto en `run`/diagnósticos/HIR/MIR; (e) criterios de aceptación y plan de regresión. |
| **Evidencia necesaria antes de implementación** | Inventario de casos reales representativos (beneficio), casos negativos/ambiguos (seguridad), y ejecución satisfactoria de suites relevantes (`fmt`, `clippy -D warnings`, pruebas semánticas/tipos/funciones/lambdas/closures, diferenciales y workspace) en una propuesta futura autorizada. |
| **Estatus** | **Candidata a futura propuesta independiente**. **No aprobada, no planificada como implementación, no autorizada.** |

### 8.3 Decisiones que requieren autorización futura
- Iniciar cualquier trabajo de implementación derivado de Etapa D (incluido D.3.1-A).
- Aprobar una especificación normativa/técnica previa para D.3.1-A.
- Considerar cualquier revisión de **D5** u otras restricciones normativas (solo como hipótesis evaluada, mediante decisión/ADR independiente).
- Aprobar, si corresponde, una propuesta de ADR futura derivada de la investigación.
- Declarar el cierre formal de la Etapa D y/o de la Fase 06.
- Modificar `DEVELOPMENT_STATE.md`, `ROADMAP_MASTER.md` u otros documentos de gobernanza/estado.

---

## 9. Decisión arquitectónica explícita

> **Esta investigación (Etapa D, D.1–D.4) no autoriza:**
> - Modificar **D5**.
> - Reabrir **C.1** o **C.2**.
> - Implementar **CFG** para inferencia avanzada.
> - Introducir **punto fijo** o convergencia iterativa.
> - Introducir **inferencia interprocedimental**.
> - Introducir **inferencia bidireccional** (parcial o completa).
> - Modificar **runtime**, **`run`**, **diagnósticos**, **HIR/MIR** o **backends**.
>
> **D.3.1-A** (refinamiento estructural conservador mínimo) **solo** podrá convertirse en propuesta de implementación mediante una **autorización independiente** y una **especificación previa** que demuestre cumplimiento de D0/D3/D4/D5/D6 y preservación de C.1/C.2.

---

## 10. Conclusión final

La Etapa D, conforme a su definición original, ha producido la base de análisis requerida para **viabilidad y riesgos**: D.1 delimitó el problema, D.2 estableció la línea base y brechas con evidencia, D.3 evaluó alternativas frente a criterios técnicos y normativos, y D.4 consolida una recomendación prudente y trazable.

**Conclusión:** en el estado actual de BASILISCO, las mejoras de inferencia avanzada más allá de C.2 no presentan, de forma demostrada, un camino seguro y justificado para implementación inmediata. La opción menos riesgosa entre las evaluadas es **D.3.1-A**, pero únicamente como **candidata condicionada** a futura decisión independiente con especificación y evidencia rigurosas. La recomendación final es **no implementar ahora**, preservar los contratos cerrados y las restricciones vigentes, y reservar cualquier evolución para un proceso de decisión explícita y gobernada.

**Estado de la subetapa D.4:** entregable documental elaborado con base en evidencia consolidada (**IMPLEMENTED/VERIFIED** en alcance documental). **CLOSED: no declarado.** El cierre formal de la Etapa D queda pendiente de revisión y autorización humana.
