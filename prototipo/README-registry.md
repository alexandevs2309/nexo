# BASILISCO Registry (R6, work in progress)

El registro es un binario independiente; no agrega comandos al intérprete `bsk`.

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

Este binario y el despliegue son scaffolding de R6, no una implementación
lista para producción. La API actual no tiene autenticación/autorización de
publicación, el endpoint de publicación previo acepta metadatos criptográficos
del cliente sin verificar una clave de confianza, la validación BPK no cubre
todo el perfil R2, y no hay integración probada desde los clientes R4/R5. No
exponga este servicio a redes no confiables. Estas brechas impiden declarar R6
cerrada; están documentadas aquí deliberadamente para no confundir el esqueleto
con un registro seguro y conforme.
