# Registro de brechas de BASILISCO

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
| GAP-015 | FFI/ABI e interoperabilidad | P2 | Integración nativa insegura | **Parcial** (prototipo 2026-10-31): puente ABI C básico con pruebas; restan callbacks, structs, variádicos y design review de seguridad |
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
| GAP-026 | Análisis semántico estático | P1 | Errores de tipos y nombres aparecen tarde, durante ejecución | Pase semántico integrado, reglas aprobadas y pruebas negativas antes de ejecutar |
| GAP-027 | IR y máquina virtual | P2 | Sin ruta de ejecución alternativa ni base para optimización/backends | Diseño aprobado, implementación integrada y suite de equivalencia con el intérprete |

## Orden de resolución recomendado
1. GAP-001 a GAP-006.
2. Implementar funciones/retorno con pruebas normativas.
3. Estabilizar errores, cadenas y colecciones.
4. Definir módulos, runtime, memoria y biblioteca mínima.
5. Después invertir en paquetes, IDE, FFI y plataformas avanzadas.

## Regla de cierre
Una brecha no se cierra porque exista un documento. Debe tener decisión aprobada, especificación, pruebas y estado de implementación; el nivel de cierre se marca por separado en cada una de esas dimensiones.

## Estado actual del prototipo (sincronizado)

| Capacidad | Código local | Cobertura de pruebas | Norma / alcance v0.4 |
|---|---|---|---|
| Núcleo numérico (entero/decimal/real, incluidos especiales IEEE de `real`) | Implementado | Suite de conformidad numérica: overflow a infinito, `NaN`, `-0.0` y división por cero con cero negativo | ADR-0005 / incluido |
| Variables, asignación, tipos anotados y ámbitos | Implementado | Variables, control y tipos | Core 0.3 + ADR-0011 / incluido |
| `si`/`sino`/`mientras`, bloques | Implementado | Conformidad y flujo de control | Core 0.3 / incluido |
| Funciones, parámetros y retorno | Implementado; tipos anotados e inferidos verificados | Suite positiva/negativa: aridad, orden de argumentos, primera llamada (incluida recursión), conversiones implícitas aprobadas, retorno anotado, `nulo` y colecciones anotadas | ADR-0008/0011; Core 0.3 no exhaustivo / perfil v0.4 aprobado |
| Texto básico | Implementado | Literales, concatenación, comparación y mostrar | Core 0.3 no exhaustivo / incluido v0.4 |
| Arreglos/listas con tipos, indexación y asignación | Implementado parcialmente | Tamaño inicial, tamaño cero, tipos de elementos, lista vacía, mutabilidad, asignación/lectura de índices, índices inválidos y tipos de índice | No normados en Core 0.3 / incluido solo en forma básica |
| `para cada`, texto iterable, rangos, `romper`/`continuar` | Implementado parcialmente; binding inmutable, ámbito limpio y detección de modificación de la colección iterada | Regresiones cubren rangos exclusivos/inclusivos, vacíos, negativos, límites, ciclos anidados, texto vacío y modificación durante iteración; señales a través de rescate y scope del iterador | ADR-0012 / extensión prototipal fuera de garantía v0.4; falta texto iterable con casos Unicode y más combinaciones de anidamiento |
| `intentar`/`rescatar` | Implementado parcialmente; separado fin normal de retorno ausente | Suite cubre error capturado, fin normal, retorno de función, señales de ciclo no capturadas y nivel superior/función | ADR-0014 / extensión prototipal fuera de garantía v0.4; faltan más casos de anidamiento y sintaxis inválida |
| E/S de texto y archivos | Implementado parcialmente | Conformidad y casos de errores pendientes de revisar | ADR-0013 / extensión prototipal fuera de garantía v0.4 |
| CLI `basilisco`/`bsk` | `run`, `demo`, ayuda, versión, extensiones y diagnóstico de ejecución | Pruebas CLI positivas/negativas y aceptación integrada | ADR-0010 / incluido |
| Módulos e importaciones | Implementado parcialmente: `importar`/`exportar`, alias `como`, detección de ciclos y caché por proceso | Suites de importación y paquetes | GAP-010 / falta visibilidad fina e inicialización controlada |
| Paquetes y dependencias | Implementado localmente + registro remoto R6: servidor Axum/SQLx con autenticación, namespaces, publicación BPK validada (SHA-256 + Ed25519), búsqueda, descarga, yank; cliente `instalar-remoto` verifica firma; persistencia y reinicio verificados | Suites `tests/paquetes.rs`, `tests/remoto_e2e.rs`, `tests/server_auth.rs`, `tests/registry_integration.rs` (ignorada sin PG) + smoke `examples/r6_smoke.rs` | GAP-016 / R6 CERRADA con evidencia (PostgreSQL real) |
| Concurrencia | Implementado mínimamente: `iniciar_tarea`/`esperar_tarea` sobre hilos, sin cancelación ni canales | Prueba manual; pendiente suite dedicada | GAP-014 / no apto para producción |
| Formateador | Implementado sobre AST para el subconjunto v0.5 | Pruebas CLI de formato | GAP-017 / sin preservación de comentarios |
| FFI/ABI | Implementado básicamente: `cargar_ffi`/`llamar_ffi`/`descargar_ffi` sobre ABI C (libffi + libloading); tipos escalar y texto; retorno `entero`/`logico`/`real`/`decimal`/`texto`/`nulo` | Suite `tests/ffi.rs`: enteros, lógico, reales, mixtos, decimal escalado, texto, nulo, errores de carga/símbolo/retorno/argumento/descarga | GAP-015 / parcial: sin callbacks, structs ni variádicos; especificación en `10-especificacion/ffi-abi.md` |
| Análisis semántico estático | No disponible como pase separado; `analizar` solo ejecuta lexer y parser | La CLI prueba sintaxis válida/inválida; validaciones semánticas ocurren principalmente al evaluar | GAP-026 / pendiente |
| IR/VM | No encontradas en el árbol actual | Sin módulo, binario ni pruebas de ejecución para IR/VM | GAP-027 / no implementado en este repositorio |
| Backends nativos/WASM y bindings | No encontrados en el árbol actual | Sin backends ni pruebas de interoperabilidad | GAP-015 y GAP-022 / pendientes |

