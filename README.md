# BASILISCO — Prototipo funcional

**Estado:** BASILISCO 0.4.0 está publicado; la versión 0.5.0 está en desarrollo. El prototipo Rust de `prototipo/` es un intérprete con lexer, parser, evaluador, funciones, control de flujo, proyectos, módulos, formateador y resolución **local** de paquetes. La ejecución normal interpreta el AST directamente. El comando `analizar` comprueba léxico y sintaxis; todavía no hay análisis semántico estático integrado. Este repositorio no contiene una IR ni una máquina virtual implementadas. El código se distribuye bajo Apache License 2.0 conforme a `LICENSE`. **Nexo** queda como nombre histórico; la identidad objetivo es **BASILISCO**.

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

## Estado comprobado y limitaciones
- El comando `analizar` solo valida léxico y sintaxis. La comprobación de tipos y varios errores ocurren durante la ejecución; no hay un pase semántico estático general.
- Los módulos locales y proyectos funcionan en el prototipo, pero su semántica y sus garantías todavía son prototipales.
- El gestor resuelve dependencias locales desde rutas configuradas, `paquetes/` o `vendor/` y genera `basilisco.lock`. No descarga paquetes ni resuelve dependencias transitivas; tampoco verifica firmas o integridad criptográfica.
- La concurrencia se limita a `iniciar_tarea` y `esperar_tarea`; no hay canales, cancelación ni sincronización de usuario.
- La biblioteca estándar es pequeña: incluye operaciones de texto y E/S básica. No hay FFI, bindings nativos, backend WASM/nativo, depurador ni IDE implementados en este repositorio.
- No hay una IR o VM en el árbol actual. Los documentos pueden proponer capacidades futuras sin que eso signifique que existan en el código.

Los ejemplos ejecutables están en `prototipo/examples/*.bsk`; los de `ejemplos/` son conceptuales. Consulta el [registro de brechas](10-especificacion/registro-de-brechas.md) para prioridades y evidencia.

## Estado de publicación
La versión `0.4.0` está publicada. La versión `0.5.0` está en desarrollo y aún no se ha publicado.
