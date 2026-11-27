// Pruebas de integración 3D.5: consistencia de gates en pipeline y
// ausencia de fallback VM->Tree-Walk ante errores de verificación.
//
// Las pruebas que fuerzan fallo de verificación requieren la feature
// `test-hooks` (el gancho solo existe en builds debug con esa feature).

#[cfg(feature = "test-hooks")]
mod con_test_hooks {
    use std::fs;
    use std::process::Command;

    fn bin() -> std::path::PathBuf {
        // Usa el binario compilado por cargo test cuando está disponible.
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .and_then(|mut d| {
                // target/debug/deps/<test> -> target/debug/basilisco
                d.pop();
                let cand = d.join("basilisco");
                if cand.exists() { Some(cand) } else { None }
            })
            .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_BIN_EXE_basilisco")))
    }

    fn tmp_bsk(nombre: &str, contenido: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bsk-3d5-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let p = dir.join(nombre);
        fs::write(&p, contenido).unwrap();
        p
    }

    #[test]
    fn depurar_aborta_en_gate_sin_fallback() {
        let src = tmp_bsk("ok.bsk", "funcion main() { mostrar 1 }\n");
        let out = Command::new(bin())
            .args(["depurar", src.to_str().unwrap()])
            .env("BASILISCO_TEST_FORCE_GATE_FAIL", "1")
            .output()
            .expect("ejecutar binario");
        assert!(!out.status.success(), "depurar debe abortar en gate");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("verificacion:"),
            "debe reportar error de verificación: {stderr}"
        );
        assert!(
            !stderr.contains("Tree-Walk") && !stderr.contains("árbol"),
            "no debe activar fallback Tree-Walk: {stderr}"
        );
    }

    #[test]
    fn run_gate_error_no_cae_a_treewalk() {
        let src = tmp_bsk("main.bsk", "funcion main() { mostrar 1 }\n");
        let out = Command::new(bin())
            .args(["run", src.to_str().unwrap()])
            .env("BASILISCO_TEST_FORCE_GATE_FAIL", "1")
            .output()
            .expect("ejecutar binario");
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("verificacion:"), "stderr: {stderr}");
        assert!(
            !stderr.contains("Tree-Walk") && !stderr.contains("árbol"),
            "no debe hacer fallback a Tree-Walk ante verificación: {stderr}"
        );
    }
}

#[test]
fn pipeline_gates_consistentes_funciones_publicas() {
    // Las mismas funciones de gate usadas por build/run/depurar/analizar
    // rechazan IR inválido y aceptan IR válido.
    use basilisco::semantico::TipoSem;
    use basilisco::{ContextoSemantico, Lexer, Parser, cli, hir, mir};

    let src_ok = "funcion main() { mostrar 1 }";
    let tokens = Lexer::nuevo(src_ok).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let h = hir::lower_programa(&ss, &ctx).unwrap();
    assert!(cli::gate_hir(&h).is_ok());
    let m = mir::lower_programa(&h);
    assert!(cli::gate_mir(&m).is_ok());

    // MIR inválido (sin bloques) rechazado por el mismo gate usado en pipeline.
    let bad = mir::MirPrograma {
        funciones: vec![mir::MirFunction {
            nombre: "f".into(),
            params: vec![],
            retorno: TipoSem::Nulo,
            locals: vec![],
            bloques: vec![],
            bloque_entrada: mir::BlockId(0),
        }],
        globals: vec![],
        contiene_coincidir: false,
        contiene_nominales: false,

        source_map: std::collections::HashMap::new(),
    };
    let e = cli::gate_mir(&bad).unwrap_err();
    assert!(e.contains("verificacion:"), "prefijo de verificación: {e}");
}
