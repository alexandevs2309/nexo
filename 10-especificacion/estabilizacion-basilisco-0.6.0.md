# Informe de estabilización — BASILISCO 0.6.0

**Fecha:** 2026-11-01  
**Autor:** Auditoría técnica integral  
**Rama:** `main` — commit `db272c7`  
**Versión declarada:** `prototipo/Cargo.toml` → `0.6.0`

---

## 1. Estado real de BASILISCO 0.6.0

BASILISCO 0.6.0 es un intérprete de árbol (Tree-Walk) funcional, escrito en Rust, con
lexer, parser, evaluador, análisis semántico parcial, sistema de módulos, CLI, biblioteca
estándar básica, gestor local de paquetes y registro remoto R6 implementado y verificado.
La suite tiene **591 pruebas, 0 fallos, 2 ignoradas** (requieren PostgreSQL aislado).
Clippy, fmt y build pasan sin errores en local.

### Hallazgo documental crítico (corregido en este informe)

La sincronización documental del 2026-11-01 declaró FFI como "RETIRADA DELIBERADAMENTE"
basándose en que el `Cargo.toml` del remoto eliminó `libffi`/`libloading`. Sin embargo,
la inspección directa del código fuente en HEAD (`db272c7`) demuestra que:

- `prototipo/Cargo.toml` **sí contiene** `libffi = "5.2"` y `libloading = "0.9"`.
- `prototipo/src/evaluador.rs` **sí implementa** `cargar_ffi`, `llamar_ffi` y
  `descargar_ffi` usando `libloading::Library`.
- Los tests `tests/ffi.rs` existen y están activos.
- La suite compila y pasa con FFI incluida.

**Conclusión:** FFI no fue retirada del código. El commit `dda5f05` la introdujo y los
commits posteriores (R6) no la eliminaron del evaluador ni del Cargo.toml. La decisión
documental de marcarla como "retirada" fue prematura. FFI está **IMPLEMENTADA-PARCIAL**
en el estado actual del repositorio.

**Acción correctiva:** Los documentos actualizados el 2026-11-01 deben revertir las
afirmaciones sobre FFI. Ver sección 11 (Inconsistencias documentales).

---

## 2. Resultados de pruebas y CI

### Ejecución local (2026-11-01, commit db272c7)

| Comando | Resultado |
|---|---|
| `cargo test` | ✅ 591 pasadas, 0 fallidas, 2 ignoradas |
| `cargo fmt --check` | ✅ Sin diferencias |
| `cargo clippy --all-targets -- -D warnings` | ✅ Sin advertencias |
| `cargo build --bins` | ✅ Binarios `basilisco`, `bsk`, `basilisco-registry` compilados |

### Tests ignorados

| Test | Archivo | Razón |
|---|---|---|
| `ciclo_publicacion_descarga_instalacion` | `registry_integration.rs:119` | Requiere PostgreSQL aislado |
| `concurrencia_misma_version` | `registry_integration.rs:308` | Requiere PostgreSQL aislado |

Ambos corren en CI bajo el job `e2e-postgres` con PostgreSQL 16 como servicio.

### CI configurada

El workflow `.github/workflows/ci.yml` cubre:

- **Job `build`:** Ubuntu, macOS, Windows (matrix). Ejecuta fmt, clippy, build y
  `cargo test --workspace`. También instala y verifica los binarios.
- **Job `e2e-postgres`:** Ubuntu únicamente. Levanta PostgreSQL 16 y ejecuta los
  tests ignorados con `--ignored`.

### Diferencias local vs CI

- Los tests de `registry_integration.rs` solo corren en CI (con PostgreSQL).
- CI usa `cargo clippy --all-features` mientras que local usa `--all-targets`.
  No genera diferencias en la práctica con el conjunto de features actual.
- La suite de `ffi.rs` requiere un compilador C en el sistema; si no existe, los
  tests se omiten con un aviso (no fallan). En CI esto puede comportarse distinto
  según el runner.

