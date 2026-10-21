# Análisis D1 y D6 + Revisión integral D1-D6 (ADR-0005)

> **RESUELTO (2026-10-20):** D1 y D6 (y las decisiones derivadas 1, 12, 15, 16, 17) fueron decididas
> en el ADR-0005 consolidado y aprobado (`ADR-0005-sistema-numerico.md`): tres tipos nativos
> (`entero` i64, `decimal` DECIMAL(28,9), `real` f64); literales `42`→entero, `42.5`→decimal,
> `1e10`→real; conversiones solo implícitas sin pérdida; el lenguaje no genera inf/NaN (predicados
> diferidos). Este documento queda como **material histórico de análisis**, no normativo.

- Documento: informe de análisis **enlazado** desde `ADR-0005-sistema-numerico.md`
- Estado: **Propuesta** (D1-D6 siguen **PENDIENTES**; el sistema numérico **no** se cierra)
- Fecha: 2026-10-20
- Base: anexos previos `ADR-0005-anexo-d3-d4.md` y `ADR-0005-anexo-d2-d5.md`
- Restricción respetada: sin cambios de código, gramática ni tipos nuevos

---

# PARTE A — D1: alcance de los tipos numéricos

## 1. Alternativas comparadas

### A1. Núcleo `entero` + `real` (decimal en biblioteca)
- **Ventajas:** superficie mínima; parser y sistema de tipos simples; una sola jerarquía de
  conversión; FFI directa (`int64_t`, `double`).
- **Desventajas:** el caso financiero queda **sin garantía de exactitud en el lenguaje**: quien
  calcule dinero dependerá de una biblioteca cuyos literales, operadores y conversiones no están
  en la gramática (`0.10` seguiría cayendo a `f64` si el literal por defecto es `real`).
- **Dependencias:** ninguna; es el mínimo viable.
- **Adecuación por dominio:** científicos/IA ✔ · web ✔ · automatización ✔ · financieros/bases de
  datos ✖ (exactitud no garantizada por el lenguaje).

### A2. Núcleo `entero` + `decimal` + `real` (tres tipos)
- **Ventajas:** cubre los tres requerimientos en tensión: rango entero exacto, exactitud decimal
  monetaria, rendimiento científico. Literales, operadores y errores de cada tipo son
  **decisión del lenguaje**, no de una biblioteca.
- **Desventajas:** mayor superficie; obliga a definir conversiones entre tres tipos; más
  literales/sufijos; más pruebas de conformidad.
- **Dependencias:** resuelve D5 (representación), condiciona D3/D6 (`real`), interacciona con D2
  (desbordamiento por tipo).
- **Adecuación:** financieros ✔ · científicos ✔ · bases de datos ✔ · web/automatización ✔ (usan
  `entero`/`real`, no pagan el decimal).

### A3. Núcleo extensible (tipos numéricos definibles/adicionables sin romper el núcleo)
- **Idea:** el núcleo fija un **contrato** (qué operaciones esenciales tiene que satisfacer todo
  tipo numérico: suma, resta, multiplicación, comparación, conversión explícita) y permite que
  tipos adicionales (big-int, racional, complejo, decimal arbitrario) se incorporen después —
  desde biblioteca o desde extensiones del lenguaje — sin redefinir el núcleo.
- **Ventajas:** crecimiento ordenado; no hay que elegir hoy todos los tipos; coherencia garantizada
  por el contrato.
- **Desventajas:** el contrato es diseño profundo (traits/instancia de operador) y no puede
  improvisarse; un contrato mal diseñado es peor que tres tipos concretos. No debe confundirse con
  "metaprogramación numérica" ni con ad-hoc polymorphism sin restricciones.
- **Dependencias:** exige el ADR de tipos/operadores antes de implementar funciones genéricas.

**Lectura:** A2 y A3 **no** son incompatibles: A2 define los tres tipos concretos del núcleo hoy;
A3 define **cómo crecerá** el sistema sin romperse. La recomendación combina ambas (§6).

## 2. ¿Qué ventajas reales da que `decimal` sea nativo frente a biblioteca?

