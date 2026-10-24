# Plan de implementación — Incrementos 6, 7 y 8

> Documento generado por el agente planificador. Solo describe qué implementar;
> no modifica código.
>
> **Restricciones globales (aplican a TODOS los ítems):**
> - No hacer commits ni push.
> - No modificar contratos de ADR-0001 a ADR-0010 ya implementados.
> - `cargo fmt --check` debe pasar limpio antes de cada verificación.
> - `cargo clippy --all-targets -- -D warnings` debe pasar sin errores.
>   No usar `#[allow(...)]` para silenciar advertencias; corrígelas.
> - `cargo test` debe pasar con 0 fallos tras cada ítem.
> - Las 268 pruebas actuales deben permanecer en verde tras cada incremento.
> - El proyecto usa la edición Rust 2024 (`Cargo.toml`).
> - `Valor::Archivo` contiene estado mutable; usar `Arc<Mutex<...>>` para que
>   `Valor` siga siendo `Clone` sin derivar `PartialEq` en él (o implementarlo
>   manualmente ignorando el campo de archivo).
> - `romper`/`continuar` en el evaluador: añadir variantes al enum interno
>   `ResultadoEjecucion`; los ciclos (`para cada`, `mientras`) los interceptan;
>   `si` y `bloque` los propagan hacia arriba sin capturarlos.
> - Las pruebas de `leer()` (stdin interactivo) pueden marcarse con `#[ignore]`
>   acompañado de un comentario `// requiere stdin interactivo; omitir en CI`.

---

## Verificación previa obligatoria

Antes de comenzar el incremento 6, confirmar que las 268 pruebas pasan:

```
cargo test              # → 268 passed, 0 failed
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

---

## Incremento 6 — Sistema de tipos gradual (ADR-0011)

### Ítem 6.1 — Añadir `TipoToken::Flecha` y `TipoToken::Nulo` en `token.rs`

**Qué hacer:**
Añadir dos nuevas variantes al enum `TipoToken` en `src/token.rs`, después de
`DosPuntos` y antes de `FinDeLinea`:

```rust
/// Flecha `->` para anotación de tipo de retorno (ADR-0011).
Flecha,
/// Palabra clave `nulo` (ADR-0011 §3.5).
Nulo,
```

`TipoToken` ya deriva `Debug, Clone, PartialEq`; no se necesita cambio adicional.

**Archivos:** `src/token.rs`

**Verificar:**
```
cargo build 2>&1   # compilación limpia (las nuevas variantes no se usan aún)
```

---

### Ítem 6.2 — Reconocer `->` y `nulo` en `lexer.rs`

**Qué hacer:**

**A. Token `Flecha` (`->`):**
En el bloque `match c` dentro de `tokenizar()`, la rama `'-'` actualmente hace:
```rust
'-' => { self.avanzar(); TipoToken::Menos }
```
Cambiarla a:
```rust
'-' => {
    self.avanzar();
    if self.actual() == Some('>') {
        self.avanzar();
        TipoToken::Flecha
    } else {
        TipoToken::Menos
    }
}
```
Esto se ejecuta ANTES de emitir `TipoToken::Menos`; al ver `-` se mira el siguiente
carácter. Si es `>` y es adyacente (ya lo es, porque el lexer avanzó por `-`), se
emite `Flecha` y se avanza una posición más. Si el siguiente carácter no es `>`,
se emite `Menos` como antes.

ADR-0011 §3.4: `a - b > 0` tiene espacios entre `-` y `>`, por lo que `-` se
tokeniza como `Menos` y `>` como `Mayor`. Solo `->` adyacente (sin espacio) produce
`Flecha`. La implementación ya garantiza esto porque los espacios se omiten *antes*
de entrar al `match c` (en `omitir_espacios_y_comentarios`), pero la adyacencia se
verifica sobre los caracteres CRUDOS: la llamada a `self.actual()` después de
`self.avanzar()` para `-` devuelve el carácter inmediatamente siguiente sin saltar
espacios. Por tanto, si hay un espacio entre `-` y `>`, `self.actual()` devolverá
`' '`, no `'>'`, y se emitirá `Menos`. Correcto.

**B. Palabra clave `nulo`:**
En `leer_identificador()`, en el bloque `match texto.as_str()`, añadir antes del
`_` final:
```rust
"nulo" => TipoToken::Nulo,
```

**Archivos:** `src/lexer.rs`

**Verificar:**
```
cargo test --lib   # pruebas internas del lexer pasan (incluidas las existentes)
```

Añadir en `tests/conformidad_tipos.rs` (creado en ítem 6.7) las pruebas léxicas
del ADR-0011 §7. Las pruebas del ítem 6.7 son las que verifican este ítem.

---

### Ítem 6.3 — Añadir `TipoAnotacion::Nulo` y extender `Sentencia::Funcion` en `parser.rs`

**Qué hacer:**

**A. Variante `Nulo` en `TipoAnotacion`:**
```rust
pub enum TipoAnotacion {
    Entero,
    Decimal,
    Real,
    Logico,
    Texto,
    Arreglo(Box<TipoAnotacion>, usize),
    Lista(Box<TipoAnotacion>),
    Nulo,   // ← nuevo (ADR-0011 §3.5)
}
```
En `TipoAnotacion::nombre()` añadir:
```rust
TipoAnotacion::Nulo => "nulo".to_string(),
```

**B. Parsear `nulo` como tipo en `parsear_tipo_anotacion_simple()`:**
Añadir antes del `_` final:
```rust
Some(TipoToken::Nulo) => {
    self.pos += 1;
    Ok(TipoAnotacion::Nulo)
}
```
Nota: `TipoToken::Nulo` no es un `Identificador(...)` sino una variante propia,
por lo que el match de `TipoToken::Identificador(t) if t == "nulo"` ya no aplica.
El token `Nulo` se produce en el lexer en el ítem 6.2.

**C. Cambiar la firma de `Sentencia::Funcion`:**
El AST actual usa `parametros: Vec<String>`. ADR-0011 §3.2 exige anotaciones de
tipo opcionales en parámetros y tipo de retorno opcional. Cambiar:

```rust
Sentencia::Funcion {
    nombre: String,
    parametros: Vec<(String, Option<TipoAnotacion>)>,  // (nombre, tipo_opcional)
    tipo_retorno: Option<TipoAnotacion>,                // -> tipo (opcional)
    cuerpo: Vec<Sentencia>,
    linea: usize,
    columna: usize,
},
```

**D. Actualizar `sentencia_funcion()` en el parser:**
Extender el bucle de parámetros para detectar la anotación opcional:

```
después de leer el identificador del parámetro:
    si el siguiente token es ':' → parsear tipo con parsear_tipo_anotacion()
       (que ya consume ':'); tipo_param = Some(tipo)
    sino → tipo_param = None

acumular como (nombre, tipo_param)
```

Validación de la regla todo-o-nada (ADR-0011 §3.2): después de recoger todos los
parámetros, verificar que todos tienen tipo o ninguno. Si hay mezcla → error
sintáctico con mensaje:
```
"la firma de 'f' mezcla parámetros con y sin tipo; anota todos o ninguno"
```

Después del `)`, intentar parsear `-> tipo`:
```
si el siguiente token es TipoToken::Flecha → avanzar; parsear tipo_anotacion_sin_dos_puntos()
    (es decir, parsear el tipo sin el ':' previo porque la flecha ya lo introduce)
