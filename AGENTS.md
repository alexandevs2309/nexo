# AGENTS.md — BASILISCO (Gobernanza permanente)

Este repositorio usa un **roadmap secuencial obligatorio**. Todo agente (incluido OpenCode) debe cumplir estas reglas antes de cualquier acción de desarrollo.

## Reglas innegociables
1. Trabajar **exclusivamente sobre la rama `main`**. No crear ramas.
2. **No** realizar commits, push, publicaciones ni cambios de versión del lenguaje.
3. **No** eliminar, ignorar, debilitar ni alterar pruebas para conseguir resultados positivos.
4. **No** declarar una funcionalidad terminada sin **evidencia** de implementación y ejecución (comandos, archivos, resultados).
5. **No** saltar fases ni adelantar funcionalidades futuras. Solo se trabaja la **fase autorizada**.
6. **No** modificar archivos fuera del alcance de la tarea/fase autorizada.
7. Si una validación falla: **investigar y corregir** antes de avanzar. No ocultar errores conocidos.
8. Si una dependencia externa impide validar: declarar el estado **BLOQUEADA** (con causa exacta). Tests ignorados/bloqueados/no ejecutados **no cuentan como aprobados**.
9. **No** aprobar una fase basándose solo en el informe del agente. El cierre requiere informe de evidencia y **autorización humana explícita** para pasar a la siguiente fase.
10. **Preservar** todos los cambios existentes del repositorio, incluidos los no confirmados. Si hay cambios ajenos a la tarea, **no sobrescribirlos**.

## Sistema de control
- Skill obligatorio: `.opencode/skills/basilisco-governance/SKILL.md` (invocar antes de trabajar).
- Roadmap maestro (15 fases): `ROADMAP_MASTER.md`.
- Estado vivo (fase autorizada, bloqueos, evidencias): `DEVELOPMENT_STATE.md`.
- Detalle del mecanismo: `docs/governance/`.

## Flujo obligatorio por tarea
1. Leer `DEVELOPMENT_STATE.md` → identificar **fase autorizada** y estado.
2. Verificar que la tarea pertenece al alcance de esa fase y sus dependencias están satisfechas.
3. Ejecutar solo tareas permitidas; registrar evidencias (archivos, comandos, resultados).
4. Ejecutar validaciones requeridas por la fase. Si falla, corregir en la misma fase.
5. Emitir informe de cierre con evidencias. **No** marcar APROBADA ni iniciar otra fase sin autorización humana explícita.

## Estados de fase
`PENDIENTE` | `EN PROGRESO` | `BLOQUEADA` | `APROBADA`. Solo `APROBADA` cuando **todos** los criterios obligatorios estén satisfechos con evidencia.
