# Registro de decisiones de arquitectura
Toda decisión importante debe documentar contexto, requisitos, opciones, criterios, evidencia, decisión, consecuencias, riesgos y condiciones de revisión.

## Reglas
1. Una sugerencia no es una decisión aprobada sin aceptación explícita.
2. Separar requisitos del usuario de hipótesis técnicas.
3. Registrar alternativas descartadas y motivos.
4. Definir qué evidencia podría cambiar la decisión.
5. Revisar decisiones si cambian requisitos, seguridad, coste o mediciones.

## ADR registradas
| ADR | Título | Estado |
|---|---|---|
| [ADR-0001](adr/ADR-0001-alcance-multiproposito.md) | Alcance: lenguaje multipropósito | Aprobado |
| [ADR-0002](adr/ADR-0002-lenguaje-anfitrión.md) | Lenguaje anfitrión de la primera implementación | Aprobado |
| [ADR-0003](adr/ADR-0003-sintaxis-lexica-minima.md) | Sintaxis léxica mínima del prototipo | Aprobado |
| [ADR-0004](adr/ADR-0004-precedencia-operadores.md) | Precedencia y asociatividad de operadores (prototipo) | Aprobado |
| [ADR-0005](adr/ADR-0005-sistema-numerico.md) | Sistema numérico del núcleo | Aprobado |
| [ADR-0006](adr/ADR-0006-declaraciones-y-variables.md) | Declaraciones, variables y ámbitos iniciales | Aprobado |
| [ADR-0007](adr/ADR-0007-estructuras-de-control-y-logicos.md) | Bloques, condiciones, ciclos y tipos lógicos (incremento 4) | **Aprobado** |
| [ADR-0008](adr/ADR-0008-funciones-y-retorno.md) | Funciones, retorno y llamadas (incremento 5) | **Aprobado** |
| [ADR-0009](adr/ADR-0009-identidad-basilisco.md) | Identidad BASILISCO y retirada progresiva de Nexo | **Aprobado** |
| [ADR-0010](adr/ADR-0010-cli-y-extension.md) | CLI y ejecución de archivos | **Aprobado** |

## ADR previstas
Modelo de ejecución; sistema de tipos; memoria; errores; concurrencia; módulos y paquetes; interoperabilidad; compatibilidad; licencia y gobernanza; codificación; IR y optimización; pruebas de conformidad.

## Trazabilidad de correcciones
| Fecha | Documento | Corrección | Naturaleza |
|---|---|---|---|
| 2026-10-20 | ADR-0004 §5 | "decimal de doble precisión" → "flotante binario de doble precisión IEEE 754 (`f64`)" | Documental; aclara la representación ya usada en el código. No altera ninguna decisión aprobada del ADR-0004. Evidencia: `0.1 + 0.2` = `0.30000000000000004`; `9007199254740993 + 1` = `9007199254740992.0`. Sistema numérico normativo → ADR-0005 (propuesto, sin aprobar). |
| 2026-10-21 | ADR-0007 §5, §6, §7 | §5: los cuatro puntos de confirmación pasan a aceptados (ADR aprobado). §6 (antes "Aceptación: Pendiente") → notas de implementación no normativas (guard de iteraciones, nombre real de la causa `TiposIncompatibles`, `sino` en la línea siguiente, comparación entre lógicos, ámbito por iteración) + §7 de aceptación. | Enmienda documental posterior a la aprobación. No altera ninguna decisión aprobada: el cortocircuito aprobado implica que el operando derecho solo se valida si se evalúa, y así queda precisado. Evidencia: `prototipo/tests/conformidad_control.rs`, `prototipo/tests/control_flujo.rs`. |
