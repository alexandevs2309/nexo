# Flujo de trabajo obligatorio

> **Nota de integración (este repositorio):** `DEVELOPMENT_STATE.md` y `ROADMAP_MASTER.md` son fuentes de verdad para la fase autorizada/estado/evidencias del roadmap secuencial. Este documento complementa la gobernanza de tareas/registros y debe mantenerse coherente con ellos. No asumir estados "UNKNOWN" como estado real sin contrastar con el repositorio.

Estados: `BACKLOG → READY → IN_PROGRESS → IMPLEMENTED → VERIFIED → CLOSED`. Alternativos: `BLOCKED`, `REOPENED`.

- IMPLEMENTED: cambio realizado.
- VERIFIED: criterios y pruebas cumplidos con evidencia.
- CLOSED: aceptación humana registrada.

Antes de implementar, presentar ID, fase, problema, objetivo, alcance, exclusiones, criterios de aceptación, pruebas y riesgos. Si no hay tarea autorizada, solo se permite inspección de solo lectura y pedir autorización.

Durante el trabajo, mantener una sola tarea activa, no ampliar alcance, registrar hallazgos ajenos y no cambiar de fase. Si aparece un bloqueo, documentarlo.

Al terminar, revisar diff, ejecutar pruebas, actualizar registros permitidos, presentar evidencia y esperar aceptación humana.

Una fase no se cierra sin inventario de objetivos, tareas verificadas, pruebas, regresiones, riesgos residuales, documentación reconciliada y aprobación humana explícita. Silencio o recomendación del agente no equivalen a aprobación.
