# Análisis D3 y D4 — División y módulo (ADR-0005)

> **RESUELTO (2026-10-20):** D3 y D4 fueron decididas en el ADR-0005 consolidado y aprobado
> (`ADR-0005-sistema-numerico.md`): división por cero = error en los tres tipos; `%` de enteros =
> resto truncado con signo del dividendo, `i64::MIN % -1` = 0; euclidiano diferido a biblioteca.
> Este documento queda como **material histórico de análisis**, no normativo.

- Documento: informe de análisis **enlazado** desde `ADR-0005-sistema-numerico.md`
- Estado: **Propuesta** (D3 y D4 siguen **PENDIENTES** de aprobación)
- Fecha: 2026-10-20
- Alcance: profundizar D4 (semántica del módulo) y D3 (división por cero en `real`), en ese orden
- Fuente de evidencia: ejecución real en `rustc -O` (temporal en `/tmp`, fuera del repo; no modifica el proyecto)
- Restricción respetada: sin cambios de código, gramática ni pruebas ejecutables

---

# PRIMERA PARTE — D4: semántica del módulo

## 1. Marco: el resto depende del redondeo de la división

Dado entero `a` y divisor `b ≠ 0`, la división entera produce un cociente `q` y el resto `r`
satisface la **identidad de división**:

```
a = q·b + r
```

La diferencia entre convenios **no** está en la identidad (todos la cumplen), sino en **cómo se
elige `q`**, y de ahí se deriva el **signo de `r`**. Tres convenios formales:

### C1. División truncada (hacia cero) — C, C++, Rust, Java, Go, JS
- `q = trunc(a/b)` (redondeo hacia cero).
- `r = a − q·b`.
- **Signo de `r` = signo de `a`** (el dividendo). Es el "remainder" de C.

### C2. División hacia menos infinito (floored) — Python, Ruby, Dart, Swift (para enteros)
- `q = floor(a/b)` (redondeo hacia −∞).
- `m = a − q·b` (se llama "módulo").
- **Signo de `m` = signo de `b`** (el divisor).

### C3. Módulo euclídeo
- Se define para que el resto sea **siempre no negativo**.
- `r ∈ [0, |b|)`.
- Coinide con C2 cuando `b > 0`; **difiere** de C2 cuando `b < 0`.

---

## 2. Tabla de resultados esperados (verificada por ejecución en i64/Rust)

Donde `trunc` es C1 (Rust/C), `floor` es C2 (Python) y `eucl` es C3.

| a | b | C1 q (`a/b`) | C1 r = `a%b` | C2 q (`a//b`) | C2 m = `a%b` | C3 r | Identidad en C1 |
|---|---|---|---|---|---|---|---|
| 7 | 3 | 2 | 1 | 2 | 1 | 1 | ✔ |
| −7 | 3 | −2 | **−1** | −3 | **2** | 2 | ✔ |
| 7 | −3 | −2 | **1** | −3 | **−2** | 1 | ✔ |
| −7 | −3 | 2 | **−1** | 2 | **−1** | 2 | ✔ |
| 6 | 3 | 2 | 0 | 2 | 0 | 0 | ✔ (exacta) |
| −6 | 3 | −2 | 0 | −2 | 0 | 0 | ✔ (exacta) |
| −6 | −3 | 2 | 0 | 2 | 0 | 0 | ✔ (exacta) |
| 7 | 0 | — | **error** | — | **error** | **error** | — |
| i64::MIN | −1 | — | **error (desborde)** | — | **error** | **error** | — |

**Evidencia [VERIFICADO]** (ejecutado, Rust i64):
```
   7 %   3 =   1  | div_trunc =    2  | identidad ok = true
  -7 %   3 =  -1  | div_trunc =   -2  | identidad ok = true
   7 %  -3 =   1  | div_trunc =   -2  | identidad ok = true
  -7 %  -3 =  -1  | div_trunc =    2  | identidad ok = true
   6 %   3 =   0  | identidad ok = true
```
- `7 % 0` → panic ("remainder with a divisor of zero").
- `i64::MIN % -1` → panic ("remainder with overflow"). `i64::MIN = -9223372036854775808`.

Nota: la fila `7 % −3` muestra `+1` en C1 y `−2` en C2: mismo valor absoluto distinto signo. Es el
caso que más confunde y por eso debe fijarse con prueba normativa explícita.

---

## 3. Invariantes matemáticos

