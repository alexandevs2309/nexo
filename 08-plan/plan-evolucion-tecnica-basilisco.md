# Plan técnico de evolución de BASILISCO

## Fase 1: consolidación del núcleo (auditoría final)

### Fase 1 — Consolidación del núcleo: CERRADA

**Fecha de cierre:** 2026-10-30.

**Evidencia de validación:**
- 498 pruebas aprobadas, 0 fallidas.
- `cargo fmt --check`: correcto.
- `cargo clippy --all-targets --all-features -- -D warnings`: correcto.

**Alcance validado:**
- Lexer.
- Parser.
- Evaluador.
- Análisis semántico parcial.
- Módulos.
- CLI.
- Biblioteca estándar básica.

**Limitaciones registradas para Fase 2:**
- `LlamadaIndirecta` sin validación estática de firma.
- Inferencia limitada en funciones no anotadas.
- Ausencia de lambdas y closures.
- Ausencia de IR, VM, compilación nativa y WASM.
- Gestor de paquetes local, sin registro remoto.

No declarar implementadas capacidades de fases posteriores.

Estado: cerrada formalmente.

Hallazgos:
- Lexer, parser, evaluador, semántico, módulos, CLI y biblioteca estándar básica implementados.
- 498 pruebas aprobadas, 0 fallidas.
- `cargo fmt --check`: correcto.
- `cargo clippy --all-targets --all-features -- -D warnings`: correcto.
- Funciones de primera clase parciales (`Valor::Funcion(String)`), sin closures ni lambdas.
- Funciones anidadas rechazadas por el parser (ADR-0008).
- `Expresion::LlamadaIndirecta` devuelve `TipoSem::Desconocido` sin validar firma/argumentos/retorno (deuda técnica para Fase 2).
- Inferencia de funciones no anotadas limitada.
- Sin IR, VM, nativo ni WASM.
- Gestor de paquetes local; sin registro remoto.

## Fase 1: consolidación del núcleo
- Objetivo: estabilizar lexer, parser, evaluador y pruebas.
- Archivos: lexer.rs, parser.rs, evaluador.rs, tests/*.
- Criterios: cargo test verde, sin regresiones.

## Fase 2: funciones y sistema de tipos
- Objetivo: lambdas, clausuras, mejor inferencia.
- Archivos: parser.rs, evaluador.rs, semantico.rs, valor.rs.
- Pruebas: funciones como valores, tipos incompatibles, recursión.

## Fase 3: módulos y paquetes
- Objetivo: registro remoto, integridad, dependencias transitivas.
- Archivos: paquetes.rs, cargador.rs, cli.rs.
- Pruebas: instalación, actualización, ciclos.

## Fase 4: arquitectura de ejecución
- Objetivo: IR, VM, backends.
- Archivos: nuevos módulos ir/vm.
- Pruebas: equivalencia semántica.

## Fase 5: ecosistema multipropósito
- Objetivo: FFI, concurrencia robusta, tooling.
- Archivos: ffi, runtime, IDE.
- Pruebas: integración, portabilidad.