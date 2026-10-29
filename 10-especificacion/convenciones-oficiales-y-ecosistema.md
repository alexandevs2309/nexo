# Convenciones oficiales y ecosistema de BASILISCO

- **Estado:** decisiones de dirección aprobadas por el autor el 2026-10-29; implementación pendiente. Este documento no afirma que el prototipo ya cumpla las convenciones objetivo.
- **Objetivo:** que BASILISCO tenga una experiencia consistente de instalación, proyectos, extensiones, bibliotecas, paquetes y distribución.
- **Decisiones aprobadas:** extensión canónica objetivo `.bs`; manifiesto `basilisco.toml` con TOML; CLI principal `bsk`; registro oficial propio por fases.
- **Principio:** una convención oficial, un formato de manifiesto, un comando principal y reglas de compatibilidad documentadas.

## 1. Aclaración de conceptos

Los lenguajes no suelen ser un único programa aislado. El ecosistema se compone de piezas distintas:

| Pieza | Función | Ejemplo externo |
|---|---|---|
| Lenguaje | Sintaxis y semántica del código | Python, JavaScript, Rust |
| Intérprete/runtime | Ejecuta programas | CPython, Node.js, runtime de BASILISCO |
| Extensión de fuente | Identifica archivos fuente | `.py`, `.js`, `.rs` |
| CLI/toolchain | Comandos oficiales de desarrollo | `python`, `node`, `cargo` |
| Gestor de paquetes | Resuelve e instala dependencias | `pip`, `npm`, `cargo` |
| Registro de paquetes | Aloja paquetes publicados | PyPI, npm registry, crates.io |
| Manifiesto | Describe el proyecto y sus dependencias | `pyproject.toml`, `package.json`, `Cargo.toml` |
| Biblioteca estándar | APIs incluidas con el lenguaje/runtime | módulos estándar de cada ecosistema |
| Editor/IDE | Facilita escribir y depurar | VS Code y otros |

**Corrección importante:** PyPy no es el equivalente de pip ni el repositorio de paquetes. PyPy es otra implementación del intérprete de Python. `pip` instala paquetes; PyPI es el registro que los aloja. En JavaScript, npm es el gestor/CLI y el npm registry es el registro. En Rust, Cargo es el gestor de paquetes y build tool; crates.io es el registro. Véanse las referencias al final.

BASILISCO debe definir estas piezas de forma explícita, aunque al principio algunas funciones sean locales o limitadas.

## 2. Convenciones objetivo aprobadas y estado de implementación

Las decisiones de dirección de esta sección fueron aprobadas por el autor. Son el objetivo de migración, no una afirmación de que la implementación actual ya las cumpla.

### 2.1 Identidad y ejecutables

- Nombre del lenguaje: **BASILISCO**.
- Comando principal objetivo: `bsk`.
- Alias de compatibilidad: `basilisco`.
- Extensión canónica objetivo de fuente: `.bs`.
- Extensión anterior: `.bsk`, que debe mantenerse temporalmente por compatibilidad.
- Extensión histórica: `.nexo`, que se conserva temporalmente mientras las pruebas confirmen su aceptación.
- No aceptar `.nx` sin una decisión explícita de compatibilidad.
- Los ejemplos nuevos deben usar `.bs` después de que la migración de código esté implementada; hasta entonces, la documentación de uso real debe identificar la extensión que admite el prototipo.
- El número de versión del CLI debe corresponder al release del paquete; la versión del lenguaje/contrato debe documentarse por separado si evoluciona de forma independiente.

### 2.2 Comandos canónicos propuestos

| Comando | Propósito |
|---|---|
| `bsk --version` | Mostrar versión del CLI/runtime |
| `bsk --help` | Ayuda general |
| `bsk nuevo proyecto <nombre>` | Crear estructura de proyecto |
| `bsk run [archivo.bs]` | Ejecutar archivo o entrada del proyecto |
| `bsk analizar [ruta]` | Lexer, parser y análisis semántico sin ejecutar |
| `bsk formatear [ruta]` | Formatear fuente |
| `bsk probar [ruta]` | Ejecutar pruebas del proyecto |
| `bsk instalar` | Resolver e instalar dependencias declaradas |
| `bsk agregar <paquete> [versión]` | Añadir dependencia al manifiesto |
| `bsk quitar <paquete>` | Quitar dependencia |
| `bsk paquetes` | Listar dependencias resueltas |
| `bsk publicar` | Validar y publicar un paquete, cuando exista registro remoto |
| `bsk doc` | Generar/mostrar documentación, cuando se implemente |
| `bsk fmt --check` | Comprobar formato sin modificar archivos, si se adopta esta forma |

