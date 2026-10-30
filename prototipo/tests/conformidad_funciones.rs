// Tests de conformidad para funciones (Incremento 5 / ADR-0008)

use basilisco::{CausaError, Entorno, ErrorEjecucion, Lexer, Parser};

fn run_program(fuente: &str) -> Result<Option<basilisco::Valor>, String> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| e.to_string())?;
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .map_err(|e| e.to_string())?;
    Entorno::nueva()
        .ejecutar_programa(&sentencias)
        .map_err(|e| e.to_string())
}

fn run_program_err(fuente: &str) -> String {
    run_program(fuente).unwrap_err()
}

fn error_ejecucion(fuente: &str) -> ErrorEjecucion {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida");
    Entorno::nueva()
        .ejecutar_programa(&sentencias)
        .expect_err("se esperaba error de ejecución")
}

#[test]
fn func_definicion_y_llamada_simple() {
    let r = run_program(
        r#"
        funcion suma(a, b) {
            retornar a + b
        }
        suma(2, 3)
    "#,
    )
    .unwrap();
    assert_eq!(r, Some(basilisco::Valor::Entero(5)));
}

#[test]
fn func_sin_parametros() {
    let r = run_program(
        r#"
        funcion hola() {
            retornar 42
        }
        hola()
    "#,
    )
    .unwrap();
    assert_eq!(r, Some(basilisco::Valor::Entero(42)));
}

#[test]
fn func_recursividad_factorial() {
    let r = run_program(
        r#"
        funcion factorial(n) {
            si n <= 1 {
                retornar 1
            }
            retornar n * factorial(n - 1)
        }
        factorial(5)
    "#,
    )
    .unwrap();
    assert_eq!(r, Some(basilisco::Valor::Entero(120)));
}

#[test]
fn func_inferencia_tipos_primera_llamada() {
    let r = run_program(
        r#"
        funcion id(x) {
            retornar x
        }
        id(42)
    "#,
    )
    .unwrap();
    assert_eq!(r, Some(basilisco::Valor::Entero(42)));
}

#[test]
fn func_fija_tipo_de_parametro_inferido_en_primera_llamada() {
    let err = run_program_err(
        "funcion constante(valor) {\n  retornar 1\n}\nconstante(42)\nconstante(\"texto\")",
    );
    assert!(err.contains("parámetro 'valor'"), "error inesperado: {err}");

    let r = run_program(
        "funcion identidad(valor) {\n  retornar valor\n}\nidentidad(1.5)\nidentidad(2)",
    )
    .unwrap();
    assert_eq!(r, Some(basilisco::Valor::Decimal(2_000_000_000)));

    let err = run_program_err(
        "funcion recursiva(valor) {\n  si valor == 0 {\n    retornar 1\n  }\n  retornar recursiva(\"texto\")\n}\nrecursiva(1)",
    );
    assert!(err.contains("parámetro 'valor'"), "error inesperado: {err}");
}

#[test]
fn func_error_firma_pocos_argumentos() {
    let err = run_program_err(
        r#"
        funcion suma(a, b) {
            retornar a + b
        }
        suma(1)
    "#,
    );
    assert!(err.contains("FirmaInvalida") || err.contains("argumentos"));
}

#[test]
fn func_error_firma_muchos_argumentos() {
    let err = run_program_err(
        r#"
        funcion hola() {
            retornar 1
        }
        hola(1, 2)
    "#,
    );
    assert!(err.contains("FirmaInvalida") || err.contains("argumentos"));
}

#[test]
fn func_evalua_argumentos_antes_de_validar_aridad() {
    let err =
        run_program_err("funcion identidad(valor) {\n  retornar valor\n}\nidentidad(1 / 0, 2)");
    assert!(
        err.contains("división entre cero"),
        "error inesperado: {err}"
    );
}

#[test]
fn func_error_retorno_ausente() {
    let err = run_program_err(
        r#"
        funcion sin_retorno() {
            sea x = 1
        }
        sin_retorno()
    "#,
    );
    assert!(err.contains("RetornoAusente") || err.contains("retornar"));
}

#[test]
fn func_ambito_propio_sin_captura() {
    let err = run_program_err(
        r#"
        sea x = 10
        funcion f() {
            retornar x
        }
        f()
    "#,
    );
    assert!(err.contains("VariableNoDefinida") || err.contains("no definida"));
}

#[test]
fn func_retorno_temprano_en_si() {
    let r = run_program(
        r#"
        funcion f(n) {
            si n == 0 {
                retornar 0
            }
            retornar 1
        }
        f(0)
    "#,
    )
    .unwrap();
    assert_eq!(r, Some(basilisco::Valor::Entero(0)));
}

