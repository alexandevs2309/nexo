# ADR-0018 — Evolución del sistema de inferencia de tipos de BASILISCO

**Estado:** Propuesto (alineación implementada en alcance C.0/C.1/C.2 aprobado; C.2/Etapa C cerrados formalmente con autorización humana 2026-11-16, Fase 06 IN PROGRESS)
**Fecha:** 2026-11-01  
**Relacionado con:** ADR-0011 (tipos graduales), ADR-0017 (análisis semántico), ADR-0008 (funciones).
**Alineación:** C.0 (MISIÓN 06.6) — Opción A conservadora aprobada; resolución normativa documental aplicada (MISIÓN 06.16) con D0/D3–D6. **C.1 CLOSED** (commit `229feb6`, aprobado posterior a MISIÓN 06.8). **C.2 IMPLEMENTED/VERIFIED y CLOSED** con aprobación humana formal (2026-11-16, MISIÓN 06.18–06.21): inferencia local conservadora de retornos por identidad de declaración (`SimbodoId`), canal separado de C.1, consulta interna por declaración, sin inferencia interprocedimental ni ampliación a lambdas/closures, sin cambios en C.1/diagnósticos/E1010/`run`/runtime/VM/backends/HIR/MIR. **Etapa C = CLOSED**; **Etapa D = NOT STARTED**.

## 1. Contexto

BASILISCO 0.6.0 dispone de inferencia por primera llamada para funciones sin anotaciones (`tipos_parametros_inferidos` en `FuncionDef`, `evaluador.rs:481-485`). El análisis semántico (`semantico.rs`) marca parámetros sin anotación como `TipoSem::Desconocido`, lo que genera divergencia entre `bsk analizar` y `bsk run`. Las lambdas y closures (`FuncionClausura`) no comparten el mismo mecanismo. Existe una batería de regresión (18 pruebas en `tests/inferencia_tipos.rs`) que documenta el comportamiento actual.

## 2. Problema

La inferencia actual depende del orden de ejecución y no se realiza estáticamente. Esto impide detectar incompatibilidades antes de ejecutar y dificulta la unificación entre funciones nombradas, lambdas y closures. Sin embargo, eliminar la inferencia por primera llamada rompería programas existentes y la semántica de tipos graduales (ADR-0011).

## 3. Decisión

Diseñar una evolución incremental que:

1. Formalice la semántica de parámetros sin anotación (inferencia por primera llamada como comportamiento de runtime, documentado y preservado).
2. Introduzca inferencia estática local para funciones con anotaciones parciales, sin reemplazar el runtime.
3. Incorpore, de forma acotada y conservadora, inferencia estática local de tipos de retorno en **C.2 (Etapa C)** conforme a la **Opción A** aprobada en C.0 y a las decisiones vinculantes D0/D3–D6, sin modificar el runtime ni la inferencia por primera llamada.
4. Unifique progresivamente el análisis de funciones nombradas, lambdas y closures.
5. Mejore la coherencia del análisis estático con las reglas de tipos sin hacer `analizar` y `run` idénticos y sin alterar la ejecución.
6. Preserve compatibilidad con programas existentes y las baterías de regresión.

Decisión clave: **no eliminar la inferencia por primera llamada**. La inferencia estática local convive con el runtime sin modificar `tipos_parametros_inferidos` y sin compartir estado inferido con el evaluador.

### 3.1 Inferencia estática local (definición)

Inferencia local significa inferir tipos únicamente dentro del ámbito léxico de una función o bloque, sin inferencia interprocedimental avanzada:

- Inferencia de expresiones: determinar el tipo de una expresión a partir de sus operandos y literales.
- Inferencia de parámetros parcialmente anotados: si un parámetro tiene anotación, validar argumentos; si no, mantener `TipoSem::Desconocido`.
- Inferencia de variables locales: `sea`/`dato` con inicializador tipado infiere el tipo de la variable.
- Inferencia de retornos (**C.2, Etapa C, Opción A conservadora, D0/D3–D6 — implementada, verificada y cerrada formalmente con aprobación humana 2026-11-16, MISIÓN 06.18–06.21, en alcance local conservador**): si el tipo de retorno no está anotado, inferirlo **solo** cuando sea conocido, demostrable y consistente a partir de evidencia local inequívoca. Los múltiples retornos solo permiten inferencia cuando sus tipos sean demostrablemente compatibles. Ante incertidumbre, incompatibilidad o evidencia insuficiente, conservar `TipoSem::Desconocido`. **D3:** no inferir automáticamente `Nulo` por alcanzar el final de una función sin `retornar` explícito; cuando no exista evidencia inequívoca de un tipo de retorno conocido, conservar `TipoSem::Desconocido`; no introducir excepciones implícitas para funciones anotadas `-> nulo` sin una regla normativa inequívoca. **D4:** `retornar` sin expresión no equivale a `retornar nulo` ni a una caída implícita al final de la función; se considera evidencia insuficiente para inferir un tipo de retorno conocido y se debe conservar `TipoSem::Desconocido` cuando ese camino intervenga en la inferencia, sin alterar el comportamiento vigente de ejecución ni E1010. **D5:** permitir exclusivamente el uso de retornos anotados explícitamente y visibles en el contexto semántico local; no propagar retornos inferidos desde cuerpos de otras funciones ni introducir inferencia interprocedimental avanzada.

