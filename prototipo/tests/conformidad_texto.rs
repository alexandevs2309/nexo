// Tests de conformidad para texto y mostrar (Fase C, Parte 1)

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
fn texto_literal_y_variable() {
    let r = run_program(r#"
        var mensaje: texto = "Hola"
        dato saludo: texto = "Mundo"
        const VACIO: texto = ""
        saludo
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Texto("Mundo".into())));
}

#[test]
fn texto_concatenacion_con_mas() {
    let r = run_program(r#"
        var a: texto = "Hola"
        var b: texto = "Mundo"
        a + b
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Texto("HolaMundo".into())));
}

#[test]
fn texto_concatenacion_multiple() {
    let r = run_program(r#"
        "a" + "b" + "c"
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Texto("abc".into())));
}

#[test]
fn texto_comparacion_igualdad() {
    assert_eq!(run_program(r#""hola" == "hola""#).unwrap(), Some(nexo::Valor::Logico(true)));
    assert_eq!(run_program(r#""hola" == "mundo""#).unwrap(), Some(nexo::Valor::Logico(false)));
    assert_eq!(run_program(r#""hola" != "mundo""#).unwrap(), Some(nexo::Valor::Logico(true)));
}

#[test]
fn texto_comparacion_orden() {
    assert_eq!(run_program(r#""a" < "b""#).unwrap(), Some(nexo::Valor::Logico(true)));
    assert_eq!(run_program(r#""b" > "a""#).unwrap(), Some(nexo::Valor::Logico(true)));
    assert_eq!(run_program(r#""a" <= "a""#).unwrap(), Some(nexo::Valor::Logico(true)));
    assert_eq!(run_program(r#""b" >= "a""#).unwrap(), Some(nexo::Valor::Logico(true)));
}

#[test]
fn texto_error_concatenacion_con_numero() {
    let err = run_program_err(r#""hola" + 1"#);
    assert!(err.contains("incompatibles") || err.contains("TiposIncompatibles"));
}

#[test]
fn texto_error_comparacion_con_numero() {
    let err = run_program_err(r#""hola" == 1"#);
    // El mensaje de error es "comparación incompatible entre texto y entero"
    assert!(err.contains("incompatible") || err.contains("TiposIncompatibles"));
}

#[test]
fn funcion_con_texto_parametro_y_retorno() {
    let r = run_program(r#"
        funcion saludar(nombre) {
            retornar "Hola " + nombre
        }
        saludar("Mundo")
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Texto("Hola Mundo".into())));
}

#[test]
fn funcion_texto_sin_parametros() {
    let r = run_program(r#"
        funcion saludo() {
            retornar "Hola"
        }
        saludo()
    "#).unwrap();
    assert_eq!(r, Some(nexo::Valor::Texto("Hola".into())));
}

#[test]
fn mostrar_literal_texto() {
    let tokens = nexo::Lexer::nuevo(r#"mostrar "Hola""#).tokenizar().unwrap();
    let sentencias = nexo::Parser::nuevo(tokens).parsear_programa().unwrap();
    let result = nexo::evaluador::Entorno::nueva().ejecutar_programa(&sentencias);
    assert!(result.is_ok());
}

#[test]
fn mostrar_variable_texto() {
    let tokens = nexo::Lexer::nuevo("var x: texto = \"Hola\"\nmostrar x").tokenizar().unwrap();
    let sentencias = nexo::Parser::nuevo(tokens).parsear_programa().unwrap();
    let result = nexo::evaluador::Entorno::nueva().ejecutar_programa(&sentencias);
    assert!(result.is_ok());
}

#[test]
fn mostrar_expresion_numerica() {
    let tokens = nexo::Lexer::nuevo("mostrar 42").tokenizar().unwrap();
    let sentencias = nexo::Parser::nuevo(tokens).parsear_programa().unwrap();
    let result = nexo::evaluador::Entorno::nueva().ejecutar_programa(&sentencias);
    assert!(result.is_ok());
}

#[test]
fn mostrar_multiple_veces() {
    let tokens = nexo::Lexer::nuevo("mostrar \"a\"\nmostrar \"b\"").tokenizar().unwrap();
    let sentencias = nexo::Parser::nuevo(tokens).parsear_programa().unwrap();
    let result = nexo::evaluador::Entorno::nueva().ejecutar_programa(&sentencias);
    assert!(result.is_ok());
}
