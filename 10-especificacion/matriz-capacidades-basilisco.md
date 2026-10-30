# Matriz de capacidades de BASILISCO

## Nota de Fase 1

- 498 pruebas aprobadas, 0 fallidas.
- `cargo fmt --check`: correcto.
- `cargo clippy --all-targets --all-features -- -D warnings`: correcto.
- Funciones de primera clase parciales (`Valor::Funcion(String)`).
- Sin closures, lambdas ni captura de entorno léxico.
- Funciones anidadas rechazadas por el parser.
- `Expresion::LlamadaIndirecta` devuelve `TipoSem::Desconocido` sin validar firma (deuda técnica Fase 2).
- Inferencia de funciones no anotadas limitada.
- Sin IR, VM, nativo ni WASM.
- Gestor de paquetes local.

# Matriz de capacidades de BASILISCO

| Funcionalidad | Estado | Evidencia código | Evidencia pruebas | Brecha | Prioridad |
|---|---|---|---|---|---|
| Lexer | IMPLEMENTADO | lexer.rs | conformidad_* | - | - |
| Parser | IMPLEMENTADO | parser.rs | conformidad_sintaxis | recuperación de errores | Media |
| Evaluador tree-walk | IMPLEMENTADO | evaluador.rs | integracion, conformidad | rendimiento | Alta |
| Variables mutables/inmutables | IMPLEMENTADO | evaluador.rs | variables.rs | - | - |
| Funciones nombradas | IMPLEMENTADO | evaluador.rs | conformidad_funciones | - | - |
| Funciones primera clase | PARCIAL | valor.rs, evaluador.rs | tests manuales | sin clausuras | Alta |
| Lambdas | AUSENTE | - | - | sintaxis y AST | Alta |
| Sistema de tipos estático | PARCIAL | semantico.rs | semantico.rs | inferencia limitada | Alta |
| Módulos | IMPLEMENTADO | cargador.rs, semantico.rs | modulos_e2e, semantico_modulos | ejecución en análisis | Media |
| Paquetes | PARCIAL | paquetes.rs, cli.rs | paquetes.rs, proyectos.rs | registro remoto | Media |
| CLI | IMPLEMENTADO | cli.rs | cli.rs | - | - |
| Biblioteca estándar | PARCIAL | evaluador.rs | conformidad_* | cobertura limitada | Media |
| IR/VM | AUSENTE | - | - | arquitectura | Baja |
| Nativo/WASM | AUSENTE | - | - | backends | Baja |
| FFI | AUSENTE | - | - | diseño | Baja |
| Concurrencia | PARCIAL | evaluador.rs (tareas) | - | robustez | Media |