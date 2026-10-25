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

| Capacidad | Código local | Cobertura de pruebas | Norma / alcance v0.4 |
|---|---|---|---|
| Núcleo numérico (entero/decimal/real) | Implementado | Suite de conformidad numérica | Core 0.3 / incluido |
| Variables, asignación, tipos anotados y ámbitos | Implementado | Variables, control y tipos | Core 0.3 + ADR-0011 / incluido |
| `si`/`sino`/`mientras`, bloques | Implementado | Conformidad y flujo de control | Core 0.3 / incluido |
| Funciones, parámetros y retorno | Implementado; tipos anotados e inferidos verificados | Suite positiva/negativa: aridad, orden de argumentos, primera llamada (incluida recursión), conversiones implícitas aprobadas, retorno anotado, `nulo` y colecciones anotadas | ADR-0008/0011; Core 0.3 no exhaustivo / perfil v0.4 candidato |
| Texto básico | Implementado | Literales, concatenación, comparación y mostrar | Core 0.3 no exhaustivo / incluido v0.4 |
| Arreglos/listas con tipos, indexación y asignación | Implementado parcialmente | Tamaño inicial, tamaño cero, tipos de elementos, lista vacía, mutabilidad, asignación/lectura de índices, índices inválidos y tipos de índice | No normados en Core 0.3 / incluido solo en forma básica |
| `para cada`, texto iterable, rangos, `romper`/`continuar` | Implementado parcialmente; binding inmutable y ámbito limpio | Regresiones cubren rangos exclusivos/inclusivos, vacíos, negativos y límites; señales a través de rescate y scope del iterador | ADR-0012 / extensión prototipal fuera de garantía v0.4; faltan ciclo anidado, iteración vacía, texto iterable y modificación durante iteración |
| `intentar`/`rescatar` | Implementado parcialmente; separado fin normal de retorno ausente | Suite cubre error capturado, fin normal, retorno de función, señales de ciclo no capturadas y nivel superior/función | ADR-0014 / extensión prototipal fuera de garantía v0.4; faltan más casos de anidamiento y sintaxis inválida |
| E/S de texto y archivos | Implementado parcialmente | Conformidad y casos de errores pendientes de revisar | ADR-0013 / extensión prototipal fuera de garantía v0.4 |
| CLI `basilisco`/`bsk` | `run`, `demo`, ayuda, versión, extensiones y diagnóstico de ejecución | Pruebas CLI positivas/negativas y aceptación integrada | ADR-0010 / incluido |
| Módulos, importaciones, paquetes, concurrencia, FFI | No disponible como capacidad completa | Sin suite de producto | GAP-010/014/015/016 / fuera de alcance |

**Verificación local (2026-10-25, código en `6a3f619`):** 302 pruebas enumeradas; formato, Clippy estricto, build y suite completa pasan localmente. El estado de CI remota no pudo verificarse desde este entorno; no declara conformidad con Core 0.3 ni superación de puertas de distribución.

**Brechas que permanecen abiertas:** GAP-001 a GAP-009 siguen requiriendo cierre por separado en norma, pruebas y código; GAP-008 se cierra parcialmente para los casos básicos de arreglos/listas, no para iteración completa. Las extensiones marcadas prototipales necesitan los criterios de límite indicados por ADR-0012 a ADR-0014 antes de entrar en una garantía de versión.
