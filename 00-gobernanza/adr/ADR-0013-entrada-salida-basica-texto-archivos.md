# ADR-0013 — E/S básica: entrada estándar, operaciones de texto y archivos

- Estado: **Aprobado** (2026-10-24)
- Fecha: 2026-10-24
- Responsable: proyecto BASILISCO
- Revisión prevista: al completar la implementación del incremento 7.
- Prerequisito: ADR-0011 aprobado (tipo `nulo`, funciones integradas con tipo de retorno).
- Relacionado: ADR-0003 (palabras clave en español), ADR-0005 (conversiones numéricas),
  ADR-0008 (funciones), ADR-0011 (tipos), ADR-0014 (manejo de errores).
- Resuelve brechas: GAP-007 (cadenas — operaciones), GAP-013 (E/S y permisos).
- Cubre decisiones de `decisiones-que-no-deben-inventarse.md`: "sandbox, permisos,
  acceso a red y archivos" (parcialmente, solo archivos locales).

---

## 1. Contexto y requisitos

El tipo `texto` está implementado como valor de primera clase. Hay concatenación (`+`),
comparación (`==`, `!=`, `<`, `>`, etc.) y `mostrar`. No hay ninguna operación sobre
cadenas ni ninguna forma de leer datos del exterior. Un programa BASILISCO no puede
interactuar con el mundo más allá de `mostrar`.

Requisitos:
- R1: Leer texto de la entrada estándar.
- R2: Operaciones básicas sobre valores de tipo `texto`.
- R3: Acceso a archivos locales con ciclo de vida explícito.
- R4: Modos de archivo en español: `"leer"`, `"escribir"`, `"agregar"`.
- R5: El tipo `archivo` es un tipo dedicado del lenguaje con representación interna
  encapsulada; no se exponen detalles del sistema operativo.
- R6: Todas las capacidades nuevas son **funciones integradas**, no sintaxis nueva.
- R7: Los errores siguen el contrato de ADR-0005 §1.6: ningún error silencioso.

---

## 2. Opciones consideradas para la API

### O1 — Funciones integradas (propuesta seleccionada)
`leer()`, `longitud(t)`, `abrir(ruta, modo)`, etc. Son identificadores normales que
el evaluador reconoce como funciones integradas (builtins).
- Ventajas: coherente con el modelo actual; sin nueva sintaxis; el programador puede
  inspeccionar el nombre con `mostrar`.
- Costes: los nombres de funciones integradas deben reservarse para evitar conflictos
  con funciones de usuario.
- **Seleccionada**.

### O2 — Métodos sobre el tipo (`texto.longitud()`)
Requiere un modelo de objetos o dispatch que el lenguaje no tiene. Introduce conceptos
(objeto, método, receptor) que aún no están decididos.
- **Descartada**: introduce decisiones de diseño no aprobadas.

### O3 — Sentencias dedicadas (`leer nombre`, `abrir archivo como f`)
Nueva sintaxis para cada operación. Más verboso; más difícil de componer.
- **Descartada**: innecesariamente complejo cuando las funciones cubren el caso.

---

## 3. Decisión

### 3.1 Infraestructura de funciones integradas

Se introduce el concepto de **función integrada** (builtin): una función que el
evaluador reconoce por nombre y cuyo cuerpo está implementado en Rust, no en BASILISCO.

Reglas:
- Los nombres de funciones integradas están **reservados**: un usuario no puede definir
  una función con el mismo nombre. Intentarlo produce error `NombreReservado { nombre }`.
- Las funciones integradas participan en la tabla de llamadas exactamente igual que las
  funciones de usuario. La sintaxis de llamada es idéntica: `nombre(args)`.
