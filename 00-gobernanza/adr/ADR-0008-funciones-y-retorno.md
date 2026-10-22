# ADR-0008 — Funciones, retorno y llamadas (incremento 5)
- Estado: **Aprobado e implementado** (incremento 5 verificado; ver §6).
- Fecha: 2026-10-21.
- Aprobado: 2026-10-22; implementación verificada el 2026-10-22.
- Responsable: proyecto BASILISCO (histórico «Nexo»).
- Revisión prevista: al implementar el incremento 5, o antes si se decide añadir módulos,
  recursión o sobrecarga.

## 1. Contexto y requisitos

El prototipo (incrementos 1–4) tiene: sistema numérico (ADR-0005), declaraciones y variables
(ADR-0006) y control de flujo con tipos lógicos (ADR-0007). Faltan las **funciones**: no hay
forma de reutilizar un fragmento de lógica ni de nombrar un resultado.

Requisitos asumidos del encargo:
1. Poder declarar una función con parámetros y **retornar** un valor.
2. Poder **llamarla** desde una expresión y desde una sentencia.
3. Errores con causa tipada y ubicación real, como en los incrementos anteriores.
4. No romper la semántica ya aprobada (ADR-0003, 0004, 0005, 0006, 0007).
5. Mantener el lenguaje pequeño: el incremento 5 **no** incluye módulos, sobrecarga,
   genéricos, valores por defecto, lambdas ni closures.

Las palabras clave `funcion`, `retornar` y `mostrar` ya están reservadas por el léxico desde
ADR-0003 (`TipoToken::Funcion`, `Retornar`, `Mostrar`), de modo que este incremento no amplía
el espacio de identificadores salvo que se proponga lo contrario en §6.

## Decisión (aprobada 2026-10-22)

Los seis puntos se resuelven de la siguiente manera:

1. **Tipos de parámetros:** inferencia fijada en la primera llamada. Se comporta igual que `sea`: el tipo queda fijado al recibir el primer valor.
2. **Retorno múltiple:** rechazado. Solo se admite `retornar expresion?` con un valor único.
3. **Acceso a variables externas:** no. El cuerpo solo accede a parámetros y variables declaradas dentro de la función (sin captura de ámbito).
4. **`mostrar`:** diferido al incremento de E/S. No se implementa en el incremento 5.
5. **Recursión:** admitida desde el primer incremento. Si fuera necesario, se añadirá un límite de profundidad con evidencia.
6. **Funciones anidadas:** no permitidas en este incremento.

Esta decisión no altera el léxico ni crea nuevos tipos; solo añade declaraciones de función, llamadas en expresiones/sentencias y `retornar` explícito.

## 2. Opciones consideradas

### O1 — Funciones con `retornar` explícito, un solo tipo de retorno
```text
funcion sumar(a, b) {
  retornar a + b
}
sea total = sumar(2, 3)
```
Ventajas: forma más común en lenguajes de scripting; `retornar` ya está reservado; separa
"valor de retorno" de "última expresión". Costes: exige dos construcciones (`funcion`/`retornar`).
Riesgos: bajo. Reversibilidad: alta. Impacto: sin cambios en el léxico.

### O2 — Funciones con valor implícito (última expresión)
Ventajas: menos sintaxis. Costes: ambiguo con el valor de bloque (ADR-0007 §1.6 difiere el
"bloque con valor"); mezcla dos reglas. Riesgos: medio (semántica confusa, difícil de
diagnosticar). Reversibilidad: baja una vez publicada. **Descartada**: contradice la decisión
de no producir valor en bloques.

### O3 — Solo funciones sin retorno (efecto lateral)
Ventajas: mínimo. Costes: inútil sin E/S; no cubre el requisito 1. Riesgos: bajo.
**Descartada**: no satisface el encargo.

### O4 — Funciones con retorno múltiple / tuplas
Ventajas: expresividad. Costes: requiere decidir el tipo del resultado compuesto, que ADR-0007
dejó fuera del alcance (`lógico` vs numérico) y abre la puerta a un sistema de tipos de
productos. Riesgos: alto (complejidad semántica temprana). **Descartada** para el incremento 5.

### O5 — Funciones como valores (lambdas/closures de primera clase)
Ventajas: máxima potencia. Costes: motor de entornos y conversiones implícitas; no es
necesario para el requisito. **Descartada** para el incremento 5; se reevalúa más adelante.

## 3. Semántica propuesta (detallada)

### 3.1 Léxico
Palabras clave ya reservadas: `funcion`, `retornar`. No hay nuevas palabras clave en O1.

### 3.2 Gramática
```text
declaracion_funcion → "funcion" IDENT "(" parametros? ")" bloque
parametros          → IDENT ("," IDENT)*
sentencia_retorno   → "retornar" expresion?
llamada             → IDENT "(" argumentos? ")"
argumentos          → expresion ("," expresion)*
```

### 3.3 Tipos de parámetros y retorno
- Los parámetros **no se anotan**: el tipo se infiere del valor recibido en la primera
  llamada y se fija, como en las variables (ADR-0006 §1.4: la declaración fija el tipo).
  Alternativa con anotación explícita (`funcion f(a: entero)`) queda para §6.
