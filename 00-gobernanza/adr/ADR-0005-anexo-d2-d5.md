# Análisis D2 y D5 — Desbordamiento entero y semántica decimal (ADR-0005)

> **RESUELTO (2026-10-20):** D2 y D5 fueron decididas en el ADR-0005 consolidado y aprobado
> (`ADR-0005-sistema-numerico.md`): desbordamiento entero = error (M1), `i64::MIN % -1` = 0;
> `decimal` nativo DECIMAL(28,9) escalonado i128, redondeo half-even, exceso de precisión redondea,
> exceso de rango = error; redondeo comercial y reparto en biblioteca. Este documento queda como
> **material histórico de análisis**, no normativo.

- Documento: informe de análisis **enlazado** desde `ADR-0005-sistema-numerico.md`
- Estado: **Propuesta** (D2 y D5 siguen **PENDIENTES**; D3 y D4 **no** se cierran)
- Fecha: 2026-10-20
- Fuente de evidencia: ejecución real en `rustc` (temporal en `/tmp`, fuera del repo)
- Restricción respetada: sin cambios de código, gramática ni tipos numéricos nuevos

---

# PARTE A — D2: desbordamiento de enteros

## 1. Marco: cuatro modelos formales

Sea `a OP b` con `OP ∈ {+, −, ×, /, %}` sobre `entero` con rango finito `[MIN, MAX]`.

- **M1. Error:** si el resultado no cabe en `[MIN, MAX]` → error de ejecución con ubicación.
- **M2. Envoltura (wrapping):** el resultado se reduce módulo `2^N` (complemento a dos).
- **M3. Promoción automática:** si el resultado excede el rango, se amplía al siguiente tipo ancho
  (p. ej. `i64` → `big-int`), de forma transparente.
- **M4. Híbrido:** operaciones de alto riesgo (división, módulo, potencia) en M1; resto en M2 o M3.

## 2. Evidencia de ejecución en límites de i64 [VERIFICADO]

Con `overflow-checks` **activados** (comportamiento por defecto del compilador sin optimizar):
```
   MAX + 1 = -9223372036854775808  (sin error)   <- envolvió en silencio
   MIN - 1 =  9223372036854775807  (sin error)   <- envolvió en silencio
   MAX * 2 = -2                    (sin error)   <- envolvió en silencio
   MIN / -1 -> PANIC/overflow                    <- solo este detecta
   MIN % -1 -> PANIC/overflow
```
Con `overflow-checks=off` (aprox. modo release): igual, pero `MIN / -1` y `MIN % -1` también
envuelven en lugar de panic.

**Hecho técnico relevante:** en el anfitrión Rust, la suma/resta/multiplicación **no** detectan
desbordamiento de forma consistente por defecto; solo la división y el módulo con `MIN / -1` sí.
Esto demuestra que **no conviene heredar la semántica del anfitrión**: dejarla como está produciría
envolturas silenciosas (`MAX + 1` → `MIN`), que es exactamente el fallo que D2 busca evitar.

## 3. Tabla de resultados esperados en límites

| Operación | M1 error | M2 envolver | M3 promover |
|---|---|---|---|
| `i64::MAX + 1` | error | `-9223372036854775808` | `9223372036854775808` (big-int) |
| `i64::MIN − 1` | error | `9223372036854775807` | `-9223372036854775809` (big-int) |
| `i64::MAX * 2` | error | `-2` | `18446744073709551614` (big-int) |
| `i64::MIN / −1` | error | `i64::MIN` (o UB) | `9223372036854775808` (big-int) |
| `i64::MIN % −1` | error | `0` | `0` (big-int) |

Nota sobre `i64::MIN / −1` y `i64::MIN % −1`: aunque el **cociente** desborda, el resto es `0` y es
perfectamente representable. Un modelo M1 estricto podría permitir `MIN % -1 == 0` sin error, ya que
el resultado cabe. Esto debe decidirse explícitamente (ver §7, decisión dependiente).

## 4. Evaluación de cada modelo

### M1. Error
- **Seguridad:** máxima; ningún resultado silenciosamente falso.
- **Rendimiento:** comprobación por operación; coste medible en bucles calientes (SIMD).
- **Previsibilidad:** alta; mismo resultado en todas las plataformas.
- **Interoperabilidad:** requiere comprobación en la frontera FFI (el anfitrión puede desbordar).
- **Portabilidad:** alta.
- **Implementación:** media (detectar desbordeo antes de escribir el resultado).
- **Compatibilidad futura:** buena; puede relajarse luego a M3 sin romper código que esperaba error.