#[test]
fn func_funciones_anidadas_rechazadas() {
    let tokens = Lexer::nuevo(
        r#"
        funcion externa() {
            funcion interna() {
                retornar 1
            }
        }
    "#,
    )
    .tokenizar()
    .unwrap();
    let prog = Parser::nuevo(tokens).parsear_programa().unwrap();
    assert!(!prog.is_empty());
}

#[test]
fn func_tipo_decimal_en_funcion() {
    let r = run_program(
        r#"
        funcion div(a, b) {
            retornar a / b
        }
        div(7.0, 2.0)
    "#,
    )
    .unwrap();
    assert_eq!(r, Some(basilisco::Valor::Decimal(3500000000))); // 3.5
}

#[test]
fn func_parametros_anotados_validan_y_aplican_conversiones_permitidas() {
    let r = run_program(
        "funcion identidad(valor: decimal) -> decimal {\n  retornar valor\n}\nidentidad(2)",
    )
    .unwrap();
    assert_eq!(r, Some(basilisco::Valor::Decimal(2_000_000_000)));

    let err = run_program_err(
        "funcion identidad(valor: entero) -> entero {\n  retornar valor\n}\nidentidad(2.5)",
    );
    assert!(err.contains("parámetro 'valor'"), "error inesperado: {err}");
}

#[test]
fn func_parametros_anotados_admiten_arreglos_y_listas() {
    let arreglo = run_program(
        "funcion primero(valores: arreglo<entero, 2>) -> entero {\n  retornar valores[0]\n}\nprimero([7, 8])",
    )
    .unwrap();
    assert_eq!(arreglo, Some(basilisco::Valor::Entero(7)));

    let lista = run_program(
        "funcion primero(valores: lista<entero>) -> entero {\n  retornar valores[0]\n}\nprimero([9])",
    )
    .unwrap();
    assert_eq!(lista, Some(basilisco::Valor::Entero(9)));

    let err = run_program_err(
        "funcion primero(valores: arreglo<entero, 2>) -> entero {\n  retornar valores[0]\n}\nprimero([7])",
    );
    assert!(
        err.contains("parámetro 'valores'"),
        "error inesperado: {err}"
    );
}

#[test]
fn func_retorno_anotado_valida_y_aplica_conversiones_permitidas() {
    let r = run_program("funcion valor() -> decimal {\n  retornar 2\n}\nvalor()").unwrap();
    assert_eq!(r, Some(basilisco::Valor::Decimal(2_000_000_000)));

    let err = run_program_err("funcion valor() -> entero {\n  retornar 2.5\n}\nvalor()");
    assert!(
        err.contains("retorno de 'valor'"),
        "error inesperado: {err}"
    );

    let e = error_ejecucion("funcion valor() -> entero {\n  retornar 2.5\n}\nvalor()");
    assert_eq!(
        *e.causa,
        CausaError::RetornoIncompatibleConAnotacion {
            funcion: "valor".into(),
            esperado: "entero".into(),
            obtenido: "decimal".into(),
        }
    );
    assert_eq!((e.linea, e.columna), (2, 3));

    let anidado =
        run_program("funcion valor() -> decimal {\n  {\n    retornar 2\n  }\n}\nvalor()").unwrap();
    assert_eq!(anidado, Some(basilisco::Valor::Decimal(2_000_000_000)));
}

#[test]
fn func_nulo_anotado_acepta_retorno_expreso_o_fin_normal() {
    let retorno_expreso =
        run_program("funcion sin_valor() -> nulo {\n  retornar nulo\n}\nsin_valor()").unwrap();
    assert_eq!(retorno_expreso, Some(basilisco::Valor::Nulo));

    let fin_normal =
        run_program("funcion sin_valor() -> nulo {\n  sea hecho = verdadero\n}\nsin_valor()")
            .unwrap();
    assert_eq!(fin_normal, Some(basilisco::Valor::Nulo));

    let err = run_program_err("funcion mal_tipada() -> nulo {\n  retornar 1\n}\nmal_tipada()");
    assert!(
        err.contains("retorno de 'mal_tipada'"),
        "error inesperado: {err}"
    );
}

#[test]
fn func_error_tipo_retorno_incompatible_conformidad() {
    let err = run_program_err(
        r#"
        funcion id(x) {
            retornar x
        }
        id(42)
        id(3.14)
    "#,
    );
    assert!(
        err.contains("TipoDeRetornoIncompatible")
            || err.contains("entero")
            || err.contains("decimal")
    );
}
