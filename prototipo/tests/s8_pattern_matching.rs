//! S8.3 - Pattern matching y exhaustividad (Fase 08).

use basilisco::{
    ContextoSemantico, EfectosObservables, Entorno, Lexer, Parser, ResultadoNormalizado, hir, mir,
    normalizar_error_tw, normalizar_error_vm, normalizar_valor, vm,
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

fn assert_paridad(src: &str) {
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    assert_eq!(
        tw, vmr,
        "paridad TW↔VM falló para:\n{src}\nTW={tw:?}\nVM={vmr:?}"
    );
}

fn assert_err_codigo(src: &str, codigo: &str) {
    let tw = tw_resultado(src);
    match tw {
        ResultadoNormalizado::Err(e) => {
            let c = e.causa.subcausa.as_deref().unwrap_or("");
            assert_eq!(
                c, codigo,
                "código esperado {codigo}, obtenido {c} en:\n{src}\nerr={e:?}"
            );
        }
        other => panic!("se esperaba error {codigo}, obtenido {other:?} en:\n{src}"),
    }
}

#[test]
fn s8_3_literal_wildcard_binding() {
    let src = r#"
        funcion main() -> entero {
            var x: entero = 2
            retornar coincidir x {
                1 => 10,
                2 => 20,
                _ => 0,
            }
        }
    "#;
    assert_paridad(src);
    let tw = tw_resultado(src);
    if let ResultadoNormalizado::Ok { valor: Some(v), .. } = tw {
        assert_eq!(format!("{v:?}"), "Entero(20)");
    } else {
        panic!("{tw:?}");
    }
}

#[test]
fn s8_3_binding_y_first_match() {
    let src = r#"
        funcion main() -> entero {
            var x: entero = 5
            retornar coincidir x {
                5 => 1,
                v => v + 10,
            }
        }
    "#;
    assert_paridad(src);
    let tw = tw_resultado(src);
    if let ResultadoNormalizado::Ok { valor: Some(v), .. } = tw {
        assert_eq!(format!("{v:?}"), "Entero(1)");
    } else {
        panic!("{tw:?}");
    }
}

#[test]
fn s8_3_bool_exhaustivo() {
    let src = r#"
        funcion main() -> entero {
            var b: logico = verdadero
            retornar coincidir b {
                verdadero => 1,
                falso => 0,
            }
        }
    "#;
    assert_paridad(src);
}

#[test]
fn s8_3_enum_variante_unitaria_y_payload() {
    let src = r#"
        enumeracion Opcion {
            Ninguno,
            Alguno(entero),
        }
        funcion main() -> entero {
            var op: Opcion = Opcion.Alguno(7)
            retornar coincidir op {
                Opcion.Ninguno => 0,
                Opcion.Alguno(v) => v,
            }
        }
    "#;
    assert_paridad(src);
}

#[test]
fn s8_3_struct_pattern() {
    let src = r#"
        estructura Punto { x: entero, b: entero }
        funcion main() -> entero {
            var p: Punto = Punto { x: 2, b: 3 }
            retornar coincidir p {
                Punto { x: a, b: c } => a + c,
            }
        }
    "#;
    assert_paridad(src);
}

#[test]
fn s8_3_anidado_enum_struct() {
    let src = r#"
        estructura Par { x: entero, b: entero }
        enumeracion E {
            A(Par),
            B(entero),
        }
        funcion main() -> entero {
            var v: E = E.A(Par { x: 4, b: 6 })
            retornar coincidir v {
                E.A(Par { x: a, b: c }) => a + c,
                E.B(n) => n,
            }
        }
    "#;
    assert_paridad(src);
}

#[test]
fn s8_3_scrutinee_expresion() {
    let src = r#"
        funcion id(x: entero) -> entero { retornar x }
        funcion main() -> entero {
            retornar coincidir id(3) {
                1 => 10,
                3 => 30,
                _ => 0,
            }
        }
    "#;
    assert_paridad(src);
}

#[test]
fn s8_3_no_exhaustivo_bas7113() {
    let src = r#"
        enumeracion Color { Rojo, Verde, Azul }
        funcion main() -> entero {
            var c: Color = Color.Rojo
            retornar coincidir c {
                Color.Rojo => 1,
                Color.Verde => 2,
            }
        }
    "#;
    assert_err_codigo(src, "BAS-7113");
}

#[test]
fn s8_3_inalcanzable_bas7114() {
    let src = r#"
        funcion main() -> entero {
            var x: entero = 1
            retornar coincidir x {
                _ => 0,
                1 => 1,
            }
        }
    "#;
    assert_err_codigo(src, "BAS-7114");
}

#[test]
fn s8_3_binding_duplicado_bas7115() {
    let src = r#"
        enumeracion Par { P(entero, entero) }
        funcion main() -> entero {
            var p: Par = Par.P(1, 2)
            retornar coincidir p {
                Par.P(x, x) => x,
            }
        }
    "#;
    assert_err_codigo(src, "BAS-7115");
}

#[test]
fn s8_3_patron_incompatible_bas7112() {
    let src = r#"
        funcion main() -> entero {
            var x: entero = 1
            retornar coincidir x {
                verdadero => 1,
                _ => 0,
            }
        }
    "#;
    assert_err_codigo(src, "BAS-7112");
}
