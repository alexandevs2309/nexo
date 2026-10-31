# BASILISCO PACKAGE REGISTRY
## Especificación técnica del protocolo remoto — R1

**Proyecto:** BASILISCO
**Ecosistema:** BASILISCO Package Ecosystem
**Versión del protocolo:** 1.0
**Estado:** Especificación arquitectónica inicial
**Alcance:** Diseño, sin implementación.

---

# 1. Objetivo

Definir un protocolo estándar para descubrir, resolver, descargar, instalar y publicar paquetes del lenguaje BASILISCO.

El protocolo debe permitir que el gestor local existente evolucione hacia un ecosistema distribuido sin tener que reconstruir su motor de resolución de dependencias.

Principios fundamentales:

- Compatibilidad con el gestor local existente.
- Registros múltiples.
- Versionado semántico.
- Distribución reproducible.
- Integridad verificable.
- Autenticidad de publicaciones.
- Independencia del proveedor de infraestructura.
- Seguridad desde el diseño.

# 2. Arquitectura general

El ecosistema estará compuesto por cuatro subsistemas independientes.

## 2.1 Package Client

Cliente utilizado por la CLI de BASILISCO.

Responsabilidades:

- Consultar registros.
- Buscar paquetes.
- Obtener metadatos.
- Resolver dependencias remotas.
- Descargar paquetes.
- Verificar integridad.
- Gestionar autenticación.
- Publicar paquetes.

## 2.2 Package Engine

Motor de paquetes existente.

Responsabilidades:

- Resolución de dependencias.
- SemVer.
- Dependencias transitivas.
- Detección de ciclos.
- Detección de conflictos.
- Lock file.
- Caché local.

El motor deberá conservar su independencia respecto al transporte HTTP y al servidor remoto.

## 2.3 Registry API

Servicio encargado de:

- Consultas de paquetes.
- Información de versiones.
- Autenticación.
- Autorización de publicaciones.
- Registro de nuevas versiones.
- Distribución de metadatos.

## 2.4 Package Storage

Almacenamiento independiente para los archivos de paquetes.

El registro podrá utilizar almacenamiento local, almacenamiento de objetos o servicios externos sin modificar el protocolo del cliente.

# 3. Registros múltiples

BASILISCO permitirá utilizar múltiples registros independientes.

Registro oficial predeterminado:

`https://registry.basilisco.dev`

Los registros podrán identificarse mediante alias.

Ejemplo:

```toml
[registries]
default = "official"

[registries.official]
url = "https://registry.basilisco.dev"

[registries.community]
url = "https://packages.example.org"
```

Cada registro tendrá su propia identidad, configuración y claves de confianza.

El cliente no deberá asumir que dos registros diferentes son igualmente confiables.

# 4. Identidad de paquetes

Formato canónico:

`namespace/name@version`

Ejemplos:

```text
auron/http@1.2.0
auron/database@2.0.1
community/json@1.4.3
```

Cada paquete tendrá:

- Namespace.
- Nombre.
- Versión.
- Autor o propietario.
- Descripción.
- Licencia.
- Compatibilidad con BASILISCO.
- Dependencias.
- Hash del contenido.
- Fecha de publicación.
- Estado de publicación.

Las versiones publicadas serán inmutables.

No se permitirá sustituir el contenido de una versión ya publicada.

# 5. Manifiesto del proyecto

Se conservará el formato TOML actual.

Ejemplo:

```toml
[package]
name = "mi-aplicacion"
version = "1.0.0"

[registries]
default = "official"

[dependencies]
http = "^1.2.0"

json = {
    version = "^2.0.0",
    registry = "community"
}

util = {
    version = "0.2.0",
    ruta = "../util",
    autorizado = true
}
```

Las dependencias locales seguirán funcionando.

Las dependencias remotas tendrán un registro de origen explícito o heredarán el registro predeterminado.

# 6. Protocolo de comunicación

Protocolo:

- HTTP sobre TLS.
- API REST.
- JSON para metadatos.
- Archivos binarios para paquetes.
- Versionado obligatorio de API.

URL base:

`https://registry.basilisco.dev/api/v1`

Todas las respuestas JSON deberán utilizar UTF-8.

# 7. Endpoints oficiales

## 7.1 Consulta de paquetes

| Método | Endpoint | Descripción |
|---|---|---|
| GET | `/packages/{namespace}/{name}` | Información general |
| GET | `/packages/{namespace}/{name}/versions` | Versiones disponibles |
| GET | `/packages/{namespace}/{name}/{version}` | Metadatos de versión |
| GET | `/packages/{namespace}/{name}/{version}/download` | Descarga |
| GET | `/search?q={query}` | Búsqueda |