---

## 3. Inventario de funcionalidades

| Componente | Estado real | Evidencia código | Evidencia pruebas |
|---|---|---|---|
| Lexer | IMPLEMENTADO | `lexer.rs` | `conformidad.rs`, `conformidad_tipos.rs` |
| Parser | IMPLEMENTADO | `parser.rs` | `conformidad_sintaxis` (dentro de conformidad.rs) |
| Evaluador tree-walk | IMPLEMENTADO | `evaluador.rs` | `integracion.rs`, `conformidad*.rs` |
| Variables mut/inmut | IMPLEMENTADO | `evaluador.rs` | `variables.rs` |
| Funciones nombradas | IMPLEMENTADO | `evaluador.rs` | `conformidad_funciones.rs` |
| Funciones de primera clase | IMPLEMENTADO | `valor.rs`, `evaluador.rs` | `closures.rs`, `lambdas.rs` |
| Lambdas | IMPLEMENTADO | `parser.rs`, `evaluador.rs` | `lambdas.rs` |
| Closures con captura | IMPLEMENTADO | `valor.rs` (`FuncionClausura`) | `closures.rs` |
| Funciones anidadas | IMPLEMENTADO | `evaluador.rs` | `conformidad_funciones.rs` |
| Recursividad | IMPLEMENTADO | `evaluador.rs` | `conformidad_funciones.rs` |
| Llamadas indirectas | IMPLEMENTADO | `evaluador.rs` | `lambdas.rs`, `semantico.rs` |
| Sistema de tipos estático | PARCIAL | `semantico.rs` | `semantico.rs` |
| Módulos | IMPLEMENTADO | `cargador.rs`, `semantico.rs` | `modulos_e2e.rs`, `semantico_modulos.rs` |
| Paquetes locales | IMPLEMENTADO | `paquetes.rs`, `cli.rs` | `paquetes.rs`, `proyectos.rs` |
| Registro remoto R6 | IMPLEMENTADO | `servidor/` | `registry_integration.rs` (CI) |
| CLI `bsk`/`basilisco` | IMPLEMENTADO | `cli.rs` | `cli.rs` |
| Biblioteca estándar básica | PARCIAL | `evaluador.rs` (builtins) | `conformidad_io.rs`, `conformidad_texto.rs` |
| Formateador | IMPLEMENTADO | `formateador.rs` | tests CLI |
| Concurrencia básica | PARCIAL | `evaluador.rs` (`iniciar_tarea`) | sin suite dedicada |
| FFI/ABI C | IMPLEMENTADO-PARCIAL | `evaluador.rs` (cargar/llamar/descargar) | `ffi.rs` |
| IR/VM | AUSENTE | — | — |
| Nativo/WASM | AUSENTE | — | — |

---

## 4. Problemas detectados

### P1 — Inconsistencia documental sobre FFI (crítico)

Los documentos actualizados el 2026-11-01 declaran FFI como "RETIRADA DELIBERADAMENTE"
pero el código HEAD la tiene completamente implementada con `libffi` y `libloading`.
Esta inconsistencia puede generar confusión en decisiones de desarrollo futuras.

**Estado real:** IMPLEMENTADA-PARCIAL (ABI C, escalares y texto; sin callbacks/structs/variádicos).

### P2 — Race condition en tests paralelos (resuelto)

Tests `semantico_modulos.rs`, `conformidad_io.rs` y `cli.rs` usaban `as_nanos()` para
generar directorios temporales, causando colisiones bajo ejecución paralela en macOS CI.
**Corregido** usando contador atómico + ID de hilo en todos los archivos afectados.

### P3 — Análisis semántico incompleto (GAP-026 abierto)

`analizar` ejecuta lexer, parser y un pase semántico parcial (`semantico.rs`), pero no
detecta todos los errores que `run` detecta en tiempo de evaluación. La divergencia más
notable: errores de tipo en expresiones complejas, E1010 en contextos de argumentos, y
accesos a módulos con símbolos privados en algunos casos de borde.

