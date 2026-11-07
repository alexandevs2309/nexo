# ROADMAP BASILISCO

**Generado:** 2026-11-05  
**Basado en:** auditoría directa del código (commit `1267bc0`, v0.7.0, 782 tests)

Las etapas están ordenadas por **dependencias técnicas**, no por conveniencia. Una etapa no debe iniciarse hasta que las dependencias de su etapa anterior estén verificadas.

---

## Principio de ponderación

El avance se mide por subsistema. Cada subsistema tiene un peso proporcional a su impacto en la utilidad del lenguaje para un programador real. El porcentaje global solo se presenta una vez explicada la metodología de ponderación.

---

## Estado actual por subsistema

| Subsistema | Avance | Peso | Contribución |
|---|---:|---:|---:|
| Lexer | 90% | 3% | 2.7% |
| Parser / AST | 85% | 5% | 4.3% |
| Análisis semántico | 50% | 7% | 3.5% |
| Sistema de tipos | 40% | 8% | 3.2% |
| HIR + Lowering | 70% | 5% | 3.5% |
| Verificador HIR | 50% | 3% | 1.5% |
| MIR + Lowering | 80% | 5% | 4.0% |
| Verificador MIR | 30% | 3% | 0.9% |
| Optimizador | 30% | 3% | 0.9% |
| VM | 85% | 8% | 6.8% |
| Tree-walk | 90% | 4% | 3.6% |
| Closures / Lambdas | 75% | 5% | 3.8% |
| Módulos | 55% | 6% | 3.3% |
| Excepciones | 60% | 4% | 2.4% |
| Colecciones | 40% | 5% | 2.0% |
| Concurrencia | 15% | 5% | 0.8% |
| FFI | 35% | 3% | 1.1% |
| CLI | 85% | 4% | 3.4% |
| Formateador | 50% | 2% | 1.0% |
| Gestión de paquetes | 85% | 5% | 4.3% |
| LSP / IDE / Debugger | 0% | 7% | 0.0% |
| Documentación / ejemplos | 30% | 5% | 1.5% |
| **TOTAL** | | **100%** | **~58%** |

> **Avance global estimado: ~58%** para alcanzar un lenguaje usable por un programador externo.  
> Este porcentaje no incluye madurez de producción (benchmarks, plataformas, seguridad de proceso).

---

## Etapa 1 — Corrección del pipeline existente

**Objetivo:** eliminar panics y falsos negativos en el compilador.  
**Dependencias:** ninguna (es la base).  
**Brechas que cierra:** GAP-028, GAP-029, GAP-030, GAP-031

### 1.1 Verificador HIR — tipo de retorno (GAP-028)
- Archivo: `hir.rs:472`
- Tarea: comparar el tipo anotado de retorno con los nodos `HirSentencia::Retornar(Some(e))` del cuerpo
- Criterio de cierre: tests negativos que detecten `funcion f() -> entero { retornar "hola" }` como error

### 1.2 Lowering HIR — propagar errores como Result (GAP-030)
- Archivo: `hir.rs` — 51 call sites documentados en el "Plan de migración DT-03"
- Tarea: cambiar `lower_expresion` a `Result<HirExpresion, LoweringError>` con propagación `?`
- Criterio de cierre: ningún `unwrap()` o `expect()` en el path principal del lowering

### 1.3 Verificador MIR — LocalId y type-checking básico (GAP-029)
- Archivo: `mir.rs:VerificadorMir`
- Tarea: verificar que todos los `LocalId` referenciados en `MirStmt` existen en `f.locals`
- Criterio de cierre: tests que generen MIR inválido y confirmen que el verificador los rechaza

### 1.4 Paridad VM — closures mutables (GAP-031)
- Archivo: `tests/diferencial_vm.rs`
- Tarea: añadir test diferencial tree-walk vs VM para el patrón `crear_contador()` de `tests/closures.rs`
- Criterio de cierre: test pasa en ambos motores con salida idéntica

---

## Etapa 2 — Completar características del núcleo del lenguaje

**Objetivo:** que el lenguaje pueda usarse para programas reales no triviales.  
**Dependencias:** Etapa 1 completa.  
**Brechas que cierra:** GAP-008 (parcial), GAP-009, GAP-010, GAP-013, GAP-026

### 2.1 Módulos — visibilidad fina (GAP-010)
- Añadir re-exportaciones selectivas: `exportar { suma, resta }`
- Definir semántica de inicialización de top-level en módulos (actualmente frágil, fix reciente en `1267bc0`)
- Tests: importar solo lo que se exporta; acceso a no-exportado falla

