# Cambios de identidad y CLI — BASILISCO

**Fecha:** 2026-10-22  
**Estado:** aplicado, commiteado y publicado en `main`; CI en verde. Publicación/licencia pendiente.

- Commit: `ec3b1f2e20657d9286a99491eda93ebb5db4d746`.
- Push: `origin/main` (`a0c1a22..ec3b1f2`).
- CI: run [39872722106](https://github.com/alexandevs2309/nexo/actions/runs/39872722106), conclusión `success`.

## Decisión

- Identidad objetivo: **BASILISCO**.
- Nexo queda como nombre histórico/temporal.
- Paquete Rust: `basilisco`.
- Binario principal: `basilisco`.
- Alias: `bsk`.
- Extensión canónica: `.bsk`.
- Extensión legada: `.nexo`.
- `.nx`: no soportada por ahora.

Véanse ADR-0009 y ADR-0010.

## Cambios aplicados

### Código

- `prototipo/Cargo.toml`: paquete `basilisco`, `default-run = "basilisco"`, binarios `basilisco` y `bsk`.
- `prototipo/src/lib.rs`: módulo público `cli`.
- `prototipo/src/cli.rs`: argumentos, lectura de archivos, extensión, errores, códigos de salida y demo.
- `prototipo/src/main.rs`: entrada del binario `basilisco`.
- `prototipo/src/bin/bsk.rs`: entrada del alias `bsk`.
- Imports y referencias de tests actualizados de `nexo` a `basilisco`.
- `prototipo/examples/funciones_basicas.bsk` y `debug_fact.bsk` añadidos; se conservan los `.nexo`.
- `.github/workflows/ci.yml`: añadidos `cargo fmt --check`, Clippy y `cargo build --bins` antes de `cargo test`; no se ejecutó en esta ronda.

### Pruebas

- `prototipo/tests/cli.rs`: 12 pruebas de integración.
- Fixtures: `.bsk`, `.nexo`, `.nx`, extensión desconocida, sin extensión y ruta con espacios.
- Resultado local: **264/264 pruebas en verde** (252 de lenguaje + 12 de CLI).
- `cargo clippy --all-targets` con `-D warnings`: sin avisos.
- `cargo fmt --check`: pasa.

### Documentación

- `README.md`: identidad, estado, comandos y limitaciones actualizadas.
- `00-gobernanza/estado-del-proyecto.md`: nombre, ADR y estado de implementación.
- `00-gobernanza/registro-de-decisiones.md`: ADR-0009 y ADR-0010 registradas.
- `06-herramientas-ecosistema/contrato-de-cli.md`: contrato implementado y pendientes.
- `INDICE-Y-MAPA-DE-COBERTURA.md`: estado real y CLI.
- `00-gobernanza/adr/ADR-0009-identidad-basilisco.md`.
- `00-gobernanza/adr/ADR-0010-cli-y-extension.md`.

## Comandos verificados

```text
bsk --version
bsk run examples/funciones_basicas.bsk
bsk run tests/fixtures/legado.nexo
bsk run tests/fixtures/legado.nx   # error esperado, código 2
```

## Pendientes

- Licencia y metadatos de publicación.
- Verificación macOS/Linux (el CI actual es Ubuntu).
- Búsqueda formal de marca, dominios y registros.
- Detección de conflicto de `bsk` con BrowserSkill en instalación.
- Política de MIME/asociación para `.bsk` (Nero/Bryce).
- Compatibilidad `.nx`: solo si aparece evidencia de archivos externos.
- Módulos, colecciones, E/S general, entrada estándar y herramientas.
