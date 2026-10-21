# ADR-0004 — Precedencia y asociatividad de operadores (prototipo)

- Estado: Aprobado
- Fecha: 2026-10-20
- Responsable: por asignar
- Revisión prevista: al incorporar operadores nuevos (comparaciones, lógicos, módulo)
- Relacionado: ADR-0003 (léxico); `02-lenguaje/gramatica-lexer-parser.md`

## Contexto y requisitos
El lexer (ADR-0003) está implementado y verificado. El siguiente paso hacia la Puerta B es un parser
y evaluador de expresiones. Para que el significado sea inequívoco (NEX-002) debe estar definida la
precedencia: `2 + 3 * 4` no puede depender del orden en que el programador lo escriba.

## Decisiones propuestas

### 1. Operadores del prototipo vertical
Solo binarios: `+`, `-`, `*`, `/` y agrupación con paréntesis `( )`.
Quedan fuera (con condición de revisión): exponenciación, módulo, comparaciones, lógicos.

### 2. Precedencia
| Nivel | Operadores | Asociatividad |
|---|---|---|
| 1 (mayor) | `( )` agrupación | — |
| 2 | `*`, `/` | izquierda |
| 3 (menor) | `+`, `-` | izquierda |

Ejemplos normativos del prototipo:
- `2 + 3 * 4` → `2 + (3 * 4)` = 14
- `(2 + 3) * 4` = 20
- `10 - 3 - 2` → `(10 - 3) - 2` = 5 (asociatividad izquierda; la derecha daría 9)

### 3. Sin operador unario en el prototipo
`-5` o `10 * -2` **no** son válidos todavía. `-5` se escribe `0 - 5`. Motivo: el unario `-` introduce
casos límite en precedencia y diagnósticos; se evaluará en un ADR propio con la Fase 3 (variables).

### 4. Asignación fuera del parser del prototipo vertical
`=` ya tokeniza (ADR-0003) pero el prototipo vertical **no** implementa asignación ni variables:
la Puerta B exige evaluar una expresión, no un programa con estado. `total = 10 + 5` llega en la
Fase 3 junto con variables y control de flujo.

### 5. Semántica numérica del evaluador del prototipo (no normativa)
Los literales numéricos se evalúan como **flotante binario de doble precisión IEEE 754 (`f64`)**,
no como decimal exacto. División entre cero produce **error de ejecución** con ubicación, no un
valor infinito. Esto es decisión de implementación del prototipo; el sistema de tipos real de Nexo
es un ADR futuro y puede cambiarlo.

> **Corrección documental (2026-10-20, trazabilidad).** La versión previa de este §5 describía la
> representación como "decimal de doble precisión". Esa redacción era inexacta: el evaluador usa
> `f64` binario, que no representa todos los decimales. Medido en el prototipo actual: `0.1 + 0.2`
> evalúa a `0.30000000000000004`, y `9007199254740993 + 1` evalúa a `9007199254740992.0`
> (pérdida de precisión por encima de 2^53). La corrección solo aclara la representación ya usada
> en el código; **no altera** las decisiones aprobadas de este ADR (precedencia, operadores,
> unario, asignación). La elección normativa del sistema numérico se difiere al ADR-0005.

## Evidencia
- Los niveles 2/3 y la asociatividad izquierda son el convenio dominante (C, Python, Java, Rust);
  reduce sorpresas y facilita diagnósticos familiares.
- No hay evidencia empírica propia aún; la validación vendrá del parser y de pruebas de usabilidad.

## Consecuencias
- El parser se implementa como descenso recursivo sobre estos tres niveles.
- NEX-002 (semántica inequívoca) queda parcialmente cubierta para aritmética; no para el lenguaje completo.

## Condiciones de revisión
- Revisar al añadir cualquier operador nuevo (el nuevo nivel obliga a rehacer la tabla).
- Revisar el operador unario al empezar la Fase 3.

## Aceptación
Aprobado por el propietario el 2026-10-20. La tabla de precedencia, la asociatividad izquierda y
las exclusiones (unario, asignación) quedan cerradas para el prototipo vertical.
