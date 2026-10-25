# ADR-0015 — Aprobación del perfil de conformidad BASILISCO v0.4

- Estado: **Aprobado**
- Fecha: 2026-10-25
- Responsable: responsable del proyecto BASILISCO
- Relacionado: ADR-0005 a ADR-0014; BASILISCO Core 0.3; `10-especificacion/perfil-de-conformidad-v0.4.md`.

## Contexto y requisitos

BASILISCO Core 0.3 es una propuesta inicial y no define exhaustivamente funciones, texto ni colecciones presentes en el prototipo. Se preparó un perfil v0.4 reducido para identificar una superficie verificable sin afirmar que el lenguaje completo esté terminado.

La revisión técnica confirmó que la semántica de `real` coincide con ADR-0005 y que el perfil acotado puede aprobarse. También se verificó la ejecución satisfactoria del workflow **CI** para el commit `0914b4e` en `main` (GitHub Actions run [40225683156](https://github.com/alexandevs2309/nexo/actions/runs/40225683156)): formato, Clippy, compilación y pruebas.

## Opciones consideradas

### O1 — Mantener el perfil como propuesta

Evita fijar el contrato, pero deja el alcance v0.4 ambiguo pese a la revisión técnica favorable y la evidencia de CI.

### O2 — Aprobar el perfil v0.4 acotado

Fija el subconjunto y sus exclusiones, manteniendo separadas la conformidad de implementación, la estabilidad y la publicación.

### O3 — Declarar estable el lenguaje completo

Excedería la evidencia: módulos, paquetes, biblioteca estándar extensa, concurrencia, soporte de plataformas y otras capacidades siguen fuera del perfil o pendientes.

## Decisión

Se aprueba `10-especificacion/perfil-de-conformidad-v0.4.md` como **contrato del perfil acotado BASILISCO v0.4**. La aprobación cubre únicamente el alcance enumerado en ese documento; no convierte en garantía las extensiones que allí se excluyen.

Esta decisión:

- no sustituye ni amplía BASILISCO Core 0.3;
- no declara estable el lenguaje completo;
- no declara publicada o lista para distribución la versión v0.4;
- no aprueba por sí sola un cambio de versión del paquete, una licencia, marca, instalador o matriz de plataformas.

La evidencia de CI indicada en este ADR corresponde al commit `0914b4e`. Cualquier cambio posterior que afecte al código debe volver a pasar CI antes de usar ese resultado como evidencia de la revisión final de release.

## Consecuencias y pendientes

- El perfil v0.4 cambia de estado **PROPUESTO** a **APROBADO** como contrato acotado.
- La versión Cargo continúa en `0.1.0`; la decisión de versión del paquete se toma separadamente para la publicación.
- Siguen pendientes la política/plataformas de distribución, instaladores publicables, y las capacidades fuera del perfil.
- El lenguaje completo no recibe un porcentaje de completitud mediante esta decisión.

## Condiciones de revisión

Reabrir este ADR si aparece una contradicción normativa, un defecto que cambie el comportamiento exigido por el perfil o evidencia de usuario incompatible con sus reglas. Las ampliaciones requieren pruebas y revisión de compatibilidad antes de incluirse en una versión posterior.