## 7.2 Publicación

| Método | Endpoint | Descripción |
|---|---|---|
| POST | `/packages` | Iniciar publicación |
| PUT | `/packages/{namespace}/{name}/{version}/upload` | Enviar archivo |
| POST | `/packages/{namespace}/{name}/{version}/publish` | Confirmar publicación |

El proceso de publicación deberá validar identidad, permisos, manifiesto, versión, contenido e integridad antes de hacer visible el paquete.

La implementación podrá utilizar carga directa o almacenamiento temporal, siempre que respete este contrato lógico.

## 7.3 Autenticación

| Método | Endpoint | Descripción |
|---|---|---|
| POST | `/auth/token` | Obtener credenciales de acceso |
| POST | `/auth/revoke` | Revocar credenciales |
| GET | `/users/me` | Consultar identidad |

La autenticación será necesaria para publicar paquetes y administrar propietarios.

Las consultas públicas podrán realizarse sin autenticación, sujetas a límites de uso.

# 8. Formato de respuestas JSON

Las respuestas exitosas deberán mantener una estructura consistente.

Ejemplo:

```json
{
  "success": true,
  "data": {
    "name": "http",
    "namespace": "auron",
    "version": "1.2.0"
  },
  "error": null
}
```

Las respuestas de error:

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "PACKAGE_NOT_FOUND",
    "message": "Package not found"
  }
}
```

Códigos iniciales:

- `PACKAGE_NOT_FOUND`
- `VERSION_NOT_FOUND`
- `VERSION_ALREADY_EXISTS`
- `INVALID_MANIFEST`
- `INVALID_PACKAGE`
- `INVALID_HASH`
- `INVALID_SIGNATURE`
- `UNAUTHORIZED`
- `FORBIDDEN`
- `RATE_LIMITED`
- `REGISTRY_UNAVAILABLE`
- `INTERNAL_ERROR`

Los errores deberán acompañarse del código HTTP apropiado.

# 9. Metadatos de una versión

Cada versión deberá exponer como mínimo:

```json
{
  "namespace": "auron",
  "name": "http",
  "version": "1.2.0",
  "description": "HTTP client for BASILISCO",
  "license": "Apache-2.0",
  "authors": ["AURON"],
  "basilisco_version": ">=0.6.0",
  "dependencies": {},
  "artifact": {
    "filename": "http-1.2.0.bpk",
    "size": 24576,
    "sha256": "HASH_SHA256",
    "media_type": "application/vnd.basilisco.package"
  },
  "signature": null,
  "published_at": "2026-10-31T00:00:00Z",
  "yanked": false
}
```

La estructura definitiva de firmas se especificará en R3.

# 10. Distribución de paquetes

Formato de distribución previsto:

`.bpk`

El paquete será un contenedor de archivos con una estructura interna definida en R2.

El registro tratará el archivo como un artefacto binario inmutable.

El cliente deberá:

1. Descargar el paquete.
2. Verificar el tamaño.
3. Calcular SHA-256.
4. Comparar con el hash esperado obtenido de metadatos confiables.
5. Validar su estructura.
6. Verificar las rutas de sus archivos.
7. Instalarlo en el directorio autorizado.

Un paquete no podrá escribir fuera de su directorio de instalación.

No se ejecutará código arbitrario durante la instalación.

# 11. Versionado

Se utilizará Semantic Versioning.

Ejemplos:

- `1.0.0`
- `1.1.0`
- `1.1.1`
- `2.0.0`

Se admitirán los rangos ya soportados por el gestor local:

- Exacto.
- `^`
- `~`
- `>=`
- `*`

Las reglas de resolución deberán ser compatibles con el motor existente.

El registro podrá marcar una versión como retirada mediante `yanked`, sin eliminar su historial ni modificar su contenido.

`yanked` es un estado administrado por el registro, no una propiedad intrínseca del archivo BPK. El registro debe informar ese estado en sus metadatos. El cliente debe respetarlo durante la resolución de dependencias. Una versión retirada no debe desaparecer del historial ni invalidar instalaciones existentes. Si se fija explícitamente una versión retirada, la instalación puede continuar; al resolver dependencias nuevas, el registro puede evitar seleccionar versiones retiradas.

# 12. Lock file

El lock file deberá evolucionar para registrar:

- Registro de procedencia (`registry`).
- Namespace.
- Nombre.
- Versión exacta.
- URL o identificador de origen (`source`).
- SHA-256 esperado (`checksum`).
- Dependencias resueltas con versiones exactas.
- Estado de verificación (`verification`).

El lock file permitirá repetir instalaciones con las mismas versiones y contenidos.

No será considerado por sí solo una fuente de autenticidad.

La confianza deberá depender de referencias verificables obtenidas de fuentes autorizadas.

Distinguir:
- Integridad: el artefacto coincide con el SHA-256 esperado.
- Autenticidad: la firma corresponde a una clave pública confiable.
- Procedencia: identifica de dónde se obtuvo el paquete.

Estados de verificación del lock file:
- `integridad_verificada`: SHA-256 coincide.
- `autenticidad_verificada`: firma válida con clave confiable.
- `autenticidad_pendiente`: sin firma o sin clave confiable.
- `verificacion_fallida`: integridad o autenticidad falló.

La coincidencia SHA-256 no demuestra autenticidad. La autenticidad no puede marcarse como comprobada sin una clave pública confiable. Un error de integridad debe impedir aceptar o instalar el artefacto. La ausencia de firma o de una clave confiable debe producir un estado explícito (`autenticidad_pendiente`), nunca una autenticidad asumida.

Ed25519 es el algoritmo criptográfico previsto. R1 establece el requisito arquitectónico de autenticidad. R3 definirá el protocolo concreto de firmas, claves, rotación, revocación y validación. Hasta que R3 esté especificada e implementada, no se debe afirmar que las publicaciones tienen autenticidad verificada. La verificación de integridad SHA-256 es independiente de la autenticación criptográfica.

# 13. Seguridad

Se establecen los siguientes requisitos:

## Transporte

- HTTPS obligatorio para registros remotos.
- Validación de certificados TLS.
- No transmitir credenciales en URL.

## Integridad

- SHA-256 obligatorio para los artefactos publicados.
- Comparación antes de instalar.
- Verificación antes de reutilizar la caché.

## Autenticidad

- Firmas Ed25519.
- Claves públicas de confianza por registro.
- Verificación de firmas antes de aceptar metadatos autenticados.
- Protección de claves privadas de publicación.

Requisitos conceptuales pendientes (R3):
- Identidad de cada registro.
- Identificador de clave pública.
- Distribución de claves públicas confiables.
- Rotación y revocación de claves.
- Relación entre la firma y el artefacto publicado.
- Comportamiento cuando una clave no es confiable o ha sido revocada.

## Distribución

- Versiones inmutables.
- Validación de archivos.
- Protección contra traversal.
- Protección contra enlaces simbólicos maliciosos.
- Límites de tamaño y cantidad de archivos.

La especificación criptográfica detallada se desarrollará en R3.

# 14. Independencia de infraestructura

El protocolo no dependerá de:

- Un proveedor cloud concreto.
- Un sistema operativo concreto.
- Un lenguaje de programación concreto para el servidor.
- Un proveedor específico de almacenamiento.

El cliente BASILISCO deberá poder comunicarse con cualquier servidor que implemente correctamente la API.

# 15. Compatibilidad con el gestor actual

La integración deberá conservar:

- Resolución transitiva.
- Detección de ciclos.
- Conflictos de versiones.
- Rangos SemVer.
- Lock file.
- Caché.
- Dependencias locales.

El cliente remoto proporcionará metadatos y artefactos al motor existente.

No se implementará un segundo resolver independiente.

# 16. Hoja de ruta del ecosistema

| Etapa | Objetivo |
|---|---|
| R1 | Protocolo HTTP y contratos JSON |
| R2 | Formato interno `.bpk` |
| R3 | Seguridad, autenticación y firmas |
| R4 | Integración con el gestor local |
| R5 | Cliente de registros múltiples |
| R6 | Servidor de registro |
| R7 | Publicación y distribución |
| R8 | Búsqueda pública y administración |

# 17. Criterios de aceptación de R1

La especificación R1 se considerará completa cuando:

- La API tenga endpoints definidos.
- Las estructuras JSON estén documentadas.
- Los errores estén normalizados.
- La identidad de paquetes esté definida.
- Los registros múltiples estén contemplados.
- El proceso lógico de publicación esté descrito.
- El proceso lógico de descarga esté descrito.
- La compatibilidad con el motor existente esté garantizada como requisito.
- Las responsabilidades del cliente y servidor estén separadas.

# 18. Restricciones de implementación

Esta especificación no autoriza por sí sola:

- Implementar el servidor.
- Cambiar la arquitectura del gestor existente.
- Sustituir el lock file.
- Introducir dependencias externas.
- Crear un formato binario sin especificación R2.
- Implementar autenticación sin R3.

Toda implementación deberá respetar esta especificación o documentar formalmente cualquier cambio aprobado.

---

**Fin de la especificación R1.**

**BASILISCO — Language and Package Ecosystem**
