//! Pruebas de integración del incremento 2: recorren el flujo real
//! cadena fuente → léxico → sintaxis → evaluación → valor o error.
//!
//! Semántica normativa: ADR-0005 (aprobado).

use nexo::{evaluar, ErrorEjecucion, ErrorSintactico, Lexer, Parser, Valor};

/// Ejecuta el flujo completo y devuelve el valor o el error de ejecución.
fn ejecutar(fuente: &str) -> Result<Valor, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido en test");
    let expr = Parser::nuevo(tokens)
        .parsear_expresion()
        .expect("sintaxis válida en test");
    evaluar(&expr)
}

fn error_ejecucion(fuente: &str) -> ErrorEjecucion {
    ejecutar(fuente).expect_err("se esperaba error de ejecución")
}

fn error_sintactico(fuente: &str) -> ErrorSintactico {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido en test");
    Parser::nuevo(tokens)
        .parsear_expresion()
        .expect_err("se esperaba error sintáctico")
}

// ---------------------------------------------------------------- Éxito

#[test]
fn flujo_completo_expresion_compuesta() {
    let r = ejecutar("2 + 3 * 4 - (10 / 5)").unwrap();
    assert_eq!(r, Valor::Entero(12));
}

#[test]
fn flujo_tipos_mixtos_hasta_decimal() {
    // entero → decimal implícito (ADR-0005 §1.4).
    let r = ejecutar("10 + 2.5 * 4").unwrap();
    assert_eq!(r.to_string(), "20.0"); // el display decimal muestra al menos un decimal
    assert_eq!(r, Valor::Decimal(20_000_000_000));
}

#[test]
fn flujo_real_con_exponencial() {
    let r = ejecutar("1e10 / 2.0e1").unwrap();
    assert_eq!(r, Valor::Real(5e8));
}

#[test]
fn flujo_multilinea_con_continuacion_tras_operador() {
    let r = ejecutar("10 +\n    5 * 2").unwrap();
    assert_eq!(r, Valor::Entero(20));
}

#[test]
fn flujo_modo_multiplicacion_y_modulo() {
    assert_eq!(ejecutar("7 + 6 % 4").unwrap(), Valor::Entero(9));
    assert_eq!(ejecutar("-7 % 3").unwrap(), Valor::Entero(-1));
}

// ---------------------------------------------------------------- Errores con ubicación y motivo

#[test]
fn error_desbordamiento_entero_ubicado() {
    let e = error_ejecucion("9223372036854775807 + 1");
    assert_eq!(e.mensaje, "desbordamiento entero");
    assert_eq!((e.linea, e.columna), (1, 1));
}

#[test]
fn error_division_cero_ubicado_en_operando_izquierdo() {
    // '10' ocupa la columna 5: la ubicación apunta al operando izquierdo de '/'.
    let e = error_ejecucion("5 + 10 / 0");
    assert_eq!(e.mensaje, "división entre cero");
    assert_eq!((e.linea, e.columna), (1, 5));
}

#[test]
fn error_division_cero_ubicado_en_linea_real() {
    // En la línea 2, '2.0' empieza en la columna 1 (columna se reinicia por línea).
    let e = error_ejecucion("1.0 +\n2.0 / 0.0");
    assert_eq!(e.mensaje, "división entre cero");
    assert_eq!((e.linea, e.columna), (2, 1));
}

#[test]
fn error_conversion_ilegal_decimal_real_ubicado() {
    let e = error_ejecucion("1.5 + 2.0e0");
    assert_eq!(e.mensaje, "conversión ilegal");
    assert_eq!((e.linea, e.columna), (1, 1));
}

#[test]
fn error_exceso_rango_decimal_ubicado() {
    // 19 enteros + 0.5 sumado a 0.5 alcanza 10^19 enteros → fuera de DECIMAL(28,9).
    let e = error_ejecucion("9999999999999999999.5 + 0.5");
    assert_eq!(e.mensaje, "exceso de rango decimal");
    assert_eq!((e.linea, e.columna), (1, 1));
}

#[test]
fn error_sintactico_ubicado_en_su_linea() {
    let e = error_sintactico("10\n+ 5");
    assert_eq!((e.linea, e.columna), (2, 1));
    assert!(e.mensaje.contains("inesperado"));
}

#[test]
fn error_literal_entero_fuera_de_rango_es_sintactico() {
    // i64::MAX + 1 como literal: se rechaza al clasificar, no evalúa.
    let e = error_sintactico("9223372036854775808 + 1");
    assert_eq!((e.linea, e.columna), (1, 1));
    assert!(e.mensaje.contains("literal numérico inválido"));
    assert!(e.mensaje.contains("desbordamiento entero"));
}

#[test]
fn display_del_error_incluye_ubicacion_y_motivo() {
    let e = error_ejecucion("5 + 10 / 0");
    assert_eq!(e.to_string(), "error de ejecución [1:5]: división entre cero");
}

// ---------------------------------------------------------------- Contrato "sin valor por defecto"

#[test]
fn ningun_fallo_de_ejecucion_devuelve_un_valor() {
    let fallidos = [
        "9223372036854775807 + 1",          // desbordamiento entero
        "-9223372036854775808 - 1",         // desbordamiento entero
        "9223372036854775807 * 2",          // desbordamiento entero
        "-9223372036854775808 / -1",        // desbordamiento entero
        "10 / 0",                           // división entre cero
        "10 % 0",                           // división entre cero
        "1.0 / 0.0",                        // división entre cero
        "1.0e0 / 0.0e0",                    // división entre cero
        "1.5 + 2.0e0",                      // conversión ilegal
        "4.5 % 2",                          // conversión ilegal
        "9999999999999999999.5 + 0.5",      // exceso de rango decimal
        "9999999999999999999.999999999 * 9999999999999999999.999999999", // exceso rango
    ];
    for fuente in fallidos {
        let r = ejecutar(fuente);
        assert!(r.is_err(), "esperaba error para {fuente:?}, obtuvo {r:?}");
    }
}

#[test]
fn todo_fallo_de_ejecucion_lleva_ubicacion_valida() {
    let fallidos = [
        "9223372036854775807 + 1",
        "10 / 0",
        "1.5 + 2.0e0",
        "9999999999999999999.5 + 0.5",
        "1.0 +\n2.0 / 0.0",
    ];
    for fuente in fallidos {
        let e = error_ejecucion(fuente);
        assert!(e.linea >= 1, "ubicación inválida para {fuente:?}");
        assert!(e.columna >= 1, "ubicación inválida para {fuente:?}");
        assert!(!e.mensaje.is_empty(), "motivo vacío para {fuente:?}");
    }
}
