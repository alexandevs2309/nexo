# Plan maestro de auditoría, alcance y brechas de BASILISCO

- **Estado:** documento de trabajo vivo; no implica que las tareas estén completadas.
- **Propósito:** conservar el alcance observado, los riesgos, las brechas y el orden recomendado de trabajo.
- **Repositorio:** `alexandevs2309/nexo`; implementación Rust en `prototipo/`.
- **Versión observada en el manifiesto consultado:** `basilisco 0.6.0`.
- **Última revisión documental inicial:** 2026-10-29.
- **Regla de evidencia:** distinguir código presente, pruebas ejecutadas, requisitos normativos y objetivos futuros. No declarar una característica completa solo porque existe un nodo AST o una función con ese nombre.

> Este documento recoge una auditoría estática del repositorio y sus documentos. La revisión inicial no ejecutó el conjunto de pruebas en el entorno de auditoría. El registro de brechas del proyecto informa una ejecución local anterior de `cargo test`; debe repetirse contra el commit que se vaya a publicar.

## 1. Resumen ejecutivo

BASILISCO es un intérprete escrito en Rust con lexer, parser, AST, evaluador, tipos básicos, funciones nombradas, control de flujo, colecciones básicas, E/S, manejo prototipal de errores, módulos, CLI y resolución local de paquetes.

La base es real y útil para scripts, automatización local y programas de consola. Sin embargo, el lenguaje y su ecosistema todavía no deben presentarse como terminados ni como plenamente multipropósito. Los principales trabajos son:

1. definir una fuente normativa única y sincronizar la documentación con el código;
2. verificar y completar el análisis semántico estático y las reglas de tipos;
3. diseñar e implementar funciones de primera clase, llamadas indirectas y después cierres;
4. completar colecciones, errores, E/S y módulos;
5. formalizar CLI, estructura de proyecto, extensiones, gestor de paquetes y registro;
6. mejorar pruebas de conformidad, CI, diagnósticos y documentación;
7. decidir con evidencia si hacen falta IR, VM, compilación nativa, WASM o FFI.

## 2. Cómo interpretar los estados

Usar estas etiquetas en el proyecto:

- **IMPLEMENTADO:** existe código y hay pruebas pertinentes; indicar el alcance probado.
- **PARCIAL:** existe código, pero hay límites conocidos, cobertura incompleta o falta cierre normativo.
- **PROPUESTO:** diseño recomendado que aún requiere decisión o implementación.
- **NO IMPLEMENTADO:** no se encontró una implementación integrada en el árbol inspeccionado.
- **VERIFICACIÓN PENDIENTE:** el código puede existir, pero no se ha demostrado en el commit/plataforma objetivo.
- **BLOQUEADO:** depende de una decisión, infraestructura o tarea anterior.

No utilizar porcentajes globales de completitud sin definir una matriz de requisitos ponderada y verificable.

## 3. Alcance observado

### 3.1 Núcleo y ejecución

- Intérprete de AST escrito en Rust.
- Lexer y parser propios.
- CLI con binarios `basilisco` y `bsk`.
- Extensión canónica documentada: `.bsk`; `.nexo` se conserva como legado; `.nx` no se acepta.
- Comandos observados en CLI: ejecución, demostración, ayuda, versión, creación de archivos/proyectos, análisis, formato, inicialización, pruebas y gestión local de paquetes.
- No se encontró una IR ni una VM integradas en el árbol inspeccionado.

### 3.2 Tipos y expresiones

Tipos presentes en la implementación: `entero` (i64), `decimal` de punto fijo con escala 9, `real` (f64), `lógico`, `texto`, `nulo`, arreglos, listas, archivos y módulos como valores internos del runtime.

Hay reglas explícitas para overflow entero, división por cero, conversiones implícitas permitidas, comparación de tipos y diagnósticos de ejecución.

Debe mantenerse una distinción clara entre tipos del lenguaje visibles al usuario y valores internos del runtime (por ejemplo, archivo y módulo). La existencia de `TipoSem` no demuestra por sí sola cobertura exhaustiva.

### 3.3 Sentencias y flujo

Se observan declaraciones mutables/inmutables, constantes, asignaciones, bloques, condiciones, ciclos, funciones nombradas, retorno, salida de texto, iteración, rangos y extensiones de control de errores. Parte de las extensiones está expresamente fuera de la garantía del perfil normativo v0.4.

### 3.4 Funciones

