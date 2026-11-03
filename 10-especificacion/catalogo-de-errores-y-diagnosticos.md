# Catálogo de errores y diagnósticos — BASILISCO Core 0.3

**Estado:** propuesta normativa para revisión (sincronizada con el prototipo).  
**Fuentes:** `prototipo/src/parser.rs`, `prototipo/src/lexer.rs`, `prototipo/src/evaluador.rs`, ADR-0005, ADR-0006, ADR-0007, ADR-0008, ADR-0011, ADR-0012.  
**Última sincronización:** 2026-10-25, commit `4b23ac2`.

## 1. Principios
- **Errores controlados:** ningún pánico interno debe alcanzarse como error del lenguaje. Los fallos se devuelven como errores tipados con ubicación.
- **Mensajes en español:** todos los mensajes visibles deben estar en español.
- **Ubicación obligatoria:** todo error visible debe incluir línea y columna (*span*).
- **Códigos estables:** se introducen códigos `BAS-XXXX` estables; el texto del mensaje puede refinarse sin romper tooling que dependa del código.
- **Sin sugerencias peligrosas:** solo se permitirán sugerencias cuando sean seguras y no alteren el significado silenciosamente.
- **Sin coerción silenciosa:** los errores por tipos incompatibles deben ser explícitos.

## 2. Formato de diagnóstico
Formato unificado para errores visibles:

```text
error <fase> [<codigo>] [<linea>:<columna>]: <mensaje breve>
  --> <archivo>:<linea>:<columna>
   |
<linea> | <línea de fuente>
   |  <marcador>
   = nota: <explicación opcional>
   = ayuda: <sugerencia opcional y segura>
```

Fases: `léxico`, `sintáctico`, `semántico`, `ejecución`. El prototipo actual usa formatos compactos (`error léxico [...]`, `error sintáctico [...]`, `error de ejecución [...]`); el formato expandido es objetivo para el futuro.

## 3. Catálogo de códigos

### 3.1 Errores léxicos (`BAS-1XXX`)
| Código | Situación | Mensaje típico | Notas |
|---|---|---|---|
| `BAS-1001` | Carácter inesperado | `carácter inesperado: <char>` | Debe incluir ubicación. |
| `BAS-1002` | Cadena sin cerrar | `cadena sin cerrar: falta la comilla de cierre` | Incluye salto de línea dentro de cadena. |
| `BAS-1003` | Escape inválido/no soportado | `escape no soportado: \<c> (soportados: \\ \" \n \t)` | Solo se soportan los listados. |
| `BAS-1004` | Retorno de carro suelto | `retorno de carro suelto no soportado; use fin de línea \n o \r\n` | Requisito del lexer. |
| `BAS-1005` | `.` suelto | `carácter inesperado: '.' suelto (¿quiso escribir '..' o '..='?)` | Para evitar ambigüedad con rangos. |
| `BAS-1006` | `!` no soportado | `operador '!' no soportado; use 'no' o '!='` | Guía hacia la sintaxis del lenguaje. |

### 3.2 Errores sintácticos (`BAS-2XXX`)
| Código | Situación | Mensaje típico | Notas |
|---|---|---|---|
| `BAS-2001` | Expresión incompleta | `expresión incompleta: falta un operando` | Falta operando tras operador o al inicio. |
| `BAS-2002` | Paréntesis sin cerrar | `falta ')' de cierre` | Agrupación. |
| `BAS-2003` | Llave sin cerrar | `falta '}' para cerrar el bloque` | Bloques. |
| `BAS-2004` | Llave en línea incorrecta | `se esperaba '{' en la misma línea que '<palabra>'` | Regla normativa ADR-0007 §1.6. |
| `BAS-2005` | `sino` sin `si` | `'sino' sin un 'si' correspondiente` | Control de flujo. |
| `BAS-2006` | `sino` mal formado | `se esperaba '{' o 'si' en la misma línea que 'sino'` | `sino si` vs `sino {}`. |
| `BAS-2007` | Comparaciones encadenadas | `comparaciones encadenadas no están permitidas` | ADR-0007. |
| `BAS-2008` | Declaración mal formada | `se esperaba un nombre tras '<palabra>'` / `se esperaba '=' tras el nombre ...` | `sea`/`var`/`dato`/`const`. |
| `BAS-2009` | Anotación de tipo mal formada | `se esperaba ':' para anotación de tipo` / `tipo desconocido; se esperaba: entero, decimal, real, logico, texto, arreglo<T, N>, lista<T>` | Tipado. |
| `BAS-2010` | `arreglo<...>` mal formado | `se esperaba '<' después de 'arreglo'` / `se esperaba ',' después del tipo de elemento en arreglo` / `se esperaba un número para el tamaño del arreglo` / `el tamaño del arreglo debe ser mayor que 0` / `se esperaba '>' para cerrar el tipo de arreglo` | Genéricos/arrays. |
| `BAS-2011` | `lista<...>` mal formado | `se esperaba '<' después de 'lista'` / `se esperaba '>' para cerrar el tipo de lista` | Genéricos/listas. |
| `BAS-2012` | Función mal formada | `se esperaba el nombre de la función` / `se esperaba '(' tras el nombre de la función` / `se esperaba a '<palabra clave'` / `se esperaba '{' en la misma línea que la función` / `el '{' debe estar en la misma línea que 'funcion'` / `funciones anidadas no están permitidas (ADR-0008)` | ADR-0008. |
| `BAS-2013` | Llamada mal formada | `se esperaba ',' o ')'` | Argumentos. |
| `BAS-2014` | Indexación mal formada | `se esperaba ']'` / `se esperaba ',' o ']'` | Arreglos/listas. |
| `BAS-2015` | Texto inesperado tras sentencia/expresión | `texto inesperado después de la sentencia` / `texto inesperado después de la expresión` | Delimitación por fin de línea/contexto. |
| `BAS-2016` | `sino` fuera de contexto | `'sino' sin un 'si' correspondiente` | Ya cubierto pero mantenido por claridad. |

