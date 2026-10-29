use basilisco::{ContextoSemantico, Expresion, Lexer, Parser, Sentencia, Valor};

fn parsear(fuente: &str) -> Vec<Sentencia> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico");
    Parser::nuevo(tokens).parsear_programa().expect("sintaxis")
}

fn analizar(fuente: &str) -> Vec<basilisco::ErrorEjecucion> {
    let ss = parsear(fuente);
    let mut ctx = ContextoSemantico::nuevo();
    ctx.analizar_programa(&ss)
}

#[test]
fn sem_func_antes_uso() {
    let e = analizar("funcion f() { retornar 1 }\nf()");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_func_despues_uso() {
    let e = analizar("f()\nfuncion f() { retornar 1 }");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_recursion() {
    let e = analizar(
        "funcion fact(n) {\n  si n <= 1 { retornar 1 }\n  retornar n * fact(n - 1)\n}\nfact(5)",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_parametros_locales() {
    let e = analizar("funcion f(xx) {\n  sea yy = xx + 1\n  retornar yy\n}\nf(3)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_dos_funciones_mismo_nombre_local() {
    let e = analizar(
        "funcion a() {\n  sea xx = 1\n  retornar xx\n}\nfuncion b() {\n  sea xx = 2\n  retornar xx\n}\na()\nb()",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_ambitos_bloques() {
    let e =
        analizar("sea x = 1\nsi x == 1 {\n  sea yy = 2\n  yy\n} sino {\n  sea zz = 3\n  zz\n}\nx");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_identificador_no_definido() {
    let e = analizar("xx");
    assert!(!e.is_empty(), "debe rechazar identificador no definido");
}

#[test]
fn sem_declaracion_duplicada() {
    let e = analizar("sea x = 1\nsea x = 2");
    assert!(!e.is_empty(), "debe rechazar duplicado");
}

#[test]
fn sem_funcion_inexistente() {
    let e = analizar("noexiste()");
    assert!(!e.is_empty(), "debe rechazar función inexistente");
}

#[test]
fn sem_expresiones_llamadas() {
    let e = analizar("funcion suma(a, b) { retornar a + b }\nsuma(2, 3) + 1");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_neg_asignacion_no_definida() {
    let e = analizar("xx = 5");
    assert!(!e.is_empty());
}

#[test]
fn sem_sombreado_valido() {
    let e = analizar("sea x = 1\n{\n  sea x = 2\n  x\n}\nx");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_asignacion_incompatible() {
    let e = analizar("var x: entero = 1\nx = \"a\"");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_operacion_tipos_incompatibles() {
    let e = analizar("1 + \"a\"");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_arity_incorrecta() {
    let e = analizar("funcion f(a, b) { retornar a }\nf(1)");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_retorno_incompatible() {
    let e = analizar("funcion f() -> entero { retornar \"a\" }");
    // calling not needed; body analyzed
    let _ = analizar("funcion f() -> entero { retornar \"a\" }\nf()");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_programa_valido_complejo() {
    let src = "funcion doble(n) { retornar n * 2 }\nfuncion main() {\n  sea i = 0\n  sea acc = 0\n  mientras i < 3 {\n    acc = acc + doble(i)\n    i = i + 1\n  }\n  retornar acc\n}\nmain()";
    let e = analizar(src);
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_diagnostico_ubicacion() {
    let e = analizar("xx");
    assert!(!e.is_empty());
    assert_eq!(e[0].linea, 1);
    assert_eq!(e[0].columna, 1);
}

#[test]
fn sem_locales_aisladas_entre_funciones() {
    let e = analizar("funcion a() { sea xx = 1\n retornar xx }\nfuncion b() { retornar xx }");
    assert!(!e.is_empty(), "b no debe ver local de a: {:?}", e);
}

#[test]
fn sem_bloque_no_visible_fuera() {
    let e = analizar("{ sea interna = 1 }\ninterna");
    assert!(!e.is_empty(), "no debe ser visible fuera: {:?}", e);
}

#[test]
fn sem_sombreado_anidado_valido() {
    let e = analizar("sea x = 1\n{\n  sea x = 2\n  {\n    sea x = 3\n    x\n  }\n  x\n}\nx");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_variable_usada_antes_de_declaracion() {
    let e = analizar("xx\nsea xx = 1");
    assert!(
        !e.is_empty(),
        "debe rechazar uso antes de declaración: {:?}",
        e
    );
}

#[test]
fn sem_llamada_anticipada_y_recursion() {
    let e = analizar("f()\nfuncion f() { retornar g() }\nfuncion g() { retornar 7 }");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_argumento_incompatible() {
    let e = analizar("funcion f(x: entero) { retornar x }\nf(\"a\")");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_retorno_anotado_incompatible() {
    let e = analizar("funcion f() -> entero { retornar 1.5 }");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_funcion_sin_anotacion_retorno() {
    let e = analizar("funcion f() { retornar 1 }\nf()");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_conversiones_implicitas() {
    let e = analizar("var xx: decimal = 1\nvar yy: real = 2\nxx = 3\nyy = 4");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_operacion_mixta_valida() {
    let e = analizar("1 + 2.0 + 3.5");
    // 1 entero + 2.0 real => real; + 3.5 real => real
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_tipo_no_determinable_no_error() {
    let e = analizar("funcion f() { retornar 1 }\nsea x = f()\nx + 1");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_retornar_fuera_funcion() {
    let sentencias = vec![Sentencia::Retorno {
        valor: Some(Expresion::Literal {
            valor: Valor::Entero(1),
            linea: 1,
            columna: 1,
        }),
        linea: 1,
        columna: 1,
    }];
    let mut ctx = ContextoSemantico::nuevo();
    let e = ctx.analizar_programa(&sentencias);
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_condicion_no_logica() {
    let e = analizar("si 1 { 2 }");
    assert!(!e.is_empty(), "errores: {:?}", e);
    let e2 = analizar("mientras 1 { 2 }");
    assert!(!e2.is_empty(), "errores: {:?}", e2);
}

#[test]
fn sem_retorno_en_condicional() {
    let e = analizar("funcion f(x) {\n  si x > 0 {\n    retornar x\n  }\n  retornar 0\n}\nf(1)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_modulo_limitacion_no_engañosa() {
    // El analizador no resuelve `usar`; no debe fallar ni declarar éxito engañoso.
    // Aquí solo se verifica que un programa con `usar` no rompa el análisis.
    let e = analizar("usar \"lib.bsk\" como lib\n1");
    // Puede ser vacío o con diagnósticos, pero no debe panic.
    let _ = e;
}

#[test]
fn sem_dato_const_reasignacion() {
    let e = analizar("dato x: entero = 1\nx = 2");
    assert!(!e.is_empty(), "errores: {:?}", e);
    let e2 = analizar("const X: entero = 1\nX = 2");
    assert!(!e2.is_empty(), "errores: {:?}", e2);
}
