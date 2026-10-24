# ADR-0011 — Sistema de tipos gradual, parámetros tipados, retorno anotado y valor nulo

- Estado: **Aprobado** (2026-10-24)
- Fecha: 2026-10-24
- Responsable: proyecto BASILISCO
- Revisión prevista: al completar la implementación del incremento 6, o antes si aparece
  evidencia que contradiga una decisión de este ADR.
- Relacionado: ADR-0005 (conversiones numéricas), ADR-0006 (declaraciones y variables),
  ADR-0007 (tipos lógicos), ADR-0008 (funciones y retorno), ADR-0012 (iteración),
  ADR-0013 (E/S), ADR-0014 (manejo de errores).
- Resuelve brechas: GAP-003 (tipos y conversiones), GAP-005 (funciones y retorno §3.3
  punto abierto), GAP-008 (colecciones — parcialmente).
- Cubre decisiones del catálogo `decisiones-que-no-deben-inventarse.md`: "lenguaje
  estático, dinámico o mixto"; "genéricos" (parcialmente, solo en parámetros de función).

---

## 1. Contexto y requisitos

El prototipo implementa variables con tipo inferido (`sea`/`var`/`dato`/`const`) y funciones
con inferencia de tipo por primera llamada (ADR-0008 §3.3). Este mecanismo funciona para
programas pequeños pero presenta dos limitaciones documentadas:

1. **Sin contrato en la interfaz de función**: la función `dividir(a, b)` no expresa que
   `a` y `b` deben ser `decimal`. El error aparece en el cuerpo o en tiempo de ejecución,
   no en el sitio de llamada.
2. **`sistema-de-tipos.md` y `decisiones-que-no-deben-inventarse.md`** listan el modelo
   de tipos como decisión pendiente, bloqueando el diseño de biblioteca estándar,
   operaciones de texto, E/S y manejo de errores (GAP-003, GAP-008, GAP-013).

Requisitos derivados de las confirmaciones del 2026-10-24:

- R1: Preservar la compatibilidad con todo el código `.bsk` existente.
- R2: No modificar ADR-0008 (el contrato de `RetornoAusente` se mantiene).
- R3: Reutilizar la gramática de tipos existente en `var`/`dato`/`const`.
- R4: No introducir genéricos definibles por el usuario en este incremento.
- R5: `nulo` debe ser distinto de `falso`, `0` y `""`.
- R6: La regla "todos los parámetros anotados o ninguno" dentro de una firma.

---

## 2. Opciones consideradas

### O1 — Tipado dinámico puro (sin anotaciones)
El sistema actual. Los tipos se infieren en ejecución; las funciones no tienen contrato
de tipo en la interfaz.
- Ventajas: sin cambio en el lenguaje.
- Costes: no permite contratos de API; los errores de tipo aparecen lejos del origen;
  bloquea la biblioteca estándar tipada.
- **Descartada**: no cierra GAP-003 ni GAP-005.

### O2 — Tipado estático obligatorio desde ahora
Toda variable y función requiere anotación de tipo; se añade un verificador estático
antes de la ejecución.
- Ventajas: contratos fuertes; errores tempranos.
- Costes: invalida todo el código `.bsk` existente (violación de R1); requiere
  un verificador de tipos completo (meses de trabajo); bloquea Puerta C.
- **Descartada**: viola R1 y bloquea el resto de la Puerta C.

### O3 — Tipado gradual (propuesta seleccionada)
Sin anotación → comportamiento actual preservado.
Con anotación → verificación en ejecución en el punto de uso.
- Ventajas: R1 y R2 satisfechos; permite contratos optativos; ruta natural hacia
  tipado estático en una versión futura; no requiere verificador estático ahora.
- Costes: la verificación ocurre en ejecución, no en análisis estático.
- Riesgos: bajo; el mecanismo de verificación ya existe en el evaluador
  (`convertir_a_tipo_anotado`).
- **Seleccionada**.

---

## 3. Decisión

### 3.1 Modelo de tipos — gradual

BASILISCO usa tipado **gradual**:

