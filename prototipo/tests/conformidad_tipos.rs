use basilisco::{Entorno, ErrorEjecucion, Lexer, Parser, Valor};

fn ejecutar(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida");
    Entorno::nueva().ejecutar_programa(&sentencias)
}

fn valor(fuente: &str) -> Valor {
    ejecutar(fuente)
        .expect("ejecución sin error")
        .expect("produce valor")
}

#[test]
fn conversion_implicita_entero_a_decimal() {
    let fuente = "var x: decimal = 5\nx";
    assert_eq!(valor(fuente), Valor::Decimal(5_000_000_000));
}

#[test]
fn conversion_implicita_entero_a_real() {
    let fuente = "var x: real = 5\nx";
    assert_eq!(valor(fuente), Valor::Real(5.0));
}

#[test]
fn suma_entero_decimal_promueve_a_decimal() {
    assert_eq!(valor("2 + 3.0"), Valor::Decimal(5_000_000_000));
}

#[test]
fn division_enteros_trunca() {
    assert_eq!(valor("7 / 2"), Valor::Entero(3));
}

#[test]
fn nulo_solo_se_compara_con_nulo() {
    assert_eq!(valor("nulo == nulo"), Valor::Logico(true));
    assert_eq!(valor("nulo != nulo"), Valor::Logico(false));
}

#[test]
fn texto_no_se_convierte_implicitamente() {
    let fuente = "var x: texto = 5";
    assert!(ejecutar(fuente).is_err());
}

#[test]
fn a_texto_convierte_explicitamente() {
    assert_eq!(valor("texto(42)"), Valor::Texto("42".into()));
    assert_eq!(valor("texto(verdadero)"), Valor::Texto("verdadero".into()));
}
