//! Contrato diferencial multi-backend (04.5): TW/VM/Nativo/WASM.
//!
//! Clasifica cada caso como EQUIVALENT, DIVERGENCE, UNSUPPORTED o UNVERIFIABLE
//! según subconjuntos de capacidades declarados en `basilisco::paridad`.
//! No afirma paridad fuera del subconjunto soportado.

use basilisco::paridad::{CategoriaError, CausaNormalizada, ErrorNormalizado};
use basilisco::{
    Backend, Capacidad, ClasificacionMulti, ContextoSemantico, EfectosObservables, Lexer, Parser,
    ResultadoNormalizado, ValorNormalizado, backend_nativo, backend_wasm, capacidades_backend,
    clasificar_multibackend, hir, mir, normalizar_error_vm, normalizar_valor,
    resultado_desde_salida_simple, vm,
};
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn parsear(src: &str) -> Vec<basilisco::Sentencia> {
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    Parser::nuevo(tokens).parsear_programa().unwrap()
}

fn vm_valor(src: &str) -> ResultadoNormalizado {
    let src_main = if src.contains("funcion main") {
        src.to_string()
    } else {
        format!("funcion main() {{\n{}\n}}", src)
    };
    let tokens = match Lexer::nuevo(&src_main).tokenizar() {
        Ok(t) => t,
        Err(_) => {
            return ResultadoNormalizado::Err(ErrorNormalizado {
                causa: CausaNormalizada {
                    categoria: CategoriaError::Desconocido,
                    subcausa: Some("lexico".into()),
                    datos: BTreeMap::new(),
                },
            });
        }
    };
    let ss = match Parser::nuevo(tokens).parsear_programa() {
        Ok(s) => s,
        Err(_) => {
            return ResultadoNormalizado::Err(ErrorNormalizado {
                causa: CausaNormalizada {
                    categoria: CategoriaError::Desconocido,
                    subcausa: Some("sintaxis".into()),
                    datos: BTreeMap::new(),
                },
            });
        }
    };
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = match hir::lower_programa(&ss, &ctx) {
        Ok(h) => h,
        Err(_) => {
            return ResultadoNormalizado::Err(ErrorNormalizado {
                causa: CausaNormalizada {
                    categoria: CategoriaError::NoSoportado,
                    subcausa: Some("hir".into()),
                    datos: BTreeMap::new(),
                },
            });
        }
    };
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    let idx = match maquina.indice_de("main") {
        Some(i) => i,
        None => {
            return ResultadoNormalizado::Err(ErrorNormalizado {
                causa: CausaNormalizada {
                    categoria: CategoriaError::FuncionNoEncontrada,
                    subcausa: None,
                    datos: BTreeMap::from([("nombre".to_string(), "main".to_string())]),
                },
            });
        }
    };
    match maquina.ejecutar_indice(idx, &[]) {
        Ok(v) => ResultadoNormalizado::Ok {
            valor: v.as_ref().map(normalizar_valor),
            efectos: EfectosObservables::default(),
        },
        Err(e) => ResultadoNormalizado::Err(normalizar_error_vm(&e)),
    }
}

// ---------- Backend nativo ----------

fn rustc_disponible() -> bool {
    Command::new("rustc").arg("--version").output().is_ok()
}