| Aspecto | Nativo | Biblioteca |
|---|---|---|
| Literales `0.10` con escala preservada | ✔ en la gramática | ✖ el literal sigue siendo `real` |
| Operador `+ - * /` sin importar módulos | ✔ | depende de sobrecarga/funciones |
| Errores de precisión/rango con ubicación | ✔ integrados en `ErrorEjecucion` | mensajes de biblioteca, sin contexto del lenguaje |
| Conversión implícita controlada `entero ↔ decimal` | ✔ regla del lenguaje | ✖ siempre explícita o automática de la librería |
| Diagnóstico en compilación futura | ✔ posible | ✖ |
| Exactitud monetaria **garantizada por defecto** | ✔ | ✖ quien olvida importar la librería usa `f64` y pierde exactitud en silencio |

**Ventaja real dominante:** el fallo típico (`0.1+0.2`) deja de ser posible **por defecto** para
quien escribe dinero con el tipo adecuado, y la elección del tipo es visible en el código fuente.
Con decimal en biblioteca, el mismo programa sin importar nada "funciona" con `f64` y produce
`0.30000000000000004` — exactamente el defecto verificado del prototipo.

**Contraargumento honesto:** una biblioteca bien diseñada con literales propios (`1.10m` o
`Decimal::nuevo(1,10)`) puede cubrir muchos casos. Lo que **no** puede cambiar desde biblioteca es
la semántica del literal del lenguaje ni las conversiones implícitas del sistema de tipos.

## 3. Costes de hacer `decimal` nativo

| Área | Coste estimado |
|---|---|
| Parser | reconocer literales decimales (puntos y sufijos) y distinguirlos de `real`; coste bajo |
| Sistema de tipos | un tipo más en la matriz de conversión; coste medio |
| Conversiones | reglas `entero↔decimal` (exactas) y frontera con `real` (pérdida); coste medio-alto (es la parte más delicada) |
| Operadores | reutilizables si el AST es genérico por tipo; coste bajo-medio |
| Bibliotecas | funciones de redondeo, reparto, financieras; coste medio (biblioteca, no núcleo) |
| Interoperabilidad | FFI con `decimal128`/SQL requiere mapeo explícito; coste medio |
| Pruebas de conformidad | matriz de pruebas por tipo; coste medio |

Los costes son reales pero **acotados**; el mayor es la matriz de conversiones, que habrá que
diseñar igualmente para `entero` y `real` (D1-A1 también la necesita).

## 4. Ampliación futura sin romper el núcleo

Mecanismos que evitan romper nada al añadir tipos después:

1. **Contrato numérico mínimo** (A3): toda operación del lenguaje se define sobre un conjunto
   fijo de capacidades. Un tipo futuro (big-int, racional, complejo) se integra si satisface el
   contrato o declara qué operaciones no soporta.
2. **Conversiones siempre explícitas para tipos nuevos:** el lenguaje no añade conversiones
   implícitas al incorporar un tipo; solo las definidas en el núcleo (`entero↔decimal`).
3. **Literales con sufijo explícito:** `100gi` (entero grande), `3ri` (racional) pueden entrar sin
   cambiar el significado de los literales existentes.
4. **Namespace de tipos, no de operadores:** `complejo.suma(x)` puede convivir sin tocar `+`
   mientras el contrato no lo admita; promover `+` a complejos es una decisión posterior y
   compatible.
5. **Versionado de la especificación:** añadir tipos es un cambio **compatible**; reasignar el
   tipo de un literal existente sería **incompatible** — por eso la reclasificación de literales
   debe resolverse ahora (decisión dependiente 15).

## 5. Operaciones comunes vs semántica específica

| Comunes a todo tipo numérico (contrato) | Semántica específica (por tipo) |
|---|---|
| `+ - *` | división: entera vs real vs decimal |
| comparación `< <= > >=` | `%` (módulo) y su convenio |
| igualdad exacta | desbordamiento (error entero; redondeo/Inf real) |
| conversión explícita entre tipos | división por cero (D3) |
| negación unaria (futura, ADR-0004) | valores especiales: NaN/Inf (D6) |
| literal y formato de impresión | escala y redondeo (decimal) |

No deben forzarse al mismo significado: `7/2` es `3` en `entero`, `3.5` en `real`/`decimal`. El
contrato fija la **firma**, no el resultado.

## 6. Evitar conversiones implícitas que pierden precisión

Reglas propuestas (sin cerrar la decisión C-c del ADR-0005 §3):

