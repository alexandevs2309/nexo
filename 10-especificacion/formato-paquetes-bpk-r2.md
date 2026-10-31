# BASILISCO PACKAGE FORMAT
## Especificación técnica del formato BPK — R2

**Proyecto:** BASILISCO
**Ecosistema:** BASILISCO Package Ecosystem
**Formato:** `.bpk`
**Versión del formato:** 1.0
**Estado:** Especificación arquitectónica
**Dependencia:** Protocolo de registro remoto R1
**Alcance:** Diseño, sin implementación.

---

# 1. Objetivo

Definir un formato estándar para distribuir paquetes del lenguaje BASILISCO.

El formato BPK deberá permitir:

- Empaquetar bibliotecas y módulos BASILISCO.
- Distribuir paquetes mediante registros remotos.
- Verificar la integridad de los archivos.
- Reproducir instalaciones.
- Instalar paquetes de manera segura.
- Mantener compatibilidad entre versiones.
- Permitir inspección del contenido sin ejecutar código.

El formato deberá ser independiente del sistema operativo.

# 2. Decisión de arquitectura

BPK será un formato de distribución propio de BASILISCO basado en un contenedor ZIP restringido.

Extensión:

`.bpk`

Tipo de contenido:

`application/vnd.basilisco.package`

El formato tendrá identidad y reglas propias, aunque internamente utilice ZIP.

No se implementará un algoritmo de compresión propio.

La implementación utilizará una biblioteca ZIP compatible con el perfil definido en esta especificación.

# 3. Estructura interna

Un paquete BPK tendrá la siguiente estructura:

```text
http-1.2.0.bpk
│
├── BPK-MANIFEST.toml
│
└── payload/
    ├── src/
    │   ├── lib.bsk
    │   └── client.bsk
    │
    ├── examples/
    │   └── basic.bsk
    │
    ├── README.md
    └── LICENSE
```

## 3.1 BPK-MANIFEST.toml

Archivo obligatorio situado en la raíz del contenedor.

Contiene:

- Identidad del paquete.
- Versión.
- Versión del formato BPK.
- Compatibilidad con BASILISCO.
- Dependencias.
- Archivos incluidos.
- Tamaño de cada archivo.
- SHA-256 de cada archivo.

Las rutas declaradas en `BPK-MANIFEST.toml` son relativas a `payload/`. No incluyen el prefijo `payload/`. Cada entrada del manifiesto corresponde exactamente a un archivo del contenedor. El manifiesto está fuera de `payload/` y no se incluye a sí mismo en la lista de archivos. La validación compara las rutas normalizadas del manifiesto con las entradas reales del ZIP.

## 3.2 payload/

Contiene exclusivamente los archivos que se distribuirán con el paquete.

No se permitirá incluir archivos ejecutables de instalación ni scripts que se ejecuten automáticamente al instalar.

Los directorios se inferirán a partir de las rutas de los archivos.

# 4. Manifiesto oficial

Ejemplo:

```toml
[format]
name = "BASILISCO_PACKAGE"
version = "1.0"

[package]
namespace = "auron"
name = "http"
version = "1.2.0"
description = "HTTP client for BASILISCO"
license = "Apache-2.0"
authors = ["AURON"]

[compatibility]
basilisco = ">=0.6.0"

[dependencies]
json = "^2.0.0"

[[files]]
path = "src/lib.bsk"
size = 4096
sha256 = "HASH_SHA256"

[[files]]
path = "src/client.bsk"
size = 8192
sha256 = "HASH_SHA256"

[[files]]
path = "README.md"
size = 1024
sha256 = "HASH_SHA256"
```

Los valores de los hashes del ejemplo son ilustrativos.

El manifiesto deberá validarse antes de aceptar el paquete.

Las dependencias pueden declarar un registro explícito:

```toml
[dependencies]
json = { version = "^2.0.0", registry = "community" }
```

El registro puede ser explícito o heredarse del registro predeterminado del proyecto. El identificador de registro debe resolverse mediante la configuración del cliente. El paquete BPK conserva la identidad y las restricciones de la dependencia, no las credenciales del usuario. La resolución final debe quedar fijada con versiones exactas y procedencia en el lock file. Las dependencias locales existentes deben conservar su compatibilidad.

