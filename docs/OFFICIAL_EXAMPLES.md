# Catálogo de aplicaciones de ejemplo — BASILISCO

**Versión del documento:** 1.0  
**Fecha:** 2026-11-05  
**Estado:** PARCIALMENTE IMPLEMENTADO — ver estado por ejemplo

---

## Principio

Los ejemplos oficiales deben ser ejecutables, no decorativos. Cada ejemplo:

- Compila y ejecuta con la versión actual de BASILISCO sin modificaciones
- Demuestra funcionalidades reales del lenguaje en combinación
- Tiene instrucciones de ejecución verificadas
- No usa capacidades marcadas como FUTURO o EXPERIMENTAL sin advertirlo
- Se mantiene compatible con cada nueva versión del lenguaje

---

## 1. Aplicación de consola (consola-basica)

**Ruta:** `ejemplos/consola-basica/` (AUSENTE — pendiente de crear)  
**Estado:** AUSENTE  
**Capacidades demostradas:** funciones, variables, control de flujo, entrada de usuario, bucles

### Descripción

El programa más simple que demuestra el ciclo completo: entrada, procesamiento, salida. Sin dependencias externas.

```bash
bsk run ejemplos/consola-basica/main.bsk
```

**Estructura objetivo:**
```
ejemplos/consola-basica/
├── basilisco.toml
├── README.md
└── src/
    └── main.bsk
```

---

## 2. Sistema de inventario (inventario)

**Ruta:** `ejemplos/inventario/` — PARCIALMENTE IMPLEMENTADO  
**Estado:** FUNCIONAL (archivo único, no modular)  
**Capacidades demostradas:** funciones, JSON, persistencia a archivo, bucles, menú de consola

### Estado actual verificado

El archivo `ejemplos/inventario/main.bsk` existe y es funcional. Ejecutar desde el directorio `prototipo/`:

```bash
cargo run -- run ../ejemplos/inventario/main.bsk
```

El programa permite listar, agregar, editar y eliminar productos con persistencia en `ejemplos/inventario/datos/inventario.json`.

### Limitaciones actuales

- Implementado como un único archivo monolítico (`main.bsk`)
- No sigue la estructura modular oficial (modelos/servicios/repositorios)
- Sin pruebas automáticas
- Sin `basilisco.toml` propio
- La interfaz de usuario es texto plano, no una CLI estructurada

### Evolución prevista (FUTURO)

La arquitectura modular objetivo:

```
ejemplos/inventario/
├── basilisco.toml
├── README.md
├── src/
│   ├── main.bsk
│   ├── modelos/
│   │   └── producto.bsk     # tipo Producto { id, nombre, precio, existencia }
│   ├── servicios/
│   │   └── inventario.bsk   # lógica: agregar, eliminar, buscar, actualizar
│   ├── repositorios/
│   │   └── almacen.bsk      # leer/escribir JSON
│   └── interfaz/
│       └── menu.bsk         # menú interactivo
├── pruebas/
│   └── prueba_inventario.bsk
└── datos/
    └── inventario.json
```

Esta arquitectura requiere: tipos producto con nombre (GAP-035), módulos jerárquicos (GAP-047), marco de pruebas BASILISCO (GAP-PS-05). No debe implementarse antes de que esas capacidades estén disponibles.

---

## 3. Sistema POS (pos)

**Ruta:** `ejemplos/pos/` (AUSENTE)  
**Estado:** AUSENTE  
**Capacidades demostradas:** modelos de dominio, relaciones entre entidades, cálculos, persistencia

### Descripción

Sistema de punto de venta con gestión de productos, ventas y facturación básica.

### Prerrequisitos para implementar

- Tipos producto con nombre (GAP-035)
- Módulos jerárquicos
- Marco de pruebas

### Estructura objetivo

```
ejemplos/pos/
├── basilisco.toml
├── src/
│   ├── main.bsk
│   ├── modelos/
│   │   ├── producto.bsk
│   │   ├── venta.bsk
│   │   └── factura.bsk
│   ├── servicios/
│   │   ├── caja.bsk
│   │   └── facturacion.bsk
│   └── repositorios/
│       └── base_datos.bsk
└── pruebas/
    └── prueba_caja.bsk
```

