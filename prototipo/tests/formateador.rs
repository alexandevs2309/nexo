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

#[test]
fn formateador_preserva_comentarios_hash() {
    let src = "# comentario inicial\nfuncion main() {\n  # dentro\n  mostrar 1\n}\n";
    let out = formateador::formatear(src).unwrap();
    assert!(out.contains("# comentario inicial"), "out={out}");
    assert!(out.contains("# dentro"), "out={out}");
    let out2 = formateador::formatear(&out).unwrap();
    assert_eq!(out, out2, "idempotencia con comentarios");
}

#[test]
fn formateador_idempotente_y_parseable() {
    let src = "funcion suma(a: entero, b: entero) -> entero {\n  retornar a + b\n}\nfuncion main() {\n  mostrar suma(1, 2)\n}\n";
    let out = formateador::formatear(src).unwrap();
    let out2 = formateador::formatear(&out).unwrap();
    assert_eq!(out, out2);
    // parseable
    let tokens = basilisco::Lexer::nuevo(&out).tokenizar().unwrap();
    basilisco::Parser::nuevo(tokens).parsear_programa().unwrap();
}

#[test]
fn formateador_comentario_final_de_bloque_no_se_mueve() {
    let src = "funcion main() {\n  mostrar 1\n  # fin bloque\n}\nmostrar 2\n";
    let out = formateador::formatear(src).unwrap();
    assert!(out.contains("  # fin bloque\n}\nmostrar 2"), "out={out}");
    let out2 = formateador::formatear(&out).unwrap();
    assert_eq!(out, out2);
}

#[test]
fn formateador_comentarios_contractuales() {
    let src = "# inicio\n# segundo\nfuncion main() {\n  # antes\n  sea x = 1\n  # entre\n  mostrar x\n  # fin\n}\n# final\n";
    let out = formateador::formatear(src).unwrap();
    for c in [
        "# inicio",
        "# segundo",
        "# antes",
        "# entre",
        "# fin",
        "# final",
    ] {
        assert!(out.contains(c), "falta {c} en:\n{out}");
    }
    // no duplicados
    assert_eq!(out.matches("# inicio").count(), 1);
    assert_eq!(out.matches("# final").count(), 1);
    let out2 = formateador::formatear(&out).unwrap();
    assert_eq!(out, out2, "idempotente");
}

#[test]
fn formateador_comentarios_anidados_y_consecutivos() {
    let src = "funcion main() {\n  si verdadero {\n    # a\n    # b\n    mostrar 1\n  }\n}\n";
    let out = formateador::formatear(src).unwrap();
    assert!(out.contains("# a"), "out={out}");
    assert!(out.contains("# b"), "out={out}");
    let a = out.find("# a").unwrap();
    let b = out.find("# b").unwrap();
    assert!(a < b, "orden consecutivo: {out}");
    assert_eq!(out, formateador::formatear(&out).unwrap());
}

fn normalizar_ast_dbg(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b',' {
            let rest = &s[i..];
            if rest.starts_with(", linea: ") {
                // skip ", linea: <digits>, columna: <digits>"
                let mut j = i + ", linea: ".len();
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if s[j..].starts_with(", columna: ") {
                    j += ", columna: ".len();
                    while j < bytes.len() && bytes[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                    continue;
                }
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[test]
fn formateador_preserva_semantica_corpus() {
    use basilisco::{Lexer, Parser};
    let casos = [
        "funcion f(a: entero, b: entero) -> entero {\n  retornar a + b * 2\n}\nfuncion main() { mostrar f(1, 2) }\n",
        "funcion main() {\n  sea x = 0\n  si x < 1 {\n    mostrar 1\n  } sino {\n    mostrar 2\n  }\n}\n",
        "funcion main() {\n  sea i = 0\n  mientras i < 3 {\n    i = i + 1\n  }\n  mostrar i\n}\n",
        "funcion main() {\n  para cada v en [1, 2, 3] {\n    mostrar v\n  }\n}\n",
    ];
    for src in casos {
        let out = formateador::formatear(src).unwrap();
        let ast1 = Parser::nuevo(Lexer::nuevo(src).tokenizar().unwrap())
            .parsear_programa()
            .unwrap();
        let ast2 = Parser::nuevo(Lexer::nuevo(&out).tokenizar().unwrap())
            .parsear_programa()
            .unwrap();
        let n1 = normalizar_ast_dbg(&format!("{ast1:?}"));
        let n2 = normalizar_ast_dbg(&format!("{ast2:?}"));
        assert_eq!(n1, n2, "semántica/AST alterada\nsrc:\n{src}\nout:\n{out}");
        let out2 = formateador::formatear(&out).unwrap();
        assert_eq!(out, out2, "idempotencia\n{src}");
    }
}

#[test]
fn formateador_idempotente_complejo() {
    let src = "# c1\nfuncion calc(x: entero) -> entero {\n  # c2\n  si x > 0 {\n    retornar x * (1 + 2)\n  } sino {\n    retornar 0\n  }\n}\n";
    let out = formateador::formatear(src).unwrap();
    let out2 = formateador::formatear(&out).unwrap();
    let out3 = formateador::formatear(&out2).unwrap();
    assert_eq!(out, out2);
    assert_eq!(out2, out3);
}
