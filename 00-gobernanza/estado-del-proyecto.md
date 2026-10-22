# Estado del proyecto
Nombre: **BASILISCO** (ADR-0009). **Nexo** queda como nombre histórico/temporal. La investigación de marca, dominios y colisiones externas sigue pendiente antes de publicar. Idioma de documentación: español. Sintaxis orientada al español: objetivo de diseño sujeto a validación de usabilidad.

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
Visión: inicial. Requisitos: marco propuesto. Especificación normativa: pendiente. Arquitectura: ADR-0001 (alcance multipropósito), ADR-0002 (lenguaje anfitrión, Rust), ADR-0003 (sintaxis léxica mínima), ADR-0004 (precedencia de operadores), **ADR-0005 (sistema numérico)**, **ADR-0006 (declaraciones, variables y ámbitos)**, **ADR-0007 (bloques, condiciones, ciclos y tipos lógicos)**, **ADR-0008 (funciones, retorno y llamadas)**, **ADR-0009 (identidad BASILISCO)** y **ADR-0010 (CLI y extensión)**. Implementación: **Puerta B superada**; incrementos 1–5 verificados: expresiones numéricas, variables, ámbitos, `si`/`sino`/`sino si`, `mientras`, bloques, comparaciones, lógica con cortocircuito, funciones, retorno y texto. La CLI `basilisco`/`bsk` ejecuta archivos `.bsk` y acepta `.nexo` como legado; `.nx` no está soportada. **264 pruebas en verde (252 de lenguaje + 12 de CLI), clippy limpio**. Validación empírica: pendiente; usabilidad sin probar. Especificación normativa: **BASILISCO Core 0.3** disponible en `10-especificacion/`; los documentos históricos pueden conservar «Nexo».