sino → tipo_retorno = None
```

Nota: `parsear_tipo_anotacion()` actualmente exige `:` al inicio. Para el retorno
se necesita una función auxiliar que parsee el tipo directamente sin `:`. Extraer
`parsear_tipo_anotacion_cuerpo()` (sin consumir `:`) y usarla tanto en `parsear_tipo_anotacion()`
(que sigue consumiendo `:` primero) como en el retorno.

**Archivos:** `src/parser.rs`

**Verificar:**
```
cargo test 2>&1   # 268 pruebas siguen pasando
```

---

### Ítem 6.4 — Añadir `Valor::Nulo` en `valor.rs`

**Qué hacer:**
Añadir la variante al enum:
```rust
/// Valor nulo (ADR-0011 §3.5). Distinto de falso, 0 y "".
Nulo,
```

Actualizar todos los `match` sobre `Valor` que el compilador detectará como
incompletos. Cambios específicos:

**`nombre_tipo()`:**
```rust
Valor::Nulo => "nulo",
```

**`Display`:**
```rust
Valor::Nulo => write!(f, "nulo"),
```

**`es_cero()`:**
```rust
Valor::Nulo => false,
```

**`a_real()`:**
```rust
Valor::Nulo => f64::NAN,
```

**Operaciones aritméticas (`suma`, `resta`, `multiplicacion`, `division`, `modulo`):**
En cada operación, el arm `_ => Err(MotivoAritmetico::TiposIncompatibles)` ya
captura `Nulo` correctamente (al no coincidir con ningún par numérico). No se
necesita arm adicional porque `Nulo` cae en el `_`. Verificar que Clippy no
advierte sobre arms inalcanzables.

**`PartialEq` derivado:** `Valor::Nulo == Valor::Nulo` será `true` automáticamente
con el derive. `Valor::Nulo == Valor::Entero(0)` será `false`. Correcto per ADR-0011 §4.5.

**Archivos:** `src/valor.rs`

**Verificar:**
```
cargo test 2>&1   # 268 pruebas siguen pasando
```

---

### Ítem 6.5 — Parsear `nulo` como literal en `parser.rs`

**Qué hacer:**
En `primaria_sin_indice()`, añadir antes del arm `Some(TipoToken::Identificador(nombre))`:

```rust
Some(TipoToken::Nulo) => {
    let t = &self.tokens[self.pos];
    let (linea, columna) = (t.linea, t.columna);
    self.pos += 1;
    Ok(Expresion::Literal {
        valor: Valor::Nulo,
        linea,
        columna,
    })
}
```

**Archivos:** `src/parser.rs`

**Verificar:**
```
cargo test 2>&1   # 268 pruebas siguen pasando
```

---

### Ítem 6.6 — Actualizar `evaluador.rs`: nuevos errores, verificación de tipos en llamadas y retorno

**Qué hacer:**

**A. Nuevas variantes en `CausaError`:**
```rust
/// Argumento de tipo incompatible en llamada tipada (ADR-0011 §4.3).
TipoParametroIncompatible {
    funcion:   String,
    parametro: String,
    esperado:  String,
    obtenido:  String,
},
/// Retorno incompatible con el tipo anotado (ADR-0011 §4.3).
RetornoIncompatibleConAnotacion {
    funcion:  String,
    esperado: String,
    obtenido: String,
},
```

Añadir sus ramas en `impl fmt::Display for CausaError`:
```rust
CausaError::TipoParametroIncompatible { funcion, parametro, esperado, obtenido } =>
    write!(f, "parámetro '{parametro}' de '{funcion}': se esperaba {esperado} y se obtuvo {obtenido}"),
CausaError::RetornoIncompatibleConAnotacion { funcion, esperado, obtenido } =>
    write!(f, "retorno de '{funcion}': se esperaba {esperado} y se obtuvo {obtenido}"),
```

**B. Actualizar `FuncionDef`:**
```rust
pub struct FuncionDef {
    pub nombre: String,
    pub parametros: Vec<(String, Option<TipoAnotacion>)>,  // ← cambiado
    pub tipo_retorno_anotado: Option<TipoAnotacion>,        // ← nuevo: anotación del programador
    pub cuerpo: Vec<Sentencia>,
    pub linea: usize,
    pub columna: usize,
    pub tipo_retorno: Option<&'static str>,                 // inferido en primera llamada
}
```

**C. Actualizar `convertir_a_tipo_anotado()` para `TipoAnotacion::Nulo`:**
```rust
TipoAnotacion::Nulo => match valor {
    Valor::Nulo => Some(valor),
    _ => None,
},
```

**D. Actualizar el registro de funciones en `ejecutar_sentencias()`:**
Donde se hace `funciones.insert(...)` al encontrar `Sentencia::Funcion`, actualizar
para usar el nuevo campo `parametros: Vec<(String, Option<TipoAnotacion>)>` y
`tipo_retorno_anotado`.

**E. Verificación de tipos en llamadas (`evaluar_con`, rama `Expresion::Llamada`):**
Después de evaluar los argumentos, antes de entrar al cuerpo:

```
para cada (param_nombre, param_tipo_opt) en fdef.parametros.iter().zip(vals.iter()):
    si param_tipo_opt es Some(tipo):
        aplicar convertir_a_tipo_anotado(val, tipo)
        si falla → error TipoParametroIncompatible { funcion, parametro: param_nombre, esperado: tipo.nombre(), obtenido: val.nombre_tipo() }
        si ok → usar el valor convertido
```

Orden de verificación (ADR-0011 §4.4): aridad primero (ya implementada), luego tipos.

Al insertar los parámetros en el nuevo entorno de la función, usar los valores convertidos.

**F. Verificación del retorno anotado en `ejecutar_funcion_sentencias()`, rama `Sentencia::Retorno`:**

El nombre de la función debe propagarse hasta `ejecutar_funcion_sentencias`. El método
`ejecutar_funcion` ya recibe `nombre_funcion: &str`. Para verificar el tipo de retorno
anotado, el evaluador necesita acceso al `tipo_retorno_anotado` de la definición.

Opción concreta: pasar `tipo_retorno_anotado: Option<TipoAnotacion>` como parámetro
adicional a `ejecutar_funcion()` y `ejecutar_funcion_sentencias()`, extraído de
`fdef.tipo_retorno_anotado.clone()` antes de llamar.

En la rama `Sentencia::Retorno` de `ejecutar_funcion_sentencias()`:
```
si hay valor de retorno y tipo_retorno_anotado es Some(tipo):
    evaluar el valor
    aplicar convertir_a_tipo_anotado(val, tipo)
    si falla → error RetornoIncompatibleConAnotacion { funcion, esperado, obtenido }
    si ok → retornar el valor convertido
```

**G. Actualizar los arms de `Sentencia::Funcion` en `ejecutar_sentencias()`:**
Donde se extrae `parametros` como `Vec<String>` para insertar en `FuncionDef`,
actualizar para usar `Vec<(String, Option<TipoAnotacion>)>`.

**H. Operaciones con `Valor::Nulo` en `comparar()`:**
Actualmente `comparar` primero filtra lógicos. Añadir filtro para `Nulo`:

```rust
// nulo solo se puede comparar con nulo (==, !=)
if matches!(va, Valor::Nulo) || matches!(vb, Valor::Nulo) {
    // Solo == y != están permitidos; ambos deben ser Nulo
    if matches!(va, Valor::Nulo) && matches!(vb, Valor::Nulo) {
        // nulo == nulo → verdadero; nulo != nulo → falso
        // El predicado recibe Ordering::Equal para nulo==nulo
        return Ok(Valor::Logico(pred(std::cmp::Ordering::Equal)));
    } else {
        return Err(error(
            CausaError::TiposIncompatibles {
                izquierdo: va.nombre_tipo(),
                derecho: vb.nombre_tipo(),
            },
            linea,
            columna,
        ));
    }
}
```

**I. `si nulo { ... }` → `CondicionNoLogica`:**
La rama `Valor::Logico(b) => b` en la comprobación de condición ya deja caer `Nulo`
en el `_` que produce `CondicionNoLogica`. No se necesita cambio adicional.

**J. `no nulo` → `OperandoNoLogico`:**
El arm `Valor::Logico(b) => Ok(Valor::Logico(!b))` ya tiene `_ => error(OperandoNoLogico)`.
No se necesita cambio adicional.

**Archivos:** `src/evaluador.rs`

**Verificar:**
```
cargo test 2>&1   # 268 pruebas siguen pasando
```

---

### Ítem 6.7 — Crear `tests/conformidad_tipos.rs`

**Qué hacer:**
Crear el archivo con la suite de pruebas del ADR-0011 §7. Función auxiliar de
test siguiendo el patrón de `conformidad_funciones.rs`:

```rust
use basilisco::{Lexer, Parser, Valor, evaluador::Entorno};

