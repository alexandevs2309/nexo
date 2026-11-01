# PLAN DE IMPLEMENTACIÓN — HIR (Etapa II, ADR-0019)

## 1. Objetivo y alcance

Planificar exclusivamente la Etapa II (HIR) de ADR-0019. Sin modificar código.

HIR es la representación semántica de alto nivel: tipada, con nombres resueltos, desazucarizada mínimamente, independiente del AST.

## 2. Estructuras y tipos Rust necesarios

- `HirPrograma`: lista de `HirItem`.
- `HirItem`: `Funcion`, `Declaracion`, `Importacion`, `Exportacion`.
- `HirFuncion`: nombre (SimbodoId), params `Vec<(SimbodoId, TipoSem)>`, retorno `TipoSem`, cuerpo `Vec<HirSentencia>`.
- `HirSentencia`: Declaracion, Asignacion, Si, Mientras, ParaCada, Retornar, Expresion, Bloque, etc.
- `HirExpresion`: Variable(SimbodoId), Literal(HirLiteral), Llamada { callee, args }, Lambda { params, body }, etc.
- `HirLiteral`: modelo propio (Entero(i64), Decimal(i128), Real(f64), Logico(bool), Texto(String), Nulo). No usa `Valor`.
- `HirTipo`: alias a `TipoSem` (reutilizar).
- `HirModulo`: exports/importaciones.

Identificadores: reutilizar `SimbodoId`, `AmbitoId` de `simbodo.rs`.

### A. Independencia de `Valor`

`Valor` es responsabilidad del runtime. HIR define `HirLiteral` propio para literales. No se referencia `Valor` en estructuras HIR.

## 3. Organización de módulos y archivos

- Nuevo módulo: `prototipo/src/hir.rs` (estructuras + lowering + verificador).
- Registro en `lib.rs`: `pub mod hir;`.
- Tests: `tests/hir.rs` (equivalencia).

No modificar `semantodo.rs`, `evaluador.rs`, `parser.rs`.

## 4. Conversión AST → HIR

- Función `lower_programa(ast: &[Sentencia], ctx: &ContextoSemantico) -> HirPrograma`.
- Lowering de sentencias y expresiones, resolviendo nombres a `SimbodoId` mediante `ctx.resolver_simbodo`.
- Tipos desde `TipoSem` (calculados por el análisis semántico existente).
- Desazucarización mínima: `para cada` → bucle con iterador (opcional, fase posterior).

### B. Disponibilidad de información semántica

Consultados: `semantodo.rs`, `simbodo.rs`, `parser.rs`, `evaluador.rs`.

- `ContextoSemantico` expone `resolver_simbodo(nombre) -> Option<SimbodoId>` y la tabla de símbodos.
- `TipoSem` se obtiene de `analizar_expresion`/`analizar_sentencia` (ya calculado).
- Para cada `Expresion`, el tipo se determina en `analizar_expresion`; el lowering debe re-ejecutar o cachear estos tipos.
- Si no existe una API suficiente para obtener tipos sin re-analizar, se documenta la necesidad de una interfaz mínima: `ContextoSemantico::tipo_de_expresion(&Expresion) -> TipoSem` (futuro, requiere aprobación).

Interfaz mínima necesaria (si se decide): método en `ContextoSemantico` que devuelva el `TipoSem` de una expresión ya analizada, sin duplicar el análisis. Justificación: evitar re-análisis y mantener una única fuente de verdad.

## 5. Integración con TipoSem y TablaSimbodo

- Reutilizar `TipoSem` y `TablaSimbodo`/`SimbodoId`.
- El HIR referencia símbodos por ID, no por nombre.
- El verificador HIR valida tipos y referencias.

## 6. Representación de funciones y closures

- Funciones nombradas: `HirFuncion` con `SimbodoId`.
- Lambdas: `HirExpresion::Lambda` con params y cuerpo, tipo `TipoSem::Funcion`.
- Closures: captura representada como lista de identificadores capturados con modalidad (por referencia/por valor), mutabilidad, ámbito de origen y relación con la semántica actual del evaluador. No se modifica el comportamiento de las closures.

### E. Representación de captura en closures

- Identificador capturado (`SimbodoId`).
- Modalidad de captura (referencia o valor).
- Mutabilidad.
- Ámbito de origen.
- Relación con la semántica actual del evaluador (captura léxica, sin cambios).

## 7. Verificador HIR

