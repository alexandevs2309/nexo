# Estrategia de pruebas, fuzzing y conformidad

## Capas
1. **Unitarias:** operadores, conversiones, lexer, parser y estructuras de datos.
2. **Integración:** fuente → lexer → parser → análisis → ejecución.
3. **Conformidad:** casos normativos con resultado exacto o error esperado.
4. **Regresión:** todo bug corregido incorpora un caso mínimo.
5. **Propiedades:** invariantes matemáticos y estructurales donde sean aplicables.
6. **Fuzzing:** entradas aleatorias y mutadas para lexer, parser y evaluador.
7. **Rendimiento:** benchmarks aislados y escenarios de usuario.
8. **Plataforma:** instalación, rutas, codificación, permisos y ejecución por sistema operativo.
9. **Documentación ejecutable:** ejemplos del tutorial ejecutados en CI.

## Reglas de calidad
- Una prueba debe probar una regla identificable.
- Los casos de error son tan importantes como los casos válidos.
- Cada error corregido debe dejar una prueba de regresión.
- Los tests no deben depender de orden accidental ni de estado global.
- Los tests de rendimiento no deben bloquear corrección; deben reportar entorno y variabilidad.
- No se deben fijar umbrales de rendimiento sin baseline medido.
- Los tests aleatorios deben registrar semilla para reproducibilidad.
- Los casos de fuzzing deben tener límites de tiempo y memoria.

## Suite mínima para el núcleo
- precedencia y asociatividad de cada operador;
- valores límite y overflow de cada tipo;
- conversiones permitidas y rechazadas;
- división por cero y casos extremos de resto;
- cortocircuito de `y` y `o`;
- condición no lógica;
- nombres desconocidos, duplicados y sombreado;
- bloques anidados;
- `si/sino` anidados y ramas no ejecutadas;
- bucles con cero, una y varias iteraciones;
- literales mal formados, delimitadores sin cerrar y Unicode;
- entradas grandes y profundamente anidadas;
- error de usuario sin pánico interno.

## Evidencia de release
Cada publicación debe registrar commit, versión de Rust/toolchain, sistema operativo, comandos ejecutados, resumen de pruebas, análisis estático, artefactos y limitaciones conocidas.