fn correr(fuente: &str) -> Result<Option<Valor>, String> { ... }
fn ok(fuente: &str) -> Option<Valor> { ... }
fn err(fuente: &str) -> String { ... }
```

Pruebas a implementar (cubrir TODOS los criterios de ADR-0011 §7):

**Léxico:**
- `"a->b"` tokeniza como `Identificador("a")`, `Flecha`, `Identificador("b")`.
- `"a - b > 0"` tokeniza como `Identificador`, `Menos`, `Identificador`, `Mayor`, `Numero`.
- `"nulo"` tokeniza como `TipoToken::Nulo`.

**Funciones con parámetros tipados:**
- Función con todos los parámetros tipados: llamada válida → ok.
- Función tipada con tipo incorrecto → mensaje contiene `"parámetro"`.
- `entero` donde se espera `decimal` → conversión implícita ok (devuelve decimal).
- `decimal` donde se espera `entero` → error `TipoParametroIncompatible`.
- Función sin tipos: comportamiento ADR-0008 preservado.
- Firma con mezcla → error sintáctico con `"mezcla parámetros"`.

**Retorno anotado:**
- `-> entero` retornando entero → ok.
- `-> decimal` retornando entero → conversión implícita ok.
- `-> entero` retornando decimal → error con `"retorno"`.
- `-> nulo` con `retornar nulo` → ok, valor `Valor::Nulo`.
- `-> nulo` con `retornar 5` → error `RetornoIncompatibleConAnotacion`.
- Sin `->`, sin `retornar`, usada como expresión → `RetornoAusente` (regresión ADR-0008).

**Valor `nulo`:**
- `nulo == nulo` → `Valor::Logico(true)`.
- `nulo != nulo` → `Valor::Logico(false)`.
- `nulo + 1` → error con `"tipos incompatibles"` (o `"incompatible"`).
- `si nulo { ... }` → error `CondicionNoLogica`.
- `retornar nulo` → `Valor::Nulo`.
- `mostrar nulo` → no falla (se imprime `"nulo"`; verificar que no produce error).
- `var x: nulo = nulo` → declaración válida.

**Archivos:** `tests/conformidad_tipos.rs`

**Verificar:**
```
cargo test --test conformidad_tipos 2>&1
cargo test 2>&1   # todas las pruebas (268 anteriores + nuevas) pasan
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

---

## Incremento 7 — Iteración y E/S (ADR-0012 + ADR-0013)

> Dependencia: el Incremento 6 debe estar completado y en verde.

### Ítem 7.1 — Nuevos tokens en `token.rs`

**Qué hacer:**
Añadir al enum `TipoToken` (después de `Nulo` o en la sección de palabras clave):

```rust
/// Palabra clave `para` (ADR-0012 §3.1).
Para,
/// Palabra clave `cada` (ADR-0012 §3.1).
Cada,
/// Palabra clave `en` (ADR-0012 §3.1).
En,
/// Palabra clave `romper` (ADR-0012 §3.2).
Romper,
/// Palabra clave `continuar` (ADR-0012 §3.2).
Continuar,
/// Operador de rango exclusivo `..` (ADR-0012 §3.3).
RangoDos,
/// Operador de rango inclusivo `..=` (ADR-0012 §3.3).
RangoDosCerrado,
/// Palabra clave `mostrar_sin_salto` (ADR-0013 §3.3).
MostrarSinSalto,
```

**Archivos:** `src/token.rs`

**Verificar:**
```
cargo build   # compila limpio
```

---

### Ítem 7.2 — Reconocer nuevos tokens en `lexer.rs`

**Qué hacer:**

**A. Palabras clave en `leer_identificador()`:**
Añadir al bloque `match texto.as_str()`:
```rust
"para"      => TipoToken::Para,
"cada"      => TipoToken::Cada,
"en"        => TipoToken::En,
"romper"    => TipoToken::Romper,
"continuar" => TipoToken::Continuar,
"mostrar_sin_salto" => TipoToken::MostrarSinSalto,
```
Nota: `mostrar_sin_salto` contiene `_` y dígitos alfanuméricos, se captura con
`es_parte_identificador` sin cambio porque `_` ya está permitido.

**B. Tokens `..` y `..=`:**
Añadir en el bloque `match c` de `tokenizar()`, ANTES del arm `'0'..='9'` (para que
el punto no confunda al `leer_numero`):

```rust
'.' => {
    // Solo en posición que no sea parte de un número (el número ya fue consumido).
    // leer_numero() termina sin consumir '.' si no hay dígito siguiente.
    // Aquí '.' en posición no-numérica forma '..' o '..='
    self.avanzar(); // primer '.'
    if self.actual() == Some('.') {
        self.avanzar(); // segundo '.'
        if self.actual() == Some('=') {
            self.avanzar(); // '='
            TipoToken::RangoDosCerrado
        } else {
            TipoToken::RangoDos
        }
    } else {
        return Err(self.error("carácter inesperado: '.' suelto (¿quiso escribir '..' o '..='?)"));
    }
}
```

La regla de ADR-0012 §5 nota: un `.` que no forma `..` ni `..=` es error léxico.
El `leer_numero()` ya consume `X.Y` (decimal) dentro de él; cuando llegamos al arm
`'.'` del loop principal, ya pasamos el número. Esto es correcto porque `leer_numero()`
se llama desde el arm `'0'..='9'`, que consume los dígitos primero.

**Archivos:** `src/lexer.rs`

**Verificar:**
```
cargo test --lib   # pruebas internas del lexer
```

---

### Ítem 7.3 — AST: nuevas variantes de `Sentencia` y `Expresion` en `parser.rs`

**Qué hacer:**

**A. Nueva variante en `Sentencia`:**
```rust
/// `para cada IDENT en expr_iteracion { ... }` (ADR-0012 §3.1).
ParaCada {
    variable: String,
    iterable: Expresion,   // colección o rango
    cuerpo: Vec<Sentencia>,
    linea: usize,
    columna: usize,
},
/// `romper` (ADR-0012 §3.2). Solo válido dentro de un ciclo.
Romper { linea: usize, columna: usize },
/// `continuar` (ADR-0012 §3.2). Solo válido dentro de un ciclo.
Continuar { linea: usize, columna: usize },
/// `mostrar_sin_salto expresión` (ADR-0013 §3.3).
MostrarSinSalto {
    valor: Expresion,
    linea: usize,
    columna: usize,
},
/// Bloque `intentar { ... } rescatar [IDENT] { ... }` (ADR-0014, Incremento 8).
/// Se define aquí para que el parser lo conozca; se implementa en el Incremento 8.
```

**B. Nueva variante en `Expresion`:**
```rust
/// Rango `inicio..fin` o `inicio..=fin` (ADR-0012 §3.3).
/// Solo válido como `iterable` en `para cada`; no almacenable en variable.
Rango {
    inicio: Box<Expresion>,
    fin: Box<Expresion>,
    inclusivo: bool,   // false = `..`; true = `..=`
    linea: usize,
    columna: usize,
},
```

**C. Añadir campo `en_ciclo: usize` al `Parser`:**
Para detectar `romper`/`continuar` fuera de ciclo:
```rust
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    en_grupo: usize,
    en_funcion: bool,
    en_ciclo: usize,   // ← nuevo: profundidad de ciclos anidados
}
```
Inicializar en 0 en `Parser::nuevo()`.

**D. Nuevo método `sentencia_para_cada()`:**
```
consume 'para'
consume 'cada' (si no hay -> error: "se esperaba 'cada' después de 'para'")
lee el identificador de la variable de iteración
consume 'en'
parsea expr_iteracion (ver E)
en_ciclo += 1
parsea bloque (con llave en la misma línea que 'para')
en_ciclo -= 1
retorna Sentencia::ParaCada { variable, iterable, cuerpo, linea, columna }
```

**E. Parseo de `expr_iteracion` con rango:**
Parsear una expresión; después comprobar si el siguiente token es `RangoDos` o
`RangoDosCerrado`. Si lo es, la expresión ya parseada es `inicio` y se parsea `fin`:
```
inicio = expresion()
if token_actual == RangoDos o RangoDosCerrado:
    inclusivo = (token == RangoDosCerrado)
    avanzar
    fin = expresion()
    retorna Expresion::Rango { inicio, fin, inclusivo, linea, columna }
else:
    retorna inicio   # es una colección directa
```