1. Implícitas **solo** las que son **sin pérdida** en todo su rango: `entero → decimal`.
2. Prohibida la implícita `decimal → real` (destruye exactitud; es la frontera monetaria).
3. `entero → real`: sola con condición explícita — se propone permitirla **solo** si se decide
   así en C-c, acompañada de advertencia en lints; alternativamente exigir `real(x)`.
4. Toda conversión con pérdida es **explícita** y aparece en el código (`real(x)`, `decimal(x)`).
5. Literales sin sufijo: el tipo por defecto debe fijarse (¿`entero` si es entero, `real` si tiene
   punto? ¿o `decimal` con punto?). Es la **decisión 15** y bloquea la Fase 4.

## 7. Ejemplos de programas (ilustrativos, no ejecutables aún)

```
# Financiero — exactitud visible en el tipo
importe = 19.99          # pendiente de decisión 15
tasa    = 0.15
impuesto = importe * tasa      # si decimal: 2.9985 exacto
a_pagar  = redondear(impuesto, 2, mitad_up)   # biblioteca

# Científico — rendimiento y IEEE
masa_tierra = 5.972e24        # real
densidad    = masa_tierra / volumen           # real / real

# Índice/contador — entero exacto
contador = 0
contador = contador + 1       # error si cruza i64::MAX (D2=M1)
```

## 8. ¿Qué se aplaza y qué bloquea antes de variables/asignación/funciones?

**Debe definirse antes de implementar variables y literales (bloquea Fase 4):**
- D1 (alcance de tipos) — sin él no hay tipo que asignar.
- Tipo por defecto de los literales (decisión 15).
- Conversión implícita mínima `entero↔decimal` y prohibición de pérdida (C-c, decisión 16).
- D2 (desbordamiento) — cambia el resultado de `x + 1` en los límites.
- D5 (representación decimal) — si `decimal` entra al núcleo.

**Puede aplazarse sin bloquear variables:**
- D3 y D6 (semántica de `real` no finito) — afectan a operaciones, no a la existencia del tipo.
- D4 (convenio de `%`) — solo bloquea cuando `%` se haga normativo.
- Funciones de redondeo/reparto (biblioteca).
- Tipos futuros (A3) y su contrato completo.

---

# PARTE B — D6: NaN, infinito y valores especiales en `real`

## 1. Políticas comparadas

### P1. Admitir los valores especiales de IEEE 754 (pleno)
`real` puede contener `+inf`, `-inf`, `NaN` y distinguir `-0.0`; las operaciones siguen IEEE.
- **Ventajas:** compatibilidad total con hardware, FFI, NumPy/PyTorch/BLAS; rendimiento (sin
  comprobaciones); reproducibilidad **dentro** de IEEE.
- **Desventajas:** igualdad no transitiva; orden no total; errores que se propagan en silencio;
  `-0.0` añade casos léxicos y semánticos.

### P2. Rechazarlos (errores en la semántica normal)
`real` solo contiene valores finitos; cualquier operación que produciría NaN/Inf produce error.
- **Ventajas:** semántica limpia; comparaciones totales; depuración inmediata; coherente con
  `entero`/`decimal` (D2/D5 con error).
- **Desventajas:** divergencia del ecosistema numérico; frontera FFI necesita validación;
  comprobación en cada operación no acotada.

### P3. Admitirlos en `real` con operaciones y comprobaciones explícitas (híbrido controlado)
Los valores existen (compatibilidad), pero el lenguaje **añade** herramientas: predicados
`es_nan`, `es_infinito`, `es_finito`, `es_cero_negativo`; conversiones a otros tipos siempre
error; documentación explícita de que `==` no es equivalencia con NaN; opcionalmente funciones
`cerca_de(a,b,eps)` para igualdad numérica práctica; en la frontera FFI, conversión/validación
explícita.
- **Ventajas:** compatibilidad + control; el peligro queda visible en la API del lenguaje; coste
  de complejidad **acotado** (un puñado de predicados, no dos dialectas).
- **Desventajas:** el programador puede ignorar los predicados; `==` con NaN sigue siendo trampa
  hasta que se use una herramienta; no elimina la no transitividad.
- **Complejidad añadida real:** 4-6 funciones de comprobación + reglas de conversión + pruebas.
  **No** introduce configuración por sesión ni modos (se descarta el híbrido configurable del
  anexo d3-d4 por no reproducible).

## 2. Efectos estudiados (evidencia previa verificada)