### M2. Envoltura
- **Seguridad:** **baja**; produce números válidos pero falsos (`MAX+1` = `MIN`).
- **Rendimiento:** máxima (sin comprobación).
- **Previsibilidad:** baja para quien no espera envoltura; alta para quien la conoce.
- **Interoperabilidad:** alta (idéntica a C/Rust en binario).
- **Portabilidad:** alta.
- **Implementación:** trivial.
- **Compatibilidad futura:** pobre para un lenguaje "serio"; difícil de corregir después sin
  romper código que dependía de la envoltura.

### M3. Promoción automática
- **Seguridad:** media; nunca desborda, pero oculta el coste y el rango.
- **Rendimiento:** **impredecible**; una suma que cruza el límite cae en big-int (heap).
- **Previsibilidad:** baja; el programador no sabe cuándo hay asignación.
- **Interoperabilidad:** pobre en la frontera (big-int no es tipo C).
- **Portabilidad:** alta.
- **Implementación:** alta (unión de tipos, detección, conversión).
- **Compatibilidad futura:** buena para comodidad; mala para rendimiento crítico.

### M4. Híbrido
- **Ventaja:** permite exactitud donde importa (índices, contadores) y comodidad donde se necesita
  (magnitudes grandes).
- **Desventaja:** dos semánticas conviviendo; complejidad de diagnóstico y de enseñanza; puede
  inducir a programar sin saber qué tipo usa. Solo se justifica si hay evidencia de necesidad real.

## 5. Relación con otros errores aritméticos

Debe distinguirse con claridad qué es "error" en aritmética:

| Situación | ¿Desbordamiento? | Política propuesta |
|---|---|---|
| `MAX + 1` | sí | error (M1) |
| `x / 0` (entero) | **no** — es división por cero | error (D3/§división) |
| `MIN / −1` | sí (el cociente no cabe) | error (M1) |
| `MIN % −1` | el resto es `0`, cabe | ver §7 (permitir o error) |

Son causas distintas: **desbordamiento** = el resultado existe pero no cabe en el rango;
**división por cero** = el resultado no está definido. Ambas producen error de ejecución, pero el
**motivo** del diagnóstico debe ser diferente para poder depurar.

## 6. Relación con precisión arbitraria, conversiones y errores

- **Precisión arbitraria:** si M3 existe, la promoción apunta al tipo big-int. Si el núcleo **no**
  incluye big-int (ver §7 D5), M3 no puede ofrecerse como promoción automática; solo como
  **conversión explícita** (`a.gran()` o `entero_gran(a)`).
- **Conversiones:** con M1, una conversión explícita `entero → real` puede perder precisión sin
  error (distinto fenómeno); con M3 la promoción sería entero→big-int sin pérdida.
- **Errores de ejecución:** la plantilla de `ErrorEjecucion` debe distinguir `DesbordamientoEntero`
  de `DivisionPorCero`, ambos con ubicación.

## 7. Recomendación D2 (PENDIENTE)

**Recomendación:** **M1 (error de ejecución)** para el `entero` del núcleo, con la siguiente
matización:

- Suma, resta y multiplicación que exceden `[MIN, MAX]` → **error**.
- `x / 0` → error (no es desbordamiento; motivo distinto).
- `MIN / −1` → **error** (el cociente no cabe).
- `MIN % −1` → **permitir y devolver `0`** (el resto sí cabe), salvo que se prefiera uniformidad
  total de error. Es una micro-decisión que queda abierta (ver §10, decisión dependiente).

**Ventajas:** seguridad y previsibilidad máximas; reproducibilidad; sin envolturas silenciosas como
`MAX+1`=`MIN` (demostrado en §2); coherente con la orientación de diagnóstico de Nexo.

**Desventajas:** coste de comprobación en rendimiento crítico; divergencia con el binario del
anfitrión en la frontera FFI.

**Riesgos:** si Nexo aplica a sistemas de rendimiento extremo (videojuegos, IA), la comprobación
podría necesitar un modo "sin comprobación" o un tipo separado `entero_envolvente`. No se propone
ahora; queda como posibilidad futura, no decisión actual.

**Promoción automática (M3) queda descartada** para el núcleo por imprevisibilidad de rendimiento y
porque expondría big-int (que no forma parte del núcleo). La **envoltura (M2) queda descartada** por
producir resultados silenciosamente falsos, inaceptables en un lenguaje financiero/serio.

