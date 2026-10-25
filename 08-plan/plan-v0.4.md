# Plan de cierre de BASILISCO v0.4

## Objetivo
Cerrar una versión útil y verificable del lenguaje, sin seguir expandiendo el alcance hacia un lenguaje multipropósito completo. La idea es convertir la base actual en una entrega mínima viable y operativa.

## Criterio de cierre
La versión v0.4 se considera terminada cuando se cumplan estas condiciones:

- `cargo test` pasa con 0 fallos.
- `cargo clippy --all-targets -- -D warnings` pasa sin advertencias.
- la CLI principal ejecuta programas reales con `.bsk` y acepta `.nexo` como legado.
- la documentación de uso básico está completa y verificable.
- la versión tiene un alcance fijo y documentado.

## Estimación de avance (orientativa, no oficial)

Estimación aportada el 2026-10-25 a partir de las tareas y evidencias de este plan; no es
una métrica formal del proyecto:

- **Prototipo del MVP v0.4: 85–90 % implementado y verificado localmente.** Hay pruebas
	para las capacidades incluidas, 302 pruebas enumeradas, formato/Clippy/build limpios y
	un smoke test de `cargo install`/`cargo uninstall` en Linux. La especificación candidata
	todavía requiere aprobación.
- **Cierre como candidata publicable: 70–75 %.** Faltan aprobar formalmente el perfil de
	conformidad, decidir la versión del paquete, verificar CI del commit final y definir y
	probar las plataformas que se respaldarán.
- **Lenguaje completo: sin porcentaje objetivo.** Su alcance no está definido de forma
	exhaustiva; módulos, paquetes, biblioteca estándar extensa, concurrencia y otras áreas
	siguen siendo brechas abiertas.

La formulación resumida es: *el núcleo v0.4 ya funciona; falta formalizarlo y cerrar la
candidata*. No usar estos porcentajes como declaración oficial de completitud.

## Alcance de la v0.4

### Incluido
- lexer y parser estables
- variables, ámbitos y reasignación
- funciones, parámetros anotados o inferidos y retorno
- flujo de control garantizado (`si`, `sino`, `mientras`)
- texto y expresiones
- listas y arreglos básicos (declaración tipada, literal, indexación y asignación)
- CLI ejecutando archivos
- mensajes de error claros
- documentación mínima de uso

### Fuera de alcance por ahora
- módulos avanzados
- paquetes y dependencias
- IDE completo
- concurrencia real
- ecosistema grande
- validación legal / marca final
- soporte multiplataforma exhaustivo
- extensiones `para cada`, rangos, `romper`/`continuar` e `intentar`/`rescatar` como contrato estable; existen en el prototipo, pero quedan fuera de la aceptación v0.4 hasta completar sus pruebas de límite
- E/S de archivos y funciones integradas de texto fuera de las operaciones básicas documentadas

## Fase 1 — Congelar el alcance real

### Objetivo
Definir qué soporta la v0.4 y qué no, dejando un contrato operativo para todo el resto del proyecto.

### MVP v0.4 — características obligatorias propuestas
- sintaxis mínima estable y documentada
- variables, ámbitos y reasignación
- funciones con parámetros y retorno
- control de flujo: `si`, `sino`, `mientras`
- texto y expresiones básicas
- arreglos y listas simples
- CLI con `run`, `--help` y `--version`
- manejo de errores claro y reproducible
- ejemplos ejecutables y pruebas de aceptación

### Decisión de alcance sobre extensiones ya prototipadas

La v0.4 garantiza `si`/`sino`/`mientras`, funciones, texto básico y colecciones por
declaración tipada, literal, indexación y asignación. Aunque el parser/evaluador
también acepta `para cada`, rangos, `romper`, `continuar`, `intentar`/`rescatar` y
operaciones de E/S, esas extensiones no forman parte de la aceptación ni se anuncian
como conformes en esta versión. Los ADR correspondientes siguen aprobados; esta
decisión limita únicamente el alcance de la entrega v0.4.

El detalle del contrato está en `10-especificacion/perfil-de-conformidad-v0.4.md`;
permanece como propuesta, no como norma aprobada.

Antes de incorporarlas a una versión garantizada, se requieren al menos estos casos:

| Extensión | Casos límite obligatorios |
|---|---|
| `para cada` | Probado: binding inmutable y scope limpio con señales; pendiente: iteración vacía, texto, orden de todos los tipos, colección inválida y mutación durante iteración. La lista vacía tipada se prueba por separado |
| Rangos | Probado: exclusivo/inclusivo, vacío/descendente, negativos y límites de `i64`; pendiente: extremos con tipo inválido y ciclos anidados |
| `romper`/`continuar` | Probado: `mientras`/`para cada`, dentro de `si` y `intentar`, nivel superior y función; pendiente: ciclos anidados y rechazo fuera de ciclo |
| `intentar`/`rescatar` | Probado: error capturado, fin normal, retorno, señales no capturadas; pendiente: anidamiento profundo, error dentro de rescate y errores léxicos/sintácticos |
| E/S | stdin determinista; UTF-8 y EOF; archivo inexistente/permisos; modos; error tras cerrar; recursos y rutas |

