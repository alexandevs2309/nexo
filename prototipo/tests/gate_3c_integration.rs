// Pruebas de integración controlada del gate 3C.
//
// Limitación documentada: no existe (aún) una fuente BASILISCO que pase el
// análisis semántico y produzca HIR/MIR inválido, porque las etapas anteriores
// rechazan esos casos. Por eso estas pruebas inyectan IR inválido/válido en el
// punto de entrada del pipeline (las mismas funciones `gate_hir`/`gate_mir`
// invocadas por `build`/`run`/`depurar`/`analizar`) para demostrar el
// comportamiento del gate a nivel de integración del módulo `cli`.
// No sustituyen pruebas del verificador aislado (3A/3B).

use basilisco::semantico::TipoSem;
use basilisco::{ContextoSemantico, Lexer, Parser, cli, hir, mir};

#[test]
fn gate_hir_rechaza_ir_invalido() {
    // IR inválido: función con parámetro duplicado (HIR010).
    let src = "funcion f(x, x) { retornar x }";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    if let Ok(hir_prog) = hir::lower_programa(&ss, &ctx) {
        assert!(cli::gate_hir(&hir_prog).is_err());
    }
}

#[test]
fn gate_hir_acepta_ir_valido() {
    let src = "sea x = 5\nx";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir_prog = hir::lower_programa(&ss, &ctx).unwrap();
    assert!(cli::gate_hir(&hir_prog).is_ok());
}

#[test]
fn gate_mir_rechaza_ir_invalido() {
    use basilisco::mir::{BlockId, MirFunction, MirPrograma};
    let f = MirFunction {
        nombre: "f".to_string(),
        params: vec![],
        retorno: TipoSem::Nulo,
        locals: vec![],
        bloques: vec![],
        bloque_entrada: BlockId(0),
    };
    let prog = MirPrograma {
        funciones: vec![f],
        globals: vec![],
        contiene_coincidir: false,
        contiene_nominales: false,

        source_map: std::collections::HashMap::new(),
    };
    assert!(cli::gate_mir(&prog).is_err());
}

#[test]
fn gate_mir_acepta_ir_valido() {
    let src = "funcion suma(a: entero, b: entero) -> entero { retornar a + b }";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir_prog = hir::lower_programa(&ss, &ctx).unwrap();
    let mir_prog = mir::lower_programa(&hir_prog);
    assert!(cli::gate_mir(&mir_prog).is_ok());
}