# 5. Identidad y versionado

La identidad seguirá el modelo definido en R1:

`namespace/name@version`

Ejemplo:

`auron/http@1.2.0`

Reglas:

- Namespace obligatorio.
- Nombre obligatorio.
- Versión SemVer obligatoria.
- La versión publicada será inmutable.
- No se permitirán dos contenidos diferentes para la misma identidad.
- Los paquetes locales podrán construirse antes de registrarse remotamente.

BPK es independiente del registro que lo distribuye. Un mismo paquete BPK puede distribuirse desde diferentes registros. La identidad del paquete no debe depender de su URL de descarga. La procedencia y el estado de publicación (incluido `yanked`) pertenecen a los metadatos externos del registro y al lock file, no al contenido intrínseco del artefacto. `yanked` no es una propiedad del archivo BPK.

La versión del formato BPK será independiente de la versión del paquete.

Por ejemplo:

- Formato BPK: `1.0`
- Paquete: `1.2.0`

# 6. Perfil ZIP permitido

El contenedor interno utilizará ZIP con un perfil restringido.

Requisitos:

- Nombres de archivos UTF-8.
- Compresión Store o Deflate.
- Sin cifrado ZIP.
- Sin archivos multidisco.
- Sin entradas duplicadas.
- Sin rutas absolutas.
- Sin rutas relativas con segmentos `.` o `..`.
- Sin enlaces simbólicos.
- Sin enlaces duros.
- Sin archivos especiales de dispositivos o sockets.
- Sin archivos fuera de `payload/`, excepto el manifiesto obligatorio.

ZIP64 podrá admitirse cuando sea necesario para tamaños mayores, sujeto a los límites de recursos configurados por el cliente.

El cliente deberá rechazar métodos de compresión no admitidos.

# 7. Normalización de rutas

Todas las rutas internas se expresarán utilizando `/`, independientemente del sistema operativo.

Ejemplos válidos:

```text
src/lib.bsk
examples/basic.bsk
docs/installation.md
```

Ejemplos rechazados:

```text
../../etc/passwd
/etc/passwd
C:\Windows\system.ini
src/../secret.bsk
```

También se rechazarán:

- Caracteres de control.
- Rutas vacías.
- Nombres reservados incompatibles con sistemas operativos soportados.
- Colisiones de nombres después de normalización.
- Rutas duplicadas.
- Rutas que puedan escapar del directorio de instalación.

Las rutas Unicode deberán normalizarse a NFC.

El cliente deberá comprobar las colisiones considerando las reglas de nombres del sistema operativo de destino.

# 8. Integridad de archivos

Cada archivo dentro de `payload/` tendrá:

- Ruta relativa.
- Tamaño esperado.
- SHA-256 esperado.

Durante la creación del paquete se calcularán los hashes de todos los archivos.

Durante la instalación:

1. Se abrirá el contenedor.
2. Se validará su estructura.
3. Se leerá el manifiesto.
4. Se comprobará la correspondencia entre entradas y archivos declarados.
5. Se verificará el tamaño de cada archivo.
6. Se calculará SHA-256 de cada archivo extraído.
7. Se comparará con el valor declarado.

Se rechazará el paquete si:

- Falta un archivo declarado.
- Existe un archivo no declarado.
- El tamaño no coincide.
- El hash no coincide.
- Existen entradas duplicadas.
- La estructura es inválida.

El hash externo del archivo `.bpk` será proporcionado por los metadatos del registro remoto definidos en R1.

El manifiesto interno permite verificar el contenido, pero no constituye por sí solo una prueba de autenticidad.

# 9. Verificación del contenedor

El cliente deberá verificar:

- Estructura ZIP.
- Directorio central.
- Encabezados locales.
- CRC32 proporcionado por ZIP.
- Método de compresión.
- Tamaños declarados.
- Correspondencia entre manifiesto y entradas.
- Integridad SHA-256 de los archivos.

No se confiará únicamente en CRC32 para verificar la integridad.

SHA-256 será el mecanismo obligatorio de comprobación de contenido.

# 10. Protección contra paquetes maliciosos

El instalador deberá protegerse contra:

## Path traversal

Rechazar cualquier archivo cuya ruta pueda escapar del directorio de instalación.