### 3.3 Errores semánticos/ejecución (`BAS-3XXX`)
| Código | Situación (`CausaError`) | Mensaje típico | Notas |
|---|---|---|---|
| `BAS-3001` | Variable no definida | `variable no definida: <nombre>` | ADR-0006. |
| `BAS-3002` | Declaración duplicada | `declaración duplicada: <nombre>` | Mismo ámbito. |
| `BAS-3003` | Asignación incompatible | `asignación incompatible: se esperaba <esperado> y se obtuvo <obtenido>` | Tipado con anotación. |
| `BAS-3004` | Condición no lógica | `la condición debe ser lógica, se obtuvo <obtenido>` | `si`/`mientras`. |
| `BAS-3005` | Operando no lógico | `el operando de '<op>' debe ser lógico, se obtuvo <obtenido>` | `no`/`y`/`o`. |
| `BAS-3006` | Comparación incompatible | `comparación incompatible entre <izq> y <der>` | Sin conversión implícita. |
| `BAS-3007` | Desbordamiento entero | `desbordamiento entero` | ADR-0005. |
| `BAS-3008` | División por cero | `división entre cero` | Todos los tipos numéricos. |
| `BAS-3009` | Exceso de rango decimal | `exceso de rango decimal` | `DECIMAL(28,9)`. |
| `BAS-3010` | Conversión ilegal | `conversión ilegal` | `decimal`+`real` etc. |
| `BAS-3011` | Tipos incompatibles (aritmética) | `tipos incompatibles` | Mezclas no permitidas. |
| `BAS-3012` | Límite de iteraciones | `se superó el límite de <limite> iteraciones: posible bucle infinito` | Protección prototipo. |
| `BAS-3013` | Firma inválida | `la función '<nombre>' espera <esperado> argumentos y se le pasaron <obtenido>` | ADR-0008. |
| `BAS-3014` | Retorno ausente | `la función '<nombre>' no ejecuta 'retornar'` | ADR-0008. |
| `BAS-3015` | Tipo de retorno incompatible | `el tipo de retorno es incompatible: se esperaba <esperado> y se obtuvo <obtenido>` | ADR-0008. |
| `BAS-3016` | Índice fuera de rango | `índice <indice> fuera de rango (tamaño <tamaño>)` | Arreglos/listas. |
| `BAS-3017` | Tamaño de arreglo inválido | `tamaño de arreglo inválido: se esperaba <esperado> elementos y se obtuvo <obtenido>` | `arreglo<T, N>`. |
| `BAS-3018` | Tipo de elemento incompatible | `elemento <indice>: tipo incompatible, se esperaba <esperado> y se obtuvo <obtenido>` | Colecciones. |
| `BAS-3019` | `romper` fuera de ciclo | `'romper' solo puede usarse dentro de a cycle 'mientras'` | ADR-0012. |
| `BAS-3020` | `continuar` fuera de ciclo | `'continuar' solo puede usarse dentro de a cycle 'mientras'` | ADR-0012. |
| `BAS-3021` | `retornar` fuera de función | (pendiente de formalizar) | Debe tener código y ubicación. |
| `BAS-3022` | Asignación a variable inmutable | (pendiente de formalizar) | `dato`/`const`. |
| `BAS-3023` | Inicialización antes del uso | (pendiente de formalizar) | Variables no inicializadas. |
| `BAS-3024` | Arreglos/listas anidados no soportados | (pendiente de formalizar) | Limitación actual. |

## 4. Ubicación y *spans*
- **Léxico/sintáctico:** usan la ubicación del token donde se detecta el error (`linea`/`columna`).
- **Ejecución:** usan la ubicación guardada en el AST (`Literal`, `Variable`, `Llamada`, `Si`, `Mientras`, `Mostrar`, etc.). Cuando no hay ubicación, debe evitarse; el parser/evaluador debe propagarla.
- **Tokens:** `Token` siempre lleva `linea` y `columna`.
- **Errores léxicos/sintácticos:** llevan `linea` y `columna`.

## 5. Recomendaciones de implementación
1. **Añadir `codigo: &'static str`** a `ErrorLexico`, `ErrorSintactico`, `ErrorEjecucion` (o envolverlos). Mantener compatibilidad con `Display` actual.
2. **Centralizar construcción de errores** con helpers que asignen código automáticamente.
3. **Añadir tests de diagnósticos**: verificar código + línea/columna para casos clave.
4. **Mejorar formato**: mostrar línea de fuente y marcador cuando sea posible (en CLI), manteniendo el formato compacto actual.
5. **Documentar estabilidad**: los códigos `BAS-*` son estables una vez aprobados; los mensajes pueden cambiar.

## 6. Estado
**Parcialmente implementado.** El prototipo ya tiene ubicación en la mayoría de errores y mensajes en español. Faltan códigos estables formales, formato expandido con fuente/marcador y catálogo centralizado. Esto cubre `GAP-004`.
