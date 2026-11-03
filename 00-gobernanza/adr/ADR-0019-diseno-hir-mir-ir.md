# ADR-0019 — Diseño de HIR, MIR e IR para BASILISCO

**Estado:** Implementado parcialmente (Etapas II-IV)  
**Fecha:** 2026-11-02  
**Relacionado con:** ADR-0017 (análisis semántico), ADR-0018 (inferencia de tipos), GAP-027.

## 1. Estado actual

BASILISCO 0.6.x ya dispone de HIR, MIR, VM y FFI para la VM, además del
Tree-Walk (que sigue siendo la referencia semántica). Módulos implementados:

- `src/hir.rs` — `HirPrograma`, `HirItem`, `HirSentencia`, `HirExpresion`, `HirLiteral`; lowering AST → HIR con `ResolutorHir` (pila de ámbitos con nombres, asignación de `SimbodoId` deterministas); `VerificadorHir`.
- `src/mir.rs` — `MirPrograma`, `MirFunction`, `MirBasicBlock`, terminadores (`Goto`, `SwitchInt`, `Return`, `Invoke`, `Unreachable`), operaciones, `lower_programa` (HIR → MIR con CFG real para `si`/`mientras`), `VerificadorMir`.
- `src/vm.rs` — `MaquinaVirtual`: intérprete directo de MIR, límites de pasos/profundidad, aritmética reutilizando `Valor` (ADR-0005/ADR-0007), registro de funciones nativas (`registrar_nativo` / `FuncionNativa`).
- `src/formateador.rs` — formateador AST → fuente (idempotente, incluye `publico`).
- `src/evaluador.rs` — Tree-Walk de referencia, con FFI/ABI C (`cargar_ffi`/`llamar_ffi`/`descargar_ffi`).

El Tree-Walk sigue ejecutando el AST y es la referencia semántica. La VM consume MIR y produce el mismo comportamiento observable para el subconjunto soportado (funciones, control de flujo, aritmética, llamadas nativas).

## 1b. Separación de representaciones

- AST: sintaxis del programa (estructura textual parseada).
- HIR: estructura semántica de alto nivel, con tipos resueltos y nombres resueltos; independiente del AST.
- MIR: representación de bajo nivel con bloques básicos y CFG; no depende directamente del enum `Valor` (usa tipos estáticos).
- TipoSem: información semántica de tipos (en `semantodo.rs`).
- Valor: representación de valores en runtime (en `valor.rs`).

MIR no debe depender del enum `Valor`; utiliza tipos estáticos y referencias a símbodos.

## 1c. Independencia del Tree-Walk

- El Tree-Walk actual continúa ejecutando el AST.
- No se modifica ni se sustituye durante las primeras etapas.
- La VM consume MIR o bytecode generado desde MIR.
- La posibilidad de interpretar HIR/MIR queda como investigación futura.

## 1d. Identificadores y símbodos

- Identidad de símbodos: IDs estables (`SimbodoId` en `simbodo.rs`).
- Ámbitos: `GestorAmbitos` con pila de `AmbitoId`.
- Resolución entre módulos: tabla de símbodos por módulo (`ModuloInfo`, `ExportadoSem`).
- Variables locales y parámetros: registrados en tabla con categoría `Variable`/`Parametro`.
- Funciones: categoría `Funcion`, firmas en `FuncionSig`.
- Closures: representación actual `FuncionClausura` (captura léxica); en IR se referencian por símbodo.
- Independencia de la IR respecto de referencias directas al AST: la IR referencia símbodos e IDs, no nodos del AST.

## 2. Problema arquitectónico

Sin una representación intermedia verificable, BASILISCO no puede:
- Optimizar código de forma sistemática.
- Soportar backends múltiples (nativo, WASM).
- Verificar propiedades (tipos, flujo) independientemente del evaluador.
- Mantener equivalencia entre motores de ejecución.

La evolución hacia un lenguaje de propósito general requiere una capa intermedia estable.

## 3. Objetivos

- Definir una arquitectura de compilación verificable sobre el AST existente.
- Conservar el Tree-Walk como referencia semántica.
- Permitir múltiples backends (VM bytecode, nativo, WASM) con comportamiento observable idéntico.
- Diseñar progresivamente, sin implementar de golpe.

## 4. No objetivos

- No eliminar el Tree-Walk (sigue siendo referencia semántica).
- No cambiar el sistema de tipos.
- No modificar FFI ABI C (la VM consume funciones nativas por cierre).
- No introducir dependencias nuevas.

## 5. Alternativas

A. AST → IR directo: simple, pero pierde información de alto nivel (tipos, nombres) y dificulta verificación.

B. AST → HIR → MIR: preserva alto nivel en HIR y bajo nivel en MIR; permite verificación y optimización. Recomendado.

C. AST → SSA directo: optimización fuerte pero complejo y prematuro.

## 6. Arquitectura recomendada

```
Código BASILISCO
    ↓
Lexer/Parser
    ↓
AST
    ↓
Resolución de nombres + Análisis semántico (existente)
    ↓
HIR (alto nivel, tipado, con scopes)
    ↓
MIR (control de flujo, bloques básicos, operaciones)
    ↓
Verificador de IR
    ↓
 ┌──────────┬──────────┬──────────┐
 │ Tree-Walk│ VM Byte  │ Backends │
 │ (ref.)   │          │ Nativo/  │
 │          │          │ WASM     │
 └──────────┴──────────┴──────────┘
```

