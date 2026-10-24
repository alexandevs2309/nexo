//! Integración y conformidad del incremento 3: variables y asignaciones (ADR-0006).
//!
//! Recorre el flujo real cadena fuente → léxico → sintaxis → ejecución con entorno.

use basilisco::{
    CausaError, Entorno, ErrorEjecucion, Lexer, MotivoAritmetico, Parser, Sentencia, Valor,
};

fn ejecutar(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .expect("léxico válido en test");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida en test");
    Entorno::nueva().ejecutar_programa(&sentencias)
}

fn error(fuente: &str) -> ErrorEjecucion {
    ejecutar(fuente).expect_err("se esperaba error de ejecución")
}

fn valor(fuente: &str) -> Valor {
    ejecutar(fuente)
        .unwrap_or_else(|e| panic!("{fuente:?} → error inesperado: {e}"))
        .expect("el programa debe producir una expresión final")
}

// ================================================================ Aceptación

#[test]
fn programa_declara_usa_modifica_y_reutiliza() {
    // Criterio de aceptación del incremento 3.
    let fuente = "\
sea precio = 10.5
sea total = precio * 3
total = total - 1.5
total";
    let r = valor(fuente);
    assert_eq!(r, Valor::Decimal(30_000_000_000));
    assert_eq!(r.to_string(), "30.0");
}

#[test]
fn programa_con_enteros_y_variable_intermedia() {
    let r = valor("sea base = 20\nsea doble = base * 2\ndoble = doble - 4\ndoble + 1");
    assert_eq!(r, Valor::Entero(37));
}

#[test]
fn programa_multilinea_expresion_final_en_ultima_linea() {
    let r = valor("sea x = 1\nsea z = 2\nx + z");
    assert_eq!(r, Valor::Entero(3));
}

#[test]
fn el_tipo_de_la_expresion_final_es_el_del_valor() {
    // entero → decimal implícito en la multiplicación mixta, sin tocar la variable.
    let r = valor("sea x = 5\nx * 0.5");
    assert_eq!(r, Valor::Decimal(2_500_000_000));
    // La variable conserva su tipo entero.
    let r = valor("sea x = 5\nx = x * 2\nx");
    assert_eq!(r, Valor::Entero(10));
}

// ================================================================ ADR-0006 §1.4 conversiones

#[test]
fn asignacion_admite_solo_conversiones_implicitas_aprobadas() {
    // entero → decimal: permitida (ADR-0005 §1.4).
    assert_eq!(
        valor("sea x = 1.5\nx = 2\nx"),
        Valor::Decimal(2_000_000_000)
    );
    // entero → real: permitida.
    assert_eq!(valor("sea x = 1.5e0\nx = 2\nx"), Valor::Real(2.0));
    // decimal ← real: prohibida (exige conversión explícita, aún sin sintaxis).
    let e = error("sea x = 1.5\nx = 2.0e0");
    assert_eq!(
        e.causa,
        CausaError::AsignacionIncompatible {
            esperado: "decimal".to_string(),
            obtenido: "real".to_string()
        }
    );
    // real ← decimal: prohibida.
    let e = error("sea x = 1.5e0\nx = 2.5");
    assert_eq!(
        e.causa,
        CausaError::AsignacionIncompatible {
            esperado: "real".to_string(),
            obtenido: "decimal".to_string()
        }
    );
    // entero ← decimal: prohibida (pérdida).
    let e = error("sea x = 5\nx = 2.5");
    assert_eq!(
        e.causa,
        CausaError::AsignacionIncompatible {
            esperado: "entero".to_string(),
            obtenido: "decimal".to_string()
        }
    );
}

// ================================================================ ADR-0006 §1.5 errores

#[test]
fn error_variable_no_definida_tipeado_y_ubicado() {
    let e = error("sea total = 1\ntotal + descuento");
    assert_eq!(e.causa, CausaError::VariableNoDefinida("descuento".into()));
    assert_eq!(e.mensaje, "variable no definida: descuento");
    assert_eq!((e.linea, e.columna), (2, 9));
    assert_eq!(
        e.to_string(),
        "error de ejecución [2:9]: variable no definida: descuento"
    );
}