- El evaluador busca primero en funciones de usuario y después en funciones integradas.
  (Esto garantiza que, si en el futuro se elimina o renombra un builtin, el código de
  usuario que define una función con ese nombre no queda misteriosamente roto.)

  **Aclaración de precedencia**: la búsqueda es usuario-primero, builtin-segundo. Pero
  los nombres listados en este ADR son reservados y el parser los rechaza si se intentan
  definir como nombre de función. Estas dos reglas no contradicen: "reservado" significa
  "no puedes nombrarlo"; la búsqueda usuario-primero es el mecanismo de dispatch en
  tiempo de ejecución para nombres no reservados.

### 3.2 Entrada estándar

| Función | Tipo de retorno | Comportamiento |
|---------|----------------|----------------|
| `leer()` | `texto` | Lee una línea de stdin hasta `\n` (excluido). Si el stream está cerrado, retorna `""`. |
| `leer_entero()` | `entero` | Lee una línea y convierte a `entero`. Si la conversión falla → error `EntradaInvalida { esperado: "entero", obtenido: texto }`. |
| `leer_decimal()` | `decimal` | Ídem para `decimal`. |
| `leer_real()` | `real` | Ídem para `real`. |

Notas:
- `leer()` nunca falla con error de ejecución (devuelve `""` ante EOF).
- `leer_entero/decimal/real()` fallan con `EntradaInvalida` si la cadena no es
  convertible. Esto permite capturarlos con `intentar`/`rescatar` (ADR-0014).
- El prompt al usuario (texto antes de leer) se hace con `mostrar_sin_salto` (§3.3).

### 3.3 `mostrar` consolidado y `mostrar_sin_salto`

`mostrar expresion` ya está implementado (ADR-0008 §2). Se consolida su contrato y se
añade una variante:

| Función/Sentencia | Comportamiento |
|-------------------|----------------|
| `mostrar expresion` | Imprime la representación de texto del valor seguida de `\n`. Acepta cualquier tipo. |
| `mostrar_sin_salto expresion` | Igual pero sin `\n` al final. Útil para prompts: `mostrar_sin_salto "Ingresa tu nombre: "`. |

`mostrar` sigue siendo una sentencia (ya aprobada); `mostrar_sin_salto` se introduce como
sentencia equivalente con la misma gramática. No se introduce como función integrada para
mantener la simetría con `mostrar`.

Nuevo token: `MostrarSinSalto` para la palabra clave `mostrar_sin_salto`.

### 3.4 Operaciones sobre `texto`

Todas son funciones integradas. Los parámetros y retornos siguen el sistema de tipos de
ADR-0011.

#### Inspección

| Función | Firma | Descripción |
|---------|-------|-------------|
| `longitud(t)` | `texto -> entero` | Número de caracteres Unicode (puntos de código), no de bytes. |
| `contiene(t, sub)` | `(texto, texto) -> logico` | Verdadero si `sub` aparece en `t`. |
| `empieza_con(t, pre)` | `(texto, texto) -> logico` | Verdadero si `t` empieza con `pre`. |
| `termina_con(t, suf)` | `(texto, texto) -> logico` | Verdadero si `t` termina con `suf`. |
| `es_vacio(t)` | `texto -> logico` | Equivalente a `longitud(t) == 0`. |

#### Transformación

| Función | Firma | Descripción |
|---------|-------|-------------|
| `mayusculas(t)` | `texto -> texto` | Convierte a mayúsculas (Unicode). |
| `minusculas(t)` | `texto -> texto` | Convierte a minúsculas (Unicode). |
| `recortar(t)` | `texto -> texto` | Elimina espacios Unicode al inicio y al final. |
| `reemplazar(t, buscar, nuevo)` | `(texto, texto, texto) -> texto` | Reemplaza todas las ocurrencias de `buscar` por `nuevo`. |
| `subcadena(t, inicio, fin)` | `(texto, entero, entero) -> texto` | Subcadena desde `inicio` hasta `fin` (índices de punto de código, exclusivo el fin). `inicio < 0` o `fin > longitud(t)` → error `IndiceTextoFueraDeRango`. |