- Una declaración sin anotación de tipo (`sea x = 5`) infiere el tipo del valor inicial y
  lo fija. Comportamiento heredado de ADR-0006 §1.1.
- Una declaración con anotación de tipo (`var x: entero = 5`) verifica que el valor sea
  compatible con el tipo declarado. Comportamiento heredado de ADR-0006 §1.4.
- Una función sin anotaciones en los parámetros infiere el tipo de cada parámetro en la
  primera llamada y lo fija. Comportamiento heredado de ADR-0008 §3.3.
- Una función con anotaciones en todos los parámetros verifica el tipo de cada argumento
  antes de entrar al cuerpo. **Nuevo en este ADR.**
- El tipo de retorno puede anotarse con `-> tipo`; si se anota, cada `retornar expresion`
  dentro del cuerpo verifica la compatibilidad. **Nuevo en este ADR.**

Las reglas de conversión implícita de ADR-0005 §1.4 se aplican en todos los casos:
`entero → decimal` y `entero → real` son las únicas conversiones implícitas aprobadas.

### 3.2 Parámetros de función — todos tipados o ninguno

Dentro de una misma firma de función, todos los parámetros deben llevar anotación de tipo
o ninguno debe llevarla. La mezcla es **error sintáctico** con mensaje:

```
error sintáctico [L:C]: la firma de 'f' mezcla parámetros con y sin tipo;
  anota todos o ninguno
```

**Motivación**: una firma con tipos parciales es más difícil de entender que una sin tipos
(el programador debe recordar qué parámetros están verificados). La regla todo-o-nada es
más predecible y más fácil de diagnosticar.

Ejemplos válidos:

```
# Sin tipos — inferencia en primera llamada (ADR-0008 §3.3)
funcion sumar(a, b) {
    retornar a + b
}

# Todos tipados — verificación en cada llamada
funcion dividir(a: decimal, b: decimal) -> decimal {
    retornar a / b
}
```

Ejemplo inválido:

```
funcion mixta(a: entero, b) {   # error sintáctico: mezcla
    retornar a + b
}
```

### 3.3 Tipos válidos para parámetros

Los tipos disponibles para anotar parámetros son los mismos que en `var`/`dato`/`const`:

| Tipo | Sintaxis |
|------|---------|
| Entero | `entero` |
| Decimal | `decimal` |
| Real | `real` |
| Lógico | `logico` |
| Texto | `texto` |
| Arreglo fijo | `arreglo<T, N>` |
| Lista dinámica | `lista<T>` |

Los tipos `T` en `arreglo<T, N>` y `lista<T>` son los tipos simples anteriores
(no tipos compuestos anidados; eso requiere un ADR de genéricos). `nulo` **no** es un
tipo válido para parámetros (ver §3.5).

### 3.4 Tipo de retorno anotado — `-> tipo`

La firma de función puede incluir, opcionalmente, el tipo de retorno:

```
funcion nombre(params) -> tipo {
    ...
}
```

- Si se anota, cada `retornar expresion` en el cuerpo verifica la compatibilidad del
  valor contra el tipo anotado. El error es `RetornoIncompatibleConAnotacion`.
- Si se omite, el comportamiento es el actual: el tipo se infiere de la primera llamada
  exitosa (ADR-0008 §3.3).
- `-> nulo` es una anotación válida que declara explícitamente que la función no retorna
  un valor útil. Es equivalente a no tener anotación de retorno cuando se combina con
  `retornar nulo` o con ausencia de `retornar`.
- La anotación `-> tipo` es **independiente** de si los parámetros están anotados: una
  función puede tener `-> entero` sin anotar parámetros, o anotar parámetros sin `-> tipo`.
- Una función con `-> tipo` anotado que ejecuta `retornar` sin expresión es error de
  ejecución `RetornoAusente` (preserva ADR-0008 §3.5 intacto).

#### Token `->` en el lexer

La secuencia de caracteres `->` (guion seguido de mayor, sin espacio entre ellos) se
tokeniza como el token `Flecha`. El lexer debe comprobar este caso antes de emitir
`Menos` y `Mayor` de forma independiente.

