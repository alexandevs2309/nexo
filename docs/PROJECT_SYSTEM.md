# Sistema de proyectos BASILISCO

**Versión del documento:** 1.0  
**Fecha:** 2026-11-05  
**Estado:** PARCIALMENTE IMPLEMENTADO — ver sección de estado por componente

---

## 1. Creación de proyectos

### Estado actual: IMPLEMENTADO (estructura mínima)

El comando `bsk nuevo proyecto <nombre>` crea un proyecto con estructura básica. Implementado en `prototipo/src/proyecto.rs:crear_proyecto()`.

```bash
bsk nuevo proyecto mi-app
```

Genera:
```
mi-app/
├── basilisco.toml
├── .gitignore
├── README.md
└── src/
    ├── main.bsk      # punto de entrada
    └── lib.bsk       # biblioteca interna
```

### Estado futuro: soporte de plantillas (AUSENTE)

El diseño previsto es:
```bash
bsk nuevo proyecto mi-api --plantilla rest-api
bsk nuevo proyecto mi-pos --plantilla pos
bsk nuevo proyecto mi-lib --plantilla biblioteca
```

La selección de plantilla no está implementada. Ver [TEMPLATE_SYSTEM.md](TEMPLATE_SYSTEM.md).

---

## 2. Tipos de proyectos reconocidos

| Tipo | Descripción | Estado |
|---|---|---|
| `consola` | Aplicación de línea de comandos | IMPLEMENTADO (sin plantilla) |
| `biblioteca` | Módulo reutilizable como paquete BPK | IMPLEMENTADO (sin plantilla) |
| `rest-api` | Servidor HTTP con endpoints JSON | FUTURO (requiere stdlib HTTP) |
| `pos` | Sistema de punto de venta con persistencia | FUTURO |
| `web` | Aplicación web completa | FUTURO (requiere WASM funcional) |

---

## 3. Estructura de directorios oficial

La estructura oficial para un proyecto BASILISCO completo es:

```
mi-proyecto/
├── basilisco.toml         # manifiesto del proyecto
├── basilisco.lock         # lockfile generado (no editar a mano)
├── .gitignore
├── README.md
├── src/
│   ├── main.bsk           # punto de entrada
│   ├── lib.bsk            # módulo raíz de la biblioteca interna
│   ├── modelos/           # tipos de datos del dominio
│   │   └── producto.bsk
│   ├── servicios/         # lógica de negocio
│   │   └── inventario.bsk
│   ├── repositorios/      # acceso a datos
│   │   └── almacen.bsk
│   └── config.bsk         # configuración de la aplicación
├── pruebas/               # pruebas de integración
│   └── prueba_inventario.bsk
└── datos/                 # datos de ejemplo o semilla
    └── inicial.json
```

**Restricciones de la implementación actual:**
- `bsk nuevo proyecto` solo crea `src/main.bsk` y `src/lib.bsk`. Los subdirectorios (`modelos/`, `servicios/`, `repositorios/`) deben crearse a mano hoy.
- `pruebas/` no tiene un runner dedicado en BASILISCO todavía; el comando `bsk probar` delega en `cargo test` internamente.
- La estructura completa se alcanzará cuando las plantillas estén implementadas.

---

## 4. Manifiesto basilisco.toml

### Estado actual: IMPLEMENTADO (subconjunto)

El manifiesto es un archivo de texto plano con un subconjunto de la sintaxis TOML. El parser está en `prototipo/src/proyecto.rs:parsear_manifiesto()`.

**Formato actual (implementado):**
```toml
nombre = "mi-proyecto"
version = "0.1.0"
entrada = "src/main.bsk"
```

**Claves admitidas hoy:** `nombre`, `version` (SemVer X.Y.Z), `entrada`.

**Formato completo deseado (FUTURO):**
```toml
nombre = "mi-proyecto"
version = "0.1.0"
entrada = "src/main.bsk"
autores = ["Nombre Apellido <correo@ejemplo.com>"]
descripcion = "Descripción del proyecto"
licencia = "Apache-2.0"
edicion = "2026"

[dependencias]
basilisco-http = "0.3.0"
basilisco-json = "1.2.0"

[dev-dependencias]
basilisco-test = "0.1.0"

[construccion]
objetivo = "nativo"   # nativo | wasm | script

[plantilla]
tipo = "consola"
version = "1.0.0"
```

