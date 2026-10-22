# BASILISCO — Base fundacional integral 0.2

**Estado:** Prototipo funcional + base documental. El prototipo Rust en `prototipo/` implementa y verifica los incrementos 1–5 (expresiones numéricas, variables, ámbitos, control de flujo, funciones, retorno y texto) y una CLI mínima para ejecutar archivos. Los documentos de gobernanza y la especificación normativa 0.3 (`10-especificacion/`) definen el contrato hacia futuras ampliaciones. **Nexo** queda como nombre histórico/temporal; la identidad objetivo es **BASILISCO**.

## Propósito
Preparar una base extensa para desarrollar BASILISCO como lenguaje multipropósito, con una experiencia de programación orientada al español, rigor técnico, interoperabilidad y evolución gradual.

## Prototipo actual (`prototipo/`)
- **Incrementos verificados:** 1 (núcleo numérico), 2 (integración/conformidad), 3 (variables y asignaciones), 4 (control de flujo y lógicos), 5 (funciones, retorno y texto).
- **Pruebas:** **264 pruebas en verde** (252 de lenguaje + 12 de CLI), `cargo clippy --all-targets` sin advertencias.
- **Ejecutar:** `cd prototipo && cargo test && cargo run -- demo`
- **Ejecutar un archivo:** `cargo run -- run examples/funciones_basicas.bsk`
- **Uso de la CLI:** `basilisco run archivo.bsk`; alias `bsk run archivo.bsk`.
- **Extensiones:** `.bsk` canónica, `.nexo` legado, `.nx` no soportada.
- **Evidencia:** `07-validacion/casos-de-prueba-y-puertas.md`, `10-especificacion/verificacion-2026-10-22/resumen.md`, ADR-0001–ADR-0010.

## Regla fundamental
Completo no significa fingir que todo está decidido. Las decisiones firmes se registran como aprobadas; las cuestiones abiertas incluyen opciones, criterios, riesgos, pruebas y condiciones de cierre. Ninguna tecnología queda aprobada solo por aparecer en estos documentos.

## Cobertura
Visión y requisitos; sintaxis y gramática; semántica; tipos; memoria; funciones y módulos; errores; concurrencia; entrada/salida; versiones; modelos de ejecución; plataformas; seguridad; calidad; CLI; paquetes; IDE; interoperabilidad; documentación; pruebas; riesgos y hoja de ruta. **Especificación normativa:** `10-especificacion/` (BASILISCO Core 0.3; documentos históricos conservan «Nexo»).

## Estados
- Aprobado: decisión explícitamente aceptada.
- Propuesto: dirección de diseño aún revisable.
- Pendiente: requiere evidencia o decisión.
- Exploratorio: posibilidad futura, no compromiso.

## Limitación actual
El prototipo cubre un subconjunto acotado (expresiones, variables, bloques, `si`/`sino`/`mientras`, comparaciones, lógica, funciones, retorno y texto). **Módulos, colecciones, E/S general, entrada estándar, sistema de paquetes, depuración y herramientas de IDE** no están implementados. La CLI ejecuta archivos `.bsk` y acepta `.nexo` como legado; `.nx` no está soportada (ver ADR-0009 y ADR-0010). Los ejemplos en `ejemplos/` documentan uso conceptual o del subconjunto soportado; los ejecutables están en `prototipo/examples/*.bsk`.
