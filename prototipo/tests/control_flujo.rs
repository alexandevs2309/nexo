//! Pruebas de integración del incremento 4: recorren el flujo real
//! cadena fuente → léxico → sintaxis → evaluación → valor o error,
//! con programas que combinan variables y control de flujo.
//!
//! Semántica normativa: ADR-0007 (aprobado) sobre ADR-0005 y ADR-0006.

use nexo::{CausaError, Entorno, ErrorEjecucion, ErrorSintactico, Lexer, Parser, Valor};

/// Ejecuta un programa completo y devuelve el valor de la última expresión.
fn ejecutar(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido en test");
    let ss = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida en test");
    Entorno::nueva().ejecutar_programa(&ss)
}

fn error_ejecucion(fuente: &str) -> ErrorEjecucion {
    ejecutar(fuente).expect_err("se esperaba error de ejecución")
}

fn error_sintactico(fuente: &str) -> ErrorSintactico {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido en test");
    Parser::nuevo(tokens)
        .parsear_programa()
        .expect_err("se esperaba error sintáctico")
}

fn valor(fuente: &str) -> Valor {
    ejecutar(fuente)
        .expect("ejecución sin error")
        .expect("el programa produce valor")
}

// ---------------------------------------------------------------- Condicionales

#[test]
fn programa_con_si_toma_la_rama_correcta() {
    let p = "sea x = 7\nsea r = 0\nsi x > 5 {\n  r = 1\n}\nr";
    assert_eq!(valor(p), Valor::Entero(1));

    let p = "sea x = 3\nsea r = 0\nsi x > 5 {\n  r = 1\n}\nr";
    assert_eq!(valor(p), Valor::Entero(0));
}

#[test]
fn programa_con_sino_toma_la_rama_alternativa() {
    let p = "sea x = 3\nsea r = 0\nsi x > 5 {\n  r = 1\n} sino {\n  r = 2\n}\nr";
    assert_eq!(valor(p), Valor::Entero(2));
}

#[test]
fn programa_con_cadena_sino_si() {
    let p = "sea x = 3\nsea r = 0\nsi x > 5 {\n  r = 1\n} sino si x > 2 {\n  r = 2\n} sino {\n  r = 3\n}\nr";
    assert_eq!(valor(p), Valor::Entero(2));
}

#[test]
fn un_bloque_anidado_dentro_de_un_si() {
    let p = "sea r = 0\nsi verdadero {\n  si verdadero {\n    r = 5\n  }\n}\nr";
    assert_eq!(valor(p), Valor::Entero(5));
}

// ---------------------------------------------------------------- Ciclos

#[test]
fn programa_con_mientras_acumula() {
    let p = "sea i = 0\nsea s = 0\nmientras i < 5 {\n  i = i + 1\n  s = s + i\n}\ns";
    assert_eq!(valor(p), Valor::Entero(15));
}

#[test]
fn programa_con_mientras_de_cero_iteraciones() {
    let p = "sea s = 0\nmientras falso {\n  s = 99\n}\ns";
    assert_eq!(valor(p), Valor::Entero(0));
}

#[test]
fn el_cuerpo_del_ciclo_ve_las_variables_externas() {
    let p = "sea total = 0\nsea i = 1\nmientras i <= 3 {\n  total = total + i\n  i = i + 1\n}\ntotal";
    assert_eq!(valor(p), Valor::Entero(6));
}

#[test]
fn el_ciclo_actualiza_la_misma_variable_del_ambito_externo() {
    // La asignación dentro del bloque actúa sobre la variable del ámbito exterior.
    let p = "sea n = 0\nmientras n < 3 {\n  n = n + 1\n}\nn";
    assert_eq!(valor(p), Valor::Entero(3));
}

// ---------------------------------------------------------------- Combinado

#[test]
fn programa_que_combina_condiciones_y_ciclos() {
    let p = "\
sea total = 0
sea i = 1
mientras i <= 10 {
  si i % 2 == 0 {
    total = total + i
  } sino {
    total = total + 1
  }
  i = i + 1
}
total";
    // Pares (2+4+6+8+10 = 30) + 5 impares (1 cada uno) = 35.
    assert_eq!(valor(p), Valor::Entero(35));
}