El verificador HIR valida invariantes estructurales y semánticas ya establecidas. No crea un segundo sistema de inferencia de tipos ni duplica diagnósticos del analizador existente.

- Validar que todos los identificadores resuelven a símbodos válidos.
- Validar tipos de expresiones (reutilizar `compatible`, `tipo_comun`).
- Validar aridad de llamadas.
- Reportar diagnósticos con spans.

### C. Responsabilidades del verificador

- Verificador de invariantes, no de inferencia.
- Reutiliza `TipoSem` y funciones existentes.
- No duplica diagnósticos; reutiliza `CausaError`.

## 8. Manejo de errores y diagnósticos

- Reutilizar `CausaError` y `ErrorEjecucion`.
- Diagnósticos con archivo/línea/columna (ya existentes).

## 9. Pruebas

Separar:
1. Pruebas de lowering (estructura HIR correcta).
2. Pruebas de resolución de símbodos (IDs válidos).
3. Pruebas de conservación de tipos (TipoSem preservado).
4. Pruebas del verificador (invariantes).
5. Pruebas de cobertura de construcciones del lenguaje.

Las pruebas de equivalencia de ejecución AST/HIR se dejan para cuando exista un motor HIR.

## 10. Criterios de aceptación

- HIR definido con estructuras documentadas.
- Lowering AST→HIR implementado y probado.
- Verificador HIR sin falsos positivos.
- Suite de conformidad pasa (Tree-Walk sigue como referencia).
- No se modifica el evaluador.
- No se elimina el Tree-Walk.

## 11. Dependencias

- Depende de ADR-0017 (semántico) y ADR-0018 (tipos).
- No depende de MIR ni VM.

## 12. Riesgos

- Duplicación de lógica de tipos entre semantodo y verificador HIR → mitigar reutilizando `TipoSem`.
- Coste de mantenimiento de dos representaciones → justificar con tests de equivalencia.

## 13. Orden de implementación

1. Definir estructuras (`hir.rs`).
2. Implementar lowering AST→HIR.
3. Implementar verificador HIR.
4. Añadir tests de equivalencia.
5. Validación completa.

## 14. Fuera de alcance

- MIR, VM, backends, optimizaciones, cambios al evaluador, cambios al sistema de tipos.

## 15. Checklist de cierre

- [ ] Estructuras definidas y documentadas.
- [ ] Lowering implementado.
- [ ] Verificador implementado.
- [ ] Tests de equivalencia pasan.
- [ ] `cargo test`, `cargo fmt`, `cargo clippy`, `cargo build --bins` OK.
- [ ] Aprobación explícita.
## 16. Archivos consultados

- `prototipo/src/semantodo.rs` (ContextoSemantico, resolver_simbodo, analizar_expresion).
- `prototipo/src/simbodo.rs` (SimbodoId, AmbitoId, TablaSimbodo, GestorAmbitos).
- `prototipo/src/parser.rs` (Expresion, Sentencia, TipoAnotacion).
- `prototipo/src/evaluador.rs` (FuncionDef, Entorno, Valor).
- `prototipo/src/valor.rs` (Valor enum).

## 17. Dependencias técnicas no resueltas

- Obtención de `TipoSem` por expresión sin re-analizar: no existe API directa. Interfaz mínima propuesta: `ContextoSemantico::tipo_de_expresion(&Expresion) -> TipoSem` (requiere aprobación).
- Sin esta interfaz, el lowering debe re-ejecutar `analizar_expresion` o cachear resultados.

## 16. Archivos consultados

- `prototipo/src/semantodo.rs` (ContextoSemantico, resolver_simbodo, analizar_expresion).
- `prototipo/src/simbodo.rs` (SimbodoId, AmbitoId, TablaSimbodo, GestorAmbitos).
- `prototipo/src/parser.rs` (Expresion, Sentencia, TipoAnotacion).
- `prototipo/src/evaluador.rs` (FuncionDef, Entorno, Valor).
- `prototipo/src/valor.rs` (Valor enum).

## 17. Dependencias técnicas no resueltas

- Obtención de `TipoSem` por expresión sin re-analizar: no existe API directa. Interfaz mínima propuesta: `ContextoSemantico::tipo_de_expresion(&Expresion) -> TipoSem` (requiere aprobación).
- Sin esta interfaz, el lowering debe re-ejecutar `analizar_expresion` o cachear resultados.