## ZIP bombs

Establecer límites configurables para:

- Tamaño comprimido.
- Tamaño total descomprimido.
- Tamaño máximo por archivo.
- Cantidad máxima de archivos.
- Relación de compresión.
- Profundidad de directorios.

Los límites deberán ser verificables antes y durante la extracción.

## Entradas duplicadas

Rechazar nombres repetidos o equivalentes después de normalizar rutas.

## Enlaces simbólicos

Rechazar entradas ZIP que representen enlaces simbólicos.

## Archivos inesperados

Rechazar entradas que no estén declaradas en el manifiesto.

## Extracción insegura

No extraer directamente utilizando rutas sin validar.

El instalador deberá controlar el directorio de destino y evitar que las operaciones de extracción sigan enlaces simbólicos externos.

# 11. Reproducibilidad

Dos construcciones del mismo paquete, a partir de los mismos archivos y configuración, deberán producir un contenido lógico idéntico.

Para favorecer la reproducibilidad:

- Ordenar las entradas por ruta normalizada.
- Utilizar timestamps normalizados.
- Normalizar permisos.
- Evitar metadatos del sistema local.
- Utilizar reglas deterministas de compresión.
- Mantener estable el manifiesto.
- No incluir rutas absolutas del equipo de construcción.

La reproducibilidad binaria exacta dependerá de la versión del compresor y de la implementación del empaquetador.

La especificación deberá distinguir entre igualdad de contenido y coincidencia byte a byte del contenedor.

# 12. Proceso de construcción

El futuro comando:

`basilisco package`

Deberá:

1. Leer el manifiesto del proyecto.
2. Validar nombre, versión y dependencias.
3. Identificar los archivos permitidos.
4. Rechazar archivos fuera de los límites autorizados.
5. Calcular tamaños y hashes.
6. Generar BPK-MANIFEST.toml.
7. Construir el contenedor ZIP.
8. Validar el paquete resultante.
9. Calcular SHA-256 del archivo BPK completo.
10. Presentar un resumen de construcción.

El empaquetador no deberá publicar automáticamente el resultado.

# 13. Proceso de instalación

El futuro instalador deberá:

1. Recibir un paquete BPK.
2. Verificar su hash externo cuando exista una referencia confiable.
3. Validar el contenedor.
4. Leer el manifiesto.
5. Verificar identidad y versión.
6. Validar compatibilidad.
7. Validar dependencias.
8. Verificar hashes internos.
9. Validar rutas.
10. Extraer de manera segura.
11. Registrar el paquete en el gestor local.
12. Actualizar el lock file.

No se ejecutará código del paquete durante la instalación.

La instalación deberá poder revertirse si falla antes de finalizar.

# 14. Compatibilidad futura

La versión del formato se incrementará cuando existan cambios incompatibles.

La versión 1.0 será la primera versión estable del formato.

Los lectores podrán admitir varias versiones del formato en el futuro.

Los paquetes no deberán depender de características no soportadas por la versión mínima de BASILISCO declarada.

# 15. Relación con el protocolo R1

R1 define:

- Comunicación con registros.
- Metadatos.
- Descarga.
- Publicación.
- Autenticación.
- Versiones.

R2 define:

- Estructura interna del paquete.
- Manifiesto.
- Archivos.
- Hashes.
- Compresión.
- Extracción segura.

Ambas especificaciones son complementarias.

# 16. Criterios de aceptación de R2

La especificación estará completa cuando:

- El formato BPK tenga estructura definida.
- El manifiesto tenga campos obligatorios definidos.
- El perfil ZIP esté restringido.
- Las rutas tengan reglas de validación.
- La integridad esté definida.
- La extracción segura esté especificada.
- Existan límites contra ZIP bombs.
- La reproducibilidad esté documentada.
- La compatibilidad con R1 esté establecida.

# 17. Restricciones de implementación

Este documento no autoriza todavía:

- Implementar el empaquetador.
- Implementar el instalador BPK.
- Cambiar el gestor local.
- Introducir dependencias ZIP.
- Implementar firmas.
- Implementar autenticación.
- Implementar el registro remoto.

La implementación requerirá una revisión y aprobación independiente.

---

**Fin de la especificación R2.**

**BASILISCO — Package Format Specification**
