# Plan de migración de convenciones del ecosistema BASILISCO

- **Estado:** plan aprobado como dirección por el autor; ninguna migración de código se declara implementada por este documento.
- **Fecha de decisión:** 2026-10-29.
- **Base inspeccionada:** rama `docs/auditoria-ecosistema-basilisco`, prototipo Rust, versión declarada `0.6.0`.
- **Decisiones del autor:** extensión canónica `.bs`; manifiesto `basilisco.toml` usando TOML; CLI principal `bsk`; registro oficial propio desarrollado por fases.
- **Objetivo:** convertir las decisiones en un contrato único, preservar compatibilidad razonable y evitar que los documentos anuncien capacidades aún no implementadas.

## 1. Evaluación de compatibilidad

| Decisión | Estado observado | Evaluación | Migración necesaria |
|---|---|---|---|
| `.bs` como extensión canónica | El código y ejemplos usan `.bsk`; `.nexo` se acepta como legado; `.nx` no está soportada | Cambio incompatible si se retiran las extensiones actuales de inmediato | Hacer `.bs` canónica; mantener temporalmente `.bsk` como extensión anterior y `.nexo` como legado; actualizar creación de archivos, resolución, ayuda, pruebas y ejemplos |
| `basilisco.toml` con TOML | Existe un parser manual para campos planos de proyecto y otro parser manual para un subconjunto de paquetes | **No es un parser TOML completo.** Los dos parsers admiten gramáticas distintas bajo el mismo nombre de archivo | Definir un esquema único y versionado; adoptar un parser TOML compatible con la especificación y validar después el esquema BASILISCO |
| `bsk` como CLI principal | Cargo ya declara los binarios `basilisco` y `bsk`; ambos usan el punto de entrada común | Compatible con el código actual, aunque mensajes de ayuda y errores usan `basilisco` como nombre principal | Cambiar documentación y ejemplos a `bsk`; conservar `basilisco` como alias compatible durante la transición |
| Registro oficial propio por fases | La resolución de paquetes es local; existen `paquetes/`, `vendor/` y `basilisco.lock`; no hay descarga desde red en el módulo inspeccionado | No existe todavía un registro remoto | Fortalecer primero paquetes locales y lockfile; luego formato distribuible, API del registro, cliente y publicación segura |

### Hallazgo crítico sobre TOML

`prototipo/src/proyecto.rs` acepta solo asignaciones simples `clave = "valor"` para `nombre`, `version` y `entrada`, y explícitamente rechaza tablas y arrays. `prototipo/src/paquetes.rs` implementa otro subconjunto que admite `[paquete]`, `[dependencias]` y algunas tablas inline. Ambos archivos se llaman `basilisco.toml`, pero el comportamiento actual no garantiza compatibilidad con TOML completo ni una única estructura de manifiesto.

**Decisión técnica recomendada:** utilizar un parser TOML mantenido y compatible con una versión fijada de la especificación, y separar dos etapas:
1. parseo sintáctico TOML;
2. validación semántica contra el esquema de manifiesto de BASILISCO.

No etiquetar el parser manual actual como implementación completa de TOML. Si se decide no añadir una dependencia de parser, la documentación tendría que llamar al formato «subconjunto TOML de BASILISCO»; eso no coincide con la decisión tomada y no se recomienda.

## 2. Contrato objetivo propuesto

### 2.1 Extensiones

- **Canónica:** `.bs`.
- **Compatibilidad de transición:** `.bsk`, extensión canónica anterior.
- **Legado:** `.nexo`, conservada temporalmente si las pruebas confirman que ya se acepta.
- **No soportada:** `.nx`, salvo decisión explícita posterior.
- Todo proyecto nuevo y ejemplo nuevo debe usar `.bs`.
- La CLI debe detectar extensiones por una tabla centralizada y no por comprobaciones dispersas.

La duración de compatibilidad de `.bsk` y `.nexo` debe quedar vinculada a una política de versiones. No retirarlas en el mismo cambio que introduce `.bs`.

### 2.2 CLI

- **Comando principal documentado:** `bsk`.
- **Alias de compatibilidad:** `basilisco`.
- Los mensajes de ayuda, tutoriales y ejemplos nuevos deben usar `bsk`.
- Los binarios de Cargo pueden seguir generando ambos ejecutables durante la transición.
- Los comandos deben conservar aliases históricos cuando sea razonable; cualquier retirada exige aviso de deprecación.

Ejemplos objetivo:

```bash
bsk --version
bsk nuevo proyecto mi_app
cd mi_app
bsk run
bsk analizar src/main.bs
```

Estos ejemplos expresan la interfaz objetivo. No implican que todos los comandos se hayan probado ya con archivos `.bs`.