### P4 — Inferencia de tipos por primera llamada (comportamiento frágil)

El evaluador aplica "inferencia por primera llamada" (`tipos_parametros_inferidos`) para
funciones sin anotaciones. Este mecanismo fija los tipos en la primera invocación y
rechaza llamadas posteriores con tipos distintos aunque sean compatibles. Es correcto por
diseño pero no está completamente documentado y puede sorprender al usuario.

### P5 — `Cargo.toml` local vs remoto

El `Cargo.toml` local tiene `tower-http` con features `["cors", "trace", "fs"]` mientras
el remoto lo tiene con `["trace", "request-id", "set-header", "timeout"]`. Esto no afecta
los tests actuales pero indica que el `Cargo.toml` local no está sincronizado con el HEAD
del repositorio. El `git diff HEAD -- prototipo/Cargo.toml` está vacío, lo que indica que
el archivo en disco coincide con HEAD; la divergencia es entre el HEAD actual y el commit
anterior antes del pull.

---

## 5. Diagnóstico de E1010

### Origen del error

`E1010` corresponde a `CausaError::RetornoAusente { funcion: String }`.

### Cuándo se genera

Existen **dos contextos** distintos:

#### Contexto A — Llamada usada como expresión (correcto)

En `evaluar_con` (línea ~2468), cuando `Expresion::Llamada` devuelve `None`:

```rust
evaluar_llamada_directa(nombre, argumentos, ...)?.ok_or_else(|| {
    error(CausaError::RetornoAusente { funcion: nombre.clone() }, ...)
})
```

Aquí el error es **correcto**: si el resultado de la llamada se usa como valor
(asignación, argumento, expresión), una función sin retorno debe fallar.

#### Contexto B — Llamada usada como sentencia (correcto)

En `evaluar_expresion_sentencia` (línea ~3025), la función despacha
`evaluar_llamada_directa` y **no** convierte `None` en error:

```rust
Expresion::Llamada { ... } => evaluar_llamada_directa(nombre, argumentos, ..., env),
```

Aquí retorna `Ok(None)` silenciosamente, lo cual es **el comportamiento correcto**.

#### Caso especial — Función sin retorno como argumento (correcto)

```basilisco
funcion sin_retorno() { sea x = 1 }
funcion consumir(valor) { retornar valor }
consumir(sin_retorno())  // → E1010
```

Este caso genera E1010 porque `sin_retorno()` se evalúa en contexto de expresión
(como argumento de `consumir`). El error es **semánticamente correcto**.

### Veredicto

**E1010 no es un defecto.** El mecanismo dual existe y funciona:
- Como sentencia → `Ok(None)`, sin error.
- Como expresión → E1010 si no hay valor.

Los 4 tests de conformidad que cubren estos casos pasan:
- `func_sin_retorno_como_sentencia_y_continuacion` ✅
- `func_retorno_se_propaga_desde_bloques_con_llamada_sin_retorno` ✅
- `func_sin_retorno_en_argumento_sigue_generando_e1010` ✅
- `clausura_sin_retorno_como_sentencia` ✅

**No se requiere corrección.**

---

## 6. Correcciones realizadas y archivos modificados

### Correcciones previas a este informe (2026-11-01)

| Archivo | Problema | Corrección |
|---|---|---|
| `tests/semantico_modulos.rs` | Race condition temporal | Contador atómico + ID hilo |
| `tests/conformidad_io.rs` | Race condition temporal | Contador atómico + ID hilo |
| `tests/cli.rs` | Race condition temporal | Contador atómico + ID hilo |

### Correcciones documentales (2026-11-01)

