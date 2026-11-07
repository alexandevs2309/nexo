# Convenciones oficiales de proyectos BASILISCO

**Versión del documento:** 1.0  
**Fecha:** 2026-11-05  
**Estado:** APROBADO — aplicable a proyectos nuevos desde hoy

Estas convenciones son la forma oficial de organizar código BASILISCO. No son sugerencias: las plantillas oficiales las siguen y las herramientas (formateador, linter, LSP futuro) las reforzarán.

---

## 1. Nombres de archivos

| Convención | Descripción | Ejemplo |
|---|---|---|
| `kebab-case` | Nombre de archivos `.bsk` | `producto.bsk`, `gestor-ventas.bsk` |
| Sin espacios | No usar espacios en nombres de archivo | `✓ mi-modulo.bsk` / `✗ mi modulo.bsk` |
| Solo ASCII | Nombres de archivo solo con ASCII | `✓ usuario.bsk` / `✗ usuário.bsk` |
| Extensión `.bsk` | Canónica desde v0.7.0 | `main.bsk`, `lib.bsk` |
| Extensión `.nexo` | Legado, aceptada pero no recomendada | Solo para archivos heredados |

**Archivos especiales con nombre fijo:**
- `main.bsk` — punto de entrada del programa (referenciado en `basilisco.toml`)
- `lib.bsk` — módulo raíz de la biblioteca interna del proyecto
- `config.bsk` — configuración de la aplicación (constantes, parámetros)

---

## 2. Módulos

Los módulos organizan el código por responsabilidad, no por tipo técnico.

```
# Correcto: organización por responsabilidad
src/
├── inventario.bsk        # módulo de inventario completo
├── facturacion.bsk       # módulo de facturación
└── reportes.bsk          # módulo de reportes

# Correcto: organización por capa (proyectos más grandes)
src/
├── modelos/
│   └── producto.bsk
├── servicios/
│   └── inventario.bsk
└── repositorios/
    └── almacen.bsk
```

**Convenciones de nombres de módulos:**
- Sustantivos en singular: `producto.bsk`, no `productos.bsk`
- Kebab-case para nombres compuestos: `gestor-tareas.bsk`
- Los módulos de servicio terminan en el nombre del dominio: `inventario.bsk`, no `servicio-inventario.bsk`

---

## 3. Servicios

Un servicio contiene la lógica de negocio. No accede directamente a archivos ni a la red: usa repositorios.

```basilisco
# src/servicios/inventario.bsk
importar "repositorios/almacen.bsk" como almacen

publico funcion agregar_producto(nombre, precio, existencia) {
  sea id = siguiente_id(almacen.cargar())
  sea productos = almacen.cargar()
  agregar(productos, [id, nombre, precio, existencia])
  almacen.guardar(productos)
  retornar id
}

publico funcion buscar_por_nombre(nombre) {
  sea productos = almacen.cargar()
  # ... lógica de búsqueda ...
}
```

**Reglas:**
- Todas las funciones públicas documentadas con un comentario de una línea
- Sin acceso directo a archivos o red (delegar a repositorios)
- Sin interacción con el usuario (delegar a la interfaz)

---

## 4. Modelos

Un modelo define la estructura de un tipo de dato del dominio. Hoy BASILISCO no tiene tipos producto con nombre (structs), por lo que los modelos se representan como arreglos con índices documentados o como convención comentada.

```basilisco
# src/modelos/producto.bsk
# Producto: [id: entero, nombre: texto, precio: decimal, existencia: entero]
# Índices:
const IDX_ID = 0
const IDX_NOMBRE = 1
const IDX_PRECIO = 2
const IDX_EXISTENCIA = 3

publico funcion nuevo(id, nombre, precio, existencia) {
  retornar [id, nombre, precio, existencia]
}

publico funcion obtener_id(p) { retornar p[IDX_ID] }
publico funcion obtener_nombre(p) { retornar p[IDX_NOMBRE] }
publico funcion obtener_precio(p) { retornar p[IDX_PRECIO] }
publico funcion obtener_existencia(p) { retornar p[IDX_EXISTENCIA] }
```

**Nota:** cuando los tipos producto (GAP-035) estén implementados, esta convención cambiará. Los modelos con índices son una convención de transición compatible con el lenguaje actual.

---

## 5. Repositorios

Un repositorio maneja el acceso a datos (archivos, red, base de datos).

```basilisco
# src/repositorios/almacen.bsk
const RUTA_DATOS = "datos/inventario.json"

publico funcion cargar() {
  si existe_archivo(RUTA_DATOS) {
    intentar {
      retornar json_parsear(leer_archivo(RUTA_DATOS))
    } rescatar e {
      mostrar("AVISO: no se pudo leer los datos: " + e)
      retornar json_parsear("[]")
    }
  }
  retornar json_parsear("[]")
}

publico funcion guardar(datos) {
  escribir_archivo(RUTA_DATOS, json_serializar(datos))
}
```

