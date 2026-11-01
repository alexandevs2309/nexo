# PLAN MAESTRO — IMPLEMENTACIÓN ADR-0018

## 1. Objetivo y alcance

Planificar la implementación de ADR-0018 (evolución del sistema de inferencia de tipos) en tres etapas (B, C, D), sin modificar código. Solo diseño y planificación.

## 2. Arquitectura actual relevante

- `evaluador.rs`: `FuncionDef` con `tipos_parametros_inferidos`, `tipo_retorno`; inferencia por primera llamada en `llamar_funcion_por_nombre_resultado`.
- `semantico.rs`: `TipoSem`, `FuncionSig`, `analizar_expresion`, `registrar_funciones`; parámetros sin anotación → `Desconocido`.
- `simbodo.rs`: tabla de símbodos, ámbitos.
- `valor.rs`: `Valor::nombre_tipo`.
- Tests: `tests/inferencia_tipos.rs` (18 regresiones), `tests/lambdas.rs`, `tests/closures.rs`.

## 3. Arquitectura objetivo

Inferencia estática local progresiva que conviva con la inferencia por primera llamada, sin modificar el runtime inicialmente. Anotaciones explícitas prioritarias; incertidumbre como `Desconocido`.

## 4. Etapa B — Inferencia estática local

**Objetivo:** inferir tipos de variables locales y expresiones estáticamente; validar anotaciones parciales; reportar incompatibilidades demostradas.

**Archivos:** `semantico.rs` (principal), `simbodo.rs` (consulta), `parser.rs` (solo lectura).

**Funciones a reutilizar:** `analizar_expresion`, `analizar_sentencia`, `buscar_variable`, `existe_en_ambito_actual`, `compatible`, `tipo_comun`, `tipo_numerico_resultado`.

**Implementación prevista:**
- En `analizar_sentencia` para `Declaracion`: si el inicializador tiene tipo conocido, propagar a la variable.
- En `analizar_expresion` para `Suma`/etc.: ya determina `TipoSem`; propagar a variables asignadas.
- Para parámetros con anotación: validar argumentos en llamadas (ya parcialmente).

**Exclusiones:** inferencia interprocedimental, inferencia estática de retornos, cambios al evaluador, modificación de la inferencia por primera llamada.

**Complejidad:** media.

**Riesgos:** falsos positivos por inferencia incorrecta; mitigar con `Desconocido` conservador.

**Pruebas:** unitarias en `semantico.rs` (tests existentes) + integración `analizar`.

**Criterios de aceptación:** inferencia local funciona para variables/expr; 18 regresiones pasan; `analizar` detecta incompatibilidades demostrables.

**Condición de reversión:** si se introducen falsos positivos en programas válidos.

## 5. Etapa C — Unificación semántica

**Objetivo:** unificar progresivamente el análisis de funciones nombradas, lambdas y closures (reglas semánticas y validación de tipos), sin imponer representación interna compartida.

**Archivos:** `semantico.rs`, `evaluador.rs` (solo lectura inicialmente), `valor.rs` (solo lectura).

**Dependencias:** Etapa B (inferencia local).

**Riesgos:** compatibilidad con closures y captura; mitigar con pruebas de regresión.

**Complejidad:** alta.

**Pruebas:** `tests/closures.rs`, `tests/lambdas.rs`, `tests/inferencia_tipos.rs`.

**Criterios de aceptación específicos:** validación de funciones nombradas, lambdas y closures mediante casos de captura léxica, llamadas indirectas y compatibilidad de firmas; pruebas en `tests/closures.rs`, `tests/lambdas.rs` y `tests/inferencia_tipos.rs`; sin regresiones.

## 6. Etapa D — Investigación avanzada

**Objetivo:** investigar inferencia de retornos, múltiples rutas, recursividad, flujo de control, interprocedimental, bidireccional.

**Salida:** documento de viabilidad y riesgos, no implementación obligatoria.

**Complejidad:** alta (investigación).

## 7. Dependencias entre etapas

B → C. C depende de B. D (investigación) no depende técnicamente de C ni de B; se recomienda realizarla después por orden de trabajo, pero la investigación puede ser independiente de la implementación.

## 8. Matriz de archivos y componentes

| Etapa | Archivos | Componentes |
|---|---|---|
| B | semantico.rs | analizar_sentencia, analizar_expresion, buscar_variable |
| C | semantico.rs, evaluador.rs (lectura) | FuncionSig, TipoSem, clausuras |
| D | (investigación) | flujo de control, retornos |

## 9. Matriz de riesgos

| Riesgo | Etapa | Mitigación |
|---|---|---|
| Falsos positivos | B | Conservador (Desconocido) |
| Regresiones en closures | C | Pruebas de regresión |
| Complejidad interprocedimental | D | Limitar a investigación |

## 10. Plan de pruebas

- Unitarias: `semantico.rs` tests existentes.
- Integración: `tests/inferencia_tipos.rs` (18).
- CLI: `analizar` vs `run`.
- Baseline: 113+ tests, 0 fallos, 2 ignorados.

## 11. Criterios de aceptación

- Cada etapa pasa validación completa.
- No se rompen programas existentes.
- Documentación actualizada.

## 12. Orden recomendado

B → C → D (investigación).

## 13. Decisiones que requieren ADR adicional

- Si la inferencia estática de retornos se implementa (Etapa D).
- Si se introduce una representación compartida para lambdas/closures.

## 14. Fuera de alcance

Genéricos, uniones, tipos algebraicos, inferencia bidireccional completa, análisis de código inalcanzable, eliminación de inferencia por primera llamada.

## 15. Checklist de cierre de cada etapa

- [ ] Código implementado (solo la etapa autorizada).
- [ ] Pruebas añadidas.
- [ ] `cargo test` OK.
- [ ] `cargo fmt --check` OK.
- [ ] `cargo clippy` OK.
- [ ] `cargo build --bins` OK.
- [ ] Documentación actualizada.
- [ ] Aprobación explícita para avanzar.