---

## 4. API REST (api-rest)

**Ruta:** `ejemplos/api-rest/` (AUSENTE)  
**Estado:** AUSENTE — requiere capacidades no implementadas  
**Capacidades demostradas:** servidor HTTP, rutas, serialización JSON, manejo de errores

### Prerrequisitos

- Stdlib HTTP (servidor y cliente) — AUSENTE en BASILISCO
- Tipos producto con nombre para modelos de request/response
- Manejo de errores con tipos (GAP-009)

**Este ejemplo no debe crearse hasta que la stdlib HTTP esté implementada.**

---

## 5. Gestor de tareas (tareas)

**Ruta:** `ejemplos/tareas/` (AUSENTE)  
**Estado:** AUSENTE  
**Capacidades demostradas:** CRUD, persistencia, filtrado, formato de salida

### Descripción

Gestor de tareas de consola con persistencia. Es el ejemplo más representativo de aplicación de consola completa con estado.

### Prerrequisitos

Solo requiere las capacidades actuales del lenguaje más tipos producto (GAP-035). Es el ejemplo más cercano a ser implementable hoy.

---

## 6. Aplicación con persistencia (notas)

**Ruta:** `ejemplos/notas/` (AUSENTE)  
**Estado:** AUSENTE  
**Capacidades demostradas:** lectura/escritura de archivos, serialización, búsqueda de texto

### Descripción

Aplicación de notas con búsqueda, etiquetas y persistencia en archivos JSON. Demuestra E/S real de archivos, que ya existe en la stdlib.

### Prerrequisitos

Las funciones de archivo ya existen (`leer_archivo`, `escribir_archivo`, `existe_archivo`). Se puede implementar hoy con el lenguaje actual, usando el inventario como referencia.

---

## 7. Integración FFI (ffi-demo)

**Ruta:** `ejemplos/ffi-demo/` (AUSENTE)  
**Estado:** AUSENTE  
**Capacidades demostradas:** `cargar_ffi`, `llamar_ffi`, `descargar_ffi`, tipos C

### Descripción

Demuestra la integración con una biblioteca C existente. Requiere tener el compilador C disponible.

### Prerrequisitos

El puente FFI ya existe (`src/ffi.rs`). Este ejemplo es implementable hoy con las capacidades actuales.

---

## 8. Aplicación web (web-app)

**Ruta:** `ejemplos/web-app/` (AUSENTE)  
**Estado:** AUSENTE — requiere capacidades no implementadas  
**Capacidades demostradas:** WASM, bindings DOM o framework web

### Prerrequisitos

- Backend WASM completo (no experimental) — GAP-043
- Bindings a APIs del navegador o servidor

**Este ejemplo no debe crearse hasta que el backend WASM soporte todos los tipos.**

---

## Orden de implementación recomendado

Basado en las capacidades actuales del lenguaje:

1. **consola-basica** — implementable hoy (solo capacidades actuales)
2. **notas** — implementable hoy (usa E/S de archivos existente)
3. **ffi-demo** — implementable hoy (FFI existe)
4. **inventario (modular)** — tras GAP-035 (tipos producto)
5. **tareas** — tras GAP-035
6. **pos** — tras GAP-035 + GAP-047 (módulos jerárquicos)
7. **api-rest** — tras stdlib HTTP
8. **web-app** — tras backend WASM completo

---

## Criterios de aceptación para un ejemplo oficial

Un ejemplo se considera "oficial" cuando:

1. Tiene su propio `basilisco.toml`
2. `bsk run` desde su directorio funciona sin argumentos
3. El README tiene instrucciones de instalación, ejecución y salida esperada
4. Existe al menos un test de humo en CI (script que ejecuta el ejemplo y verifica el código de salida)
5. El ejemplo está referenciado en `OFFICIAL_EXAMPLES.md`
6. Los mantenedores del lenguaje se comprometen a actualizarlo cuando el lenguaje evolucione