- Funciones nombradas, parámetros, retorno y recursividad: implementados en el prototipo.
- Anotaciones e inferencia parcial de tipos: presentes.
- Validación de aridad, argumentos y retorno: existe en runtime y hay lógica semántica estática.
- Funciones anidadas: no admitidas según el alcance documentado.
- Funciones de primera clase, funciones anónimas y cierres: no deben declararse completos sin pruebas end-to-end. El tipo de valor inspeccionado no muestra una variante dedicada a funciones como valor.
- El AST incluye llamadas indirectas y acceso a miembros, pero esos nodos no equivalen por sí solos a un modelo completo de funciones como valores.

### 3.5 Colecciones, E/S y errores

- Arreglos de tamaño fijo y listas dinámicas: soporte básico.
- Indexación y asignación por índice: presentes.
- Iteración, rangos, `romper` y `continuar`: extensión prototipal con límites documentados.
- E/S de texto/archivos y `intentar`/`rescatar`: soporte parcial.
- Concurrencia: primitivas `iniciar_tarea`/`esperar_tarea` basadas en hilos, sin canales/cancelación/sincronización de usuario; no declararla apta para producción sin suite dedicada.

### 3.6 Módulos y paquetes

- Cargador de módulos con caché, exportaciones e importaciones.
- Manifiesto `basilisco.toml` y lockfile `basilisco.lock`.
- Resolución local de dependencias en rutas configuradas, `paquetes/` y `vendor/`.
- No equivale todavía a un ecosistema completo: la resolución remota, integridad/verificación, dependencias transitivas y semver completo siguen siendo brechas según el registro del proyecto.
- Deben reconciliarse las interfaces de manifiesto del proyecto y de paquetes: evitar dos formatos incompatibles llamados `basilisco.toml`.

## 4. Registro de brechas y plan de trabajo

Las prioridades son recomendaciones de ingeniería, no una afirmación de que el código actual falle en todos los casos. Cada tarea se cierra con pruebas, documentación y revisión del commit.

### P0 — Coherencia normativa y validación

#### GAP-A: fuente de verdad de la especificación
- [ ] Definir qué documento es normativo para cada versión.
- [ ] Separar núcleo estable, extensiones prototipales y propuestas.
- [ ] Resolver discrepancias entre README, plan de versión, perfil de conformidad y registro de brechas.
- [ ] Definir precedencia/gramática, conversiones, mutabilidad, ámbitos, retornos, errores y reglas de evaluación.
- [ ] Para cada decisión, registrar ejemplos válidos e inválidos y pruebas.
- **Criterio de cierre:** documentación y pruebas describen el mismo comportamiento para el commit publicado.

#### GAP-B: análisis semántico estático
- [ ] Ejecutar y auditar el comando `basilisco analizar` en el commit actual.
- [ ] Verificar variables indefinidas, declaraciones duplicadas, inmutabilidad, ámbitos, llamadas, aridad, parámetros, retornos y tipos incompatibles.
- [ ] Verificar que los errores detectables estáticamente no requieran ejecutar el programa.
- [ ] Definir cómo se trata `Desconocido`: aceptación gradual explícita o error, nunca comportamiento accidental.
- [ ] Revisar ramas, ciclos, retornos parciales, recursión y llamadas anticipadas.
- [ ] Asegurar ubicación archivo/línea/columna y evitar diagnósticos inconsistentes o duplicados.
- [ ] Añadir suite positiva/negativa independiente del evaluador.
- **Criterio de cierre:** matriz de reglas con pruebas automatizadas y diferencias runtime/estático explicadas.

#### GAP-C: contrato del sistema de tipos
- [ ] Documentar conversiones implícitas y explícitas.
- [ ] Definir inferencia para declaraciones y retornos.
- [ ] Definir tipos de funciones y compatibilidad de firmas.
- [ ] Definir tipos de arreglos/listas, listas vacías, indexación, asignación y mutabilidad.
- [ ] Definir nulabilidad y el papel de `nulo`.
- [ ] Probar que el analizador y el runtime rechacen los mismos programas inválidos cuando la regla sea estática.
- **Criterio de cierre:** tabla normativa de tipos + pruebas de conformidad.

### P0/P1 — Funciones de primera clase

