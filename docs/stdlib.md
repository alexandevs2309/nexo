# Referencia de la biblioteca estándar de BASILISCO

Funciones disponibles en ambos motores (Tree-Walk y VM). Las funciones marcadas con ⚠ requieren un compilador C (FFI) o red (HTTP) y pueden omitirse en entornos restringidos.

## E/S y salida

| Función | Firma | Descripción |
|---|---|---|
| `mostrar(x)` | `cualquiera -> nulo` | Imprime `x` con salto de línea. |
| `mostrar_sin_salto(x)` | `cualquiera -> nulo` | Imprime `x` sin salto de línea. |
| `leer()` | `-> texto` | Lee una línea de stdin. |
| `leer_entero()` | `-> entero` | Lee y parsea un entero de stdin. |
| `leer_decimal()` | `-> decimal` | Lee y parsea un decimal de stdin. |
| `leer_real()` | `-> real` | Lee y parsea un real de stdin. |

## Texto

| Función | Firma | Descripción |
|---|---|---|
| `longitud(x)` | `texto\|colección -> entero` | Longitud en caracteres/elementos. |
| `mayusculas(s)` | `texto -> texto` | Mayúsculas. |
| `minusculas(s)` | `texto -> texto` | Minúsculas. |
| `recortar(s)` | `texto -> texto` | Quita espacios extremos. |
| `reemplazar(s, de, a)` | `texto, texto, texto -> texto` | Reemplaza todas las ocurrencias. |
| `subcadena(s, ini, fin)` | `texto, entero, entero -> texto` | Subcadena `[ini, fin)`. |
| `dividir(s, sep)` | `texto, texto -> lista<texto>` | Divide por separador. |
| `unir(lista, sep)` | `lista<texto>, texto -> texto` | Une con separador. |
| `contiene(s, sub)` | `texto, texto -> logico` | ¿Contiene subcadena? |
| `empieza_con(s, pref)` | `texto, texto -> logico` | ¿Empieza con prefijo? |
| `termina_con(s, suf)` | `texto, texto -> logico` | ¿Termina con sufijo? |
| `es_vacio(s)` | `texto\|colección -> logico` | ¿Está vacío? |
| `repetir(s, n)` | `texto, entero -> texto` | Repite `s` `n` veces. |
| `invertir(s)` | `texto\|colección -> texto\|colección` | Invierte. |
| `a_entero(s)` | `texto -> entero` | Parsea entero (error si inválido). |
| `a_decimal(s)` | `texto -> decimal` | Parsea decimal. |
| `a_real(s)` | `texto -> real` | Parsea real. |

## Colecciones

| Función | Firma | Descripción |
|---|---|---|
| `agregar(col, x)` | `arreglo\|lista, cualquiera -> col` | Añade `x` (devuelve copia). |
| `eliminar(col, i)` | `arreglo\|lista, entero -> col` | Elimina índice `i`. |
| `map(col, f)` | `col, funcion -> col` | Aplica `f` a cada elemento. |
| `filtrar(col, f)` | `col, funcion -> col` | Filtra por predicado lógico. |
| `reducir(col, f, ini)` | `col, funcion, cualquiera -> cualquiera` | Reduce con acumulador. |
| `ordenar(col)` | `arreglo\|lista\|texto -> mismo` | Ordena. |
| `abs(x)` | `entero\|real\|decimal -> mismo` | Valor absoluto. |
| `min(a, b)` | `entero\|real, entero\|real -> mismo` | Mínimo. |
| `max(a, b)` | `entero\|real, entero\|real -> mismo` | Máximo. |
| `potencia(b, e)` | `entero\|real, entero\|real -> mismo` | Potencia. |
| `raiz(x)` | `entero\|real -> real` | Raíz cuadrada. |

## JSON

| Función | Firma | Descripción |
|---|---|---|
| `json_parsear(s)` | `texto -> objeto\|lista\|...` | Parsea JSON. |
| `json_serializar(v)` | `cualquiera -> texto` | Serializa a JSON. |
| `json_obtener(obj, clave)` | `objeto, texto -> cualquiera` | Obtiene campo (o nulo). |

## Archivos

| Función | Firma | Descripción |
|---|---|---|
| `leer_archivo(ruta)` | `texto -> texto` | Lee archivo completo. |
| `escribir_archivo(ruta, contenido)` | `texto, texto -> nulo` | Escribe (sobrescribe). |
| `existe_archivo(ruta)` | `texto -> logico` | ¿Existe? |
| `abrir(ruta, modo)` | `texto, texto -> archivo` | Abre en modo `"leer"`, `"escribir"` o `"agregar"`. |
| `leer_linea(f)` | `archivo -> texto` | Lee una línea (vacío en EOF). |
| `leer_todo(f)` | `archivo -> texto` | Lee todo el resto. |
| `escribir(f, texto)` | `archivo, texto -> nulo` | Escribe texto. |
| `escribir_linea(f, texto)` | `archivo, texto -> nulo` | Escribe línea. |
| `cerrar(f)` | `archivo -> nulo` | Cierra. |
| `fin_de_archivo(f)` | `archivo -> logico` | ¿EOF? |

## Fechas

| Función | Firma | Descripción |
|---|---|---|
| `ahora()` | `-> entero` | Milisegundos desde epoch (UTC). |
| `fecha_actual()` | `-> texto` | Fecha actual ISO `YYYY-MM-DD`. |
| `formatear_fecha(ms, formato)` | `entero, texto -> texto` | Formatea instante con formato strftime. |

## HTTP ⚠ (requiere red)

| Función | Firma | Descripción |
|---|---|---|
| `http_get(url)` | `texto -> texto` | GET, devuelve cuerpo. |
| `http_post(url, cuerpo)` | `texto, texto -> texto` | POST con cuerpo. |

## Concurrencia

| Función | Firma | Descripción |
|---|---|---|
| `iniciar_tarea(nombre)` | `texto -> entero` | Lanza función en hilo. |
| `esperar_tarea(id)` | `entero -> cualquiera` | Espera y devuelve resultado. |

## FFI (interoperabilidad C) ⚠ (requiere compilador C)

| Función | Firma | Descripción |
|---|---|---|
| `cargar_ffi(ruta)` | `texto -> entero` | Carga biblioteca, devuelve id. |
| `llamar_ffi(id, simbodo, args, retorno)` | `entero, texto, lista, texto -> cualquiera` | Llama símbodo C. Retorno: `"entero"`, `"logico"`, `"real"`, `"decimal"`, `"texto"`, `"nulo"`. |
| `descargar_ffi(id)` | `entero -> nulo` | Descarga biblioteca. |

## Módulos (sentencias, no funciones)

- `usar "lib.bsk" { doble, PI }` — importa nombres específicos.
- `usar "lib.bsk" como lib` — importa como módulo (`lib.doble(...)`).
- `importar "lib.bsk" como l` — equivalente con objeto de módulo.
- `exportar funcion ...` / `publico funcion ...` — exporta desde el módulo.

## Manejo de errores

- `intentar { ... } rescatar e { ... }` — captura cualquier error en `e`.
- `intentar { ... } rescatar { ... }` — sin variable de error.
