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
**Verificación local (2026-10-25):** 302 pruebas enumeradas; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --bins` y `cargo test --quiet` pasan en el árbol local. Estos cambios aún no están asociados a un commit ni a CI; no se declara conformidad con Core 0.3 ni superadas las puertas de distribución/estabilidad. La versión Cargo permanece en `0.1.0`; instalación, plataformas, validación empírica y usabilidad siguen pendientes.