| Invariante | C1 truncado | C2 floored | C3 euclídeo |
|---|---|---|---|
| Identidad `a = q·b + r` | ✔ | ✔ | ✔ |
| Signo de `r` ligado a `a` (dividendo) | ✔ | ✖ | ✖ |
| Signo de `m` ligado a `b` (divisor) | ✖ | ✔ | ✖ |
| `r ≥ 0` siempre (no negativo) | ✖ | ✖ | ✔ |
| `0 ≤ r < \|b\|` (rango acotado) | ✖ (`|r|<\|b\|` sí, pero con signo) | ✖ | ✔ |
| Conmutatividad del signo (mismo `r` con `b` y `−b`) | ✖ | ✖ | ✔ |

Lectura:
- **Identidad de división y resto**: se cumple en los tres **siempre que `q` y `r` provengan del
  mismo convenio**. Mezclar `q` de un convenio con `r` de otro rompe la identidad.
- **Signo del resto**: C1 lo ata al dividendo; C2 al divisor; C3 no lo ata a ninguno (es no
  negativo).
- **No negatividad euclidiana**: es la propiedad exclusiva de C3; útil para índices, hashing,
  aritmética modular y "wrapping" seguro a un rango `[0, n)`.

---

## 4. Relación con división, redondeo, desbordamiento y tipos

### 4.1 El módulo no es independiente de la división
El operador `%` **no puede elegirse aislado** de la división entera. Si el lenguaje fija división
hacia cero (recomendación §4.2 del ADR-0005), el resto coherente es **C1** (signo del dividendo).
Si fija división hacia menos infinito, el coherente es **C2**. Proponer división "hacia cero" y
módulo "euclídeo" a la vez es posible, pero debe **declararse explícito** porque rompe el pareo
automático `q`+`r`.

### 4.2 Desbordamiento
`i64::MIN / -1` y `i64::MIN % -1` desbordan porque `+9223372036854775808` no cabe en i64. El
anfitrión Rust **panica** (verificado). En Nexo, coherente con D2 (desbordamiento = error), ambos
deben ser **error de ejecución con ubicación**, no envoltura.

### 4.3 No asumir identidad entre tipos
`entero`, `decimal` y `real` **no** deben comportarse igual en división/módulo. Es un error de
diseño tratarlos idénticos:

| Tipo | División `/` | Módulo `%` | Nota |
|---|---|---|---|
| `entero` | división **entera** (cociente entero) | resto entero (C1/C2/C3) | aquí aplica la tabla §2 |
| `decimal` | división **decimal** (escala creciente, puede ser periódica) | módulo **decimal** con redondeo de cociente a fijar | `1/3` no es finito |
| `real` | división **real** (IEEE 754) | `fmod` (truncado) o `remainder` (IEEE) | depende de D3 |

En `real`, IEEE define **dos** restos distintos: `fmod` (signo del dividendo, análogo a C1) y
`remainder` (round-to-nearest, signo distinto). Esto confirma que `%` sobre `real` es **otra
decisión** y no debe heredarse ciegamente de `entero`.

**Consecuencia:** `%` sobre `decimal` y `real` debe definirse aparte (o aplazarse a biblioteca), no
asumirse por analogía con `entero`.

---

## 5. Recomendación D4 (PENDIENTE)

**Recomendación:** adoptar **C1 (truncado, signo del dividendo)** como `%` **predeterminado** para
`entero`, alineado con la división entera hacia cero (§4.2 ADR-0005) y con el anfitrión Rust.

- **Ventajas:** coherencia interna con `/`; consistencia con C/Rust/Java/JS (ecosistema amplio);
  la identidad `a=(a/b)*b+a%b` es directa; menor sorpresa para programadores habituales.
- **Desventajas:** menos amigable para principiantes que esperan resto no negativo (`-7 % 3` da
  `−1`, no `2`); requiere documentación explícita del signo.
- **Riesgos:** confusión en enseñanza; en hashing/índices se debe usar la forma euclidiana para
  no obtener índices negativos.

**Sí conviene ofrecer una operación explícita de módulo euclídeo**, como función distinta (p. ej.
`euclideo(a, b)` o `modulo_positivo`), **no** como significado del `%` predeterminado:

- **Ventajas:** da `r ∈ [0,|b|)` sin ambigüedad; base correcta para aritmética modular, índices,
  hashing y "wrapping"; elimina la necesidad de que el programador recuerde el signo.
