# Perfil de conformidad BASILISCO v0.4 (candidata)

**Estado:** propuesta técnica para revisión; no aprobada ni declaración de estabilidad.
**Relación:** complementa, no reemplaza, BASILISCO Core 0.3 ni los ADR aprobados.
**Propósito:** fijar el subconjunto que una implementación puede anunciar como garantizado en v0.4.

Este perfil no define todas las capacidades del prototipo ni del lenguaje futuro. El código puede aceptar extensiones fuera del perfil; estas no son requisito de conformidad y no deben usarse como base de compatibilidad v0.4.

## 1. Conformidad

Una implementación conforme con este perfil debe:

- aceptar toda construcción válida definida aquí y producir el resultado o error especificado;
- rechazar los usos inválidos indicados sin pánico ni valor por defecto;
- informar errores léxicos, sintácticos y de ejecución con ubicación cuando exista;
- pasar las suites de aceptación identificadas en §9;
- publicar versión, commit, plataforma y comando de reproducción.

El prototipo local aún no se declara conforme: esta propuesta necesita revisión/aprobación, y la evidencia de pruebas debe ligarse a un commit y CI antes de publicar una versión.

## 2. Fuente y léxico

El código fuente es UTF-8. Se ignora un BOM inicial. Los finales `LF` y `CRLF` separan sentencias; un `CR` aislado es error. Espacios y tabuladores se ignoran entre tokens. `#` inicia un comentario que continúa hasta el fin de línea.

Los identificadores son ASCII y sensibles a mayúsculas: `[A-Za-z_][A-Za-z0-9_]*`. Las palabras clave se escriben exactamente en minúsculas. Identificadores Unicode no están incluidos en este perfil.

Literales numéricos:

- `42` es `entero` (`i64`);
- `42.5` es `decimal` de escala fija 9;
- `1e10`, `2.5e-3` son `real` (`f64`).

Los signos `+` y `-` son operadores. Un literal negativo se admite como número negativo en posición primaria; el menos no es un operador unario general (`-variable` no está incluido). Overflow de enteros, exceso de rango decimal y división por cero son errores, no valores especiales ni resultados por defecto.

Los textos usan comillas dobles, admiten Unicode y los escapes `\\`, `\"`, `\n` y `\t`. Un salto de línea antes de la comilla de cierre, un escape desconocido o una cadena sin cerrar producen error léxico.

## 3. Tipos y conversiones

| Tipo | Dominio del perfil |
|---|---|
| `entero` | Entero con signo de 64 bits. División truncada hacia cero; `%` conserva el signo del dividendo. |
| `decimal` | Punto fijo `DECIMAL(28,9)`, redondeo half-even y error por exceso de rango. |
| `real` | Coma flotante binaria de 64 bits; la división por cero produce error. Los casos no finitos por overflow no forman parte de este perfil y requieren criterios adicionales. |
| `logico` | `verdadero` o `falso`; no se convierte a/desde números. |
| `texto` | Secuencia UTF-8; concatenación con `+`, igualdad/desigualdad y comparación lexicográfica. |
| `nulo` | Valor de ausencia, distinto de `falso`, `0` y `""`. Sólo `nulo == nulo` y `nulo != nulo` son comparaciones válidas. |
| `arreglo<T, N>` | Colección indexada de tamaño fijo `N > 0`. |
| `lista<T>` | Colección indexada de tamaño variable. |

`T` de arreglos/listas debe ser un tipo escalar entre `entero`, `decimal`, `real`, `logico` y `texto`; colecciones anidadas y `nulo` como tipo de elemento no forman parte de este perfil.

Las únicas conversiones implícitas son `entero → decimal` y `entero → real`. No se convierte implícitamente entre `decimal` y `real`, ni de un tipo numérico a `texto`. En una colección tipada, cada elemento y cada valor asignado se comprueba con estas mismas reglas.

| Operación | Operandos admitidos | Tipo de resultado |
|---|---|---|
| `+`, `-`, `*`, `/` | Pares numéricos compatibles según las conversiones anteriores | Tipo común resultante; `/` de enteros trunca hacia cero |
| `%` | Dos `entero` | `entero`; resto con signo del dividendo |
| `+` | Dos `texto` | `texto` concatenado |
| `==`, `!=`, `<`, `<=`, `>`, `>=` | Dos números compatibles o dos textos | `logico` |
| `==`, `!=` | Dos `nulo` | `logico`; igualdad verdadera y desigualdad falsa |
| `y`, `o`, `no` | Sólo valores `logico` | `logico`; `y`/`o` usan cortocircuito |

