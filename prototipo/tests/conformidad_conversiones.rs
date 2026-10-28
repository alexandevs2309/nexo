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
fn entero_overflow_es_error() {
    assert!(ejecutar("9223372036854775807 + 1").is_err());
}

#[test]
fn decimal_overflow_es_error() {
    assert!(ejecutar("a_decimal(\"99999999999999999999.9\")").is_err());
}

#[test]
fn nan_no_es_igual_a_si_mismo() {
    let fuente = "a_real(\"NaN\") == a_real(\"NaN\")";
    assert_eq!(valor(fuente), Valor::Logico(false));
}

#[test]
fn menos_cero_igual_a_cero() {
    assert_eq!(valor("-0.0 == 0.0"), Valor::Logico(true));
}

#[test]
fn conversion_entero_a_decimal_explicita_con_a_decimal() {
    assert_eq!(valor("a_decimal(\"5\")"), Valor::Decimal(5_000_000_000));
}

#[test]
fn conversion_texto_invalida_es_error() {
    assert!(ejecutar("a_entero(\"abc\")").is_err());
}
