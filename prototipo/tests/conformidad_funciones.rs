// Tests de conformidad para funciones (Incremento 5 / ADR-0008)

use nexo::{Lexer, Parser, evaluador::Entorno};

fn run_program(fuente: &str) -> Result<Option<nexo::Valor>, String> {
    let tokens = Lexer::nuevo(fuente).tokenizar().map_err(|e| e.to_string())?;
    let sentencias = Parser::nuevo(tokens).parsear_programa().map_err(|e| e.to_string())?;
    Entorno::nueva().ejecutar_programa(&sentencias).map_err(|e| e.to_string())
}

fn run_program_err(fuente: &str) -> String {
    run_program(fuente).unwrap_err()
}

#[test]
fn func_definicion_y_llamada_simple() {
    let r = run_program(r#"
        funcion suma(a, b) {
            retornar a + b
        }
        suma(2, 3)
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Entero(5)));
}

#[test]
fn func_sin_parametros() {
    let r = run_program(r#"
        funcion hola() {
            retornar 42
        }
        hola()
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Entero(42)));
}

#[test]
fn func_recursividad_factorial() {
    let r = run_program(r#"
        funcion factorial(n) {
            si n <= 1 {
                retornar 1
            }
            retornar n * factorial(n - 1)
        }
        factorial(5)
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Entero(120)));
}

#[test]
fn func_inferencia_tipos_primera_llamada() {
    let r = run_program(r#"
        funcion id(x) {
            retornar x
        }
        id(42)
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Entero(42)));
}

#[test]
fn func_error_firma_pocos_argumentos() {
    let err = run_program_err(r#"
        funcion suma(a, b) {
            retornar a + b
        }
        suma(1)
    "#);
    assert!(err.contains("FirmaInvalida") || err.contains("argumentos"));
}

#[test]
fn func_error_firma_muchos_argumentos() {
    let err = run_program_err(r#"
        funcion hola() {
            retornar 1
        }
        hola(1, 2)
    "#);
    assert!(err.contains("FirmaInvalida") || err.contains("argumentos"));
}

#[test]
fn func_error_retorno_ausente() {
    let err = run_program_err(r#"
        funcion sin_retorno() {
            sea x = 1
        }
        sin_retorno()
    "#);
    assert!(err.contains("RetornoAusente") || err.contains("retornar"));
}

#[test]
fn func_ambito_propio_sin_captura() {
    let err = run_program_err(r#"
        sea x = 10
        funcion f() {
            retornar x
        }
        f()
    "#);
    assert!(err.contains("VariableNoDefinida") || err.contains("no definida"));
}

#[test]
fn func_retorno_temprano_en_si() {
    let r = run_program(r#"
        funcion f(n) {
            si n == 0 {
                retornar 0
            }
            retornar 1
        }
        f(0)
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Entero(0)));
}

#[test]
fn func_funciones_anidadas_rechazadas() {
    let tokens = Lexer::nuevo(r#"
        funcion externa() {
            funcion interna() {
                retornar 1
            }
        }
    "#).tokenizar().unwrap();
    let err = Parser::nuevo(tokens).parsear_programa().unwrap_err();
    assert!(err.to_string().contains("anidadas") || err.to_string().contains("nested"));
}

#[test]
fn func_tipo_decimal_en_funcion() {
    let r = run_program(r#"
        funcion div(a, b) {
            retornar a / b
        }
        div(7.0, 2.0)
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Decimal(3500000000))); // 3.5
}

#[test]
fn func_error_tipo_retorno_incompatible_conformidad() {
    let err = run_program_err(r#"
        funcion id(x) {
            retornar x
        }
        id(42)
        id(3.14)
    "#);
    assert!(err.contains("TipoDeRetornoIncompatible") || err.contains("entero") || err.contains("decimal"));
}
