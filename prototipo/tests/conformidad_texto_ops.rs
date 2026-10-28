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
fn longitud_cuenta_puntos_de_codigo() {
    assert_eq!(valor("longitud(\"hola\")"), Valor::Entero(4));
    assert_eq!(valor("longitud(\"ñ\")"), Valor::Entero(1));
}

#[test]
fn contiene_empieza_termina() {
    assert_eq!(
        valor("contiene(\"hola mundo\", \"mundo\")"),
        Valor::Logico(true)
    );
    assert_eq!(valor("empieza_con(\"hola\", \"ho\")"), Valor::Logico(true));
    assert_eq!(valor("termina_con(\"hola\", \"la\")"), Valor::Logico(true));
}

#[test]
fn mayusculas_minusculas_recortar() {
    assert_eq!(valor("mayusculas(\"hola\")"), Valor::Texto("HOLA".into()));
    assert_eq!(valor("minusculas(\"HOLA\")"), Valor::Texto("hola".into()));
    assert_eq!(valor("recortar(\"  hola  \")"), Valor::Texto("hola".into()));
}

#[test]
fn reemplazar_y_subcadena() {
    assert_eq!(
        valor("reemplazar(\"hola mundo\", \"mundo\", \"BASILISCO\")"),
        Valor::Texto("hola BASILISCO".into())
    );
    assert_eq!(
        valor("subcadena(\"hola\", 1, 3)"),
        Valor::Texto("ol".into())
    );
}

#[test]
fn dividir_y_unir() {
    assert_eq!(
        valor("dividir(\"a,b,c\", \",\")"),
        Valor::Lista(vec![
            Valor::Texto("a".into()),
            Valor::Texto("b".into()),
            Valor::Texto("c".into())
        ])
    );
    assert_eq!(
        valor("unir([\"a\", \"b\", \"c\"], \"-\")"),
        Valor::Texto("a-b-c".into())
    );
}

#[test]
fn conversiones_texto() {
    assert_eq!(valor("a_entero(\"42\")"), Valor::Entero(42));
    assert_eq!(valor("texto(verdadero)"), Valor::Texto("verdadero".into()));
    assert_eq!(valor("texto(nulo)"), Valor::Texto("nulo".into()));
}

#[test]
fn subcadena_fuera_de_rango_es_error() {
    let r = ejecutar("subcadena(\"hola\", 0, 10)");
    assert!(r.is_err());
}