Este parseo se hace en un método auxiliar `expr_iteracion()` llamado desde
`sentencia_para_cada()`.

**F. Parseo de `romper` y `continuar`:**
Añadir en `sentencia()`:
```rust
} else if matches!(self.actual(), Some(TipoToken::Romper)) {
    let t = &self.tokens[self.pos];
    if self.en_ciclo == 0 {
        return Err(self.error("'romper' fuera de un ciclo"));
    }
    let (linea, columna) = (t.linea, t.columna);
    self.pos += 1;
    self.terminar_sentencia()?;
    return Ok(Sentencia::Romper { linea, columna });
} else if matches!(self.actual(), Some(TipoToken::Continuar)) {
    let t = &self.tokens[self.pos];
    if self.en_ciclo == 0 {
        return Err(self.error("'continuar' fuera de un ciclo"));
    }
    let (linea, columna) = (t.linea, t.columna);
    self.pos += 1;
    self.terminar_sentencia()?;
    return Ok(Sentencia::Continuar { linea, columna });
}
```

Actualizar también el bloque `while` existente (`sentencia_mientras()`) para
incrementar/decrementar `en_ciclo`:
```
en_ciclo += 1
parsear bloque
en_ciclo -= 1
```

**G. Parseo de `mostrar_sin_salto`:**
```rust
} else if matches!(self.actual(), Some(TipoToken::MostrarSinSalto)) {
    let t = &self.tokens[self.pos];
    let (linea, columna) = (t.linea, t.columna);
    self.pos += 1;
    let expr = self.expresion()?;
    self.terminar_sentencia()?;
    return Ok(Sentencia::MostrarSinSalto { valor: expr, linea, columna });
}
```

Añadir `para_cada` en el dispatch principal de `sentencia()`:
```rust
} else if matches!(self.actual(), Some(TipoToken::Para)) {
    let st = self.sentencia_para_cada()?;
    self.terminar_sentencia()?;
    return Ok(st);
}
```

**Archivos:** `src/parser.rs`

**Verificar:**
```
cargo test 2>&1   # todas las pruebas anteriores siguen pasando
```

---

### Ítem 7.4 — Infraestructura de builtins y señales de ciclo en `evaluador.rs`

**Qué hacer:**

**A. Añadir variantes `Romper` y `Continuar` a `ResultadoEjecucion`:**
```rust
#[derive(Debug, Clone)]
enum ResultadoEjecucion {
    Ninguno,
    Valor(Valor),
    Retorno(Valor),
    Romper,     // ← nuevo (ADR-0012 §3.2)
    Continuar,  // ← nuevo (ADR-0012 §3.2)
}
```
El enum actualmente está `#[allow(dead_code)]`; con el uso nuevo se elimina esa anotación.

**B. Refactorizar el evaluador para propagar `Romper`/`Continuar`:**
El evaluador tiene dos ramas de ejecución: `ejecutar_sentencias` (nivel superior, no
en función) y `ejecutar_funcion_sentencias` (dentro de función). Ambas necesitan
propagar las señales de ciclo.

Opción concreta (la más limpia dada la arquitectura existente): el `ErrorEjecucion`
es el mecanismo de señal actual. Añadir dos nuevas variantes a `CausaError` que actúan
como señales internas (interceptadas por los ciclos, nunca vistas por el usuario):

```rust
/// Señal interna: `romper` dentro de ciclo (ADR-0012). Interceptada por `ParaCada`/`Mientras`.
SenalRomper,
/// Señal interna: `continuar` dentro de ciclo (ADR-0012). Interceptada por `ParaCada`/`Mientras`.
SenalContinuar,
```

Añadir sus arms en `Display`:
```rust
CausaError::SenalRomper => write!(f, "romper fuera de ciclo (error interno)"),
CausaError::SenalContinuar => write!(f, "continuar fuera de ciclo (error interno)"),
```

Las sentencias `Sentencia::Romper` y `Sentencia::Continuar` emiten estos errores:
```rust
Sentencia::Romper { linea, columna } =>
    return Err(error(CausaError::SenalRomper, linea, columna)),
Sentencia::Continuar { linea, columna } =>
    return Err(error(CausaError::SenalContinuar, linea, columna)),
```

Los ciclos (`Mientras`, `ParaCada`) interceptan `SenalRomper` y `SenalContinuar`.
`Si` y `Bloque` los propagan sin capturar.

**C. Actualizar `Mientras` para interceptar señales:**
En el loop del `Mientras`:
```rust
match self.ejecutar_bloque(&cuerpo) {
    Ok(()) => {}
    Err(e) if matches!(e.causa, CausaError::SenalRomper) => break,
    Err(e) if matches!(e.causa, CausaError::SenalContinuar) => continue,
    Err(e) => return Err(e),
}
```
(Lo mismo en la versión dentro de función `ejecutar_bloque_funcion`.)

**D. Implementar `ParaCada` en `ejecutar_sentencias()`:**
```rust
Sentencia::ParaCada { variable, iterable, cuerpo, linea, columna } => {
    let coleccion = evaluar_con(&iterable, self)?;
    let elementos: Vec<Valor> = match coleccion {
        Valor::Lista(v) | Valor::Arreglo(v) => v,
        _ => return Err(error(
            CausaError::IteracionSobreNoColeccion { obtenido: coleccion.nombre_tipo() },
            linea, columna,
        )),
    };
    'ciclo: for elem in elementos {
        // Crear ámbito con la variable de iteración (inmutable = dato)
        self.ambitos.push(HashMap::new());
        self.insertar_variable(variable.clone(), VariableInfo {
            valor: elem,
            tipo_declarado: None,
            mutable: false,   // variable de iteración es inmutable (dato)
            es_const: false,
        });
        let r = self.ejecutar_sentencias_bloque_ciclo(&cuerpo);
        self.ambitos.pop();
        match r {
            Ok(()) => {}
            Err(e) if matches!(e.causa, CausaError::SenalRomper) => break 'ciclo,
            Err(e) if matches!(e.causa, CausaError::SenalContinuar) => continue 'ciclo,
            Err(e) => return Err(e),
        }
    }
}
```

Para rangos, `Expresion::Rango` se evalúa antes de pasar al match:
en `evaluar_con`, añadir el arm `Expresion::Rango`:
```rust
Expresion::Rango { inicio, fin, inclusivo, linea, columna } => {
    let v_inicio = evaluar_con(inicio, env)?;
    let v_fin = evaluar_con(fin, env)?;
    let i = match v_inicio { Valor::Entero(n) => n,
        _ => return Err(error(CausaError::RangoTipoInvalido { obtenido: v_inicio.nombre_tipo() }, *linea, *columna))
    };
    let f = match v_fin { Valor::Entero(n) => n,
        _ => return Err(error(CausaError::RangoTipoInvalido { obtenido: v_fin.nombre_tipo() }, *linea, *columna))
    };
    // Construir un Valor::Lista con los enteros del rango
    let mut v = Vec::new();
    if *inclusivo {
        let mut n = i;
        while n <= f { v.push(Valor::Entero(n)); n += 1; }
    } else {
        let mut n = i;
        while n < f { v.push(Valor::Entero(n)); n += 1; }
    }
    Ok(Valor::Lista(v))
}
```

También añadir `ubicacion()` para `Expresion::Rango`.

**E. Nuevas variantes en `CausaError` para iteración y E/S:**
```rust
/// Iteración sobre valor que no es colección ni rango (ADR-0012 §4.4).
IteracionSobreNoColeccion { obtenido: &'static str },
/// Extremos del rango no son enteros (ADR-0012 §4.4).
RangoTipoInvalido { obtenido: &'static str },
/// Modificación de colección durante iteración (ADR-0012 §4.2).
ModificacionDuranteIteracion { variable: String },
/// Entrada inválida desde stdin (ADR-0013 §3.2).
EntradaInvalida { esperado: String, obtenido: String },
/// Archivo no encontrado (ADR-0013 §3.7).
ArchivoNoEncontrado { ruta: String },
/// Permisos denegados (ADR-0013 §3.7).
PermisosDenegados { ruta: String },
/// Error genérico de archivo (ADR-0013 §3.7).
ErrorDeArchivo { ruta: String, detalle: String },
/// Operación sobre archivo ya cerrado (ADR-0013 §3.7).
ArchivoYaCerrado,
/// Índice de texto fuera de rango (ADR-0013 §3.7).
IndiceTextoFueraDeRango { indice: i64, longitud: usize },
/// Conversión de texto a número fallida (ADR-0013 §3.7).
ConversionTextoInvalida { valor: String, esperado: String },
/// Separador vacío en `dividir` (ADR-0013 §3.7).
SeparadorVacio,
/// Nombre reservado de función integrada (ADR-0013 §3.1).
NombreReservado { nombre: String },
```

