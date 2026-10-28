use basilisco::{Expresion, Lexer, Parser, Sentencia, TipoToken};

fn tokens(fuente: &str) -> Vec<TipoToken> {
    Lexer::nuevo(fuente)
        .tokenizar()
        .expect("léxico válido")
        .into_iter()
        .map(|t| t.tipo)
        .collect()
}

fn parsear(fuente: &str) -> Result<Vec<Sentencia>, basilisco::ErrorSintactico> {
    let toks = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
    Parser::nuevo(toks).parsear_programa()
}

// --- Lexer: palabras clave ---

#[test]
fn usar_es_palabra_clave() {
    let toks = tokens("usar");
    assert!(toks.contains(&TipoToken::Usar));
    assert!(
        !toks
            .iter()
            .any(|t| matches!(t, TipoToken::Identificador(n) if n == "usar"))
    );
}

#[test]
fn como_es_palabra_clave() {
    let toks = tokens("como");
    assert!(toks.contains(&TipoToken::Como));
}

#[test]
fn publico_es_palabra_clave() {
    let toks = tokens("publico");
    assert!(toks.contains(&TipoToken::Publico));
}

#[test]
fn punto_es_token() {
    let toks = tokens("a.b");
    assert!(toks.contains(&TipoToken::Punto));
}

// ---Parser: usar con alias ---

#[test]
fn usar_con_alias_parsea() {
    let ss = parsear(r#"usar "lib.bsk" como lib"#).expect("sintaxis válida");
    assert_eq!(ss.len(), 1);
    match &ss[0] {
        Sentencia::Usar {
            ruta,
            alias,
            nombres,
            ..
        } => {
            assert_eq!(ruta, "lib.bsk");
            assert_eq!(alias.as_deref(), Some("lib"));
            assert!(nombres.is_none());
        }
        _ => panic!("esperaba Usar"),
    }
}

// ---Parser: usar con lista ---

#[test]
fn usar_con_lista_parsea() {
    let ss = parsear(r#"usar "lib.bsk" { sumar, PI }"#).expect("sintaxis válida");
    assert_eq!(ss.len(), 1);
    match &ss[0] {
        Sentencia::Usar {
            ruta,
            alias,
            nombres,
            ..
        } => {
            assert_eq!(ruta, "lib.bsk");
            assert!(alias.is_none());
            assert_eq!(
                nombres.as_ref().unwrap(),
                &vec!["sumar".to_string(), "PI".to_string()]
            );
        }
        _ => panic!("esperaba Usar"),
    }
}

// ---Parser: usar sin alias ni lista ---

#[test]
fn usar_sin_forma_es_error() {
    let err = parsear(r#"usar "lib.bsk""#).expect_err("debería fallar");
    assert!(
        err.mensaje.contains("se esperaba")
            || err.mensaje.contains("como")
            || err.mensaje.contains("{")
    );
}

// ---Parser: usar con lista vacía ---

#[test]
fn usar_lista_vacia_es_error() {
    let err = parsear(r#"usar "lib.bsk" { }"#).expect_err("debería fallar");
    assert!(err.mensaje.contains("vacía") || err.mensaje.contains("nombre"));
}

// ---Parser: usar sin cadena ---

#[test]
fn usar_sin_cadena_es_error() {
    let err = parsear("usar lib").expect_err("debería fallar");
    assert!(err.mensaje.contains("cadena") || err.mensaje.contains("se esperaba"));
}

// ---Parser: publico en declaración ---

#[test]
fn publico_sea_parsea() {
    let ss = parsear("publico sea x = 1").expect("sintaxis válida");
    assert_eq!(ss.len(), 1);
    match &ss[0] {
        Sentencia::Declaracion {
            nombre, publico, ..
        } => {
            assert_eq!(nombre, "x");
            assert!(*publico);
        }
        _ => panic!("esperaba Declaracion"),
    }
}

#[test]
fn publico_var_parsea() {
    let ss = parsear("publico var x: entero = 1").expect("sintaxis válida");
    match &ss[0] {
        Sentencia::Declaracion { publico, .. } => assert!(*publico),
        _ => panic!("esperaba Declaracion"),
    }
}

#[test]
fn publico_funcion_parsea() {
    let ss = parsear("publico funcion f() { retornar 1 }").expect("sintaxis válida");
    match &ss[0] {
        Sentencia::Funcion {
            nombre, publico, ..
        } => {
            assert_eq!(nombre, "f");
            assert!(*publico);
        }
        _ => panic!("esperaba Funcion"),
    }
}

#[test]
fn sin_publico_es_privado() {
    let ss = parsear("sea x = 1").expect("sintaxis válida");
    match &ss[0] {
        Sentencia::Declaracion { publico, .. } => assert!(!*publico),
        _ => panic!("esperaba Declaracion"),
    }
}

// ---Parser: publico inválido ---

#[test]
fn publico_en_bloque_es_error() {
    let err = parsear("si verdadero {\n  publico sea x = 1\n}").expect_err("debería fallar");
    assert!(err.mensaje.contains("nivel superior") || err.mensaje.contains("publico"));
}

#[test]
fn publico_sin_declaracion_es_error() {
    let err = parsear("publico mostrar 1").expect_err("debería fallar");
    assert!(err.mensaje.contains("publico") || err.mensaje.contains("preceder"));
}

// ---Parser: acceso a miembro ---

#[test]
fn acceso_miembro_parsea() {
    let ss = parsear("x.foo").expect("sintaxis válida");
    assert_eq!(ss.len(), 1);
    match &ss[0] {
        Sentencia::Expresion(Expresion::Acceso { miembro, .. }) => {
            assert_eq!(miembro, "foo");
        }
        _ => panic!("esperaba Expresion(Acceso)"),
    }
}

#[test]
fn acceso_miembro_encadenado() {
    let ss = parsear("a.b.c").expect("sintaxis válida");
    match &ss[0] {
        Sentencia::Expresion(Expresion::Acceso {
            miembro, objeto, ..
        }) => {
            assert_eq!(miembro, "c");
            match objeto.as_ref() {
                Expresion::Acceso { miembro, .. } => assert_eq!(miembro, "b"),
                _ => panic!("esperaba Acceso anidado"),
            }
        }
        _ => panic!("esperaba Expresion(Acceso)"),
    }
}

#[test]
fn acceso_sin_nombre_es_error() {
    // `si` es palabra clave, no identificador: falla en el parser tras '.'.
    let err = parsear("x.si").expect_err("debería fallar");
    assert!(err.mensaje.contains("nombre") || err.mensaje.contains("se esperaba"));
}

// ---Compatibilidad ---

#[test]
fn programa_existente_sigue_parseando() {
    let fuente = "sea x = 10\nvar z: entero = 20\nfuncion f(a) { retornar a + 1 }\nmostrar f(x)\n";
    let ss = parsear(fuente).expect("programa existente debe parsear");
    assert!(ss.len() >= 3);
}

#[test]
fn ubicacion_error_usar() {
    let err = parsear("\n\nusar 42").expect_err("debería fallar");
    assert_eq!((err.linea, err.columna), (3, 6));
}
