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
Visión inicial; requisitos en evolución; BASILISCO Core 0.3 es una propuesta normativa inicial. El perfil `10-especificacion/perfil-de-conformidad-v0.4.md` fue aprobado por ADR-0015 como contrato acotado; no define ni declara estable el lenguaje completo. `para cada`, rangos, `romper`/`continuar`, `intentar`/`rescatar` y E/S permanecen como extensiones prototipales fuera de esa garantía.
**Publicación:** BASILISCO `0.4.0` está publicado en crates.io y en el Release GitHub `v0.4.0` (commit `810c40d`). La matriz de CI para Linux, macOS y Windows pasó en la preparación de esa versión. El paquete verificado contiene la guía de instalación y un primer programa.

**Versión siguiente:** `0.5.0` está en desarrollo. Su primer alcance es el comando `basilisco nuevo`, que crea un archivo inicial en la carpeta actual.

**Licencia:** Apache License 2.0, aprobada por el autor el 2026-10-25. El historial Git revisado atribuye los commits a la cuenta del autor y el paquete no declara dependencias externas. Esto no constituye una revisión legal independiente.
