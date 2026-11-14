---
id: basilisco-governance
name: Basilisco Governance
description: Procedimiento obligatorio para trabajar en BASILISCO bajo roadmap secuencial (fases 00-14), control de alcance, evidencia y cierre con autorización humana. Usar antes de cualquier tarea de desarrollo, validación o reporte de fase.
---

# Basilisco Governance (procedimiento obligatorio)

## Cuándo usar
- Antes de **cualquier** cambio de código, prueba, config o tarea de desarrollo en BASILISCO.
- Cuando el usuario pida avanzar, validar, reportar estado o “continuar con la siguiente fase”.

## Fuentes de verdad (leer primero)
1. `AGENTS.md` — reglas innegociables.
2. `DEVELOPMENT_STATE.md` — **fase autorizada (`current`)**, estado, bloqueos, evidencias. *Si hay conflicto, `DEVELOPMENT_STATE.md` define la fase autorizada.*
3. `ROADMAP_MASTER.md` — definición de la fase (objetivo, alcance, dependencias, tareas, archivos previstos, pruebas, criterios de aceptación).
4. `docs/governance/` — mecanismo de control y validación.

## Procedimiento (cada lote)
1. **Identificar fase autorizada** en `DEVELOPMENT_STATE.md`. Confirmar dependencias satisfechas según `ROADMAP_MASTER.md`. Si la tarea pedida pertenece a otra fase → **detenerse** y reportar; no ejecutar.
2. **Delimitar alcance**: listar archivos previstos; no tocar archivos fuera de alcance; no sobrescribir cambios ajenos preexistentes.
3. **Ejecutar solo tareas de la fase**. Preservar pruebas existentes (no eliminar/ignorar/debilitar/alterar para forzar verde).
4. **Validar** con las pruebas/comandos requeridos por la fase. Capturar evidencia (comando, exit, contadores, rutas). Si falla → investigar y corregir dentro de la fase; no ocultar.
5. **Clasificar** resultados: pruebas pasadas/ fallidas / ignoradas-bloqueadas-no ejecutadas (estas **no** cuentan como aprobadas). Si depende de infra externa → **BLOQUEADA** con causa exacta.
6. **Reportar cierre**: objetivo, cambios (archivos), evidencias, criterios cumplidos/no cumplidos, riesgos, estado propuesto (PENDIENTE/EN PROGRESO/BLOQUEADA; **APROBADA solo con autorización humana explícita**). Actualizar `DEVELOPMENT_STATE.md` (evidencias/bloqueos/estado de trabajo) **sin** auto-aprobar ni cambiar de fase.
7. **Parar**: no iniciar fase posterior sin autorización humana explícita tras el informe.

## Restricciones duras
- Rama `main` única. Sin ramas/commits/push/publicaciones/cambios de versión.
- Sin adelantar fases. Sin aprobar por informe propio.
- Conservar trabajo ajeno/preexistente.

## Consultas rápidas (mecanismo)
- **status:** leer `DEVELOPMENT_STATE.md` (tabla resumen + detalle fase autorizada).
- **current:** extraer “Fase autorizada (current)” de `DEVELOPMENT_STATE.md`.
- **validate:** ejecutar/verificar los “Pruebas requeridas” y “Criterios de aceptación” de la fase en `ROADMAP_MASTER.md`, contrastando evidencia real; reportar cumple/no cumple/bloqueado.
- **report:** generar informe de lote/cierre con la plantilla de `docs/governance/VALIDATION.md`.

*No existe comando que apruebe automáticamente una fase.*
