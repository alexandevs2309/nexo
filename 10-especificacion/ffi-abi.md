# FFI / ABI básico

## Estado y alcance
Implementado en el prototipo (`prototipo/src/evaluador.rs`) como puente de **ABI C**
sobre las bibliotecas `libffi` (llamadas dinámicas con tipos mixtos) y
`libloading` (carga y descarga de `.so`/`.dll`). Cubierto por pruebas en
`prototipo/tests/ffi.rs` (compilan una biblioteca C en tiempo de ejecución).

**Fuera de alcance actual:** callbacks hacia BASILISCO, paso de estructuras
(`struct`), símbolos con aridad variádica C (`...`), y LLVM/JIT.

## Funciones integradas

| Función | Firma | Retorna |
|---|---|---|
| `cargar_ffi(ruta)` | 1 argumento, ruta de la biblioteca (`.so`/`.dll`) | `entero` (identificador) |
| `llamar_ffi(id, "símbolo", [args], "tipo_retorno"?)` | 3 o 4 argumentos | depende de `tipo_retorno` |
| `descargar_ffi(id)` | 1 argumento | `nulo` |

El cuarto argumento de `llamar_ffi` es el tipo de retorno declarado; si se
omite, se asume `entero`. Los tipos de retorno aceptados son:
`entero` (por omisión), `logico`, `real`, `decimal`, `texto`, `nulo` / `vacío`.

Errores: `E1040` (`CausaError::FFIInvalido`) con mensaje
`error de FFI: {detalle}` para carga fallida, símbolo inexistente, argumento
de tipo no aceptado, tipo de retorno desconocido, id inexistente, puntero
nulo en retorno de texto o resultado no representable en `decimal`.

## Mapeo de tipos (ABI C)

El tipo del lado C debe **coincidir con el literal que pasa el programa**:
un argumento `entero` de BASILISCO se pasa como `long long` (64 bits con
signo); un argumento `real`/`decimal` se pasa como `double` de 64 bits
IEEE 754.

| Valor BASILISCO | Tipo C supuesto |
|---|---|
| `Entero` | `int64_t` / `long long` |
| `Real` | `double` |
| `Decimal` (entero escalado /10^9) | `double` |
| `Texto` | `const char *` (puntero UTF‑8 terminado en NUL) |

| tipo_retorno | Tipo C supuesto | Resultado BASILISCO |
|---|---|---|
| `entero` | `int64_t` | `entero` (el valor C tal cual) |
| `logico` | `int64_t` | `lógico`: distinto de 0 es verdadero |
| `real` | `double` | `real` |
| `decimal` | `double` | `decimal` escalado ×10^9; error si no es finito o se desborda |
| `texto` | `const char *` | `texto` (copia UTF‑8; ver convención de propiedad) |
| `nulo` / `vacío` | `void` | `nulo` |

Ejemplos: una función C `double media(double, double)` se llama con
`llamar_ffi(lib, "media", [2.0, 4.0], "real")` (literales `real`); si se le
pasan literales `entero` E1040 no ocurre pero el resultado será incorrecto,
porque ambos lados suponen tamaños de registro distintos. La regla es: la
firma C manda y el programa BASILISCO debe emitir literales del tipo
equivalente.

## Convención de propiedad de textos

- **Argumentos de texto:** BASILISCO crea un `CString` y pasa el puntero;
  el callee NO debe conservar el puntero más allá de la llamada ni
  liberarlo (la memoria la posee el calling code de BASILISCO).
- **Retorno de texto:** se copia el contenido del puntero devuelto y **no
  se libera**. Por lo tanto: solo es seguro para cadenas estáticas o
  gestionadas por el callee; un puntero devuelto por `malloc` y jamás
  liberado deja una fuga (se acepta como límite de este puente básico).
- Un puntero nulo en retorno de texto produce E1040.

## Gestión de bibliotecas

- `cargar_ffi` devuelve un identificador incremental global; la biblioteca
  vive en un registro interno (`HashMap<i64, Arc<Library>>`) hasta
  `descargar_ffi`.
- El `Arc` mantiene viva la biblioteca durante una llamada aunque otro
  hilo la descargue (no hay desreferencia de símbolo tras `dlclose`).
- `llamar_ffi` con un id no cargado (por ejemplo tras `descargar_ffi`, o
  un id inventado) produce E1040 `no hay biblioteca cargada con id N`.

## Límites declarados

1. Sin callbacks: una función C no puede llamar de vuelta a BASILISCO.
2. Sin structs: solo tipos escalares y punteros a texto de la tabla.
3. Sin variádicos C: la firma debe fijar la cantidad de argumentos.
4. Sin manejo de señales ni excepciones C++: comportamientos no definidos
   del callee se propagan al proceso.
5. Los textos devueltos no se liberan (ver convención de propiedad).
