# BASILISCO v0.4 — Publicado

**Fecha de verificación:** 2026-10-25
**Estado:** publicado como `basilisco 0.4.0` en crates.io y como Release GitHub `v0.4.0`.
**Commit etiquetado:** `810c40d`.

Este documento describe el alcance funcional publicado de BASILISCO v0.4. No declara
estable el lenguaje completo.

## Alcance disponible

- Expresiones numéricas y lógicas, variables, anotaciones de tipo, mutabilidad y
  ámbitos.
- Semántica IEEE de `real` para infinitos, `NaN` y `-0.0`; los predicados de inspección se difieren a la biblioteca estándar.
- Funciones con parámetros y retorno, incluidas funciones recursivas.
- Parámetros anotados o inferidos en la primera llamada; retornos anotados, conversiones aprobadas y `nulo` según ADR-0011.
- Control de flujo garantizado con `si`, `sino` y `mientras`.
- Texto, arreglos y listas con operaciones básicas e indexación.
- CLI `basilisco` y alias `bsk`, con `run`, `--help` y `--version`.
- Extensión canónica `.bsk`; `.nexo` se conserva como legado y `.nx` se rechaza.
- Errores de ejecución con causa y ubicación, incluyendo la ruta del archivo al
  ejecutar desde la CLI.

El prototipo también contiene `para cada`, rangos, `romper`/`continuar`,
`intentar`/`rescatar` y E/S. Son extensiones experimentales fuera de la garantía v0.4;
no se declara conformidad para ellas. El plan v0.4 enumera sus casos límite pendientes.
El perfil contractual acotado está en `10-especificacion/perfil-de-conformidad-v0.4.md`;
fue aprobado por ADR-0015 y no declara estable el lenguaje completo.

## Evidencia local

Verificado desde `prototipo/` en Linux:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --bins
cargo test --quiet
```

Los comandos pasaron localmente en el commit `0914b4e` del 2026-10-25. `cargo test -- --list`
enumeró 304 pruebas. GitHub Actions pasó para ese commit en el run 40225683156; la matriz de
instalación, compilación y pruebas de Ubuntu, macOS y Windows también pasó para la publicación.
La aceptación CLI
incluye un programa que combina variables, arreglo e indexación, ciclo, condición,
función y texto; además verifica que un error informe archivo, línea, columna y causa.
El tutorial está en `prototipo/tutorial/01-primer-programa.md`. Se corrigió además la
inmutabilidad de la variable de `para cada` y la limpieza del ámbito de iteración en
funciones ante `romper`/`continuar`; son regresiones de una extensión prototipal, no
una ampliación del contrato v0.4.
También se corrigieron los rangos exclusivos y vacíos, la propagación de señales de
ciclo por `intentar`/`rescatar` y la distinción entre fin normal de función y retorno
ausente. Estas extensiones siguen fuera de la garantía v0.4.

## Fuera de alcance

No se incluye un sistema de paquetes, módulos avanzados, biblioteca estándar extensa,
IDE o depurador, concurrencia ni soporte multiplataforma exhaustivo. La matriz de CI
verifica runners Ubuntu, macOS y Windows; no cubre todas las versiones de esos sistemas.
La entrada y salida general siguen siendo limitadas.

## Pendiente para futuras versiones

- Ampliar pruebas de uso con programas reales y recoger errores reportados por quienes instalen BASILISCO.
- Revisar marca y estrategia de publicación institucional según `CAMBIOS-BASILISCO.md`.
