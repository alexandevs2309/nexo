# Contrato de la CLI de BASILISCO

**Estado:** implementado el subconjunto mínimo (ADR-0010). El resto sigue siendo propuesta.

## Objetivo
Definir una interfaz estable y automatizable. Los nombres finales deben validarse con usuarios hispanohablantes y aprobarse antes de volverse API pública.

## Comandos implementados
- `basilisco run <archivo>`: ejecutar un programa `.bsk` o un archivo legado `.nexo`.
- `basilisco ejecutar <archivo>`: alias de `run`.
- `basilisco demo`: ejecutar la demostración integrada.
- `basilisco --version`: imprimir la versión de BASILISCO.
- `basilisco --help`: mostrar ayuda.
- `bsk ...`: alias de todos los comandos anteriores.

## Extensiones
- `.bsk`: canónica.
- `.nexo`: legado; se acepta con aviso por `stderr`.
- `.nx`: no soportada; se rechaza con código 2 y un mensaje que indica usar `.bsk` o `.nexo`.
- Otra extensión o ausencia de extensión: error de uso.

## Comandos candidatos (no implementados)
- `basilisco analizar <archivo>`: analizar sin ejecutar efectos.
- `basilisco probar [ruta]`: ejecutar pruebas del proyecto.
- `basilisco formatear <ruta>`: formatear o comprobar formato.
- `basilisco inicializar [ruta]`: crear estructura mínima de proyecto.

## Contrato común
- salida normal a stdout; errores y diagnósticos a stderr;
- códigos de salida documentados y estables;
- modo de salida estructurada para herramientas (pendiente);
- rutas Unicode y espacios gestionados correctamente;
- no ejecutar dependencias durante análisis (pendiente de un analizador separado);
- opciones explícitas para permisos y límites de recursos (pendiente);
- `--` separa opciones de argumentos del programa cuando sea necesario (pendiente);
- `--version` y `--help` deben estar disponibles aunque se localicen los alias.

## Códigos de salida
- `0`: éxito;
- `1`: error de lectura, sintaxis o ejecución;
- `2`: error de uso o extensión no soportada;
- `3`: error de configuración/dependencias (reservado);
- `4`: fallo interno del toolchain (reservado).

Los códigos implementados tienen tests de integración en `prototipo/tests/cli.rs`.

## Evidencia
- `prototipo/src/cli.rs`
- `prototipo/tests/cli.rs`
- ADR-0010
