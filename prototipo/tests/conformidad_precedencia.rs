use basilisco::{Entorno, ErrorEjecucion, Lexer, Parser, Valor};

fn eval(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida");
    Entorno::nueva().ejecutar_programa(&sentencias)
}

fn valor(fuente: &str) -> Valor {
    eval(fuente)
        .expect("ejecución sin error")
        .expect("produce valor")
}

#[test]
fn multiplicacion_tiene_mayor_precedencia_que_suma() {
    assert_eq!(valor("2 + 3 * 4"), Valor::Entero(14));
}

#[test]
fn parentesis_altera_precedencia() {
    assert_eq!(valor("(2 + 3) * 4"), Valor::Entero(20));
}

#[test]
fn comparacion_menor_que_suma() {
    assert_eq!(valor("1 + 2 < 4"), Valor::Logico(true));
}

#[test]
fn no_tiene_mayor_precedencia_que_comparacion() {
    assert_eq!(valor("no 2 < 1"), Valor::Logico(true));
}

#[test]
fn y_tiene_mayor_precedencia_que_o() {
    assert_eq!(valor("falso o verdadero y falso"), Valor::Logico(false));
}

#[test]
fn concatenacion_texto_con_suma() {
    assert_eq!(valor("\"a\" + \"b\""), Valor::Texto("ab".into()));
}
