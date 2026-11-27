//! S8.2 - Enums/ADTs nominales v1 (Fase 08). Positivas, negativas, TW↔VM y capacidad backend.

use basilisco::{
    Comparacion, ContextoSemantico, EfectosObservables, Entorno, Lexer, Parser,
    ResultadoNormalizado, comparar_semantica, hir, mir, normalizar_error_tw, normalizar_error_vm,
    normalizar_valor, vm,
};
use std::collections::BTreeMap;

fn parsear(src: &str) -> Vec<basilisco::Sentencia> {
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    Parser::nuevo(tokens).parsear_programa().unwrap()
}

fn tw_resultado(src: &str) -> ResultadoNormalizado {
    let ss = parsear(&format!("{}\nmain()", src));
    let mut ctx = ContextoSemantico::nuevo();
    let errores_sem = ctx.analizar_programa(&ss);
    if let Some(e) = errores_sem.first() {
        return ResultadoNormalizado::Err(normalizar_error_tw(e));
    }
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    if let Some(e) = errs_g4.first() {
        return ResultadoNormalizado::Err(normalizar_error_tw(e));
    }
    let mut env = Entorno::nueva();
    match env.ejecutar_programa(&ss) {
        Ok(v) => ResultadoNormalizado::Ok {
            valor: v.as_ref().map(normalizar_valor),
            efectos: EfectosObservables::default(),
        },
        Err(e) => ResultadoNormalizado::Err(normalizar_error_tw(&e)),
    }
}

fn vm_resultado(src: &str) -> ResultadoNormalizado {
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errores_sem = ctx.analizar_programa(&ss);
    if let Some(e) = errores_sem.first() {
        return ResultadoNormalizado::Err(normalizar_error_tw(e));
    }
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    if let Some(e) = errs_g4.first() {
        return ResultadoNormalizado::Err(normalizar_error_tw(e));
    }
    let hir_prog = match hir::lower_programa(&ss, &ctx) {
        Ok(h) => h,
        Err(e) => {
            return ResultadoNormalizado::Err(basilisco::paridad::ErrorNormalizado {
                causa: basilisco::paridad::CausaNormalizada {
                    categoria: basilisco::paridad::CategoriaError::NoSoportado,
                    subcausa: Some("hir_no_implementado".to_string()),
                    datos: {
                        let mut d = BTreeMap::new();
                        d.insert("detalle".into(), format!("{e:?}"));
                        d
                    },
                },
            });
        }
    };
    let mir_prog = mir::lower_programa(&hir_prog);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir_prog);
    maquina.registrar_stdlib();
    let Some(idx) = maquina.indice_de("main") else {
        return ResultadoNormalizado::Err(basilisco::paridad::ErrorNormalizado {
            causa: basilisco::paridad::CausaNormalizada {
                categoria: basilisco::paridad::CategoriaError::FuncionNoEncontrada,
                subcausa: None,
                datos: {
                    let mut d = BTreeMap::new();
                    d.insert("nombre".into(), "main".into());
                    d
                },
            },
        });
    };
    match maquina.ejecutar_indice(idx, &[]) {
        Ok(v) => ResultadoNormalizado::Ok {
            valor: v.as_ref().map(normalizar_valor),
            efectos: EfectosObservables::default(),
        },
        Err(e) => ResultadoNormalizado::Err(normalizar_error_vm(&e)),
    }
}

fn assert_tw_vm_ok(src: &str) {
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(cmp, Comparacion::Equivalente),
        "TW↔VM no equivalentes.\nTW: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}\nSRC:\n{src}"
    );
}