| Fenómeno | Comportamiento verificado (f64/Rust) | Consecuencia P1/P3 | Consecuencia P2 |
|---|---|---|---|
| `1.0/0.0` | `inf` | depende de D3 | error |
| `-1.0/0.0` | `-inf` | idem | error |
| `0.0/0.0` | `NaN` | idem | error |
| `1.0/-0.0` | `-inf` | `-0.0` debe existir en el léxico | error |
| `inf - inf`, `inf*0` | `NaN` | propagación | error |
| `NaN == NaN` | falso | igualdad no es equivalencia | no aplica (NaN inexistente) |
| `NaN < / >` | falsos | orden no total | no aplica |
| `-0.0 == 0.0` | verdadero | distinto en bits, igual en valor | no aplica |
| `(inf) as i64` en Rust | saturante/0 según versión | conversión debe ser error en Nexo | no aplica |

## 3. Qué condiciona D3 y qué se decide aquí (independencia)

- **Condicionado por D3:** qué produce `1.0/0.0` (`inf` vs error) y, por tanto, si `inf`/`NaN`
  **nacen** en el lenguaje por división. Si D3 = P1 (error), `inf`/`NaN` no nacen por división,
  pero **podrían seguir entrando** por FFI, constantes (`real::NAN`), desbordamiento de
  multiplicación (`1e308*10` → `inf` en hardware) y conversiones.
- **Independiente de D3 (decidir aquí):**
  - si el **tipo** `real` puede **representar** valores no finitos (modelo de valores);
  - reglas de igualdad y orden;
  - conversión a `entero`/`decimal` (propuesta: siempre error);
  - `-0.0` en el léxico;
  - predicados y herramientas;
  - comportamiento de desbordamiento de magnitud (`1e308*10`): `inf` (P1) o error (P2).

**Matiz crítico:** aun si D3 = error, un programa que recibe un `f64` externo puede contener NaN.
La política de valores especiales debe responder a eso: o se **inspecciona en la frontera** (P2/P3
con validación) o se **admite internamente** (P1/P3).

## 4. Recomendación D6 (PENDIENTE)

**Recomendación:** **P3** (admitir valores especiales en `real` + predicados explícitos +
conversiones a `entero`/`decimal` siempre error + documentación de igualdad/orden).

- **Ventajas:** interoperabilidad científica/IA real; complejidad acotada y concreta; no crea dos
  dialectas; coexistente con D3 = P1 (si se aprueba error, los especiales solo llegan por FFI y se
  detectan con predicados).
- **Desventajas:** `==` con NaN sigue siendo trampa; requiere educación y lints.
- **Riesgos:** programador ignora predicados → NaN silencioso; mitigable con lints (`advertir_si_real_no_finito` en contextos estrictos) sin cambiar semántica.
- **P2 descartado como política del tipo** por el costo de frontera FFI y por no poder impedir
  que NaN entre externamente; **se conserva** como modo de verificación (validación en frontera +
  predicados), que es compatible con P3.

Si D3 se aprueba como **error** (P1 del anexo d3-d4), P3 sigue siendo válida: el conjunto de
valores del tipo es amplio, pero el operador `/` no genera especiales; los predicados cubren FFI y
magnitud. **No hay contradicción obligatoria entre D3=P1 y D6=P3.**

## 5. Pruebas normativas propuestas (D6)

- PN-SPC-1: `es_nan` distingue NaN de todo finito.
- PN-SPC-2: `es_infinito(+inf)` y `es_infinito(-inf)` verdaderos; `es_finito` falso.
- PN-SPC-3: conversión `entero(inf)`, `entero(NaN)`, `decimal(inf)`, `decimal(NaN)` → error.
- PN-SPC-4: `-0.0` es igual a `0.0` pero `es_cero_negativo(-0.0)` verdadero.
- PN-SPC-5: con NaN presente, `==` falso y comparaciones de orden falsas (si D6=P1/P3).
- PN-SPC-6: frontera FFI: un `f64` externo NaN no se convierte en `0` ni en valor por defecto.
- PN-SPC-7: `cerca_de(0.1+0.2, 0.3, 1e-9)` verdadero (herramienta de igualdad práctica).

---

# PARTE C — Revisión integral de coherencia D1-D6

## 1. Matriz conjunta

