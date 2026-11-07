# Registro de brechas de BASILISCO

**Última actualización:** 2026-11-05  
**Basado en:** auditoría directa del código fuente (commit `1267bc0`, v0.7.0)  
**Metodología:** inspección de código + ejecución de tests (782 tests, 0 fallos)

---

## Leyenda de estado

| Estado | Significado |
|---|---|
| ABIERTA | Brecha sin implementación funcional |
| PARCIAL | Implementación existe pero incompleta o con gaps conocidos |
| CERRADA | Implementación, tests y criterio de cierre satisfechos |
| EXPERIMENTAL | Existe código pero no es apto para producción ni garantizado |

---

## Tabla de brechas

| ID | Área | Estado | Prioridad | Evidencia / Archivo |
|---|---|---|---|---|
| GAP-001 | Alcance exacto del núcleo 0.3 | CERRADA | P0 | ADR-0001; subconjunto definido en `10-especificacion/especificacion-del-nucleo-0.3.md` |
| GAP-002 | Gramática normativa y precedencia | CERRADA | P0 | `10-especificacion/gramatica-de-referencia-0.3.md`; ADR-0004 y tests de precedencia |
| GAP-003 | Tipos y conversiones | PARCIAL | P0 | `semantico.rs:TipoSem`; tipos básicos definidos; sin tipos algebraicos ni generics |
| GAP-004 | Errores y diagnósticos | PARCIAL | P0 | `02-lenguaje/catalogo-de-errores-y-diagnosticos.md`; catálogo existe; falta cobertura completa de spans en todos los errores |
| GAP-005 | Funciones y retorno | CERRADA | P0 | `parser.rs`, `evaluador.rs`, `hir.rs`, `vm.rs`; tests en `conformidad_funciones.rs` y `conformidad_funciones_extra.rs` |
| GAP-006 | Inicialización, mutabilidad y sombreado | CERRADA | P0 | `semantico.rs`; `sea`/`var`/`dato`/`const`; tests en `variables.rs` y `conformidad_semantica.rs` |
| GAP-007 | Representación de cadenas y Unicode | PARCIAL | P1 | `lexer.rs`: escapes básicos y BOM; `unicode-normalization` en dependencias para BPK; sin NFC completo para identificadores del lenguaje |
| GAP-008 | Colecciones y genéricos | PARCIAL | P1 | `arreglo<T,N>` y `lista<T>` implementados; sin genéricos definibles por usuario; sin mapas/conjuntos/iteradores propios |
| GAP-009 | Errores recuperables y propagación | PARCIAL | P1 | `intentar`/`rescatar` implementado en tree-walk y VM; sin `?` ni tipos de error personalizados |
| GAP-010 | Módulos e importaciones | PARCIAL | P1 | `cargador.rs`, `hir.rs` módulos; sin visibilidad fina; sin módulos anidados como jerarquía; inicialización de top-level corregida en `1267bc0` |
| GAP-011 | Modelo de memoria y recursos | ABIERTA | P1 | Sin diseño formal de ownership ni gestión explícita de recursos |
| GAP-012 | Runtime / modelo de ejecución | CERRADA | P1 | VM MIR como motor por defecto desde `156b8e8`; tree-walk como respaldo; `con_limites()` para control |
| GAP-013 | Entrada/salida y permisos | PARCIAL | P1 | E/S básica implementada en stdlib de tree-walk y VM; sin modelo de permisos expuesto al lenguaje |
| GAP-014 | Concurrencia y cancelación | EXPERIMENTAL | P2 | `iniciar_tarea`/`esperar_tarea` en `evaluador.rs:4947` y `vm.rs`; sin canales, sin cancelación, sin mutexes |
| GAP-015 | FFI / ABI e interoperabilidad | EXPERIMENTAL | P2 | `ffi.rs`: ABI C con `libffi`/`libloading`; tipos escalares y texto; sin callbacks, structs, variádicos; 14 tests en `tests/ffi.rs` |
| GAP-016 | Paquetes y dependencias | CERRADA (local) / PARCIAL (E2E CI) | P1 | `paquetes.rs`, `cliente_remoto.rs`: BPK + SHA-256 + Ed25519 + SemVer; `tests/registry_integration.rs` marcado `#[ignore]`, no corre en CI estándar |
| GAP-017 | CLI, formatter, linter, debugger | PARCIAL | P1 | CLI completa; formateador sin comentarios; sin linter; sin debugger |
| GAP-018 | Biblioteca estándar | PARCIAL | P1 | Stdlib vía builtins en `evaluador.rs` y `vm.rs`; sin módulos de stdlib separados ni documentación de API |
| GAP-019 | Compatibilidad / versionado | PARCIAL | P1 | Política SemVer en paquetes; sin política formal de compatibilidad del lenguaje |
| GAP-020 | Seguridad de cadena de suministro | CERRADA (paquetes) / ABIERTA (ejecución) | P1 | Firma Ed25519 en BPK; sin sandbox de proceso, sin seccomp |
| GAP-021 | Rendimiento y benchmarks | ABIERTA | P2 | No existen benchmarks reproducibles |
| GAP-022 | Plataformas objetivo | PARCIAL | P1 | Compilado y probado en Linux; sin matriz de plataformas verificada |
| GAP-023 | Accesibilidad y localización | ABIERTA | P2 | Sin guía de UX; mensajes de error en español (coherente) pero sin catálogo de i18n |
| GAP-024 | Instalación / actualización / desinstalación | ABIERTA | P2 | Solo instalación via `cargo install`; sin instalador propio |
| GAP-025 | Especificación de extensiones | ABIERTA | P2 | Sin política de extensión del lenguaje |
| GAP-026 | Análisis semántico estático | PARCIAL | P1 | `semantico.rs:analizar_programa()` antes del lowering; sin inferencia completa; sin detección de uso-antes-de-inicialización en todos los caminos |
| GAP-027 | IR y máquina virtual | CERRADA | P2 | MIR CFG + VM + verificadores + optimizador + backends experimentales; `tests/mir.rs`, `tests/vm.rs`, `tests/diferencial_vm.rs` |
| GAP-028 | Verificador HIR — tipo de retorno | ABIERTA | P1 | `hir.rs:472`: `let _ = retorno;` — chequeo de tipo de retorno vacío; declarado pero no implementado |
| GAP-029 | Verificador MIR — type-checking de operandos | ABIERTA | P1 | `mir.rs:1284`: solo verifica CFG estructural; sin verificación de tipos de operandos ni use-before-def |
| GAP-030 | Lowering HIR — propagación de errores Result | ABIERTA | P1 | Plan de migración documentado en versión anterior; 51 call sites con panics en lugar de `Result`; refactor pendiente |
| GAP-031 | Paridad VM — closures mutables | ABIERTA | P1 | `tests/closures.rs` usa solo tree-walk para closures mutables; sin test diferencial VM para contador mutable |
| GAP-032 | Formateador — preservación de comentarios | ABIERTA | P2 | Lexer descarta `#` antes del AST; los comentarios no llegan al formateador |
| GAP-033 | Genéricos definibles por usuario | ABIERTA | P2 | No existe en AST, HIR ni MIR; sería un cambio de diseño mayor |
| GAP-034 | Traits / Interfaces | ABIERTA | P2 | No existe en ninguna capa |
| GAP-035 | OOP / Estructuras con nombre | ABIERTA | P2 | No existe en ninguna capa |
| GAP-036 | Canales de concurrencia | ABIERTA | P2 | Búsqueda exhaustiva confirma ausencia total |
| GAP-037 | Cancelación de tareas | ABIERTA | P2 | No existe mecanismo de cancelación |
| GAP-038 | REPL | ABIERTA | P3 | No existe en ningún módulo |
| GAP-039 | LSP | ABIERTA | P3 | No existe ningún servidor LSP |
| GAP-040 | Debugger DAP | ABIERTA | P3 | No existe ningún adaptador de debug |
| GAP-041 | Extensión IDE | ABIERTA | P3 | No existe ningún plugin |
| GAP-042 | Backend nativo — cobertura de tipos | ABIERTA (si se quiere usar en producción) | P2 | `funcion_soportada()` rechaza todo tipo que no sea entero/lógico; inutilizable para programas reales |
| GAP-043 | Backend WASM — binario directo | ABIERTA | P2 | Transpila a Rust, no genera WAT/Wasm directamente |
| GAP-044 | Documentación / índice desactualizado | ABIERTA | P2 | `INDICE-Y-MAPA-DE-COBERTURA.md` dice "264 pruebas"; `registro-de-brechas.md` anterior decía "580"; real: 782 |
| GAP-045 | `registry_integration.rs` fuera de CI | ABIERTA | P1 | Test E2E del registro marcado `#[ignore]`; R6 "cerrada" no se verifica en CI estándar |
| GAP-046 | Sandbox de ejecución | ABIERTA | P2 | Sin seccomp, sin capabilities, sin proceso aislado |
| GAP-047 | Módulos anidados jerárquicos | ABIERTA | P2 | Solo alias plano; sin `mod` jerárquico |
| GAP-048 | Iteradores definibles por usuario | ABIERTA | P2 | `para cada` solo funciona con colecciones integradas y rangos |
| GAP-049 | Benchmarks del lenguaje | ABIERTA | P3 | No existen benchmarks de rendimiento del intérprete |
| GAP-050 | Aplicaciones de ejemplo reales | ABIERTA | P3 | Solo ejemplos de tutorial; sin aplicaciones completas |