#[test]
fn programa_con_operadores_logicos_entre_comparaciones() {
    let p = "sea x = 5\nsi x > 0 y x < 10 {\n  x = 1\n}\nx";
    assert_eq!(valor(p), Valor::Entero(1));

    let p = "sea x = 50\nsi x > 0 y x < 10 {\n  x = 1\n}\nx";
    assert_eq!(valor(p), Valor::Entero(50));

    let p = "sea x = 50\nsi x < 0 o x > 10 {\n  x = 1\n}\nx";
    assert_eq!(valor(p), Valor::Entero(1));
}

// ---------------------------------------------------------------- Errores

#[test]
fn condicion_no_logica_es_error_tipeado_con_ubicacion() {
    let e = error_ejecucion("si 5 {\n  1\n}");
    assert_eq!(e.causa, CausaError::CondicionNoLogica { obtenido: "entero" });
    assert_eq!((e.linea, e.columna), (1, 4));
}

#[test]
fn error_dentro_de_un_bloque_propaga_su_tipo_y_ubicacion() {
    let e = error_ejecucion("si verdadero {\n  10 / 0\n}");
    assert_eq!(
        e.causa,
        CausaError::Aritmetica(nexo::MotivoAritmetico::DivisionPorCero)
    );
    assert_eq!((e.linea, e.columna), (2, 3));
}

#[test]
fn error_dentro_de_un_ciclo_propaga_su_tipo_y_ubicacion() {
    let e = error_ejecucion("sea i = 0\nmientras i < 3 {\n  10 / 0\n}");
    assert_eq!(
        e.causa,
        CausaError::Aritmetica(nexo::MotivoAritmetico::DivisionPorCero)
    );
    assert_eq!((e.linea, e.columna), (3, 3));
}

#[test]
fn un_error_dentro_de_una_rama_no_produce_valor_por_defecto() {
    let e = error_ejecucion("si verdadero {\n  10 / 0\n}\n42");
    assert!(matches!(e.causa, CausaError::Aritmetica(_)));
}

#[test]
fn el_cortocircuito_evita_el_error_del_operando_derecho() {
    // El `10 / 0` está en el operando derecho y no llega a evaluarse.
    let p = "si falso y (10 / 0) {\n  1\n}\n7";
    assert_eq!(valor(p), Valor::Entero(7));

    // Aquí sí se evalúa y el error aparece.
    let e = error_ejecucion("si verdadero y (10 / 0) {\n  1\n}\n7");
    assert_eq!(
        e.causa,
        CausaError::Aritmetica(nexo::MotivoAritmetico::DivisionPorCero)
    );
}

#[test]
fn la_llave_en_otra_linea_es_error_sintactico() {
    let e = error_sintactico("si verdadero\n{\n  1\n}");
    assert!(
        e.mensaje.contains("misma línea"),
        "mensaje inesperado: {}",
        e.mensaje
    );
}

#[test]
fn la_comparacion_decimal_real_es_error_tipeado() {
    let e = error_ejecucion("si 0.1 == 0.1e0 {\n  1\n}");
    assert_eq!(
        e.causa,
        CausaError::TiposIncompatibles {
            izquierdo: "decimal",
            derecho: "real"
        }
    );
}

// ---------------------------------------------------------------- Compatibilidad

#[test]
fn los_programas_del_incremento_anterior_siguen_funcionando() {
    // Expresiones, literales y aritmética sin cambios (ADR-0005).
    let p = "sea precio = 10.5\nsea total = precio * 3\ntotal = total - 1.5\ntotal";
    assert_eq!(valor(p), Valor::Decimal(30_000_000_000));
}

#[test]
fn los_nombres_de_variable_siguen_libres_si_no_son_palabras_clave() {
    // `y`/`o` están reservadas, pero `y_2`, `total` o `contador` son identificadores válidos.
    let p = "sea y_2 = 2\nsea total = 3\ntotal = total + y_2\ntotal";
    assert_eq!(valor(p), Valor::Entero(5));
}