Añadir sus ramas en `Display`.

**F. Añadir `Valor::Archivo`:**
Ver ítem 7.5.

**G. Infraestructura de builtins:**
Añadir una función en el evaluador:
```rust
fn llamar_builtin(
    nombre: &str,
    argumentos: Vec<Valor>,
    linea: usize,
    columna: usize,
) -> Option<Result<Valor, ErrorEjecucion>>
```
Devuelve `None` si el nombre no es un builtin (para distinguir de función de usuario).
Devuelve `Some(Ok(v))` o `Some(Err(e))` si lo es.

En la rama `Expresion::Llamada` del evaluador, antes de buscar en `env.funciones`,
comprobar si el nombre es builtin:
```rust
// Evaluar argumentos primero (igual que ahora)
let vals = ...;
// Intentar builtin
if let Some(resultado) = llamar_builtin(nombre, vals, *linea, *columna) {
    return resultado;
}
// Buscar en funciones de usuario
let fdef = env.funciones.get(nombre).cloned().ok_or_else(...)?;
```

**H. Reservar nombres de builtins al definir funciones:**
En la rama `Sentencia::Funcion` de `ejecutar_sentencias()`, antes de insertar:
```rust
const NOMBRES_RESERVADOS: &[&str] = &[
    "leer", "leer_entero", "leer_decimal", "leer_real",
    "longitud", "contiene", "empieza_con", "termina_con", "es_vacio",
    "mayusculas", "minusculas", "recortar", "reemplazar", "subcadena",
    "dividir", "unir", "a_entero", "a_decimal", "a_real", "texto",
    "abrir", "leer_linea", "leer_todo", "escribir", "escribir_linea",
    "cerrar", "fin_de_archivo",
];
if NOMBRES_RESERVADOS.contains(&nombre.as_str()) {
    return Err(error(CausaError::NombreReservado { nombre: nombre.clone() }, linea, columna));
}
```

**Archivos:** `src/evaluador.rs`

**Verificar:**
```
cargo test 2>&1   # todas las pruebas anteriores siguen pasando
```

---

### Ítem 7.5 — Añadir `Valor::Archivo` en `valor.rs`

**Qué hacer:**
Importar:
```rust
use std::sync::{Arc, Mutex};
use std::io::{BufReader, BufRead, Write};
use std::fs::File;
```

Definir el estado interno:
```rust
/// Estado interno de un archivo abierto (ADR-0013 §3.5).
/// Opaco: el código BASILISCO no accede a sus campos.
#[derive(Debug)]
pub struct ArchivoInterno {
    /// Ruta del archivo (para mensajes de error).
    pub ruta: String,
    /// Lector con buffer para lectura línea a línea. `None` si está en modo escritura.
    pub lector: Option<BufReader<File>>,
    /// Escritor para modo "escribir" / "agregar". `None` si está en modo lectura.
    pub escritor: Option<File>,
    /// Modo de apertura: "leer", "escribir" o "agregar".
    pub modo: String,
    /// Indica si el archivo fue cerrado.
    pub cerrado: bool,
}
```

Añadir la variante al enum `Valor`:
```rust
/// Archivo abierto. Usa `Arc<Mutex<...>>` para que `Valor` sea `Clone` (ADR-0013 §3.6).
Archivo(Arc<Mutex<ArchivoInterno>>),
```

Dado que `Arc<Mutex<ArchivoInterno>>` no implementa `PartialEq` automáticamente,
el derive `#[derive(PartialEq)]` en `Valor` dejará de compilar. Soluciones:
- Opción A: Implementar `PartialEq` manualmente para `Valor`, tratando
  `Valor::Archivo(_) == Valor::Archivo(_)` como `false` (dos archivos distintos
  no son iguales).
- **Opción elegida (A):** implementar `PartialEq` a mano; es la que causa menos
  cambio en los tests existentes que usan `assert_eq!(valor, Valor::Entero(...))`.

Actualizar todos los `match` sobre `Valor`:

**`nombre_tipo()`:**
```rust
Valor::Archivo(_) => "archivo",
```

**`Display`:**
```rust
Valor::Archivo(_) => write!(f, "<archivo>"),
```

**`es_cero()`:**
```rust
Valor::Archivo(_) => false,
```

**`a_real()`:**
```rust
Valor::Archivo(_) => f64::NAN,
```

**Operaciones aritméticas:** el arm `_ => Err(TiposIncompatibles)` captura `Archivo`.

**Archivos:** `src/valor.rs`

**Verificar:**
```
cargo test 2>&1   # todas las pruebas anteriores siguen pasando
```

---

### Ítem 7.6 — Implementar todas las funciones integradas en `evaluador.rs`

**Qué hacer:**
Implementar la función `llamar_builtin()` definida en el ítem 7.4.G. Cada builtin
verifica la aridad y los tipos de los argumentos, retornando errores apropiados.

A continuación se detalla cada grupo:

**Entrada estándar (ADR-0013 §3.2):**

```rust
"leer" => {
    // aridad 0; lee línea de stdin
    if args.len() != 0 { return Some(Err(aridad_error("leer", 0, args.len(), l, c))); }
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();
    let s = buf.trim_end_matches(['\n', '\r']).to_string();
    Some(Ok(Valor::Texto(s)))
}
"leer_entero" => {
    if args.len() != 0 { ... }
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();
    let s = buf.trim();
    match s.parse::<i64>() {
        Ok(n) => Some(Ok(Valor::Entero(n))),
        Err(_) => Some(Err(error(CausaError::EntradaInvalida {
            esperado: "entero".into(), obtenido: s.to_string()
        }, l, c))),
    }
}
// análogos para leer_decimal y leer_real
```

**`mostrar_sin_salto` (ADR-0013 §3.3):** no es builtin sino sentencia; se ejecuta en
la rama `Sentencia::MostrarSinSalto`:
```rust
Sentencia::MostrarSinSalto { valor, .. } => {
    let v = evaluar_con(&valor, self)?;
    print!("{v}");
    // flush inmediato para que el prompt aparezca antes de leer
    use std::io::Write;
    let _ = std::io::stdout().flush();
}
```
Añadir en ambas ramas (`ejecutar_sentencias` y `ejecutar_funcion_sentencias`).

**Operaciones de texto (ADR-0013 §3.4):**