#[test]
fn asignacion_a_desconocido_no_declara() {
    // Restricción del incremento: nunca declaración implícita.
    let e = error("inexistente = 10");
    assert_eq!(
        e.causa,
        CausaError::VariableNoDefinida("inexistente".into())
    );
    // Después del intento, el nombre sigue sin existir.
    let e = error("inexistente = 10\ninexistente");
    assert_eq!(
        e.causa,
        CausaError::VariableNoDefinida("inexistente".into())
    );
}

#[test]
fn error_declaracion_duplicada_tipeada_y_ubicada() {
    let e = error("sea x = 1\nsea x = 2\nx");
    assert_eq!(e.causa, CausaError::DeclaracionDuplicada("x".into()));
    assert_eq!(e.mensaje, "declaración duplicada: x");
    assert_eq!((e.linea, e.columna), (2, 5));
}

#[test]
fn error_asignacion_incompatible_tipeado_y_ubicado() {
    let e = error("sea contador = 0\ncontador = 0.5");
    assert_eq!(
        e.causa,
        CausaError::AsignacionIncompatible {
            esperado: "entero".to_string(),
            obtenido: "decimal".to_string()
        }
    );
    assert_eq!(
        e.mensaje,
        "asignación incompatible: se esperaba entero y se obtuvo decimal"
    );
    assert_eq!((e.linea, e.columna), (2, 1));
}

// ================================================================ Aritmética en programas

#[test]
fn errores_aritmeticos_con_causa_y_ubicacion_en_programas() {
    let e = error("sea x = 10\nx / 0");
    assert_eq!(
        e.causa,
        CausaError::Aritmetica(MotivoAritmetico::DivisionPorCero)
    );
    assert_eq!((e.linea, e.columna), (2, 1));

    let e = error("sea grande = 9223372036854775807\ngrande + 1");
    assert_eq!(
        e.causa,
        CausaError::Aritmetica(MotivoAritmetico::DesbordamientoEntero)
    );
    assert_eq!((e.linea, e.columna), (2, 1));
}

#[test]
fn semantica_numerica_aprobada_no_se_modifica_con_variables() {
    // Los valores pasados por variables se comportan igual que los literales.
    assert_eq!(
        valor("sea x = 0.1\nsea z = 0.2\nx + z"),
        Valor::Decimal(300_000_000)
    );
    assert_eq!(valor("sea a = -7\na % 3"), Valor::Entero(-1));
    assert_eq!(valor("sea a = 7\nsea b = -3\na % b"), Valor::Entero(1));
    let e = error("sea cero = 0\n10 / cero");
    assert_eq!(
        e.causa,
        CausaError::Aritmetica(MotivoAritmetico::DivisionPorCero)
    );
}

// ================================================================ Entorno y ámbitos

#[test]
fn programa_sin_expresion_final_no_produce_valor() {
    assert_eq!(ejecutar("sea x = 5\nx = x + 1").unwrap(), None);
}

#[test]
fn el_entorno_conserva_las_variables_entre_sentencias() {
    let tokens = Lexer::nuevo("sea x = 3\nsea z = 4").tokenizar().unwrap();
    let ss: Vec<Sentencia> = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut env = Entorno::nueva();
    env.ejecutar_programa(&ss).unwrap();
    assert_eq!(env.variable("x"), Some(Valor::Entero(3)));
    assert_eq!(env.variable("z"), Some(Valor::Entero(4)));
    assert_eq!(env.variable("w"), None);
}

#[test]
fn declaracion_duplicada_solo_falla_en_el_mismo_ambito() {
    // Con un solo ámbito (incremento 3) toda duplicidad falla; el chequeo es por ámbito.
    let e = error("sea x = 1\nsea x = 2");
    assert!(matches!(e.causa, CausaError::DeclaracionDuplicada(_)));
}