| Decisión | Recomendación actual | Estado | Depende de | Condiciona |
|---|---|---|---|---|
| **D1** alcance | tres tipos + contrato de extensibilidad (A2+A3) | Pendiente | D5 (decimal en núcleo) | literales, conversiones, Fase 4 |
| **D2** desbordamiento | M1 error; `MIN % -1` → `0` (abierta) | Pendiente | D1 (entero en núcleo) | `ErrorEjecucion`, FFI |
| **D3** div/0 en `real` | P1 error | Pendiente | D6 (si `real` admite especiales) | D6 nacimiento de inf/NaN por `/` |
| **D4** módulo | C1 truncado + euclidiano explícito | Pendiente | redondeo de división entera (decisión 1) | `%` en decimal/real |
| **D5** decimal | B2 `DECIMAL(p,s)` acotado | Pendiente | D1 | D1, C-c, reparto |
| **D6** especiales | P3 admitir + predicados | Pendiente | D3 (parcialmente) | conversiones, orden, FFI |

## 2. Contradicciones detectadas (y su estado)

1. **D3=error vs D6=admitir inf/NaN:** **no es contradicción real.** `/` no genera especiales; el
   tipo puede representarlos (FFI, constantes, desbordamiento de magnitud). Requiere decisión
   explícita sobre `1e308*10` (ver §4 decisiones 17).
2. **D2=error vs desbordamiento a `inf` en `real`:** asimetría aparente. Resolución coherente:
   `entero` no tiene infinito (el resultado no cabe → error); `real` IEEE satura a `inf` si D6
   admite especiales. **Coherencia ≠ uniformidad**: cada tipo acota su dominio. Debe documentarse.
3. **D4=C1 (signo del dividendo) vs euclidiano no negativo:** resuelto: `%` predeterminado C1;
   euclidiano como operación **distinta** y nombrada.
4. **Reclasificación de literales (decisión 15) vs compatibilidad:** hoy el literal con punto es
   `f64` experimental. Cambiar su tipo a `decimal` **antes** de normativizar es compatible **ahora**;
   después sería un cambio roto. **Bloqueante** y con fecha.

No se detectaron contradicciones que impidan una propuesta unificada.

## 3. Decisiones que deben resolverse **juntas**

- **Grupo ALFA (bloquea Fase 4 — variables/literales):** D1 + D5 + decisión 15 (tipo por defecto
  de literales) + decisión 16 (C-c conversiones implícitas).
- **Grupo BETA (bloquea operadores `%` y `/` normativos):** D2 + D3 + D4 + decisión 1 (`i64::MIN % -1` y división entera).
- **Grupo GAMMA (bloquea FFI/ciencia):** D3 + D6 + decisión 17 (desbordamiento de magnitud en `real`) + frontera FFI.

## 4. Decisiones que pueden aplazarse

- Redondeo comercial y reparto (biblioteca).
- Contrato completo de extensibilidad A3 (puede definirse con el ADR de tipos).
- `%` sobre `decimal` y `real` (sub-delagación a D4/ADR de tipos).
- Constantes `pi`, `e`; notación exponencial avanzada.
- Modo sin comprobación de desbordamiento (rendimiento extremo).

## 5. Revisión de puntos señalados

### 5.1 Micro-decisión `i64::MIN % -1`
Matemáticamente `MIN = (-1)·MIN + 0`; el resto es `0`, representable. Dos opciones coherentes:
- **(a) Permitir `0`:** el operador `%` solo falla si el **resto** no cabe (nunca en i64) o
  divisor 0. Ventaja: `MIN % -1` usable en algoritmos de hash/abs sin casos especiales.
- **(b) Error por uniformidad:** toda operación con `MIN` y `-1` falla; regla más simple de
  enunciar ("división y módulo por −1 en MIN: error").
- **Recomendación:** (a) `0`, con prueba normativa PN-INT-OVF-5 fijando el valor. Añade un caso
  especial de implementación pero elimina un falso error. **Pendiente.**

