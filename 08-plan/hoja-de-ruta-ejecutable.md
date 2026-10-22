# Hoja de ruta ejecutable de Nexo

La hoja de ruta está ordenada por dependencias, no por fechas prometidas. Cada fase debe cerrarse con evidencia.

## Fase 0 — Congelar la referencia
**Objetivo:** saber qué existe realmente.
- Fijar commit de referencia.
- Ejecutar `cargo test`, `cargo clippy` y formato en CI.
- Guardar resultados de CI.
- Alinear README, estado del proyecto y ejemplos.
**Salida:** baseline reproducible y lista de discrepancias.

## Fase 1 — Cerrar el contrato del núcleo
- Aprobar alcance y gramática del núcleo.
- Completar tabla de precedencia, conversiones y overflow.
- Definir nombres, mutabilidad, inicialización y sombreado.
- Estabilizar formato de diagnósticos.
**Salida:** especificación Core versionada y suite de conformidad.

## Fase 2 — Funciones y retorno
- Diseñar parámetros, aridad, retorno y errores.
- Resolver recursión, ámbitos y evaluación.
- Implementar casos positivos y negativos.
- Actualizar ADR, ejemplos y trazabilidad.
**Salida:** programas con abstracción reutilizable sin ambigüedad semántica.

## Fase 3 — Cadenas, colecciones y errores
- Definir Unicode, escapes y representación de cadenas.
- Elegir colecciones mínimas e iteración.
- Definir propagación de errores.
- Medir costes y documentar complejidad cuando sea relevante.
**Salida:** núcleo apto para aplicaciones pequeñas.

## Fase 4 — Herramientas mínimas
- CLI con `ejecutar`, `probar`, `formatear` y `analizar` según contrato aprobado.
- Mensajes y códigos de salida estables.
- Ejemplos ejecutables en CI.
- Instalación reproducible para una plataforma inicial.
**Salida:** experiencia de uso repetible.

## Fase 5 — Módulos y biblioteca estándar
- Resolver importaciones, visibilidad, ciclos y versiones.
- Crear biblioteca estándar mínima: texto, colecciones, matemáticas, archivos y pruebas.
- Definir errores, permisos y compatibilidad de APIs.
**Salida:** proyectos de varios archivos.

## Fase 6 — Paquetes y distribución segura
- Manifiesto, lockfile, integridad y política de ejecución.
- Versionado semántico y migraciones.
- Publicación reproducible y artefactos firmados cuando la infraestructura lo permita.
**Salida:** dependencias gestionables y verificables.

## Fase 7 — Rendimiento e interoperabilidad
- Comparar intérprete, bytecode y compilación mediante prototipos.
- Definir FFI solo tras estabilizar tipos, memoria y errores.
- Benchmark reproducible y perfiles.
**Salida:** decisión basada en datos, no en preferencias.

## Fase 8 — Expansión por dominio
Priorizar con pilotos reales entre web/API, automatización, datos, IA, videojuegos, escritorio, móvil y sistemas. Cada dominio necesita bibliotecas, empaquetado, pruebas y matriz de soporte propia.
**Salida:** un dominio se declara soportado solo después de un piloto mantenible.

## Regla de avance
No iniciar una fase por entusiasmo si sus dependencias críticas siguen abiertas. Se permite investigación paralela, pero no declarar soporte oficial antes de superar su puerta de validación.