```rust
"longitud" => {
    // 1 arg texto
    let t = tomar_texto(&args, 0, "longitud", l, c)?;
    Some(Ok(Valor::Entero(t.chars().count() as i64)))
}
"contiene" => {
    let t = tomar_texto(&args, 0, "contiene", l, c)?;
    let sub = tomar_texto(&args, 1, "contiene", l, c)?;
    Some(Ok(Valor::Logico(t.contains(sub.as_str()))))
}
"empieza_con" => {
    let t = tomar_texto(&args, 0, "empieza_con", l, c)?;
    let pre = tomar_texto(&args, 1, "empieza_con", l, c)?;
    Some(Ok(Valor::Logico(t.starts_with(pre.as_str()))))
}
"termina_con" => { ... t.ends_with(...) ... }
"es_vacio" => {
    let t = tomar_texto(&args, 0, "es_vacio", l, c)?;
    Some(Ok(Valor::Logico(t.is_empty())))
}
"mayusculas" => {
    let t = tomar_texto(&args, 0, "mayusculas", l, c)?;
    Some(Ok(Valor::Texto(t.to_uppercase())))
}
"minusculas" => { ... t.to_lowercase() ... }
"recortar" => { ... t.trim().to_string() ... }
"reemplazar" => {
    let t = tomar_texto(&args, 0, "reemplazar", l, c)?;
    let buscar = tomar_texto(&args, 1, "reemplazar", l, c)?;
    let nuevo = tomar_texto(&args, 2, "reemplazar", l, c)?;
    Some(Ok(Valor::Texto(t.replace(buscar.as_str(), nuevo.as_str()))))
}
"subcadena" => {
    let t = tomar_texto(&args, 0, "subcadena", l, c)?;
    let inicio = tomar_entero(&args, 1, "subcadena", l, c)?;
    let fin = tomar_entero(&args, 2, "subcadena", l, c)?;
    let chars: Vec<char> = t.chars().collect();
    let lon = chars.len();
    if inicio < 0 || fin as usize > lon || inicio > fin {
        return Some(Err(error(CausaError::IndiceTextoFueraDeRango {
            indice: if inicio < 0 { inicio } else { fin },
            longitud: lon
        }, l, c)));
    }
    let s: String = chars[inicio as usize..fin as usize].iter().collect();
    Some(Ok(Valor::Texto(s)))
}
"dividir" => {
    let t = tomar_texto(&args, 0, "dividir", l, c)?;
    let sep = tomar_texto(&args, 1, "dividir", l, c)?;
    if sep.is_empty() {
        return Some(Err(error(CausaError::SeparadorVacio, l, c)));
    }
    let partes: Vec<Valor> = t.split(sep.as_str())
        .map(|p| Valor::Texto(p.to_string()))
        .collect();
    Some(Ok(Valor::Lista(partes)))
}
"unir" => {
    let lista = match args.get(0) {
        Some(Valor::Lista(v)) => v.clone(),
        Some(Valor::Arreglo(v)) => v.clone(),
        _ => return Some(Err(tipo_error("unir", 0, "lista<texto>", args.get(0), l, c))),
    };
    let sep = tomar_texto(&args, 1, "unir", l, c)?;
    let partes: Vec<String> = lista.iter().map(|v| v.to_string()).collect();
    Some(Ok(Valor::Texto(partes.join(sep.as_str()))))
}
"a_entero" => {
    let t = tomar_texto(&args, 0, "a_entero", l, c)?;
    match t.parse::<i64>() {
        Ok(n) => Some(Ok(Valor::Entero(n))),
        Err(_) => Some(Err(error(CausaError::ConversionTextoInvalida {
            valor: t, esperado: "entero".into()
        }, l, c))),
    }
}
"a_decimal" => { ... parse::<f64>() ... convertir a Decimal ... }
"a_real" => { ... parse::<f64>() ... }
"texto" => {
    // 1 arg de cualquier tipo
    match args.into_iter().next() {
        Some(v) => Some(Ok(Valor::Texto(v.to_string()))),
        None => Some(Err(aridad_error("texto", 1, 0, l, c))),
    }
}
```

**Archivos (ADR-0013 §3.5):**
```rust
"abrir" => {
    let ruta = tomar_texto(&args, 0, "abrir", l, c)?;
    let modo = tomar_texto(&args, 1, "abrir", l, c)?;
    match modo.as_str() {
        "leer" => {
            match std::fs::File::open(&ruta) {
                Ok(f) => {
                    let interno = ArchivoInterno {
                        ruta: ruta.clone(),
                        lector: Some(BufReader::new(f)),
                        escritor: None,
                        modo: "leer".into(),
                        cerrado: false,
                    };
                    Some(Ok(Valor::Archivo(Arc::new(Mutex::new(interno)))))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound =>
                    Some(Err(error(CausaError::ArchivoNoEncontrado { ruta }, l, c))),
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied =>
                    Some(Err(error(CausaError::PermisosDenegados { ruta }, l, c))),
                Err(e) => Some(Err(error(CausaError::ErrorDeArchivo {
                    ruta, detalle: e.to_string()
                }, l, c))),
            }
        }
        "escribir" => { ... OpenOptions::new().write(true).create(true).truncate(true) ... }
        "agregar"  => { ... OpenOptions::new().append(true).create(true) ... }
        _ => Some(Err(error(CausaError::ErrorDeArchivo {
            ruta, detalle: format!("modo desconocido: {modo}; use \"leer\", \"escribir\" o \"agregar\"")
        }, l, c))),
    }
}
"leer_linea" => {
    let arc = tomar_archivo(&args, 0, "leer_linea", l, c)?;
    let mut interno = arc.lock().unwrap();
    if interno.cerrado { return Some(Err(error(CausaError::ArchivoYaCerrado, l, c))); }
    match &mut interno.lector {
        Some(lector) => {
            let mut linea = String::new();
            match lector.read_line(&mut linea) {
                Ok(0) => Some(Ok(Valor::Texto(String::new()))), // EOF
                Ok(_) => {
                    let s = linea.trim_end_matches(['\n', '\r']).to_string();
                    Some(Ok(Valor::Texto(s)))
                }
                Err(e) => Some(Err(error(CausaError::ErrorDeArchivo {
                    ruta: interno.ruta.clone(), detalle: e.to_string()
                }, l, c))),
            }
        }
        None => Some(Err(error(CausaError::ErrorDeArchivo {
            ruta: interno.ruta.clone(), detalle: "el archivo no está en modo lectura".into()
        }, l, c))),
    }
}
"leer_todo" => { ... lector.read_to_string(&mut contenido) ... }
"escribir" => {
    let arc = tomar_archivo(&args, 0, "escribir", l, c)?;
    let texto = tomar_texto(&args, 1, "escribir", l, c)?;
    let mut interno = arc.lock().unwrap();
    if interno.cerrado { return Some(Err(error(CausaError::ArchivoYaCerrado, l, c))); }
    match &mut interno.escritor {
        Some(f) => {
            f.write_all(texto.as_bytes()).map_err(|e| ...)?;
            Some(Ok(Valor::Nulo))
        }
        None => ...
    }
}
"escribir_linea" => { ... escribir + "\n" ... retorna Valor::Nulo }
"cerrar" => {
    let arc = tomar_archivo(&args, 0, "cerrar", l, c)?;
    let mut interno = arc.lock().unwrap();
    interno.cerrado = true;
    interno.lector = None;
    interno.escritor = None;  // drop cierra el archivo
    Some(Ok(Valor::Nulo))
}
"fin_de_archivo" => {
    let arc = tomar_archivo(&args, 0, "fin_de_archivo", l, c)?;
    let mut interno = arc.lock().unwrap();
    if interno.cerrado { return Some(Err(error(CausaError::ArchivoYaCerrado, l, c))); }
    match &mut interno.lector {
        Some(lector) => {
            // EOF: intentar leer 1 byte; si devuelve 0 bytes, es EOF.
            // Se usa fill_buf() para peek sin consumir.
            match lector.fill_buf() {
                Ok(buf) => Some(Ok(Valor::Logico(buf.is_empty()))),
                Err(e) => Some(Err(error(CausaError::ErrorDeArchivo {
                    ruta: interno.ruta.clone(), detalle: e.to_string()
                }, l, c))),
            }
        }
        None => Some(Err(error(CausaError::ErrorDeArchivo {
            ruta: interno.ruta.clone(), detalle: "solo disponible en modo lectura".into()
        }, l, c))),
    }
}
```

Funciones auxiliares privadas usadas dentro de `llamar_builtin`:
```rust
fn tomar_texto(args: &[Valor], idx: usize, nombre: &str, l: usize, c: usize) -> Result<String, ...>
fn tomar_entero(args: &[Valor], idx: usize, nombre: &str, l: usize, c: usize) -> Result<i64, ...>
fn tomar_archivo(args: &[Valor], idx: usize, nombre: &str, l: usize, c: usize) -> Result<Arc<Mutex<ArchivoInterno>>, ...>
fn aridad_error(nombre: &str, esperado: usize, obtenido: usize, l: usize, c: usize) -> ErrorEjecucion
```

Importar `ArchivoInterno` de `crate::valor`.

**Archivos:** `src/evaluador.rs`

**Verificar:**
```
cargo test 2>&1   # todas las pruebas anteriores siguen pasando
```

---

### Ítem 7.7 — Crear suites de pruebas de iteración y E/S