### 2.2 Análisis semántico — cobertura completa (GAP-026)
- Detectar uso de variable no inicializada en todos los caminos (actualmente solo detecta declaración duplicada)
- Verificar que funciones `dato`/`const` no se reasignan en cadenas de llamadas
- Tests negativos por cada regla nueva

### 2.3 Colecciones — iteradores y mapas (GAP-008)
- Implementar `mapa<clave, valor>` o `diccionario<K, V>` como tipo integrado
- Implementar `filtrar`, `mapear`, `reducir` como builtins (parcialmente existen en stdlib)
- Definir si `para cada` puede recorrer mapas y con qué semántica

### 2.4 Excepciones — modelo de propagación (GAP-009)
- Definir tipos de error como valores (`Resultado`, `Error`)
- Implementar un operador de propagación (`?` o `propagar`)
- Tests: propagación anidada, conversión entre tipos de error

### 2.5 E/S — modelo de permisos (GAP-013)
- Definir qué operaciones de archivo requieren declaración de permisos
- Implementar restricción de rutas fuera del directorio del proyecto

---

## Etapa 3 — Completar el formateador e integrar CI

**Objetivo:** herramientas de desarrollo confiables.  
**Dependencias:** Etapa 1 (para que el AST sea estable).  
**Brechas que cierra:** GAP-032, GAP-044, GAP-045

### 3.1 Formateador — preservar comentarios (GAP-032)
- El lexer debe emitir tokens de comentario o trivia que el parser adjunte al AST
- El formateador reconstruye comentarios en la posición correcta
- Tests: formatear código con comentarios y verificar que se preservan

### 3.2 Actualizar índice y métricas (GAP-044)
- `INDICE-Y-MAPA-DE-COBERTURA.md` debe reflejar 782 tests (o el conteo actual)
- Automatizar extracción de métricas con un script en CI

### 3.3 Registry integration en CI (GAP-045)
- Configurar un servicio PostgreSQL en GitHub Actions para el job de `registry_integration.rs`
- Quitar `#[ignore]` del test o añadir un job separado con `--ignored`
- Criterio de cierre: test E2E del registro corre en cada PR

---

## Etapa 4 — Sistema de tipos avanzado

**Objetivo:** dar al lenguaje expresividad suficiente para APIs limpias.  
**Dependencias:** Etapas 1 y 2.  
**Brechas que cierra:** GAP-003 (parcial), GAP-033, GAP-034, GAP-035

### 4.1 Tipos producto con nombre (structs) (GAP-035)
- Sintaxis: `tipo Punto { x: entero, y: entero }`
- Acceso: `p.x`
- Necesita nuevos nodos en AST, HIR, MIR, y un runtime en `Valor`
- Tests: construcción, acceso, mutabilidad

### 4.2 Genéricos básicos (GAP-033)
- Sintaxis: `funcion identity<T>(x: T) -> T { retornar x }`
- Solo monomorfización en tiempo de compilación (no dinámico)
- Bloqueado por: structs con nombre (4.1)

### 4.3 Traits / Interfaces (GAP-034)
- Sintaxis: `interfaz Printable { funcion mostrar_repr(self) -> texto }`
- Implementación: `implementar Printable para Punto { ... }`
- Bloqueado por: genéricos básicos (4.2)

---

## Etapa 5 — Concurrencia real

**Objetivo:** concurrencia segura y utilizable.  
**Dependencias:** Etapa 2 (modelo de errores), Etapa 4 (tipos necesarios para mensajes de canal).  
**Brechas que cierra:** GAP-014, GAP-036, GAP-037

### 5.1 Canales (GAP-036)
- `sea (env, rec) = canal()`
- `enviar(env, valor)` / `recibir(rec)` (bloqueante)
- Semántica de capacidad: ilimitado vs. limitado
- Tests: productor-consumidor, múltiples productores

### 5.2 Cancelación de tareas (GAP-037)
- `cancelar(id_tarea)` o timeout en `esperar_tarea`
- Definir semántica de cancelación cooperativa vs. forzada

---

## Etapa 6 — Backends compilados

**Objetivo:** generar ejecutables nativos o WASM desde programas completos.  
**Dependencias:** Etapas 1-4 (el sistema de tipos debe estar estable).  
**Brechas que cierra:** GAP-042, GAP-043

### 6.1 Backend nativo — cobertura completa de tipos (GAP-042)
- Extender `generar_funcion()` para soportar `Texto` (puntero a CString), `Real` (f64), colecciones
- Alternativa: abandonar la transpilación a Rust y usar Cranelift o LLVM directamente
- Criterio de cierre: el 90% de los tests de conformidad pasan compilados con el backend nativo

