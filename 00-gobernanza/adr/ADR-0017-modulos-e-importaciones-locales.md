# ADR-0017 — Módulos e importaciones locales para BASILISCO 0.5.0

- **Estado:** Aprobado (2026-10-26) para la fase de diseño; implementación por fases.
- **Fecha:** 2026-10-26.
- **Responsable:** proyecto BASILISCO.
- **Revisión prevista:** al completar la implementación de v0.5.0, o antes si aparece evidencia que contradiga una decisión.
- **Relacionado:** ADR-0008 (funciones, revisión prevista al añadir módulos), ADR-0009 (identidad), ADR-0010 (CLI, revisión prevista al añadir módulos), ADR-0011 (tipos gradual), ADR-0012 (iteración), ADR-0013 (E/S), ADR-0014 (errores), ADR-0015 (perfil v0.4), ADR-0016 (licencia).
- **Resuelve:** ausencia de sistema de módulos, falta de identidad de archivo en diagnósticos, limitaciones de `basilisco nuevo`.

---

## 1. Contexto y requisitos

BASILISCO 0.4.0 está publicado en crates.io. El objetivo de v0.5.0 es evolucionar desde la
ejecución de archivos individuales hacia proyectos reales con módulos reutilizables.

Requisitos de producto (confirmados):

- Módulos locales e importaciones.
- Visibilidad pública y privada (privado por defecto).
- Resolución determinista de archivos.
- Diagnósticos con archivo, línea y columna.
- Compatibilidad con programas de un solo archivo.
- Un proyecto de consola mínimo y ejecutable.

Fuera de alcance: gestor de paquetes, plantillas web/desktop, distribución multiplataforma
(puede avanzar en paralelo), concurrencia, FFI.

---

## 2. Decisiones

### D1 — Sintaxis de importación y alias

**Decisión:** una sola palabra clave `usar` con dos formas complementarias:

```
usar "ruta.bsk" como alias          (* módulo completo bajo namespace *)
usar "ruta.bsk" { nombre1, nombre2 }  (* importa nombres al ámbito actual *)
```

- `como` es palabra clave reservada.
- No se introduce `desde` ni `importar` en v0.5.0.

**Alternativas evaluadas:** `importar ... como` (familiar, Python); `desde ... importar` (orden natural en español); `importar modulo` por nombre (requiere resolución por nombre/paquetes, fuera de alcance).

**Justificación:** una sola palabra clave reduce la superficie léxica y el riesgo de ambigüedad en el parser (que ya maneja ~25 palabras clave). La forma con alias mantiene el namespace explícito.

### D2 — Declaraciones públicas y privadas

**Decisión:** por defecto privado. Modificador `publico` antepuesto en nivel superior:

```
publico funcion sumar(a, b) { ... }
publico sea PI = 3.14
```

- `privado` no se introduce en v0.5.0 (el default ya es privado).
- `publico` solo tiene efecto en el nivel superior de un módulo. En bloque o función es error sintáctico.

**Justificación:** coherente con Rust (privado por defecto) y con el principio de menor privilegio del modelo de amenazas.

### D3 — Resolución de rutas relativas

**Decisión:**

- Ruta relativa al **directorio del archivo que contiene la sentencia `usar`** (no al cwd).
- Sin extensión → se añade `.bsk`. `.nexo` aceptado con aviso. `.nx` y otras → error.
- Rutas absolutas: **rechazadas** (`RutaNoPermitida`).
- La ruta se canonicaliza y se valida que quede dentro del root del proyecto (D9).

**Justificación:** la resolución relativa al archivo que contiene el `usar` es la única que hace que mover un proyecto no rompa las importaciones. Es el comportamiento de `#include` en C y `mod` en Rust.

### D4 — Sentencias de nivel superior en módulos

**Decisión (ajustada por condición 3 del usuario):** los módulos **NO ejecutan automáticamente** sentencias de nivel superior arbitrarias. Solo se permiten en el nivel superior de un módulo:

- Declaraciones (`sea`, `var`, `dato`, `const`) — procesadas al cargar.
- Definiciones de función (`funcion`) — registradas al cargar.
- Sentencias `usar` — para importaciones transitivas (Fase 4). No producen efectos secundarios directos; solo cargan y registran el módulo importado.

Cualquier otra sentencia en nivel superior de un módulo (expresión, `mostrar`, control de flujo, E/S) es **error sintáctico**:

```
'mostrar' no está permitido en el nivel superior de un módulo
```

**Justificación:** prioriza importaciones predecibles y evita efectos secundarios inesperados al importar. Un módulo es una unidad de declaraciones y funciones, no un script con efectos. Esto se documenta como decisión revisable en v0.5.1 con evidencia de uso.

**Alternativas evaluadas:** permitir sentencias de nivel superior (más flexible, pero introduce efectos secundarios al importar); prohibir todo excepto declaraciones/funciones (elegida).

### D5 — Identidad y almacenamiento de archivos fuente

**Decisión:**

