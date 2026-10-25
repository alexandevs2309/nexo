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
Visión inicial; requisitos en evolución; BASILISCO Core 0.3 es una propuesta normativa inicial. El perfil `10-especificacion/perfil-de-conformidad-v0.4.md` propone un contrato acotado que incluye funciones tipadas, texto básico, colecciones básicas y CLI; aún requiere revisión/aprobación y no declara conformidad. `para cada`, rangos, `romper`/`continuar`, `intentar`/`rescatar` y E/S permanecen como extensiones prototipales fuera de esa garantía, aunque haya ADR aprobados y código local.
**Verificación local (2026-10-25):** 304 pruebas enumeradas; formato, Clippy estricto, build de binarios, suite completa y `cargo install`/`cargo uninstall` en un root temporal Linux pasan. CI remota no pudo verificarse desde este entorno (GitHub API devuelve 404). No se declara conformidad con Core 0.3 ni superadas las puertas de distribución/estabilidad. La versión Cargo permanece en `0.1.0`; instaladores publicados, matriz de plataformas, validación empírica y usabilidad siguen pendientes.
