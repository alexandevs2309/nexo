# Nexo — Base fundacional integral 0.2

**Estado:** Prototipo funcional + base documental. El prototipo Rust en `prototipo/` implementa y verifica los incrementos 1–4 (expresiones numéricas, variables, ámbitos, `si`/`sino`/`sino si`, `mientras`, bloques, comparaciones y lógica con cortocircuito). Los documentos de gobernanza y la especificación normativa 0.3 (`10-especificacion/`) definen el contrato hacia futuras ampliaciones.

## Propósito
Preparar una base extensa para desarrollar Nexo como lenguaje multipropósito, con una experiencia de programación orientada al español, rigor técnico, interoperabilidad y evolución gradual.

## Prototipo actual (`prototipo/`)
- **Incrementos verificados:** 1 (núcleo numérico), 2 (integración/conformidad), 3 (variables y asignaciones), 4 (control de flujo y lógicos).
- **Pruebas:** **211 pruebas en verde**, `cargo clippy --all-targets` sin advertencias.
- **Ejecutar:** `cd prototipo && cargo test && cargo run`
- **Evidencia:** `07-validacion/casos-de-prueba-y-puertas.md`, ADR-0001–ADR-0008.

## Regla fundamental
Completo no significa fingir que todo está decidido. Las decisiones firmes se registran como aprobadas; las cuestiones abiertas incluyen opciones, criterios, riesgos, pruebas y condiciones de cierre. Ninguna tecnología queda aprobada solo por aparecer en estos documentos.

## Cobertura
Visión y requisitos; sintaxis y gramática; semántica; tipos; memoria; funciones y módulos; errores; concurrencia; entrada/salida; versiones; modelos de ejecución; plataformas; seguridad; calidad; CLI; paquetes; IDE; interoperabilidad; documentación; pruebas; riesgos y hoja de ruta. **Especificación normativa:** `10-especificacion/` (Nexo Core 0.3).

## Estados
- Aprobado: decisión explícitamente aceptada.
- Propuesto: dirección de diseño aún revisable.
- Pendiente: requiere evidencia o decisión.
- Exploratorio: posibilidad futura, no compromiso.

## Limitación actual
El prototipo cubre un subconjunto acotado (expresiones, variables, bloques, `si`/`sino`/`mientras`, comparaciones, lógica). **Funciones, retorno, módulos, colecciones, cadenas como valores completos, E/S y sistema de paquetes** no están implementados (ver ADR-0008 y `10-especificacion/registro-de-brechas.md`). Los ejemplos en `ejemplos/` documentan uso conceptual o del subconjunto soportado.
