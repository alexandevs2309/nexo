# Notas de implementación — corrección del prototipo BASILISCO

**Fecha:** aplicado en un solo commit  
**Archivos modificados:** `src/evaluador.rs`, `src/parser.rs`, `src/cli.rs`, `tests/variables.rs`

---

## Cambios aplicados

### Bug 1 — evaluador.rs: Ramas duplicadas eliminadas
Se eliminaron del `match` de `evaluar_con` la primera aparición de `Expresion::ArregloLiteral`
(que retornaba `Valor::Arreglo` incorrecto) y la primera aparición de `Expresion::Indice`
(que usaba `CausaError::TiposIncompatibles` para todos los errores). Quedan solo las
implementaciones correctas: `ArregloLiteral` crea `Valor::Lista`, e `Indice` distingue
`Arreglo` de `Lista` con mensajes detallados usando `CausaError::AsignacionIncompatible`.

### Bug 2 — evaluador.rs: Nueva variante `RetornarFueraDeBloque`
- Añadida `CausaError::RetornarFueraDeBloque` al enum con su rama en `Display`.
- En `ejecutar_sentencias`, el handler de `Sentencia::Retorno` ahora lanza
  `CausaError::RetornarFueraDeBloque` en vez de `CausaError::VariableNoDefinida("retornar")`.

### MP3 — evaluador.rs: Unificación de ejecutar_sentencias / ejecutar_funcion_sentencias
Se adoptó la **alternativa conservadora**: la lógica duplicada se mantiene en las dos funciones
existentes. `ResultadoEjecucion` conserva su `#[allow(dead_code)]` hasta que se complete
una refactorización futura. La deuda técnica sigue documentada en el plan original.

### MP4 — evaluador.rs: Prohibir redefinición silenciosa de funciones
El bloque `Sentencia::Funcion` en `ejecutar_sentencias` ahora lanza
`CausaError::DeclaracionDuplicada(nombre)` en lugar de silenciosamente reemplazar
la definición. Añadido test `funcion_duplicada_es_error`.

### MP5 — evaluador.rs: Limpieza de dead_code
- `existe_en_ambito_actual`: eliminado `#[allow(dead_code)]` (ya se usaba en ambas funciones).
- `obtener_variable_mut` / `obtener_variable_info`: comentarios actualizados a
  "reservado para uso futuro"; conservan `#[allow(dead_code)]`.
- `buscar_ambito_actual`: eliminado (no se usaba en ningún lugar).
- `obtener_mut_en_entorno` (función libre): eliminada (no se usaba).

### MP6 — evaluador.rs: Doble búsqueda en AsignacionIndice
Se mantiene el uso de `localizar_info_entorno` que ya devuelve `(usize, VariableInfo)`,
evitando una segunda búsqueda separada. La función `localizar_info_entorno` se conserva
ya que sigue siendo necesaria para el bloque `AsignacionIndice` en ambas funciones.

### MP7 — parser.rs + evaluador.rs: `TipoAnotacion::nombre()` retorna `String`
- `TipoAnotacion::nombre()` cambiado de `&'static str` a `String`.
  Ahora retorna `format!("arreglo<{}, {}>", t.nombre(), n)` para arreglos y
  `format!("lista<{}>", t.nombre())` para listas. Eliminado el comentario en inglés.
- `CausaError::AsignacionIncompatible.esperado` cambiado de `&'static str` a `String`.
- Todos los sitios de construcción en `evaluador.rs` actualizados con `.into()` o `.to_string()`.
- Tests en `tests/variables.rs` y `src/evaluador.rs` actualizados con `.to_string()`.

### L8 — evaluador.rs: Doc-comment duplicado
Eliminada la segunda copia del doc-comment antes de `fn evaluar_con`.

### L9 — cli.rs: Doc-comment en `principal()`
Añadido `/// Solo para la demo; evalúa una única expresión.` antes de `fn principal`.

### L10 — evaluador.rs: Tests duplicados eliminados
Eliminados de `evaluador::tests` los 9 tests que eran copia exacta de los de
`tests/conformidad_funciones.rs`:
- `funcion_suma_dos_parametros`
- `funcion_sin_parametros`
- `funcion_recursiva_factorial`
- `funcion_inferencia_tipos_primera_llamada`
- `funcion_con_tipo_decimal`
- `funcion_error_retorno_ausente`
- `funcion_ambito_propio_no_captura`
- `funcion_retorno_temprano`
- `funcion_anidada_rechazada`

Conservados los 4 tests que verifican `CausaError` con tipos concretos
(no duplicados exactos): `funcion_error_firma_invalida_pocos_args`,
`funcion_error_firma_invalida_muchos_args`, `funcion_error_tipo_retorno_incompatible`,
`funcion_retorno_sin_valor_en_funcion_que_espera_valor`.

### L11 — parser.rs: Test `debug_dato_tokens` eliminado
Eliminado el test sin assertions que solo hacía `eprintln!`. El test
`declaraciones_tipadas_var_dato_const` cubre el mismo caso con assertions reales.

---

## Resultado final

```
cargo build   →  Finished dev profile (0 warnings, 0 errors)
cargo test    →  102 passed, 0 failed, 0 ignored
```

Todos los archivos de conformidad (`conformidad_funciones.rs`, `conformidad_arreglos_listas.rs`,
`conformidad_control.rs`, `variables.rs`, etc.) pasan sin modificaciones en su contenido.
