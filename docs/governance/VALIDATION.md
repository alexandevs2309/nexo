# Validación y reporte (gobernanza BASILISCO)

## Mecanismo de consulta (simple, no auto-aprobatorio)
- **status:** revisar `DEVELOPMENT_STATE.md` (tabla global + detalle de fase autorizada + bloqueos).
- **current:** valor de “Fase autorizada (current)” en `DEVELOPMENT_STATE.md`.
- **validate:** para la fase autorizada, contrastar cada **Prueba requerida** y **Criterio de aceptación** de `ROADMAP_MASTER.md` contra evidencia real ejecutada (comando, exit, conteos, rutas). Clasificar: `CUMPLE` / `NO CUMPLE` / `BLOQUEADA (causa)`. *Ignorados/bloqueados/no ejecutados no cuentan como aprobados.*
- **report:** usar la plantilla de abajo. `report` **no** aprueba fases.

Ayuda opcional: `docs/governance/check.sh status|current` (solo lectura).

## Plantilla de reporte de lote/cierre
```text
# Reporte — Fase NN (lote X)
- Fase autorizada (current): 
- Estado propuesto: PENDIENTE | EN PROGRESO | BLOQUEADA | (APROBADA solo con autorización humana explícita)
- Objetivo del lote:
- Alcance (archivos tocados / previstos):
- Cambios realizados (resumen factual):
- Validaciones ejecutadas:
  - <comando> → exit <n>, resultado <...>
- Evidencias (rutas/logs):
- Criterios de aceptación: [ID/desc] CUMPLE/NO CUMPLE/BLOQUEADA + evidencia
- Pruebas no ejecutadas/ignoradas/bloqueadas (causa):
- Cambios ajenos encontrados (preservados, no tocados):
- Riesgos/hallazgos:
- Solicitud: ¿autoriza humano pasar a Fase NN+1? (si aplica)
```

## Regla de aprobación
Una fase pasa a `APROBADA` **solo** cuando: (1) todos los criterios obligatorios cumplen con evidencia, (2) no hay fallos ocultos/skips silenciosos tratados como verde, (3) bloqueos externos están resueltos o formalmente aceptados por humano, y (4) hay **autorización humana explícita** registrada en `DEVELOPMENT_STATE.md`. El agente **nunca** auto-aprueba.
