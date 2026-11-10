# Gobernanza BASILISCO (docs/governance)

Sistema de control para desarrollo secuencial por fases (Fase 00–14). **Ninguna fase se aprueba automáticamente**; requiere evidencia y autorización humana explícita.

## Componentes
| Archivo | Rol |
|---|---|
| `AGENTS.md` | Reglas permanentes (innegociables). |
| `.opencode/skills/basilisco-governance/SKILL.md` | Procedimiento obligatorio del agente (skill OpenCode). |
| `ROADMAP_MASTER.md` | 15 fases con objetivo, alcance, dependencias, tareas, archivos previstos, pruebas, criterios de aceptación, evidencias, estado. |
| `DEVELOPMENT_STATE.md` | Estado vivo: `current` (fase autorizada), estados, bloqueos, evidencias, historial. **Fuente de verdad de la fase autorizada.** |
| `docs/governance/VALIDATION.md` | Plantilla de reporte y reglas de validación. |
| `docs/governance/check.sh` | Ayuda de consulta **solo lectura** (status/current/report pointers). No valida compilador ni aprueba fases. |

## Principios
- **Secuencial:** no iniciar fase posterior sin dependencias + autorización humana tras informe de cierre.
- **Evidencia:** comandos, resultados, archivos; diferenciar implementado/probado/funcional.
- **Honestidad:** ignorados/bloqueados/no ejecutados ≠ aprobados; fallos/flakiness/skips silenciosos deben reportarse.
- **Preservación:** no sobrescribir cambios ajenos; rama `main` única.

## Operación
1. Invocar skill `basilisco-governance` (ver `SKILL.md`).
2. Consultar `status/current` en `DEVELOPMENT_STATE.md`.
3. Trabajar solo la fase autorizada; validar con criterios de `ROADMAP_MASTER.md`.
4. Emitir `report` (plantilla) y esperar autorización humana. No usar mecanismos de auto-aprobación.
