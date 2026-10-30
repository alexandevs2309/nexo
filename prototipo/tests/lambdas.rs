use basilisco::{Entorno, Lexer, Parser, Valor};

fn ejecutar(src: &str) -> Option<Valor> {
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut env = Entorno::nueva();
    env.ejecutar_programa(&ss).unwrap()
}

#[test]
fn lambda_asignacion() {
    let src = "sea doble = funcion (x) { retornar x * 2 }\ndoble(5)";
    assert_eq!(ejecutar(src), Some(Valor::Entero(10)));
}

#[test]
fn lambda_como_argumento() {
    let src = "map([1, 2, 3], funcion (x) { retornar x * 2 })";
    assert_eq!(
        ejecutar(src),
        Some(Valor::Lista(vec![
            Valor::Entero(2),
            Valor::Entero(4),
            Valor::Entero(6)
        ]))
    );
}

#[test]
fn lambda_retorno() {
    let src = "funcion obtener() { retornar funcion (x) { retornar x + 1 } }\nobtener()(9)";
    assert_eq!(ejecutar(src), Some(Valor::Entero(10)));
}

#[test]
fn lambda_captura() {
    let src = "funcion crear(x) { retornar funcion (yy) { retornar x + yy } }\ncrear(10)(5)";
    assert_eq!(ejecutar(src), Some(Valor::Entero(15)));
}
