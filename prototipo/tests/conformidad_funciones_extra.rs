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

#[test]
fn llamada_anticipada_nivel_superior_coincide_con_analizar() {
    let fuente = "mostrar_double()\nfuncion mostrar_double() { retornar 7 }";
    let r = ejecutar(fuente).expect("llamada anticipada válida");
    assert_eq!(r, Some(Valor::Entero(7)));
}

#[test]
fn mutual_recursion_nivel_superior() {
    let fuente = "funcion impar(n) {\n  si n == 0 { retornar \"no\" }\n  retornar par(n - 1)\n}\nfuncion par(n) {\n  si n == 0 { retornar \"si\" }\n  retornar impar(n - 1)\n}\npar(6)";
    let r = ejecutar(fuente).expect("recursión mutua válida");
    assert_eq!(r, Some(Valor::Texto("si".to_string())));
}

#[test]
fn funcion_duplicada_nivel_superior_sigue_erreur() {
    let fuente = "funcion f() { retornar 1 }\nfuncion f() { retornar 2 }";
    let e = ejecutar(fuente).unwrap_err();
    assert!(matches!(*e.causa, CausaError::DeclaracionDuplicada(_)));
}

#[test]
fn lambda_asignada_partner_antes_de_declarar_su_uso() {
    // Uso anticipado de lambda sigue siendo error: solo las funciones
    // declaradas se izan.
    let fuente = "doble(2)\nsea doble = funcion (x) { retornar x * 2 }";
    let e = ejecutar(fuente).unwrap_err();
    assert!(matches!(*e.causa, CausaError::VariableNoDefinida(_)));
}
