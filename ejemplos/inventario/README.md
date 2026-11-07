# Ejemplo: Inventario

Proyecto de ejemplo que demuestra módulos (`usar ... como`), funciones
puras sobre un modelo `[nombre, cantidad, precio]`, colecciones, carga
de datos desde JSON (`leer_archivo` + `json_parsear`) y lógica de
negocio: ventas con control de stock, reposición, eliminación, más caro,
agotados y valor total.

## Ejecutar

```bash
cd ejemplos/inventario
basilisco run main.bsk
```

La salida es idéntica con ambos motores (`--motor=vm` y `--motor=arbol`);
el motor VM es el predeterminado.

## Notas

- El catálogo base vive en `datos/inventario.json` (rutas de lectura
  relativas al directorio de trabajo, por eso se ejecuta con `cd`).
- Los precios del JSON son **céntimos enteros**: los decimales de JSON
  no se mezclan con enteros en la aritmética del lenguaje. Por eso el
  valor total se muestra como `2610.0` (céntimos).
- Una venta sin stock suficiente y un alta duplicada se ignoran de
  forma explícita (el inventario se devuelve sin cambios).
