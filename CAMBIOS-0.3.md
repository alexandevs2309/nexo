# Cambios de Base Ultra 0.3 respecto a 0.2

## Correcciones
- Se corrigió el estado histórico que decía que no existía implementación: ahora reconoce el prototipo en Rust, pero exige verificar el commit actual.
- Se separan explícitamente estado documental, normativo, implementado y verificado.
- Se aclara que una cobertura temática no equivale a especificación completa.

## Nuevos documentos
- Estado normativo y especificación inicial del núcleo.
- Registro priorizado de brechas.
- Gramática de referencia acotada.
- Arquitectura de referencia.
- Catálogo de errores.
- Estrategia de pruebas y fuzzing.
- Modelo de amenazas.
- Matriz de trazabilidad.
- Puertas de liberación.
- Hoja de ruta ejecutable.
- Política propuesta de compatibilidad/versionado.
- Contrato propuesto de CLI.
- Registro de decisiones que no deben inventarse.

## Límites de esta edición
- No cambia el repositorio de código.
- No afirma haber ejecutado pruebas del prototipo.
- No convierte todas las propuestas en decisiones aprobadas.
- No sustituye la revisión de los ADR existentes.
- La gramática de referencia es un borrador que debe compararse con el lexer/parser reales.