**Reglas:**
- Un repositorio por tipo de dato o fuente
- Las funciones `cargar()` y `guardar()` son los nombres canónicos para lectura/escritura
- Manejar errores de E/S con `intentar`/`rescatar`, nunca dejar que propaguen sin captura

---

## 6. Manejo de errores

```basilisco
# Correcto: capturar y reportar
intentar {
  sea datos = json_parsear(contenido)
  retornar datos
} rescatar e {
  mostrar("Error al leer datos: " + e)
  retornar json_parsear("[]")
}

# Incorrecto: ignorar el error
intentar {
  json_parsear(contenido)
} rescatar e { }

# Incorrecto: mostrar un error sin contexto
intentar {
  json_parsear(contenido)
} rescatar e {
  mostrar(e)
}
```

**Convenciones:**
- Siempre incluir contexto en el mensaje de error: "¿qué se estaba intentando hacer?"
- El nombre `e` es el estándar para la variable de error en `rescatar e`
- Valores de retorno seguros: devolver una colección vacía `[]` o `""` en lugar de `nulo` cuando sea posible

---

## 7. Configuración

Las constantes de configuración van en `src/config.bsk`:

```basilisco
# src/config.bsk
const RUTA_DATOS = "datos/inventario.json"
const VERSION_APP = "1.0.0"
const NOMBRE_APP = "Mi Inventario"
const LIMITE_RESULTADOS = 50
```

**Reglas:**
- `SCREAMING_SNAKE_CASE` para constantes (`const`)
- Todas las rutas de archivo en `config.bsk`, no dispersas por el código
- Sin valores hardcoded en servicios o repositorios

---

## 8. Pruebas

**Convención de nombres:**
- Los archivos de prueba van en `pruebas/`
- El nombre del archivo sigue al módulo que prueba: `pruebas/prueba_inventario.bsk`
- Las funciones de prueba tienen el prefijo `test_`: `test_agregar_producto()`

```basilisco
# pruebas/prueba_inventario.bsk
importar "../src/servicios/inventario.bsk" como inv

funcion test_agregar_producto() {
  sea id = inv.agregar_producto("Manzana", 1.50, 100)
  afirmar(id > 0)
}

funcion test_buscar_inexistente() {
  sea resultado = inv.buscar_por_nombre("XYZ-no-existe")
  afirmar(longitud(resultado) == 0)
}
```

---

## 9. Documentación

**Convenciones:**
- Comentario de una línea antes de cada función pública: explica qué hace, no cómo
- Los comentarios de implementación usan `#` al inicio de la línea
- El `README.md` de cada proyecto incluye: descripción, requisitos, instalación, ejecución, ejemplos de uso

```basilisco
# Calcula el total de una venta sumando precio × cantidad de cada línea.
publico funcion calcular_total(lineas) {
  sea total = 0
  para cada linea en lineas {
    total = total + linea[1] * linea[2]
  }
  retornar total
}
```

---

## 10. Organización del código

**Orden dentro de un archivo `.bsk`:**

1. Importaciones (`importar`, `usar`)
2. Constantes (`const`)
3. Funciones privadas (auxiliares, sin `publico`)
4. Funciones públicas (con `publico`)
5. Código de nivel superior (solo en `main.bsk`)

```basilisco
# 1. Importaciones
importar "config.bsk" como cfg
usar "repositorios/almacen.bsk" como almacen

# 2. Constantes locales
const MAX_INTENTOS = 3

# 3. Funciones privadas
funcion validar_precio(precio) {
  retornar precio > 0
}

# 4. Funciones públicas
publico funcion agregar(nombre, precio, existencia) {
  si no validar_precio(precio) {
    mostrar("Error: el precio debe ser positivo")
    retornar -1
  }
  # ...
}
```

---

## 11. Nombres de variables y funciones

| Patrón | Uso |
|---|---|
| `snake_case` | Variables y funciones (`precio_total`, `calcular_iva`) |
| `SCREAMING_SNAKE_CASE` | Constantes (`TASA_IVA`, `MAX_STOCK`) |
| Español | Todos los identificadores en español |
| Verbos para funciones | `calcular_total()`, `buscar_producto()`, `guardar_datos()` |
| Sustantivos para variables | `precio`, `lista_productos`, `nombre_cliente` |
| Evitar abreviaciones | `producto` no `prod`, `existencia` no `exist` |

---

## 12. Formato

El formateador oficial es `bsk formatear <archivo>`. Reglas que aplica automáticamente:

- Indentación de 2 espacios
- Sin espacios antes de `(` en llamadas de función
- Espacio después de `,` en listas de argumentos
- Líneas en blanco entre funciones

**Limitación actual:** `bsk formatear` pierde los comentarios (GAP-032). Hasta que este bug esté corregido, no usar `bsk formatear` en archivos con comentarios importantes.
