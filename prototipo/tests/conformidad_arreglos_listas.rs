// Tests de conformidad para arreglos y listas (Fase C, Parte 2)
// Incluye cobertura de asignación por índice (B-001 arreglos, B-002 listas).

use basilisco::{Lexer, Parser, evaluador::Entorno};

fn run_program(fuente: &str) -> Result<Option<basilisco::Valor>, String> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| e.to_string())?;
    eprintln!("Tokens: {:?}", tokens);
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .map_err(|e| e.to_string())?;
    eprintln!("Sentencias parseadas: {:?}", sentencias);
    Entorno::nueva()
        .ejecutar_programa(&sentencias)
        .map_err(|e| e.to_string())
}

fn run_program_err(fuente: &str) -> String {
    run_program(fuente).unwrap_err()
}

#[test]
fn arreglo_literal_tipado() {
    let r = run_program("var a: arreglo<entero, 3> = [1, 2, 3]\na").unwrap();
    match r {
        Some(basilisco::Valor::Arreglo(v)) => {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], basilisco::Valor::Entero(1));
            assert_eq!(v[1], basilisco::Valor::Entero(2));
            assert_eq!(v[2], basilisco::Valor::Entero(3));
        }
        _ => panic!("esperaba Arreglo"),
    }
}

#[test]
fn arreglo_tamano_cero_y_tamano_inicial_incorrecto_se_rechazan() {
    let err = run_program_err("var vacio: arreglo<entero, 0> = []");
    assert!(err.contains("mayor que 0"), "error inesperado: {err}");

    let err = run_program_err("var corto: arreglo<entero, 2> = [1]");
    assert!(err.contains("incompatible"), "error inesperado: {err}");
}

#[test]
fn lista_vacia_tipada_es_valida() {
    let r = run_program("var vacia: lista<entero> = []\nvacia").unwrap();
    assert!(matches!(r, Some(basilisco::Valor::Lista(v)) if v.is_empty()));
}

#[test]
fn colecciones_rechazan_elementos_de_tipo_incompatible_al_declararse() {
    let err = run_program_err("var numeros: arreglo<entero, 2> = [1, verdadero]");
    assert!(err.contains("incompatible"), "error inesperado: {err}");

    let err = run_program_err("var numeros: lista<entero> = [1, verdadero]");
    assert!(err.contains("incompatible"), "error inesperado: {err}");
}

#[test]
fn asignacion_por_indice_requiere_indice_entero() {
    let err = run_program_err("var a: arreglo<entero, 1> = [1]\na[verdadero] = 2");
    assert!(err.contains("entero"), "error inesperado: {err}");

    let err = run_program_err("var l: lista<entero> = [1]\nl[verdadero] = 2");
    assert!(err.contains("entero"), "error inesperado: {err}");
}

// ============================================================= Arreglos (B-001)

/// Asignación válida: el valor actualizado debe leerse correctamente.
#[test]
fn arreglo_asignacion_indice_valida_y_lectura() {
    let r = run_program("var a: arreglo<entero, 3> = [1, 2, 3]\na[1] = 99\na[1]").unwrap();
    assert_eq!(r, Some(basilisco::Valor::Entero(99)));
}

/// Asignación en el primer y último índice del arreglo.
#[test]
fn arreglo_asignacion_primer_y_ultimo_indice() {
    let r = run_program("var a: arreglo<entero, 3> = [10, 20, 30]\na[0] = 1\na[2] = 3\na").unwrap();
    match r {
        Some(basilisco::Valor::Arreglo(v)) => {
            assert_eq!(v[0], basilisco::Valor::Entero(1));
            assert_eq!(v[1], basilisco::Valor::Entero(20)); // sin cambio
            assert_eq!(v[2], basilisco::Valor::Entero(3));
        }
        _ => panic!("esperaba Arreglo"),
    }
}

/// Rechazo: asignación por índice en variable inmutable (`dato`).
#[test]
fn arreglo_asignacion_inmutable_rechazada() {
    let err = run_program_err("dato a: arreglo<entero, 2> = [1, 2]\na[0] = 99");
    assert!(
        err.contains("inmutable") || err.contains("AsignacionIncompatible"),
        "error inesperado: {err}"
    );
}

/// Rechazo: índice fuera de rango.
#[test]
fn arreglo_asignacion_indice_fuera_de_rango() {
    let err = run_program_err("var a: arreglo<entero, 2> = [1, 2]\na[5] = 99");
    assert!(
        err.contains("fuera de rango") || err.contains("AsignacionIncompatible"),
        "error inesperado: {err}"
    );
}

