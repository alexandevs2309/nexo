# Paquetes y dependencias (Fase 12)

Estado: **implementación verificada en alcance Fase 12** (local + registro remoto en entorno controlado). No es una aprobación de fase; la aprobación/cierre corresponde al mantenedor humano.

## Manifiesto (`basilisco.toml`)

Cada proyecto declara identidad y dependencias:

```toml
[paquete]
nombre = "mi-proyecto"
version = "0.1.0"

[dependencias]
mate = "0.1.0"
util = { version = "0.2.0", ruta = "../util", autorizado = true }
remoto = { version = "1.0.0", registro = "official" }
```

Reglas verificadas:
- `[paquete].nombre` obligatorio: ASCII alfanumérico, `-`, `_`, máx. 64, empieza por alfanumérico.
- `[paquete].version` opcional (default `0.0.0`); validación de formato.
- Dependencias: versión string o tabla en línea (`version`, `ruta/path`, `hash`, `firma`, `clave`, `registro`, `nonce`, `expira`, `autorizado`).
- Versionado: exacto, `*`, y rangos SemVer compatibles con el motor (`^`, `~`, `>=`) donde aplique.
- Rechazo de tablas/claves inválidas, duplicados, valores malformados.

## Resolución e instalación

Orden local:
1. `ruta` explícita (relativa al manifiesto; rutas externas requieren `autorizado = true` y quedan acotadas por validación de ruta).
2. `<proyecto>/paquetes/<nombre>`.
3. `<proyecto>/vendor/<nombre>`.
4. `<proyecto>/<nombre>`.

Comportamiento:
- Resolución **transitiva** con detección de ciclos y conflictos de versión.
- Entrada de paquete: archivo `.bsk` o directorio con `lib.bsk`, `src/lib.bsk`, `principal.bsk` o `<nombre>.bsk`.
- Si el paquete tiene `basilisco.toml`, se valida nombre/versión declarada.
- Dependencias con `registro` pueden resolverse vía registro remoto configurado (flujo verificado en pruebas controladas).

## Bloqueo (`basilisco.lock`)

`basilisco instalar` escribe bloqueo determinista (v1) con rutas portables relativas al proyecto y hash SHA-256. Campos de procedencia/autenticidad soportados cuando están disponibles: `registro`, `key_id`, `firma`, `nonce`, `expira`, `verificado`.

```toml
version = 1

[[dependencia]]
nombre = "mate"
version = "0.1.0"
ruta = "paquetes/mate/lib.bsk"
hash = "..."
registro = "official"
key_id = "key-1"
verificado = true
```

`importar "nombre"` consulta primero el bloqueo; si no existe, usa rutas convencionales.

## Integridad, autenticidad y seguridad

- **Integridad:** SHA-256 de artefacto/archivo; comparación obligatoria ante hash esperado. La divergencia rechaza instalación.
- **Autenticidad:** metadatos firmados con Ed25519; verificación contra almacén de claves confiables (`.basilisco/trust.json`) por `registry_id/key_id`. Claves revocadas no se aceptan.
- **Anti-replay:** nonce por registro persistido en `.basilisco/nonces.json`; nonce repetido rechazado.
- **BPK (R2):** ZIP restringido con `BPK-MANIFEST.toml`, `payload/`, rutas normalizadas NFC, sin cifrado/symlinks/entradas especiales, límites de tamaño/cantidad, verificación de tamaño+SHA-256 por archivo y rechazo de archivos no declarados/faltantes.
- **Extracción segura:** validación de rutas, rechazo de traversal, rechazo de symlinks en ruta de extracción, creación sin sobrescritura.
- **Transporte/registro:** HTTPS requerido para registros remotos no locales; localhost/127.0.0.1 permitido para pruebas aisladas. Credenciales no se incluyen en URL.

## Registro remoto

- Configuración local de registros: `basilisco registry add/list/remove/set-default/enable/disable/info` (`.basilisco/registries.json`).
- Cliente remoto: obtener metadatos, descargar artefacto, verificar hash, validar BPK, verificar firma/clave confiable/nonce.
- Servidor de registro (`basilisco-registry`, Axum+SQLx/PostgreSQL): auth/token, namespaces, publish, download, yanked, search, claves. Publicación valida BPK, hash, identidad manifiesto, SemVer, registry_id, expiración, autorización por namespace, unicidad de versión/nonce, almacenamiento atómico.
- Integración E2E PostgreSQL aislado: tests `registry_integration` (ignorados por defecto, ejecutables con `DATABASE_URL` aislado) verificados localmente con PostgreSQL en Docker.

## Comandos CLI (verificados)

```bash
basilisco instalar [ruta]
basilisco paquetes [ruta]
basilisco update [ruta]
basilisco add <paquete> [versión] [--ruta=<ruta>]
basilisco publish [ruta] [--clave=<hex32>] [--salida=<archivo>]
basilisco instalar-remoto <metadata-url> <artifact-url> <destino>
basilisco registry <add|list|remove|set-default|enable|disable|info> ...
```

## Limitaciones residuales

- La resolución remota automática en `instalar` se activa para dependencias con metadatos remotos/registro explícito; no se afirma cobertura total de todos los flujos de resolución remota posibles.
- El lock v1 conserva compatibilidad y campos de procedencia; la semántica completa de estados de verificación R1 (`integridad_verificada`, `autenticidad_verificada`, `autenticidad_pendiente`, `verificacion_fallida`) no está modelada como enum formal en el lock.
- R3 (distribución/rotación/revocación avanzada de claves, protocolo completo de firmas) no está implementado por completo; no se declara autenticidad verificada sin clave confiable.
- Publicación remota desde CLI está soportada a nivel de artefacto/cliente/servidor en piezas existentes; el flujo CLI puede requerir endurecimiento adicional según alcance operativo.
- `registry_integration` requiere PostgreSQL aislado; fuera de ese entorno permanece `#[ignore]` (no falso verde).
- No hay sandbox de ejecución para código de paquetes; instalar no ejecuta código, pero el consumo/ejecución de código del paquete sigue sujeto al modelo de seguridad general del lenguaje.
- No se declara soporte de registros reales públicos ni publicación en registros reales.
- Las rutas de lock/importación se resuelven respecto al directorio de trabajo actual.
