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
Visión inicial; requisitos en evolución; BASILISCO Core 0.3 es una propuesta normativa inicial. No declarar conformidad del prototipo: funciones, texto y colecciones ya implementados no están exhaustivamente definidos por Core 0.3. La garantía prevista para v0.4 cubre expresiones, variables, ámbitos, `si`/`sino`/`mientras`, funciones/retorno, texto básico, arreglos/listas básicos y CLI. `para cada`, rangos, `romper`/`continuar`, `intentar`/`rescatar` y E/S permanecen como extensiones prototipales fuera de esa garantía, aunque haya ADR aprobados y código local.

**Verificación local (2026-10-25):** 295 pruebas enumeradas; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --bins` y `cargo test --quiet` pasan. Se añadieron regresiones para límites de colecciones, rangos y propagación de señales en `intentar`/`rescatar`. La evidencia es local, aún sin commit/CI identificable. No se declara conformidad con Core 0.3 ni superadas las puertas de distribución/estabilidad: instalación, matriz de plataformas, artefactos reproducibles, validación empírica y usabilidad siguen pendientes. La versión Cargo permanece en `0.1.0`. Los documentos históricos pueden conservar «Nexo».
