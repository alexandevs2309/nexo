# BASILISCO — Prototipo funcional

**Estado del código:** el manifiesto `prototipo/Cargo.toml` declara la versión `0.7.0`; la versión publicada en el registro de paquetes debe comprobarse por separado. El prototipo Rust de `prototipo/` es un intérprete con lexer, parser, evaluador, funciones, control de flujo, proyectos, módulos, formateador y resolución **local** de paquetes. La ejecución normal utiliza la VM (basada en MIR) por defecto, con fallback al Tree-Walk (`--motor=arbol`). `analizar` integra validaciones léxicas, sintácticas y análisis semántico disponible en el prototipo. El prototipo incluye IR (MIR), VM, optimizador y backends experimentales nativo/WASM. El código se distribuye bajo Apache License 2.0 conforme a `LICENSE`. **Nexo** queda como nombre histórico; la identidad del lenguaje es **BASILISCO**.

## Propósito
Preparar una base extensa para desarrollar BASILISCO como lenguaje multipropósito, con una experiencia de programación orientada al español, rigor técnico, interoperabilidad y evolución gradual.

## Licencia
El código de BASILISCO se distribuye bajo **Apache-2.0**. Consulta el archivo [LICENSE](LICENSE) y [ADR-0016](00-gobernanza/adr/ADR-0016-licencia-apache-2.0.md). La licencia no concede derechos de uso de la marca BASILISCO.

## Prototipo actual (`prototipo/`)
- **Validación:** ejecutar `cargo test` y `cargo clippy --all-targets -- -D warnings`.
- **Ejecutar las pruebas y la demo:** `cd prototipo && cargo test && cargo run -- demo`
- **Ejecutar un archivo:** desde `prototipo/`, `cargo run -- run examples/tutorial_inicio.bsk`.
- **CLI actual:** los binarios `basilisco` y `bsk` comparten el punto de entrada. El contrato actual de mensajes/documentación todavía usa `basilisco` como comando principal; ambos ofrecen `--help` y `--version`.
- **Extensiones actuales:** `.bsk` es canónica; `.nexo` se acepta como legado; `.nx` no está soportada.
- **Migración aprobada, aún no implementada:** `.bs` será la extensión canónica y `bsk` el comando principal documentado. `.bsk` y `.nexo` deben conservarse durante una transición compatible. Ver [plan de migración](08-plan/migracion-convenciones-basilisco.md).
- **Tutorial verificable:** [prototipo/tutorial/01-primer-programa.md](prototipo/tutorial/01-primer-programa.md).
- **Ejemplos ejecutables actuales:** `prototipo/examples/*.bsk`. La migración a `.bs` no se considera terminada hasta actualizar resolución, imports, formateador, fixtures, ejemplos y pruebas.
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
- El comando `analizar` integra el análisis semántico existente; no debe interpretarse como garantía de que todas las reglas de tipos y nombres del lenguaje estén cubiertas. La cobertura debe cerrarse con pruebas positivas y negativas.
- Los módulos locales y proyectos funcionan en el prototipo, pero su semántica y sus garantías todavía son prototipales.
- El gestor resuelve dependencias locales desde rutas configuradas, `paquetes/` o `vendor/` y genera `basilisco.lock`. El comando `instalar-remoto` descarga paquetes desde un registro remoto verificando firma Ed25519 y SHA-256. La revisión del código detectó dos parsers manuales distintos para `basilisco.toml`, ambos limitados a subconjuntos diferentes; hasta unificar el esquema y usar un parser TOML conforme, no debe afirmarse compatibilidad completa con TOML. **Registro remoto R6:** implementado y verificado (servidor Axum/SQLx, auth, namespaces, BPK, Ed25519); ver `prototipo/README-registry.md` y `10-especificacion/registro-de-brechas.md`.
- La concurrencia se limita a `iniciar_tarea` y `esperar_tarea`; no hay canales, cancelación ni sincronización de usuario.
- La biblioteca estándar es pequeña: incluye operaciones de texto y E/S básica. No hay bindings nativos, backend WASM/nativo, depurador ni IDE implementados en este repositorio. **FFI (puente ABI C):** implementada parcialmente — `cargar_ffi`/`llamar_ffi`/`descargar_ffi` con `libffi`/`libloading`; tipos escalares y texto. Límites: sin callbacks, structs ni variádicos. La reimplementación con diseño de seguridad completo queda prevista para Fase 5.
- **IR y VM implementadas** (v0.7.0): el prototipo incluye un IR de medio nivel (MIR) con CFG y verificador, una máquina virtual (VM) con límites de pasos/profundidad, un optimizador básico, y backends experimentales nativo (Rust) y WASM. El motor por defecto es la VM; el Tree-Walk sigue disponible como fallback (`--motor=arbol`). Los backends nativo/WASM son prototipos (soportan enteros y lógicos).

Los ejemplos ejecutables actuales están en `prototipo/examples/*.bsk`; los de `ejemplos/` son conceptuales. Consulta el [registro de brechas](10-especificacion/registro-de-brechas.md) y el [plan de migración de convenciones](08-plan/migracion-convenciones-basilisco.md) para prioridades, estado y criterios de aceptación.

## Estado de publicación
La versión declarada por `prototipo/Cargo.toml` en la rama auditada es `0.6.0`. La publicación en crates.io es un estado distinto del número del manifiesto y debe verificarse antes de anunciar una versión disponible.
