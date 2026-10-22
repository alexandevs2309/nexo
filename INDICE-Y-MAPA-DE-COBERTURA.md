# Índice y mapa de cobertura
La base cubre gobernanza, visión, requisitos, sintaxis, gramática, semántica, tipos, memoria, funciones, concurrencia, E/S, versiones, arquitectura, plataformas, seguridad, calidad, herramientas, interoperabilidad, pruebas y hoja de ruta.

## Estado real
Los temas están organizados y cuentan con marcos de análisis. Muchas decisiones normativas siguen pendientes y no hay implementación funcional. La existencia de un documento no equivale a que el área esté resuelta.

## Para acercarse a una especificación técnica completa
1. Resolver preguntas abiertas mediante investigación y ADR.
2. Publicar gramática normativa y modelo semántico completo.
3. Definir tipos, conversiones, memoria, errores y concurrencia.
4. Definir interfaces y formatos versionados para los componentes elegidos.
5. Añadir pruebas normativas por regla.
6. Validar con prototipos, benchmarks y usuarios.
7. Revisar seguridad y mantenimiento con evidencia.
8. Versionar la especificación y establecer control de cambios.

Toda decisión debe enlazarse con requisitos, evidencia y pruebas.


## Especificación normativa (añadida en fusión 0.3.1)

- `10-especificacion/` — Núcleo normativo 0.3: `estado-normativo.md`, `especificacion-del-nucleo-0.3.md`, `gramatica-de-referencia-0.3.md`, `registro-de-brechas.md`, `auditoria-cruzada-repositorio.md`, `compatibilidad-y-versionado.md`, `decisiones-que-no-deben-inventarse.md`
- `02-lenguaje/catalogo-de-errores-y-diagnosticos.md` — Catálogo unificado de errores/diagnósticos
- `06-herramientas-ecosistema/contrato-de-cli.md` — Contrato de CLI para futuras implementaciones
- `07-validacion/puertas-de-liberacion.md` y `matriz-de-trazabilidad.md` — Trazabilidad ADR→regla→test
- `03-arquitectura/arquitectura-de-referencia-0.3.md` — Arquitectura de referencia actualizada
- `08-plan/hoja-de-ruta-ejecutable.md` — Hoja de ruta accionable
- `05-calidad-seguridad/estrategia-de-pruebas-y-fuzzing.md` y `modelo-de-amenazas-0.3.md` — Calidad y seguridad