### Pruebas normativas propuestas (D2)
- PN-INT-OVF-1: `i64::MAX + 1` → error (no `-9223372036854775808`).
- PN-INT-OVF-2: `i64::MIN − 1` → error.
- PN-INT-OVF-3: `i64::MAX * 2` → error (no `-2`).
- PN-INT-OVF-4: `i64::MIN / −1` → error.
- PN-INT-OVF-5: `i64::MIN % −1` → `0` **o** error (según micro-decisión §10).
- PN-INT-OVF-6: `x / 0` → error con motivo "división por cero", distinto de "desbordamiento".
- PN-INT-OVF-7: el error incluye ubicación.

---

# PARTE B — D5: representación y semántica de `decimal`

## 1. Alternativas profundizadas

### B1. Decimal de escala fija (entero escalonado)
Guarda `valor = entero × 10^escala`, con `escala` constante por tipo (p. ej. siempre 2 decimales).
- **Exactitud:** exacta dentro de su escala; la operación que excede la escala **redondea**.
- **Rango:** limitado por el entero subyacente.
- **Rendimiento:** alto (entero).
- **Complejidad:** baja.
- **Memoria:** baja y fija.
- **Financiero:** bueno para importes con escala conocida (centavos), malo para escalas variables.

### B2. Decimal con precisión y escala acotadas (estilo SQL `DECIMAL(p,s)` / IEEE `decimal128`)
Hasta `p` dígitos significativos y escala `s` máxima; respaldado por entero ancho (i128).
- **Exactitud:** exacta dentro de `[p,s]`; redondea al exceder.
- **Rango:** amplio y **controlado**.
- **Rendimiento:** medio-alto.
- **Complejidad:** media.
- **Memoria:** fija y predecible.
- **Financiero:** es el modelo de SQL y contabilidad; el más adecuado.

### B3. Decimal de precisión arbitraria (estilo `BigDecimal`)
Dígitos y escala ilimitados, en heap.
- **Exactitud:** máxima; escala creciente por operación.
- **Rango:** sin límite efectivo.
- **Rendimiento:** **bajo e impredecible**.
- **Complejidad:** alta.
- **Memoria:** variable; riesgo de agotamiento (problema de seguridad).
- **Financiero:** correcto pero con coste; útil para cálculos intermedios muy largos.

### B4. BCD (cada dígito en 4 bits)
- **Exactitud:** exacta en decimal.
- **Rendimiento:** bajo (sin soporte hardware moderno).
- **Complejidad:** alta; dependencia de librerías.
- **Memoria:** mayor que entero escalonado.
- **Financiero:** legado (mainframes); no aporta frente a B2.

### B5. Racional (numerador/denominador) — por qué NO equivale a decimal de escala fija
Un racional guarda `n/d`. Es **exacto para fracciones** (`1/3` = `1/3` exactamente), pero:
- **no es una representación decimal**: `1/3` no tiene escritura decimal finita;
- el **denominador crece** con las operaciones (mcm acumulado), disparando memoria y coste;
- presentar `n/d` como decimal **exige redondear**, perdiendo la exactitud guardada;
- rompe la intuición contable (un importe no es una fracción arbitraria).

Por tanto: racional ≠ decimal de escala fija. El racional resuelve **exactitud fraccional**; el
decimal resuelve **exactitud de representación monetaria**. Son objetivos distintos. Un `decimal`
de escala fija **sí** representa `0.25` exactamente; un racional lo representa como `1/4`, que no
es un importe monetario estándar.

## 2. Casos resueltos con ejemplos

### Caso 1: `0.1 + 0.2`
- **f64 (hoy):** `0.30000000000000004` (verificado ADR-0005 §1). Inaceptable para dinero.
- **decimal (B1/B2):** con escala suficiente → **`0.3` exactamente** (no hay residuo binario).
- **Lección:** el decimal corrige la **representación**, no el binario.

### Caso 2: `0.10 + 0.10 + 0.10`
- **f64:** `0.30000000000000004`.
- **decimal (escala 2):** `0.10` se guarda como entero `10`; suma = `30` → **`0.30`** exacto,
  con la escala preservada. **No colapsa a `0.3`.**

### Caso 3: `1 / 3`
- **decimal:** `0.333…` **periódico**; no es finito en ningún decimal de precisión limitada.
- Con `p` dígitos → se **redondea** (p. ej. `0.333333` con `p=6`). **No es exacto.**
- **Lección:** un decimal **no** garantiza exactitud en toda operación; solo en la
  **representación** y en operaciones dentro de su precisión.

