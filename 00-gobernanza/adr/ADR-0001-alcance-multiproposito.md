# ADR-0001 — Alcance de Nexo: lenguaje multipropósito

- Estado: Aprobado
- Fecha: 2026-10-20
- Responsable: propietario del proyecto (decisión explícita)
- Revisión prevista: al cerrar la Fase 2 (prototipo vertical), con evidencia de viabilidad

## Contexto y requisitos
El expediente 0.2 cubre web, datos, escritorio, móvil, IA, videojuegos y sistemas/bajo nivel.
El propio expediente identifica el «alcance inabarcable» como riesgo nº 1 (`08-plan/hoja-de-ruta-y-riesgos.md`)
y recomienda un núcleo pequeño (`02-lenguaje/alcance-del-nucleo.md`).
Se planteó al propietario reducir el alcance a un nicho para la v1.

## Opciones consideradas
- **A. Reducir a un nicho (educación + automatización CLI).** Ventajas: ciclo de validación rápido, competencia mínima. Riesgo: el propietario descarta el objetivo original de lenguaje multipropósito.
- **B. Mantener multipropósito (elegida).** Ventajas: coherente con la visión original y con `01-vision/objetivos-y-no-objetivos.md`. Riesgo: probabilidad histórica alta de fracaso por alcance; exige secuenciación estricta para no estancarse en documentación.

## Criterios y ponderación
Visión del propietario > velocidad de validación. La secuenciación se delega en la hoja de ruta existente, no en la reducción del objetivo.

## Evidencia
- Decisión explícita del propietario (2026-10-20): mantener multipropósito.
- No hay evidencia empírica a favor o en contra; el riesgo queda registrado, no resuelto.

## Decisión
Nexo se mantiene como lenguaje multipropósito. **Aprobado por aceptación explícita.**
Esta decisión no aprueba ninguna tecnología ni acelera fases: solo fija el objetivo.

## Consecuencias y migración
- Obligación de secuenciar: ninguna fase puede abrirse sin cerrar la anterior (Fases 0-7 de `08-plan`).
- Los ADR siguientes (anfitrión, ejecución, tipos, memoria) deben evaluarse con la lente multipropósito:
  elecciones que bloqueen futuras plataformas se rechazan; elecciones demasiado genéricas que impidan el prototipo también.
- Se mantiene el riesgo de alcance registrado; su mitigación es procesal (puertas de calidad de `07-validacion`), no de objetivo.

## Condiciones de revisión
Revisar si tras la Fase 2 no existe un prototipo vertical funcional, o si el propietario cambia de objetivo.
