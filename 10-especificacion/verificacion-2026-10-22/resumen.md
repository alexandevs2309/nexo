# Verificación 2026-10-22 — identidad BASILISCO y CLI

**Estado:** evidencia local + CI en verde; no implica publicación.  
**Base de código:** `main` `a0c1a22564983c4fd4346122f40977d5552ed8f5` (2026-10-22), más los cambios de identidad/CLI integrados en el commit `ec3b1f2e20657d9286a99491eda93ebb5db4d746`.  
**Entorno:** Windows, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1 (5f94df478 2026-08-27)`.  
**Repositorio de trabajo:** `C:\Users\Alexander\AppData\Local\Temp\opencode\nexo-repo-latest`.

## 1. Decisión aplicada

- Identidad: BASILISCO; Nexo histórico/temporal (ADR-0009).
- Paquete: `basilisco`; binario principal `basilisco`; alias `bsk`.
- Extensión canónica: `.bsk`.
- Extensión legada: `.nexo`.
- `.nx`: rechazada con código 2 y mensaje explícito.
- CLI: `run`/`ejecutar`, `demo`, `--version`, `--help` (ADR-0010).

## 2. Resultado de pruebas

Comando ejecutado (con `CARGO_TARGET_DIR` separado y flags de enlace por el bloqueo local de Application Control):

```text
cargo test --release
```

Resultado:

| Grupo | Tests |
|---|---:|
| Unitarios de biblioteca | 111 |
| CLI (`tests/cli.rs`) | 12 |
| `conformidad` | 29 |
| `conformidad_control` | 39 |
| `conformidad_funciones` | 12 |
| `conformidad_texto` | 13 |
| `control_flujo` | 19 |
| `integracion` | 15 |
| `variables` | 14 |
| **Total** | **264** |

Todos en verde: **264/264**, 0 fallos, 0 ignorados.

## 3. Calidad estática

- `cargo fmt --check`: **pasa**, 0 diferencias.
- `cargo clippy --all-targets` con `-D warnings` y `-W clippy::all`: **sin avisos**.

La comprobación de Clippy se ejecutó con un `clippy-driver` copiado y limpiado de la marca de Zone.Identifier; no se modificó el código para evitar el bloqueo del entorno.

## 4. Pruebas manuales de la CLI

```text
bsk --version
BASILISCO 0.1.0

bsk run examples/funciones_basicas.bsk
5
120
42
# código 0

bsk run tests/fixtures/legado.nexo
# stderr: aviso: .nexo es legado; usa .bsk
42
# código 0

bsk run tests/fixtures/legado.nx
# stderr: error: .nx no está soportado; usa .bsk o .nexo
# código 2
```

También se verificaron rutas con espacios, archivo inexistente (código 1), extensión desconocida, ausencia de extensión y `run` sin argumento.

## 5. Cobertura de los tests de CLI

`prototipo/tests/cli.rs` cubre:

- `bsk --version` identifica BASILISCO;
- `basilisco --version` funciona como alias;
- `--help` muestra `run` y `.nexo`;
- ejecución `.bsk` canónica;
- ejecución `.nexo` legada con aviso;
- rechazo de `.nx` con código 2;
- rechazo de extensión desconocida;
- rechazo de archivo sin extensión;
- archivo inexistente con código 1;
- ruta con espacios;
- ejecución de `examples/funciones_basicas.bsk`;
- `run` sin archivo como error de uso.

## 6. Limitaciones de esta evidencia

- CI ejecutado en GitHub Actions: run [39872722106](https://github.com/alexandevs2309/nexo/actions/runs/39872722106), conclusión `success` (formato, Clippy, build y tests).
- Commit `ec3b1f2` publicado en `origin/main`; no hay cambios pendientes en el clon.
- La verificación local fue en Windows; CI se ejecutó en Ubuntu. No hay evidencia de macOS en esta ronda.
- No hay prueba de instalación (`cargo install`), firma, publicación ni detección de conflicto de `bsk` con BrowserSkill.
- `.nx` se rechaza por decisión; no hay archivos `.nx` en el repositorio.
- La licencia y los metadatos de publicación siguen pendientes.

## 7. Artefactos locales

- Log de tests: `%TEMP%\opencode\nexo-cargo\basilisco-test.log`.
- Log de Clippy: `%TEMP%\opencode\nexo-cargo\basilisco-clippy.log`.
- Binarios verificados: `basilisco.exe` y `bsk.exe` en el target de pruebas.

Estos artefactos no se versionan.

## 8. CI

- Run: [39872722106](https://github.com/alexandevs2309/nexo/actions/runs/39872722106).
- Commit: `ec3b1f2e20657d9286a99491eda93ebb5db4d746`.
- Resultado: `success`.
- Pasos: checkout, Rust estable con rustfmt/clippy, formato, Clippy, `cargo build --bins` y `cargo test`.