**Qué hacer:**
Crear tres archivos de prueba. Patrón auxiliar compartido igual a
`conformidad_funciones.rs` (usar `Entorno::nueva().ejecutar_programa(...)`).

**`tests/conformidad_iteracion.rs`** — criterios de ADR-0012 §7:
- `para cada` sobre `lista<entero>` de 3 elementos → 3 iteraciones.
- `para cada` sobre `arreglo<texto, 2>` → 2 iteraciones.
- Lista vacía → 0 iteraciones, sin error.
- Variable de iteración inmutable: asignación dentro del bloque → error.
- Colección no válida (`para cada x en 5`) → error `IteracionSobreNoColeccion`.
- Rango `0..5` → itera 0,1,2,3,4.
- Rango `1..=3` → itera 1,2,3.
- Rango vacío `5..5` → 0 iteraciones.
- Rango negativo `-2..=2` → itera -2,-1,0,1,2.
- Rango con tipo no entero → error `RangoTipoInvalido`.
- `romper` en `para cada` termina el ciclo.
- `continuar` en `para cada` salta al siguiente.
- `romper` en `mientras` termina el ciclo.
- `continuar` en `mientras` reevalúa la condición.
- `romper` fuera de ciclo → error sintáctico (verificar con `Parser::parsear_programa`).
- `continuar` fuera de ciclo → error sintáctico.
- `romper` dentro de `si` anidado en `para cada` → termina el `para cada`.

**`tests/conformidad_texto_ops.rs`** — criterios de ADR-0013 §7 (operaciones de texto):
- `longitud("hola")` → 4; `longitud("ñ")` → 1.
- `contiene("hola mundo", "mundo")` → `verdadero`.
- `empieza_con("hola", "ho")` → `verdadero`.
- `termina_con("hola", "la")` → `verdadero`.
- `mayusculas("hola")` → `"HOLA"`.
- `minusculas("HOLA")` → `"hola"`.
- `recortar("  hola  ")` → `"hola"`.
- `reemplazar("hola mundo", "mundo", "BASILISCO")` → `"hola BASILISCO"`.
- `subcadena("hola", 1, 3)` → `"ol"`.
- `subcadena("hola", 0, 10)` → error `IndiceTextoFueraDeRango`.
- `dividir("a,b,c", ",")` → lista `[a, b, c]`.
- `unir(["a","b","c"], "-")` → `"a-b-c"`.
- `a_entero("42")` → 42; `a_entero("abc")` → error `ConversionTextoInvalida`.
- `texto(42)` → `"42"`; `texto(verdadero)` → `"verdadero"`; `texto(nulo)` → `"nulo"`.
- Declarar `funcion leer() { }` → error `NombreReservado`.

**`tests/conformidad_io.rs`** — criterios de ADR-0013 §7 (archivos):
- `abrir("archivo_test.txt", "escribir")` → crear; `escribir_linea(f, "hola")`; `cerrar(f)`.
- `abrir("archivo_test.txt", "leer")` + `leer_linea(f)` → `"hola"`; después `fin_de_archivo(f)` → `verdadero`; `cerrar(f)`.
- `abrir("no_existe.txt", "leer")` → error `ArchivoNoEncontrado`.
- Modo inválido → error `ErrorDeArchivo`.
- `cerrar(f)`; luego `leer_linea(f)` → error `ArchivoYaCerrado`.
- `leer_todo` sobre archivo con contenido conocido → texto completo.
- Prueba `leer()` stdin: marcar `#[ignore]` con comentario `// requiere stdin interactivo; omitir en CI`.
- `leer_entero()` con entrada `"abc"` desde proceso separado (o `#[ignore]`) → error `EntradaInvalida`.

Usar `std::env::temp_dir()` o un path fijo en el directorio actual para los
archivos temporales de prueba; limpiarlos al final con `std::fs::remove_file`.

**Archivos:**
- `tests/conformidad_iteracion.rs`
- `tests/conformidad_texto_ops.rs`
- `tests/conformidad_io.rs`

**Verificar:**
```
cargo test 2>&1   # 268 pruebas anteriores + nuevas de I7 pasan
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

---

## Incremento 8 — Manejo de errores (ADR-0014)

> Dependencia: los Incrementos 6 y 7 deben estar completados y en verde.

### Ítem 8.1 — Nuevos tokens `Intentar` y `Rescatar` en `token.rs`

**Qué hacer:**
Añadir al enum `TipoToken`:
```rust
/// Palabra clave `intentar` (ADR-0014 §3.1).
Intentar,
/// Palabra clave `rescatar` (ADR-0014 §3.1).
Rescatar,
```

**Archivos:** `src/token.rs`

**Verificar:**
```
cargo build   # compila limpio
```

---

### Ítem 8.2 — Reconocer `intentar` y `rescatar` en `lexer.rs`

**Qué hacer:**
En `leer_identificador()`, añadir:
```rust
"intentar" => TipoToken::Intentar,
"rescatar" => TipoToken::Rescatar,
```

**Archivos:** `src/lexer.rs`

**Verificar:**
```
cargo test --lib
```

---

### Ítem 8.3 — AST e implementación del parser para `intentar`/`rescatar` en `parser.rs`

**Qué hacer:**

**A. Nueva variante en `Sentencia`:**
```rust
/// `intentar { ... } rescatar [IDENT] { ... }` (ADR-0014 §3.1).
IntentarRescatar {
    cuerpo_intentar: Vec<Sentencia>,
    variable_error: Option<String>,   // None si `rescatar { ... }` sin nombre
    cuerpo_rescatar: Vec<Sentencia>,
    linea: usize,
    columna: usize,
},
```

**B. Nuevo método `sentencia_intentar_rescatar()`:**
```
consume 'intentar'
exigir llave en la misma línea (ADR-0014 §3.1; misma regla que 'si')
parsea cuerpo_intentar = bloque()
omitir_finales_de_linea()  (rescatar puede ir en línea siguiente, como 'sino')
exigir TipoToken::Rescatar (si no hay → error sintáctico "'intentar' sin 'rescatar'")
variable_error = if next token is Identificador → leer nombre; avanzar; Some(nombre)
                 else → None
