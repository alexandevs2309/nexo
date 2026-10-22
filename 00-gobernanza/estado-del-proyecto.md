# Estado del proyecto
Nombre de trabajo: Nexo. Nombre definitivo pendiente de investigación de marca y colisiones. Idioma de documentación: español. Sintaxis orientada al español: objetivo de diseño sujeto a validación de usabilidad.

## Objetivos
- Reducir barreras lingüísticas sin sacrificar precisión ni calidad profesional.
- Mantener reglas formales y diagnósticos útiles.
- Diseñar interoperabilidad y evolución a largo plazo.
- Expandir capacidades por etapas, según evidencia.

## Hipótesis por validar
- Que la sintaxis española mejore la comprensión.
- Que el lenguaje pueda cubrir múltiples dominios sin un núcleo excesivamente complejo.
- Que el proyecto pueda sostener bibliotecas, herramientas y compatibilidad.

## Madurez
Visión: inicial. Requisitos: marco propuesto. Especificación normativa: pendiente. Arquitectura: ADR-0001 (alcance multipropósito), ADR-0002 (lenguaje anfitrión, Rust), ADR-0003 (sintaxis léxica mínima), ADR-0004 (precedencia de operadores), **ADR-0005 (sistema numérico)**, **ADR-0006 (declaraciones, variables y ámbitos)** y **ADR-0007 (bloques, condiciones, ciclos y tipos lógicos) aprobados**. Implementación: **Puerta B superada**; incrementos 1 (núcleo numérico), 2 (integración/conformidad), **3 (variables y asignaciones)** y **4 (control de flujo y lógicos) verificados**: `si`/`sino`/`sino si`, `mientras`, bloques con ámbito propio, comparaciones y operadores lógicos con cortocircuito ejecutan de extremo a extremo; los_operandos y las condiciones exigen `lógico` estricto; los errores llevan causa tipada (`CausaError`) y ubicación real. **211 pruebas en verde, clippy limpio** (evidencia en `07-validacion/casos-de-prueba-y-puertas.md`). Validación empírica: pendiente; usabilidad sin probar. Especificación normativa: **Nexo Core 0.3** disponible en `10-especificacion/` (fusión con base documental 0.3.1). Siguiente incremento: funciones y retorno — **autorizado (ADR-0008 aprobado)**.