#### Conversión entre tipos y texto

| Función | Firma | Descripción |
|---------|-------|-------------|
| `dividir(t, sep)` | `(texto, texto) -> lista<texto>` | Divide `t` por `sep`. Si `sep` es `""` → error `SeparadorVacio`. |
| `unir(lista, sep)` | `(lista<texto>, texto) -> texto` | Une los elementos de `lista` con `sep` entre ellos. |
| `a_entero(t)` | `texto -> entero` | Convierte `t` a entero. Falla con `ConversionTextoInvalida` si no es válido. |
| `a_decimal(t)` | `texto -> decimal` | Ídem para `decimal`. |
| `a_real(t)` | `texto -> real` | Ídem para `real`. |
| `texto(valor)` | `cualquier_tipo -> texto` | Representación de texto de cualquier valor. |

**Nota sobre `dividir`**: su tipo de retorno es `lista<texto>`. El tipo de retorno
depende del argumento, lo que es un caso simple de genérico. Se implementa como caso
especial de función integrada sin necesidad de genéricos definibles por el usuario.

### 3.5 Tipo `archivo`

Se introduce `archivo` como nuevo tipo primitivo del lenguaje con las siguientes
garantías:

**Ciclo de vida:**
1. Se crea con `abrir(ruta, modo)`.
2. Se usa con las funciones de operación.
3. Se libera con `cerrar(f)`.
4. Después de `cerrar`, cualquier uso produce error `ArchivoYaCerrado`.

**Propiedad:** el valor `archivo` puede almacenarse en una variable y pasarse a
funciones. No se puede copiar (si se asigna a otra variable, ambas referencian el
mismo archivo — comportamiento de alias; la primera en cerrarse invalida la otra).

**Representación interna:** opaca. El programa no puede acceder al descriptor de
archivo del sistema operativo ni a ningún detalle interno. Solo puede usar las
funciones de este ADR.

**Comportamiento ante fallos:** todos los errores de archivo son errores de ejecución
con `CausaError` tipada (§3.7). Se pueden capturar con `intentar`/`rescatar` (ADR-0014).

**Funciones de archivo:**

| Función | Firma | Descripción |
|---------|-------|-------------|
| `abrir(ruta, modo)` | `(texto, texto) -> archivo` | Abre el archivo. `modo` ∈ `{"leer", "escribir", "agregar"}`. |
| `leer_linea(f)` | `archivo -> texto` | Lee la siguiente línea (sin `\n`). Al llegar al EOF retorna `""`. |
| `leer_todo(f)` | `archivo -> texto` | Lee el contenido completo del archivo. |
| `escribir(f, t)` | `(archivo, texto) -> nulo` | Escribe `t` en el archivo. |
| `escribir_linea(f, t)` | `(archivo, texto) -> nulo` | Escribe `t` seguido de `\n`. |
| `cerrar(f)` | `archivo -> nulo` | Cierra el archivo y libera el recurso. |
| `fin_de_archivo(f)` | `archivo -> logico` | `verdadero` si el cursor está al final. |

**Modos:**

| Modo | Semántica |
|------|-----------|
| `"leer"` | Solo lectura. El archivo debe existir; si no → `ArchivoNoEncontrado`. |
| `"escribir"` | Escritura. Crea el archivo si no existe; si existe, **trunca** el contenido. |
| `"agregar"` | Escritura al final. Crea el archivo si no existe; si existe, no trunca. |

**Restricciones de alcance para Puerta C:**
- Solo archivos locales; sin acceso a red en este ADR.
- Sin bloqueo de archivos entre procesos; sin permisos de acceso; sin operaciones
  asíncronas. Estas capacidades se definen en un ADR de E/S avanzada posterior.
- Rutas relativas son relativas al directorio de trabajo del proceso.
- No se permite acceso fuera del directorio de trabajo sin ruta absoluta explícita
  (restricción de seguridad provisional; se revisa con el modelo de permisos de GAP-020).

