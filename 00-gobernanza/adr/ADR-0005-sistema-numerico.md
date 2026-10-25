# ADR-0005 — Sistema numérico del núcleo de Nexo

- Estado: **Aprobado** (aprobación conjunta del propietario, 2026-10-20; decisiones D1-D6 y derivadas 1, 12, 15, 16 resueltas según §1)
- Fecha: 2026-10-20 (consolidación final)
- Relacionado: ADR-0004 §5; ADR de tipos (futuro); ADR de errores (futuro)
- Anexos de soporte (análisis previo, no normativos):
  [d3-d4](ADR-0005-anexo-d3-d4.md) · [d2-d5](ADR-0005-anexo-d2-d5.md) · [d1-d6](ADR-0005-anexo-d1-d6.md)

---

## 1. Decisión (propuesta consolidada)

Nexo nace con **tres tipos numéricos nativos**, cada uno con propósito definido:

| Tipo | Representación | Propósito |
|---|---|---|
| `entero` | entero con signo de 64 bits (i64) | contadores, índices, rango exacto |
| `decimal` | entero escalonado de 128 bits, precisión y escala acotadas (`DECIMAL(28,9)`) | dinero, bases de datos, exactitud decimal |
| `real` | flotante binario IEEE 754 de doble precisión (f64) | ciencia, simulación, IA, gráficos |

### 1.1 `entero`
- **Desbordamiento:** error de ejecución con ubicación (`MAX+1`, `MIN-1`, `MAX*2`, `MIN/-1` → error). Sin envolvura, sin promoción automática.
- **División `/`:** división entera con redondeo **hacia cero** (`-7/2` = `-3`).
- **División por cero:** error de ejecución con ubicación.
- **Módulo `%`:** resto de la división truncada, **signo del dividendo** (C/Rust): `-7%3` = `-1`; `7%-3` = `1`; `-7%-3` = `-1`; división exacta → `0`.
- **`i64::MIN % -1`:** devuelve `0` (el resto cabe; solo el cociente desborda).
- **Operación euclidiana:** no es operador del núcleo; función `modulo_euclideo(a,b)` en la biblioteca estándar, con `0 ≤ r < |b|`, cuando se necesite.

### 1.2 `decimal` (`DECIMAL(28,9)`)
- **Representación:** valor = entero de 128 bits × 10^−escala. Escala fija **9** decimales; hasta **19** dígitos enteros (precisión total 28).
- **Exactitud:** exacto en decimal dentro de precisión y escala. `0.10` se guarda como `10` con escala 9 (se imprime `0.100000000` o con formato de moneda); `0.1+0.2` = `0.3` exactamente. **No** garantiza exactitud en `1/3` (periódico): la división decimal **redondea** con la regla fija.
- **Redondeo por defecto:** mitad-par (half-even, banquero), coherente con IEEE y con el anfitrión.
- **Redondeo comercial (half-up) y reparto de restos:** funciones **explícitas** de la biblioteca estándar (`redondear(x, escala, regla)`, `repartir(importe, n)`), nunca comportamiento implícito del operador.
- **Exceso de precisión** (resultado con más de 9 decimales): se redondea con la regla fija (half-even); la operación **no** es error.
- **Exceso de rango** (más de 19 dígitos enteros): error de ejecución con ubicación (coherente con `entero`).
- **División por cero:** error de ejecución con ubicación.
- **División `a/b`:** resultado con escala 9, redondeo half-even, documentado (p. ej. `10/4` = `2.500000000`; `1/3` = `0.333333333`).
- **Intermedios financieros:** las funciones de biblioteca acumulan con escala suficiente y redondean **una sola vez** al final (regla de estilo normativa).

### 1.3 `real`
- Flotante f64 con semántica IEEE 754 para aritmética finita.
- **División por cero (`x/0`, `x≠0`): error** de ejecución con ubicación. `0/0`: error también (no se generan `inf`/`NaN` desde el lenguaje).
- **Valores especiales:** el tipo **admite** `+inf`, `-inf`, `NaN` y `-0.0` (nacen por desbordamiento de magnitud, p. ej. `1e308*10`, y por fronteras FFI), con **predicados explícitos**: `es_nan(x)`, `es_infinito(x)`, `es_finito(x)`, `es_cero_negativo(x)`.
- **Igualdad y orden:** con especiales presentes, `==` no es equivalencia (`NaN == NaN` es falso) y las comparaciones de orden con `NaN` son todas falsas; documentado como advertencia.
- **`-0.0`:** igual a `0.0` en comparación; distinto internamente; `1/(-0.0)` es error por la regla de división por cero.
- **Conversión a `entero`/`decimal`:** siempre error si el valor no es finito.

