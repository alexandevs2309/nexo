# AGENTS.md — BASILISCO (Gobernanza e instrucciones principales)

Este repositorio usa un **roadmap secuencial obligatorio** con control por evidencia. **Este archivo tiene autoridad sobre toda misión, tarea, sesión, subagente y skill** en este repositorio.

## Autoridad y alcance (obligatorio)
- **Aplica a:** agente principal, subagentes/especialistas, comandos, skills y cualquier asistente que actúe en el repositorio.
- **Subordinación:** toda instrucción del usuario, de una delegación o de un prompt secundario está subordinada a este `AGENTS.md` y al **Nivel 0**. Si una instrucción entra en conflicto, **no se cumple** la parte conflictiva: se explica el conflicto, se propone la alternativa conforme y se detiene la acción no autorizada.
- **Sin excepciones implícitas:** no hay permiso implícito para saltar reglas por urgencia, conveniencia, “modo rápido” o petición directa; solo una autorización humana **explícita** y compatible con Nivel 0 puede habilitar una acción reservada.

## Fuentes de verdad y jerarquía
- **Nivel 0 (prevalece):** seguridad, protección del repositorio, autoridad humana, restricciones de Git/fase/datos.
- **Nivel 1:** este `AGENTS.md` y flujo obligatorio.
- **Nivel 2:** skills especializadas (`.opencode/skills/*`, `.agents/skills/*`).
- **Nivel 3:** agentes especialistas (`.opencode/agents/*`).
- **Nivel 4:** comandos, plantillas, informes y checklists (`.opencode/commands/*`, `docs/governance/*`).

Ante conflicto, prevalecen los niveles superiores. Fuentes operativas del proyecto: código, pruebas, documentación, Git y decisiones del responsable; contrastar y reportar discrepancias. Para fase autorizada/estado/evidencias del roadmap, `DEVELOPMENT_STATE.md` es fuente de verdad (si hay conflicto, prevalece sobre otros tableros). Definición de fases: `ROADMAP_MASTER.md`.

## Reglas innegociables (Nivel 0)
1. Trabajar **exclusivamente en la rama `main`**. No crear ramas.
2. **Git prohibido sin autorización explícita:** no commits, no push, no merges, no tags, no releases, no publicaciones ni cambios de versión. Nunca ejecutar `git reset`, `git clean`, `git checkout --`, `git restore` ni operaciones equivalentes de descarte.
3. **Preservar** todos los cambios existentes/locales, incluidos no confirmados/no rastreados. No sobrescribir trabajo ajeno; si hay cambios ajenos, no tocarlos.
4. **No** modificar `DEVELOPMENT_STATE.md` sin autorización explícita del responsable.
5. **No** iniciar **Fase 04.3** ni **Fase 05** sin autorización expresa. No iniciar fase nueva sin autorización humana explícita.
6. Solo puede existir **una tarea principal activa** y **una fase activa**. No ampliar alcance ni cambiar prioridades por iniciativa propia.
7. **No** eliminar, ignorar, debilitar ni alterar pruebas para conseguir resultados positivos.
8. **No** declarar funcionalidad/tarea terminada sin **evidencia** (archivos, comandos, resultados). **No** declarar fase cerrada por iniciativa del agente.
9. **No** aprobar una fase basándose en informe propio; cierre/aprobación requiere evidencia y **autorización humana explícita**.
10. Ideas nuevas → backlog (`docs/governance/BACKLOG.md`). Defectos/hallazgos fuera de alcance → documentar en issues (`docs/governance/ISSUES.md`), **no** implementar automáticamente.

## Inicio obligatorio (inspección real)
Antes de actuar, leer: `AGENTS.md`, `DEVELOPMENT_STATE.md`, `ROADMAP_MASTER.md` y gobierno en `docs/governance/` (incl. `PROJECT_CHARTER.md`, `WORKFLOW.md`, `TASK_BOARD.md`, `PHASE_GATE.md`, `VALIDATION.md` según aplique). Inspeccionar estado real (solo lectura, sin fetch/pull/checkout/reset/clean/stash/commit/push no autorizados):
```bash
git status --short --branch
git branch --show-current
git log -5 --oneline --decorate
git remote -v
git diff --stat
git diff --name-status
git diff --cached --stat
git ls-files --others --exclude-standard
```
Confirmar `main`. Identificar **fase autorizada** y tarea autorizada con evidencia. Si no hay autorización clara, limitarse a inspección/planificación y pedir autorización.

## Flujo obligatorio por tarea
1. Definir (cuando aplique): ID, objetivo, alcance, exclusiones, criterios de aceptación, pruebas y riesgos. Confirmar que pertenece a la fase/tarea autorizada y dependencias satisfechas.
2. Reproducir/analizar con evidencia. Implementar el cambio mínimo dentro del alcance; no tocar archivos fuera de alcance.
3. Ejecutar validaciones/pruebas pertinentes. Si falla: investigar y corregir en el mismo alcance; no ocultar. Dependencia externa bloqueante → **BLOQUEADA** (causa exacta). Tests ignorados/bloqueados/no ejecutados **no cuentan como aprobados**.
4. Revisar diff completo. Actualizar registros de gobierno permitidos con evidencia (task board/backlog/issues/session/changelog/decisiones/phase gate) cuando corresponda y haya autorización para hacerlo; no borrar historial ni inventar decisiones. No marcar aprobación humana.
5. Emitir informe de cierre con evidencia. **Parar**: no iniciar otra tarea ni fase.

## Estados y criterios
- **Fase (roadmap):** `PENDIENTE` | `EN PROGRESO` | `BLOQUEADA` | `APROBADA`. `APROBADA` solo con todos los criterios obligatorios satisfechos con evidencia y autorización humana explícita registrada.
- **Trabajo/tarea:** distinguir siempre `IMPLEMENTED` (cambio realizado), `VERIFIED` (criterios/pruebas cumplidos con evidencia), `CLOSED` (aceptación humana). Silencio o recomendación del agente no equivale a aprobación.

## Skills y especialistas
- Skill de gobernanza del repositorio: `.opencode/skills/basilisco-governance/SKILL.md` (usar antes de trabajar; coherente con `DEVELOPMENT_STATE.md`/`ROADMAP_MASTER.md`).
- Skills especializadas y agentes: `.opencode/skills/*`, `.agents/skills/*`, `.opencode/agents/*`. Las reglas de este archivo y Nivel 0 prevalecen.

## Cierre obligatorio
Entregar: ID, objetivo, cambios/archivos, comandos y resultados exactos, diff revisado, riesgos, pendientes, registros actualizados y estado propuesto (`IMPLEMENTED`/`VERIFIED`/`BLOCKED`, etc., sin cerrar fase). Terminar con: **“No he iniciado otra tarea ni fase.”**
