# Cambios de Base Ultra 0.3.1

> **Documento histórico.** Describe la edición 0.3.1; para el estado actual de identidad y CLI, ver `CAMBIOS-BASILISCO.md`.

- Auditoría cruzada realizada contra la rama `main` del repositorio `alexandevs2309/nexo`.
- Corregida la gramática de referencia para reflejar la precedencia aprobada en ADR-0007.
- Documentado que `funcion`, `retornar` y `mostrar` están reservados por el lexer, pero que ADR-0008 está propuesto y el parser/evaluador aún no implementan funciones.
- Documentado que las cadenas se tokenizan, pero no son primarias admitidas por el parser actual.
- Registrado el límite de 100 000 iteraciones por ciclo del prototipo.
- No se modificó el repositorio GitHub ni el código Rust.
- No se ejecutaron `cargo test`, `cargo clippy` ni `cargo fmt`; se recomienda hacerlo en CI para confirmar el estado del commit actual.