#### Alcance de implementación para v0.4 (aclaración 2026-10-25)

Esta aclaración no cambia la decisión aprobada sobre `real`; precisa su implementación
en el perfil acotado v0.4:

- Se conservan `+inf`, `-inf`, `NaN` y `-0.0` que surgen de operaciones `f64`.
- Las comparaciones siguen IEEE: `NaN == NaN` es falso, `NaN != NaN` es verdadero y
  todo orden con `NaN` es falso. Infinito se compara según su orden numérico; `-0.0`
  compara igual que `0.0`.
- Dividir por `+0.0` o `-0.0` produce `DivisionPorCero`; no se genera infinito mediante
  división por cero.
- `es_nan`, `es_infinito`, `es_finito` y `es_cero_negativo` no son parte del núcleo ni
  de la garantía v0.4; quedan aplazados a una futura biblioteca estándar. Hasta que
  existan, un programa BASILISCO no puede inspeccionar directamente el signo interno
  de cero ni clasificar los valores especiales.

La suite `prototipo/tests/conformidad.rs` cubre overflow a ambos signos de infinito,
igualdad de infinito, `NaN` en igualdad/desigualdad/orden, conservación de `-0.0`, su
igualdad numérica con `0.0` y división por cero negativo.

### 1.4 Conversiones entre tipos

| Origen → destino | Regla |
|---|---|
| `entero → decimal` | **implícita** (sin pérdida en rango) |
| `entero → real` | **implícita** (admitida; puede perder exactitud si \|v\| > 2^53; advertencia de lint) |
| `decimal → entero` | **explícita** (`entero(x)`); redondeo hacia cero con la parte fraccionaria descartada |
| `decimal → real` | **explícita** (`real(x)`) — nunca implícita |
| `real → entero` | **explícita**; no finito → error |
| `real → decimal` | **explícita**; no finito → error |

### 1.5 Literales
- Sin sufijo: `42` → `entero`; `42.5` → `decimal`; `1e10` → `real`.
- Sufijos explícitos para forzar tipo (detallados en la especificación léxica al implementar).

### 1.6 Errores
`ErrorEjecucion` distingue motivos: `DesbordamientoEntero`, `DivisionPorCero`, `ExcesoRangoDecimal`, `ConversionIlegal`. Siempre con ubicación. Ningún error se sustituye por un valor por defecto (`0`, `NaN` oculto, etc.).

### 1.7 Extensibilidad
Big-int, racionales, complejos, precisión arbitraria y funciones transcendentales: **biblioteca estándar o extensiones futuras**, bajo contrato numérico común (suma, resta, multiplicación, comparación, conversión explícita), sin reasignar el tipo de literales existentes.

---

## 2. Motivos (resumen)

- Cubre los tres dominios en tensión (dinero exacto, rango entero, rendimiento científico) sin un tipo universal ni conversiones silenciosas que destruyan exactitud.
- El fallo monetario verificado en el prototipo (`0.1+0.2` = `0.30000000000000004`) deja de ser posible con `decimal`.
- Error en desbordamiento y división por cero: diagnóstico temprano, reproducibilidad, coherencia con la filosofía de errores del lenguaje.
- `%` truncado: coherente con la división hacia cero y con el anfitrión; euclidiano disponible como función cuando se necesite.

## 3. Consecuencias

- Reclasificación de literales respecto al prototipo experimental (`f64`): compatible **ahora**, porque nada está normativizado; se fija al implementar el núcleo (incremento 1).
- ADR de tipos hereda §1.4; ADR de errores hereda §1.6; especificación léxica hereda §1.5.
- D2, D3, D4, D5, D6 y decisiones derivadas 1, 12, 15, 16, 17 quedan **resueltas en esta propuesta**, sin perjuicio de revisión por evidencia futura.

## 4. Estado

**Aprobado** el 2026-10-20 por el propietario (aprobación conjunta de D1, D2, D3, D4, D5, D6 y
decisiones derivadas 1, 12, 15, 16). Normativo a partir de la implementación del incremento 1
(núcleo numérico). Revisión: solo por contradicción, defecto o evidencia nueva.
