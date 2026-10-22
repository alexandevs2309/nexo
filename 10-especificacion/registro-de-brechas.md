# Registro de brechas de Nexo

Este registro es el backlog de especificación. Las prioridades son recomendaciones de ingeniería, no estimaciones de tiempo.

| ID | Área | Prioridad | Riesgo si queda abierta | Condición de cierre |
|---|---|---:|---|---|
| GAP-001 | Alcance exacto del núcleo 0.3 | P0 | Cambios de alcance y promesas contradictorias | ADR aprobado y lista de conformidad |
| GAP-002 | Gramática normativa y precedencia | P0 | Ambigüedad entre parser, docs y ejemplos | EBNF completa del subconjunto + tests parser |
| GAP-003 | Tipos y conversiones | P0 | Resultados inesperados y pérdida de datos | Tabla normativa de conversiones y overflow |
| GAP-004 | Errores/diagnósticos | P0 | Fallos difíciles de depurar | Catálogo de errores con códigos y spans |
| GAP-005 | Funciones y retorno | P0 | No se pueden construir programas modulares | Semántica, gramática, ADR, tests positivos/negativos |
| GAP-006 | Inicialización, mutabilidad y sombreado | P0 | Inconsistencias de variables | Reglas y suite de análisis semántico |
| GAP-007 | Representación de cadenas y Unicode | P1 | Incompatibilidad de texto | Modelo de cadena, escapes, normalización y tests |
| GAP-008 | Colecciones y genéricos | P1 | APIs y tipos difíciles de evolucionar | Tipos, mutabilidad, iteración, errores y complejidad |
| GAP-009 | Errores recuperables y propagación | P1 | APIs frágiles y excepciones confusas | Modelo de error/resultado y estrategia de propagación |
| GAP-010 | Módulos e importaciones | P1 | Colisiones, ciclos y builds no reproducibles | Resolución, visibilidad, inicialización y versiones |
| GAP-011 | Modelo de memoria y recursos | P1 | Fugas, uso después de liberar o costes ocultos | ADR basado en prototipos y análisis de seguridad |
| GAP-012 | Runtime/modelo de ejecución | P1 | Arquitectura difícil de cambiar | ADR con benchmarks y contratos de runtime |
| GAP-013 | Entrada/salida y permisos | P1 | APIs inseguras o no portables | API, errores, encoding, rutas, timeout y permisos |
| GAP-014 | Concurrencia y cancelación | P2 | Deadlocks y comportamiento no determinista | Modelo de tareas, cancelación, sincronización y tests |
| GAP-015 | FFI/ABI e interoperabilidad | P2 | Integración nativa insegura | ABI, ownership, callbacks, errores y compatibilidad |
| GAP-016 | Paquetes y dependencias | P1 | Builds inseguros/no reproducibles | Formato manifiesto, lockfile, integridad y política |
| GAP-017 | CLI, formatter, linter, debugger | P1 | Mala experiencia de desarrollo | Contratos de comandos, formatos de salida y tests |
| GAP-018 | Biblioteca estándar | P1 | Cada proyecto reinventa utilidades | Módulos mínimos, estabilidad, documentación y pruebas |
| GAP-019 | Compatibilidad/versionado | P1 | Rupturas silenciosas | Política semántica, migraciones y pruebas de API |
| GAP-020 | Seguridad de cadena de suministro | P1 | Ejecución de código malicioso | Threat model, permisos, firma, auditoría y respuesta |
| GAP-021 | Rendimiento y benchmarks | P2 | Afirmaciones sin evidencia | Benchmarks reproducibles frente a casos reales |
| GAP-022 | Plataformas objetivo | P1 | Soporte declarado sin pruebas | Matriz por OS/arquitectura/función |
| GAP-023 | Accesibilidad y localización | P2 | Barreras de uso y mensajes inconsistentes | Guía de UX, contraste, lector de pantalla y catálogo |
| GAP-024 | Instalación/actualización/desinstalación | P2 | Experiencia insegura o rota | Paquetes verificables, rollback y canales de release |
| GAP-025 | Especificación de extensiones | P2 | Fragmentación del ecosistema | Política de extensión, namespaces y versionado |

## Orden de resolución recomendado
1. GAP-001 a GAP-006.
2. Implementar funciones/retorno con pruebas normativas.
3. Estabilizar errores, cadenas y colecciones.
4. Definir módulos, runtime, memoria y biblioteca mínima.
5. Después invertir en paquetes, IDE, FFI y plataformas avanzadas.

## Regla de cierre
Una brecha no se cierra porque exista un documento. Debe tener decisión aprobada, especificación, pruebas y estado de implementación; el nivel de cierre se marca por separado en cada una de esas dimensiones.

## Estado actual del prototipo (sincronizado)

| Capacidad | Estado | Evidencia |
|---|---|---|
| Núcleo numérico (entero/decimal/real) | **IMPLEMENTADO** | ADR-0005, `prototipo/tests/conformidad.rs` |
| Variables, asignación, ámbitos | **IMPLEMENTADO** | ADR-0006, `prototipo/tests/variables.rs` |
| `si`/`sino`/`sino si`, `mientras`, bloques | **IMPLEMENTADO** | ADR-0007, `prototipo/tests/conformidad_control.rs`, `prototipo/tests/control_flujo.rs` |
| Funciones/parámetros/retorno/llamadas | **IMPLEMENTADO** | ADR-0008, `prototipo/tests/conformidad_funciones.rs` |
| Cadenas como valores primarios | **IMPLEMENTADO (prototipo)** | `prototipo/tests/conformidad_texto.rs`; la especificación normativa sigue pendiente |
| Módulos/importaciones | **PENDIENTE** | GAP-010 |
| Colecciones | **PENDIENTE** | GAP-008 |
| E/S | **PENDIENTE** | GAP-013 |
| CLI completa | **PARCIAL** | `run`/`demo`/`--version`/`--help` implementados (ADR-0010); análisis, formato, pruebas e init pendientes |

**Total:** 264 pruebas en verde (252 de lenguaje + 12 de CLI), fmt y clippy limpios.