| Archivo | Corrección |
|---|---|
| `README.md` | Corregida afirmación "No hay FFI"; añadida nota sobre R6 |
| `prototipo/README-registry.md` | Actualizado de "WIP" a R6 CERRADA con evidencia |
| `10-especificacion/ffi-abi.md` | Añadida cabecera de estado (que debe revertirse; ver sección 11) |
| `10-especificacion/registro-de-brechas.md` | GAP-015 y FFI actualizados |
| `10-especificacion/matriz-capacidades-basilisco.md` | Paquetes actualizado a IMPLEMENTADO; FFI actualizado |
| `10-especificacion/auditoria-integral-basilisco.md` | Resumen ejecutivo y §E Paquetes actualizados |
| `10-especificacion/comparativa-lenguajes-basilisco.md` | Lambdas/closures/paquetes/FFI corregidos |

### Corrección requerida por este informe

| Archivo | Corrección pendiente |
|---|---|
| `10-especificacion/ffi-abi.md` | Revertir cabecera "RETIRADA DELIBERADAMENTE" a "IMPLEMENTADA-PARCIAL" |
| `10-especificacion/registro-de-brechas.md` | GAP-015 debe volver a PARCIAL, no RETIRADA |
| `10-especificacion/matriz-capacidades-basilisco.md` | FFI → PARCIAL, no RETIRADA |
| `README.md` | Ajustar párrafo de FFI según estado real |

---

## 7. Pruebas nuevas y resultados

### Pruebas añadidas en esta auditoría

No se añadieron pruebas nuevas porque:

1. E1010 ya está cubierto por 4 tests existentes que pasan.
2. No se encontraron huecos reales en la cobertura de funciones, closures o lambdas.
3. Los módulos tienen cobertura completa en `semantico_modulos.rs` y `modulos_e2e.rs`.

La suite actual de 591 pruebas cubre adecuadamente las capacidades implementadas.

### Resultado final tras correcciones

```
cargo test:    591 pasadas, 0 fallidas, 2 ignoradas (PostgreSQL)
cargo fmt:     OK
cargo clippy:  OK
cargo build:   OK (basilisco, bsk, basilisco-registry)
```

---

## 8. Estado del sistema de tipos

### Tipos primitivos

| Tipo | Anotación | Inferencia | Validación estática |
|---|---|---|---|
| `entero` | ✅ | ✅ por primera llamada | ✅ en semantico.rs |
| `decimal` | ✅ | ✅ | ✅ |
| `real` | ✅ | ✅ | ✅ |
| `logico` | ✅ | ✅ | ✅ |
| `texto` | ✅ | ✅ | ✅ |
| `nulo` | ✅ (retorno) | ❌ | Parcial |

### Tipos compuestos

| Tipo | Anotación | Inferencia | Validación estática |
|---|---|---|---|
| `arreglo<T, N>` | ✅ | ✅ (desde literal) | ✅ tamaño y tipo |
| `lista<T>` | ✅ | ✅ | ✅ tipo elemento |
| Mapa/diccionario | ❌ | ❌ | ❌ |
| Struct/registro | ❌ | ❌ | ❌ |

### Funciones como tipos

| Capacidad | Estado |
|---|---|
| `funcion(params) -> retorno` como tipo de variable | ✅ en semantico.rs |
| Firma de lambda con anotación | ✅ |
| Compatibilidad de firmas en llamadas indirectas | ✅ |
| Funciones de orden superior con tipo genérico | ❌ |

### Matriz de capacidades del sistema de tipos

| Capacidad | Implementada | Parcial | Ausente | Evidencia |
|---|---|---|---|---|
| Tipos primitivos | ✅ | | | `parser.rs TipoAnotacion` |
| Anotaciones en variables | ✅ | | | `evaluador.rs` |
| Anotaciones en parámetros | ✅ | | | `evaluador.rs` |
| Anotaciones en retorno | ✅ | | | `evaluador.rs` |
| Inferencia local básica | ✅ | | | `semantico.rs` |
| Inferencia por primera llamada | | ✅ | | `evaluador.rs tipos_parametros_inferidos` |
| Conversiones implícitas aprobadas | | ✅ | | `evaluador.rs convertir_para_tipo` |
| Tipos de función | | ✅ | | `TipoSem::Funcion` |
| Arreglos y listas tipados | | ✅ | | `TipoAnotacion::Arreglo/Lista` |
| Tipos opcionales / `nulo` seguro | | | ✅ | — |
| Tipos unión | | | ✅ | — |
| Genéricos | | | ✅ | — |
| Validación estática completa (GAP-026) | | ✅ | | `semantico.rs` incompleto |

