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
| `texto(x)` | `cualquiera -> texto` | Convierte a representación textual. |

## Colecciones

| Función | Firma | Descripción |
|---|---|---|
| `agregar(col, x)` | `arreglo\|lista, cualquiera -> col` | Añade `x` (devuelve copia). |
| `eliminar(col, i)` | `arreglo\|lista, entero -> col` | Elimina índice `i`. |
| `map(col, f)` | `col, funcion -> col` | Aplica `f` a cada elemento. |
| `filtrar(col, f)` | `col, funcion -> col` | Filtra por predicado lógico. |
| `reducir(col, f, ini)` | `col, funcion, cualquiera -> cualquiera` | Reduce con acumulador. |
| `ordenar(col)` | `arreglo\|lista\|texto -> mismo` | Ordena. |
| `posicion(col, x)` | `texto\|arreglo\|lista, cualquiera -> entero` | Índice 0-based de primera ocurrencia, o -1 si no existe. En texto: índices por caracteres Unicode (no bytes); subcadena vacía devuelve 0. En arreglo/lista: compara por igualdad de valor; lista vacía o ausente devuelve -1. |
| `abs(x)` | `entero\|real\|decimal -> mismo` | Valor absoluto. |
| `min(a, b)` | `entero\|real, entero\|real -> mismo` | Mínimo. |
| `max(a, b)` | `entero\|real, entero\|real -> mismo` | Máximo. |
| `potencia(b, e)` | `entero\|real, entero\|real -> mismo` | Potencia. |
| `raiz(x)` | `entero\|real -> real` | Raíz cuadrada. |
| `piso(x)` | `entero\|real\|decimal -> entero` | Piso hacia −∞. Para `decimal` usa aritmética entera escalada (sin `f64`). Para `real`, error si NaN/∞ o si el resultado está fuera de `i64` o fuera de precisión entera exacta representable con seguridad. |
| `techo(x)` | `entero\|real\|decimal -> entero` | Techo hacia +∞. Mismas reglas de precisión/errores que `piso`. |
| `redondear(x)` | `entero\|real\|decimal -> entero` | Redondeo half away from zero (desempate `.5` se aleja de 0). Mismas reglas de precisión/errores que `piso`. |

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

## Disponibilidad por motor (Fase 09)

| Dominio / función | Tree-Walk | VM | Nativo | WASM | Notas |
|---|---:|---:|---:|---:|---|
| E/S: mostrar/mostrar_sin_salto | OBL* | OBL | OBL | NO | *TW: sentencias; VM/nativo: soporte de salida. WASM sin E/S de texto. |
| E/S stdin: leer, leer_entero, leer_decimal, leer_real | OBL | OBL | NO | NO | Solo motores interactivos/proceso. |
| Texto: longitud, mayusculas, minusculas, recortar, reemplazar, subcadena, dividir, unir, contiene, empieza_con, termina_con, es_vacio, repetir, invertir, a_entero, a_decimal, a_real | OBL | OBL | LIM | NO | Nativo: subconjunto inline parcial; resto NO/LIM según `backend_nativo.rs`. |
| Texto: texto(x), posicion(texto, sub) | OBL | OBL | NO | NO | Añadidos/corregidos Fase 09; paridad TW↔VM verificada. `posicion`: índices Unicode por caracteres, subcadena vacía=0, no encontrado=-1. Nativo/WASM: no reclamado. |
| Colecciones: agregar, eliminar, map, filtrar, reducir, ordenar | OBL | OBL | LIM | NO | Nativo: `agregar/eliminar/longitud/contiene/es_vacio/invertir/ordenar` en subconjunto soportado; `map/filtrar/reducir` no garantizados en nativo/WASM. |
| Colecciones: posicion(arreglo/lista, x) | OBL | OBL | NO | NO | Fase 09; paridad TW↔VM verificada. Igualdad de valor, ausente/vacía=-1. Nativo/WASM: no reclamado. |
| Matemática: abs, min, max, potencia, raiz | OBL | OBL | LIM | LIM | Nativo/WASM: depende de subconjunto numérico soportado por backend; sin reclamo de paridad total fuera de casos probados. |
| Matemática: piso, techo, redondear | OBL | OBL | NO | NO | TW/VM verificados con paridad. Decimal: aritmética entera escalada exacta en contrato decimal. Real: IEEE-754; error si NaN/∞ o fuera de rango/precisión entera segura de `i64`. Nativo/WASM: no reclamado. |
| JSON: json_parsear, json_serializar, json_obtener | OBL | OBL | LIM | NO | Nativo: `json_parsear` por runtime; `json_serializar/json_obtener` no garantizados. |
| Archivos: leer_archivo, escribir_archivo, existe_archivo, abrir/leer_linea/leer_todo/escribir/escribir_linea/cerrar/fin_de_archivo | OBL | OBL | LIM | NO | Nativo: `leer_archivo` en subconjunto; handles/otros archivos no garantizados. WASM sin E/S de archivos. |
| Fechas: ahora, fecha_actual, formatear_fecha | OBL | OBL | NO | NO | Dependiente de reloj/host; no soportado en nativo/WASM. |
| HTTP: http_get, http_post | OBL | OBL | NO | NO | Requiere red/host; no soportado en nativo/WASM. |
| Concurrencia: iniciar_tarea, esperar_tarea, canales | OBL | OBL | NO | NO | Fase 10, solo TW/VM. |
| FFI: cargar_ffi, llamar_ffi, descargar_ffi | OBL | OBL | NO | NO | Fase 11, solo TW/VM; nativo/WASM rechazo explícito BAS-7124. |

**Leyenda:** OBL = soportado en alcance del motor; LIM = subconjunto limitado, no paridad total reclamada; NO = no soportado / fuera de alcance, debe rechazar o no reclamarse. Las afirmaciones de paridad TW↔VM se respaldan con pruebas diferenciales/conformidad; nativo/WASM solo donde hay evidencia.

### Contratos y límites (Fase 09)

- **`piso/techo/redondear`**: `decimal` se procesa en aritmética entera escalada (`FACTOR_ESCALA`), sin `f64`, preservando el contrato decimal. `real` usa IEEE-754; si el valor es NaN/∞, o el resultado no es representable como `i64` seguro, se produce error aritmético de desbordamiento. No se promete precisión decimal exacta para `real` fuera de la precisión entera exacta de IEEE-754. Redondeo: `half away from zero`.
- **`posicion`**: texto indexado por caracteres Unicode (no bytes); devuelve -1 si no existe; subcadena vacía devuelve 0. En arreglo/lista devuelve índice 0-based del primer elemento igual, o -1 si vacío/ausente.
- **`texto(x)`**: conversión textual de valores soportados por el runtime; disponible en TW/VM.
