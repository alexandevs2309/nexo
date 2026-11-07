# Ejemplo: Punto de Venta

Sistema de punto de venta que demuestra módulos (`usar ... como`),
colecciones con modelo `[nombre, cantidad, precio]`, aritmética decimal
y reglas de negocio: validación de líneas, totales, descuento por
porcentaje y descuento por volumen (10% si el total supera 100).

## Ejecutar

```bash
cd ejemplos/punto_venta
basilisco run main.bsk
```

La salida es idéntica con ambos motores (`--motor=vm` y `--motor=arbol`);
el motor VM es el predeterminado.
