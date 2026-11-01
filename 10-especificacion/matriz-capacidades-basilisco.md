# Matriz de capacidades de BASILISCO

## Nota de sincronización (2026-11-01)

- Corrección de race condition en tests paralelos (`semantico_modulos`, `conformidad_io`, `cli`): directorios temporales ahora usan contador atómico + ID de hilo para garantizar unicidad.
- R6 (registro remoto) **CERRADA** con evidencia contra PostgreSQL 16 real: servidor Axum/SQLx, auth Bearer, namespaces, publicación BPK (SHA-256 + Ed25519), cadena de confianza, descarga, yank, test E2E `registry_integration.rs` automatizado, persistencia verificada ante reinicio.
- **FFI RETIRADA DELIBERADAMENTE:** el puente ABI C (`cargar_ffi`/`llamar_ffi`/`descargar_ffi` sobre `libffi`/`libloading`) fue implementado en el commit `dda5f05` y retirado en el ciclo R6 para eliminar dependencias pesadas de compilación. La especificación técnica histórica se conserva en `10-especificacion/ffi-abi.md`. Su reimplementación está prevista en Fase 5 (interoperabilidad), sin IR estable.

## Nota de sincronización (2026-10-30)

- 551 pruebas aprobadas, 0 fallidas.
- `cargo fmt --check`: correcto.
- `cargo clippy --all-targets -- -D warnings`: correcto.
- Funciones de primera clase completas en el evaluador (`Valor::FuncionClausura`), con captura de entorno.
- Lambdas y closures con pruebas dedicadas (`tests/lambdas.rs`, `tests/closures.rs`).
- Funciones anidadas soportadas; semántica coherente entre `analizar`, `run` y el formateador.
- `Expresion::LlamadaIndirecta` valida firma, parámetros y retorno en `semantico.rs`.
- Inferencia de funciones no anotadas limitada.
- Sin IR, VM, nativo ni WASM.
- Gestor de paquetes local.

## Nota de Fase 1 (histórico, superada)

- 498 pruebas aprobadas, 0 fallidas.
- Funciones de primera clase parciales (`Valor::Funcion(String)`).
- Sin closures, lambdas ni captura de entorno léxico.
- Funciones anidada rechazadas por el parser.
- `Expresion::LlamadaIndirecta` sin validación estática.

# Matriz de capacidades de BASILISCO

| Funcionalidad | Estado | Evidencia código | Evidencia pruebas | Brecha | Prioridad |
|---|---|---|---|---|---|
| Lexer | IMPLEMENTADO | lexer.rs | conformidad_* | - | - |
| Parser | IMPLEMENTADO | parser.rs | conformidad_sintaxis | recuperación de errores | Media |
| Evaluador tree-walk | IMPLEMENTADO | evaluador.rs | integracion, conformidad | rendimiento | Alta |
| Variables mutables/inmutables | IMPLEMENTADO | evaluador.rs | variables.rs | - | - |
| Funciones nombradas | IMPLEMENTADO | evaluador.rs | conformidad_funciones | - | - |
| Funciones primera clase | IMPLEMENTADO | valor.rs, evaluador.rs | closures.rs, lambdas.rs, semantico.rs | - | - |
| Lambdas | IMPLEMENTADO | parser.rs, evaluador.rs, semantico.rs | lambdas.rs | - | - |
| Sistema de tipos estático | PARCIAL | semantico.rs | semantico.rs | inferencia limitada | Alta |
| Módulos | IMPLEMENTADO | cargador.rs, semantico.rs | modulos_e2e, semantico_modulos | ejecución en análisis | Media |
| Paquetes | IMPLEMENTADO (local + R6 remoto) | paquetes.rs, cli.rs, servidor/ | paquetes.rs, remoto_e2e.rs, server_auth.rs, registry_integration.rs | - | - |
| CLI | IMPLEMENTADO | cli.rs | cli.rs | - | - |
| Biblioteca estándar | PARCIAL | evaluador.rs | conformidad_* | cobertura limitada | Media |
| IR/VM | AUSENTE | - | - | arquitectura | Baja |
| Nativo/WASM | AUSENTE | - | - | backends | Baja |
| FFI | PARCIAL | evaluador.rs (cargar/llamar/descargar_ffi), libffi, libloading | ffi.rs | ABI C: escalares y texto; sin callbacks/structs/variádicos; diseño de seguridad pendiente | Fase 5 |
| Concurrencia | PARCIAL | evaluador.rs (tareas) | - | robustez | Media |