# FASE 14.1 — Auditoría forense del frontend del lenguaje

> **Fecha:** 2026-11-27  
> **Rama:** `main` | **HEAD:** `bb1bfed` (`feat(basilisco): complete phase 07 generics and monomorphization`)  
> **Worktree:** sucio preexistente preservado (74 entradas antes de esta misión; se agregan solo artefactos de esta misión).  
> **Alcance:** frontend (lexer, parser, AST, spans/posiciones, recuperación de errores, semántico, resolución de nombres, módulos/imports, visibilidad, diagnósticos). Sin auditoría general del proyecto.

## 1. Alcance

Auditoría forense del frontend real de BASILISCO, con correcciones mínimas justificadas solo para defectos reproducibles dentro del alcance. No se realizaron cambios en Fase 13, no se modificó `DEVELOPMENT_STATE.md` ni `ROADMAP_MASTER.md`, no se tocaron componentes congelados sin regresión reproducible, no se ejecutaron operaciones Git destructivas ni commits/push/merge/tags/releases/publicaciones.

## 2. Arquitectura inspeccionada

- **Lexer:** `prototipo/src/lexer.rs` (627 líneas) + `prototipo/src/token.rs` (141). Tokeniza UTF-8, char-based, emite `FinDeLinea`, captura comentarios, reporta `ErrorLexico` con línea/columna/archivo.
- **Parser/AST:** `prototipo/src/parser.rs` (3493). Descenso recursivo; AST `Sentencia`/`Expresion` con línea/columna en nodos principales; `ErrorSintactico` con línea/columna/archivo; sin recuperación/sincronización.
- **Diagnósticos:** `prototipo/src/diagnostico.rs` (76). `Span`, `Diagnostico`, `Severidad`, conversión LSP.
- **Semántico:** `prototipo/src/semantico.rs` (6904). Análisis estático con scopes, TablaSimbodo, resolución de funciones/variables, nominales, imports/módulos, G2/G3/G4 y side tables LSP.
- **Símbolos:** `prototipo/src/simbodo.rs` (180). `SimbodoId`, `AmbitoId`, `TablaSimbodo`, `GestorAmbitos`.
- **Módulos/cargador:** `prototipo/src/cargador.rs` (461). Resolución canónica bajo root, caché, ciclo, validación top-level, ejecución para exportaciones.
- **Evaluador (solo inspección de contrato):** `prototipo/src/evaluador.rs` para contraste de comportamiento runtime en acceso a miembros de módulo.

## 3. Archivos revisados

- `prototipo/src/lexer.rs`, `prototipo/src/token.rs`, `prototipo/src/parser.rs`, `prototipo/src/diagnostico.rs`, `prototipo/src/semantico.rs`, `prototipo/src/simbodo.rs`, `prototipo/src/cargador.rs`, `prototipo/src/lib.rs`, `prototipo/src/evaluador.rs` (lectura dirigida).
- Pruebas: `prototipo/tests/semantico.rs`, `prototipo/tests/semantico_modulos.rs`, `prototipo/tests/sintaxis_modulos.rs`, `prototipo/tests/modulos_e2e.rs`, `prototipo/tests/fuzz_basico.rs`, `prototipo/tests/integracion.rs`.

## 4. Pruebas ejecutadas

Comandos ejecutados (desde `prototipo/` salvo indicado):

| Comando | Resultado |
|---|---|
| `cargo test --lib lexer -- --nocapture` | PASS — 17 passed, 0 failed |
| `cargo test --lib parser -- --nocapture` | PASS — 27 passed, 0 failed |
| `cargo test --test semantico` | PASS — 109 passed, 0 failed |
| `cargo test --test sintaxis_modulos` | PASS — 20 passed, 0 failed |
| `cargo test --test semantico_modulos` | PASS — 11 passed, 0 failed |
| `cargo test --test modulos_e2e` (post-fix) | PASS — 31 passed, 0 failed |
| `cargo test --test fuzz_basico` | PASS — 1 passed, 0 failed |
| `cargo fmt --check` | PASS |
| `git diff --check` (raíz repo) | PASS |

Pruebas adicionales de reproducción/regresión:

| Comando | Resultado |
|---|---|
| Probe temporal con `CargadorModulos::cargar` para expresión top-level no permitida | Reprodujo FE-01: `LOC=1:1`; tras fix `LOC=3:1` |
| Probe temporal con `ContextoSemantico` para `usar ... como lib; lib.noexiste; lib.secreto` | Reprodujo FE-02: `ERRS=[]` (sin diagnóstico semántico) |
| `cargo test --test modulos_e2e e2e_modulo_sentencia_no_permitida_reporta_ubicacion_expresion` | PASS — 1 passed |

No se ejecutó `cargo test --workspace` completo en esta misión; se ejecutaron suites focales de frontend y verificaciones mínimas exigidas. No se declara PASS global de workspace.

## 5. Hallazgos