- El tipo de retorno se infiere de la expresión de `retornar`; una función sin `retornar`
  devuelve **nada** (se distinguirá de un valor `lógico`/numérico).
- Una función sin `retornar` que se use en una expresión es error
  `RetornoAusente { funcion }`.

### 3.4 Ámbitos
- Los parámetros viven en el ámbito del bloque de la función, uno por llamada (equivalente a
  `sea` en el bloque, ADR-0006 §1.6).
- **Las funciones no capturan el ámbito del llamante**: el cuerpo ve solo sus parámetros y
  las variables declaradas dentro de él. Esto mantiene la semántica de ámbitos simple y
  evita la categoría de closures. (Consecuencia a decidir: el acceso a globales queda
  abierto; ver §6.)
- Cada llamada tiene su propia pila de ámbitos → la recursión es natural si se permite.

### 3.5 Errores nuevos (extensión de `CausaError`)
- `FirmaInvalida { funcion, detalle }` — nº de argumentos incorrecto.
- `RetornoAusente { funcion }` — se usa el resultado de una función que no retorna.
- `TipoDeRetornoIncompatible { esperado, obtenido }` — retorno incompatible con el tipo
  fijado en la primera llamada.

### 3.6 Reglas decidibles sin nueva evidencia
1. `retornar` sin expresión dentro de una función que sí retorna valor → error sintáctico.
2. `retornar` fuera de una función → error sintáctico ("`retornar` solo puede usarse dentro
   de una función").
3. Una llamada en una sentencia sin usar el resultado es válida (se descarta el valor).
4. La recursión se admite (la pila de llamadas crece; se añadirá un límite de profundidad
   solo si la evidencia lo justifica).

## 4. Evidencia disponible
- El léxico ya reconoce `funcion`/`retornar`/`mostrar` (ADR-0003), lo que reduce el riesgo
  léxico de este incremento a casi cero.
- La infraestructura de ámbitos (pila de `HashMap`) y de errores tipados con ubicación ya
  existe y está verificada por 252 pruebas de lenguaje (ADR-0006, ADR-0007).
- La implementación y las pruebas del incremento 5 están registradas en §6. Siguen pendientes
  las mediciones de rendimiento y usabilidad, pero no bloquean la aprobación del contrato del
  prototipo.

## 5. Consecuencias y migración
- El espacio de identificadores no cambia (las palabras clave ya existían): **sin migración**.
- El alcance del lenguaje crece de forma notable; es el primer mecanismo de abstracción de
  procedures y, por tanto, el punto donde la complejidad del compilador deja de ser trivial.
- Riesgo de que la decisión de "sin tipos explícitos" resulte restrictiva más adelante: está
  registrada como condición de revisión.

## 6. Puntos que requieren confirmación antes de aprobar
1. **Tipo de los parámetros**: ¿inferencia fija en la primera llamada (propuesto) o
   anotación explícita `funcion f(a: entero)`?
2. **Retorno múltiple**: ¿se admite `retornar a, b` (desempacado) o se rechaza (propuesto:
   rechazar, y usar registros/variables en el futuro)?
3. **Acceso a variables externas**: ¿el cuerpo de una función puede leer/escribir variables
   del ámbito global (propuesto: no, hasta que se decida el modelo de módulos) o sí?
4. **`mostrar`**: ¿se implementa en este incremento como sentencia de salida por
   pantalla, o se difiere al incremento de E/S (propuesto: diferir)?
5. **Recursión**: ¿se admite desde el primer incremento de funciones (propuesto: sí, con
   límite de profundidad solo si aparece evidencia de que hace falta)?
6. **Funciones anidadas**: ¿se permiten funciones dentro de funciones (propuesto: no) o sí?

## 7. Condiciones de revisión
- Al implementar: registrar la evidencia real (pruebas, errores, migraciones).
- Si se añaden módulos, sobrecarga, genéricos o closures: revisar §3.3, §3.4 y §5.
- Si aparece evidencia de que la inferencia de tipos por primera llamada es problemática
  (p. ej. funciones con el mismo nombre usadas con tipos distintos), revisar §3.3 y §6.1.

## 6. Notas de implementación (actualizadas 2026-10-22)

- **Parser**: añadidos `sentencia_funcion`, `sentencia_retorno`, parsing de llamadas en `primaria`.
- **Evaluador**: `Entorno` con `HashMap<String, FuncionDef>`, `ejecutar_funcion`/`ejecutar_funcion_sentencias` con pila de ámbitos propia, propagación de `retornar` vía `Result<Valor, ErrorEjecucion>`.
- **Errores**: `FirmaInvalida`, `RetornoAusente`, `TipoDeRetornoIncompatible` — **todos implementados y validados**.
  - `TipoDeRetornoIncompatible`: se guarda el tipo de retorno en la primera llamada exitosa; llamadas posteriores con tipo distinto producen error con ubicación.
- **Semántica**: inferencia de tipos en primera llamada (parámetros fijos tras primera invocación), sin captura de ámbito léxico, recursión permitida, funciones anidadas rechazadas en parser.
- **Tests**: 13 unitarios en `evaluador::tests` + 12 conformidad en `tests/conformidad_funciones.rs`.