fn assert_codigo_semantico(src: &str, codigo: &str) {
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(
        errs.iter().any(|e| e.causa.codigo() == codigo),
        "se esperaba {codigo}, got: {:?}",
        errs.iter()
            .map(|e| (e.causa.codigo(), e.mensaje.clone()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn s8_enum_unitaria_tw_vm() {
    let src = r#"
enumeracion Color { Rojo, Verde, Azul }
funcion main() {
    sea c = Color.Rojo
    imprimir(c)
}
"#;
    assert_tw_vm_ok(src);
}

#[test]
fn s8_enum_payload_posicional_tw_vm() {
    let src = r#"
enumeracion Opcion { Nada, Algo(entero) }
funcion main() {
    sea x = Opcion.Algo(42)
    imprimir(x)
}
"#;
    assert_tw_vm_ok(src);
}

#[test]
fn s8_enum_payload_multiple_tw_vm() {
    let src = r#"
enumeracion Par { P(entero, texto) }
funcion main() {
    sea p = Par.P(7, "ok")
    imprimir(p)
}
"#;
    assert_tw_vm_ok(src);
}

#[test]
fn s8_enum_variante_desconocida_bas7109() {
    let src = r#"
enumeracion Color { Rojo }
funcion main() {
    sea c = Color.Verde
}
"#;
    assert_codigo_semantico(src, "BAS-7109");
}

#[test]
fn s8_enum_payload_incompatible_bas7110() {
    let src = r#"
enumeracion Opcion { Algo(entero) }
funcion main() {
    sea x = Opcion.Algo("no")
}
"#;
    assert_codigo_semantico(src, "BAS-7110");
}

#[test]
fn s8_enum_aridad_payload_incompatible_bas7110() {
    let src = r#"
enumeracion Opcion { Algo(entero) }
funcion main() {
    sea x = Opcion.Algo(1, 2)
}
"#;
    assert_codigo_semantico(src, "BAS-7110");
}

#[test]
fn s8_enum_construccion_unitaria_con_args_bas7110() {
    let src = r#"
enumeracion Color { Rojo }
funcion main() {
    sea c = Color.Rojo(1)
}
"#;
    assert_codigo_semantico(src, "BAS-7110");
}

#[test]
fn s8_enum_variante_duplicada_bas7111() {
    let src = r#"
enumeracion Color { Rojo, Rojo }
funcion main() { }
"#;
    assert_codigo_semantico(src, "BAS-7111");
}

#[test]
fn s8_enum_igualdad_no_soportada_bas7126_tw_vm() {
    let src = r#"
enumeracion Color { Rojo, Verde }
funcion main() {
    sea a = Color.Rojo
    sea b = Color.Rojo
    a == b
}
"#;
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(cmp, Comparacion::Equivalente),
        "TW↔VM no equivalentes en BAS-7126.\nTW: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
    match tw {
        ResultadoNormalizado::Err(e) => {
            assert_eq!(e.causa.subcausa.as_deref(), Some("BAS-7126"));
        }
        other => panic!("esperaba error BAS-7126, got {other:?}"),
    }
}

#[test]
fn s8_enum_backend_nativo_soporta_basico() {
    let src = r#"
enumeracion Color { Rojo, Verde }
funcion main() -> entero {
    sea c = Color.Rojo
    coincidir c {
        Color.Rojo => { retornar 1 }
        Color.Verde => { retornar 2 }
    }
}
"#;
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(errs.is_empty(), "semántica: {errs:?}");
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(errs_g4.is_empty(), "g4: {errs_g4:?}");
    let hir_prog = hir::lower_programa(&ss, &ctx).expect("hir");
    let mir_prog = mir::lower_programa(&hir_prog);
    let rust = basilisco::backend_nativo::generar_rust(&mir_prog);
    assert!(
        !rust.contains("BAS-7124"),
        "no debe rechazar nominal soportado: {rust}"
    );
    if std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("aviso: rustc no disponible; se omite ejecución");
        return;
    }
    let dir = std::env::temp_dir().join(format!(
        "s8en-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let fuente = dir.join("gen.rs");
    let bin = dir.join(if cfg!(windows) { "gen.exe" } else { "gen" });
    std::fs::write(&fuente, &rust).unwrap();
    let out = std::process::Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("-O")
        .arg(&fuente)
        .arg("-o")
        .arg(&bin)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "rustc fallo: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let run = std::process::Command::new(&bin).output().unwrap();
    assert!(
        run.status.success(),
        "bin fallo: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "1");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn s8_enum_backend_wasm_rechaza_capacidad() {
    let src = r#"
enumeracion Color { Rojo }
funcion main() {
    sea c = Color.Rojo
    c
}
"#;
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(errs.is_empty(), "semántica: {errs:?}");
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(errs_g4.is_empty(), "g4: {errs_g4:?}");
    let hir_prog = hir::lower_programa(&ss, &ctx).expect("hir");
    let mir_prog = mir::lower_programa(&hir_prog);
    let out = basilisco::backend_entero::generar_funciones_enteras(&mir_prog);
    assert!(
        out.contains("BAS-7124"),
        "esperaba BAS-7124 en entero/wasm: {out}"
    );
}

#[test]
fn s8_nominal_igualdad_enum_rechazada_bas7126() {
    let src = r#"
enumeracion Color { Rojo, Verde }
funcion main() -> logico {
    sea a = Color.Rojo
    sea b = Color.Rojo
    retornar a == b
}
"#;
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "TW↔VM no equivalentes en BAS-7126 enum ==.\nTW: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
    match tw {
        ResultadoNormalizado::Err(e) => {
            assert_eq!(e.causa.subcausa.as_deref(), Some("BAS-7126"), "{e:?}")
        }
        other => panic!("esperaba BAS-7126, got {other:?}"),
    }

    let src_ne = r#"
enumeracion Color { Rojo, Verde }
funcion main() -> logico {
    sea a = Color.Rojo
    sea b = Color.Verde
    retornar a != b
}
"#;
    let tw2 = tw_resultado(src_ne);
    let vmr2 = vm_resultado(src_ne);
    let cmp2 = comparar_semantica(&tw2, &vmr2);
    assert!(
        matches!(
            cmp2,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "TW↔VM no equivalentes en BAS-7126 enum !=.\nTW: {tw2:?}\nVM: {vmr2:?}\nCMP: {cmp2:?}"
    );
    match tw2 {
        ResultadoNormalizado::Err(e) => {
            assert_eq!(e.causa.subcausa.as_deref(), Some("BAS-7126"), "{e:?}")
        }
        other => panic!("esperaba BAS-7126, got {other:?}"),
    }
}