| ID | Severidad | Componente | Estado | Resumen |
|---|---|---|---|---|
| FE-01 | LOW | Cargador/Semántico (validación top-level módulo) | **CORREGIDO** | Ubicación 1:1 incorrecta para sentencia no permitida en nivel superior de módulo cuando la sentencia es `Expresion` o `Bloque`. |
| FE-02 | HIGH | Semántico (imports/visibilidad/resolución de miembros) | **CONFIRMADO, NO CORREGIDO** | El análisis semántico no valida acceso a miembro inexistente o privado vía alias de módulo (`usar "lib.bsk" como lib; lib.noexiste/lib.secreto`), devolviendo `Desconocido` sin diagnóstico. El runtime sí rechaza correctamente. |
| FE-03 | INFORMATIONAL | Parser | **LIMITACIÓN** | No hay recuperación/sincronización de errores: `parsear_programa`/sentencias retornan al primer `ErrorSintactico`. |
| FE-04 | INFORMATIONAL | Diagnóstico/LSP contract | **CONTRATO OBSERVADO** | Posiciones lexer/parser son 1-based, basadas en Unicode scalar chars; `diagnostico.a_lsp` convierte a 0-based char. LSP por defecto espera UTF-16 code units; puede haber mismatch en no-BMP. No se modificó (área congelada/contrato sensible). |

## 6. Detalle de hallazgos

### FE-01 — Ubicación incorrecta en sentencia top-level no permitida (CORREGIDO)

- **Archivo/módulo:** `prototipo/src/cargador.rs`, `prototipo/src/semantico.rs`.
- **Comportamiento esperado:** al rechazar una sentencia no permitida en nivel superior de módulo, el diagnóstico debe apuntar a la ubicación real de la sentencia.
- **Comportamiento observado:** para `Sentencia::Expresion(_)` y `Sentencia::Bloque(_)`, `ubicacion_sentencia` devolvía `(1,1)`.
- **Reproducción mínima:**
  ```bsk
  # lib.bsk
  # comentario

  1 + 2
  ```
  ```bsk
  # main.bsk
  usar "lib.bsk" como lib
  ```
  Antes del fix: `lib.bsk:1:1` / E1037. Después: `lib.bsk:3:1`.
- **Causa raíz:** las variantes `Sentencia::Expresion` y `Sentencia::Bloque` no contienen línea/columna propias; el helper devolvía fallback fijo `(1,1)` sin inspeccionar la expresión o el primer elemento del bloque.
- **Severidad:** LOW (calidad de diagnóstico, sin crash ni semántica incorrecta).
- **Corrección mínima:** en `cargador.rs` y `semantico.rs`, `ubicacion_sentencia` ahora obtiene ubicación de `Expresion` mediante `ubicacion_expresion` y de `Bloque` mediante primera sentencia; se agregó `ubicacion_expresion` local pura para cubrir nodos binarios/unarios que no almacenan span propio.
- **Prueba de regresión:** `prototipo/tests/modulos_e2e.rs::e2e_modulo_sentencia_no_permitida_reporta_ubicacion_expresion`.
- **Resultado posterior:** PASS. Probe manual y e2e reportan `lib.bsk:3:1`.

### FE-02 — Miembro de módulo alias no validado semánticamente (CONFIRMADO, NO CORREGIDO)

- **Archivo/módulo:** `prototipo/src/semantico.rs` (manejo de `Expresion::Acceso`, `Expresion::LlamadaIndirecta`, y binding de `Sentencia::Usar` con alias).
- **Comportamiento esperado:** `usar "lib.bsk" como lib` debe permitir que el análisis semántico diagnostique miembros inexistentes y miembros privados accedidos por alias, coherente con visibilidad/exportaciones del módulo.
- **Comportamiento observado:** el analizador registra el alias como variable con `TipoSem::Desconocido`; `Acceso`/`LlamadaIndirecta` no resuelven miembros de módulo y retornan `Desconocido` sin error. Runtime sí rechaza: `SimbodoNoExportado` / `SimbodoPrivado`.
- **Reproducción mínima:**
  ```bsk
  # lib.bsk
  publico sea valor = 5
  sea secreto = 9
  ```
  ```bsk
  # main.bsk
  usar "lib.bsk" como lib
  lib.noexiste
  lib.secreto
  ```
  Análisis semántico: `ERRS=[]`. Runtime/evaluador: rechaza (verificado por inspección de `evaluador.rs` y contrato runtime existente).
- **Causa raíz:** no existe mapeo semántico alias→módulo canónico/exportaciones para resolver accesos de miembro; el alias solo se introduce como variable opaca.
- **Severidad:** HIGH (brecha de validación semántica en visibilidad/resolución de nombres; errores detectables en frontend llegan al runtime).
- **Corrección mínima propuesta (no aplicada):** añadir tracking scope-aware de alias de módulo en `ContextoSemantico`, resolver en `Acceso`/`LlamadaIndirecta` contra `ModuloInfo.exports/nombres` y emitir `SimbodoNoExportado`/`SimbodoPrivado` con ubicación; requiere cuidado con shadowing y scope push/pop.
- **Motivo de no corrección en esta misión:** el cambio toca estado central del analizador semántico (varios push/pop de scopes, shadowing, LSP side tables, rutas de llamada indirecta). No se aplicó una corrección apresurada para evitar regresión fuera del defecto puntual. Se documenta para misión/hotfix dirigido.
- **Prueba de regresión:** no agregada (defecto no corregido); reproducción confirmada por probe temporal.

