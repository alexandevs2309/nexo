# ADR-0010 — CLI y ejecución de archivos

- Estado: **Aprobado**.
- Fecha: 2026-10-22.
- Responsable: proyecto BASILISCO.
- Revisión: al añadir módulos, entrada estándar, formato o un cambio de extensión.

## 1. Contexto

Hasta esta decisión, `prototipo/src/main.rs` era un demo con casos hardcodeados. No había lectura de archivos, argumentos, validación de extensión ni códigos de salida. La biblioteca sí permitía ejecutar programas representados como cadenas.

La migración a BASILISCO necesita una interfaz mínima y reproducible para ejecutar archivos `.bsk`, conservar los ejemplos `.nexo` y rechazar extensiones no respaldadas por evidencia.

## 2. Decisión

### 2.1 Comandos

```text
basilisco run <archivo.bsk>
basilisco run <archivo.nexo>
basilisco demo
basilisco --version
basilisco --help
```

`bsk` es un alias con los mismos comandos. `run` y `ejecutar` son equivalentes.

### 2.2 Extensiones

- `.bsk`: canónica.
- `.nexo`: legado; se acepta y se emite un aviso por `stderr`.
- `.nx`: no soportada; se rechaza con un mensaje que indica usar `.bsk` o `.nexo`.
- Otra extensión o ausencia de extensión: error de uso.

### 2.3 Comportamiento

- Sin argumentos se ejecuta el demo, preservando `cargo run`.
- `run` lee el archivo como UTF-8; los errores de lectura van a `stderr`.
- El resultado de la última expresión de nivel superior se imprime si existe.
- `mostrar` imprime su valor desde el evaluador.
- Códigos de salida: `0` éxito; `1` error de lectura, sintaxis o ejecución; `2` error de uso o extensión no soportada.

### 2.4 Implementación

- `prototipo/src/cli.rs`: parser de argumentos, lectura de archivo, política de extensiones y demo.
- `prototipo/src/main.rs`: entrada del binario `basilisco`.
- `prototipo/src/bin/bsk.rs`: entrada del alias `bsk`.
- `prototipo/tests/cli.rs`: pruebas de integración de los dos binarios.
- `prototipo/tests/fixtures/`: archivos `.bsk`, `.nexo`, `.nx` y casos de error.

## 3. Consecuencias

- `cargo run` sigue funcionando como demo.
- `cargo run -- run examples/funciones_basicas.bsk` ejecuta un archivo.
- Los usuarios pueden invocar `basilisco run` o `bsk run`.
- `.nx` no se presenta como compatibilidad inexistente; si aparecen archivos externos, se evaluará una bandera o una extensión adicional con evidencia.
- La CLI no implementa todavía módulos, entrada estándar, formateo ni análisis separado.

## 4. Criterios de aceptación

- `bsk --version` contiene `BASILISCO`.
- `basilisco --version` contiene `BASILISCO`.
- `bsk run tests/fixtures/hola.bsk` imprime `Hola BASILISCO`.
- `bsk run tests/fixtures/legado.nexo` imprime `42` y avisa que es legado.
- `bsk run tests/fixtures/legado.nx` falla con código 2 y explica que `.nx` no está soportado.
- Una ruta con espacios funciona.
- Un archivo inexistente falla con código 1.
- La suite completa sigue pasando.

## 5. Riesgos y pendientes

- Conflicto de `bsk` con BrowserSkill: se mitiga con alias, detección y documentación.
- `.bsk` también está asociada a Nero/Bryce; no se depende del doble clic.
- No hay todavía tests de instalación ni de CI en Windows/macOS/Linux.
- No hay licencia ni metadatos de publicación.
- No se admite `.nx`; si el usuario confirma archivos externos, se revisará esta decisión.

## 6. Reversión

El CLI se puede desactivar volviendo a una versión anterior de `main.rs`; los tests de biblioteca no dependen de él. No se modifica el formato de los archivos ni se migran datos.