- **Desventajas:** añade un nombre a la superficie del lenguaje/biblioteca; debe enseñarse la
  diferencia con `%`.
- **Riesgo:** duplicidad de operaciones puede confundir si no se documentan con ejemplos contrastados.

** `%` sobre `decimal` y `real`:** diferir. Proponer `%` pleno solo para `entero` en el núcleo; para
`decimal` y `real`, definir en el ADR de tipos o dejar a biblioteca (ver §7 decisiones dependientes).

### Pruebas normativas propuestas (D4)
- PN-MOD-1: `7 % 3` == `1`.
- PN-MOD-2: `-7 % 3` == `-1` (signo del dividendo, C1).
- PN-MOD-3: `7 % -3` == `1`.
- PN-MOD-4: `-7 % -3` == `-1`.
- PN-MOD-5: `6 % 3` == `0` y `-6 % 3` == `0` (división exacta → resto cero).
- PN-MOD-6: `x % 0` → error de ejecución con ubicación.
- PN-MOD-7: `i64::MIN % -1` → error de ejecución (desbordamiento), no envoltura.
- PN-MOD-8: identidad `a == (a/b)*b + a%b` verificada para combinaciones de signos.
- PN-MOD-9 (si euclidiano): `euclideo(-7, 3)` == `2`; `euclideo(7, -3)` == `1`;
  `euclideo(-7, -3)` == `2`; `0 ≤ euclideo(a,b) < |b|`.

### Consecuencias para compatibilidad futura
- Fijar C1 acerca Nexo a C/Rust/Java/JS → buena compatibilidad mental y de portación.
- Dejar `%` de `decimal`/`real` sin definir **no bloquea** el núcleo, pero debe resolverse antes de
  promover literales y operadores a normativos.
- La elección de C1 versus C2 debe revisarse junto a la del redondeo de división entera (§4.2); si
  se cambiara a "hacia menos infinito", el `%` predeterminado debería cambiar a C2 para conservar
  coherencia.

---

# SEGUNDA PARTE — D3: división por cero en `real`

## 1. Políticas comparadas

### P1. Error de ejecución
La división por cero de `real` produce error con ubicación (como ya hace el prototipo actual para
todos los tipos).
- **Ventajas:** coherencia con `entero`/`decimal`; errores tempranos y localizables; sin valores
  "fantasma"; reproducibilidad total (mismo resultado en toda plataforma).
- **Desventajas:** rompe compatibilidad con librerías IEEE; puede interrumpir flujos que en ciencia
  esperan `inf`/`NaN`; exige comprobación en cada división (coste potencial en bucles calientes).

### P2. Comportamiento IEEE 754 (infinito y NaN)
Replicar el hardware: `1/0`→`+inf`, `-1/0`→`-inf`, `0/0`→`NaN`.
- **Ventajas:** igualdad con C/Rust/Python numérico, BLAS, NumPy, librerías científicas y de IA;
  sin comprobación por división (rendimiento); resultados finitos bien definidos para `x/0` con
  `x≠0`; compatibilidad FFI.
- **Desventajas:** `NaN` contamina comparaciones (`NaN==NaN` es falso, verificado); errores que se
  propagan en silencio; difícil de depurar; introduce valores no finitos en un lenguaje que podría
  mantener el conjunto de valores acotado.

### P3. Híbrida / configurable
Ej.: modo de sesión, atributo por módulo, o conversión de `inf`/`NaN` a error solo en contextos
estrictos.
- **Ventajas:** flexibilidad; permite strict-mode para finanzas y modo IEEE para ciencia.
- **Desventajas y riesgo principal:** complejidad injustificada. Rompe la **reproducibilidad** (un
  mismo programa da resultados distintos según configuración); multiplica la matriz de pruebas;
  obliga a documentar dos semánticas; crea "efecto lenguaje" impredecible y puntos ciegos de
  portabilidad. **No se recomienda** mientras no exista evidencia de necesidad real.

---

## 2. Casos concretos (verificados por ejecución en f64/Rust)

**Evidencia [VERIFICADO]**:
```
 1.0/0.0   = inf
-1.0/0.0   = -inf
 0.0/0.0   = NaN
 1.0/-0.0  = -inf
-1.0/-0.0  = inf
 inf - inf = NaN
 inf * 0.0 = NaN
 inf > 1e308= true
 NaN == NaN = false
 NaN < 1.0  = false
 NaN > 1.0  = false
 -0.0 == 0.0= true
```