### FE-03 — Sin recuperación/sincronización de parser (LIMITACIÓN)

- **Archivo/módulo:** `prototipo/src/parser.rs`.
- **Observación:** el parser reporta el primer error y retorna; no hay `panic_mode`/synchronization. `parsear_programa` usa `self.sentencia()?`.
- **Clasificación:** INFORMATIONAL/LIMITACIÓN. No es crash; afecta experiencia de diagnóstico múltiple.
- **Acción:** documentado, sin cambio.

### FE-04 — Contrato de posiciones char-based vs UTF-16 (CONTRATO OBSERVADO)

- **Archivo/módulo:** `prototipo/src/lexer.rs`, `prototipo/src/token.rs`, `prototipo/src/diagnostico.rs`.
- **Observación:** lexer incrementa columna por `char` Unicode scalar; tokens y AST almacenan línea/columna 1-based; `Span` es char-based; `Diagnostico::a_lsp` resta 1 y emite `character` como índice char. LSP default es UTF-16 code units.
- **Clasificación:** INFORMATIONAL/CONTRATO. No se cambia en 14.1 por sensibilidad transversal y cercanía con componentes congelados LSP.
- **Acción:** documentado como contrato observado; requiere decisión de contrato antes de cambio.

## 7. Causas raíz

- **FE-01:** helper de ubicación incompleto para variantes de sentencia sin campos de posición propios.
- **FE-02:** modelado semántico insuficiente para alias de módulo en forma `usar ... como ...`; acceso por alias no cruza a metadatos de exportación.
- **FE-03:** diseño de parser de primer error.
- **FE-04:** contrato histórico char-based sin negociación UTF-16 explícita.

## 8. Correcciones aplicadas

1. `prototipo/src/cargador.rs`
   - Importa `Expresion`.
   - `ubicacion_sentencia`: maneja `Sentencia::Expresion` vía `ubicacion_expresion` y `Bloque` vía primera sentencia.
   - Agrega `ubicacion_expresion` para extraer ubicación de nodos con/sin span directo.
2. `prototipo/src/semantico.rs`
   - Misma corrección en `ubicacion_sentencia` y helper `ubicacion_expresion` local para diagnósticos semánticos de módulo top-level.
3. `prototipo/tests/modulos_e2e.rs`
   - Regresión `e2e_modulo_sentencia_no_permitida_reporta_ubicacion_expresion`.

No se modificó sintaxis, contratos públicos, semántica de tipos/inferencia, VM, LSP, DAP, ni componentes congelados.

## 9. Pruebas de regresión

- `e2e_modulo_sentencia_no_permitida_reporta_ubicacion_expresion` (`prototipo/tests/modulos_e2e.rs`): verifica que `# comentario\n\n1 + 2` en módulo importado reporta `lib.bsk:3:1` y el mensaje E1037 esperado.

## 10. Limitaciones

- FE-02 no corregido en esta misión (confirmado y reproducido).
- No se ejecutó workspace completo ni suites no focales fuera de las necesarias.
- No se auditó rendimiento/memoria ni se hicieron cambios de contrato UTF-16/LSP.
- No se revisó a fondo el pipeline completo de semántico (6904 líneas) más allá de áreas objetivo y hallazgos reproducibles.

## 11. Riesgos residuales

- **Alto:** FE-02 permite que código con miembro de módulo inexistente/privado por alias pase el análisis semántico y falle recién en runtime.
- **Medio:** ausencia de recuperación del parser limita reporte de múltiples errores por archivo.
- **Bajo:** ambigüedad de contrato de columnas en caracteres no-BMP puede afectar diagnósticos/LSP en casos edge.

## 12. Estado Git

- **Rama:** `main`
- **HEAD:** `bb1bfed`
- **`git diff --check`:** PASS
- **Worktree:** sucio preexistente preservado. Cambios de esta misión: `prototipo/src/cargador.rs`, `prototipo/src/semantico.rs`, `prototipo/tests/modulos_e2e.rs`, `docs/reports/FASE_14_1_AUDITORIA_FRONTEND.md` (nuevo). No se modificó `DEVELOPMENT_STATE.md` ni `ROADMAP_MASTER.md`. No se realizaron commits/push/merge/tags/releases/publicaciones.

## 13. Dictamen

**INCOMPLETE — FINDINGS DOCUMENTED**

Las pruebas focales ejecutadas pasan y FE-01 fue corregido con regresión. Sin embargo, queda FE-02 (HIGH) sin resolver dentro del alcance: el análisis semántico no diagnostica miembros inexistentes/privados accedidos por alias de módulo. No se ocultaron hallazgos; no se inició Fase 14.2 ni Fase 15.
