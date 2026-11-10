use basilisco::{ContextoSemantico, Lexer, Parser, hir};

#[test]
fn hir_lower_literal() {
    let src = "42";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    assert!(hir::VerificadorHir::verificar(&hir));
}

#[test]
fn hir_lower_declaracion() {
    let src = "sea x = 5\nx";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    assert!(hir::VerificadorHir::verificar(&hir));
}

#[test]
fn hir_acepta_top_level_en_main_sintetico() {
    let src = "mientras verdadero { mostrar(1) }";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let res = hir::lower_programa(&ss, &ctx);
    assert!(res.is_ok());
}

#[test]
fn hir_lower_intentar_rescatar() {
    let src = "intentar { mostrar(1) } rescatar e { mostrar(e) }";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    match &hir.items[0] {
        hir::HirItem::IntentarRescatar { variable_error, .. } => {
            assert!(variable_error.is_some());
        }
        _ => panic!("esperaba IntentarRescatar"),
    }
}

#[test]
fn hir_lower_importar() {
    let src = "importar \"m.bas\" como m";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    match &hir.items[0] {
        hir::HirItem::Importar { ruta, alias } => {
            assert_eq!(ruta, "m.bas");
            assert_eq!(alias.as_deref(), Some("m"));
        }
        _ => panic!("esperaba Importar"),
    }
}

#[test]
fn hir_lower_exportar() {
    let src = "exportar sea x = 5";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    assert!(matches!(hir.items[0], hir::HirItem::Exportar { .. }));
}

#[test]
fn hir_lower_usar() {
    let src = "usar \"m.bas\" { a, b }";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    match &hir.items[0] {
        hir::HirItem::Usar { ruta, nombres, .. } => {
            assert_eq!(ruta, "m.bas");
            assert_eq!(
                nombres.as_ref().unwrap(),
                &vec!["a".to_string(), "b".to_string()]
            );
        }
        _ => panic!("esperaba Usar"),
    }
}

#[test]
fn hir_verificador_rechaza_tipos_literal_mal() {
    let src = "funcion main() -> logico { retornar (1 + 2) > \"x\" }";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let res = hir::lower_programa(&ss, &ctx);
    if let Ok(hir_prog) = res {
        assert!(!hir::VerificadorHir::verificar(&hir_prog));
    }
}

#[test]
fn hir_verificador_rechaza_params_duplicados() {
    let src = "funcion f(x, x) { retornar x }\nf(1, 2)";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    // lower_programa debe fallar o verificar devolver false.
    if let Ok(hir_prog) = hir::lower_programa(&ss, &ctx) {
        assert!(!hir::VerificadorHir::verificar(&hir_prog));
    }
}

// --- DT-03: robustecimiento del lowering HIR ---

#[test]
fn hir_lower_asignacion_no_declarada_falla() {
    let src = "x = 5";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let res = hir::lower_programa(&ss, &ctx);
    assert!(
        res.is_err(),
        "asignación a variable no declarada debe fallar en lowering"
    );
    let errs = res.unwrap_err();
    assert!(
        errs.iter().any(|e| e.contains("no declarada en lowering")),
        "debe reportar 'no declarada en lowering': {:?}",
        errs
    );
}

#[test]
fn hir_lower_programa_con_varios_errores_no_panica() {
    // Varios errores independientes: no debe panic y debe reportar lista.
    let src = "x = 5\nw = 10\nz = x + w";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let res = hir::lower_programa(&ss, &ctx);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    // Al menos un error por cada asignación no declarada.
    assert!(!errs.is_empty(), "debe haber al menos un error: {:?}", errs);
}

#[test]
fn hir_lower_estructura_incompleta_no_panica() {
    // Función cuyo cuerpo referencia una variable no declarada.
    let src = "funcion f() { x = 5 }\nf()";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    // No debe panic; el resultado puede ser Ok o Err, pero no panic.
    let _ = hir::lower_programa(&ss, &ctx);
}

#[test]
fn hir_lower_valido_sigue_ok() {
    let src = "sea x = 5\nx = x + 1\nx";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    assert!(hir::VerificadorHir::verificar(&hir));
}