#### GAP-D: valores función, llamadas indirectas y cierres
- [ ] Diseñar representación runtime para funciones como valores.
- [ ] Añadir tipos de función con parámetros y retorno.
- [ ] Permitir asignar una función nombrada a una variable.
- [ ] Permitir pasar funciones como argumentos y devolverlas como resultado.
- [ ] Definir llamadas indirectas y validar que el callee sea invocable.
- [ ] Decidir e implementar funciones anónimas.
- [ ] Diseñar captura léxica de variables y semántica de cierres antes de implementarla.
- [ ] Definir mutabilidad, duración de valores capturados, recursión y errores de referencia.
- [ ] Añadir pruebas: callback, map/aplicar, composición, función retornada, cierre, tipos incorrectos y aridad incorrecta.
- [ ] Actualizar gramática, AST, evaluador, analizador, formateador y documentación.
- **Criterio de cierre:** ejemplos ejecutables en CLI y suite positiva/negativa; no basta con que el parser acepte la sintaxis.

### P1 — Completar las capacidades del lenguaje

#### GAP-E: colecciones e iteración
- [ ] Definir operaciones básicas de lista/arreglo y sus costes/semántica.
- [ ] Cerrar casos de listas vacías, elementos incompatibles, índices inválidos y mutación durante iteración.
- [ ] Definir iteración de texto y comportamiento Unicode.
- [ ] Probar ciclos anidados, rangos, límites enteros y señales de control a través de manejo de errores.
- **Criterio de cierre:** perfil de conformidad para colecciones e iteración.

#### GAP-F: errores y recursos
- [ ] Definir qué errores captura `intentar`/`rescatar` y cuáles no.
- [ ] Definir propagación, retorno, `romper` y `continuar` dentro de bloques de rescate.
- [ ] Probar bloques anidados y sintaxis inválida.
- [ ] Definir cierre de archivos, errores de permisos, UTF-8 y comportamiento tras fallo.
- [ ] Evitar que un error del usuario termine en pánico interno.
- **Criterio de cierre:** semántica normativa y suite de errores de E/S/control.

#### GAP-G: módulos y paquetes
- [ ] Unificar el formato de manifiesto de proyecto y el de dependencias.
- [ ] Definir imports relativos al archivo importador, visibilidad y exportaciones.
- [ ] Definir inicialización, ciclos de importación y errores de carga.
- [ ] Completar dependencias transitivas, resolución de versiones y lockfile reproducible.
- [ ] Diseñar integridad/hash y política de confianza para paquetes.
- [ ] Separar el instalador local actual de un futuro registro remoto.
- **Criterio de cierre:** pruebas end-to-end de proyectos con varias dependencias, versiones, conflictos y fallos.

#### GAP-H: biblioteca estándar
- [ ] Inventariar y documentar cada función integrada: firma, tipos, errores y ejemplos.
- [ ] Definir API estable para texto, colecciones, archivos, rutas, entorno, fechas, JSON y red según el alcance aprobado.
- [ ] No añadir APIs a la sintaxis central si corresponden a la biblioteca estándar.
- [ ] Definir compatibilidad de biblioteca por versión del lenguaje.
- **Criterio de cierre:** catálogo versionado y pruebas de API.

### P1 — Herramientas y calidad

#### GAP-I: CLI y experiencia de proyecto
- [ ] Definir nombres, aliases, códigos de salida y mensajes de todos los comandos.
- [ ] Unificar `nuevo`, `inicializar`, `instalar`, `paquetes`, `probar`, `analizar` y `formatear`.
- [ ] Definir búsqueda de proyecto/archivo y comportamiento de `basilisco run` sin argumentos.
- [ ] Crear un único formato oficial de proyecto y una plantilla mínima.
- [ ] Garantizar que el formateador sea idempotente y documentar la preservación de comentarios.
- [ ] Definir comandos de compilación/validación solo cuando correspondan al runtime real.
- **Criterio de cierre:** pruebas CLI en entorno limpio y guía reproducible.

#### GAP-J: pruebas, CI y publicación
- [ ] Ejecutar `cargo fmt --check`.
- [ ] Ejecutar `cargo clippy --all-targets -- -D warnings`.
- [ ] Ejecutar `cargo test` en el commit objetivo.
- [ ] Añadir pruebas de humo del binario instalado, no solo de funciones internas.
- [ ] Verificar Linux, macOS y Windows mediante CI si se declaran compatibles.
- [ ] Verificar instalación, versión, archivos de ejemplo y publicación reproducible.
- [ ] Generar informe de pruebas con commit, versión, plataforma y comandos.
- **Criterio de cierre:** CI verde en la matriz publicada y reporte adjunto al release.

