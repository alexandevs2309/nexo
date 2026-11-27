# FASE 14.1-C — Corrección forense FE-02: Resolución semántica de miembros de módulos

> **Fecha:** 2026-11-27  
> **Rama:** `main` | **HEAD:** `bb1bfed` (verificado; worktree sucio preexistente preservado)  
> **Hallazgo origen:** FE-02 (HIGH) en `docs/reports/FASE_14_1_AUDITORIA_FRONTEND.md`

## 1. Causa raíz

En `ContextoSemantico`, `Sentencia::Usar { alias, ... }` registraba el alias como variable en `scopes` con `TipoSem::Desconocido`, pero no conservaba la asociación semántica alias→módulo canónico en un estado scope-aware.  

En `Expresion::Acceso`, cuando el objeto era un alias de módulo, el análisis caía al camino genérico: analizaba el objeto (Desconocido), no resolvía miembros de exportación y retornaba `Desconocido` sin diagnóstico. En `LlamadaIndirecta` (`lib.func(...)`), el callee es un `Acceso`; al devolver Desconocido, el chequeo de llamada indirecta no emitía error de miembro inexistente/privado.  

El runtime sí aplicaba la frontera de exportación (`SimbodoPrivado`/`SimbodoNoExportado`), por lo que el error se descubría tarde.

## 2. Diseño de la solución

- **Tracking scope-aware:** se agregó `modulos_alias_scopes: Vec<HashMap<String, PathBuf>>` paralelo a `scopes` en `ContextoSemantico`, inicializado en `nuevo()` y guardado/restaurado en `analizar_modulo_ast` junto con `scopes`.
- **Balance de ámbitos:** se sincronizó push/pop de alias scopes en todos los puntos donde se hace push/pop de `scopes` (8 pares existentes), incluyendo el caso de `analizar_cuerpo_metodo` y casos con orden `gestor.salir(); scopes.pop()`.
- **Registro de alias:** en `Usar` con alias, tras registrar la variable en scope/tabla, se inserta `alias -> canonical` en el alias scope actual.
- **Resolución con sombreado:** `resolver_alias_modulo` recorre `scopes` y `modulos_alias_scopes` en reversa; si el nombre se encuentra en un scope de variables, solo resuelve como alias si ese mismo scope tiene el alias. Un sombreado local no-alias oculta el alias.
- **Acceso a miembro:** en `Expresion::Acceso`, si el objeto es `Variable` y resuelve a alias de módulo, se analiza el objeto para preservar side tables LSP/referencias, luego se resuelve el miembro contra `ModuloInfo.exports`/`nombres`: exportado variable→tipo, exportado función→`TipoSem::Funcion`, exportado nominal→`Desconocido` conservador, presente no exportado→`SimbodoPrivado`, ausente→`SimbodoNoExportado`. Si el módulo no está cargado en el contexto, emite `ModuloNoEncontrado`.
- **Llamadas `lib.func(...)`:** cubiertas por el camino de `Acceso` (callee evaluado como `Acceso` devuelve firma de función para exportados; para privado/inexistente emite diagnóstico y devuelve Desconocido, evitando doble error en `LlamadaIndirecta`).
- **Sin estado global contaminante:** el mapeo es por ámbito paralelo; `analizar_modulo_ast` lo aísla/restaura.

No se cambió sintaxis, no se modificó runtime, no se añadió resolución por nombre global sin ámbito.

## 3. Archivos modificados

- `prototipo/src/semantico.rs` — campo `modulos_alias_scopes`, init/save/restore, push/pop sincronizados, `resolver_alias_modulo`, registro en `Usar` alias, resolución FE-02 en `Acceso`.
- `prototipo/src/cargador.rs` — corrección FE-01 previa de esta línea de trabajo (ubicación top-level módulo).
- `prototipo/tests/semantico_modulos.rs` — 14 pruebas de regresión FE-02.
- `prototipo/tests/modulos_e2e.rs` — regresión FE-01 previa.
- `docs/reports/FASE_14_1_AUDITORIA_FRONTEND.md` — reporte misión anterior.
- `docs/reports/FASE_14_1C_CORRECCION_FE02.md` — este reporte.

## 4. Comportamiento antes y después

| Caso | Antes | Después |
|---|---|---|
| `usar "lib.bsk" como lib; lib.noexiste` | Sin diagnóstico semántico (`Desconocido`), runtime lo detectaba | Diagnóstico semántico `SimbodoNoExportado` con línea/columna |
| `usar "lib.bsk" como lib; lib.secreto` (privado) | Sin diagnóstico semántico, runtime lo detectaba | Diagnóstico semántico `SimbodoPrivado` con línea/columna |
| `lib.func()` público | Aceptado en runtime; semántico no tipaba bien el acceso por alias | Semántico resuelve firma exportada y type-checkea llamada |
| Miembro público existente `lib.valor` | Runtime OK; semántico no lo resolvía por alias | Semántico resuelve tipo exportado |

## 5. Pruebas de regresión

