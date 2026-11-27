//! S8.1 - Structs nominales (Fase 08). Pruebas positivas, negativas y diferenciales TW↔VM.

use basilisco::{
    Comparacion, ContextoSemantico, EfectosObservables, Entorno, Lexer, Parser,
    ResultadoNormalizado, cargador::CargadorModulos, comparar_semantica, hir, mir,
    normalizar_error_tw, normalizar_error_vm, normalizar_valor, vm,
};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

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
    let hir = match hir::lower_programa(&ss, &ctx) {
        Ok(h) => h,
        Err(e) => {
            return ResultadoNormalizado::Err(basilisco::paridad::ErrorNormalizado {
                causa: basilisco::paridad::CausaNormalizada {
                    categoria: basilisco::paridad::CategoriaError::NoSoportado,
                    subcausa: Some("hir_no_implementado".to_string()),
                    datos: {
                        let mut d = std::collections::BTreeMap::new();
                        d.insert("detalle".into(), format!("{e:?}"));
                        d
                    },
                },
            });
        }
    };
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    let Some(idx) = maquina.indice_de("main") else {
        return ResultadoNormalizado::Err(basilisco::paridad::ErrorNormalizado {
            causa: basilisco::paridad::CausaNormalizada {
                categoria: basilisco::paridad::CategoriaError::FuncionNoEncontrada,
                subcausa: None,
                datos: {
                    let mut d = std::collections::BTreeMap::new();
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

fn assert_tw_vm_equivalente(src: &str) {
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(cmp, Comparacion::Equivalente),
        "TW!=VM\nTW: {:?}\nVM: {:?}\nCMP: {:?}\nSRC:\n{}",
        tw,
        vmr,
        cmp,
        src
    );
}

fn assert_semantico_error(src: &str, codigo_esperado: &str) {
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errores = ctx.analizar_programa(&ss);
    assert!(
        errores.iter().any(|e| e.causa.codigo() == codigo_esperado),
        "se esperaba {codigo_esperado}, got: {:?}",
        errores
            .iter()
            .map(|e| (e.causa.codigo(), e.mensaje.clone()))
            .collect::<Vec<_>>()
    );
}

fn assert_parse_error(src: &str) {
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let r = Parser::nuevo(tokens).parsear_programa();
    assert!(r.is_err(), "se esperaba error de parse, got Ok");
}

// ---- Positivos ----

#[test]
fn s8_declara_y_construye_struct() {
    let src = r#"
estructura Punto { x: entero, yv: entero }
funcion main() {
    var p: Punto = Punto { x: 1, yv: 2 }
    retornar p.x
}
"#;
    assert_tw_vm_equivalente(src);
}

#[test]
fn s8_acceso_campo() {
    let src = r#"
estructura Punto { x: entero, yv: entero }
funcion main() {
    var p: Punto = Punto { x: 3, yv: 4 }
    retornar p.yv
}
"#;
    assert_tw_vm_equivalente(src);
}

#[test]
fn s8_asignacion_campo_directo() {
    let src = r#"
estructura Punto { x: entero, yv: entero }
funcion main() {
    var p: Punto = Punto { x: 1, yv: 2 }
    p.x = 10
    retornar p.x
}
"#;
    assert_tw_vm_equivalente(src);
}

#[test]
fn s8_struct_vacia() {
    let src = r#"
estructura Vacia { }
funcion main() {
    var v: Vacia = Vacia { }
    retornar 1
}
"#;
    assert_tw_vm_equivalente(src);
}

#[test]
fn s8_struct_anidada() {
    let src = r#"
estructura Punto { x: entero, yv: entero }
estructura Linea { a: Punto, b: Punto }
funcion main() {
    var l: Linea = Linea { a: Punto { x: 1, yv: 2 }, b: Punto { x: 3, yv: 4 } }
    retornar l.b.x
}
"#;
    assert_tw_vm_equivalente(src);
}

#[test]
fn s8_struct_parametro_retorno() {
    let src = r#"
estructura Punto { x: entero, yv: entero }
funcion mover(p: Punto, dx: entero) -> Punto {
    var q: Punto = Punto { x: p.x + dx, yv: p.yv }
    retornar q
}
funcion main() {
    var p: Punto = Punto { x: 1, yv: 2 }
    var r: Punto = mover(p, 5)
    retornar r.x
}
"#;
    assert_tw_vm_equivalente(src);
}

#[test]
fn s8_copia_por_valor() {
    let src = r#"
estructura Punto { x: entero, yv: entero }
funcion main() {
    var a: Punto = Punto { x: 1, yv: 2 }
    var b: Punto = a
    b.x = 9
    retornar a.x
}
"#;
    assert_tw_vm_equivalente(src);
}

// ---- Negativos ----

#[test]
fn s8_campo_desconocido_b7103() {
    let src = r#"
estructura Punto { x: entero }
funcion main() {
    var p: Punto = Punto { x: 1 }
    retornar p.yv
}
"#;
    assert_semantico_error(src, "BAS-7103");
}

#[test]
fn s8_campo_faltante_b7105() {
    let src = r#"
estructura Punto { x: entero, yv: entero }
funcion main() {
    var p: Punto = Punto { x: 1 }
    retornar 0
}
"#;
    assert_semantico_error(src, "BAS-7105");
}

#[test]
fn s8_campo_sobrante_b7106() {
    let src = r#"
estructura Punto { x: entero }
funcion main() {
    var p: Punto = Punto { x: 1, yv: 2 }
    retornar 0
}
"#;
    assert_semantico_error(src, "BAS-7106");
}

#[test]
fn s8_campo_duplicado_b7104() {
    let src = r#"
estructura Punto { x: entero, yv: entero }
funcion main() {
    var p: Punto = Punto { x: 1, x: 2, yv: 3 }
    retornar 0
}
"#;
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let parse = Parser::nuevo(tokens).parsear_programa();
    if let Err(e) = parse {
        assert!(e.mensaje.contains("duplicado"), "{e:?}");
        return;
    }
    let mut ctx = ContextoSemantico::nuevo();
    let errores = ctx.analizar_programa(&parse.unwrap());
    assert!(
        errores.iter().any(|e| e.causa.codigo() == "BAS-7104"),
        "{:?}",
        errores
            .iter()
            .map(|e| (e.causa.codigo(), e.mensaje.clone()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn s8_tipo_incompatible_b7107() {
    let src = r#"
estructura Punto { x: entero }
funcion main() {
    var p: Punto = Punto { x: "hola" }
    retornar 0
}
"#;
    assert_semantico_error(src, "BAS-7107");
}

#[test]
fn s8_tipo_nominal_desconocido_b7101() {
    let src = r#"
funcion main() {
    var p: NoExiste = NoExiste { x: 1 }
    retornar 0
}
"#;
    assert_semantico_error(src, "BAS-7101");
}

#[test]
fn s8_igualdad_nominal_rechazada_b7126() {
    let src = r#"
estructura Punto { x: entero }
funcion main() {
    var a: Punto = Punto { x: 1 }
    var b: Punto = Punto { x: 1 }
    si a == b { retornar 1 }
    retornar 0
}
"#;
    assert_semantico_error(src, "BAS-7126");
}

#[test]
fn s8_recursividad_valor_b7130() {
    let src = r#"
estructura Nodo { siguiente: Nodo }
funcion main() { retornar 0 }
"#;
    assert_semantico_error(src, "BAS-7130");
}

#[test]
fn s8_duplicado_nominal_b7102() {
    let src = r#"
estructura Punto { x: entero }
estructura Punto { x: entero }
funcion main() { retornar 0 }
"#;
    assert_semantico_error(src, "BAS-7102");
}

#[test]
fn s8_asignacion_campo_inmutable() {
    let src = r#"
estructura Punto { x: entero }
funcion main() {
    dato p: Punto = Punto { x: 1 }
    p.x = 2
    retornar 0
}
"#;
    assert_semantico_error(src, "E1029");
}

#[test]
fn s8_lvalue_campo_indice_rechazado_c2() {
    let src = r#"
estructura Caja { xs: lista de entero }
funcion main() {
    var c: Caja = Caja { xs: [1] }
    c.xs[0] = 2
    retornar 0
}
"#;
    // C2: solo id.campo = expr
    assert_parse_error(src);
}

fn temp_dir_s8() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tid = std::thread::current().id();
    let dir = std::env::temp_dir().join(format!("basilisco-s8-{n}-{seq}-{tid:?}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn escribir(dir: &std::path::Path, nombre: &str, contenido: &str) {
    fs::write(dir.join(nombre), contenido).unwrap();
}

fn analizar_archivo(root: &std::path::Path, archivo: &str) -> Vec<basilisco::ErrorEjecucion> {
    let ruta = root.join(archivo);
    let fuente = fs::read_to_string(&ruta).unwrap();
    let tokens = Lexer::nuevo(&fuente).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let cargador = Arc::new(Mutex::new(CargadorModulos::nuevo(root.to_path_buf())));
    let mut ctx = ContextoSemantico::con_cargador(cargador, Some(ruta.display().to_string()));
    ctx.analizar_programa(&ss)
}

#[test]
fn s8_c3_modulo_publico_struct_usable() {
    let dir = temp_dir_s8();
    escribir(
        &dir,
        "lib.bsk",
        "publico estructura Punto { publico x: entero }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" { Punto }\nfuncion main() {\n  var p: Punto = Punto { x: 7 }\n  retornar p.x\n}\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn s8_c3_modulo_struct_privado_no_exportado() {
    let dir = temp_dir_s8();
    escribir(&dir, "lib.bsk", "estructura Secreto { x: entero }\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" { Secreto }\nfuncion main() { retornar 0 }\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(
        !e.is_empty(),
        "debe fallar por privado/no exportado: {:?}",
        e
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn s8_c3_modulo_campo_privado_b7128() {
    let dir = temp_dir_s8();
    escribir(&dir, "lib.bsk", "publico estructura Punto { x: entero }\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" { Punto }\nfuncion main() {\n  var p: Punto = Punto { x: 1 }\n  retornar p.x\n}\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(
        e.iter().any(|x| x.causa.codigo() == "BAS-7128"),
        "esperaba BAS-7128, got: {:?}",
        e.iter()
            .map(|x| (x.causa.codigo(), x.mensaje.clone()))
            .collect::<Vec<_>>()
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn s8_c3_modulo_exportar_struct() {
    let dir = temp_dir_s8();
    escribir(
        &dir,
        "lib.bsk",
        "exportar estructura Punto { publico x: entero }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" { Punto }\nfuncion main() {\n  var p: Punto = Punto { x: 3 }\n  retornar p.x\n}\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn s8_b7124_backend_nativo_soporta_nominal_basico() {
    let src = r#"
estructura Punto { x: entero }
funcion main() -> entero {
    var p: Punto = Punto { x: 1 }
    retornar p.x
}
"#;
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(errs.is_empty(), "{:?}", errs);
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(errs_g4.is_empty(), "{:?}", errs_g4);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mirp = mir::lower_programa(&hir);
    let rust = basilisco::backend_nativo::generar_rust(&mirp);
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
        "s8n-{}-{}",
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
fn s8_b7124_backend_wasm_rechaza_nominal() {
    let src = r#"
estructura Punto { x: entero }
funcion main() {
    var p: Punto = Punto { x: 1 }
    retornar p.x
}
"#;
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(errs.is_empty(), "{:?}", errs);
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(errs_g4.is_empty(), "{:?}", errs_g4);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mirp = mir::lower_programa(&hir);
    let rust = basilisco::backend_wasm::generar_rust_wasm(&mirp);
    assert!(
        rust.contains("BAS-7124"),
        "esperaba BAS-7124 en salida wasm: {rust}"
    );
}

#[test]
fn s8_nominal_igualdad_struct_rechazada_bas7126() {
    let src = r#"
estructura Punto { x: entero }
funcion main() -> logico {
    sea a = Punto { x: 1 }
    sea b = Punto { x: 1 }
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
        "TW↔VM no equivalentes en BAS-7126 struct ==.\nTW: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
    match tw {
        ResultadoNormalizado::Err(e) => {
            assert_eq!(e.causa.subcausa.as_deref(), Some("BAS-7126"), "{e:?}")
        }
        other => panic!("esperaba BAS-7126, got {other:?}"),
    }

    let src_ne = r#"
estructura Punto { x: entero }
funcion main() -> logico {
    sea a = Punto { x: 1 }
    sea b = Punto { x: 2 }
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
        "TW↔VM no equivalentes en BAS-7126 struct !=.\nTW: {tw2:?}\nVM: {vmr2:?}\nCMP: {cmp2:?}"
    );
    match tw2 {
        ResultadoNormalizado::Err(e) => {
            assert_eq!(e.causa.subcausa.as_deref(), Some("BAS-7126"), "{e:?}")
        }
        other => panic!("esperaba BAS-7126, got {other:?}"),
    }
}