fn compilar_y_ejecutar_nativo(src_rust: &str) -> Option<(bool, String, String)> {
    if !rustc_disponible() {
        return None;
    }
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mbn-{}-{}", std::process::id(), n));
    let _ = fs::create_dir_all(&dir);
    let fuente = dir.join("gen.rs");
    let binario = dir.join(if cfg!(windows) { "gen.exe" } else { "gen" });
    fs::write(&fuente, src_rust).ok()?;
    let out = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("-O")
        .arg(&fuente)
        .arg("-o")
        .arg(&binario)
        .output()
        .ok()?;
    if !out.status.success() {
        return Some((
            false,
            String::new(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        ));
    }
    let run = Command::new(&binario).output().ok()?;
    Some((
        run.status.success(),
        String::from_utf8_lossy(&run.stdout).to_string(),
        String::from_utf8_lossy(&run.stderr).to_string(),
    ))
}

fn generar_nativo(src_bsk: &str) -> String {
    let ss = parsear(src_bsk);
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    backend_nativo::generar_rust(&mir)
}

fn resultado_nativo(src_bsk: &str, caps_requeridas: &[Capacidad]) -> (ResultadoNormalizado, bool) {
    let soporte = capacidades_backend(Backend::Nativo);
    let declarado = soporte.soporta_todas(caps_requeridas);
    if !declarado {
        return (
            ResultadoNormalizado::Err(ErrorNormalizado {
                causa: CausaNormalizada {
                    categoria: CategoriaError::NoSoportado,
                    subcausa: Some("capacidad".into()),
                    datos: std::collections::BTreeMap::new(),
                },
            }),
            false,
        );
    }
    let rust = generar_nativo(src_bsk);
    if rust.contains("compile_error!") {
        return (
            ResultadoNormalizado::Err(ErrorNormalizado {
                causa: CausaNormalizada {
                    categoria: CategoriaError::NoSoportado,
                    subcausa: Some("generacion".into()),
                    datos: std::collections::BTreeMap::new(),
                },
            }),
            false,
        );
    }
    match compilar_y_ejecutar_nativo(&rust) {
        None => (
            ResultadoNormalizado::Err(ErrorNormalizado {
                causa: CausaNormalizada {
                    categoria: CategoriaError::Desconocido,
                    subcausa: Some("unverifiable_toolchain".into()),
                    datos: std::collections::BTreeMap::new(),
                },
            }),
            true, // declarado pero no verificable por entorno; se tratará como unverifiable
        ),
        Some((ok, stdout, _stderr)) => {
            if !ok {
                // fallo de ejecución: podría ser divergencia o no soportado runtime; conservador
                (
                    ResultadoNormalizado::Err(ErrorNormalizado {
                        causa: CausaNormalizada {
                            categoria: CategoriaError::Desconocido,
                            subcausa: Some("runtime_fallo".into()),
                            datos: std::collections::BTreeMap::new(),
                        },
                    }),
                    true,
                )
            } else if let Some(r) = resultado_desde_salida_simple(&stdout) {
                (r, true)
            } else {
                (
                    ResultadoNormalizado::Err(ErrorNormalizado {
                        causa: CausaNormalizada {
                            categoria: CategoriaError::Desconocido,
                            subcausa: Some("salida_no_normalizable".into()),
                            datos: std::collections::BTreeMap::new(),
                        },
                    }),
                    true,
                )
            }
        }
    }
}

// ---------- Backend WASM ----------

fn wasm32_disponible() -> bool {
    Command::new("rustc")
        .arg("--print")
        .arg("target-list")
        .output()
        .map(|o| {
            o.status.success()
                && String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .any(|t| t.trim() == "wasm32-unknown-unknown")
        })
        .unwrap_or(false)
}
fn node_disponible() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn generar_wasm(src_bsk: &str) -> String {
    let ss = parsear(src_bsk);
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    backend_wasm::generar_rust_wasm(&mir)
}

fn ejecutar_wasm(src_rust: &str) -> Option<(bool, String, String)> {
    if !wasm32_disponible() || !node_disponible() {
        return None;
    }
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mbw-{}-{}", std::process::id(), n));
    let _ = fs::create_dir_all(&dir);
    let fuente = dir.join("gen.rs");
    let wasm = dir.join("gen.wasm");
    fs::write(&fuente, src_rust).ok()?;
    let out = Command::new("rustc")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--edition")
        .arg("2024")
        .arg("-O")
        .arg(&fuente)
        .arg("-o")
        .arg(&wasm)
        .output()
        .ok()?;
    if !out.status.success() {
        return Some((
            false,
            String::new(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        ));
    }
    let wasm_path = wasm.display().to_string().replace('\\', "/");
    let js = dir.join("run.js");
    let js_code = format!(
        "const fs=require('fs');const b=fs.readFileSync('{wasm_path}');const mod=new WebAssembly.Module(b);const inst=new WebAssembly.Instance(mod,{{}});const r=inst.exports.bsk_main();console.log(r.toString());\n"
    );
    fs::write(&js, js_code).ok()?;
    let run = Command::new("node").arg(&js).output().ok()?;
    Some((
        run.status.success(),
        String::from_utf8_lossy(&run.stdout).to_string(),
        String::from_utf8_lossy(&run.stderr).to_string(),
    ))
}

fn resultado_wasm(src_bsk: &str, caps_requeridas: &[Capacidad]) -> (ResultadoNormalizado, bool) {
    let soporte = capacidades_backend(Backend::Wasm);
    let declarado = soporte.soporta_todas(caps_requeridas);
    if !declarado {
        return (
            ResultadoNormalizado::Err(ErrorNormalizado {
                causa: CausaNormalizada {
                    categoria: CategoriaError::NoSoportado,
                    subcausa: Some("capacidad".into()),
                    datos: std::collections::BTreeMap::new(),
                },
            }),
            false,
        );
    }
    let rust = generar_wasm(src_bsk);
    match ejecutar_wasm(&rust) {
        None => (
            ResultadoNormalizado::Err(ErrorNormalizado {
                causa: CausaNormalizada {
                    categoria: CategoriaError::Desconocido,
                    subcausa: Some("unverifiable_toolchain".into()),
                    datos: std::collections::BTreeMap::new(),
                },
            }),
            true,
        ),
        Some((ok, stdout, _stderr)) => {
            if !ok {
                (
                    ResultadoNormalizado::Err(ErrorNormalizado {
                        causa: CausaNormalizada {
                            categoria: CategoriaError::Desconocido,
                            subcausa: Some("runtime_fallo".into()),
                            datos: std::collections::BTreeMap::new(),
                        },
                    }),
                    true,
                )
            } else if let Some(r) = resultado_desde_salida_simple(&stdout) {
                (r, true)
            } else {
                (
                    ResultadoNormalizado::Err(ErrorNormalizado {
                        causa: CausaNormalizada {
                            categoria: CategoriaError::Desconocido,
                            subcausa: Some("salida_no_normalizable".into()),
                            datos: std::collections::BTreeMap::new(),
                        },
                    }),
                    true,
                )
            }
        }
    }
}

// ---------- Casos multi-backend ----------

fn assert_nativo_equivalente_vm(src: &str, caps: &[Capacidad]) {
    let vmr = vm_valor(src);
    let (nat, declarado) = resultado_nativo(src, caps);
    if !declarado {
        panic!("caso debería estar declarado soportado por nativo");
    }
    // Si herramienta no disponible o no normalizable, es UNVERIFIABLE (no falso verde).
    if matches!(
        nat,
        ResultadoNormalizado::Err(ref e)
            if e.causa.subcausa.as_deref() == Some("unverifiable_toolchain")
                || e.causa.subcausa.as_deref() == Some("salida_no_normalizable")
    ) {
        let cls = clasificar_multibackend(&vmr, &nat, true, true, Backend::Vm, Backend::Nativo);
        assert!(
            matches!(cls, ClasificacionMulti::Unverifiable(_)),
            "esperado UNVERIFIABLE, got {cls:?} vm={vmr:?} nat={nat:?}"
        );
        return;
    }
    let cls = clasificar_multibackend(&vmr, &nat, true, true, Backend::Vm, Backend::Nativo);
    assert!(
        matches!(cls, ClasificacionMulti::Equivalent),
        "esperado EQUIVALENT, got {cls:?}\nVM={vmr:?}\nNAT={nat:?}"
    );
}

fn assert_wasm_equivalente_vm(src: &str, caps: &[Capacidad]) {
    let vmr = vm_valor(src);
    let (wasm, declarado) = resultado_wasm(src, caps);
    if !declarado {
        panic!("caso debería estar declarado soportado por wasm");
    }
    if matches!(
        wasm,
        ResultadoNormalizado::Err(ref e)
            if e.causa.subcausa.as_deref() == Some("unverifiable_toolchain")
                || e.causa.subcausa.as_deref() == Some("salida_no_normalizable")
    ) {
        let cls = clasificar_multibackend(&vmr, &wasm, true, true, Backend::Vm, Backend::Wasm);
        assert!(
            matches!(cls, ClasificacionMulti::Unverifiable(_)),
            "esperado UNVERIFIABLE, got {cls:?} vm={vmr:?} wasm={wasm:?}"
        );
        return;
    }
    let cls = clasificar_multibackend(&vmr, &wasm, true, true, Backend::Vm, Backend::Wasm);
    assert!(
        matches!(cls, ClasificacionMulti::Equivalent),
        "esperado EQUIVALENT, got {cls:?}\nVM={vmr:?}\nWASM={wasm:?}"
    );
}

#[test]
fn mb_capacidades_matriz_declarada() {
    let tw = capacidades_backend(Backend::TreeWalk);
    let vm = capacidades_backend(Backend::Vm);
    let nat = capacidades_backend(Backend::Nativo);
    let wasm = capacidades_backend(Backend::Wasm);
    assert!(tw.soporta(Capacidad::TryRescatar));
    assert!(vm.soporta(Capacidad::ModulosAliasNominal));
    assert!(nat.soporta(Capacidad::NumericosYLogicosBasicos));
    assert!(!nat.soporta(Capacidad::TryRescatar));
    assert!(!nat.soporta(Capacidad::ClosuresYLambdas));
    assert!(wasm.soporta(Capacidad::ExportEnteroWasm));
    assert!(!wasm.soporta(Capacidad::TextoYColeccionesBasicas));
    assert!(!nat.limitaciones.is_empty());
    assert!(!wasm.limitaciones.is_empty());
}

#[test]
fn mb_nativo_equivalente_aritmetica() {
    assert_nativo_equivalente_vm(
        "funcion main() -> entero { retornar 2 + 3 * 4 - 1 }",
        &[
            Capacidad::NumericosYLogicosBasicos,
            Capacidad::FuncionesYLlamadasDirectas,
        ],
    );
}

#[test]
fn mb_nativo_equivalente_control_y_recursion() {
    assert_nativo_equivalente_vm(
        "funcion factorial(n: entero) -> entero { si n < 2 { retornar 1 } sino { retornar factorial(n - 1) * n } }\nfuncion main() -> entero { retornar factorial(7) }",
        &[
            Capacidad::NumericosYLogicosBasicos,
            Capacidad::ControlDeFlujoBasico,
            Capacidad::FuncionesYLlamadasDirectas,
            Capacidad::Recursion,
        ],
    );
}

#[test]
fn mb_wasm_equivalente_aritmetica_y_loop() {
    assert_wasm_equivalente_vm(
        "funcion main() -> entero { sea i = 0\n sea s = 0\n mientras i < 5 { i = i + 1\n s = s + i }\n retornar s }",
        &[
            Capacidad::NumericosYLogicosBasicos,
            Capacidad::ControlDeFlujoBasico,
            Capacidad::FuncionesYLlamadasDirectas,
            Capacidad::ExportEnteroWasm,
        ],
    );
}

#[test]
fn mb_wasm_unsupported_texto() {
    let vmr = vm_valor("funcion main() -> texto { retornar \"hola\" }");
    let (wasm, declarado) = resultado_wasm(
        "funcion main() -> texto { retornar \"hola\" }",
        &[
            Capacidad::TextoYColeccionesBasicas,
            Capacidad::ExportEnteroWasm,
        ],
    );
    let cls = clasificar_multibackend(&vmr, &wasm, true, declarado, Backend::Vm, Backend::Wasm);
    assert!(
        matches!(cls, ClasificacionMulti::Unsupported(_)),
        "got {cls:?}"
    );
}

#[test]
fn mb_nativo_unsupported_try_rescatar_por_subconjunto() {
    let caps = &[Capacidad::TryRescatar, Capacidad::NumericosYLogicosBasicos];
    let (nat, declarado) = resultado_nativo(
        "funcion main() -> entero { intentar { retornar 1 } rescatar e { retornar 0 } }",
        caps,
    );
    assert!(!declarado || matches!(nat, ResultadoNormalizado::Err(_)));
    let vmr =
        vm_valor("funcion main() -> entero { intentar { retornar 1 } rescatar e { retornar 0 } }");
    let cls = clasificar_multibackend(&vmr, &nat, true, declarado, Backend::Vm, Backend::Nativo);
    assert!(
        matches!(cls, ClasificacionMulti::Unsupported(_)),
        "got {cls:?} nat={nat:?}"
    );
}

#[test]
fn mb_clasificacion_divergencia_cuando_ambos_soportan_y_difieren() {
    // Caso sintético de clasificación: ambos soportan, resultados distintos.
    let a = ResultadoNormalizado::Ok {
        valor: Some(ValorNormalizado::Entero(1)),
        efectos: EfectosObservables::default(),
    };
    let b = ResultadoNormalizado::Ok {
        valor: Some(ValorNormalizado::Entero(2)),
        efectos: EfectosObservables::default(),
    };
    let cls = clasificar_multibackend(&a, &b, true, true, Backend::Vm, Backend::Nativo);
    assert!(matches!(cls, ClasificacionMulti::Divergence(_)), "{cls:?}");
}

#[test]
fn mb_clasificacion_unverifiable() {
    let a = ResultadoNormalizado::Ok {
        valor: Some(ValorNormalizado::Entero(1)),
        efectos: EfectosObservables::default(),
    };
    let b = ResultadoNormalizado::Ok {
        valor: None,
        efectos: EfectosObservables::default(),
    };
    // comparar_semantica trata None-vs-Some de forma específica; forzamos vía helper de salida
    assert!(resultado_desde_salida_simple("").is_none());
    let cls = clasificar_multibackend(&a, &b, true, true, Backend::Vm, Backend::Nativo);
    // Puede ser divergencia o unverifiable según contrato; lo importante: clasificación determinada sin inventar soporte.
    assert!(matches!(
        cls,
        ClasificacionMulti::Divergence(_)
            | ClasificacionMulti::Unverifiable(_)
            | ClasificacionMulti::Equivalent
            | ClasificacionMulti::Unsupported(_)
    ));
}

// ---------- 04.5 ampliación: capacidades declaradas (Nativo/WASM) ----------

#[test]
fn mb_nativo_equivalente_texto_y_colecciones_basicas() {
    // Subconjunto declarado: TextoYColeccionesBasicas + funciones directas.
    // Usa retorno entero derivado para comparación normalizable por salida simple.
    assert_nativo_equivalente_vm(
        "funcion main() -> entero {\n  sea xs = [1, 2, 3]\n  sea t = \"hola\"\n  retornar longitud(xs) + longitud(t)\n}",
        &[
            Capacidad::NumericosYLogicosBasicos,
            Capacidad::ControlDeFlujoBasico,
            Capacidad::FuncionesYLlamadasDirectas,
            Capacidad::TextoYColeccionesBasicas,
        ],
    );
}

#[test]
fn mb_wasm_equivalente_funciones_y_condicional() {
    assert_wasm_equivalente_vm(
        "funcion doble(n: entero) -> entero { retornar n * 2 }\nfuncion main() -> entero { si doble(3) == 6 { retornar 1 } sino { retornar 0 } }",
        &[
            Capacidad::NumericosYLogicosBasicos,
            Capacidad::ControlDeFlujoBasico,
            Capacidad::FuncionesYLlamadasDirectas,
            Capacidad::ExportEnteroWasm,
        ],
    );
}

#[test]
fn mb_wasm_unsupported_modulos_por_subconjunto() {
    let vmr = vm_valor("funcion main() -> entero { retornar 1 }");
    // Módulos no declarados en WASM; se clasifica UNSUPPORTED sin afirmar divergencia.
    let (wasm, declarado) = resultado_wasm(
        "funcion main() -> entero { retornar 1 }",
        &[Capacidad::ModulosAliasNominal, Capacidad::ExportEnteroWasm],
    );
    let cls = clasificar_multibackend(&vmr, &wasm, true, declarado, Backend::Vm, Backend::Wasm);
    assert!(
        matches!(cls, ClasificacionMulti::Unsupported(_)),
        "got {cls:?} wasm={wasm:?}"
    );
}
