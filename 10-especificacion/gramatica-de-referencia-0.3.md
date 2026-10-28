# Gramática de referencia del subconjunto Nexo 0.3

**Estado:** referencia documental para revisión; no sustituye la gramática del parser real.
**Fuentes:** ADR-0003, ADR-0004, ADR-0006 y ADR-0007 del repositorio `alexandevs2309/nexo`.

## Convenciones
`*` significa cero o más; `+` uno o más; `?` opcional. Los tokens son los definidos por el lexer del prototipo.

## Gramática conceptual alineada con ADR-0006/0007
```ebnf
programa        = { fin_de_linea } , { sentencia , { fin_de_linea } } ;
sentencia       = declaracion
                | asignacion
                | condicional
                | bucle
                | bloque
                | expresion ;

declaracion     = "sea" , IDENTIFICADOR , "=" , expresion ;
asignacion      = IDENTIFICADOR , "=" , expresion ;
condicional     = "si" , expresion , bloque ,
                  [ "sino" , ( condicional | bloque ) ] ;
bucle           = "mientras" , expresion , bloque ;
bloque          = "{" , { fin_de_linea } ,
                  [ sentencia , { fin_de_linea , sentencia } ] ,
                  { fin_de_linea } , "}" ;

expresion       = expresion_o ;
expresion_o     = expresion_y , { "o" , expresion_y } ;
expresion_y     = expresion_no , { "y" , expresion_no } ;
expresion_no    = "no" , expresion_no | comparacion ;
comparacion     = expresion_suma , [ comparador , expresion_suma ] ;
comparador      = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
expresion_suma  = expresion_producto , { ( "+" | "-" ) , expresion_producto } ;
expresion_producto = primaria , { ( "*" | "/" | "%" ) , primaria } ;
primaria        = ENTERO | DECIMAL | REAL | "verdadero" | "falso"
                | IDENTIFICADOR | "(" , expresion , ")" ;
```

## Gramática ampliada (v0.5, alineada con el parser real)
```ebnf
programa        = { fin_de_linea } , { sentencia , { fin_de_linea } } ;
sentencia       = declaracion | asignacion | asignacion_indice | condicional
                | bucle_mientras | bucle_para_cada | bloque | funcion | retorno
                | mostrar | mostrar_sin_salto | intentar_rescatar | importar
                | romper | continuar | expresion ;
declaracion     = ("sea" , IDENTIFICADOR , [ ":" , tipo ] , "=" , expresion)
                | ("var" | "dato" | "const") , IDENTIFICADOR , ":" , tipo , "=" , expresion ;
asignacion      = IDENTIFICADOR , "=" , expresion ;
asignacion_indice = expresion , "[" , expresion , "]" , "=" , expresion ;
condicional     = "si" , expresion , bloque , [ "sino" , ( condicional | bloque ) ] ;
bucle_mientras  = "mientras" , expresion , bloque ;
bucle_para_cada = "para" , "cada" , IDENTIFICADOR , "en" , expresion_iterable , bloque ;
expresion_iterable = expresion | rango ;
rango           = expresion , ( ".." | "..=" ) , expresion ;
bloque          = "{" , { fin_de_linea } , [ sentencia , { fin_de_linea , sentencia } ] , { fin_de_linea } , "}" ;
funcion         = "funcion" , IDENTIFICADOR , "(" , [ parametros ] , ")" , [ "->" , tipo ] , bloque ;
parametros      = parametro , { "," , parametro } ;
parametro       = IDENTIFICADOR , [ ":" , tipo ] ;
retorno         = "retornar" , [ expresion ] ;
mostrar         = "mostrar" , expresion ;
mostrar_sin_salto = "mostrar_sin_salto" , expresion ;
intentar_rescatar = "intentar" , bloque , "rescatar" , [ IDENTIFICADOR ] , bloque ;
importar        = "importar" , CADENA ;
romper          = "romper" ;
continuar       = "continuar" ;

tipo            = "entero" | "decimal" | "real" | "logico" | "texto" | "nulo"
                | "arreglo" , "<" , tipo , "," , ENTERO , ">"
                | "lista" , "<" , tipo , ">" ;

expresion       = expresion_o ;
expresion_o     = expresion_y , { "o" , expresion_y } ;
expresion_y     = expresion_no , { "y" , expresion_no } ;
expresion_no    = "no" , expresion_no | comparacion ;
comparacion     = expresion_suma , [ comparador , expresion_suma ] ;
comparador      = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
expresion_suma  = expresion_producto , { ( "+" | "-" ) , expresion_producto } ;
expresion_producto = primaria , { ( "*" | "/" | "%" ) , primaria } ;
primaria        = ENTERO | DECIMAL | REAL | "verdadero" | "falso" | "nulo"
                | CADENA | IDENTIFICADOR | llamada | indexacion
                | "(" , expresion , ")" | arreglo_literal | rango ;
llamada         = IDENTIFICADOR , "(" , [ expresion , { "," , expresion } ] , ")" ;
indexacion      = expresion , "[" , expresion , "]" ;
arreglo_literal = "[" , [ expresion , { "," , expresion } ] , "]" ;
```

## Precedencia normativa de referencia
De mayor a menor, según ADR-0007:
1. Paréntesis.
2. `*`, `/`, `%`.
3. `+`, `-`.
4. Comparaciones (`==`, `!=`, `<`, `<=`, `>`, `>=`); no se permiten comparaciones encadenadas.
5. `no`.
6. `y`.
7. `o`.

Los operadores binarios indicados son asociativos a izquierda, salvo `no`, que es unario y se asocia a derecha. La tabla debe mantenerse sincronizada con ADR-0007 y los tests de precedencia.

## Restricciones observadas
- La llave de apertura de `si`, `sino` y `mientras` debe ir en la misma línea de la palabra clave/condición, conforme a ADR-0007.
- El salto de línea termina una sentencia cuando esta está completa; puede ignorarse en los contextos de continuación que define el parser.
- Los identificadores del prototipo son ASCII (`[A-Za-z_][A-Za-z0-9_]*`); las cadenas admiten Unicode en el lexer.
- `funcion`, `retornar` y `mostrar` están implementados en el prototipo (ADR-0008), pero esta gramática de referencia no se ha actualizado todavía para reflejarlos por completo.
- El token de cadena existe en el lexer y el prototipo acepta cadenas como expresión primaria desde el 2026-10-22; la gramática normativa aún debe actualizarse.
- El signo negativo se admite en la posición de literal numérico negativo; no equivale a un operador unario general aplicable a cualquier expresión.

## Límites
Esta gramática de referencia todavía no incluye funciones ni llamadas, aunque el prototipo sí las implementa desde el 2026-10-22. La gramática debe actualizarse y transformarse en una norma de conformidad automatizada antes de declarar especificación completa.


## Sincronización con implementación

Comparado con `prototipo/src/parser.rs`:

- **Sentencias:** `sea`, asignación, `si`/`sino`/`sino si`, `mientras`, bloque, expresión
- **Precedencia:** `( )` → `* / %` → `+ -` → comparaciones → `no` → `y` → `o`
- **Bloques:** `{` debe estar en la misma línea que `si`/`sino`/`mientras` (ADR-0007 §1.6)
- **Lógicos:** `verdadero`/`falso`, `y`/`o` con cortocircuito, `no` unario
- **Comparaciones:** sin encadenamiento
- **Variables:** `y`/`o` son palabras clave (migración requerida si se usaban como identificadores)

**Fuente de verdad para el parser:** `prototipo/src/parser.rs`. Esta gramática es de referencia y debe actualizarse ante cambios en el AST.
