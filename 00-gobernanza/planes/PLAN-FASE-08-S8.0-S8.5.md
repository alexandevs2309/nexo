# PLAN Fase 08 — S8.0 a S8.5 (Propuesta R3)

> **Estado:** APROBADA POR REVISIÓN HUMANA (2026-11-22) — S8.0 spec/contrato aprobado; **implementación de Fase 08 / S8.1 NO autorizada**  
> **Fecha consolidación R3:** 2026-11-22  
> **Fecha de aprobación humana:** 2026-11-22  
> **Fuente principal:** `10-especificacion/propuesta-fase-08-s8.0-especificacion-normativa.md`  
> **Subetapa actual:** S8.0 R3 APROBADA (spec/contrato). READY FOR S8.1 AUTHORIZATION REQUEST. **No autoriza S8.1 ni implementación.**

## 1. Orden confirmado

| Subetapa | Nombre | Dependencias | Salida |
|---|---|---|---|
| S8.0 | Especificación y contrato | Fase 07 CLOSED | Aprobación humana de S8.0 |
| S8.1 | Structs nominales | S8.0 aprobado | Structs TW↔VM verificados |
| S8.2 | Enums/ADTs | S8.1 verificado | Enums TW↔VM verificados |
| S8.3 | Pattern matching | S8.2 verificado | Match exhaustivo TW↔VM |
| S8.4a | Métodos inherentes | S8.1 verificado | Resolución estática TW↔VM |
| S8.4b | Traits estáticos mínimos | S8.4a verificado, S8.3 recomendable | Traits mínimos TW↔VM |
| S8.5 | Consolidación y cierre | S8.1-S8.4b verificados | READY FOR HUMAN REVIEW de cierre (aprobación humana requerida) |

Ninguna subetapa autorizada por este plan.

## 2. Contrato por subetapa

### S8.1 — Structs nominales
- **Alcance:** `estructura`, TipoNominalId/registro, campos, construcción, acceso, asignación lvalue mutable, visibilidad, recursividad por valor, BAS-7101-7108,7128,7130,7126 donde aplique, HIR/MIR/TW/VM.
- **Excluye:** structs genéricos, igualdad nominal, traits, pattern struct no requerido por S8.3 (pero patrón struct se define en S8.3; S8.1 prepara layout).
- **Componentes:** token/lexer/parser/AST, semántico, simbodo/tipos, HIR, MIR, verificadores, evaluador, VM, diagnósticos, docs.
- **Pruebas:** conformidad struct, negativos, diferenciales TW↔VM, IR, regresión G4.
- **Aceptación:** CA08-G1-G8 aplicables.

### S8.2 — Enums/ADTs
- **Alcance:** `enumeracion`, variantes unitarias/payload posicional, construcción, discriminante, visibilidad, recursividad, BAS-7109-7111.
- **Excluye:** payload struct-like, genéricos nominales, discriminantes usuario.
- **Aceptación:** construcción/match básico preparado, TW↔VM.

### S8.3 — Pattern matching
- **Alcance:** `coincidir` expresión, patrones v1, binding, exhaustividad, inalcanzable error, lowering árbol decisión, BAS-7112-7116.
- **Excluye:** guards, or, rangos, listas, `..`.
- **Aceptación:** scrutinee evaluado una vez; no caída inválida.

### S8.4a — Métodos inherentes
- **Alcance:** `implementar Tipo`, `yo`/`yo var`, métodos asociados, resolución estática, visibilidad, BAS-7117,7118,7128.
- **Excluye:** dispatch dinámico, sobrecarga ad hoc.

### S8.4b — Traits estáticos mínimos
- **Alcance:** `rasgo`, `implementar Rasgo para Tipo`, verificación firma, coherencia local, BAS-7119-7122.
- **Excluye:** dyn/default/super/assoc/bounds/generics.

### S8.5 — Consolidación
- **Alcance:** matriz final, evidencia, docs, diagnósticos, no regresión, límites.
- **Aceptación:** READY FOR HUMAN REVIEW de cierre; sin cierre sin autorización humana.

## 3. Dependencias y discrepancias resueltas

- Patrón struct: definido normativamente en S8.0; implementado en S8.3. S8.1 debe exponer layout/campos necesarios. Sin `..` en v1.
- Igualdad nominal: no en v1; BAS-7126 (solo nominales Fase 08; no altera Objeto dinámico/tipos existentes).
- Unreachable: error en v1.
- Sintaxis canónica: española, sin aliases.
- **C1 (R3):** desambiguación struct literal en cabezas con bloque - regla normativa con paréntesis.
- **C2 (R3):** lvalues v1 solo formas directas (`id`, `id.campo`, `id[indice]`); encadenadas diferidas.
- **C3 (R3):** visibilidad anclada a `publico`/`exportar`/`usar`/`importar`; coherencia impl por unidad de compilación del programa analizado.

## 4. Gates

1. Implementado en alcance.
2. Positivos/negativos/límite/diferenciales TW↔VM.
3. fmt/clippy/workspace/suites.
4. Diagnósticos/docs.
5. No regresión Fase 07.
6. Informe de evidencia. No marca CLOSED.