- Cada fuente tiene identidad: `ruta_canonica: PathBuf` + `contenido: String`.
- Se añade `archivo: Option<String>` a `ErrorLexico`, `ErrorSintactico` y `ErrorEjecucion`.
- `Display` incluye el archivo solo si está presente:
  - Con archivo: `error ... [ruta:linea:columna]: mensaje`
  - Sin archivo: formato actual `error ... [linea:columna]: mensaje` (compatibilidad).
- Constructores `Lexer::con_archivo(fuente, archivo)` y `Parser::con_archivo(tokens, archivo)` además de los existentes `nuevo(...)` (archivo None).
- Los errores de ejecución se enriquecen por el llamador (CLI o cargador) si `archivo` es None.

**Justificación:** minimiza cambios en el evaluador (no toca ~50 call sites de `error()`), mantiene compatibilidad con los 304 tests existentes.

### D6 — Carga y registro de módulos

**Decisión:** nuevo componente `CargadorModulos` (en `prototipo/src/cargador.rs`).

- Cache por ruta canónica: `HashMap<PathBuf, ModuloCargado>`.
- `ModuloCargado`: ruta, AST, entorno, exportaciones (`HashMap<String, SimbodoExportado>`), estado (`EnCarga`/`Cargado`/`Error`).
- `cargar(ruta, archivo_actual)`: resolver, canonicalizar, validar root, cache, detección de ciclos, leer, lexar, parsear, ejecutar en entorno de módulo, registrar exportaciones (solo `publico`), cachear.
- El `Entorno` lleva `cargador: Option<Rc<RefCell<CargadorModulos>>>`.

**Estrategia del cargador (evaluación de alternativas):**

| Opción | Descripción | Pros | Contras |
|---|---|---|---|
| A1: `Rc<RefCell<CargadorModulos>>` | Compartido vía RefCell | Simple, sin cambio de firmas internas, single-thread | Panic por borrow anidado si no se cuida; no thread-safe |
| A2: Cargador como parámetro explícito | Se pasa a `ejecutar_programa`, `ejecutar_bloque`, etc. | Sin estado compartido mutable | Cambia ~10 firmas internas; alto riesgo |
| A3: Cargador global estático | Estado global | No cambia firmas | Rompe testabilidad (tests paralelos) |
| A4: `Arc<Mutex<CargadorModulos>>` | Thread-safe | Seguro en concurrencia | Overhead innecesario en single-thread |

**Decisión:** A1 para v0.5.0. Justificación: el intérprete es single-threaded; el cargador se crea una vez por ejecución; A2 es demasiado invasiva; A3 rompe testabilidad; A4 añade complejidad sin beneficio. Mitigación: borrows de corta duración, no mantener `RefMut` a través de llamadas reentrantes. Documentar como "no thread-safe". Revisable si se añade concurrencia.

### D7 — Importaciones repetidas y ciclos

**Decisión:**

- Importaciones repetidas: no es error; devuelve el módulo ya cargado; no se re-ejecuta.
- Ciclos: error `CicloDeImportacion { cadena: Vec<String> }` con la lista de archivos.
- Módulo que falla al cargar: se marca `Error` y se cachea el error; reintentar produce el mismo error.

### D8 — Orden de resolución de símbodos

**Decisión:**

Dentro de un módulo, al resolver un identificador:

1. Ámbito actual (bloque más interno).
2. Ámbitos externos hasta el global del módulo.
3. Funciones del módulo (globales).
4. Builtins.

No hay acceso implícito a símbodos de otros módulos. Solo lo importado explícitamente entra en el ámbito global del importador.

Para `alias.miembro` (Acceso): resolver `miembro` en `exportaciones` del módulo. Si no existe o no es público → error `SimbodoNoExportado` / `SimbodoPrivado`.

Colisiones: si `usar "lib.bsk" { sumar }` y `sumar` ya existe → error `ImportacionDuplicada`.

### D9 — Seguridad de rutas y root del proyecto

**Decisión:**

- Root del proyecto:
  - Si existe `basilisco.toml` en el directorio del archivo de entrada o en un ancestro → root = directorio que lo contiene.
  - Si no → root = directorio del archivo de entrada.
  - Si no hay archivo (stdin/eval) → importaciones deshabilitadas.
- Validación: la ruta canónica del módulo debe estar dentro del root (`starts_with`). Si escapa → `RutaNoPermitida`.
- Rutas absolutas: rechazadas siempre.
- Symlinks: se siguen (canonicalize los resuelve); la validación es sobre la ruta canónica final. Documentar como limitación.

### D10 — Formato del manifiesto (`basilisco.toml`)

**Evaluación (condición 4 del usuario: por robustez, mantenimiento y seguridad, no solo por evitar dependencias):**

| Criterio | Opción A: crate `toml` | Opción B: parser propio mínimo |
|---|---|---|
| Robustez | Alta: maneja todos los casos TOML, bien testeado | Media: solo los campos necesarios, casos edge no cubiertos |
| Mantenimiento | Bajo: mantenido por la comunidad, actualizaciones automáticas | Alto: código propio que mantener y auditar |
| Seguridad (cadena de suministro) | Requiere auditoría de procedencia de nueva dependencia; aumenta superficie de ataque | Sin dependencias nuevas; superficie mínima |
| Ergonomía | Alta: formato estándar, herramientas existentes | Media: formato familiar pero parser limitado |
| Tamaño del binario | Aumenta | Mínimo |
| Compatibilidad con "sin dependencias externas" (v0.4) | Rompe la política | Mantiene la política |

