# Matriz de capacidades de BASILISCO

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
| Paquetes | PARCIAL | paquetes.rs, cli.rs | paquetes.rs, proyectos.rs | registro remoto | Media |
| CLI | IMPLEMENTADO | cli.rs | cli.rs | - | - |
| Biblioteca estándar | PARCIAL | evaluador.rs | conformidad_* | cobertura limitada | Media |
| IR/VM | AUSENTE | - | - | arquitectura | Baja |
| Nativo/WASM | AUSENTE | - | - | backends | Baja |
| FFI | AUSENTE | - | - | diseño | Baja |
| Concurrencia | PARCIAL | evaluador.rs (tareas) | - | robustez | Media |