Comparar lógicos, colecciones, `nulo` con otro tipo, o pares numéricos sin conversión aprobada produce error tipado. `nulo` no admite aritmética ni se usa como condición.

## 4. Expresiones

La gramática de expresiones, de mayor a menor precedencia, es:

```ebnf
expresion       = expresion_o ;
expresion_o     = expresion_y , { "o" , expresion_y } ;
expresion_y     = expresion_no , { "y" , expresion_no } ;
expresion_no    = "no" , expresion_no | comparacion ;
comparacion     = suma , [ comparador , suma ] ;
comparador      = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
suma            = producto , { ( "+" | "-" ) , producto } ;
producto        = primaria , { ( "*" | "/" | "%" ) , primaria } ;
primaria        = numero | texto | "verdadero" | "falso" | "nulo"
                | identificador | llamada | arreglo_literal
                | indice | "(" , expresion , ")" ;
llamada         = identificador , "(" , [ argumentos ] , ")" ;
argumentos      = expresion , { "," , expresion } ;
arreglo_literal = "[" , [ expresion , { "," , expresion } ] , "]" ;
indice          = primaria , "[" , expresion , "]" ;
```

Los operadores binarios son asociativos a izquierda; `no` se asocia a derecha. Las comparaciones encadenadas son error sintáctico. `y` y `o` son de cortocircuito y requieren operandos lógicos cuando se evalúan. Las condiciones de `si` y `mientras` deben ser lógicas.

El fin de línea termina una expresión completa. Puede ignorarse al continuar después de un operador o dentro de paréntesis, conforme a la regla contextual del parser.

## 5. Sentencias y ámbitos

```ebnf
programa       = { fin_de_linea } , { sentencia , { fin_de_linea } } ;
sentencia      = declaracion | asignacion | asignacion_indice | condicional
               | ciclo | bloque | funcion | retorno | mostrar | expresion ;
declaracion    = "sea" , identificador , "=" , expresion
               | ( "var" | "dato" ) , identificador , ":" , tipo , "=" , expresion ;
asignacion     = identificador , "=" , expresion ;
asignacion_indice = identificador , "[" , expresion , "]" , "=" , expresion ;
condicional    = "si" , expresion , bloque , [ "sino" , ( condicional | bloque ) ] ;
ciclo          = "mientras" , expresion , bloque ;
bloque         = "{" , { sentencia } , "}" ;
```

`sea` infiere el tipo del inicializador y declara una variable mutable. `var` declara una variable mutable con tipo anotado. `dato` declara una variable inmutable con tipo anotado. La asignación requiere una variable existente; no crea nombres. Una declaración duplicada en el mismo ámbito es error; el sombreado en ámbitos internos está permitido.

Los bloques, ramas y cuerpos de ciclo introducen un ámbito que se retira al salir. No producen un valor para el programa. El resultado del programa es la última expresión de nivel superior, si existe.

En asignación por índice, los índices son enteros desde cero. Índice negativo, no entero o fuera de rango es error tanto para lectura como para escritura. Los arreglos requieren `N > 0` y un inicializador de exactamente `N` elementos. Las listas admiten inicializador vacío. No se incluyen operaciones para agregar o quitar elementos.

## 6. Funciones y salida

```ebnf
funcion        = "funcion" , identificador , "(" , parametros? , ")"
               , ( "->" , tipo )? , bloque ;
parametros     = parametro , { "," , parametro } ;
parametro      = identificador [ ":" , tipo ] ;
retorno        = "retornar" [ expresion ] ;
mostrar        = ( "mostrar" | "mostrar_sin_salto" ) , expresion ;
tipo           = tipo_escalar | "arreglo" , "<" , tipo_elemento , "," , entero , ">"
               | "lista" , "<" , tipo_elemento , ">" ;
tipo_escalar   = "entero" | "decimal" | "real" | "logico" | "texto" | "nulo" ;
tipo_elemento  = "entero" | "decimal" | "real" | "logico" | "texto" ;
```