No prometer comandos que todavía no estén implementados. Mantener compatibilidad con los nombres existentes mediante aliases o deprecación documentada; evitar dos comandos que hagan lo mismo de forma diferente.

## 3. Extensiones de archivo

Propuesta inicial:

| Extensión | Uso |
|---|---|
| `.bs` | Extensión canónica objetivo de fuente BASILISCO |
| `.bsk` | Extensión anterior; mantener temporalmente por compatibilidad |
| `.bskm` | Propuesta para módulo BASILISCO separado, solo si hace falta distinguirlo de un programa ejecutable |
| `.bskpkg` | Propuesta para archivo de paquete distribuible, únicamente si se define un formato binario/archivo oficial |
| `.bsk.lock` | Alternativa propuesta para lockfile; actualmente el proyecto usa `basilisco.lock`, que debe conservarse hasta una migración aprobada |
| `.bskc` | No reservar todavía; decidir solo si aparece un artefacto compilado con formato estable |

**Decisión objetivo:** `.bs` será la extensión canónica; `.bsk` y `.nexo` deben permanecer aceptadas durante la transición según una política de deprecación. La implementación actual sigue usando `.bsk` como canónica, por lo que este cambio requiere código, pruebas y migración de ejemplos. No inventar extensiones adicionales antes de que tengan una función técnica real.

## 4. Estructura oficial propuesta para un proyecto

Ejemplo de proyecto ejecutable:

```text
mi_proyecto/
├── basilisco.toml
├── basilisco.lock
├── README.md
├── src/
│   └── principal.bsk
├── tests/
│   └── principal.bsk
├── paquetes/
├── vendor/
└── .gitignore
```

Reglas:

- `basilisco.toml` es la única fuente de verdad para nombre, versión, entrada y dependencias.
- `src/principal.bsk` es la entrada por defecto, salvo que el manifiesto especifique otra.
- `tests/` contiene pruebas del proyecto; la CLI debe documentar exactamente cómo las descubre.
- `paquetes/` y `vendor/` son ubicaciones locales, no dos sistemas de paquetes con semánticas distintas.
- `basilisco.lock` registra versiones/rutas resueltas para reproducibilidad.
- Los artefactos generados deben ir en un directorio ignorado por Git, por ejemplo `build/` o `.basilisco/`, tras decidir uno.
- La creación del proyecto debe producir un ejemplo que se ejecute sin conexión ni dependencias externas.

## 5. Un único manifiesto TOML

El repositorio contiene dos parsers manuales diferentes para `basilisco.toml`: `proyecto.rs` admite solo asignaciones simples de cadenas y rechaza tablas/arrays; `paquetes.rs` admite un subconjunto distinto con `[paquete]`, `[dependencias]` y tablas inline. Por tanto, la implementación actual no es un parser TOML completo ni un único esquema.

**Decisión:** conservar el nombre `basilisco.toml` y usar TOML como formato de configuración. La implementación debe usar un parser compatible con la versión de TOML fijada y validar por separado el esquema BASILISCO. No declarar compatibilidad completa con TOML hasta que las pruebas lo demuestren.

Propuesta de un solo formato TOML:

```toml
[proyecto]
nombre = "mi_proyecto"
version = "0.1.0"
entrada = "src/principal.bs"
requerimiento_basilisco = ">=0.6.0"

[dependencias]
matematica = "1.2.0"
utilidades = { version = "0.4.0", ruta = "../utilidades" }

[paquete]
tipo = "aplicacion"
```

Este ejemplo es **ilustrativo y no necesariamente compatible con el parser actual**. Antes de adoptarlo, se debe decidir si la tabla raíz será `[proyecto]` o `[paquete]`, qué campos son obligatorios, cómo se expresan rangos semver, rutas, dependencias opcionales y targets. No deben coexistir silenciosamente dos gramáticas con el mismo nombre de archivo.

Decisiones pendientes del manifiesto:

- [ ] Un esquema TOML oficial y versionado.
- [ ] Nombre/version/entrada y restricciones de versión de BASILISCO.
- [ ] Dependencias por versión exacta o rango semver.
- [ ] Dependencias locales por ruta.
- [ ] Dependencias de desarrollo y opcionales, si se necesitan.
- [ ] Targets: aplicación, biblioteca o herramienta CLI.
- [ ] Reglas de resolución, prioridad, conflictos y errores.
- [ ] Compatibilidad entre versión de manifiesto y versión de CLI.

## 6. Bibliotecas y módulos

Distinguir tres niveles:

1. **Módulo local:** archivo fuente importado por otro archivo del mismo proyecto.
2. **Biblioteca/paquete:** unidad versionada que puede reutilizarse en más de un proyecto.
3. **Biblioteca estándar:** módulos oficiales mantenidos con la distribución de BASILISCO.

Propuesta de uso futuro:

```basilisco
usar "matematica" como mate

dato resultado = mate.sumar(10, 20)
mostrar resultado
```

Esta sintaxis es ilustrativa. La forma definitiva debe elegirse en una decisión de diseño y unificarse con los mecanismos actuales `importar`, `usar`, `exportar` y aliases. No mantener varias sintaxis equivalentes sin razón.

Para cada biblioteca se debe definir:

- nombre canónico y namespace;
- API pública frente a privada;
- versión y compatibilidad;
- dependencias;
- licencia y procedencia;
- pruebas y documentación;
- política de seguridad;
- política de deprecación.

## 7. Gestor y registro de paquetes

### 7.1 Objetivo de experiencia de usuario

La experiencia deseada debe ser sencilla:

```bash
basilisco nuevo proyecto mi_app
cd mi_app
basilisco run
basilisco agregar json
basilisco instalar
basilisco probar
```

Los comandos `agregar` y la instalación remota son propuestas futuras, no deben documentarse como funcionalidad disponible hasta implementarse.

### 7.2 Evolución por etapas

**Etapa A — paquetes locales confiables**
- Unificar manifiesto y lockfile.
- Resolver dependencias locales con versiones y conflictos claros.
- Registrar hashes y errores reproducibles.
- Probar instalación desde directorios limpios.

**Etapa B — formato de paquete**
- Definir archivo distribuible, metadatos, versión, checksum y contenido.
- Rechazar rutas inseguras, archivos duplicados y contenido fuera de la raíz.
- Definir firma/procedencia como mecanismo posterior, con política explícita.

**Etapa C — registro remoto oficial**
- Diseñar el servicio de registro y su API.
- Definir nombres reservados, namespaces de usuario/organización, autenticación y publicación.
- Añadir búsqueda, descarga, cache y verificación de integridad.
- Establecer política de malware, reportes, retirada y transferencia de paquetes.

**Etapa D — publicación**
- `basilisco publicar --dry-run` valida paquete, pruebas, licencia y metadatos.
- `basilisco publicar` publica solo con autenticación explícita.
- Releases inmutables, checksums, historial y política de deprecación.

No es necesario construir el registro remoto antes de cerrar la semántica del lenguaje y el formato de paquetes. Pero sí conviene definir desde ahora los contratos para no crear una migración incompatible.

## 8. Biblioteca estándar: política

La biblioteca estándar debe ser pequeña, estable y portable. Propuesta de módulos por etapas:

- `texto`: longitud, búsqueda, división, reemplazo y normalización claramente especificada.
- `colecciones`: operaciones comunes de listas y arreglos.
- `matematica`: funciones numéricas deterministas donde sea posible.
- `archivos` y `rutas`: operaciones de sistema con errores tipados.
- `json`: parseo/serialización interoperable.
- `tiempo`: fechas, duración y reloj.
- `sistema`: argumentos, entorno y plataforma, con límites documentados.
- `red`: cliente/servidor y protocolos, solo después de definir modelo de errores y seguridad.
- `pruebas`: utilidades para escribir y ejecutar tests.

No incorporar todo al lenguaje central. Definir cada API con firmas, tipos de retorno, errores, ejemplos y pruebas. Las APIs dependientes del sistema deben quedar en módulos explícitos.

## 9. Convenciones de nombres

Propuesta:

- Archivos fuente: minúsculas con guion bajo, por ejemplo `gestor_clientes.bsk`.
- Módulos: identificadores simples y estables, evitando nombres reservados.
- Paquetes: minúsculas, números y guiones cuando el registro lo permita; el identificador de importación debe estar definido por el manifiesto.
- Variables y funciones: escoger una convención oficial (por ejemplo, `snake_case`) y aplicarla en toda la biblioteca estándar.
- Constantes: decidir una regla única y documentarla.
- Nombres de tipos: escoger una convención estable; no mezclar reglas entre módulos oficiales.
- Comandos y mensajes de CLI: mantener el español si esa es la política de producto, pero evitar traducir identificadores de API de forma inconsistente.
- No cambiar nombres públicos sin deprecación, alias temporal y nota de migración.

La sintaxis en español es una decisión de producto; debe someterse a pruebas de usabilidad y no impedir interoperabilidad.

## 10. Compatibilidad y versionado

Definir por separado:

- versión del CLI/runtime;
- versión del contrato del lenguaje;
- versión del formato de manifiesto;
- versión de paquetes/bibliotecas.

Usar SemVer cuando se haya definido qué constituye cambio incompatible:

- **MAJOR:** cambio incompatible en sintaxis, semántica, API pública o formato.
- **MINOR:** funcionalidad compatible nueva.
- **PATCH:** corrección compatible.

El lenguaje debe tener un perfil de conformidad por versión. Una extensión experimental no debe convertirse en garantía estable sin especificación y pruebas.

## 11. Comandos de referencia frente a otros ecosistemas

| Ecosistema | Fuente | Herramienta | Registro |
|---|---|---|---|
| Python | `.py` | `pip` instala; `python` ejecuta | PyPI |
| JavaScript/Node.js | `.js`, `.mjs`, `.cjs` según contexto | `npm` | npm registry |
| Rust | `.rs` | `cargo` | crates.io |
| BASILISCO (objetivo) | `.bs` | `bsk` (alias `basilisco`) | Registro oficial propio por fases |

Las convenciones externas son referencias, no plantillas que deban copiarse literalmente. La meta es que el usuario no necesite entender el interior del intérprete para crear, ejecutar, probar, instalar y publicar.

## 12. Decisiones y tareas pendientes

Las decisiones principales de extensión, manifiesto, CLI y registro ya fueron aprobadas como dirección el 2026-10-29. Siguen pendientes de especificación e implementación:

- [ ] Migrar a `.bs` como extensión canónica y mantener `.bsk`/`.nexo` durante la transición.
- [ ] Usar `bsk` como comando principal y conservar `basilisco` como alias.
- [ ] Implementar un único esquema versionado para `basilisco.toml` usando TOML conforme.
- [ ] Aprobar estructura estándar de proyectos.
- [ ] Elegir convención de nombres de variables, funciones, tipos y paquetes.
- [ ] Unificar `importar`/`usar`/`exportar` en una interfaz modular coherente.
- [ ] Completar y probar el gestor local antes de construir el registro remoto propio.
- [ ] Aprobar política de compatibilidad y deprecación.
- [ ] Definir qué módulos integran la biblioteca estándar inicial.
- [ ] Definir qué significa “estable” para cada perfil del lenguaje.

## 13. Fuentes oficiales de referencia

- Python Packaging User Guide: https://packaging.python.org/en/latest/
- PyPI: https://docs.pypi.org/
- Diferencia entre pip y PyPI: https://docs.pypi.org/using-pypi/finding-and-installing/
- PyPy (implementación de Python): https://doc.pypy.org/introduction.html
- npm, paquetes y módulos: https://docs.npmjs.com/about-packages-and-modules/
- The Cargo Book: https://doc.rust-lang.org/cargo/
- Cargo y crates.io: https://doc.rust-lang.org/cargo/guide/dependencies.html

## Regla final

BASILISCO debe ofrecer una experiencia coherente de extremo a extremo: instalar el runtime, crear un proyecto, escribir `.bs`, ejecutar, analizar, probar, añadir una biblioteca, reproducir dependencias y —cuando exista un registro— publicar un paquete. Hasta que la migración esté implementada y verificada, la extensión `.bs` y el uso principal de `bsk` son objetivos, no capacidades confirmadas del código. Las convenciones se deben definir antes de multiplicar herramientas y formatos.