### Viabilidad de evolución

1. **Inferencia más completa:** viable sobre `semantico.rs` con un algoritmo de
   propagación bidireccional. Dependencia: GAP-026 cerrado primero.
2. **Tipos opcionales:** alta prioridad. La representación de `Valor::Nulo` existe;
   falta semántica de acceso seguro y propagación.
3. **Tipos compuestos (mapas, structs):** requiere ADR. El parser y el evaluador
   necesitan extensión no trivial.
4. **Tipos unión:** depende de compuestos y del análisis de exhaustividad.
5. **Genéricos:** requieren monomorfización o tipado estructural. Complejidad alta;
   depende de todo lo anterior.

---

## 9. Estado del manejo de errores

### Clasificación actual

| Categoría | Representación | Cobertura | Notas |
|---|---|---|---|
| Errores léxicos | `ErrorSintactico` con línea/columna | ✅ | Bien cubiertos |
| Errores sintácticos | `ErrorSintactico` con línea/columna | ✅ | Bien cubiertos |
| Errores semánticos | `ErrorEjecucion` con `CausaError` | Parcial | Detectados en evaluación, no siempre en `analizar` |
| Errores de ejecución | `ErrorEjecucion` con código E#### | ✅ | 38 causas distintas catalogadas |
| Errores de módulos | `CausaError::ModuloNoEncontrado`, `CicloDeImportacion` | ✅ | — |
| Errores de paquetes | Propios de `paquetes.rs` | Parcial | — |
| Errores de E/S | `ArchivoNoEncontrado`, `ErrorDeArchivo`, `PermisosDenegados` | ✅ | — |

### Todos los códigos E#### catalogados

`E1001`–`E1010`, `E1011`–`E1040` (FFI), y varios sin código asignado aún.

### Catálogo de errores (38 CausaError distintos)

`Aritmetica`, `VariableNoDefinida`, `DeclaracionDuplicada`, `AsignacionIncompatible`,
`FirmaInvalida`, `RetornoAusente (E1010)`, `RetornoIncompatibleConAnotacion`,
`TipoParametroIncompatible`, `TiposIncompatibles`, `CondicionNoLogica`,
`OperandoNoLogico`, `NombreReservado`, `ReasignacionInmutable`, `RetornarFueraDeBloque`,
`LimiteIteraciones`, `IteracionSobreNoColeccion`, `ModificacionDuranteIteracion`,
`RangoTipoInvalido`, `IndiceTextoFueraDeRango`, `ConversionTextoInvalida`,
`EntradaInvalida`, `SeparadorVacio`, `ArchivoNoEncontrado`, `ArchivoYaCerrado`,
`ErrorDeArchivo`, `PermisosDenegados`, `ModuloNoEncontrado`, `CicloDeImportacion`,
`ImportacionDuplicada`, `ImportacionesNoDisponibles`, `SimbodoNoExportado`,
`SimbodoPrivado`, `SentenciaNoPermitidaEnModulo`, `RutaNoPermitida`,
`ExtensionNoSoportada`, `TipoDeRetornoIncompatible`, `FFIInvalido`, `SenalRomper`,
`SenalContinuar`.

### Propuesta de evolución

El manejo de errores actual es correcto pero el usuario no tiene acceso a un tipo
`Resultado<V, E>` dentro del lenguaje. La propuesta para la siguiente fase:

1. Introducir `Opcion` (equivalente a `Option`) como tipo del lenguaje, con acceso
   seguro mediante `si es_nulo(x)`.
