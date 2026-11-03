//! Suite de Conformidad Semántica BASILISCO 0.5.0 (H0.5)

use basilisco::{Lexer, Parser, Valor};

fn ejecutar_expresion(fuente: &str) -> Result<Valor, String> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| format!("léxico: {e}"))?;
    let expr = Parser::nuevo(tokens)
        .parsear_expresion()
        .map_err(|e| format!("sintaxis: {e}"))?;
    basilisco::evaluar(&expr).map_err(|e| e.mensaje)
}

fn ejecutar_programa(fuente: &str) -> Result<Option<Valor>, String> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| format!("léxico: {e}"))?;
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .map_err(|e| format!("sintaxis: {e}"))?;
    let mut entorno = basilisco::Entorno::nueva();
    entorno
        .ejecutar_programa(&sentencias)
        .map_err(|e| e.mensaje)
}

#[test]
fn sem_lit_enteros() {
    assert_eq!(ejecutar_expresion("0").unwrap(), Valor::Entero(0));
    assert_eq!(ejecutar_expresion("123").unwrap(), Valor::Entero(123));
    assert_eq!(ejecutar_expresion("-42").unwrap(), Valor::Entero(-42));
}
#[test]
fn sem_lit_decimales() {
    assert_eq!(ejecutar_expresion("3.14").unwrap().to_string(), "3.14");
}
#[test]
fn sem_lit_texto() {
    assert_eq!(ejecutar_expresion(r#""hola""#).unwrap().to_string(), "hola");
    assert_eq!(ejecutar_expresion(r#""""#).unwrap().to_string(), "");
}
#[test]
fn sem_lit_booleanos() {
    assert_eq!(
        ejecutar_expresion("verdadero").unwrap(),
        Valor::Logico(true)
    );
    assert_eq!(ejecutar_expresion("falso").unwrap(), Valor::Logico(false));
}
#[test]
fn sem_lit_nulo() {
    assert_eq!(ejecutar_expresion("nulo").unwrap(), Valor::Nulo);
}

#[test]
fn sem_var_sea_declaracion_y_uso() {
    assert_eq!(
        ejecutar_programa("sea x = 10\nx").unwrap(),
        Some(Valor::Entero(10))
    );
}
#[test]
fn sem_var_asignacion_mutable() {
    assert_eq!(
        ejecutar_programa("sea x = 5\nx = x + 3\nx").unwrap(),
        Some(Valor::Entero(8))
    );
}
#[test]
fn sem_var_dato_inmutable() {
    assert!(ejecutar_programa("dato x = 5\nx = 10").is_err());
}
#[test]
fn sem_var_declaracion_duplicada_mismo_ambito() {
    assert!(ejecutar_programa("sea x = 1\nsea x = 2").is_err());
}

#[test]
fn sem_op_precedencia() {
    assert_eq!(ejecutar_expresion("2 + 3 * 4").unwrap(), Valor::Entero(14));
    assert_eq!(
        ejecutar_expresion("(2 + 3) * 4").unwrap(),
        Valor::Entero(20)
    );
}
#[test]
fn sem_op_logicos() {
    assert_eq!(
        ejecutar_expresion("verdadero y falso").unwrap(),
        Valor::Logico(false)
    );
    assert_eq!(
        ejecutar_expresion("verdadero o falso").unwrap(),
        Valor::Logico(true)
    );
    assert_eq!(ejecutar_expresion("no falso").unwrap(), Valor::Logico(true));
}
#[test]
fn sem_op_comparacion() {
    assert_eq!(ejecutar_expresion("5 == 5").unwrap(), Valor::Logico(true));
    assert_eq!(ejecutar_expresion("5 != 3").unwrap(), Valor::Logico(true));
    assert_eq!(ejecutar_expresion("3 < 5").unwrap(), Valor::Logico(true));
}

#[test]
fn sem_func_def_y_llamada() {
    assert_eq!(
        ejecutar_programa("funcion suma(a, b) {\n  retornar a + b\n}\nsuma(2, 3)").unwrap(),
        Some(Valor::Entero(5))
    );
}
#[test]
fn sem_func_argumentos_incorrectos() {
    assert!(ejecutar_programa("funcion suma(a, b) {\n  retornar a + b\n}\nsuma(1)").is_err());
}

#[test]
fn sem_si_sino() {
    assert_eq!(
        ejecutar_programa(
            "funcion f() {\n  si 3 > 2 {\n    retornar 1\n  } sino {\n    retornar 0\n  }\n}\nf()"
        )
        .unwrap(),
        Some(Valor::Entero(1))
    );
}
#[test]
fn sem_mientras() {
    assert_eq!(
        ejecutar_programa("sea i = 0\nmientras i < 3 {\n  i = i + 1\n}\ni").unwrap(),
        Some(Valor::Entero(3))
    );
}
#[test]
fn sem_para_cada() {
    assert_eq!(
        ejecutar_programa("sea s = 0\npara cada x en [1, 2, 3] {\n  s = s + x\n}\ns").unwrap(),
        Some(Valor::Entero(6))
    );
}

#[test]
fn sem_err_variable_no_definida() {
    assert!(ejecutar_programa("x").is_err());
}
#[test]
fn sem_err_division_cero() {
    assert!(ejecutar_expresion("1 / 0").is_err());
}

#[test]
fn sem_pendiente_concurrencia_async() {}
#[test]
fn sem_pendiente_ffi() {}
#[test]
fn sem_pendiente_clases_objetos() {}

#[test]
fn reporte_conformidad_semantica() {
    println!("\n=== REPORTE CONFORMIDAD SEMÁNTICA BASILISCO 0.5.0 (H0.5) ===");
    println!(
        "CONFORME: literales, variables/sea/dato, asignación, operaciones, funciones, si/sino, mientras, para-cada, errores básicos"
    );
    println!(
        "PENDIENTE: concurrencia/async, FFI, POO/clases, sistema de paquetes, VM integrada, backends WASM/nativo"
    );
    println!("AMBIGUO: modelo de memoria definitivo, equivalencia entre backends");
    println!("=== FIN REPORTE ===");
}