| Caso | IEEE (P2) | Error (P1) |
|---|---|---|
| `1.0 / 0.0` | `+inf` | error |
| `-1.0 / 0.0` | `-inf` | error |
| `0.0 / 0.0` | `NaN` | error |
| `1.0 / -0.0` | `-inf` | error (requiere distinguir `-0.0`) |
| `inf - inf` | `NaN` | error (solo si `inf` existe) |
| `inf * 0.0` | `NaN` | error |
| resultado no finito usado después | se propaga en silencio | se detiene con error |

Puntos críticos que obliga a decidir P2:
- **`-0.0`**: debe distinguirse del cero positivo en el denominador (`1/-0.0` = `-inf`), aunque
  `-0.0 == 0.0` es verdadero. Añade complejidad léxica y semántica.
- **Propagación**: una vez creado `inf`/`NaN`, todas las operaciones posteriores los propagan y
  las comparaciones dan resultados contra-intuitivos (verificado: `NaN==NaN` falso, todas las
  comparaciones de orden con `NaN` falsas).

---

## 3. Consecuencias

- **Comparaciones, igualdad y orden:** `NaN` rompe la transitividad de la igualdad y hace que el
  orden no sea total. Un lenguaje con `NaN` debe advertir que `==` sobre `real` no es una
  relación de equivalencia. Para ordenar colecciones de `real` se necesita una política adicional
  (¿se rechazan `NaN`? ¿se usa orden total IEEE `totalOrder`?).
- **Propagación de `NaN`:** convierte errores locales en estados tóxicos difusos. Beneficia la
  aritmética vectorial; perjudica la depuración.
- **Depuración:** P1 da fallo localizado y reproducible. P2 requiere caza explícita de `NaN`/`inf`
  (herramientas, aserciones, sanitizadores) para no dejar resultados inválidos corriendo.
- **Reproducibilidad:** P1 es determinista y idéntica en plataformas. P2 es reproducible **dentro**
  de IEEE, pero los resultados no finitos dependen de la política de propagación y pueden variar en
  la conversión a otros tipos.
- **Bibliotecas científicas e IA:** P2 es el ecosistema dominante (NumPy, TensorFlow, PyTorch,
  BLAS). Interoperar con ellas favorece P2 o, al menos, exige conversión explícita en la frontera.

### IEEE 754 garantiza vs. lo que Nexo aún decidiría
Garantiza IEEE: existencia de `±inf` y `NaN`, redondeo a precisión del formato, `x/0` con `x≠0`,
`0/0`→`NaN`, comparaciones con `NaN` falsas, `NaN` como propagador.
**No lo garantiza, y Nexo debe decidirlo:**
- si el lenguaje **expone** `inf`/`NaN` como valores válidos o los convierte en error (P1 vs P2);
- el significado de `-0.0` y si el léxico lo distingue;
- igualdad y orden de `real` con `NaN` presentes;
- conversión de `inf`/`NaN` a `entero`/`decimal` (propuesta: siempre error);
- modo de redondeo por defecto (IEEE fija round-to-nearest, verificado en Rust: banquero);
- si se exponen modos de redondeo alternativos;
- semántica de `real` en igualdad exacta vs tolerancia (ver ADR-0005 §5).

---

## 4. División por cero en `entero` y `decimal` (por separado)

- **`entero / 0`**: no existe resultado representable y no hay infinito entero → **error**. Ya
  comportamiento actual del prototipo; coherente con D2.
- **`decimal / 0`**: aunque el decimal admite gran rango, no define infinito en el modelo
  propuesto → **error**. Mantiene una sola regla para los dos tipos exactos.
- **`real / 0`**: es el único donde cabe `inf`/`NaN` (hardware), de ahí que D3 sea decisión propia
  y no se herede de los otros.

### Detección y reporte (sin ocultar)
Regla transversal: **ningún error se convierte silenciosamente en un valor aparentemente válido**.
- El error de ejecución debe incluir **ubicación** y **motivo** ("división entre cero"), igual que
  el `ErrorEjecucion` actual del prototipo.
- **Prohibido** devolver `0`, `None` o un valor por defecto para "seguir adelante": eso oculta el
  fallo y produce resultados plausibles pero falsos (riesgo alto en finanzas).
- Si en el futuro existe un modo de comprobación estático, la división por cero constante podría
  diagnosticarse en compilación; fuera de alcance ahora.

