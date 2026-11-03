# BASILISCO Registry (R6 — cerrada)

El registro es un binario independiente; no agrega comandos al intérprete `bsk`.

## Estado

**R6 CERRADA** (2026-10-31). Las verificaciones documentadas en `10-especificacion/registro-de-brechas.md` confirman:

- Servidor Axum/SQLx operativo con autenticación Bearer (Argon2 + SHA-256).
- Namespaces con control de membresía.
- Publicación BPK con validación completa (SHA-256 por archivo, Ed25519, SemVer, expiración, ZIP safety, NFC).
- Cadena de confianza: bootstrap de clave Ed25519 fuera de banda; el servidor re-firma con su clave propia.
- Descarga, búsqueda y yank funcionales.
- Cliente `bsk instalar-remoto` verifica firma Ed25519 → `verificado=true`.
- Persistencia y recuperación ante reinicio del servidor y la BD verificadas.
- Test E2E automatizado `tests/registry_integration.rs` pasa contra PostgreSQL 16 real.
- `cargo test` → verde; `cargo clippy` y `cargo fmt --check` limpios.

**Limitaciones residuales que no impiden el uso prototipal:**
- Sin rate limiting ni auditoría de operaciones.
- `/health` no verifica conectividad con PostgreSQL.
- No apto para exposición a redes no confiables sin revisión de seguridad adicional.
- Requiere PostgreSQL; no hay modo embebido ni SQLite de desarrollo.


## Ejecución local

Requiere PostgreSQL. Configure `DATABASE_URL` y `BASILISCO_REGISTRY_ID`; el
servidor rechaza iniciar si falta cualquiera de los dos. `BASILISCO_BIND`
controla la dirección (por defecto `0.0.0.0:8080`) y `BASILISCO_ARTIFACTS`
controla el directorio persistente (por defecto `./artifacts`).

```sh
cargo run --bin basilisco-registry
```

Para desarrollo con Docker Compose, cree `.env` con valores locales propios:

```dotenv
POSTGRES_PASSWORD=use-un-secreto-local-largo
BASILISCO_REGISTRY_ID=registry-dev
```

```sh
docker compose -f compose.registry.yaml up --build
```

PostgreSQL y los artefactos usan volúmenes nombrados. El puerto publicado se
limita a loopback. El endpoint `/health` indica disponibilidad HTTP básica;
todavía no verifica la salud de PostgreSQL.

## Estado funcional

Ver sección **Estado** al inicio de este documento. R6 está cerrada con evidencia; las limitaciones residuales están listadas allí.
