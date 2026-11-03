use basilisco::formateador;

#[test]
fn formateador_idempotente_basico() {
    let src = "funcion suma(a: entero, b: entero) -> entero {\n  retornar a + b\n}\n";
    let out = formateador::formatear(src).unwrap();
    let out2 = formateador::formatear(&out).unwrap();
    assert_eq!(out, out2);
}

#[test]
fn formateador_conserva_publico() {
    let src = "publico sea x = 5\npublico funcion f(n: entero) -> entero { retornar n }\n";
    let out = formateador::formatear(src).unwrap();
    assert!(out.contains("publico sea x = 5"));
    assert!(out.contains("publico funcion f"));
    let out2 = formateador::formatear(&out).unwrap();
    assert_eq!(out, out2);
}

#[test]
fn formateador_control_de_flujo() {
    let src = r#"funcion main() {
  sea x = 0
  si x < 0 {
    mostrar x
  } sino si x == 0 {
    mostrar 0
  } sino {
    mostrar 1
  }
  mientras x < 3 {
    x = x + 1
  }
  para cada v en [1, 2, 3] {
    mostrar v
  }
}
"#;
    let out = formateador::formatear(src).unwrap();
    let out2 = formateador::formatear(&out).unwrap();
    assert_eq!(out, out2);
}

#[test]
fn formateador_intentar_y_modulos() {
    let src = r#"intentar {
  mostrar 1
} rescatar e {
  mostrar e
}
exportar funcion h() { retornar 1 }
usar "m" { a, b }
importar "l" como l
mostrar_sin_salto 42
"#;
    let out = formateador::formatear(src).unwrap();
    let out2 = formateador::formatear(&out).unwrap();
    assert_eq!(out, out2);
}

#[test]
fn formateador_error_en_fuente_invalida() {
    let src = "funcion f() { retornar }";
    // `retornar` sin valor es válido; forzamos un error sintáctico real.
    let src2 = "sea x = \n";
    assert!(formateador::formatear(src2).is_err());
    let _ = src;
}

#[test]
fn formateador_expresiones_precedencia() {
    let src = "funcion f() { retornar (1 + 2) * 3 + 4 }\n";
    let out = formateador::formatear(src).unwrap();
    let out2 = formateador::formatear(&out).unwrap();
    assert_eq!(out, out2);
    assert!(out.contains("1 + 2"));
}
