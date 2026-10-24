# Corrección de bugs y deuda técnica del intérprete BASILISCO

El commit corrige los dos bugs críticos del evaluador (ramas duplicadas en `evaluar_con` y error semántico en `retornar` fuera de función), resuelve seis de las siete malas prácticas del plan, y aplica los cuatro cambios de limpieza. El coder adoptó la alternativa conservadora para MP3 (no unificar `ejecutar_sentencias` / `ejecutar_funcion_sentencias`), explícitamente contemplada en el plan. El resultado reportado es `cargo build` sin warnings, `cargo test` con 102 tests pasando.

Watch for: (1) **MP5 parcial (confirmed)** — `obtener_variable_mut` y `obtener_variable_info` siguen con `#[allow(dead_code)]` cuando el plan decía eliminarlos. (2) **Asimetría en `AsignacionIndice` (confirmed)** — las dos copias del handler divergen en el bloque de mutación final, dificultando la eventual deduplicación de MP3.

**Verdict**: APPROVED

---

## High-level view

Los dos bugs críticos están resueltos. La primera `Expresion::ArregloLiteral` (devolvía `Valor::Arreglo` incorrecto) y la primera `Expresion::Indice` (con `TiposIncompatibles` genérico) fueron eliminadas; queda una sola rama correcta de cada una. `RetornarFueraDeBloque` existe en el enum, tiene su rama en `Display`, y `ejecutar_sentencias` la usa en lugar del hack `VariableNoDefinida("retornar")`.

MP4 está completo: `ejecutar_sentencias` lanza `DeclaracionDuplicada` antes de insertar una función, con un test nuevo que lo verifica. Los archivos de conformidad no fueron modificados.

MP7 está completo: `TipoAnotacion::nombre()` retorna `String`, el campo `esperado` de `CausaError::AsignacionIncompatible` cambió de `&'static str` a `String`, y todos los sitios de construcción en `evaluador.rs` y `tests/variables.rs` fueron actualizados.

MP5 está parcialmente completo. `buscar_ambito_actual` y `obtener_mut_en_entorno` fueron eliminados. Pero `obtener_variable_mut` y `obtener_variable_info` siguen presentes sin ningún call site, con `#[allow(dead_code)]` y un comentario "reservado para uso futuro" no documentado en ningún ADR — es exactamente la mala práctica que el plan buscaba eliminar.

---

<details>
<summary>Issues (2)</summary>

1. **MP5 incompleto — métodos sin uso conservados** — `obtener_variable_mut` y `obtener_variable_info` no tienen ningún call site en todo el crate y siguen con `#[allow(dead_code)]`. El plan decía eliminarlos. Eliminarlos, o si la intención es exponerlos como API pública futura, convertirlos en `pub` y documentarlo en el plan o en un ADR.

2. **Asimetría en copias de `AsignacionIndice`** — la copia en `ejecutar_funcion_sentencias` (~línea 368) usa `match` con brazo `_ => {}` para la mutación final; la copia en `ejecutar_sentencias` (~línea 765) usa `if let`. Comportamiento idéntico, código distinto. Unificar con `if let` en ambas para que las copias sean idénticas y MP3 (unificación pendiente) sea trivial.

</details>

<details>
<summary>Details</summary>

### Bug 2 — fallback de ubicación en `RetornarFueraDeBloque`

`ejecutar_sentencias` llama a `ubicacion_linea(valor.as_ref())` donde `valor: Option<Box<Expresion>>`. Cuando la sentencia es un `retornar;` bare (sin expresión), `valor` es `None` y `ubicacion_linea` devuelve línea 1 como fallback — no panic, pero el error apuntará a línea 1 en lugar de la línea real del `retornar`. Esto es suficiente para el prototipo, pero en producción se necesitaría preservar la ubicación de la sentencia `Retorno` independientemente del valor.

### MP5 — dos métodos sin uso conservados con `#[allow(dead_code)]`

`obtener_variable_mut` y `obtener_variable_info` no tienen ningún call site en el crate (confirmado con grep). El plan decía explícitamente eliminarlos. El coder los conservó con `#[allow(dead_code)]` y el comentario "reservado para uso futuro". Esto suprime el warning del compilador sin resolver la deuda — el mismo patrón que el plan identificó como MP5.

### Asimetría residual en las dos copias de `AsignacionIndice`

La diferencia está solo en el último paso, el bloque de mutación:

```rust
// ejecutar_funcion_sentencias (~línea 368):
match &mut info_mut.valor {
    Valor::Arreglo(v) | Valor::Lista(v) => {
        if idx_usize < v.len() { v[idx_usize] = ajustado; }
    }
    _ => {}
}

// ejecutar_sentencias (~línea 765):
if let Valor::Arreglo(v) | Valor::Lista(v) = &mut info_mut.valor {
    if idx_usize < v.len() { v[idx_usize] = ajustado; }
}
```

Semánticamente idénticos. La divergencia no introduce un bug ahora, pero cuando llegue MP3 (unificación de las dos funciones) habrá que resolver qué forma es la canónica. El `if let` es más idiomático para Rust 2024 y debería ser la forma unificada.

</details>

---

<details>
<summary>Archivos modificados</summary>

| Archivo | Cambio |
|---|---|
| `src/evaluador.rs` | Elimina ramas duplicadas; añade `RetornarFueraDeBloque`; MP3 conservador, MP4, MP5 parcial, MP6, MP7a; limpieza L8, L10 |
| `src/parser.rs` | MP7b — `TipoAnotacion::nombre()` retorna `String`; L11 — elimina `debug_dato_tokens` |
| `src/cli.rs` | L9 — doc-comment en `principal()` |
| `tests/variables.rs` | MP7 — actualiza literales `esperado:` a `.to_string()` |
| `.agents/tasks/implementation-notes.md` | Nuevo archivo |

Diff completo: `git -C prototipo diff HEAD~1`

</details>