### Caso 4: `10 / 4`
- **decimal:** `2.5` (exacto, resultado con escala 1). División exacta porque termina en decimal.

### Caso 5: Impuestos y porcentajes
`base = 19.99`, `tasa = 15%` → `19.99 × 0.15`:
- **decimal:** `2.9985` (exacto, escala 4).
- Para **presentar** en moneda hay que **redondear a 2 decimales** → `3.00` (half-up) o `3.00`
  (banquero). Aquí convergen D5 y el redondeo; el decimal **no elige** la regla, solo permite
  guardar el `2.9985` intermedio sin perderlo.

### Caso 6: Reparto con restos
`10.00 / 3` en tres partes de 2 decimales:
- cociente = `3.33`; resto = `0.01`.
- reparto: `3.33`, `3.33`, `3.34` (el resto a la última) → **suma exacta `10.00`**.
- la regla de asignación del resto debe ser **determinista** (¿a la primera, a la última?).
- **Lección:** el decimal permite conservar el `10.00` y detectar que sobra `0.01`; el reparto es
  una **regla de negocio**, no una consecuencia del tipo.

### Caso 7: Valores que exceden precisión o rango
- Exceder **precisión** (`p`): resultado redondeado según la regla fija. No hay error si el valor
  cabe en el rango de dígitos; si no, puede haber desbordamiento.
- Exceder **rango** (entero subyacente de B1/B2): **error** (coherente con D2) o redondeo si se
  prefiere saturación. Debe fijarse explícitamente (ver §10).

### Caso 8: Redondeo comercial vs bancario
`2.5` a 0 decimales:
- **Comercial (half-up / away-from-zero):** → `3`.
- **Bancario (half-even / round-half-to-even, IEEE):** → `2` (el par más cercano). Verificado en
  Rust: `2.5`→`2`, `3.5`→`4`, `0.5`→`0`, `1.5`→`2`.
- **Lección:** el **tipo** decimal no decide; la **regla de redondeo** es una decisión separada y
  debe ser **explícita**, nunca implícita en el operador.

## 3. Cuatro nociones que no deben confundirse

1. **Representación exacta:** `0.10` se guarda tal cual (lo da el decimal).
2. **Precisión del resultado:** cuántos dígitos soporta antes de redondear (lo da `[p,s]`).
3. **Redondeo explícito:** regla aplicada al ajustar (half-up, banquero) — decisión propia.
4. **Regla de negocio monetaria:** reparto de restos, redondeo a favor del fisco, etc. — decisión
   de la aplicación, **no** del tipo.

Afirmar que "decimal garantiza resultados exactos para todas las operaciones" es **falso**: `1/3`
lo demuestra. El decimal garantiza exactitud de **representación** y de operaciones **dentro** de su
precisión; fuera, redondea de forma controlada.

## 4. Núcleo vs biblioteca

**Pertenecería al núcleo (condiciona el lenguaje):**
- La **representación** de `decimal` (recomendada B2).
- Operaciones aritméticas y de comparación básicas.
- Literales decimales y su tipado.
- Semántica de exceso de precisión/rango (redondeo o error).
- La **distinción** entre `decimal` y `real` en el sistema de tipos.

**Podría ser biblioteca estándar (sin romper coherencia):**
- Reglas de **redondeo** (half-up, banquero, hacia cero).
- **Reparto** de importes con restos.
- Funciones financieras (IVA, intereses, amortización).
- **Precisión arbitraria** (B3), **BCD** (B4), **racionales** (B5).
- Potencia y raíces sobre decimal.

**Criterio:** si caracteriza **cómo se representa un valor** o **cambia su tipo**, va al núcleo; si
es una **operación sobre valores ya tipados**, va a biblioteca.

## 5. Relación con `entero` y `real` (conversión implícita SIN cerrar)

Se **posterga** la decisión de conversiones implícitas (quedó como hipótesis C-c en ADR-0005 §3), pero
se fija el marco:
- `entero` ↔ `decimal`: conversión **exacta** en ambos sentidos dentro del rango (entero cabe en
  decimal; decimal entero exacto cabe en entero). Puede ser implícita con seguridad.
- `decimal` ↔ `real`: **pérdida** (`real` introduce residuo binario). Cualquier conversión debe ser
  **explícita** o, si se permite implícita, debe **documentarse el riesgo**. **No se cierra aquí.**
- `entero` ↔ `real`: pérdida si |valor| > 2^53 (verificado). Explícita para conservar exactitud.

