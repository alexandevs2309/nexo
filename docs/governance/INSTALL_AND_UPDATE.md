# Instalación y mantenimiento

> **Nota de integración (este repositorio):** `DEVELOPMENT_STATE.md` y `ROADMAP_MASTER.md` son fuentes de verdad para la fase autorizada/estado/evidencias del roadmap secuencial. Este documento complementa la gobernanza de tareas/registros y debe mantenerse coherente con ellos. No asumir estados "UNKNOWN" como estado real sin contrastar con el repositorio.

1. Clona BASILISCO y revisa `git status --short --branch`.
2. Extrae el paquete en la raíz del clon.
3. Si existen nombres coincidentes, compara y fusiona manualmente.
4. Comprueba las carpetas `.agents/skills`, `.opencode/skills`, `.opencode/agents`, `.opencode/commands` y `docs/governance`.
5. Ejecuta el bootstrap de solo lectura.

Mantén sincronizadas las skills espejo. Revisa compatibilidad de frontmatter con la versión instalada de cada agente. No añadas hooks automáticos sin revisar su seguridad.

- [ ] AGENTS.md instalado.
- [ ] Skills Codex presentes.
- [ ] Skills OpenCode presentes.
- [ ] Agentes OpenCode presentes.
- [ ] Comandos OpenCode presentes.
- [ ] Gobierno presente.
- [ ] Bootstrap ejecutado y revisado.
