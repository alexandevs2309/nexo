# Cambios de BASILISCO v0.5.0

**Estado:** en desarrollo. La versión publicada más reciente es `0.4.0`.

## Alcance

BASILISCO v0.5.0 evoluciona desde la ejecución de archivos individuales hacia
proyectos reales con módulos reutilizables.

### Módulos e importaciones

- `usar "ruta.bsk" como alias` — importa un módulo completo bajo un alias.
- `usar "ruta.bsk" { nombre1, nombre2 }` — importa nombres específicos.
- `publico` — modificador de visibilidad (privado por defecto).
- Acceso a miembros: `alias.campo`, `alias.func(args)`.
- Importaciones transitivas (A → B → C).
- Detección de ciclos directos e indirectos.
- Validación de rutas: solo archivos dentro de la raíz del proyecto.
- Extensiones: `.bsk` canónica, `.nexo` legado, `.nx` y otras rechazadas.
- Módulos no ejecutan sentencias arbitrarias en nivel superior (solo declaraciones, funciones y `usar`).

### Proyectos

- `basilisco nuevo proyecto <nombre>` — crea un proyecto con `basilisco.toml`, `src/main.bsk`, `src/lib.bsk`, `README.md`, `.gitignore`.
- `basilisco.toml` — manifiesto con `nombre`, `version`, `entrada`.
- `basilisco run` (sin args) — ejecuta el proyecto actual.
- `basilisco run <archivo>` — ejecución de archivo individual (sin cambios).
- Root del proyecto = directorio del manifiesto. Las importaciones no pueden escapar.

### Diagnósticos

- Errores léxicos, sintácticos y de ejecución llevan el archivo de origen.
- Mensajes en español con línea y columna.

### Infraestructura

- Cargador de módulos con caché por ruta canónica.
- Parser propio de manifiesto (sin dependencias externas).
- `Rc<RefCell<CargadorModulos>>` para compartir el cargador entre entornos.

### Pruebas

- 391 pruebas pasando (0 fallos).
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --bins` verdes.
- Pruebas E2E de módulos, importaciones, seguridad de rutas y proyectos.

### Fuera de alcance (v0.5.0)

- Gestor de paquetes y dependencias remotas.
- Instaladores.
- `privado` explícito.
- Validación de versión del lenguaje en el manifiesto.
- Búsqueda ascendente de manifiesto.
- Compilación a bytecode o nativo.
- Concurrencia.

### Pendiente para futuras versiones

- Ampliar pruebas de uso con programas reales.
- Revisar marca y estrategia de publicación institucional.
- Migrar a crate `toml` si se necesita TOML completo.
- Implementar `privado` explícito.