2. Introducir `Resultado` (equivalente a `Result`) para funciones que pueden fallar
   de forma tipada.
3. Introducir un operador de propagación (`?` o equivalente en español).
4. Todo esto requiere un ADR antes de implementación.

---

## 10. Estado de la biblioteca estándar

### Funciones builtin actuales (41 en NOMBRES_RESERVADOS)

| Área | Funciones | Estado |
|---|---|---|
| E/S básica | `mostrar`, `mostrar_sin_salto`, `leer`, `leer_entero`, `leer_decimal`, `leer_real` | ✅ |
| Archivos | `abrir`, `leer_linea`, `leer_todo`, `escribir`, `escribir_linea`, `cerrar`, `fin_de_archivo`, `existe_archivo`, `leer_archivo`, `escribir_archivo` | ✅ |
| Texto | `longitud`, `contiene`, `empieza_con`, `termina_con`, `es_vacio`, `mayusculas`, `minusculas`, `recortar`, `reemplazar`, `subcadena`, `dividir`, `unir` | ✅ |
| Conversiones | `a_entero`, `a_decimal`, `a_real`, `texto` | ✅ |
| Matemática | `abs`, `max`, `min`, `raiz`, `potencia`, `redondear`, `piso`, `techo` | ✅ |
| Colecciones | `map`, `filtrar`, `reducir`, `posicion`, `repetir`, `agregar`, `eliminar`, `invertir`, `ordenar` | ✅ |
| Concurrencia | `iniciar_tarea`, `esperar_tarea` | Parcial |
| FFI | `cargar_ffi`, `llamar_ffi`, `descargar_ffi` | Parcial |

### Áreas ausentes por prioridad de valor

| Área | Prioridad | Bloqueante | Nota |
|---|---|---|---|
| JSON (parseo/serialización) | Alta | No | Añadible como builtin sobre `serde_json` ya en Cargo |
| Variables de entorno | Alta | No | Añadible con `std::env` |
| Procesos externos | Media | No | Añadible con `std::process` |
| Fechas y horas | Media | No | `chrono` ya está en Cargo |
| HTTP (cliente) | Media | Tipos opcionales | Sin errores tipados no se puede modelar bien |
| Expresiones regulares | Media | No | Requiere nueva dependencia |
| TOML (parseo) | Baja | No | `toml` ya está en Cargo |
| Bases de datos | Baja | Tipos opcionales + Result | No viable sin manejo de errores tipado |
| Criptografía | Baja | No | `sha2`, `ed25519-dalek` ya en Cargo |

**Recomendación:** JSON, variables de entorno y fechas son los candidatos más
inmediatos porque las dependencias ya existen en Cargo.toml y no requieren cambios
arquitectónicos.

---

## 11. Inconsistencias documentales detectadas y corregidas

### Inconsistencia principal: FFI

La sincronización documental del 2026-11-01 marcó FFI como "RETIRADA DELIBERADAMENTE"
basándose en los diffs del pull, sin verificar el código fuente en HEAD. La inspección
real demuestra que FFI **sigue implementada**. Los documentos deben corregirse:

| Documento | Estado incorrecto | Estado correcto |
|---|---|---|
| `10-especificacion/ffi-abi.md` | "RETIRADA DELIBERADAMENTE" | "IMPLEMENTADA-PARCIAL" |
| `10-especificacion/registro-de-brechas.md` | GAP-015 "RETIRADA" | GAP-015 "PARCIAL" |
| `10-especificacion/matriz-capacidades-basilisco.md` | FFI "RETIRADA-DELIBERADAMENTE" | FFI "PARCIAL" |
| `README.md` | Párrafo de explicación de retirada | Estado real: implementada con dependencias |

### Otras inconsistencias (ya corregidas el 2026-11-01)

- `README.md`: "No hay FFI" → corregido.
- `prototipo/README-registry.md`: "WIP, no declarar R6 cerrada" → corregido.
- `auditoria-integral-basilisco.md`: "Registro remoto: ausente" → corregido.
- `comparativa-lenguajes-basilisco.md`: Lambdas/closures "No" → corregido a "Sí".

