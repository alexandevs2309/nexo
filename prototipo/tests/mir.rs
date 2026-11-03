use basilisco::{ContextoSemantico, Lexer, Parser, hir, mir};

fn parsear_y_bajar(src: &str) -> mir::MirPrograma {
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    assert!(hir::VerificadorHir::verificar(&hir));
    let mir = mir::lower_programa(&hir);
    assert!(mir::VerificadorMir::verificar(&mir));
    mir
}

#[test]
fn mir_programa_simple() {
    let src = "funcion suma(a: entero, b: entero) -> entero { retornar a + b }";
    let mir = parsear_y_bajar(src);
    assert!(!mir.funciones.is_empty());
    let f = &mir.funciones[0];
    assert!(!f.bloques.is_empty());
    // Debe tener al menos un bloque con terminator Return.
    let has_return = f
        .bloques
        .iter()
        .any(|b| matches!(b.terminator, mir::MirTerminator::Return(_)));
    assert!(has_return);
}

#[test]
fn mir_si_genera_bloques() {
    let src =
        "funcion abs(x: entero) -> entero { si x < 0 { retornar 0 - x } sino { retornar x } }";
    let mir = parsear_y_bajar(src);
    let f = &mir.funciones[0];
    // if debe generar >= 3 bloques (cond/then/else/merge).
    assert!(f.bloques.len() >= 3);
}

#[test]
fn mir_mientras_genera_loop() {
    let src = "funcion contar(n: entero) -> entero {
  sea i = 0
  mientras i < n {
    i = i + 1
  }
  retornar i
}";
    let mir = parsear_y_bajar(src);
    let f = &mir.funciones[0];
    assert!(f.bloques.len() >= 3);
}