exigir llave en la misma línea que 'rescatar'
parsea cuerpo_rescatar = bloque()
retornar Sentencia::IntentarRescatar { cuerpo_intentar, variable_error, cuerpo_rescatar, linea, columna }
```

ADR-0014 §3.7: `intentar { ... }` sin `rescatar` es error sintáctico.

**C. Añadir en `sentencia()`:**
```rust
} else if matches!(self.actual(), Some(TipoToken::Intentar)) {
    let st = self.sentencia_intentar_rescatar()?;
    self.terminar_sentencia()?;
    return Ok(st);
} else if matches!(self.actual(), Some(TipoToken::Rescatar)) {
    return Err(self.error("'rescatar' sin un 'intentar' correspondiente"));
}
```

**Archivos:** `src/parser.rs`

**Verificar:**
```
cargo test 2>&1   # todas las pruebas anteriores siguen pasando
```

---

### Ítem 8.4 — Implementar `IntentarRescatar` en `evaluador.rs`

**Qué hacer:**

**A. Añadir la rama en `ejecutar_sentencias()` y `ejecutar_funcion_sentencias()`:**

```rust
Sentencia::IntentarRescatar {
    cuerpo_intentar, variable_error, cuerpo_rescatar, ..
} => {
    let resultado = self.ejecutar_bloque(&cuerpo_intentar);
    match resultado {
        Ok(()) => {
            // Sin error: rescatar no se ejecuta
        }
        Err(e) => {
            // Verificar que NO es una señal de ciclo ni un retorno
            if matches!(e.causa,
                CausaError::SenalRomper | CausaError::SenalContinuar
            ) {
                // Las señales de ciclo se propagan sin capturar
                return Err(e);
            }
            // Capturar el error: ejecutar cuerpo_rescatar
            self.ambitos.push(HashMap::new());
            if let Some(nombre) = &variable_error {
                // La variable de error es de tipo texto y contiene e.mensaje
                self.insertar_variable(nombre.clone(), VariableInfo {
                    valor: Valor::Texto(e.mensaje.clone()),
                    tipo_declarado: Some(TipoAnotacion::Texto),
                    mutable: false,
                    es_const: false,
                });
            }
            let r = self.ejecutar_sentencias(&cuerpo_rescatar);
            self.ambitos.pop();
            // Si el cuerpo rescatar también falla, el error sube al intentar exterior
            r?;  // propagar errores del cuerpo_rescatar
        }
    }
}
```

Para la versión dentro de función (`ejecutar_funcion_sentencias`), el mismo patrón
pero usando `ejecutar_bloque_funcion` para el `cuerpo_intentar` y manejando el
retorno correctamente:
```rust
// Si el cuerpo_intentar retorna (éxito), propagar el retorno
// Si el cuerpo_intentar falla con señal de ciclo, propagar
// Si el cuerpo_intentar falla con ErrorEjecucion ordinario, ejecutar cuerpo_rescatar
```

Detalle importante: `ejecutar_bloque_funcion` retorna `Result<Valor, ErrorEjecucion>`.
Si retorna `Ok(v)`, es un `retornar` exitoso: debe propagarse (no ejecutar `rescatar`).
Si retorna `Err(e)` donde la causa NO es `RetornoAusente`, es un error a capturar.
`RetornoAusente` indica que no se ejecutó `retornar`; en ese caso el bloque terminó
sin error, así que tampoco se ejecuta `rescatar`.

Lógica:
```rust
match self.ejecutar_bloque_funcion(&cuerpo_intentar, nombre_funcion) {
    Ok(v) => {
        // retornar ejecutado: propagar directamente
        return Ok(v);
    }
    Err(e) if matches!(e.causa, CausaError::RetornoAusente { .. }) => {
        // Bloque terminó normalmente (sin retornar): no capturar
    }
    Err(e) if matches!(e.causa, CausaError::SenalRomper | CausaError::SenalContinuar) => {
        return Err(e);  // señales de ciclo no se capturan
    }
    Err(e) => {
        // Error real: ejecutar cuerpo_rescatar
        self.ambitos.push(HashMap::new());
        if let Some(nombre) = &variable_error {
            self.insertar_variable(nombre.clone(), VariableInfo {
                valor: Valor::Texto(e.mensaje.clone()),
                tipo_declarado: Some(TipoAnotacion::Texto),
                mutable: false,
                es_const: false,
            });
        }
        let r = self.ejecutar_bloque_funcion(&cuerpo_rescatar, nombre_funcion);
        self.ambitos.pop();
        match r {
            Ok(v) => return Ok(v),  // retornar en rescatar
            Err(e2) if matches!(e2.causa, CausaError::RetornoAusente { .. }) => {
                // rescatar terminó sin retornar: continuar
            }
            Err(e2) => return Err(e2),  // error en rescatar: propagar
        }
    }
}
```

**B. Actualizar `ubicacion()` para `Expresion::Rango`:**
Ya debería estar hecho en el Incremento 7, pero confirmar que el compilador no advierte.

**Archivos:** `src/evaluador.rs`

**Verificar:**
```
cargo test 2>&1   # todas las pruebas anteriores siguen pasando
```

---

### Ítem 8.5 — Crear `tests/conformidad_errores.rs`

**Qué hacer:**
Crear la suite de pruebas del ADR-0014 §7:

**Captura básica:**
```rust
// División por cero capturada
let r = run_program(r#"
    var resultado: texto = ""
    intentar {
        var x: entero = 10 / 0
        resultado = "sin error"
    } rescatar e {
        resultado = e
    }
    resultado
"#);
assert_eq!(r, Ok(Some(Valor::Texto("división entre cero".into()))));
```

- Error en `intentar` → `rescatar` se ejecuta con el mensaje del error.
- Sin error en `intentar` → `rescatar` no se ejecuta.
- Error en `rescatar` sube al `intentar` exterior o aborta.
- `rescatar` sin nombre de variable (`rescatar { ... }`) → válido.

**Alcance:**
- Variable declarada en `intentar` no visible en `rescatar` → error `VariableNoDefinida`.
- Variable declarada antes de `intentar` visible en `rescatar` → ok.

**Retorno:**
- `retornar` dentro de `intentar` retorna de la función; `rescatar` no se ejecuta.
- `retornar` dentro de `rescatar` retorna de la función.

**Anidamiento:**
- `intentar` dentro de `intentar`: el error lo captura el más cercano.
- Error en `rescatar` interior sube al `intentar` exterior.

**`romper`/`continuar` no capturados:**
- `romper` dentro de `intentar` dentro de `para cada` → rompe el ciclo, no activa `rescatar`.
- `continuar` dentro de `intentar` dentro de `para cada` → salta iteración, no activa `rescatar`.

**Sintáctico:**
- `intentar { }` sin `rescatar` → error sintáctico.
- `rescatar { }` sin `intentar` → error sintáctico.

**Regresión:**
- Las funciones de todas las suites anteriores siguen pasando (verificado con `cargo test`).

**Archivos:** `tests/conformidad_errores.rs`

**Verificar:**
```
cargo test --test conformidad_errores 2>&1
cargo test 2>&1   # TODO en verde
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

---

## Verificación final — estado completo

Ejecutar en orden tras completar el Incremento 8:

```bash
# 1. Formato
cargo fmt --check

# 2. Linting (sin #[allow(...)] añadidos)
cargo clippy --all-targets -- -D warnings

# 3. Pruebas completas
cargo test 2>&1
# Resultado esperado: todas las pruebas (268 originales + nuevas de I6, I7, I8) en verde, 0 fallos.

# 4. Confirmar que las 268 originales siguen en verde
cargo test --test conformidad 2>&1
cargo test --test conformidad_arreglos_listas 2>&1
cargo test --test conformidad_control 2>&1
cargo test --test conformidad_funciones 2>&1
cargo test --test conformidad_texto 2>&1
cargo test --test variables 2>&1
cargo test --test integracion 2>&1
cargo test --test control_flujo 2>&1
cargo test --test cli 2>&1
```

---

## Verificación de nombres reservados (ADR-0013 §5)

Resultado del grep sobre `prototipo/tests/**`:

Los resultados encontrados son:
- `cli.rs:96`: `"no se pudo leer"` — es texto en un string, no un identificador. ✓ seguro.
- `conformidad_control.rs:220`: comentario `# Efecto observable: leer una variable...` — es texto
  en un comentario dentro de un string de BASILISCO, no un identificador Rust. ✓ seguro.

**Conclusión:** ningún nombre reservado de ADR-0013 §5 aparece como identificador
en los tests existentes. Los dos matches son falsos positivos (texto literal y comentarios
dentro de cadenas de prueba).

---

## Resumen de archivos modificados por incremento

| Archivo | I6 | I7 | I8 |
|---------|----|----|-----|
| `src/token.rs` | Flecha, Nulo | Para, Cada, En, Romper, Continuar, RangoDos, RangoDosCerrado, MostrarSinSalto | Intentar, Rescatar |
| `src/lexer.rs` | `->`, `nulo` | `para`, `cada`, `en`, `romper`, `continuar`, `mostrar_sin_salto`, `..`, `..=` | `intentar`, `rescatar` |
| `src/parser.rs` | TipoAnotacion::Nulo, literal nulo, params tipados, `->` retorno | ParaCada, Romper, Continuar, MostrarSinSalto, Rango, en_ciclo | IntentarRescatar |
| `src/valor.rs` | Valor::Nulo | Valor::Archivo, ArchivoInterno, PartialEq manual | — |
| `src/evaluador.rs` | TipoParametroIncompatible, RetornoIncompatibleConAnotacion, FuncionDef ampliado, verificación en llamadas/retorno | SenalRomper, SenalContinuar, nuevos CausaError, builtins, ParaCada, MostrarSinSalto | IntentarRescatar |
| `tests/conformidad_tipos.rs` | ✦ nuevo | — | — |
| `tests/conformidad_iteracion.rs` | — | ✦ nuevo | — |
| `tests/conformidad_texto_ops.rs` | — | ✦ nuevo | — |
| `tests/conformidad_io.rs` | — | ✦ nuevo | — |
| `tests/conformidad_errores.rs` | — | — | ✦ nuevo |
