# BASILISCO Agent System

Sistema especializado para gobernar el trabajo de agentes en BASILISCO.

## Compatibilidad
- Codex: `AGENTS.md` e instrucciones de skills en `.agents/skills/<nombre>/SKILL.md`.
- OpenCode: `AGENTS.md`, agentes en `.opencode/agents/` y skills en `.opencode/skills/<nombre>/SKILL.md`.

Las reglas esenciales están en `AGENTS.md`; las skills aportan procedimientos especializados. No dependen de que un proveedor cargue correctamente una skill.

## Instalación
1. Clona BASILISCO.
2. Revisa `git status --short --branch`.
3. Extrae este paquete en la raíz del clon.
4. Si ya existen archivos o directorios con los mismos nombres, compara y fusiona; nunca sobrescribas a ciegas.
5. Abre el repositorio desde Codex u OpenCode.
6. Ejecuta únicamente `docs/governance/BOOTSTRAP_MISSION.md` en modo de solo lectura.

## Autonomía controlada
El agente puede investigar, planificar, implementar una misión autorizada y ejecutar pruebas. No puede cambiar de fase, aprobar cierres, publicar, ni realizar operaciones Git reservadas.

El estado inicial de las fases es UNKNOWN / TO BE VERIFIED. Este paquete no certifica avances.
