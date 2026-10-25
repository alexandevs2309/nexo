# BASILISCO — Prototipo funcional

**Estado:** BASILISCO 0.4.0 está publicado en crates.io y GitHub. El prototipo Rust incluye expresiones, variables y ámbitos, funciones, control de flujo, texto, arreglos y listas básicos, además de una CLI. La versión candidata 0.5.0 está en desarrollo e incorpora `basilisco nuevo` para crear un archivo inicial. El código se distribuye bajo Apache License 2.0 conforme a `LICENSE`. La especificación normativa 0.3 está en `10-especificacion/`. **Nexo** queda como nombre histórico; la identidad objetivo es **BASILISCO**.

## Propósito
Preparar una base extensa para desarrollar BASILISCO como lenguaje multipropósito, con una experiencia de programación orientada al español, rigor técnico, interoperabilidad y evolución gradual.

## Licencia
El código de BASILISCO se distribuye bajo **Apache-2.0**. Consulta el archivo [LICENSE](LICENSE) y [ADR-0016](00-gobernanza/adr/ADR-0016-licencia-apache-2.0.md). La licencia no concede derechos de uso de la marca BASILISCO.

## Prototipo actual (`prototipo/`)
- **Validación:** ejecutar `cargo test` y `cargo clippy --all-targets -- -D warnings`.
- **Ejecutar las pruebas y la demo:** `cd prototipo && cargo test && cargo run -- demo`
- **Ejecutar un archivo:** desde `prototipo/`, `cargo run -- run examples/tutorial_inicio.bsk`.
- **CLI:** `basilisco nuevo` crea `hola.bsk` en la carpeta actual; `basilisco run archivo.bsk` lo ejecuta. `bsk` es el alias. `--help` y `--version` muestran ayuda y versión.
- **Extensiones:** `.bsk` es canónica; `.nexo` se acepta como legado; `.nx` no está soportada.
- **Tutorial verificable:** [prototipo/tutorial/01-primer-programa.md](prototipo/tutorial/01-primer-programa.md).
- **Ejemplos ejecutables:** `prototipo/examples/*.bsk`. La CLI indica archivo, línea y columna en errores de ejecución.
- **Perfil v0.4:** [perfil de conformidad](10-especificacion/perfil-de-conformidad-v0.4.md) aprobado como contrato acotado por ADR-0015. No declara estable el lenguaje completo ni autoriza por sí solo su publicación.
- **Gobernanza:** documentos y decisiones en `00-gobernanza/`, validación en `07-validacion/` y especificación en `10-especificacion/`.

## Regla fundamental
Completo no significa fingir que todo está decidido. Las decisiones firmes se registran como aprobadas; las cuestiones abiertas incluyen opciones, criterios, riesgos, pruebas y condiciones de cierre. Ninguna tecnología queda aprobada solo por aparecer en estos documentos.

## Cobertura
Visión y requisitos; sintaxis y gramática; semántica; tipos; memoria; funciones y módulos; errores; concurrencia; entrada/salida; versiones; modelos de ejecución; plataformas; seguridad; calidad; CLI; paquetes; IDE; interoperabilidad; documentación; pruebas; riesgos y hoja de ruta. **Especificaciones:** BASILISCO Core 0.3 sigue siendo una propuesta inicial; el perfil v0.4 está aprobado para un subconjunto y no define el lenguaje completo.

## Estados
- Aprobado: decisión explícitamente aceptada.
- Propuesto: dirección de diseño aún revisable.
- Pendiente: requiere evidencia o decisión.
- Exploratorio: posibilidad futura, no compromiso.

## Limitaciones actuales
El lenguaje sigue siendo un subconjunto en evolución. Arreglos y listas tienen operaciones básicas; no hay módulos avanzados, sistema de paquetes, biblioteca estándar extensa, depurador ni IDE. La entrada/salida y la distribución requieren trabajo adicional. Los documentos de diseño pueden describir capacidades futuras que todavía no existen en el prototipo. Los ejemplos ejecutables están en `prototipo/examples/*.bsk`; los de `ejemplos/` son conceptuales.

## Estado de publicación
La versión `0.4.0` está publicada. La versión `0.5.0` está en desarrollo y aún no se ha publicado.