Alcance por etapas: la **Etapa B (completada/CLOSED)** incorporó inferencia local de variables y expresiones, **sin inferencia estática de retornos**. La **Etapa C** comprende C.0 (CLOSED), C.1 (CLOSED) y **C.2 (CLOSED, implementado/verificado con aprobación humana 2026-11-16)**, donde C.2 queda acotado a la inferencia local conservadora descrita (D0/D3–D6). **D6:** la inferencia estática de C.2 pertenece al análisis semántico y no debe modificar la ejecución, el evaluador ni el estado de inferencia dinámica. **D0:** no introducir diagnósticos nuevos, no modificar diagnósticos existentes/códigos/comportamiento, no alterar `run`, no modificar runtime ni `tipos_parametros_inferidos` (ni VM/backends/HIR/MIR por esta capacidad). La **Etapa D (NOT STARTED)** es investigación avanzada más allá de C.2, sin implementación obligatoria y **no iniciada automáticamente**.

### 3.2 Reglas de evidencia

- Las anotaciones explícitas tienen prioridad absoluta.
- Solo se infieren tipos demostrables a partir del código fuente.
- La incertidumbre se representa mediante `TipoSem::Desconocido`.
- No se inventan tipos cuando falta información.
- Las incompatibilidades solo se reportan cuando están demostradas estáticamente.

### 3.3 Convivencia estática y dinámica

- El análisis estático no modifica `tipos_parametros_inferidos`.
- El evaluador conserva la inferencia por primera llamada y el comportamiento dinámico vigente.
- Una inferencia estática no altera silenciosamente el comportamiento del runtime ni la semántica de ejecución de `run`.
- Las reglas de conversión existentes (implícitas sin pérdida) se preservan.
- La convivencia es determinista y documentada (D0, D6).

### 3.5 Relación entre `analizar` y `run`

- No se comparte estado inferido entre el análisis semántico y el evaluador.
- El análisis estático y el runtime conservan responsabilidades independientes.
- Ambos respetan las mismas reglas de tipos, anotaciones y conversiones cuando son aplicables.
- **D6:** la inferencia estática de retornos de C.2 pertenece al análisis semántico; no debe modificar la ejecución, el evaluador ni el estado de inferencia dinámica.
- `analizar` puede disponer de información estática de retorno demostrable (C.2) sin inventar tipos, sin alterar la ejecución de `run` y sin introducir ni modificar diagnósticos existentes, sus códigos o su comportamiento vigente (**D0**).
- `run` conserva la inferencia por primera llamada y la semántica de ejecución vigente.

### 3.4 Política de diagnósticos

Se distinguen:
- Incompatibilidad demostrada estáticamente.
- Tipo desconocido.
- Incompatibilidad de anotación explícita.
- Incompatibilidad descubierta durante ejecución.

Se preservan los códigos E1xxx existentes y E1010. **D0 (política conservadora aprobada para C.2):** no introducir diagnósticos nuevos; no modificar diagnósticos existentes, sus códigos ni su comportamiento vigente; no alterar `run`; no modificar runtime ni `tipos_parametros_inferidos`. No se introducen nuevos códigos sin una decisión documentada.

## 4. Alternativas consideradas

- Reemplazo completo por inferencia estática: descartado por riesgo de regresiones y complejidad (recursión, llamadas indirectas).
- Inferencia bidireccional por restricciones: fuera del alcance actual (sin genéricos).
- Tipado gradual con anotaciones + inferencia local: recomendado.

## 5. Arquitectura propuesta

### 5.1 Representación interna de lambdas y closures

- Se mantienen inicialmente las representaciones internas actuales (`FuncionClausura`).
- La Etapa C unifica progresivamente las reglas semánticas y la validación de tipos (incluida C.2, sin representación interna compartida obligatoria).
- No se impone una representación interna compartida hasta que exista una justificación técnica y una evaluación de compatibilidad.

- Mantener `tipos_parametros_inferidos` en runtime (sin cambios) y preservar la inferencia por primera llamada.
- En `semantico.rs`, evolucionar la inferencia local por etapas: variables/expresiones (**Etapa B, completada**), unificación semántica conservadora de valores-función (**C.1, CLOSED**) e inferencia local acotada de retornos (**C.2, CLOSED**, implementada/verificada en alcance conservador) conforme a Opción A y D0/D3–D6, sin inferencia interprocedimental avanzada.
- Lambdas/clausuras: analizar tipos anotados o inferibles localmente cuando sean demostrables; conservar `TipoSem::Desconocido` ante incertidumbre; runtime sin cambios.
- `analizar`: no introduce nuevos diagnósticos ni altera los existentes (D0); la inferencia de C.2 es conservadora, pertenece al análisis semántico y no modifica la ejecución ni el estado dinámico (D6).

