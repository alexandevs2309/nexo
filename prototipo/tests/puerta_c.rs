use basilisco::{Entorno, Lexer, Parser};

#[test]
fn retornar_fuera_de_funcion_es_error_sintactico_ubicado() {
    let fuente = "\n\nretornar 5";
    let tokens = Lexer::nuevo(fuente).tokenizar().unwrap();
    let error = Parser::nuevo(tokens).parsear_programa().unwrap_err();

    assert_eq!((error.linea, error.columna), (3, 1));
    assert!(error.mensaje.contains("dentro de una función"));
}

#[test]
fn retornar_vacio_en_funcion_conserva_ubicacion_real() {
    let fuente = "funcion vacia() {\n  retornar\n}\nvacia()";
    let tokens = Lexer::nuevo(fuente).tokenizar().unwrap();
    let sentencias = Parser::nuevo(tokens).parsear_programa().unwrap();
    let error = Entorno::nueva().ejecutar_programa(&sentencias).unwrap_err();

    assert_eq!((error.linea, error.columna), (2, 3));
}
