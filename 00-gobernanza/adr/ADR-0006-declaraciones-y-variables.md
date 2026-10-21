# ADR-0006 — Declaraciones, variables y ámbitos iniciales

- Estado: **Aprobado** (autorización de incremento 3, 2026-10-21).
- Alcance: sentencias nuevas del prototipo; no modifica ADR-0003, ADR-0004 ni ADR-0005.

## 1. Decisión

### 1.1 Declaración
`sea nombre = expresión`. El valor se evalúa una vez; el **tipo de la variable es el del
resultado** y queda fijo. Sin inicializador no existe la variable (la gramática exige `=`).

### 1.2 Asignación
`nombre = expresión` sobre una variable ya declarada y visible. Una asignación a un nombre
desconocido **no crea la variable**: error `VariableNoDefinida` con ubicación.

### 1.3 Mutabilidad
Las variables son **mutables** sin calificador. Un calificador de inmutabilidad (`inmutable`)
se estudia en un ADR futuro; no se introduce ahora.

### 1.4 Tipos y conversiones
Los valores son los de ADR-0005 (`entero`, `decimal`, `real`). La asignación acepta:
1. un valor del **mismo tipo** que la variable; o
2. las **conversiones implícitas ya aprobadas** en ADR-0005 §1.4 aplicadas al tipo de la
   variable: `entero → decimal` y `entero → real`.
Cualquier otra combinación → error `AsignacionIncompatible { esperado, obtenido }`
(con `decimal → real` y todas las con pérdida pendientes de conversión explícita, que aún
no tiene sintaxis: se documenta como limitación, no se reabre §1.4).

### 1.5 Errores de nombres
- Referencia a variable no declarada → `VariableNoDefinida` con ubicación; nunca valor por
  defecto (consistente con ADR-0005 §1.6).
- Declaración repetida en el **mismo ámbito** → `DeclaracionDuplicada` con ubicación,
  comprobado antes de evaluar el inicializador.
Los tres motivos de este ADR son causas tipadas de `ErrorEjecucion` (enum `CausaError`),
además de los cuatro motivos aritméticos de ADR-0005.

### 1.6 Ámbitos
Pila de ámbitos (mapa por nivel). La declaración escribe en el nivel actual; la lectura
recorre de dentro hacia fuera. El incremento 3 tiene **un solo ámbito (global)**.
Los bloques (incremento 4) empujarán un nivel al entrar y lo retirarán al salir;
el **sombreado en niveles anidados queda recomendado** (declarar un nombre ya usado en un
nivel exterior está permitido; la duplicidad solo falla dentro del mismo nivel) y se
confirmará al implementar bloques. Diseño compatible con funciones y cuadros de mando futuros.

## 2. Gramática añadida (sentencias)

```text
programa  → sentencia*
sentencia → "sea" IDENT "=" expresión
          | IDENT "=" expresión
          | expresión
```

Cada sentencia termina en fin de línea o EOF. `parsear_expresion` se conserva sin cambios
(expresión única hasta EOF) para compatibilidad con el núcleo existente.

## 3. Alternativas descartadas
- Declaración implícita en asignación (`x = 5` crea `x`): descartada por inseguridad
  (errores de tipeo silenciosos); la restricción del incremento lo prohíbe explícitamente.
- Tipos estáticos con anotación obligatoria (`sea x: entero = 5`): diferido a un ADR de
  sistema de tipos; aquí se infiere de la inicialización.
- Inmutables por defecto: pendiente de decisión propia; mutabilidad explícita es el
  comportamiento mínimo del incremento.

## 4. Consecuencias
- `evaluar` libre sobre una expresión sigue existiendo (entorno vacío); las variables se
  usan vía `Entorno`/`ejecutar_programa`.
- El chequeo de duplicados y de existencia es de ejecución, no estático (intérprete).
