# BASILISCO — Prototipo funcional

**Estado:** Prototipo funcional con documentación de uso y aceptación local para el alcance v0.4. El intérprete Rust de `prototipo/` incluye expresiones, variables y ámbitos, funciones, control de flujo, texto, arreglos y listas básicos, además de una CLI para ejecutar archivos. No es una distribución publicada: la versión Cargo sigue en `0.1.0` y v0.4 identifica el objetivo de cierre. La especificación normativa 0.3 está en `10-especificacion/`. **Nexo** queda como nombre histórico; la identidad objetivo es **BASILISCO**.

## Propósito
Preparar una base extensa para desarrollar BASILISCO como lenguaje multipropósito, con una experiencia de programación orientada al español, rigor técnico, interoperabilidad y evolución gradual.

## Prototipo actual (`prototipo/`)
- **Validación:** `cargo test` y `cargo clippy --all-targets -- -D warnings` pasan en el estado verificado.
- **Ejecutar las pruebas y la demo:** `cd prototipo && cargo test && cargo run -- demo`
- **Ejecutar un archivo:** desde `prototipo/`, `cargo run -- run examples/tutorial_inicio.bsk`.
- **CLI:** `basilisco run archivo.bsk`; `bsk` es el alias. `--help` y `--version` muestran ayuda y versión.
- **Extensiones:** `.bsk` es canónica; `.nexo` se acepta como legado; `.nx` no está soportada.
- **Tutorial verificable:** [prototipo/tutorial/01-primer-programa.md](prototipo/tutorial/01-primer-programa.md).
- **Ejemplos ejecutables:** `prototipo/examples/*.bsk`. La CLI indica archivo, línea y columna en errores de ejecución.
- **Perfil de lenguaje candidato:** [perfil de conformidad v0.4](10-especificacion/perfil-de-conformidad-v0.4.md). Es una propuesta para revisión; no es una especificación aprobada ni declara una versión estable.
- **Documentación del proyecto:** gobernanza en `00-gobernanza/`, validación en `07-validacion/` y especificación en `10-especificacion/`.

## Regla fundamental
Completo no significa fingir que todo está decidido. Las decisiones firmes se registran como aprobadas; las cuestiones abiertas incluyen opciones, criterios, riesgos, pruebas y condiciones de cierre. Ninguna tecnología queda aprobada solo por aparecer en estos documentos.

## Cobertura
Visión y requisitos; sintaxis y gramática; semántica; tipos; memoria; funciones y módulos; errores; concurrencia; entrada/salida; versiones; modelos de ejecución; plataformas; seguridad; calidad; CLI; paquetes; IDE; interoperabilidad; documentación; pruebas; riesgos y hoja de ruta. **Especificaciones:** BASILISCO Core 0.3 es una propuesta inicial; el perfil v0.4 es una candidata acotada pendiente de revisión/aprobación.

## Estados
- Aprobado: decisión explícitamente aceptada.
- Propuesto: dirección de diseño aún revisable.
- Pendiente: requiere evidencia o decisión.
- Exploratorio: posibilidad futura, no compromiso.

## Limitaciones actuales
El lenguaje sigue siendo un subconjunto en evolución. Arreglos y listas tienen operaciones básicas; no hay módulos avanzados, sistema de paquetes, biblioteca estándar extensa, depurador ni IDE. La entrada/salida y el soporte de plataformas también son limitados. Los documentos de diseño pueden describir capacidades futuras que todavía no existen en el prototipo. Los ejemplos ejecutables están en `prototipo/examples/*.bsk`; los de `ejemplos/` son conceptuales.