### 2.3 Manifiesto TOML unificado

Se recomienda un único esquema de proyecto/paquete con `[proyecto]` como tabla de identidad y `[dependencias]` como tabla de dependencias. Ejemplo de diseño, pendiente de especificación normativa:

```toml
[proyecto]
nombre = "mi_app"
version = "0.1.0"
entrada = "src/main.bs"
requerimiento_basilisco = ">=0.6.0"

[dependencias]
matematica = "1.2.0"
utilidades = { version = "0.4.0", ruta = "../utilidades" }
```

Reglas que deben especificarse antes de implementar:
- versión del esquema de manifiesto y estrategia de evolución;
- campos requeridos y opcionales;
- validación de nombre y SemVer;
- sintaxis de rangos de versiones;
- rutas locales, normalización y prohibición de escape del directorio permitido;
- dependencias opcionales/de desarrollo, si se justifican;
- detección de claves duplicadas y desconocidas;
- reglas de resolución y errores deterministas;
- relación con `basilisco.lock`;
- compatibilidad del CLI con cada versión del esquema.

El ejemplo anterior es una propuesta, no un formato ya aceptado por el prototipo.

### 2.4 Lockfile

- Mantener el nombre actual `basilisco.lock` durante esta migración.
- Versionar explícitamente el formato del lockfile.
- Guardar identidad/versiones/rutas normalizadas y, cuando haya paquetes distribuibles, hashes criptográficos del contenido.
- La resolución debe ser determinista; la ejecución con lockfile no debe resolver silenciosamente versiones diferentes.
- La migración no debe cambiar el formato de lockfile sin lector compatible, pruebas y estrategia de regeneración.

## 3. Fases de implementación

### Fase 0 — Inventario y línea base

- [ ] Registrar el commit objetivo y la versión del prototipo.
- [ ] Ejecutar `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`.
- [ ] Identificar cada uso de `.bsk`, `.nexo`, `basilisco.toml`, `basilisco.lock`, `basilisco` y `bsk` en código, pruebas, documentación y ejemplos.
- [ ] Crear pruebas de caracterización del comportamiento actual antes de cambiarlo.
- **Salida:** informe reproducible de línea base. No avanzar con fallos sin clasificar.

### Fase 1 — Especificación normativa

- [ ] Aprobar la política de extensiones y el periodo de compatibilidad.
- [ ] Aprobar `bsk` como nombre principal y `basilisco` como alias.
- [ ] Definir el esquema de `basilisco.toml`, versión de TOML soportada y errores de validación.
- [ ] Definir el formato y la compatibilidad de `basilisco.lock`.
- [ ] Crear una matriz de compatibilidad CLI × lenguaje × manifiesto × lockfile.
- **Salida:** especificación y ADR aprobados antes de tocar la implementación.

### Fase 2 — Manifiesto y validación

- [ ] Añadir y fijar una implementación TOML compatible con la especificación elegida.
- [ ] Centralizar el parseo en un único módulo, con parseo sintáctico separado de validación semántica.
- [ ] Unificar la información de proyecto y paquete que hoy interpretan `proyecto.rs` y `paquetes.rs`.
- [ ] Proporcionar errores con archivo, línea y columna cuando el parser lo permita.
- [ ] Añadir pruebas válidas, inválidas, duplicados, tablas inline, arrays, escapes, UTF-8, comentarios y campos desconocidos.
- [ ] Añadir pruebas de ida/vuelta solo para campos que la herramienta realmente serialice.
- **Salida:** un único lector de manifiesto y ninguna gramática paralela no documentada.

### Fase 3 — Migración de extensión y CLI

- [ ] Centralizar la clasificación de extensiones: `.bs`, `.bsk` y `.nexo`.
- [ ] Cambiar `bsk nuevo` y `bsk nuevo proyecto` para generar `.bs`.
- [ ] Actualizar resolución de entrada, imports, módulos, formateador y diagnósticos.
- [ ] Cambiar ejemplos nuevos a `.bs`; conservar fixtures específicos para compatibilidad.
- [ ] Cambiar ayuda, mensajes, README y tutoriales para mostrar `bsk` como comando principal.
- [ ] Mantener `basilisco` funcionando como alias; probar ambos binarios.
- [ ] Añadir pruebas positivas y negativas por extensión y por comando.
- **Salida:** el usuario nuevo utiliza `.bs` y `bsk`; los proyectos existentes siguen funcionando según la política de compatibilidad aprobada.

### Fase 4 — Paquetes locales reproducibles