**Verificación local (2026-10-28):** `cargo test --manifest-path prototipo/Cargo.toml` pasa en el árbol actual. La suite incluye pruebas de paquetes (resolución, bloqueo, versiones, ruta explícita e importación de extremo a extremo) y de cargador de módulos. Esta ejecución no certifica otras plataformas ni declara conformidad con Core 0.3 o superación de puertas de distribución. `cargo clippy` y `cargo fmt --check` no se ejecutaron en esta revisión.

**Verificación local (2026-10-31):** con FFI básico incluido: 578 pruebas, 0 fallos; `cargo clippy --all-targets -- -D warnings` limpio; `cargo fmt --check` OK (prototipo/Cargo.toml).

**Verificación R6 (2026-10-31):** servidor `basilisco-registry` contra PostgreSQL 16 real: migraciones aplicadas, bootstrap admin + clave Ed25519, publicación BPK (SHA-256 + Ed25519) → 201, descarga con SHA correcto, instalación con `bsk instalar-remoto` → `verificado=true`, supervivencia a reinicio del servidor+BD confirmada. `cargo test` → 580 pasadas, 0 fallos; clippy y fmt limpios.

**Verificación R6 (2026-10-31, cierre definitivo):** cadena de confianza corregida (servidor re-firma con su clave), validación BPK completa conforme R2 (TOML estricto, SHA-256/tamaños por archivo, NFC, ZIP safety), SemVer y expiración estrictos, persistencia atómica con `create_new`, test E2E automatizado `tests/registry_integration.rs` (publicación→descarga→instalación→verificación) pasa contra PostgreSQL real. `cargo test` verde (exit 0), clippy y fmt limpios. R6 CERRADA.

**Brechas que permanecen abiertas:** GAP-001 a GAP-009 siguen requiriendo cierre por separado en norma, pruebas y código; GAP-008 se cierra parcialmente para los casos básicos de arreglos/listas, no para iteración completa. GAP-010, GAP-014, GAP-016 y GAP-017 tienen implementación prototipal, no cierre completo. GAP-026 (análisis semántico estático) y GAP-027 (IR/VM) no están implementados. Las extensiones prototipales necesitan criterios de límite y conformidad antes de entrar en una garantía de versión.
