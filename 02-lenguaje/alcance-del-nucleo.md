# Alcance del núcleo
Candidatos a estudiar: literales, expresiones, variables/constantes, bloques, tipos básicos, operadores, condicionales, bucles, funciones, colecciones, errores y módulos.

Separar:
1. Lenguaje central: sintaxis y semántica.
2. Biblioteca estándar: APIs distribuidas con la implementación.
3. Herramientas: CLI, analizador, formateador, depurador.
4. Paquetes externos.
5. Plataformas y APIs de sistema.

No añadir al núcleo una función solo porque sea útil en una plataforma.

## Alcance actual del prototipo v0.5 (implementación observada; no equivale a conformidad completa)
- Léxico: identificadores ASCII, literales numéricos, texto con escapes básicos, comentarios `#`.
- Tipos: `entero`, `decimal`, `real`, `logico`, `texto`, `nulo`, `arreglo`, `lista`, `archivo`.
- Expresiones: aritmética, comparación, lógicos, concatenación de texto, indexación, rangos.
- Sentencias: declaraciones, asignación, `si`/`sino`, `mientras`, `para cada`, bloques, funciones, `retornar`, `mostrar`, `mostrar_sin_salto`, `intentar`/`rescatar`, `importar`.
- Funciones integradas: operaciones de texto y E/S de archivos/stdin.
- CLI: `nuevo`, `run`/`ejecutar`, `analizar`, `formatear`, `inicializar`, `probar`, `instalar`, `paquetes`, `demo`, ayuda, versión.
- Paquetes locales: manifiesto `basilisco.toml`, resolución en `paquetes/` y `vendor/`, bloqueo `basilisco.lock`.
- Análisis: `analizar` comprueba léxico y sintaxis. No hay un pase semántico estático general; los errores semánticos se detectan principalmente al evaluar.
- Modelo de ejecución: intérprete del AST. No hay una IR ni una VM implementadas en este repositorio.
- Concurrencia: primitivas `iniciar_tarea`/`esperar_tarea`; sin canales, cancelación ni sincronización de usuario.
- FFI básico: `cargar_ffi`/`llamar_ffi`/`descargar_ffi` sobre ABI C (extensión prototipal; ver `10-especificacion/ffi-abi.md`).

## Fuera del núcleo v0.5
- Paquetes remotos, firmas e integridad; dependencias transitivas y rangos semver.
- Análisis semántico estático general; IR/VM; backends nativos o WASM; FFI; biblioteca estándar extensa; depurador/IDE.
- Visibilidad de módulos más fina que `exportar` y resolución de rutas relativa al archivo importador.