**Compatibilidad**: los programas existentes que usen `-` y `>` como operadores separados
(`a - b > 0`, `a -> b` escrito con espacio) no se ven afectados: `-` seguido de cualquier
carácter que no sea `>` sigue siendo `Menos`; `->` solo nace cuando los dos caracteres son
adyacentes sin espacio.

Pruebas léxicas requeridas (ver §7):
- `a -> b` con espacio tokeniza como `Menos` + `Mayor`.  
  **Nota:** `a -> b` es `a`, `Menos`, `>` `b` solo si hay espacio entre `-` y `>`.
  Sin espacio, `->` es `Flecha`. El lexer distingue por adyacencia.
- `a->b` sin espacio tokeniza como `Identificador("a")`, `Flecha`, `Identificador("b")`.
- `funcion f() -> entero` tokeniza con `Flecha` antes de `entero`.
- `a - b > c` tokeniza como `Menos` + `Mayor` (con espacios, son tokens distintos).

### 3.5 Valor `nulo`

Se introduce `nulo` como nuevo valor primitivo del lenguaje.

**Propiedades:**
- Es un valor, no un tipo numérico ni lógico. El nombre de su tipo es `"nulo"`.
- Distinto de `falso`, `0`, `0.0`, `""`, y de cualquier otro valor del lenguaje.
- No puede participar en operaciones aritméticas (`nulo + 1` → error `TiposIncompatibles`).
- No puede usarse como condición (`si nulo { ... }` → error `CondicionNoLogica`).
- No puede compararse con `==`/`!=` con tipos distintos → error `TiposIncompatibles`.
  `nulo == nulo` → `verdadero`. `nulo != nulo` → `falso`. (Coherente con: dos ausencias
  son la misma ausencia; no hay identidad de objeto en este modelo.)
- Se puede almacenar en una variable: `var x: nulo = nulo`.
- Una función puede retornarlo explícitamente: `retornar nulo`.
- Literal en el código fuente: `nulo` (palabra clave reservada).

**Relación con ADR-0008 §3.5 — sin modificación:**
Una función que no ejecuta `retornar` cuando su resultado se usa como expresión **sigue
produciendo el error `RetornoAusente`**. La introducción de `nulo` no cambia esto.
El programador que quiera señalar ausencia de valor explícitamente escribe `retornar nulo`.
El comportamiento implícito ante la falta de `retornar` no cambia.

```
# Válido: retorno explícito de nulo
funcion sin_resultado() -> nulo {
    mostrar "hola"
    retornar nulo
}

# Error RetornoAusente — no cambia respecto a ADR-0008
funcion olvidado() {
    sea x = 1
}
sea r = olvidado()   # error: RetornoAusente
```

---

## 4. Reglas semánticas completas

### 4.1 Verificación de parámetros anotados

Al ejecutar una llamada a función con parámetros anotados:

1. Se evalúan todos los argumentos.
2. Para cada par `(argumento_i, tipo_i)`: se aplica `convertir_a_tipo_anotado(argumento_i, tipo_i)`.
3. Si alguna conversión devuelve `None` → error `TipoParametroIncompatible { funcion, parametro, esperado, obtenido }` con la ubicación de la llamada.
4. Si todas las conversiones son exitosas → se entra al cuerpo con los valores convertidos.

Las conversiones implícitas aprobadas de ADR-0005 §1.4 se aplican durante el paso 2
(`entero → decimal`, `entero → real`).

### 4.2 Verificación del retorno anotado

Al ejecutar `retornar expresion` dentro de una función con `-> tipo`:

1. Se evalúa la expresión.
2. Se aplica `convertir_a_tipo_anotado(valor, tipo_retorno)`.
3. Si falla → error `RetornoIncompatibleConAnotacion { funcion, esperado, obtenido }`.
4. Si tiene éxito → se retorna el valor convertido.

### 4.3 Nuevos errores (`CausaError`)

```
TipoParametroIncompatible {
    funcion:   String,
    parametro: String,
    esperado:  String,
    obtenido:  String,
}

RetornoIncompatibleConAnotacion {
    funcion:  String,
    esperado: String,
    obtenido: String,
}
```

