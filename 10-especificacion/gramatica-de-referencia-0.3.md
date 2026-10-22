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
- `funcion`, `retornar` y `mostrar` son palabras reservadas en el lexer, pero eso no significa que el parser o evaluador implementen esas construcciones.
- El token de cadena existe en el lexer, pero la gramática de expresiones del parser actual no acepta cadenas como expresión primaria.
- El signo negativo se admite en la posición de literal numérico negativo; no equivale a un operador unario general aplicable a cualquier expresión.

## Límites
Esta gramática no incluye funciones ni llamadas: ADR-0008 sigue propuesto y el parser actual no contiene nodos de AST para declaración/llamada de funciones o retorno. La gramática debe transformarse en una norma de conformidad automatizada antes de declarar especificación completa.
