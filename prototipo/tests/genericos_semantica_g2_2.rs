//! Pruebas de semántica G2.2 (contrato cerrado, 2026-11-20).
//!
//! Trazabilidad G2.2 → G3 (post-auditoría G3):
//! - Se preservan los contratos estructurales G2.2: transporte de metadatos genéricos,
//!   identidad/ámbito de parámetros de tipo (sin herencia implícita), validación de turbofish
//!   (aridad, callee no genérico, ligadura), forwarding explícito con preservación de identidad,
//!   y ausencia de G4 (sin instanciación/monomorfización/especialización ejecutable).
//! - Algunas expectativas diagnósticas/operativas evolucionan bajo G3 autorizado:
//!   opacidad diferenciada (BAS-7013/E1048) y resolución por inferencia local sin materialización
//!   (BAS-7006/E1041 ya no aplica cuando la inferencia G3 es completa y válida). Estas
//!   divergencias son intencionales y trazables; no reabren G2.2.

use basilisco::{ContextoSemantico, Lexer, Parser};

fn analizar(fuente: &str) -> Vec<basilisco::ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico");
    let ss = Parser::nuevo(tokens).parsear_programa().expect("sintaxis");
    let mut ctx = ContextoSemantico::nuevo();
    ctx.analizar_programa(&ss)
}

fn ok(fuente: &str) {
    let e = analizar(fuente);
    assert!(
        e.is_empty(),
        "se esperaba análisis sin errores.\nfuente: {fuente}\nerrores: {e:?}"
    );
}

fn err_codigo(fuente: &str, codigo: &str) {
    let e = analizar(fuente);
    assert!(
        !e.is_empty(),
        "se esperaba error semántico.\nfuente: {fuente}"
    );
    let codigos: Vec<&str> = e.iter().map(|x| x.causa.codigo()).collect();
    assert!(
        codigos.contains(&codigo),
        "código inesperado.\nfuente: {fuente}\nesperado: {codigo}\nreal: {codigos:?}\nerrores: {e:?}"
    );
}

// --- Resolución de declaraciones y separación Param vs Desconocido / opacidad ---

#[test]
fn funcion_generica_transporte_parametros_y_retorno() {
    ok(
        "funcion identidad<T>(x: T) -> T {\n  retornar x\n}\nfuncion main() {\n  identidad::<entero>(1)\n}\nmain()",
    );
}

#[test]
fn funcion_generica_multiples_params_y_tipos_compuestos() {
    ok(
        "funcion combinar<T, U>(a: T, b: lista<U>) -> T {\n  retornar a\n}\nfuncion main() {\n  combinar::<entero, texto>(1, [\"a\"])\n}\nmain()",
    );
}

#[test]
fn parametro_generico_es_opaco_no_numerico() {
    // Param es distinto de Desconocido: no permite operaciones numéricas implícitas (V2, opacidad).
    // G3 (BAS-7013/E1048): operación prohibida por opacidad diferenciada.
    err_codigo("funcion f<T>(x: T) -> T {\n  retornar x + 1\n}\n", "E1048");
}

#[test]
fn parametro_generico_es_opaco_no_comparable() {
    let e = analizar("funcion f<T>(x: T, z: T) -> logico {\n  retornar x == z\n}\n");
    assert!(
        !e.is_empty(),
        "comparar parámetros opacos sin capacidades no debe permitirse implícitamente"
    );
}

// --- Identidad, anidación y visibilidad (G2.2 §3.3) ---

#[test]
fn funcion_anidada_con_mismo_nombre_param_tiene_identidad_propia() {
    ok(
        "funcion exterior<T>(x: T) -> T {\n  funcion interior<T>(v: T) -> T {\n    retornar v\n  }\n  retornar x\n}\n",
    );
}

#[test]
fn funcion_anidada_no_hereda_parametro_exterior_implicitamente() {
    // S8.1: T no declarado en interior se parsea como Nominal y se rechaza en semántica (BAS-7101).
    err_codigo(
        "funcion exterior<T>(x: T) {\n  funcion interior<U>(v: T) -> T {\n    retornar v\n  }\n}\n",
        "BAS-7101",
    );
}