---

## Brechas cerradas confirmadas

| ID | Área | Evidencia de cierre |
|---|---|---|
| GAP-001 | Alcance núcleo 0.3 | ADR aprobado, lista de conformidad en `10-especificacion/` |
| GAP-002 | Gramática | EBNF en `gramatica-de-referencia-0.3.md` + tests de parser |
| GAP-005 | Funciones y retorno | 54 tests en `conformidad_funciones*.rs` |
| GAP-006 | Mutabilidad | `sea`/`var`/`dato`/`const` verificados; 14 tests en `variables.rs` |
| GAP-012 | Runtime / VM | 782 tests pasando; VM motor por defecto |
| GAP-016 (local) | Paquetes local | BPK + SHA-256 + Ed25519; 29+ tests |
| GAP-027 | IR / VM | Completado; 55 tests diferenciales; verificadores funcionales |

---

## Regla de cierre

Una brecha se cierra cuando:
1. Existe una decisión de diseño documentada (ADR o spec)
2. La implementación cubre los casos principales
3. Los tests cubren casos positivos y negativos
4. Los tests forman parte de la suite que corre en CI (no marcados `#[ignore]`)

Una brecha marcada como CERRADA en un informe anterior que tenga tests `#[ignore]` se reclasifica como PARCIAL hasta que el test sea ejecutable sin prerequisitos manuales, o el prerequisito esté documentado en CI.