Ambos errores llevan ubicación (línea/columna de la llamada o del `retornar`).
Deben sincronizarse con `02-lenguaje/catalogo-de-errores-y-diagnosticos.md`.

### 4.4 Orden de verificación en una llamada tipada

```
evaluar argumentos → verificar aridad → verificar tipos → entrar al cuerpo
```

La verificación de tipos ocurre después de verificar la aridad para que
`FirmaInvalida` (aridad incorrecta) no quede enmascarado por `TipoParametroIncompatible`.

### 4.5 `nulo` en operaciones

| Operación | Resultado |
|-----------|-----------|
| `nulo + cualquier_cosa` | Error `TiposIncompatibles` |
| `nulo == nulo` | `verdadero` |
| `nulo != nulo` | `falso` |
| `nulo == otro_tipo` | Error `TiposIncompatibles` |
| `si nulo { ... }` | Error `CondicionNoLogica { obtenido: "nulo" }` |
| `no nulo` | Error `OperandoNoLogico { operador: "no", obtenido: "nulo" }` |
| `nulo y verdadero` | Error `OperandoNoLogico` |
| Convertir nulo a entero/decimal/real | Error `ConversionIlegal` |
| Mostrar nulo | Imprime el texto `"nulo"` |

---

## 5. Gramática ampliada

```ebnf
declaracion_funcion = "funcion" , IDENT , "(" , params_tipados? , ")"
                    , ( "->" , tipo_anotacion )?
                    , bloque ;

params_tipados      = params_con_tipo | params_sin_tipo ;

params_sin_tipo     = IDENT , { "," , IDENT } ;
                      (* igual que antes; ninguno lleva ":" tipo *)

params_con_tipo     = IDENT , ":" , tipo_anotacion ,
                      { "," , IDENT , ":" , tipo_anotacion } ;
                      (* todos llevan ":" tipo *)

(* Restricción semántica: no se puede mezclar params_sin_tipo y
   params_con_tipo en la misma firma. El parser puede detectarlo
   o el análisis semántico puede hacerlo; en cualquier caso el
   error debe ser sintáctico (antes de la ejecución). *)

primaria            = ... | "nulo" | ... ;   (* nuevo literal *)

tipo_anotacion      = "entero" | "decimal" | "real" | "logico" | "texto"
                    | "nulo"
                    | "arreglo" , "<" , tipo_simple , "," , ENTERO , ">"
                    | "lista"   , "<" , tipo_simple , ">" ;

tipo_simple         = "entero" | "decimal" | "real" | "logico" | "texto" ;
                      (* tipos simples; no se admiten arreglo/lista anidados *)
```

Nuevo token léxico:

```
"->"   →  TipoToken::Flecha
"nulo" →  TipoToken::Nulo
```

---

## 6. Compatibilidad y migración

- **Todo el código `.bsk` existente sigue siendo válido** [VERIFIED por análisis]: ninguna
  función actual usa anotaciones de tipo. La inferencia por primera llamada se preserva.
- **Sin cambio en el comportamiento de funciones existentes.**
- **Nuevo token `Flecha`**: no hay secuencia `->` en ningún archivo `.bsk` de prueba
  existente [VERIFIED con grep en `prototipo/tests/` y `prototipo/examples/`].
- **Nueva palabra clave `nulo`**: ningún test ni ejemplo usa `nulo` como identificador
  [VERIFIED con grep].

---

## 7. Criterios de aceptación (pruebas requeridas)

Las siguientes suites deben existir y pasar antes de declarar este ADR implementado:

### Léxico
- `->` adyacente tokeniza como `Flecha`.
- `- >` con espacio tokeniza como `Menos` + `Mayor`.
- `nulo` tokeniza como `TipoToken::Nulo`.
- Un identificador llamado `nulo` es rechazado (ya no es identificador válido).

