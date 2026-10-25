# Plan BASILISCO v0.5.0

**Estado:** en desarrollo. La versión publicada más reciente es `0.4.0`.

## Objetivo

Hacer que una persona pueda empezar un programa BASILISCO desde una carpeta vacía, sin crear el primer archivo a mano.

## Alcance inicial

- `basilisco nuevo` crea `hola.bsk` en la carpeta actual.
- `basilisco nuevo nombre` crea `nombre.bsk`; si se indica `.bsk`, la conserva.
- No sobrescribir archivos existentes ni aceptar otras extensiones.
- El archivo generado debe poder ejecutarse con `basilisco run archivo.bsk`.
- Mantener el mismo comportamiento con el alias `bsk`.
- Actualizar la guía de instalación y validar el flujo en Linux, macOS y Windows.

## Fuera de este incremento

Generación de directorios completos, módulos e importaciones, gestor de paquetes, depurador e IDE. Se priorizarán después de recibir evidencia de uso de la primera distribución.