#[test]
fn funcion_anidada_generica_independiente_valida() {
    ok(
        "funcion exterior<T>(x: T) -> T {\n  funcion interior<U>(v: U) -> U {\n    retornar v\n  }\n  retornar x\n}\n",
    );
}

// --- Turbofish: aridad, función no genérica, inferencia no disponible ---

#[test]
fn llamada_generica_explicita_aridad_correcta() {
    ok(
        "funcion par<A, B>(a: A, b: B) -> A {\n  retornar a\n}\nfuncion main() {\n  par::<entero, texto>(1, \"x\")\n}\nmain()",
    );
}

#[test]
fn llamada_generica_explicita_aridad_incorrecta() {
    err_codigo(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion main() {\n  id::<entero, texto>(1)\n}\nmain()",
        "E1043",
    );
}

#[test]
fn turbofish_en_funcion_no_generica_rechazado() {
    err_codigo(
        "funcion f(x: entero) -> entero {\n  retornar x\n}\nfuncion main() {\n  f::<entero>(1)\n}\nmain()",
        "E1042",
    );
}

#[test]
fn turbofish_en_builtin_rechazado() {
    err_codigo(
        "funcion main() {\n  longitud::<entero>([1, 2, 3])\n}\nmain()",
        "E1042",
    );
}

#[test]
fn llamada_generica_sin_argumentos_tipo_resuelta_por_inferencia_g3() {
    // G3: inferencia local disponible y unívoca (T=entero); sin instanciación/monomorfización (G4).
    ok("funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion main() {\n  id(1)\n}\nmain()");
}

#[test]
fn funcion_no_generica_sin_argumentos_tipo_sigue_valida() {
    ok("funcion f(x: entero) -> entero {\n  retornar x\n}\nf(1)\n");
}

// --- Forwarding explícito y ligadura válida ---

#[test]
fn forwarding_explicito_preserva_identidad_parametro() {
    ok(
        "funcion primero<T>(valor: T) -> T {\n  retornar valor\n}\nfuncion intermediaria<T>(valor: T) -> T {\n  retornar primero::<T>(valor)\n}\nfuncion main() {\n  intermediaria::<entero>(7)\n}\nmain()",
    );
}

#[test]
fn forwarding_explicito_multiples_parametros() {
    ok(
        "funcion segundo<A, B>(a: A, b: B) -> B {\n  retornar b\n}\nfuncion envuelve<A, B>(a: A, b: B) -> B {\n  retornar segundo::<A, B>(a, b)\n}\nfuncion main() {\n  envuelve::<entero, texto>(1, \"z\")\n}\nmain()",
    );
}

#[test]
fn llamada_generica_explicita_no_exige_compatibilidad_ordinaria_en_g22() {
    // Aridad/ligadura válidas: compatibilidad completa ordinaria/concreta diferida a G3.
    ok(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion main() {\n  var r: entero = id::<entero>(1)\n  r\n}\nmain()",
    );
}

// --- Ausencia de especialización / sin inferencia / sin ejecución genérica nueva ---

#[test]
fn sin_argumentos_tipo_explicitos_inferencia_g3_sin_instanciacion() {
    // G3: inferencia local resuelve argumentos de tipo; no implica instanciación/monomorfización (G4).
    ok("funcion id<T>(x: T) -> T {\n  retornar x\n}\nid(1)\n");
}

// --- Preservación de programas no genéricos ---

#[test]
fn preserva_programas_no_genericos_existentes() {
    ok(
        "funcion suma(a: entero, b: entero) -> entero {\n  retornar a + b\n}\nsea x = suma(2, 3)\nx\n",
    );
    ok("var xs: lista<entero> = [1, 2, 3]\nxs\n");
}

#[test]
fn funciones_no_genericas_homonimas_anidadas_sin_regresion() {
    ok(
        "funcion exterior() {\n  funcion interna() -> entero { retornar 1 }\n  interna()\n}\nexterior()\n",
    );
}

// --- Sombreado de variable no convierte llamada en genérica válida ---

#[test]
fn turbofish_sobre_variable_sombreante_rechazado() {
    err_codigo(
        "funcion f<T>(x: T) -> T {\n  retornar x\n}\nfuncion main() {\n  sea f = 1\n  f::<entero>(1)\n}\nmain()",
        "E1042",
    );
}