### Funciones con parámetros tipados
- Función con todos los parámetros tipados: llamada válida con tipos correctos → ok.
- Función tipada llamada con tipo incorrecto → `TipoParametroIncompatible`.
- Función tipada llamada con `entero` donde se espera `decimal` → conversión implícita ok.
- Función tipada llamada con `decimal` donde se espera `entero` → `TipoParametroIncompatible`.
- Función sin tipos: comportamiento de ADR-0008 preservado (inferencia por primera llamada).
- Firma con mezcla de parámetros → error sintáctico.

### Retorno anotado
- Función con `-> entero` que retorna un entero → ok.
- Función con `-> decimal` que retorna un entero → conversión implícita ok.
- Función con `-> entero` que retorna un decimal → `RetornoIncompatibleConAnotacion`.
- Función con `-> nulo` que ejecuta `retornar nulo` → ok.
- Función con `-> nulo` que ejecuta `retornar 5` → `RetornoIncompatibleConAnotacion`.
- Función sin `->` sin `retornar` usada como expresión → `RetornoAusente` (regresión ADR-0008).

### Valor `nulo`
- `nulo == nulo` → `verdadero`.
- `nulo != nulo` → `falso`.
- `nulo + 1` → error `TiposIncompatibles`.
- `si nulo { ... }` → error `CondicionNoLogica`.
- `retornar nulo` en una función → ok; la función retorna `Valor::Nulo`.
- `mostrar nulo` → imprime `"nulo"`.
- `var x: nulo = nulo` → declaración válida.

### Regresión
- Todas las 268 pruebas existentes siguen en verde.

---

## 8. Archivos a modificar (solo para el plan; no se modifican ahora)

| Archivo | Cambio |
|---------|--------|
| `prototipo/src/token.rs` | Añadir `TipoToken::Flecha`, `TipoToken::Nulo` |
| `prototipo/src/lexer.rs` | Reconocer `->` como `Flecha`; `"nulo"` como `Nulo` |
| `prototipo/src/parser.rs` | Parsear anotaciones en parámetros, `-> tipo`, literal `nulo` |
| `prototipo/src/evaluador.rs` | Verificar tipos en llamadas y retornos; nuevas causas de error |
| `prototipo/src/valor.rs` | Añadir `Valor::Nulo`; actualizar `nombre_tipo`, `Display`, operaciones |
| `prototipo/tests/conformidad_tipos.rs` | Nueva suite (criterios de §7) |
| `prototipo/tests/conformidad_funciones.rs` | Nuevos casos tipados |
| `02-lenguaje/catalogo-de-errores-y-diagnosticos.md` | Añadir `TipoParametroIncompatible`, `RetornoIncompatibleConAnotacion` |
| `10-especificacion/gramatica-de-referencia-0.3.md` | Actualizar con gramática §5 |

---

## 9. Riesgos y condiciones de revisión

| Riesgo | Probabilidad | Mitigación |
|--------|-------------|------------|
| La verificación en ejecución en vez de estática dificulta depuración | Media | Los mensajes de error incluyen la ubicación de la llamada, no solo del cuerpo |
| El token `->` rompe código con ese par de caracteres adyacentes | Baja | Verificado: no existe en el código actual; el lexer tiene prueba de regresión |
| `nulo` como palabra clave invalida identificadores `nulo` existentes | Baja | Verificado: no existe en el código actual |
| La regla todo-o-nada en parámetros resulta frustrante en funciones largas | Media | Es revisable con evidencia de usabilidad; se registra como condición de revisión |

**Condiciones de revisión:**
- Si la evidencia de usabilidad muestra que la regla todo-o-nada dificulta la adopción,
  se puede relajar a "si el primer parámetro tiene tipo, todos deben tenerlo".
- Si se introduce un verificador de tipos estático en una versión futura, la verificación
  en ejecución se puede elevar a verificación previa sin romper la semántica.

---

## 10. Lo que este ADR no decide

- Genéricos definibles por el usuario (tipos `Mapa<K,V>`, `Opcion<T>`, etc.).
- `resultado<T>` — diferido a ADR-0014.
- Tipos algebraicos / uniones etiquetadas.
- Inferencia de tipos en toda la expresión (Hindley-Milner).
- Verificación estática antes de la ejecución.
- Interfaces, traits o protocolos.
