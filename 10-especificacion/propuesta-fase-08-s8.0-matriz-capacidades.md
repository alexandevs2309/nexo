# Propuesta S8.0 R3 — Matriz de capacidades por motor y capa Fase 08

> **Estado:** APROBADA POR REVISIÓN HUMANA (2026-11-22) — S8.0 spec/contrato aprobado; **implementación de Fase 08 / S8.1 NO autorizada**  
> **Fecha consolidación R3:** 2026-11-22  
> **Fecha de aprobación humana:** 2026-11-22  
> **Fuente principal:** `propuesta-fase-08-s8.0-especificacion-normativa.md`  
> **Regla:** TW↔VM obligatorio en subconjunto aprobado. Nativo/WASM por capacidad declarada con evidencia. Sin evidencia = NO SOPORTADO/LIMITADO explícito. `NO SOPORTADO` en ejecución/codegen usa BAS-7124. Contenido normativo sin cambios; solo estado documental actualizado.

## 1. Capas

| Capa | Significado |
|---|---|
| Semántica | Parser/AST/semántica/tipos/visibilidad/diagnósticos |
| HIR/MIR | Lowering, representación IR, verificadores |
| Codegen | Generación de código (nativo/WASM) |
| Ejecución | Runtime TW/VM/Nativo/WASM |

## 2. Leyenda

- **OBL**: obligatorio en v1 para la capacidad aprobada.
- **LIM**: soporte parcial declarado con subconjunto y pruebas.
- **NO**: no soportado; debe diagnosticarse/rechazarse limpiamente.
- **DIF**: diferido a v1 futura/G4-ext.

## 3. Matriz

| Capacidad | Semántica | HIR/MIR | TW | VM | Nativo codegen | Nativo ejec. | WASM codegen | WASM ejec. | Evidencia |
|---|---|---|---|---|---|---|---|---|---|
| Declaración `estructura` | OBL | OBL | OBL | OBL | NO | NO | NO | NO | parse/semántica/registro tipo |
| Construcción struct | OBL | OBL | OBL | OBL | LIM | LIM | NO | NO | conformidad + diferencial TW↔VM |
| Acceso campo struct | OBL | OBL | OBL | OBL | LIM | LIM | NO | NO | visibilidad/tipo + diferencial |
| Asignación campo struct directo (`id.campo`, C2) | OBL | OBL | OBL | OBL | NO | NO | NO | NO | mutabilidad/copia/atomicidad + diferencial |
| Asignación lvalue encadenada (`id.a.b`, `id[i].a`, etc.) | NO v1 | NO | NO | NO | NO | NO | NO | NO | rechazo explícito forma no soportada |
| Declaración `enumeracion` | OBL | OBL | OBL | OBL | NO | NO | NO | NO | registro tipo/variantes |
| Variante unitaria | OBL | OBL | OBL | OBL | LIM | LIM | LIM | LIM | construcción/match + diferencial |
| Variante payload posicional | OBL | OBL | OBL | OBL | NO | NO | NO | NO | aridad/tipo + diferencial |
| `coincidir` exhaustivo patrones v1 | OBL | OBL | OBL | OBL | NO | NO | NO | NO | exhaustividad/reachability/binding |
| Patrón struct | OBL | OBL | OBL | OBL | NO | NO | NO | NO | destructuring + diferencial |
| Métodos inherentes instancia/asociado | OBL | OBL | OBL | OBL | NO | NO | NO | NO | resolución estática |
| `rasgo` + `implementar para` mínimo | OBL | OBL | OBL | OBL | NO | NO | NO | NO | firma/impl/coherencia |
| Igualdad nominal `==/!=` | NO (BAS-7126) | NO | NO | NO | NO | NO | NO | NO | negativo diagnóstico |
| Genéricos nominales | NO (BAS-7125) | NO | NO | NO | NO | NO | NO | NO | negativo diagnóstico |
| Traits avanzados (dyn/default/super/bounds/assoc) | DIF/NO | DIF/NO | NO | NO | NO | NO | NO | NO | rechazo/deferral |
| Recursividad indirecta lista/arreglo | OBL | OBL | OBL | OBL | LIM | LIM | NO | NO | layout/ciclo |
| Visibilidad `publico` nominal/campo/método | OBL | OBL | OBL | OBL | N/A | N/A | N/A | N/A | BAS-7128 negativos |

## 4. Notas

- "Declaración" puede ser semántica/HIR sin codegen ejecutable. No marcar codegen soportado por existencia de declaración.
- Nativo/WASM: cualquier intento en capacidad NO debe emitir BAS-7124 y no producir artefacto inválido.
- TW↔VM: toda capacidad OBL requiere diferencial de resultado o de diagnóstico normalizado.