### 6.2 Backend WASM — binario directo (GAP-043)
- Generar WAT (WebAssembly Text) directamente, sin pasar por Rust
- Alternativa: usar `wasm-encoder` o similar
- Criterio de cierre: tutorial_inicio.bsk compila y corre en Node.js sin `rustc`

---

## Etapa 7 — FFI robusto

**Objetivo:** interoperabilidad con C/Rust segura.  
**Dependencias:** Etapas 1 y 4 (tipos producto para structs C).  
**Brechas que cierra:** GAP-015

### 7.1 Callbacks como argumentos FFI
- Pasar funciones Basilisco como punteros de función a C
- Requiere ABI estable para closures

### 7.2 Structs C en FFI
- Definir structs de layout C desde Basilisco
- Pasar/recibir structs por valor y puntero

### 7.3 Diseño de seguridad FFI
- Sandbox de llamadas FFI (no escalar privilegios)
- Documentar surface de ataque

---

## Etapa 8 — Optimizador avanzado

**Objetivo:** mejorar el rendimiento del código generado.  
**Dependencias:** Etapa 6 (tiene más impacto en backends compilados).  
**Brechas que cierra:** GAP-021 (benchmarks), mejora de GAP-027

### 8.1 Eliminación de código muerto entre bloques
- Marcar bloques inalcanzables tras simplificación de ramas
- Eliminar stores sin usos posteriores (dead store elimination)

### 8.2 Inlining de funciones pequeñas
- Criterio: funciones de 1-3 instrucciones MIR con única llamada

### 8.3 Benchmarks reproducibles
- Suite de benchmarks (fibonacci, sort, procesamiento de texto)
- Comparativa con Python y Lua para posicionar el intérprete

---

## Etapa 9 — Herramientas de desarrollo (LSP, debugger, IDE)

**Objetivo:** experiencia de desarrollo integrada.  
**Dependencias:** Etapas 1-2 (el análisis semántico debe ser estable y completo).  
**Brechas que cierra:** GAP-017 (linter), GAP-039, GAP-040, GAP-041

> Esta etapa puede iniciarse en paralelo con las etapas 3-6 para equipos separados. Solo requiere que el análisis semántico (etapa 1+2) sea estable, no que los backends estén completos.

### 9.1 LSP (GAP-039)
- Servidor LSP independiente (binario `basilisco-lsp`)
- Diagnósticos: errores léxicos, sintácticos y semánticos con spans
- Hover: tipo de la expresión bajo el cursor
- Ir a definición: variables y funciones
- Autocompletado: nombres del ámbito actual

### 9.2 Extensión VS Code (GAP-041)
- Sintax highlighting (TextMate grammar)
- Integración con el LSP
- Snippets de código

### 9.3 Debugger DAP (GAP-040)
- Adaptador DAP sobre la VM
- Breakpoints en instrucciones MIR (mapeadas a líneas del fuente)
- Inspección de locales y globals en cada paso

---

## Etapa 10 — Ecosistema y madurez

**Objetivo:** lenguaje utilizable por desarrolladores externos.  
**Dependencias:** Etapas 1-9.  
**Brechas que cierra:** GAP-023, GAP-024, GAP-038, GAP-046, GAP-049, GAP-050

### 10.1 REPL (GAP-038)
- Modo interactivo incremental sobre el árbol de evaluación
- Historial, edición de línea

### 10.2 Sandbox de ejecución (GAP-046)
- Seccomp-BPF en Linux para limitar syscalls
- Modelo de permisos declarado en `basilisco.toml`

### 10.3 Instalador propio (GAP-024)
- Script de instalación multiplataforma
- Canales: estable, beta, nightly

### 10.4 Aplicaciones de ejemplo reales (GAP-050)
- Servidor HTTP simple usando el registro de paquetes
- Procesador de CSV / JSON
- Juego de texto

---

## Capacidades listas para usar hoy (sin riesgos mayores)

- Escritura y ejecución de scripts Basilisco con funciones, recursión, control de flujo
- Colecciones básicas (arreglos, listas, rangos)
- Manejo de excepciones con `intentar`/`rescatar`
- Módulos e importaciones con alias
- Gestión de paquetes local (BPK)
- Registro remoto de paquetes (con PostgreSQL)
- CLI completa (`run`, `nuevo proyecto`, `analizar`, `formatear`, `instalar`)

## Capacidades experimentales (usar con precaución)

- FFI con ABI C (sin callbacks ni structs)
- Concurrencia básica (sin canales ni cancelación)
- Backend nativo (solo tipos entero/lógico)
- Backend WASM (mismas limitaciones)

## Capacidades ausentes (no usar)

- Genéricos, traits, OOP
- LSP, debugger, extensión IDE
- REPL
- Sandbox de proceso
- Canales de concurrencia
- Backend nativo para programas reales