### Pruebas normativas propuestas (D3)
- PN-DIV-1: `entero / 0` → error con ubicación (comportamiento actual, se fija).
- PN-DIV-2: `decimal / 0` → error con ubicación.
- PN-DIV-3 (según política elegida):
  - si P2: `1.0/0.0` == `+inf`; `-1.0/0.0` == `-inf`; `0.0/0.0` es `NaN`.
  - si P1: `1.0/0.0` → error con ubicación.
- PN-DIV-4 (si P2): `NaN == NaN` → falso; comparaciones de orden con `NaN` → todas falsas.
- PN-DIV-5 (si P2): conversión `inf` o `NaN` a `entero`/`decimal` → error.
- PN-DIV-6: ningún caso de división por cero devuelve `0` ni valor por defecto.

---

## 5. Recomendación D3 (PENDIENTE)

**Recomendación:** **P1 (error de ejecución)** como política del núcleo para `real`, coherente con
`entero` y `decimal`.

- **Ventajas:** una sola regla para los tres tipos; reproducibilidad; errores localizables; sin
  `-0.0`, sin `NaN`, sin comparaciones no transitivas; el lenguaje mantiene un conjunto de valores
  acotado y predecible, coherente con la orientación de diagnóstico explícito de Nexo.
- **Desventajas:** diverge del ecosistema IEEE; una frontera con librerías científicas/IA deberá
  manejar `inf`/`NaN` externos con conversión explícita a error; coste de comprobación en divisiones
  de `real`.
- **Riesgos:** si Nexo apunta en serio a IA/ciencia, imponer P1 puede encarecer la interoperabilidad
  y forzar a recurrir pronto a un `real` alternativo o a bibliotecas que reintroduzcan `inf`/`NaN`.
  Este es el riesgo real de la recomendación y debe sopesarse antes de aprobar.

**Mitigación si se aprueba P1:** definir, en el ADR de tipos o de interoperabilidad, cómo se traduce
la frontera con librerías IEEE externas (¿el `f64` de FFI puede portar `inf`/`NaN`? ¿se validan al
entrar?). **Esto queda como decisión dependiente**, no resuelta aquí.

**Alternativa digna de consideración (no recomendada ahora, no descartada):** P2 con advertencias y
herramientas de caza de `NaN`, si la evidencia futura muestra que el objetivo de IA/ciencia pesa
más que la coherencia de diagnóstico. Requeriría además decidir `-0.0`, orden total y conversión.

---

## 6. Decisiones dependientes (a resolverse en ADR de tipos y errores)

Estas **no** se deciden en este informe; quedan registradas como dependencias:

1. **Redondeo de división entera** (hacia cero vs menos infinito) — condiciona el convenio de `%`
   (D4). Ver ADR-0005 §4.2.
2. **`%` sobre `decimal`** — definición y redondeo del cociente; o aplazo a biblioteca.
3. **`%` sobre `real`** — `fmod` vs `remainder`; depende de si `real` admite `inf`/`NaN` (D3).
4. **Significado de `-0.0`** — solo relevante si D3 aprueba P2.
5. **Igualdad y orden de `real` con `NaN`** — solo si D3 aprueba P2.
6. **Conversión de `inf`/`NaN` a `entero`/`decimal`** — propuesta: siempre error; fijar en ADR de
   tipos.
7. **Modo de redondeo por defecto de `real`** — IEEE round-to-nearest (banquero) propuesto; confirmar
   y decidir si se exponen modos alternativos.
8. **Frontera FFI con librerías IEEE** — cómo se manejan `inf`/`NaN` externos, según D3.
9. **Función euclidiana** — si se incluye en núcleo o biblioteca estándar (ver §5 D4).
10. **Propagación y reporte de errores de ejecución** — plantilla de `ErrorEjecucion` con ubicación
    y motivo; ADR de errores.

---

## 7. Estado final

- **D4:** recomendación C1 (truncado, signo del dividendo) + operación euclidiana explícita
  opcional. **PENDIENTE de aprobación.**
- **D3:** recomendación P1 (error de ejecución) para `real`. **PENDIENTE de aprobación.**
- Sistema numérico **no cerrado**. Conservado estado **Propuesto** del ADR-0005.
- Sin cambios de código, gramática ni pruebas ejecutables. Evidencia obtenida por ejecución
  temporal en `/tmp` (fuera del repo).