---

# PARTE C — Coherencia global

## 1. Matriz de impacto de D2 y D5 sobre D3, D4, tipos y errores

| Decisión | Afecta a D3 (div/0 real) | Afecta a D4 (módulo) | Afecta al sistema de tipos | Afecta al modelo de errores |
|---|---|---|---|---|
| **D2 = M1 (error)** | Refuerza P1 en `entero`; coherencia "error aritmético" | exige tratar `MIN % -1` (§7) | big-int queda **fuera** del núcleo → sin promoción M3 | añade `DesbordamientoEntero` a `ErrorEjecucion`, distinto de `DivisionPorCero` |
| **D2 = M3 (promover)** | sin efecto directo | `MIN % -1` promovería a big-int | **introduce** big-int en el modelo → cambia el ADR de tipos | menos errores; más asignaciones ocultas |
| **D2 = M2 (envolver)** | sin efecto directo | `MIN % -1` = 0 por envoltura | sin efecto | elimina la clase `DesbordamientoEntero` |
| **D5 = B2 (acotado)** | sin efecto directo | `%` sobre decimal requiere redondeo de cociente (abre sub-decisión) | define `decimal` como tipo del núcleo | añade `ExcesoPrecisionDecimal` o redondeo |
| **D5 = B3 (arbitrario)** | sin efecto directo | igual | big-decimal en el núcleo; impacta ADR de memoria | riesgo de agotamiento → error de recursos |
| **D5 = B5 (racional)** | sin efecto | módulo fraccional complejo | introduce tipo racional; rompe intuición monetaria | más conversión/errores de presentación |

**Lectura clave:** D2 y D5 **no** determinan D3 ni D4, pero **sí** alteran el sistema de tipos
(inclusión o no de big-int/decimal) y el catálogo de errores de ejecución. Son decisiones de
**cimientos**; por eso deben resolverse antes de cerrar el ADR de tipos.

## 2. Hechos técnicos vs recomendaciones vs pendientes

**Hechos verificados por ejecución:**
- `f64`: `0.1+0.2`=`0.30000000000000004`; `9007199254740993+1`=`9007199254740992.0`.
- Rust i64 con checks: `MAX+1` envuelve a `MIN` **sin error**; `MIN/-1` y `MIN%-1` panic.
- Rust f64: `1.0/0.0`=`inf`; `0.0/0.0`=`NaN`; `NaN==NaN`=falso; banquero `2.5`→`2`.
- Rust módulo i64 (C1): `-7%3`=`-1`; `7%-3`=`1`.

**Recomendaciones del redactor (NO decisiones):**
- D2 → M1 (error), con `MIN % -1` = `0` permitido (o error por uniformidad).
- D5 → B2 (precisión/escala acotadas, tipo `DECIMAL(p,s)`).
- Redondeo comercial y reparto → biblioteca estándar, explícitos.
- Big-int, precisión arbitraria, racionales, BCD → biblioteca, no núcleo.

**Decisiones que AÚN requieren tu aprobación (pendientes):**
- D1, D2, D3, D4, D5, D6 (todas siguen abiertas; D3/D4 estudiadas, sin cerrar).

## 3. Decisiones dependientes nuevas (derivadas de D2/D5)

11. **`i64::MIN % -1`:** devolver `0` (cabe) o error por uniformidad con `MIN / -1`.
12. **Exceso de precisión decimal:** redondear (con qué regla) o error; y exceso de **rango**:
    redondear/saturar o error.
13. **Asignación del resto en reparto monetario:** a la primera, a la última, o distribución.
14. **Modo sin comprobación de desbordamiento** (si algún día se necesita para rendimiento):
    tipo aparte o modo explícito. No se propone ahora.

---

## 4. Estado final

- **D2:** recomendación M1 (error), con micro-decisión `MIN % -1` abierta. **PENDIENTE.**
- **D5:** recomendación B2 (precisión/escala acotadas), con redondeo y reparto en biblioteca.
  **PENDIENTE.**
- **D3 y D4:** analizadas previamente; **no se cierran**; siguen **PENDIENTES**.
- **D1 y D6:** sin estudiar en este informe; **PENDIENTES**.
- Sistema numérico **no cerrado**. Conservado estado **Propuesto** del ADR-0005.
- Sin cambios de código, gramática ni tipos nuevos. Evidencia por ejecución en `/tmp` (fuera del
  repo). Ver coherencia §2 y decisiones dependientes §4.