---

## 12. Riesgos técnicos pendientes

| Riesgo | Severidad | Descripción |
|---|---|---|
| GAP-026: análisis semántico incompleto | Alta | `analizar` no detecta todos los errores antes de ejecutar; puede causar regresiones silenciosas al ampliar el sistema de tipos |
| Inferencia por primera llamada | Media | Fija tipos en runtime; comportamiento sorpresivo para usuarios; difícil de ampliar hacia genéricos |
| Dos parsers de `basilisco.toml` | Media | Subconjuntos diferentes pueden generar comportamientos distintos; debe unificarse antes de garantizar compatibilidad TOML |
| `Cargo.toml` con features desactualizadas | Baja | `tower-http` features `["cors", "fs"]` pueden no usarse; aumento de tamaño de compilación |
| FFI sin gestión de señales C++ | Media | Comportamientos no definidos del callee se propagan al proceso; documentado como límite conocido |
| Concurrencia sin cancelación ni canales | Alta | `iniciar_tarea`/`esperar_tarea` no son aptas para producción; cualquier expansión requiere ADR |
| Tests `ffi.rs` con compilador C ausente | Baja | Se omiten silenciosamente en CI runners sin `cc`; pérdida de cobertura invisible |

---

## 13. Propuesta de la siguiente fase

### Orden recomendado con justificación técnica

#### Fase 1: Cierre de GAP-026 — Análisis semántico completo

**Objetivo:** que `analizar` detecte todos los errores que `run` detectaría, antes de
ejecutar.

**Problema que resuelve:** los errores semánticos hoy se descubren en ejecución.
Esto hace que el comando `analizar` no sea fiable como herramienta de verificación.
Es el bloqueante de todo el sistema de tipos avanzado.

**Componentes afectados:** `semantico.rs`, tests `semantico.rs`, `semantico_modulos.rs`.

**Dependencias:** ninguna.

**Riesgos:** puede generar falsos positivos en código que actualmente pasa; requiere
suite negativa exhaustiva.

**Criterios de aceptación:**
- Mismos errores detectados por `run` detectables con `analizar` para los casos
  cubiertos por los tests existentes.
- Sin falsos positivos en la suite actual de 591 pruebas.

**Complejidad:** Media.

---

#### Fase 2: Sistema de tipos — Inferencia y opcionales

**Objetivo:** inferencia bidireccional básica y tipo `Opcion<T>`.

**Problema que resuelve:** sin inferencia, todas las variables necesitan anotación o
dependen del mecanismo frágil de primera llamada. Sin `Opcion<T>` no hay forma segura
de representar ausencia de valor.

**Componentes afectados:** `parser.rs` (nuevo TipoAnotacion), `evaluador.rs`,
`semantico.rs`.

**Dependencias:** GAP-026 cerrado primero; de lo contrario, los nuevos tipos no
podrán validarse estáticamente.

**Riesgos:** cambio de semántica en variables no anotadas; ruptura de comportamiento
de la inferencia por primera llamada.

**Criterios de aceptación:**
- ADR aprobado antes de implementación.
- Pruebas positivas y negativas para `Opcion`.
- La suite existente sigue en verde.

**Complejidad:** Alta.

---

#### Fase 3: Biblioteca estándar — JSON, entorno, fechas

**Objetivo:** añadir funciones builtin de alto valor que no requieren cambios
arquitectónicos.

**Problema que resuelve:** el 80% de los programas reales necesitan al menos JSON
y variables de entorno. Las dependencias (`serde_json`, `chrono`) ya existen.

**Componentes afectados:** `evaluador.rs` (nuevos builtins), `NOMBRES_RESERVADOS`,
tests nuevos.

**Dependencias:** ninguna (puede hacerse en paralelo con Fase 1).

**Riesgos:** los nombres de las funciones deben ser consistentes con la convención
española del lenguaje; requieren ADR de nomenclatura.