### 5.2 Límites e intermedios de `DECIMAL(p,s)`
Reglas que deben fijarse al elegir B2 (todas pendientes):
- **Paso previo:** definir `p` (precisión total) y `s` (escala) del tipo; ej. `DECIMAL(38,9)`.
- **Suma/resta:** escala resultado = max(s1,s2); si los dígitos totales exceden `p` → **redondeo** según regla fija o error (decisión 12). Nunca envoltura silenciosa.
- **Multiplicación:** escala s1+s2 (excede con frecuencia) → normalizar a escala del contexto, con redondeo explícito.
- **División:** no hay escala natural exacta (`1/3`) → resultado con escala fija del contexto y **redondeo obligatorio**; esto debe ser visible y documentado; la identidad `a = q·b + r` no se garantiza en decimal.
- **Intermedios:** las funciones de biblioteca (impuestos, intereses) deben usar **acumulación con precisión suficiente** y redondear **una sola vez** al final; regla de estilo normativa propuesta.
- **Rango:** exceder el entero subyacente → error (coherente con D2).

### 5.3 Reglas de conversión entre los tres tipos (marco, sin cerrar)
| Origen → Destino | Naturaleza | Regla propuesta (pendiente) |
|---|---|---|
| `entero → decimal` | sin pérdida en rango | implícita permitida |
| `decimal → entero` | pérdida si hay fracción | **explícita** + regla de truncado/redondeo a fijar |
| `entero → real` | pérdida si >2^53 | decisión 16 (implícita con lint o explícita) |
| `real → entero` | pérdida + inf/NaN | explícita; inf/NaN → error (D6) |
| `decimal → real` | pérdida decimal | **explícita** (nunca implícita) |
| `real → decimal` | pérdida + especiales | explícita; especiales → error (D6) |

---

# Propuesta integral del sistema numérico (SÍNTESIS, no aprobada)

1. **Tres tipos nativos** (D1=A2): `entero` (i64), `decimal` (`DECIMAL(p,s)` acotado), `real` (f64), con contrato de extensibilidad (A3) para tipos futuros desde biblioteca.
2. **`entero`:** desbordamiento = error (D2=M1); `x/0` = error; división entera hacia cero; `%` C1 signo del dividendo (D4); `MIN % -1` = `0` (micro-decisión 1).
3. **`decimal`:** acotado (D5=B2); división con redondeo documentado; exceso de precisión redondea con regla fija, exceso de rango = error; redondeo comercial/bancario y reparto en biblioteca.
4. **`real`:** división por cero = error (D3=P1) salvo que se prefiera IEEE; valores especiales representables con predicados (D6=P3); conversiones a `entero`/`decimal` siempre error para inf/NaN.
5. **Conversiones:** solo implícitas sin pérdida (`entero→decimal`); toda pérdida explícita.
6. **Errores:** `ErrorEjecucion` distingue desbordamiento, división por cero, exceso decimal, conversión ilegal; siempre con ubicación.
7. **Crecimiento:** big-int, racionales, complejos, precisión arbitraria → biblioteca/extensión bajo el contrato A3, sin tocar literales existentes.

---

# Lista concreta de decisiones que requieren aprobación

| # | Decisión | Recomendación | Bloquea |
|---|---|---|---|
| **D1** | Alcance: 2 vs 3 vs extensible | tres tipos + contrato A3 | Fase 4 |
| **D2** | Desbordamiento entero | error (M1) | operadores |
| **D3** | División por cero en `real` | error | D6/FFI |
| **D4** | Módulo con negativos | C1 + euclidiano explícito | `%` normativo |
| **D5** | Representación decimal | `DECIMAL(p,s)` acotado | D1/Fase 4 |
| **D6** | Valores especiales en `real` | admitir + predicados (P3) | FFI/ciencia |
| **1** | `i64::MIN % -1` | devolver `0` | D2 |
| **12** | Exceso de precisión/rango decimal | redondeo con regla fija / error en rango | D5 |
| **15** | Tipo por defecto de literales | entero si entero; decimal con punto (revisar) | **Fase 4 (bloqueante)** |
| **16** | Conversión implícita `entero→real` | implícita con lint (o explícita) | Fase 4 |
| **17** | Desbordamiento de magnitud en `real` (`1e308*10`) | `inf` si D6 la admite; si no, error | D3+D6 |

---

## Estado final

- **D1-D6: TODAS PENDIENTES.** Ninguna decisión de este informe es aprobada.
- Sistema numérico **NO cerrado**; ADR-0005 conserva estado **Propuesto**.
- Sin cambios de código, gramática ni pruebas ejecutables; sin tipos nuevos.
- Trazabilidad: anexos d3-d4, d2-d5 y este documento enlazados desde ADR-0005.
- Bloqueantes principales para pasar a especificación formal: **grupo ALFA** (D1, D5, 15, 16).
