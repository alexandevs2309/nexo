use basilisco::{Entorno, Lexer, Parser, Valor};

#[test]
fn tutorial_inicio_es_ejecutable_y_devuelve_el_resultado_documentado() {
    let fuente = include_str!("../examples/tutorial_inicio.bsk");
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .expect("lexer del tutorial");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("parser del tutorial");
    let resultado = Entorno::nueva()
        .ejecutar_programa(&sentencias)
        .expect("ejecución del tutorial");

    assert_eq!(resultado, Some(Valor::Entero(12)));
}
