# BASILISCO v0.4 — Candidata local

**Fecha de verificación:** 2026-10-25
**Estado:** candidata local; no publicada ni etiquetada.
**Versión del paquete Cargo:** `0.1.0`.

Este documento describe el alcance funcional del prototipo que se está preparando
para cerrar como v0.4. No declara que exista una distribución estable.

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
El perfil contractual candidato está en `10-especificacion/perfil-de-conformidad-v0.4.md`;
es una propuesta para revisión, no una especificación aprobada.

## Evidencia local

Verificado desde `prototipo/` en Linux:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --bins
cargo test --quiet
```

Los comandos pasan localmente en el árbol actual del 2026-10-25. `cargo test -- --list`
enumera 304 pruebas. El estado de CI no pudo consultarse desde este entorno: la página
del commit y el API de checks devolvieron 404. La aceptación CLI
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
IDE o depurador, concurrencia, ni soporte de distribución multiplataforma. La entrada
y salida general siguen siendo limitadas.

## Pendiente antes de publicar

- Decidir y aplicar la versión de paquete que corresponda a la publicación v0.4.
- Ejecutar CI sobre el estado final y asociar la evidencia a un commit identificable.
- Se verificó `cargo install` y `cargo uninstall` en un root temporal de Linux; aún falta verificar el mecanismo de instalación que se publique y la matriz de plataformas objetivo.
- Completar la revisión de seguridad, dependencias y errores conocidos requerida por
  las puertas de distribución y estabilidad.
- Resolver los pendientes de licencia, marca y publicación institucional registrados
  en `CAMBIOS-BASILISCO.md`.