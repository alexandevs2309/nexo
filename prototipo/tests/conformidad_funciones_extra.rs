use basilisco::{CausaError, Entorno, ErrorEjecucion, Lexer, Parser, Valor};

fn ejecutar(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida");
    Entorno::nueva().ejecutar_programa(&sentencias)
}

#[test]
fn aridad_incorrecta_es_error() {
    let fuente = "funcion f(a, b) { retornar a + b }\nf(1)";
    let e = ejecutar(fuente).unwrap_err();
    assert!(matches!(*e.causa, CausaError::FirmaInvalida { .. }));
}

#[test]
fn retorno_ausente_es_error() {
    let fuente = "funcion f() { 1 + 2 }\nf()";
    let e = ejecutar(fuente).unwrap_err();
    assert!(matches!(*e.causa, CausaError::RetornoAusente { .. }));
}

#[test]
fn recursion_factorial() {
    let fuente =
        "funcion fact(n) {\n  si n <= 1 { retornar 1 }\n  retornar n * fact(n - 1)\n}\nfact(5)";
    assert_eq!(ejecutar(fuente).unwrap().unwrap(), Valor::Entero(120));
}

#[test]
fn retorno_anotado() {
    let fuente = "funcion doble(n: entero) -> entero { retornar n * 2 }\ndoble(21)";
    assert_eq!(ejecutar(fuente).unwrap().unwrap(), Valor::Entero(42));
}
