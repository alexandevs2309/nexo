# Plan BASILISCO v0.5.0

**Estado:** en desarrollo. La versión publicada más reciente es `0.4.0`.

## Objetivo

Consolidar una candidata utilizable para proyectos pequeños: instalación y creación de proyectos, ejecución de programas, módulos y dependencias locales, con comportamiento documentado y reproducible. v0.5 no promete un lenguaje completo ni backends multiplataforma.

## Ya implementado en el prototipo

- Creación de archivos y proyectos con la CLI `basilisco`/`bsk`.
- Ejecución interpretada de archivos y proyectos.
- Módulos locales, alias y exportaciones públicas.
- Resolución local de dependencias y generación/lectura de `basilisco.lock`.
- Comandos `analizar`, `formatear`, `probar`, `instalar` y `paquetes`.

## Trabajo necesario para cerrar la candidata

- Alinear README, especificación normativa, alcance del núcleo y registro de brechas con el comportamiento real.
- Definir el alcance garantizado de v0.5 y cubrirlo con pruebas positivas, negativas y de límites.
- Comprobar instalación y flujos de CLI en Linux, macOS y Windows; la presencia de una ruta de código no demuestra soporte de plataforma.
- Dejar claro que `analizar` solo valida léxico y sintaxis y que la semántica se valida principalmente durante ejecución.
- Documentar explícitamente las limitaciones del gestor local, concurrencia, módulos y biblioteca estándar.

## Fuera de alcance de v0.5

Análisis semántico estático general, VM/IR, descarga de paquetes, dependencias transitivas, verificación de integridad, FFI, backends nativos/WASM, biblioteca estándar extensa, depurador e IDE. El árbol actual no contiene una VM ni una IR implementadas.
