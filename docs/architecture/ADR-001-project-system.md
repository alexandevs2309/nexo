# ADR-001: Sistema de proyectos, plantillas y ejemplos como componente oficial del ecosistema BASILISCO

**Fecha:** 2026-11-05  
**Estado:** APROBADO  
**Decisores:** Proyecto BASILISCO  
**Relacionado con:** GAP-PS-01 a GAP-PS-07 (registro de brechas), Etapa 2+ del ROADMAP

---

## Contexto

BASILISCO v0.7.0 tiene un comando `bsk nuevo proyecto <nombre>` que genera una estructura mínima fija (5 archivos). No hay sistema de plantillas, no hay ejemplos de aplicaciones completas mantenidos, y no hay convenciones oficiales documentadas sobre cómo organizar un proyecto BASILISCO.

Esto crea varios problemas:

1. **Barrera de entrada:** un desarrollador nuevo no sabe cómo organizar su código más allá del archivo único
2. **Dispersión:** cada desarrollador inventa su propia estructura, dificultando la colaboración
3. **Promesas incumplidas:** documentos anteriores mencionaban "plantillas" y "ejemplos" sin que existieran
4. **Deuda técnica invisible:** la falta de ejemplos ejecutables oculta las limitaciones reales del lenguaje

Adicionalmente, la visión del proyecto BASILISCO siempre ha incluido una experiencia de desarrollo comparable a Cargo/Rust, Go y Django CLI. Esa visión no estaba documentada formalmente dentro del repositorio y dependía del historial de conversaciones con agentes AI.

---

## Problema

¿Cómo convertir BASILISCO de "un lenguaje con CLI" en "un ecosistema de desarrollo"?

La respuesta técnica es: mediante un sistema de proyectos con convenciones oficiales, plantillas que generen aplicaciones funcionales, y ejemplos completos ejecutables.

El problema inmediato es: nada de esto está implementado ni documentado con suficiente especificidad para que un desarrollador pueda contribuir o un agente AI pueda ejecutarlo.

---

## Decisión

Se adopta el sistema de proyectos, plantillas y ejemplos como un **componente oficial de primer nivel del ecosistema BASILISCO**, con las siguientes implicaciones:

1. **Documentación permanente:** los documentos en `docs/` (PROJECT_SYSTEM.md, TEMPLATE_SYSTEM.md, OFFICIAL_EXAMPLES.md, PROJECT_CONVENTIONS.md, VISION.md) son documentos oficiales del proyecto, no notas temporales.

2. **Categoría propia en el registro de brechas:** las deudas de este subsistema tienen identificadores GAP-PS-xx y se gestionan igual que las deudas del compilador.

3. **Convenciones obligatorias:** las convenciones en PROJECT_CONVENTIONS.md son el estándar oficial. Las plantillas las implementan. Las herramientas futuras (LSP, linter) las refuerzan.

4. **Honestidad sobre el estado:** la documentación distingue explícitamente entre IMPLEMENTADO, EXPERIMENTAL y FUTURO. Nada se marca como completo si no lo está.

5. **Ejemplos como prueba de madurez:** los ejemplos oficiales son el criterio de madurez del lenguaje. Un lenguaje que no puede ejecutar sus propios ejemplos no está listo para producción.

---

## Consecuencias

### Positivas

- Cualquier desarrollador puede entender cómo organizar un proyecto BASILISCO desde el primer día
- Los agentes AI que examinen el repositorio encontrarán la visión del ecosistema sin necesidad de conversaciones previas
- El sistema de plantillas tiene un diseño formal antes de implementarse, evitando decisiones apresuradas
- Los ejemplos existentes (inventario) tienen un camino documentado hacia su arquitectura final

### Negativas / Riesgos

- Los documentos deben mantenerse actualizados cuando el lenguaje evolucione (riesgo de desincronización)
- Las convenciones oficiales pueden ser restrictivas para casos de uso que no anticipamos
- El diseño del sistema de plantillas puede necesitar revisión cuando se implemente

### Neutrales

- Esta decisión no impone un timeline de implementación; solo formaliza la dirección y los criterios de aceptación
- La implementación técnica queda en el ROADMAP, no en este ADR

---

## Evolución prevista

| Fase | Capacidad | Dependencias |
|---|---|---|
| Actual (0.7.0) | Estructura mínima fija, sin plantillas | ninguna |
| Corto plazo | Plantillas `consola` y `biblioteca` integradas en `bsk` | Unificar parser TOML |
| Medio plazo | Sistema de variables en plantillas, validación CI | Plantillas básicas |
| Largo plazo | Plantillas del registro (comunitarias) | Sistema BPK estable |

---

## Alternativas consideradas

### Alternativa 1: No hacer nada

Mantener el sistema actual (estructura mínima fija) sin documentar la visión.

**Rechazada porque:** la visión existe y guía decisiones de diseño. No documentarla crea dependencia del historial de conversaciones y dificulta contribuciones.

### Alternativa 2: Implementar el sistema completo ahora

Implementar plantillas, ejemplos y convenciones en una sola iteración.

**Rechazada porque:** el lenguaje no tiene todavía las capacidades que los ejemplos más complejos requieren (tipos producto, módulos jerárquicos). Implementar los ejemplos ahora significaría hacerlos incompletos o usar workarounds.

### Alternativa 3: Separar en un repositorio externo

Tener el sistema de plantillas y ejemplos en un repositorio separado.

**Rechazada porque:** crea fragmentación y dificulta mantener la compatibilidad con el compilador. Los ejemplos oficiales deben vivir junto al lenguaje.

---

## Criterios de éxito

Esta decisión se considera exitosa cuando:

1. `bsk nuevo proyecto --plantilla consola` genera una aplicación ejecutable
2. Los ejemplos de consola y notas están en `ejemplos/` y pasan un test de humo en CI
3. El formateador y el LSP (cuando exista) siguen las convenciones de PROJECT_CONVENTIONS.md
4. Un desarrollador nuevo puede seguir PROJECT_SYSTEM.md y PROJECT_CONVENTIONS.md sin ayuda adicional