**Criterios de aceptación:**
- `json_parsear(texto)`, `json_serializar(valor)`.
- `entorno_var(nombre)`, `entorno_vars()`.
- `fecha_ahora()`, `fecha_formatear(fecha, formato)`.
- Suite de pruebas para cada función.

**Complejidad:** Baja-Media.

---

#### Fase 4: Manejo de errores tipado — `Resultado<V, E>`

**Objetivo:** introducir `Resultado` como tipo del lenguaje con propagación.

**Problema que resuelve:** actualmente los errores solo se manejan con
`intentar/rescatar`. No hay forma de representar "función que puede fallar" en el
sistema de tipos ni propagación automática.

**Componentes afectados:** `parser.rs`, `evaluador.rs`, `semantico.rs`, gramática.

**Dependencias:** Fase 2 (sistema de tipos con opcionales primero).

**Riesgos:** cambio significativo en la gramática; puede entrar en conflicto con
`intentar/rescatar` si no se diseña con cuidado.

**Complejidad:** Alta.

---

#### Fase 5: Tipos compuestos — Mapas y structs

**Objetivo:** `mapa<K, V>` y registros con campos nombrados.

**Dependencias:** Fases 2 y 4 (tipos opcionales y manejo de errores tipado).

**Complejidad:** Alta (requiere ADR de modelo de memoria).

---

#### Fase 6: IR — Representación intermedia

**Objetivo:** diseñar e implementar una IR que permita optimizaciones y múltiples
backends.

**Dependencias:** el sistema de tipos debe estar estabilizado (Fases 1-3) para que
la IR tenga semántica completa.

**Complejidad:** Muy alta.

---

#### Fases 7-9: VM, Nativo, WASM, Concurrencia avanzada

Dependen de la IR. No deben acometerse sin ella.

---

### Tabla resumen

| Fase | Objetivo | Dep. | Complejidad | Prioridad |
|---|---|---|---|---|
| 1 | GAP-026: análisis semántico completo | — | Media | Inmediata |
| 2 | Inferencia + `Opcion<T>` | Fase 1 | Alta | Siguiente |
| 3 | Stdlib: JSON, entorno, fechas | — | Baja-Media | Paralela a Fase 1 |
| 4 | `Resultado<V, E>` | Fase 2 | Alta | Tras Fase 2 |
| 5 | Mapas y structs | Fases 2+4 | Alta | Tras Fase 4 |
| 6 | IR | Fases 1-3 estables | Muy alta | Futuro |
| 7+ | VM, Nativo, WASM, Concurrencia | IR | Muy alta | Futuro |

---

## 14. Recomendación del próximo objetivo de desarrollo

**Comenzar por la Fase 3 (stdlib básica) en paralelo con el diseño de Fase 1.**

Razón: la Fase 3 tiene valor tangible e inmediato para cualquier usuario del
lenguaje, no requiere cambios arquitectónicos, las dependencias ya están en
`Cargo.toml`, y puede avanzarse mientras se diseña y revisa el ADR del análisis
semántico completo. No es un sustituto de la Fase 1 — es trabajo que no la bloquea.

El ADR de la Fase 1 debe escribirse y aprobarse antes de tocar `semantico.rs` para
evitar introducir más inconsistencias entre `analizar` y `run`.

---

## Apéndice: Acción requerida sobre documentos FFI

Los siguientes archivos deben actualizarse para reflejar que FFI está **IMPLEMENTADA-PARCIAL**
en el código, no retirada:

1. `10-especificacion/ffi-abi.md` — revertir cabecera de estado.
2. `10-especificacion/registro-de-brechas.md` — GAP-015 a PARCIAL.
3. `10-especificacion/matriz-capacidades-basilisco.md` — FFI a PARCIAL.
4. `README.md` — ajustar párrafo según estado real.

Esta corrección no reabre ningún trabajo; solo alinea la documentación con el código.