### 3.6 Nuevos valores en `Valor`

```rust
Valor::Archivo(ArchivoInterno)
```

Donde `ArchivoInterno` es una estructura opaca que encapsula el `std::fs::File` de Rust
y el estado de cierre. No se expone en la API pública del tipo `Valor`. El
`nombre_tipo()` de `Valor::Archivo` retorna `"archivo"`.

Operaciones sobre `Valor::Archivo`:
- Aritmética: error `TiposIncompatibles`.
- Comparación: error `TiposIncompatibles`.
- `mostrar`: imprime `"<archivo>"`.
- Pasar como argumento: válido (se pasa la referencia interna).

### 3.7 Nuevos errores (`CausaError`)

```
EntradaInvalida    { esperado: String, obtenido: String }
ArchivoNoEncontrado { ruta: String }
PermisosDenegados  { ruta: String }
ErrorDeArchivo     { ruta: String, detalle: String }
ArchivoYaCerrado   { }
IndiceTextoFueraDeRango { indice: i64, longitud: usize }
ConversionTextoInvalida { valor: String, esperado: String }
SeparadorVacio     { }
NombreReservado    { nombre: String }
```

Todos llevan ubicación (línea/columna). Deben sincronizarse con
`02-lenguaje/catalogo-de-errores-y-diagnosticos.md`.

---

## 4. Gramática ampliada

No se añaden nuevas construcciones sintácticas salvo `mostrar_sin_salto`:

```ebnf
sentencia = ...
          | "mostrar_sin_salto" , expresion ;
```

Nuevo token: `TipoToken::MostrarSinSalto`.

Las funciones integradas son llamadas de función normales; no requieren gramática nueva.

---

## 5. Nombres reservados de funciones integradas

Los siguientes nombres quedan reservados y no pueden usarse como nombre de función
de usuario ni como identificador de variable:

```
leer  leer_entero  leer_decimal  leer_real
longitud  contiene  empieza_con  termina_con  es_vacio
mayusculas  minusculas  recortar  reemplazar  subcadena
dividir  unir  a_entero  a_decimal  a_real  texto
abrir  leer_linea  leer_todo  escribir  escribir_linea
cerrar  fin_de_archivo
```

La reserva se implementa en el evaluador (no en el lexer): estos identificadores son
tokens `Identificador` normales léxicamente, pero el evaluador los rechaza cuando
aparecen en el lado izquierdo de una declaración `funcion`.

---

## 6. Compatibilidad y migración

- **Sin rotura de código existente** [INFERRED]: ningún nombre de los listados en §5
  aparece en los tests actuales como nombre de función o variable. Requiere verificación
  con grep antes de implementar.
- **`mostrar` no cambia** su comportamiento.
- **Nuevo token `MostrarSinSalto`**: `mostrar_sin_salto` actualmente sería un
  identificador; al volverse palabra clave, el código con ese nombre queda roto.
  [INFERRED: improbable dado el nombre compuesto con guion bajo].

---

## 7. Criterios de aceptación

### Entrada estándar
- `leer()` retorna la línea leída (prueba con proceso separado o inyección de stdin).
- `leer_entero()` con entrada `"42\n"` → `Entero(42)`.
- `leer_entero()` con entrada `"abc\n"` → error `EntradaInvalida`.

### Operaciones de texto
- `longitud("hola")` → `4`; `longitud("ñ")` → `1` (punto de código, no byte).
- `contiene("hola mundo", "mundo")` → `verdadero`.
- `empieza_con("hola", "ho")` → `verdadero`.
- `termina_con("hola", "la")` → `verdadero`.
- `mayusculas("hola")` → `"HOLA"`.
- `minusculas("HOLA")` → `"hola"`.
- `recortar("  hola  ")` → `"hola"`.
- `reemplazar("hola mundo", "mundo", "BASILISCO")` → `"hola BASILISCO"`.
- `subcadena("hola", 1, 3)` → `"ol"`.
- `subcadena("hola", 0, 10)` → error `IndiceTextoFueraDeRango`.
- `dividir("a,b,c", ",")` → lista `["a", "b", "c"]`.
- `unir(["a", "b", "c"], "-")` → `"a-b-c"`.
- `a_entero("42")` → `42`; `a_entero("abc")` → error `ConversionTextoInvalida`.
- `texto(42)` → `"42"`; `texto(verdadero)` → `"verdadero"`; `texto(nulo)` → `"nulo"`.

