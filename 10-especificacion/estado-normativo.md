# Estado normativo de BASILISCO (edición histórica «Nexo Core 0.3»)

> **Nota de identidad:** «Nexo Core 0.3» es el nombre histórico de esta edición. La identidad objetivo es **BASILISCO** (ADR-0009). El contrato normativo no se renombra en esta edición para no romper referencias; los documentos nuevos usan BASILISCO.

## Propósito
Este documento evita mezclar decisiones firmes, reglas del prototipo y propuestas de futuro.

## Etiquetas normativas
- **NORMATIVO:** requisito de conformidad para una versión específica de la especificación.
- **IMPLEMENTADO-REFERENCIA:** comportamiento atribuido al prototipo en el expediente actual; requiere verificación en el commit señalado.
- **PROPUESTO:** recomendación todavía no vinculante.
- **PENDIENTE:** decisión necesaria para cerrar la especificación.
- **EXPLORATORIO:** investigación sin promesa de compatibilidad o fecha.

## Alcance de la especificación 0.3
La especificación del núcleo en esta edición es un **contrato normativo inicial y acotado**, no una especificación exhaustiva del lenguaje final. Sus reglas de tipos numéricos y lógica recogen las decisiones documentadas como aprobadas en ADR-0005 y ADR-0007; antes de publicarla como norma, deben compararse con los ADR y el código actuales.

La gramática incluida es una gramática de referencia del subconjunto; no sustituye una gramática completa generada y probada contra el parser real.

## Precedencia documental
En caso de conflicto:
1. Un ADR aprobado gobierna la decisión arquitectónica dentro de su alcance.
2. La especificación normativa de la versión define la conformidad externa del lenguaje.
3. El código es evidencia de implementación, no autoridad para cambiar silenciosamente la especificación.
4. Las pruebas de conformidad verifican reglas, pero no sustituyen su definición.
5. README, tutoriales y ejemplos deben corregirse si contradicen una norma aprobada.

Si hay conflicto entre un ADR y la especificación, se abre una incidencia de coherencia; no se resuelve por interpretación informal.

## Proceso de cambio
Todo cambio que pueda afectar programas existentes requiere:
- propuesta con motivación y alternativas;
- clasificación de impacto: sintaxis, semántica, tipos, runtime, API o tooling;
- pruebas nuevas y pruebas de regresión;
- decisión de compatibilidad;
- actualización de ADR/especificación/gramática/ejemplos;
- nota de migración si el cambio es incompatible.

## Criterio de conformidad
Una implementación solo puede declararse conforme con `Nexo Core 0.3` cuando:
- pasa todos los casos obligatorios de la suite de conformidad identificada;
- produce los resultados y clases de error exigidos;
- no acepta sintaxis que deba rechazarse ni rechaza sintaxis obligatoria;
- publica versión, commit, plataforma y comando de reproducción;
- informa explícitamente cualquier capacidad opcional no soportada.