Una firma lleva tipos en todos sus parámetros o en ninguno. Las funciones no pueden anidarse ni capturan variables del llamante; la recursión está permitida. Los argumentos se evalúan de izquierda a derecha antes de validar aridad. Una aridad incorrecta produce `FirmaInvalida`.

Sin anotación, los parámetros fijan sus tipos en la primera llamada exitosa; llamadas posteriores admiten el mismo tipo o una conversión implícita aprobada. Sin tipo de retorno, el tipo se fija en la primera llamada exitosa. Una función usada como expresión que termina sin `retornar` produce `RetornoAusente`.

Con parámetros anotados, cada argumento se verifica en la llamada. Con retorno anotado, el valor devuelto se verifica al regresar. La incompatibilidad produce `TipoParametroIncompatible` o `RetornoIncompatibleConAnotacion`, respectivamente. `-> nulo` admite terminar normalmente o ejecutar `retornar nulo`; ambas formas producen `nulo`. `retornar` sin expresión en otro contexto de retorno produce error.

`mostrar` escribe la representación del valor y un salto de línea en stdout; `mostrar_sin_salto` escribe sin añadirlo. `mostrar` acepta cualquier valor del perfil.

## 7. Control de flujo

`si`/`sino` y `mientras` usan condiciones de tipo `logico`. La llave `{` debe aparecer en la misma línea que la cabecera de `si`, `sino` o `mientras`; `sino si` se escribe con ambas palabras. `mientras` reevalúa su condición antes de cada iteración.

Como protección del prototipo, `mientras` limita cada ejecución del ciclo a 100 000 iteraciones; superar el límite produce `LimiteIteraciones`. Este límite es específico de la implementación v0.4 y no constituye una regla universal del lenguaje.

## 8. Errores y CLI

Los errores son controlados y no se sustituyen por `0`, `falso`, una cadena vacía u otro valor. Los errores de ejecución exponen causa, línea y columna. Al ejecutar archivo, la CLI incluye también su ruta en stderr.

La CLI del perfil incluye:

- `basilisco run <archivo.bsk>` y alias `bsk run <archivo.bsk>`;
- `--help`, `--version` y `demo`;
- `.bsk` como extensión canónica; `.nexo` se acepta con aviso de legado; `.nx` y otras extensiones se rechazan;
- código de salida `0` ante éxito, `1` ante error de lectura/sintaxis/ejecución y `2` ante error de uso/extensión.

La sintaxis de `intentar`/`rescatar`, `para cada`, rangos, `romper` y `continuar`, así como las funciones integradas de texto y E/S de archivos, queda fuera de la garantía v0.4 aunque exista en el prototipo.

## 9. Suites que verifican el perfil

| Área | Evidencia ejecutable |
|---|---|
| Números, operadores, tipos lógicos y comparaciones | `prototipo/tests/conformidad.rs`, `prototipo/tests/conformidad_control.rs` |
| Variables, ámbitos, tipos anotados y asignación | `prototipo/tests/variables.rs`, `prototipo/tests/control_flujo.rs` |
| Funciones, inferencia, anotaciones y retorno | `prototipo/tests/conformidad_funciones.rs` |
| Texto básico | `prototipo/tests/conformidad_texto.rs` |
| Arreglos/listas y límites de indexación | `prototipo/tests/conformidad_arreglos_listas.rs` |
| CLI, aceptación integrada y errores visibles | `prototipo/tests/cli.rs`, `prototipo/tests/tutorial.rs` |

Toda regla del perfil que no esté cubierta por una prueba positiva y otra negativa debe permanecer como pendiente antes de marcar una versión conforme.

## 10. Límites y publicación

Este documento no especifica módulos, paquetes, biblioteca estándar extensa, concurrencia, depuración, IDE, FFI ni soporte de plataformas. Tampoco incorpora las extensiones experimentales enumeradas en §8. La conformidad con este perfil no significa conformidad con BASILISCO Core 0.3 ni estabilidad del lenguaje completo.

Para aprobar/publicar v0.4 se requiere revisión formal del perfil, suite completa, `cargo fmt --check`, Clippy estricto, build de binarios, CI asociada al commit final, versión de paquete decidida y evidencia de instalación/plataformas declaradas. Hasta cumplir esas condiciones, BASILISCO sigue siendo un prototipo/candidata local.