Agregadas en `prototipo/tests/semantico_modulos.rs` (prefijo `fe02_`):

- Resolución: `fe02_acceso_miembro_publico_existente`, `fe02_acceso_miembro_inexistente_diagnostico_semantico`, `fe02_acceso_miembro_privado_desde_otro_modulo`, `fe02_miembro_privado_mismo_modulo_via_alias_rechazado_por_contrato_exportacion` (contrato de exportación por frontera de módulo), `fe02_llamada_funcion_publica_por_alias`, `fe02_llamada_miembro_inexistente_por_alias`, `fe02_llamada_miembro_privado_por_alias`.
- Aislamiento: `fe02_dos_aliases_mismo_modulo_sin_contaminacion`, `fe02_dos_modulos_mismo_nombre_miembro_aislados`, `fe02_alias_sombreado_por_variable_local_no_resuelve_como_modulo`, `fe02_alias_mismo_nombre_en_ambitos_diferentes`, `fe02_modulos_anidados_aislamiento_alias`.
- Integración/coherencia: `fe02_acceso_publico_valido_runtime_semantico_coherente`, `fe02_diagnostico_conserva_ubicacion`.

## 6. Resultados exactos de comandos

Ejecutados en `prototipo/` salvo indicado:

```text
$ cargo test --test semantico_modulos fe02
running 14 tests
test fe02_acceso_miembro_inexistente_diagnostico_semantico ... ok
test fe02_acceso_miembro_privado_desde_otro_modulo ... ok
test fe02_acceso_miembro_publico_existente ... ok
test fe02_acceso_publico_valido_runtime_semantico_coherente ... ok
test fe02_alias_mismo_nombre_en_ambitos_diferentes ... ok
test fe02_alias_sombreado_por_variable_local_no_resuelve_como_modulo ... ok
test fe02_diagnostico_conserva_ubicacion ... ok
test fe02_dos_aliases_mismo_modulo_sin_contaminacion ... ok
test fe02_dos_modulos_mismo_nombre_miembro_aislados ... ok
test fe02_llamada_miembro_inexistente_por_alias ... ok
test fe02_llamada_funcion_publica_por_alias ... ok
test fe02_llamada_miembro_privado_por_alias ... ok
test fe02_miembro_privado_mismo_modulo_via_alias_rechazado_por_contrato_exportacion ... ok
test fe02_modulos_anidados_aislamiento_alias ... ok
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out
```

```text
$ cargo test --test semantico
test result: ok. 109 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

```text
$ cargo test --test sintaxis_modulos
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

```text
$ cargo test --test semantico_modulos
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

```text
$ cargo test --test modulos_e2e
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

```text
$ cargo fmt --check
(no output, exit 0)
```

```text
$ git diff --check
OK (exit 0)
```

No se ejecutó `cargo test --workspace` completo; no se declara aprobación global del workspace.

## 7. Riesgos residuales

- **LSP/refs:** se preserva el análisis de la variable alias (`analizar_expresion(objeto)`) para side tables; sin embargo, accesos a miembros de módulo no registran referencias simbólicas al miembro exportado del módulo origen (limitación preexistente del modelo de side tables). No afecta diagnóstico FE-02.
- **Tipos nominales exportados vía acceso alias:** se retorna `TipoSem::Desconocido` conservador para `ExportadoSem::TipoNominal` accedido como `alias.Tipo`; construcción/uso nominal ya tiene caminos propios. No se amplió semántica.
- **Cobertura workspace no completa:** solo suites focales solicitadas.

## 8. Evidencia de no contaminación entre ámbitos

- `fe02_dos_aliases_mismo_modulo_sin_contaminacion`: dos alias al mismo módulo resuelven sin interferencia.
- `fe02_dos_modulos_mismo_nombre_miembro_aislados`: `valor` público coexiste en dos módulos; solo `mb.secreto` reporta `SimbodoPrivado`.
- `fe02_alias_sombreado_por_variable_local_no_resuelve_como_modulo`: alias sombreado por variable local en bloque interno; alias externo sigue resolviendo.
- `fe02_alias_mismo_nombre_en_ambitos_diferentes`: mismo nombre de alias en dos bloques diferentes apunta a módulos distintos sin fuga.
- `fe02_modulos_anidados_aislamiento_alias`: cadena b→c y acceso directo a c desde main mantienen aislamiento.

## 9. Estado final de FE-02

**RESOLVED**

Accesos inexistentes/privados por alias de módulo producen diagnóstico semántico (`SimbodoNoExportado`/`SimbodoPrivado`) con ubicación; miembros públicos válidos se resuelven; suites focales pasan; no se observaron regresiones conocidas atribuibles a la corrección.

## 10. Estado Git

- `main`, `bb1bfed`, worktree sucio preexistente preservado.
- `git diff --check`: PASS.
- No commits/push/merge/tags/releases/publicaciones. No se inició Fase 14.2 ni Fase 15. No se modificó `DEVELOPMENT_STATE.md` ni `ROADMAP_MASTER.md`.