**Limitación actual:** el parser solo acepta `clave = "valor"` por línea y rechaza tablas (`[sección]`), arrays y números sin comillas. Hay dos parsers distintos para `basilisco.toml` en el prototipo (`proyecto.rs` y `paquetes.rs`), ambos limitados.

---

## 5. Organización modular

### Estado actual: PARCIAL

Los módulos se importan con `importar` o `usar`. El cargador resuelve rutas relativas al archivo actual y desde `bsk_modules/`.

```basilisco
# En src/main.bsk
usar "src/servicios/inventario.bsk" como inventario

inventario.listar()
```

```basilisco
# En src/servicios/inventario.bsk
publico funcion listar() {
  mostrar "Lista de productos"
}
```

**Lo que funciona hoy:** importación por alias, detección de ciclos, caché por proceso.  
**Lo que no funciona hoy:** rutas absolutas, módulos como paquetes jerárquicos (`modelos::producto`), visibilidad fina más allá de `publico`.

---

## 6. Código de entrada

El campo `entrada` en `basilisco.toml` apunta al archivo principal. `bsk run` sin argumentos lo resuelve automáticamente.

```bash
cd mi-proyecto
bsk run          # usa basilisco.toml para encontrar src/main.bsk
bsk run src/main.bsk   # explícito
```

**Estado:** IMPLEMENTADO en `cli.rs` y `proyecto.rs`.

---

## 7. Ejecución y construcción

| Comando | Estado | Descripción |
|---|---|---|
| `bsk run` | IMPLEMENTADO | Ejecuta el proyecto usando la VM |
| `bsk run --motor=arbol` | IMPLEMENTADO | Ejecuta con el evaluador tree-walk |
| `bsk analizar` | IMPLEMENTADO | Análisis semántico sin ejecutar |
| `bsk formatear` | IMPLEMENTADO (sin comentarios) | Formatea el código fuente |
| `bsk build` | AUSENTE | Compilar a binario nativo o WASM |
| `bsk test` | PARCIAL | Ejecuta pruebas del proyecto |

El comando `bsk build` es el puente hacia los backends compilados. Depende de que el backend nativo soporte todos los tipos (GAP-042), lo que a su vez depende del sistema de tipos avanzado (Etapa 4 del ROADMAP).

---

## 8. Pruebas

### Estado actual: PARCIAL

El comando `bsk probar [ruta]` existe en la CLI (`cli.rs`). Sin embargo, no hay todavía un marco de pruebas dedicado para proyectos BASILISCO: las pruebas del lenguaje usan `cargo test` de Rust.

**Diseño deseado para proyectos BASILISCO:**

```bash
bsk test                    # ejecutar todas las pruebas en pruebas/
bsk test --filtro nombre    # filtrar por nombre
```

Cada archivo en `pruebas/` sería un módulo de pruebas:
```basilisco
# pruebas/prueba_suma.bsk
funcion test_suma() {
  afirmar(2 + 2 == 4)
  afirmar(suma(3, 4) == 7)
}
```

Depende de: función `afirmar()` en stdlib (hoy existe como builtin), runner que recolecte funciones con prefijo `test_`, reporte de resultados.

---

## 9. Dependencias actuales del sistema

Para que este sistema de proyectos esté completo, se requiere:

| Componente | Estado | Bloquea |
|---|---|---|
| Manifiesto TOML completo | AUSENTE | plantillas, configuración avanzada |
| Sistema de plantillas | AUSENTE | `bsk nuevo --plantilla` |
| Backend compilado completo | AUSENTE (experimental) | `bsk build` |
| Marco de pruebas BASILISCO | AUSENTE | `bsk test` real |
| Módulos jerárquicos | AUSENTE | organización modular avanzada |
| Unificación de parsers TOML | AUSENTE | coherencia del manifiesto |