**Decisión:** Opción B (parser propio mínimo) para v0.5.0. Justificación: el manifiesto v0.5.0 necesita solo 3-4 campos (`nombre`, `version`, `entrada`, `descripcion` opcional). Un parser mínimo de ~50 líneas es suficiente y verificable. La seguridad de la cadena de suministro (modelo de amenazas) y el mantenimiento favorecen no añadir dependencias. **Condición de revisión:** si la evidencia de uso muestra necesidad de features TOML (arrays, tablas anidadas, comentarios complejos), migrar a `toml` con un ADR en v0.5.1.

### D11 — Compatibilidad con programas de un solo archivo

**Decisión:** un programa sin `usar` funciona exactamente como en v0.4.0. El `Entorno` principal se crea igual. El cargador está presente pero no se invoca. Los tests de conformidad (fuente sin ruta) pasan sin cambios de formato.

### D12 — Nuevos errores (`CausaError`)

Nuevas variantes para módulos/importaciones:

| Variante | Cuándo |
|---|---|
| `ModuloNoEncontrado { ruta }` | archivo no existe |
| `CicloDeImportacion { cadena: Vec<String> }` | ciclo detectado |
| `SimbodoPrivado { nombre, modulo }` | acceso a símbodo no público |
| `SimbodoNoExportado { nombre, modulo }` | nombre no existe en el módulo |
| `ImportacionDuplicada { nombre, modulo }` | colisión al importar nombre |
| `RutaNoPermitida { ruta }` | ruta absoluta o fuera del root |
| `ExtensionNoSoportada { extension }` | extensión no válida |
| `ImportacionesNoDisponibles` | `usar` sin cargador configurado |
| `SentenciaNoPermitidaEnModulo { sentencia }` | sentencia no permitida en nivel superior de módulo |

Códigos BAS: proponer `BAS-4XXX` para módulos/importaciones (extensión del catálogo propuesto, no aprobado).

---

## 3. Contrato normativo propuesto (resumen)

### Léxico (nuevos tokens)

```
"usar"     → TipoToken::Usar
"como"     → TipoToken::Como
"publico"  → TipoToken::Publico
```

### Gramática (nuevas producciones)

```
sentencia = ...
          | usar
          | declaracion_publica ;

usar = "usar" , CADENA , ( "como" , IDENT | "{" , lista_nombres , "}" ) ;

lista_nombres = IDENT , { "," , IDENT } ;

declaracion_publica = "publico" , ( declaracion | funcion ) ;

acceso = primaria , "." , IDENT ;
```

### Reglas semánticas clave

1. `usar` es una sentencia; se ejecuta en el orden en que aparece.
2. El módulo se carga (procesa declaraciones y funciones) en el momento del `usar`.
3. Un módulo se carga una sola vez por ruta canónica (cache).
4. Ciclo de importación → error `CicloDeImportacion`.
5. Solo símbodos `publico` son accesibles desde fuera.
6. `publico` solo en nivel superior.
7. Rutas relativas al archivo que contiene el `usar`; validadas contra el root.
8. Importaciones repetidas no re-procesan el módulo.
9. Módulos no ejecutan sentencias de nivel superior (solo declaraciones, funciones y `usar`).

---

## 4. Lo que este ADR no decide

- Genéricos definibles por el usuario.
- `resultado<T>` (diferido, ADR-0014).
- Gestor de paquetes y registro (fuera de alcance).
- Plantillas web/desktop.
- Concurrencia.
- `privado` explícito (diferido a v0.5.1).
- `finalmente` en `intentar`/`rescatar` (diferido).
- Compilación a bytecode/nativo (decisión abierta).
- Migración a crate `toml` (condición de revisión en D10).

---

## 5. Condiciones de revisión

- Si la evidencia de uso muestra necesidad de efectos secundarios al importar, revisar D4.
- Si la evidencia muestra necesidad de features TOML, migrar con ADR en v0.5.1 (D10).
- Si se añade concurrencia, revisar la estrategia del cargador (D6, A1).
- Si `publico` resulta insuficiente, añadir `privado` explícito.

---

## 6. Plan de implementación (resumen)

| Fase | Objetivo | Estado |
|---|---|---|
| 0 | Identidad de archivo en diagnósticos | En implementación |
| 1 | Cargador de módulos | Pendiente |
| 2 | Sintaxis `usar` y `publico` | Pendiente |
| 3 | Ejecución de importaciones y visibilidad | Pendiente |
| 4 | Seguridad de rutas | Pendiente |
| 5 | Proyecto mínimo y CLI | Pendiente |
| 6 | Documentación, aceptación y release | Pendiente |

Cada fase se cierra con pruebas reales, fmt, Clippy y build verdes, y aprobación explícita.
