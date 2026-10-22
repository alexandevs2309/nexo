# Auditoría cruzada: Base Ultra 0.3 frente al repositorio Nexo

**Fecha de consulta:** 2026-10-21  
**Repositorio:** `alexandevs2309/nexo`  
**Referencia consultada:** rama predeterminada `main`  
**Alcance:** lectura de documentos, ADR, fuentes Rust y manifiesto; no se ejecutaron comandos de compilación en este entorno.  
**Regla:** esta auditoría describe lo que aparece en el repositorio consultado; no certifica que los tests pasen en el momento de leerla.

## 1. Dictamen ejecutivo
La Base Ultra 0.3 es útil como expediente de diseño, pero necesita alinearse con el estado real del prototipo. La discrepancia más importante no es solo el README: el repositorio ya contiene una implementación ejecutable de expresiones, variables, ámbitos, condiciones, ciclos, comparaciones y lógica; en cambio, las funciones siguen siendo una propuesta de incremento posterior.

## 2. Evidencia revisada

| Archivo | Evidencia observada | Implicación documental |
|---|---|---|
| `README.md` | Todavía declara que el expediente no es una implementación funcional | Está desactualizado respecto al código; debe describirse como prototipo separado de la base documental |
| `00-gobernanza/estado-del-proyecto.md` | Declara incrementos 1–4 verificados y 211 pruebas verdes; funciones/retorno como siguiente incremento | Conservar la cifra como evidencia registrada por el proyecto, pero no presentarla como una ejecución realizada en esta auditoría |
| `ADR-0005-sistema-numerico.md` | Aprobado; `entero` i64, `decimal` DECIMAL(28,9), `real` f64; reglas de overflow, división, módulo y conversiones | La especificación debe preservar estas decisiones y no simplificarlas hasta perder casos límite |
| `ADR-0006-declaraciones-y-variables.md` | Aprobado; `sea`, asignación a variable declarada, tipo fijado al inicializar, mutabilidad, ámbitos | La especificación debe incluir variables mutables, sombreado en ámbitos anidados y error de variable no definida |
| `ADR-0007-estructuras-de-control-y-logicos.md` | Aprobado; `si`, `sino si`, `mientras`, bloques, comparaciones, cortocircuito y precedencia completa | La gramática de Base Ultra 0.3 debía corregirse para separar precedencia de `no`, `y` y `o` y permitir bloques independientes |
| `ADR-0008-funciones-y-retorno.md` | Estado **Propuesto**; indica expresamente que el incremento 5 no está implementado | Funciones, retorno y llamadas deben seguir marcados como pendientes; antes de aprobar ADR-0008 deben resolverse sus preguntas abiertas |
| `prototipo/src/parser.rs` | AST de expresiones y sentencias: declaración, asignación, `si`, `mientras`, bloque y expresión; sin nodos de función, llamada o retorno | No afirmar que el parser admite funciones aunque el lexer reconozca sus palabras clave |
| `prototipo/src/evaluador.rs` | Evalúa expresiones, variables, ámbitos, control de flujo, comparaciones y cortocircuito; error tipado y límite de iteraciones | El estado de capacidades debe distinguir implementación actual de trabajo propuesto |
| `prototipo/src/lexer.rs` y `token.rs` | Reconoce `funcion`, `retornar`, `mostrar` y cadenas, además de las palabras clave del núcleo | Un token reservado no equivale a una construcción implementada; el parser actual no acepta cadenas como primaria |
| `prototipo/Cargo.toml` | Paquete Rust `nexo` versión `0.1.0`, edición 2024, sin dependencias externas declaradas | La versión del expediente documental no debe confundirse con la versión del paquete |
| `prototipo/tests/*.rs` | Suites de integración y conformidad para números, variables y control de flujo | Enlazar requisitos a tests existentes por nombre y ampliar con pruebas de conformidad para las próximas funciones |

## 3. Capacidades clasificadas

