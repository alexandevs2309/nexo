# ADR-0018 — Evolución del sistema de inferencia de tipos de BASILISCO

**Estado:** Propuesto  
**Fecha:** 2026-11-01  
**Relacionado con:** ADR-0011 (tipos graduales), ADR-0017 (análisis semántico), ADR-0008 (funciones).

## 1. Contexto

BASILISCO 0.6.0 dispone de inferencia por primera llamada para funciones sin anotaciones (`tipos_parametros_inferidos` en `FuncionDef`, `evaluador.rs:481-485`). El análisis semántico (`semantico.rs`) marca parámetros sin anotación como `TipoSem::Desconocido`, lo que genera divergencia entre `bsk analizar` y `bsk run`. Las lambdas y closures (`FuncionClausura`) no comparten el mismo mecanismo. Existe una batería de regresión (18 pruebas en `tests/inferencia_tipos.rs`) que documenta el comportamiento actual.

## 2. Problema

La inferencia actual depende del orden de ejecución y no se realiza estáticamente. Esto impide detectar incompatibilidades antes de ejecutar y dificulta la unificación entre funciones nombradas, lambdas y closures. Sin embargo, eliminar la inferencia por primera llamada rompería programas existentes y la semántica de tipos graduales (ADR-0011).

## 3. Decisión

Diseñar una evolución incremental que:

1. Formalice la semántica de parámetros sin anotación (inferencia por primera llamada como comportamiento de runtime, documentado y preservado).
2. Introduzca inferencia estática local para funciones con anotaciones parciales, sin reemplazar el runtime.
3. Defina cómo se infieren retornos (primera llamada exitosa, como hoy).
4. Unifique progresivamente el análisis de funciones nombradas, lambdas y closures.
5. Mejore la coherencia entre `analizar` y `run` sin hacerlos idénticos.
6. Preserve compatibilidad con programas existentes y la batería de regresión.

Decisión clave: **no eliminar la inferencia por primera llamada**. Introducir una alternativa estática que conviva durante una transición documentada.

### 3.1 Inferencia estática local (definición)

Inferencia local significa inferir tipos únicamente dentro del ámbito léxico de una función o bloque, sin interprocedimental avanzado:

- Inferencia de expresiones: determinar el tipo de una expresión a partir de sus operandos y literales.
- Inferencia de parámetros parcialmente anotados: si un parámetro tiene anotación, validar argumentos; si no, mantener `Desconocido`.
- Inferencia de variables locales: `sea`/`dato` con inicializador tipado infiere el tipo de la variable.
- Inferencia de retornos: si el tipo de retorno no está anotado, inferirlo a partir de las expresiones `retornar` dentro del cuerpo (cuando sea inequívoco).

La Etapa B tendrá alcance limitado: inferencia de variables locales y expresiones, sin inferencia interprocedimental avanzada ni propagación entre funciones.

La Etapa B excluye expresamente la inferencia estática de retornos. El comportamiento actual de inferencia de retorno en runtime se mantiene. Su análisis estático se reserva para una etapa posterior, con diseño y pruebas específicas.

### 3.2 Reglas de evidencia

- Las anotaciones explícitas tienen prioridad absoluta.
- Solo se infieren tipos demostrables a partir del código fuente.
- La incertidumbre se representa mediante `TipoSem::Desconocido`.
- No se inventan tipos cuando falta información.
- Las incompatibilidades solo se reportan cuando están demostradas estáticamente.

### 3.3 Convivencia estática y dinámica

- El análisis estático no modifica `tipos_parametros_inferidos`.
- El evaluador conserva inicialmente la inferencia por primera llamada.
- Una inferencia estática no altera silenciosamente el comportamiento del runtime.
- Las reglas de conversión existentes (implícitas sin pérdida) se preservan.
- La convivencia es determinista y documentada.

### 3.5 Relación entre `analizar` y `run`

- No se comparte estado inferido entre el análisis semántico y el evaluador.
- El análisis estático y el runtime conservan responsabilidades independientes.
- Ambos respetan las mismas reglas de tipos, anotaciones y conversiones cuando son aplicables.
- `analizar` puede detectar incompatibilidades demostrables, pero no inventa tipos ni altera el comportamiento de ejecución.
- `run` conserva inicialmente la inferencia por primera llamada.

### 3.4 Política de diagnósticos

Se distinguen:
- Incompatibilidad demostrada estáticamente.
- Tipo desconocido.
- Incompatibilidad de anotación explícita.
- Incompatibilidad descubierta durante ejecución.

Se preservan los códigos E1xxx existentes y E1010. No se introducen nuevos códigos sin una decisión documentada.

## 4. Alternativas consideradas

- Reemplazo completo por inferencia estática: descartado por riesgo de regresiones y complejidad (recursión, llamadas indirectas).
- Inferencia bidireccional por restricciones: fuera del alcance actual (sin genéricos).
- Tipado gradual con anotaciones + inferencia local: recomendado.

## 5. Arquitectura propuesta

### 5.1 Representación interna de lambdas y closures

- Se mantienen inicialmente las representaciones internas actuales (`FuncionClausura`).
- La Etapa C unifica progresivamente las reglas semánticas y la validación de tipos.
- No se impone una representación interna compartida hasta que exista una justificación técnica y una evaluación de compatibilidad.

- Mantener `tipos_parametros_inferidos` en runtime (sin cambios).
- En `semantico.rs`, añadir un pase opcional de inferencia local para funciones con anotaciones parciales (fase posterior).
- Lambdas/clausuras: analizar tipos de parámetros anotados estáticamente; inferir el resto en runtime (como hoy).
- `analizar`: reportar diagnósticos estáticos adicionales sin bloquear `run` por inferencias no comprobables.

## 6. Consecuencias

Positivas: mejor DX, mayor coherencia, evolución gradual. Negativas: complejidad adicional, dos caminos (estático/dinámico) coexisten.

## 7. Plan de implementación (etapas)

Etapa B — Inferencia estática local para funciones anotadas (alcance limitado, verificable, sin interprocedimental avanzado). Criterios: inferencia de variables locales y expresiones; anotaciones parciales validadas; pruebas de regresión y compatibilidad.

Etapa C — Unificación semántica y progresiva de funciones nombradas, lambdas y closures. No implica compartir inmediatamente la misma representación interna en el runtime; la unificación es semántica, preservando el comportamiento existente. Criterios: coherencia de análisis entre los tres; pruebas de regresión; compatibilidad con closures y captura.

Etapa D — Investigación de inferencia estática del cuerpo, sin implementación obligatoria. Criterios: documento de viabilidad y riesgos.

## 8. Criterios de aceptación

- La batería de regresión (18 tests) sigue pasando.
- No se rompen programas existentes.
- `analizar` y `run` mantienen su comportamiento actual.
- Documentación actualizada.

Criterios específicos por etapa:
- Etapa B: inferencia local y anotaciones parciales verificables; pruebas de regresión y compatibilidad.
- Etapa C: funciones nombradas, lambdas y closures con análisis coherente; pruebas de regresión.
- Etapa D: investigación documentada, sin implementación obligatoria.

## 9. Limitaciones fuera de alcance

- Genéricos, uniones, tipos algebraicos.
- Inferencia bidireccional completa.
- Análisis de código inalcanzable.
- Eliminación de la inferencia por primera llamada.