- HIR: desazucarización mínima, tipos resueltos, nombres resueltos, estructura de control de alto nivel.
- MIR: bloques básicos, CFG, operaciones tipadas, sin azúcar sintáctico.
- Verificador: valida tipos, flujo, invariantes antes de ejecutar/compilar.

## 7. Modelo de datos (borrador)

- HIR: nodos tipados con identificadores de símbodos (de `simbodo.rs`), expresiones y sentencias desazucarizadas.
- MIR: bloques básicos, instrucciones (operaciones, llamadas, ramas), terminadores (return, branch, jump).
- Valores: reutilizar `Valor` como representación runtime; en IR, tipos estáticos.

## 8. Flujo de compilación

1. Lexer → Parser → AST.
2. Análisis semántico (existente) → AST validado + tabla de símbodos.
3. Lowering AST → HIR (desazucarización, resolución).
4. Verificación HIR.
5. Lowering HIR → MIR (CFG, bloques).
6. Verificación MIR.
7. Backend: Tree-Walk (interpretación directa de MIR o AST), VM bytecode, o compilación nativa/WASM.

## 9. Integración con el runtime

- El runtime actual (`Entorno`, `Valor`, cargador de módulos) se reutiliza.
- El Tree-Walk sigue operando sobre el AST como referencia semántica.
- Los backends deben producir el mismo comportamiento observable.
- FFI/ABI C se integra en MIR como llamadas externas.

## 10. Estrategia de compatibilidad

- El Tree-Walk permanece como motor de referencia.
- Los backends se introducen progresivamente (VM primero, luego nativo/WASM).
- Cada backend debe pasar la suite de conformidad.
- No se elimina el Tree-Walk hasta que los backends tengan paridad verificada.

## 11. Plan de migración incremental

Etapa I — Diseño y ADR (este documento). ✅  
Etapa II — HIR: nodos definidos, lowering desde AST, verificador. ✅ (`src/hir.rs`, `tests/hir.rs`)  
Etapa III — MIR: bloques/CFG definidos, lowering desde HIR, verificador. ✅ (`src/mir.rs`, `tests/mir.rs`)  
Etapa IV — VM bytecode: intérprete de MIR/bytecode. ✅ (`src/vm.rs`, `tests/vm.rs`, incluye FFI por cierre)  
Etapa V — Backends nativo/WASM (investigación y prototipos). ⏳  
Etapa VI — Optimizaciones sobre MIR. ⏳

## 11b. Criterios de salida por etapa

Ninguna etapa puede comenzar sin cumplir los criterios de la anterior.

- Etapa I (diseño): ADR aprobado.
- Etapa II (HIR): nodos definidos, lowering desde AST, verificador; suite de conformidad pasa.
- Etapa III (MIR): bloques/CFG definidos, lowering desde HIR, verificador; suite de conformidad pasa.
- Etapa IV (VM bytecode): intérprete de MIR/bytecode; pruebas de equivalencia Tree-Walk vs VM.
- Etapa V (backends): prototipos comparables; benchmarks reproducibles.
- Etapa VI (optimizaciones): benchmarks demuestran mejora.

## 11c. Compatibilidad y pruebas diferenciales

Pruebas diferenciales entre Tree-Walk y VM para:
- Valores (numéricos, texto, colecciones, objetos, fechas).
- Errores (códigos E1xxx, ubicaciones).
- Funciones (aridad, retorno, recursividad).
- Closures (captura léxica).
- Módulos (import/export).
- FFI (llamadas C).
- Orden observable de operaciones.
- Manejo de recursos (archivos).

## 12. Riesgos

- Complejidad: HIR/MIR añaden capas que deben mantenerse sincronizadas.
- Divergencia entre backends: mitigar con suite de conformidad y tests de equivalencia.
- Rendimiento del Tree-Walk vs backends: benchmarks reproducibles (GAP-021).
- Coste de mantenimiento: justificar cada etapa con evidencia.

## 13. Criterios de aceptación

- ADR aprobado.
- Diseño coherente con ADR-0017 y ADR-0018.
- No se modifica código en esta etapa.
- Plan de migración incremental definido.

## 14. Pruebas futuras

- Tests de equivalencia entre Tree-Walk y VM.
- Tests de verificación de IR (tipos, flujo).
- Benchmarks comparativos.
- Fuzzing de lowering.

## 15. Limitaciones fuera de alcance

- Implementación de HIR/MIR/IR/VM.
- Eliminación del Tree-Walk.
- Backends nativo/WASM.
- Optimizaciones avanzadas.
- Cambios al sistema de tipos.
## 16. Implementación realizada (2026-11-02)

- `src/hir.rs`: HIR completo con `ResolutorHir` (pila de ámbitos con nombres).
- `src/mir.rs`: MIR con CFG real; `lower_programa` y `VerificadorMir`.
- `src/vm.rs`: VM con límites de pasos/profundidad y FFI por cierre.
- `src/formateador.rs`: formateador AST→fuente, idempotente, con `publico`.
- Pruebas: `tests/hir.rs`, `tests/mir.rs`, `tests/vm.rs`, `tests/formateador.rs`.
- Validación: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test` (707+ tests en verde).