### Implementadas según el código consultado
- Lexer con posiciones de línea/columna, comentarios `#`, literales numéricos, cadenas tokenizadas y palabras clave.
- Parser de expresiones numéricas y lógicas.
- Tipos `entero`, `decimal`, `real` y `lógico`.
- Variables mutables con `sea`, asignación a variables declaradas, tipos fijados en la inicialización y conversiones aprobadas.
- Ámbitos léxicos, bloques, sombreado y control de flujo `si`/`sino si`/`sino` y `mientras`.
- Comparaciones, `no`, `y`, `o`, cortocircuito y rechazo de comparaciones encadenadas.
- Errores de ejecución tipados con ubicación.
- Protección de ciclo con límite de iteraciones.

### Reservado léxicamente, pero no implementado como construcción del lenguaje
- `funcion`
- `retornar`
- `mostrar`
- Cadenas como valores primarios de expresión (el lexer las tokeniza, pero el parser no las acepta actualmente).

### Pendiente según el estado consultado
- Funciones, parámetros, llamadas y retorno.
- Decisiones finales del ADR-0008.
- Módulos, imports, colecciones, sistema general de tipos, memoria/ownership, concurrencia, I/O de biblioteca estándar, gestor de paquetes, IDE y soporte oficial multiplaforma.

## 4. Correcciones realizadas en Base Ultra 0.3.1
1. Se añadió este informe de auditoría.
2. Se reemplazó la gramática de referencia para reflejar la precedencia real del ADR-0007: comparación, `no`, `y`, `o` en niveles separados.
3. Se añadió el bloque autónomo a la gramática de sentencias.
4. Se documentó que el léxico reserva palabras clave y tokeniza cadenas, pero eso no significa que parser/evaluador las soporten.
5. Se registró explícitamente que ADR-0008 está propuesto y que las funciones aún no están implementadas.
6. Se documentó el límite de 100 000 iteraciones del prototipo como detalle que debe decidirse si será norma o protección de implementación.

## 5. Incidencias y siguientes acciones

| Prioridad | Incidencia | Acción recomendada |
|---|---|---|
| P0 | README del repositorio contradice el estado del código | Actualizarlo en una rama/commit separado, sin mezclar cambios de lenguaje |
| P0 | La especificación de referencia no estaba alineada con la precedencia aprobada | Corregido en este paquete; comparar con parser y tests antes de normativizar |
| P0 | ADR-0008 tiene seis preguntas abiertas | Resolverlas con decisiones explícitas antes de aprobar o implementar funciones |
| P1 | El estado del proyecto depende de cifras de tests históricas | Ejecutar `cargo test`, `cargo clippy --all-targets` y `cargo fmt --check` en CI y adjuntar commit |
| P1 | El lexer acepta cadenas, pero parser no las admite como valores | Decidir cuándo se incorpora `cadena` al sistema de valores y su semántica |
| P1 | `mostrar` está reservado, pero no implementado | Mantenerlo documentado como reservado/no soportado o definir salida estándar mediante ADR |
| P1 | Falta prueba automatizada de que los ejemplos documentales ejecuten | Crear suite de ejemplos que corra en CI |
| P2 | Columnas, Unicode en identificadores, FFI, memoria y concurrencia no están cerrados | Mantener como brechas, no como soporte prometido |

## 6. Comandos que debe ejecutar CI
Desde `prototipo/`:
```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo run
```
Registrar el commit exacto, versión de Rust, sistema operativo y salida. No marcar `verificado` a partir de esta auditoría documental.

## 7. Huellas de los archivos consultados
Las huellas SHA consultadas en `main` incluyen:
- `README.md`: `cf9e382f39935f3be5359c8721606d48b7830210`
- `00-gobernanza/estado-del-proyecto.md`: `fc1507689a8dc99004035142810aae076b734406`
- `ADR-0005-sistema-numerico.md`: `780b2dafcf96c7841386b79de71e5153529c4ae4`
- `ADR-0006-declaraciones-y-variables.md`: `ad54410d55f09c2ffd7e67a065dc64759e84275f`
- `ADR-0007-estructuras-de-control-y-logicos.md`: `47586e87ee0481d9c4dc487f8af1d3dda5cb0c30`
- `ADR-0008-funciones-y-retorno.md`: `e1bab1a855c1a0a37626c8fde49c838ecf2699ab`
- `prototipo/src/parser.rs`: `f6598895479c47cad581d6310894c2909b46db14`
- `prototipo/src/evaluador.rs`: `6e589809c2b4beb646830a52de3551ae9d07cb0f`
