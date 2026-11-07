# Sistema de plantillas BASILISCO

**Versión del documento:** 1.0  
**Fecha:** 2026-11-05  
**Estado:** DISEÑO — no implementado

---

## 1. Estado actual

Hoy `bsk nuevo proyecto <nombre>` genera una estructura mínima fija (sin opciones de plantilla). La selección de plantilla es una capacidad futura.

Lo que existe:
- `plantillas/main.bsk`: un archivo de ejemplo básico
- `prototipo/src/proyecto.rs:crear_proyecto()`: genera exactamente `src/main.bsk`, `src/lib.bsk`, `basilisco.toml`, `README.md`, `.gitignore`

Lo que no existe:
- Ningún sistema de selección, descarga o aplicación de plantillas
- Ningún formato de metadatos de plantilla
- Ningún repositorio de plantillas

---

## 2. Diseño del sistema (futuro)

### 2.1 Invocación

```bash
bsk nuevo proyecto mi-app                           # plantilla por defecto (consola)
bsk nuevo proyecto mi-api --plantilla rest-api      # plantilla nombrada
bsk nuevo proyecto mi-pos --plantilla pos           # sistema punto de venta
bsk nuevo proyecto mi-lib --plantilla biblioteca    # módulo/biblioteca
```

### 2.2 Fuentes de plantillas

**Nivel 1 — Plantillas integradas (bundled):** incluidas en el binario `bsk`. No requieren conexión a internet. Son las plantillas oficiales mantenidas por el proyecto BASILISCO.

**Nivel 2 — Plantillas locales:** una ruta en el sistema de archivos. `bsk nuevo proyecto nombre --plantilla /ruta/a/mi-plantilla`.

**Nivel 3 — Plantillas del registro (futuro):** descargadas desde el registro de paquetes bajo el namespace `plantillas/`. Requiere la misma infraestructura de paquetes BPK que ya existe.

---

## 3. Plantillas oficiales

Las plantillas oficiales se distribuyen con el binario y se mantienen compatibles con la versión actual del lenguaje.

| ID | Nombre | Descripción | Versión mínima de BASILISCO |
|---|---|---|---|
| `consola` | Aplicación de consola | Menú, E/S básica, funciones | 0.7.0 |
| `biblioteca` | Biblioteca/módulo | Funciones públicas, manifiesto, pruebas básicas | 0.7.0 |
| `inventario` | Sistema de inventario | Modelo de datos, persistencia JSON, menú | 0.7.0 |
| `rest-api` | API REST | Rutas HTTP, serialización JSON, manejo de errores | FUTURO (requiere stdlib HTTP) |
| `pos` | Punto de venta | Inventario, ventas, facturación, persistencia | FUTURO |

Las plantillas marcadas como FUTURO no deben generarse hasta que el lenguaje soporte sus dependencias (stdlib HTTP, tipos producto, persistencia estructurada).

---

## 4. Formato de una plantilla

Una plantilla es un directorio con la siguiente estructura:

```
plantilla-rest-api/
├── bsk-template.toml        # metadatos de la plantilla
├── {{nombre}}/              # directorio raíz del proyecto generado
│   ├── basilisco.toml
│   ├── src/
│   │   ├── main.bsk
│   │   ├── rutas/
│   │   │   └── usuarios.bsk
│   │   └── modelos/
│   │       └── usuario.bsk
│   └── README.md
└── pruebas/                 # pruebas de la propia plantilla
    └── verificar_generacion.sh
```

### 4.1 Metadatos (bsk-template.toml)

```toml
nombre = "rest-api"
descripcion = "API REST con endpoints JSON"
version = "1.0.0"
basilisco_minimo = "0.9.0"
autor = "Proyecto BASILISCO"
etiquetas = ["api", "http", "servidor"]

[variables]
nombre_proyecto = { descripcion = "Nombre del proyecto", defecto = "mi-api" }
puerto = { descripcion = "Puerto del servidor", defecto = "8080" }
base_datos = { descripcion = "Nombre de la base de datos", defecto = "miapp" }
```

### 4.2 Variables de personalización

Los archivos de la plantilla pueden contener marcadores `{{variable}}`:

```basilisco
# src/main.bsk generado para "mi-api" en puerto 3000
const NOMBRE_APP = "mi-api"
const PUERTO = 3000
```

El motor de plantillas sustituye `{{nombre_proyecto}}` → `"mi-api"` y `{{puerto}}` → `3000` en todos los archivos antes de escribirlos.

### 4.3 Validación de plantillas

Una plantilla es válida si:
1. Tiene `bsk-template.toml` con las claves obligatorias
2. La versión de BASILISCO requerida es menor o igual a la instalada
3. El proyecto generado compila sin errores (`bsk analizar` pasa)
4. El proyecto generado ejecuta sin errores (`bsk run` pasa) con los valores por defecto

Las plantillas se prueban automáticamente en CI antes de distribuirse.

---

## 5. Ciclo de vida de una plantilla

```
[Crear plantilla] → [Validar en CI] → [Incluir en binario bsk] → [Publicar versión bsk]
                                                    ↓
                                         [Actualizar plantilla]
                                                    ↓
                                   [Nueva versión bsk incluye nueva versión plantilla]
```

Las plantillas se versionan junto con el binario `bsk`. No hay actualización independiente de plantillas integradas.

Para las plantillas del registro (futuro), la actualización sigue el mismo ciclo de BPK (SHA-256 + Ed25519).

---

## 6. Compatibilidad

- Una plantilla para BASILISCO 0.7.0 no debe usar características de 0.9.0
- El campo `basilisco_minimo` en `bsk-template.toml` es obligatorio y se verifica en tiempo de generación
- Las plantillas no usan características marcadas como EXPERIMENTAL en la documentación oficial

---

## 7. Plantillas comunitarias (diseño futuro)

Las plantillas comunitarias son paquetes BPK en el registro bajo el namespace `plantillas/`. El proceso de publicación es idéntico al de paquetes normales.

```bash
bsk nuevo proyecto mi-app --plantilla comunidad/ecommerce@1.2.0
```

Para instalarse, una plantilla comunitaria debe:
- Tener `bsk-template.toml` válido
- Pasar validación de seguridad básica (no contiene llamadas FFI peligrosas, no referencia URLs externas hardcoded)
- Estar firmada con Ed25519 como cualquier paquete BPK

---

## 8. Dependencias de implementación

Para implementar este sistema se necesita (en orden):

1. Unificar el parser de `basilisco.toml` (GAP unificación TOML)
2. Definir el formato `bsk-template.toml` y el motor de sustitución de variables
3. Incluir al menos `consola` y `biblioteca` como plantillas integradas
4. Añadir `--plantilla <id>` al comando `bsk nuevo proyecto`
5. Añadir pruebas automáticas de generación en CI
6. Publicar las plantillas en el repositorio junto con sus pruebas

Los pasos 3-6 son la condición de cierre de GAP-PS-03 (ver registro de brechas).
