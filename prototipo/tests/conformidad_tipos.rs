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

// Regresión FE-14.2: literal colección con tamaño incompatible contra arreglo anotado
// debe ser rechazado estáticamente (no solo en runtime).
#[test]
fn literal_coleccion_tamano_incompatible_con_arreglo_es_error() {
    let fuente = "var x: arreglo<entero, 3> = [1, 2]";
    assert!(ejecutar(fuente).is_err());
}

#[test]
fn literal_coleccion_tamano_compatible_con_arreglo_aceptado() {
    let fuente = "var x: arreglo<entero, 2> = [1, 2]\nx[0]";
    assert_eq!(valor(fuente), Valor::Entero(1));
}

#[test]
fn literal_coleccion_tamano_incompatible_rechazado_por_semantico() {
    let src = "var x: arreglo<entero, 3> = [1, 2]";
    let tokens = basilisco::Lexer::nuevo(src).tokenizar().unwrap();
    let ast = basilisco::Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = basilisco::ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ast);
    assert!(
        !errs.is_empty(),
        "el semántico debe rechazar tamaño incompatible de colección literal"
    );
}

// Regresión FE-14.2R: unificar Coleccion<T,n> con Arreglo<T,m> de distinto tamaño debe fallar.
#[test]
fn unificar_coleccion_arreglo_tamano_distinto_falla() {
    use basilisco::semantico::{TipoParamId, TipoSem, unificar};
    use std::collections::{HashMap, HashSet};
    let param = TipoParamId {
        propietario: basilisco::simbodo::SimbodoId(0),
        nombre: "T".to_string(),
    };
    let mut objetivo = HashSet::new();
    objetivo.insert(param.clone());
    let formal = TipoSem::Coleccion(Box::new(TipoSem::Param(param)), 3);
    let observado = TipoSem::Arreglo(Box::new(TipoSem::Entero), 2);
    let mut sigma = HashMap::new();
    let r = unificar(&formal, &observado, &objetivo, &mut sigma);
    assert!(r.is_err(), "debe rechazar tamaño incompatible: {:?}", r);
}
