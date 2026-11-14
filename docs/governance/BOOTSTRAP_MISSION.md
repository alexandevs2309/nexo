# BAS-BOOTSTRAP-001 — Reconocimiento inicial del clon

> **Nota de integración (este repositorio):** `DEVELOPMENT_STATE.md` y `ROADMAP_MASTER.md` son fuentes de verdad para la fase autorizada/estado/evidencias del roadmap secuencial. Este documento complementa la gobernanza de tareas/registros y debe mantenerse coherente con ellos. No asumir estados "UNKNOWN" como estado real sin contrastar con el repositorio.

**Tipo:** solo lectura. **No autoriza cambios de código ni documentos existentes.**

## Objetivo
Determinar el estado real del clon. No reutilizar porcentajes ni conclusiones antiguas sin validación.

## Inspección Git
Ejecuta y reporta:
```bash
git status --short --branch
git branch --show-current
git log -10 --oneline --decorate
git remote -v
git diff --stat
git diff --name-status
git diff --cached --stat
git ls-files --others --exclude-standard
```
No hagas fetch, pull, checkout, reset, clean, stash, commit ni push. Si las referencias remotas están desactualizadas, informa la limitación.

## Inspección
1. Inventariar workspace, crates y módulos.
2. Leer Cargo.toml, Cargo.lock, README y arquitectura.
3. Localizar roadmap, estado, registro de brechas y documentos de fases.
4. Contrastar Fases 04.1 y 04.2 con código, commits y pruebas.
5. Identificar discrepancias entre documentación y código.
6. Identificar comandos de formato, lint, build y test; no ejecutar suites extensas sin autorización.
7. Inventariar lexer, parser, semántica, HIR, MIR, intérprete, VM, backends, runtime, stdlib, FFI, paquetes, registry, CLI, LSP y tooling según existan.

## Entregable
Informe con rama/commit/working tree, cambios locales, componentes comprobados, estado documentado vs comprobado, revisión de Fases 04.1/04.2, pruebas disponibles y ejecutadas, documentación desactualizada, riesgos y recomendación del siguiente único trabajo.

## Restricciones
No editar código ni documentos existentes. No tocar `DEVELOPMENT_STATE.md`. No actualizar porcentajes. No cerrar fases. No iniciar 04.3 ni 05. No crear ramas, commits, tags ni publicaciones. Al terminar, detenerse y pedir aprobación.