#### GAP-K: documentación
- [ ] Sincronizar README, guía de instalación, tutorial, referencia de sintaxis, biblioteca estándar y hoja de ruta.
- [ ] Añadir ejemplos ejecutables y negativos.
- [ ] Marcar cada característica como estable, experimental o propuesta.
- [ ] Incluir una tabla de compatibilidad entre versión de CLI, versión de lenguaje y versión del formato de paquetes.
- **Criterio de cierre:** un usuario nuevo puede instalar, crear, ejecutar, probar e instalar una dependencia siguiendo solo la documentación oficial.

### P2 — Decisiones estratégicas

#### GAP-L: IR, VM y compilación
- [ ] No iniciar IR/VM solo por paridad con otros lenguajes.
- [ ] Medir rendimiento del intérprete y definir cargas de trabajo objetivo.
- [ ] Comparar AST interpreter, bytecode VM, compilación nativa y WASM.
- [ ] Si se aprueba IR/VM, especificar representación, errores, depuración, compatibilidad y pruebas diferenciales contra el intérprete.
- [ ] Definir FFI/ABI y bindings como proyecto separado.
- **Criterio de cierre:** ADR con evidencia de rendimiento, costes y casos de uso.

#### GAP-M: concurrencia
- [ ] Definir modelo de tareas, aislamiento, errores, cancelación y sincronización.
- [ ] Diseñar canales o primitivas equivalentes si el producto lo requiere.
- [ ] Añadir pruebas deterministas, carrera, terminación y propagación de errores.
- **Criterio de cierre:** contrato de concurrencia y suite dedicada; hasta entonces, marcar como experimental.

## 5. Orden recomendado de ejecución

1. **Auditoría reproducible del commit actual:** ejecutar fmt, clippy, tests y pruebas CLI; guardar resultados.
2. **Congelar el contrato del núcleo:** cerrar discrepancias entre código, perfil y documentación.
3. **Completar análisis semántico y tipos:** base para todas las funciones posteriores.
4. **Implementar funciones de primera clase:** primero valores función y firmas; luego cierres.
5. **Completar colecciones, errores y módulos.**
6. **Formalizar convención de proyecto, CLI y paquetes.**
7. **Ampliar biblioteca estándar y documentación.**
8. **Mejorar publicación/CI y, con métricas, decidir IR/VM/FFI.**

No iniciar varias fases semánticas grandes a la vez. Cada fase debe terminar con pruebas, documentación y un commit revisable.

## 6. Matriz mínima para dar por terminada una característica

Una característica solo puede marcarse como estable cuando tiene:

- [ ] Decisión de diseño registrada.
- [ ] Gramática y semántica documentadas.
- [ ] Implementación en lexer/parser/AST/runtime/análisis donde corresponda.
- [ ] Pruebas positivas.
- [ ] Pruebas negativas.
- [ ] Diagnósticos con ubicación cuando proceda.
- [ ] Integración en CLI y formateador, si aplica.
- [ ] Ejemplo ejecutable.
- [ ] Compatibilidad/versionado definido.
- [ ] CI verde en plataformas declaradas.
- [ ] Documentación y registro de brechas actualizados.

## 7. Evidencia y límites de esta auditoría

Se inspeccionaron archivos del repositorio relacionados con CLI, parser, evaluador, valores, cargador de módulos, paquetes, proyecto, analizador semántico, alcance del núcleo, perfil de conformidad y registro de brechas.

El registro de brechas del proyecto informa una ejecución local de `cargo test` el 2026-10-28, pero esta revisión documental no volvió a ejecutar esos comandos. Antes de una publicación debe repetirse la verificación sobre el commit exacto. Las capacidades marcadas como parciales requieren validación adicional; la existencia de código no es evidencia suficiente de conformidad.

## 8. Fuentes internas que deben mantenerse sincronizadas

- `README.md`
- `00-gobernanza/estado-del-proyecto.md`
- `02-lenguaje/alcance-del-nucleo.md`
- `10-especificacion/perfil-de-conformidad-v0.4.md`
- `10-especificacion/registro-de-brechas.md`
- `10-especificacion/estado-normativo.md`
- `08-plan/` (planes de versión y este plan maestro)
- `prototipo/src/` y `prototipo/tests/`

**Regla final:** ante una discrepancia, no asumir que el README o el código gana automáticamente. Registrar el conflicto, decidir cuál es el comportamiento normativo, añadir la prueba correspondiente y actualizar las fuentes afectadas.