### Fuera del MVP v0.4
- módulos avanzados
- paquetes y dependencias reales
- librería estándar extensa
- IDE o depurador
- concurrencia
- entrada/salida compleja
- compatibilidad móvil/escritorio/web
- marca, dominio y publicación institucional

### Criterio de aceptación de la versión
La v0.4 se considera terminada cuando:
- un usuario puede escribir un script simple y ejecutarlo con la CLI,
- se ejecutan ejemplos básicos sin errores de base,
- los comandos principales funcionan sin ambigüedad,
- el alcance está documentado y no se añade más funcionalidad sin acuerdo,
- `cargo test` y `cargo clippy --all-targets -- -D warnings` están en verde.

### Evidencia
- documento de alcance de v0.4 aprobado
- perfil de conformidad candidato escrito y pendiente de revisión/aprobación formal
- lista de features fuera de alcance
- ejemplos mínimos ejecutables
- checklist de aceptación en verde
- prueba integrada `aceptacion_v04_ejecuta_el_nucleo_integrado` ejecuta por CLI variables, arreglo e indexación, ciclo, condición, función y texto
- prueba `aceptacion_v04_diagnostica_errores_con_archivo_y_ubicacion` verifica salida fallida y diagnóstico de runtime con archivo, línea, columna y causa

## Fase 2 — Revisión técnica del runtime

### Objetivo
Dejar el intérprete estable y consistente para el subconjunto apoyado.

### Tareas
- revisar [prototipo/src/evaluador.rs](../prototipo/src/evaluador.rs)
- verificar casos límite de tipos y flujo
- corregir errores no cubiertos por la suite existente
- limpiar warnings visibles
- documentar el comportamiento de `retornar`, ciclos y errores
- corregir la mutabilidad y limpieza del ámbito de iteradores (regresiones ADR-0012 añadidas y pasando)
- corregir los límites de rangos y la propagación de `romper`/`continuar` a través de `intentar`/`rescatar`
- distinguir el fin normal de un bloque de función del error `RetornoAusente`

### Evidencia
- `cargo test` en verde
- `cargo clippy --all-targets -- -D warnings` en verde

## Fase 3 — Estabilizar la CLI

### Objetivo
Que la herramienta sea ejecutable y consistente para usuarios reales.

### Tareas
- revisar `basilisco run`, `bsk run`, `--help` y `--version`
- definir salida estándar y errores
- cerrar manejo de extensiones `.bsk` y `.nexo`
- validar ejecución con ejemplos reales

### Evidencia
- pruebas CLI en verde
- ayuda de uso clara
- ejemplos ejecutables documentados

## Fase 4 — Documentación de uso (completada localmente)

### Objetivo
Permitir que una persona sin contexto pueda usar el lenguaje de forma básica.

### Tareas
- crear tutorial mínimo
- documentar sintaxis soportada
- poner ejemplos ejecutables
- registrar limitaciones y casos no soportados

### Evidencia
- tutorial de 3 a 5 ejemplos sencillos
- README actualizado con la versión v0.4
- tutorial de cuatro ejemplos ejecutables en `prototipo/tutorial/01-primer-programa.md`
- CLI y tutorial cubiertos por pruebas de integración

## Fase 5 — Release candidate

### Objetivo
Preparar la versión publicable del proyecto dentro del alcance cerrado.

### Tareas
- revisar la lista de cambios
- preparar changelog
- confirmar que toda la evidencia está en el repo
- marcar la versión como candidata estable

### Evidencia
- changelog de v0.4
- release notes con alcance y límites

## Regla de avance
No se añade funcionalidad nueva mientras no se cumplan estas condiciones:

- la base actual está estable
- una prueba de aceptación fallida está cerrada
- el cambio encaja dentro del alcance de v0.4

## Orden recomendado de trabajo
1. alcance de v0.4 y checklist de aceptación
2. runtime y evaluación
3. CLI y ejecución real
4. documentación mínima
5. release candidate

## Siguiente acción inmediata
Resolver la lista de pendientes de publicación en `CAMBIOS-0.4.md`: fijar la versión de paquete, confirmar CI sobre un commit final y aportar evidencia de instaladores/plataformas objetivo antes de declarar una distribución. La ruta local `cargo install`/`cargo uninstall` ya pasó en Linux.

## Verificación actual
Verificación local ejecutada el 2026-10-25, después de los cambios en CLI, evaluador y documentación:

```bash
cd /home/auron/Escritorio/nexo/prototipo
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --bins
cargo test --quiet
cargo test -- --list
```

Resultado local del árbol actual: todos los comandos pasan; Clippy sin advertencias; 302 pruebas enumeradas y 0 fallos. El estado de CI remota no es verificable desde este entorno; no se presenta como aprobado.

## Decisión clave
La prioridad no es “hacer más lenguaje”, sino “dejar una versión útil y terminable”. Eso es lo que marca la diferencia entre un prototipo que se queda así y una base que puede crecer con criterio.