### Archivos
- `abrir("archivo.txt", "escribir")` crea el archivo; `escribir_linea(f, "hola")`; `cerrar(f)`.
- `abrir("archivo.txt", "leer")` + `leer_linea(f)` → `"hola"`.
- `abrir("no_existe.txt", "leer")` → error `ArchivoNoEncontrado`.
- Modo inválido → error `ErrorDeArchivo` con mensaje claro.
- `cerrar(f)` + `leer_linea(f)` → error `ArchivoYaCerrado`.
- Declarar `funcion leer() { ... }` → error `NombreReservado`.

### Regresión
- Las 268 pruebas existentes siguen en verde.

---

## 8. Archivos a modificar (plan; sin cambios ahora)

| Archivo | Cambio |
|---------|--------|
| `prototipo/src/token.rs` | Añadir `TipoToken::MostrarSinSalto` |
| `prototipo/src/lexer.rs` | Reconocer `mostrar_sin_salto` |
| `prototipo/src/parser.rs` | Sentencia `mostrar_sin_salto` |
| `prototipo/src/evaluador.rs` | Infraestructura de builtins; implementar todas las funciones integradas; nuevas causas de error |
| `prototipo/src/valor.rs` | Añadir `Valor::Archivo(ArchivoInterno)`; actualizar `nombre_tipo`, `Display` |
| `prototipo/tests/conformidad_texto_ops.rs` | Nueva suite — operaciones de texto |
| `prototipo/tests/conformidad_io.rs` | Nueva suite — entrada estándar y archivos |
| `02-lenguaje/catalogo-de-errores-y-diagnosticos.md` | Añadir nuevos errores |

---

## 9. Riesgos y condiciones de revisión

| Riesgo | Probabilidad | Mitigación |
|--------|-------------|------------|
| `Valor::Archivo` contiene estado mutable (el archivo abierto); la semántica de alias puede ser sorprendente | Media | Documentar el comportamiento de alias; añadir prueba que demuestra el comportamiento |
| Las funciones de texto con Unicode pueden ser lentas o incorrectas para algunos caracteres (ligaduras, emojis, texto de derecha a izquierda) | Media | `longitud` cuenta puntos de código, no grafemas; documentar como limitación conocida |
| Restricción de ruta relativa puede limitar casos de uso legítimos | Baja | Revisable con evidencia; la restricción es provisional |
| Los nombres reservados en §5 podrían colisionar con nombres de biblioteca estándar futura | Media | Los nombres son revisables; la lista de §5 se puede ampliar con un ADR posterior |

**Condiciones de revisión:**
- Al implementar el modelo de permisos (GAP-020), revisar las restricciones de ruta.
- Al implementar la biblioteca estándar (GAP-018), revisar los nombres reservados.
- Si hay evidencia de que `longitud` por puntos de código confunde a los usuarios
  (esperan grafemas), se documenta como advertencia en el catálogo de diagnósticos.

---

## 10. Lo que este ADR no decide

- Acceso a red (sockets, HTTP, etc.).
- E/S binaria (lectura de bytes crudos).
- Rutas con validación de seguridad completa.
- Archivos temporales, directorios, permisos, metadatos.
- Entrada/salida asíncrona.
- Codificaciones distintas de UTF-8 en archivos.
- Formatos estructurados (JSON, CSV, etc.) — biblioteca estándar futura.
