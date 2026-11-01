# ADR-0019 — Diseño de HIR, MIR e IR para BASILISCO

**Estado:** Propuesto  
**Fecha:** 2026-11-01  
**Relacionado con:** ADR-0017 (análisis semántico), ADR-0018 (inferencia de tipos), GAP-027.

## 1. Estado actual

BASILISCO 0.6.0 dispone de lexer, parser, AST, análisis semántico parcial (`semantodo.rs`), evaluador Tree-Walk, tabla de símbodos, tipos graduales, funciones/closures/lambdas, módulos, FFI/ABI C parcial, biblioteca estándar inicial y registro remoto R6. No existe HIR, MIR, IR ni VM. El Tree-Walk es el único motor de ejecución y referencia semántica.

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

- No implementar HIR/MIR/IR/VM en esta etapa.
- No eliminar el Tree-Walk.
- No cambiar el sistema de tipos.
- No modificar FFI.
- No introducir dependencias.

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

Etapa I — Diseño y ADR (este documento).  
Etapa II — HIR: definir nodos, lowering desde AST, verificador.  
Etapa III — MIR: definir bloques/CFG, lowering desde HIR, verificador.  
Etapa IV — VM bytecode: intérprete de MIR/bytecode.  
Etapa V — Backends nativo/WASM (investigación y prototipos).  
Etapa VI — Optimizaciones sobre MIR.

Cada etapa requiere ADR y aprobación.

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