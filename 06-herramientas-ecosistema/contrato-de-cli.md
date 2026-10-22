# Contrato propuesto de la CLI de Nexo

## Objetivo
Definir una interfaz estable y automatizable. Los nombres finales deben validarse con usuarios hispanohablantes y aprobarse antes de volverse API pública.

## Comandos candidatos
- `nexo ejecutar <archivo>`: ejecutar un programa.
- `nexo analizar <archivo>`: analizar sin ejecutar efectos.
- `nexo probar [ruta]`: ejecutar pruebas del proyecto.
- `nexo formatear <ruta>`: formatear o comprobar formato.
- `nexo version`: imprimir versiones de CLI, runtime y especificación.
- `nexo ayuda [comando]`: mostrar ayuda.
- `nexo inicializar [ruta]`: crear estructura mínima de proyecto.

## Contrato común
- salida normal a stdout; errores y diagnósticos a stderr;
- códigos de salida documentados y estables;
- modo de salida estructurada para herramientas;
- rutas Unicode y espacios gestionados correctamente;
- no ejecutar dependencias durante análisis;
- opciones explícitas para permisos y límites de recursos;
- `--` separa opciones de argumentos del programa cuando sea necesario;
- `--version` y `--help` deben estar disponibles aunque se localicen los alias.

## Códigos de salida propuestos
- `0`: éxito;
- `1`: error de programa/ejecución;
- `2`: error de uso o argumentos;
- `3`: error de configuración/dependencias;
- `4`: fallo interno del toolchain.

Los códigos son una propuesta; requieren aprobación y tests de integración.
