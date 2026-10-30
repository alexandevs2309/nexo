use basilisco::{Entorno, Lexer, Parser, Valor};

fn ejecutar(src: &str) -> Option<Valor> {
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut env = Entorno::nueva();
    env.ejecutar_programa(&ss).unwrap()
}

#[test]
fn closure_captura_y_vida_util() {
    let src = r#"
        funcion crear_sumador(x) {
            funcion sumar(yy) {
                retornar x + yy
            }
            retornar sumar
        }
        sea sumar10 = crear_sumador(10)
        sumar10(5)
    "#;
    assert_eq!(ejecutar(src), Some(Valor::Entero(15)));
}

#[test]
fn closure_dos_instancias_independientes() {
    let src = r#"
        funcion crear_sumador(x) {
            funcion sumar(yy) {
                retornar x + yy
            }
            retornar sumar
        }
        sea a = crear_sumador(10)
        sea b = crear_sumador(20)
        a(5) + b(5)
    "#;
    assert_eq!(ejecutar(src), Some(Valor::Entero(40)));
}

#[test]
fn closure_mutable() {
    let src = r#"
        funcion crear_contador() {
            sea x = 0
            funcion inc() {
                x = x + 1
                retornar x
            }
            retornar inc
        }
        sea c = crear_contador()
        c()
        c()
    "#;
    assert_eq!(ejecutar(src), Some(Valor::Entero(2)));
}

#[test]
fn closure_como_argumento() {
    let src = r#"
        funcion aplicar(f, x) {
            retornar f(x)
        }
        funcion doble(n) {
            retornar n * 2
        }
        aplicar(doble, 4)
    "#;
    assert_eq!(ejecutar(src), Some(Valor::Entero(8)));
}

#[test]
fn closure_retornada() {
    let src = r#"
        funcion obtener() {
            funcion interna(x) {
                retornar x + 1
            }
            retornar interna
        }
        obtener()(9)
    "#;
    assert_eq!(ejecutar(src), Some(Valor::Entero(10)));
}
