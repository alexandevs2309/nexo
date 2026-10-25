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
**Verificación (2026-10-25):** 304 pruebas enumeradas; formato, Clippy estricto, build de binarios y suite completa pasan localmente. GitHub Actions confirmó CI exitosa en `0914b4e` (run 40225683156). También pasó `cargo install`/`cargo uninstall` en un root temporal Linux. La versión candidata del paquete es `0.4.0`, todavía no etiquetada ni publicada. Falta CI de la nueva matriz de plataformas y decidir cuáles se respaldarán en la distribución.