## 6. Consecuencias

Positivas: mejor DX, mayor coherencia, evolución gradual. Negativas: complejidad adicional, dos caminos (estático/dinámico) coexisten.

## 7. Plan de implementación (etapas)

**Etapa B (completada, CLOSED)** — Inferencia estática local de variables locales y expresiones (alcance limitado, verificable, sin interprocedimental avanzado), **sin inferencia estática de retornos**. Criterios: inferencia de variables locales y expresiones; anotaciones parciales validadas; pruebas de regresión y compatibilidad.

**Etapa C (COMPLETADA / CLOSED)** — Unificación semántica progresiva de funciones nombradas, lambdas y closures (sin compartir inmediatamente la misma representación interna en runtime), preservando el comportamiento existente.
- **C.0 (CLOSED):** cierre formal de diseño (Opción A conservadora), aprobado.
- **C.1 (CLOSED):** unificación semántica conservadora de firmas de valores-función.
- **C.2 (CLOSED, implementado/verificado y aprobado formalmente 2026-11-16, MISIÓN 06.18–06.21):** inferencia estática local y conservadora de retornos conocidos, demostrables y consistentes desde cuerpos/casos inequívocos. Múltiples retornos solo si demostrablemente compatibles. Incertidumbre/incompatibilidad/evidencia insuficiente → `TipoSem::Desconocido`. **D3:** no inferir automáticamente `Nulo` por caída sin `retornar` explícito; sin excepciones implícitas para `-> nulo` sin regla normativa inequívoca. **D4:** `retornar` sin expresión ≠ `retornar nulo` ≠ caída implícita; evidencia insuficiente para tipo conocido (conservar `TipoSem::Desconocido` cuando intervenga); sin alterar comportamiento vigente de ejecución ni E1010. **D5:** solo retornos anotados explícitamente y visibles en contexto semántico local; sin propagación desde cuerpos de otras funciones ni inferencia interprocedimental avanzada. **D6:** inferencia en análisis semántico; sin modificar ejecución/evaluador/estado de inferencia dinámica. **D0:** sin nuevos diagnósticos, sin cambios en diagnósticos/códigos/comportamiento existentes, sin alterar `run`, sin modificar runtime ni `tipos_parametros_inferidos` (ni VM/backends/HIR/MIR por esta capacidad). Capacidad implementada en alcance conservador aprobado (canal separado por identidad de declaración, consulta interna), sin consumidores productivos artificiales.

**Etapa D (NOT STARTED, investigación avanzada, sin implementación obligatoria y no prerrequisito de C.2)** — Investigación de inferencia **avanzada** de cuerpo/retornos más allá del alcance acotado de C.2 (p. ej. interprocedimental avanzada, flujos complejos u otras técnicas). Criterios: documento de viabilidad/riesgos y, si corresponde, propuesta de decisión/ADR futura.

## 8. Criterios de aceptación

- Las baterías de regresión vigentes siguen pasando; no se rompen programas existentes.
- `run` conserva su semántica de ejecución y comportamiento vigente; runtime y `tipos_parametros_inferidos` sin cambios (D0, D6).
- No se introducen nuevos diagnósticos ni se alteran diagnósticos/códigos/comportamiento existentes (D0).
- `analizar` aplica inferencia conservadora sin inventar tipos y sin compartir estado con el evaluador (D6).
- Documentación actualizada y alineada con C.0/Opción A y D0/D3–D6.

Criterios específicos por etapa:
- **Etapa B (completada):** inferencia local de variables/expresiones y anotaciones parciales verificables; sin inferencia estática de retornos; pruebas de regresión y compatibilidad.
- **Etapa C (C.1 cerrada):** funciones nombradas, lambdas y closures con análisis/firmas coherentes y conservadores; pruebas de regresión.
- **Etapa C.2 (CLOSED, aprobada formalmente 2026-11-16):** inferencia local acotada de retornos (conocida/demostrable/consistente), con D3/D4/D5/D0/D6 aplicados; sin inferencia interprocedimental avanzada; sin cambios runtime/`tipos_parametros_inferidos`/ejecución/diagnósticos existentes; sin regresiones observadas en validación reportada (06.18–06.21).
- **Etapa D (investigación):** investigación documentada avanzada más allá de C.2, sin implementación obligatoria y no prerrequisito de C.2.

## 9. Limitaciones fuera de alcance

- Genéricos, uniones, tipos algebraicos.
- Inferencia bidireccional completa.
- Inferencia interprocedimental avanzada (fuera del alcance acotado de C.2).
- Análisis de código inalcanzable como requisito adicional (salvo capacidades ya existentes y demostrables).
- Eliminación de la inferencia por primera llamada.
- Modificaciones de runtime, VM, backends, HIR/MIR o `tipos_parametros_inferidos` por C.2.
- Introducción o modificación de diagnósticos, códigos o comportamiento diagnóstico por C.2.
- Alteración de la semántica de ejecución de `run` o del estado de inferencia dinámica por C.2.