#[test]
fn arreglo_lectura_rechaza_indices_fuera_de_rango_y_negativos() {
    for indice in ["2", "negativo"] {
        let prefijo = if indice == "negativo" {
            "var a: arreglo<entero, 2> = [1, 2]\nvar negativo: entero = 0 - 1\n"
        } else {
            "var a: arreglo<entero, 2> = [1, 2]\n"
        };
        let err = run_program_err(&format!("{prefijo}a[{indice}]"));
        assert!(err.contains("fuera de rango"), "error inesperado: {err}");
    }
}

#[test]
fn indice_de_arreglo_debe_ser_entero() {
    let err = run_program_err("var a: arreglo<entero, 1> = [1]\na[verdadero]");
    assert!(err.contains("entero"), "error inesperado: {err}");
}

/// Rechazo: índice negativo.
#[test]
fn arreglo_asignacion_indice_negativo() {
    // El parser trata el '-' como resta; usamos una variable para pasar un entero negativo.
    let err = run_program_err(
        "var a: arreglo<entero, 3> = [1, 2, 3]\nvar idx: entero = 0 - 1\na[idx] = 9",
    );
    assert!(
        err.contains("negativo") || err.contains("AsignacionIncompatible"),
        "error inesperado: {err}"
    );
}

/// Rechazo: tipo de valor incompatible con el elemento del arreglo.
#[test]
fn arreglo_asignacion_tipo_incompatible() {
    let err = run_program_err("var a: arreglo<entero, 2> = [1, 2]\na[0] = verdadero");
    assert!(
        err.contains("incompatible") || err.contains("AsignacionIncompatible"),
        "error inesperado: {err}"
    );
}

// ============================================================= Listas (B-002)

/// Asignación válida en lista mutable; el valor actualizado debe leerse correctamente.
#[test]
fn lista_asignacion_indice_valida_y_lectura() {
    let r = run_program("var l: lista<entero> = [10, 20, 30]\nl[2] = 99\nl[2]").unwrap();
    assert_eq!(r, Some(basilisco::Valor::Entero(99)));
}

/// Asignación en el primer y último índice de la lista.
#[test]
fn lista_asignacion_primer_y_ultimo_indice() {
    let r = run_program("var l: lista<entero> = [5, 6, 7]\nl[0] = 1\nl[2] = 3\nl").unwrap();
    match r {
        Some(basilisco::Valor::Lista(v)) => {
            assert_eq!(v[0], basilisco::Valor::Entero(1));
            assert_eq!(v[1], basilisco::Valor::Entero(6)); // sin cambio
            assert_eq!(v[2], basilisco::Valor::Entero(3));
        }
        _ => panic!("esperaba Lista"),
    }
}

/// Rechazo: asignación por índice en lista inmutable (`dato`).
#[test]
fn lista_asignacion_inmutable_rechazada() {
    let err = run_program_err("dato l: lista<entero> = [1, 2, 3]\nl[0] = 99");
    assert!(
        err.contains("inmutable") || err.contains("AsignacionIncompatible"),
        "error inesperado: {err}"
    );
}

/// Rechazo: índice fuera de rango en lista.
#[test]
fn lista_asignacion_indice_fuera_de_rango() {
    let err = run_program_err("var l: lista<entero> = [1, 2]\nl[10] = 99");
    assert!(
        err.contains("fuera de rango") || err.contains("AsignacionIncompatible"),
        "error inesperado: {err}"
    );
}

#[test]
fn lista_lectura_rechaza_indices_fuera_de_rango_y_negativos() {
    for indice in ["2", "negativo"] {
        let prefijo = if indice == "negativo" {
            "var l: lista<entero> = [1, 2]\nvar negativo: entero = 0 - 1\n"
        } else {
            "var l: lista<entero> = [1, 2]\n"
        };
        let err = run_program_err(&format!("{prefijo}l[{indice}]"));
        assert!(err.contains("fuera de rango"), "error inesperado: {err}");
    }
}

#[test]
fn indice_de_lista_debe_ser_entero() {
    let err = run_program_err("var l: lista<entero> = [1]\nl[verdadero]");
    assert!(err.contains("entero"), "error inesperado: {err}");
}

/// Rechazo: índice negativo en lista.
#[test]
fn lista_asignacion_indice_negativo() {
    let err =
        run_program_err("var l: lista<entero> = [1, 2, 3]\nvar idx: entero = 0 - 1\nl[idx] = 9");
    assert!(
        err.contains("negativo") || err.contains("AsignacionIncompatible"),
        "error inesperado: {err}"
    );
}

/// Rechazo: tipo de valor incompatible con el elemento de la lista.
#[test]
fn lista_asignacion_tipo_incompatible() {
    let err = run_program_err("var l: lista<entero> = [1, 2]\nl[0] = verdadero");
    assert!(
        err.contains("incompatible") || err.contains("AsignacionIncompatible"),
        "error inesperado: {err}"
    );
}