- [ ] Resolver paquetes locales con el manifiesto unificado.
- [ ] Definir semántica para rutas explícitas, `paquetes/` y `vendor/`; eliminar ambigüedades de prioridad.
- [ ] Validar conflictos de versiones, ciclos y dependencias transitivas.
- [ ] Generar/leer `basilisco.lock` de forma determinista.
- [ ] Registrar hashes para artefactos cuando el formato lo permita.
- [ ] Probar instalaciones desde directorios limpios y sin red.
- **Salida:** paquetes locales reproducibles y errores previsibles.

### Fase 5 — Formato de distribución y registro oficial

- [ ] Definir paquete distribuible, metadatos, estructura, checksum, límites de tamaño y protección contra path traversal.
- [ ] Definir API del registro, nombres reservados, namespaces, autenticación y permisos de publicación.
- [ ] Implementar cliente de búsqueda/descarga y caché con verificación de integridad.
- [ ] Definir procedimiento de retirada, reporte de paquetes maliciosos y recuperación de cuentas.
- [ ] Implementar publicación con validación previa y autenticación explícita.
- [ ] Probar fallos de red, paquetes corruptos, sustitución de versiones y registros no confiables.
- **Salida:** registro remoto solo cuando la instalación local, integridad y política de seguridad estén cerradas.

## 4. Cambios documentales requeridos

| Documento/área | Cambio requerido | Momento |
|---|---|---|
| `README.md` | Distinguir convenciones actuales y objetivo; no anunciar `.bs` como implementada hasta superar pruebas | Antes del release de migración |
| `10-especificacion/convenciones-oficiales-y-ecosistema.md` | Registrar las decisiones aprobadas y las brechas de implementación | Ahora y en cada fase |
| `08-plan/plan-maestro-alcance-y-brechas-basilisco.md` | Añadir la migración de extensiones, manifiesto, CLI y registro al orden de ejecución | Ahora |
| Especificación de manifiesto | Documentar gramática/esquema, tipos, errores y compatibilidad | Fase 1 |
| ADR de extensiones/CLI | Aprobar `.bs`, compatibilidad de `.bsk`/`.nexo` y jerarquía de comandos | Fase 1 |
| ADR de paquetes | Fijar esquema TOML, lockfile, resolución y registro por fases | Fase 1 |
| Tutoriales y ejemplos | Usar `.bs`/ `bsk` solo después de que la fase 3 esté implementada | Fase 3 |
| Referencia CLI | Documentar comandos existentes y marcar futuros como propuestos | Cada fase |
| Registro de brechas | Abrir tareas separadas para parser TOML, extensión, CLI, lockfile, seguridad y registro | Ahora |
| CI | Añadir fixtures de compatibilidad y pruebas de instalación del binario | Fases 0–4 |

## 5. Criterios de aceptación global

La migración no se considera completada hasta que:

1. un parser TOML estándar acepte archivos TOML válidos y el validador BASILISCO rechace esquemas inválidos con diagnósticos claros;
2. proyecto y gestor de paquetes consuman el mismo modelo de manifiesto;
3. `bsk` sea la interfaz principal documentada y `basilisco` siga funcionando según la política aprobada;
4. archivos `.bs` se puedan crear, ejecutar, analizar, importar y formatear mediante pruebas;
5. la compatibilidad de `.bsk` y `.nexo` esté cubierta por pruebas y por una política de deprecación;
6. el lockfile produzca resolución reproducible;
7. CI pase en plataformas declaradas;
8. README, tutorial, referencia de CLI, especificación y changelog describan el mismo estado real.

## 6. Riesgos y controles

- **Ruptura de proyectos existentes:** mantener `.bsk` y `.nexo` durante la transición; no renombrar fixtures sin pruebas.
- **Falsa compatibilidad TOML:** no llamar TOML completo a un parser parcial; usar parser estándar y validar esquema aparte.
- **Dependencias transitivas:** revisar licencia, mantenimiento, versiones y superficie de ataque de la biblioteca TOML.
- **Manifiesto ambiguo:** un solo esquema y una sola ruta de parseo.
- **Registro inseguro:** no descargar ni ejecutar paquetes remotos hasta tener integridad, autenticación, límites y política de respuesta a incidentes.
- **Documentación adelantada al código:** separar claramente `DECIDIDO`, `EN IMPLEMENTACIÓN`, `IMPLEMENTADO` y `VERIFICADO`.

## 7. Referencia externa

La especificación oficial de TOML define un formato de configuración legible y portable. BASILISCO debe declarar la versión que acepta y utilizar un parser conforme, sin confundir parseo TOML con validación del esquema propio:
- TOML 1.1.0: https://toml.io/en/v1.1.0
- Especificación TOML: https://toml.io/en/

**Estado final de este documento:** la dirección de migración está definida; la implementación y la verificación siguen pendientes.
