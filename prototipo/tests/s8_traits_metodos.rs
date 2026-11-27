//! S8.4 - Métodos inherentes y traits (Fase 08). Positivos, negativos, TW↔VM.

use basilisco::{
    ContextoSemantico, EfectosObservables, Entorno, Lexer, Parser, ResultadoNormalizado,
    cargador::CargadorModulos, hir, mir, normalizar_error_tw, normalizar_valor, vm,
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
        Err(e) => ResultadoNormalizado::Err(basilisco::normalizar_error_vm(&e)),
    }
}

fn assert_tw_vm_ok(src: &str, esperado: basilisco::ValorNormalizado) {
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    match (&tw, &vmr) {
        (
            ResultadoNormalizado::Ok { valor: v1, .. },
            ResultadoNormalizado::Ok { valor: v2, .. },
        ) => {
            assert_eq!(v1.as_ref(), Some(&esperado), "TW inesperado: {tw:?}");
            assert_eq!(v2.as_ref(), Some(&esperado), "VM inesperado: {vmr:?}");
        }
        _ => panic!("esperado Ok en TW y VM, tw={tw:?}, vm={vmr:?}"),
    }
}

fn assert_sem_err(src: &str, codigo: &str) {
    let ss = parsear(&format!("{}\nmain()", src));
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(
        errs.iter().any(|e| e.causa.codigo() == codigo),
        "se esperaba {codigo}, errs={:?}",
        errs.iter().map(|e| e.causa.codigo()).collect::<Vec<_>>()
    );
}

#[test]
fn s8_4_metodo_instancia_yo() {
    let src = r#"
estructura Punto { x: entero, yv: entero }
implementar Punto {
    publico funcion suma(yo) -> entero { retornar yo.x + yo.yv }
}
funcion main() -> entero {
    sea p = Punto { x: 2, yv: 3 }
    retornar p.suma()
}
"#;
    assert_tw_vm_ok(src, basilisco::ValorNormalizado::Entero(5));
}

#[test]
fn s8_4_metodo_instancia_yo_var_muta_copia_local() {
    let src = r#"
estructura Contador { n: entero }
implementar Contador {
    publico funcion inc(yo var) -> entero {
        yo.n = yo.n + 1
        retornar yo.n
    }
}
funcion main() -> entero {
    sea c = Contador { n: 1 }
    sea r = c.inc()
    retornar r + c.n
}
"#;
    // yo var muta copia local: inc retorna 2, c.n sigue 1 => 3
    assert_tw_vm_ok(src, basilisco::ValorNormalizado::Entero(3));
}

#[test]
fn s8_4_metodo_asociado() {
    let src = r#"
estructura Punto { x: entero, yv: entero }
implementar Punto {
    publico funcion origen() -> Punto { retornar Punto { x: 0, yv: 0 } }
    publico funcion suma(yo) -> entero { retornar yo.x + yo.yv }
}
funcion main() -> entero {
    sea p = Punto.origen()
    retornar p.suma()
}
"#;
    assert_tw_vm_ok(src, basilisco::ValorNormalizado::Entero(0));
}

#[test]
fn s8_4_trait_completo_y_llamada() {
    let src = r#"
rasgo Mostrable {
    funcion mostrar_texto(yo) -> texto
}
estructura Punto { x: entero, yv: entero }
implementar Mostrable para Punto {
    publico funcion mostrar_texto(yo) -> texto { retornar "Punto" }
}
funcion main() -> texto {
    sea p = Punto { x: 1, yv: 2 }
    retornar p.mostrar_texto()
}
"#;
    assert_tw_vm_ok(src, basilisco::ValorNormalizado::Texto("Punto".to_string()));
}

#[test]
fn s8_4_bas7117_metodo_no_encontrado() {
    let src = r#"
estructura Punto { x: entero }
funcion main() -> entero {
    sea p = Punto { x: 1 }
    retornar p.no_existe()
}
"#;
    assert_sem_err(src, "BAS-7117");
}

#[test]
fn s8_4_bas7118_firma_incompatible() {
    let src = r#"
rasgo R { funcion f(yo) -> entero }
estructura A { x: entero }
implementar R para A {
    publico funcion f(yo) -> texto { retornar "x" }
}
funcion main() -> entero { retornar 0 }
"#;
    assert_sem_err(src, "BAS-7118");
}

#[test]
fn s8_4_bas7118_colision_campo_metodo() {
    let src = r#"
estructura A { x: entero }
implementar A {
    publico funcion x(yo) -> entero { retornar 1 }
}
funcion main() -> entero { retornar 0 }
"#;
    assert_sem_err(src, "BAS-7118");
}

#[test]
fn s8_4_bas7119_trait_no_encontrado() {
    let src = r#"
estructura A { x: entero }
implementar NoExiste para A {
    publico funcion f(yo) -> entero { retornar 1 }
}
funcion main() -> entero { retornar 0 }
"#;
    assert_sem_err(src, "BAS-7119");
}

#[test]
fn s8_4_bas7121_impl_incompleta() {
    let src = r#"
rasgo R { funcion f(yo) -> entero funcion g(yo) -> entero }
estructura A { x: entero }
implementar R para A {
    publico funcion f(yo) -> entero { retornar 1 }
}
funcion main() -> entero { retornar 0 }
"#;
    assert_sem_err(src, "BAS-7121");
}

#[test]
fn s8_4_bas7122_impl_duplicada() {
    let src = r#"
rasgo R { funcion f(yo) -> entero }
estructura A { x: entero }
implementar R para A { publico funcion f(yo) -> entero { retornar 1 } }
implementar R para A { publico funcion f(yo) -> entero { retornar 2 } }
funcion main() -> entero { retornar 0 }
"#;
    assert_sem_err(src, "BAS-7122");
}

#[test]
fn s8_4_bas7118_colision_inherente_trait() {
    let src = r#"
rasgo R { funcion f(yo) -> entero }
estructura A { x: entero }
implementar A { publico funcion f(yo) -> entero { retornar 1 } }
implementar R para A { publico funcion f(yo) -> entero { retornar 2 } }
funcion main() -> entero { retornar 0 }
"#;
    assert_sem_err(src, "BAS-7118");
}

#[test]
fn s8_4_bas7120_trait_no_implementado() {
    let src = r#"
rasgo R { funcion f(yo) -> entero }
estructura A { x: entero }
funcion main() -> entero {
    sea a = A { x: 1 }
    retornar a.f()
}
"#;
    assert_sem_err(src, "BAS-7120");
}

fn temp_dir_s8t() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tid = std::thread::current().id();
    let dir = std::env::temp_dir().join(format!("basilisco-s8t-{n}-{seq}-{tid:?}"));
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
fn s8_4_bas7128_metodo_privado_no_visible_modulo() {
    let dir = temp_dir_s8t();
    escribir(
        &dir,
        "lib.bsk",
        "publico estructura Punto { publico x: entero }\nimplementar Punto {\n  funcion secreto(yo) -> entero { retornar yo.x }\n  publico funcion visible(yo) -> entero { retornar yo.x }\n}\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" { Punto }\nfuncion main() -> entero {\n  sea p = Punto { x: 1 }\n  retornar p.secreto()\n}\n",
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
fn s8_4_bas7128_metodo_publico_visible_modulo() {
    let dir = temp_dir_s8t();
    escribir(
        &dir,
        "lib.bsk",
        "publico estructura Punto { publico x: entero }\nimplementar Punto {\n  publico funcion visible(yo) -> entero { retornar yo.x + 1 }\n}\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" { Punto }\nfuncion main() -> entero {\n  sea p = Punto { x: 2 }\n  retornar p.visible()\n}\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(
        e.is_empty(),
        "errores inesperados: {:?}",
        e.iter()
            .map(|x| (x.causa.codigo(), x.mensaje.clone()))
            .collect::<Vec<_>>()
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn s8_4_bas7122_coherencia_impl_fuera_de_modulos() {
    let dir = temp_dir_s8t();
    escribir(
        &dir,
        "rasgo.bsk",
        "publico rasgo R { funcion f(yo) -> entero }\n",
    );
    escribir(
        &dir,
        "tipo.bsk",
        "publico estructura A { publico x: entero }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"rasgo.bsk\" { R }\nusar \"tipo.bsk\" { A }\nimplementar R para A {\n  publico funcion f(yo) -> entero { retornar yo.x }\n}\nfuncion main() -> entero { retornar 0 }\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(
        e.iter().any(|x| x.causa.codigo() == "BAS-7122"),
        "esperaba BAS-7122 coherencia, got: {:?}",
        e.iter()
            .map(|x| (x.causa.codigo(), x.mensaje.clone()))
            .collect::<Vec<_>>()
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn s8_4_trait_metodo_con_param_tw_vm() {
    let src = r#"
rasgo Sumable {
    funcion sumar(yo, n: entero) -> entero
}
estructura Contador { v: entero }
implementar Sumable para Contador {
    publico funcion sumar(yo, n: entero) -> entero { retornar yo.v + n }
}
funcion main() -> entero {
    sea c = Contador { v: 5 }
    retornar c.sumar(7)
}
"#;
    assert_tw_vm_ok(src, basilisco::ValorNormalizado::Entero(12));
}

#[test]
fn s8_4_metodo_inherente_en_enum_tw_vm() {
    let src = r#"
enumeracion Opcion {
    Ninguno,
    Alguno(entero),
}
implementar Opcion {
    publico funcion valor_o(yo, defecto: entero) -> entero {
        retornar coincidir yo {
            Opcion.Ninguno => defecto,
            Opcion.Alguno(v) => v,
        }
    }
}
funcion main() -> entero {
    sea a = Opcion.Alguno(9)
    sea b = Opcion.Ninguno
    retornar a.valor_o(0) + b.valor_o(3)
}
"#;
    assert_tw_vm_ok(src, basilisco::ValorNormalizado::Entero(12));
}

#[test]
fn s8_4_trait_sobre_enum_tw_vm() {
    let src = r#"
rasgo Valorado {
    funcion puntos(yo) -> entero
}
enumeracion Color { Rojo, Verde, Azul }
implementar Valorado para Color {
    publico funcion puntos(yo) -> entero {
        retornar coincidir yo {
            Color.Rojo => 1,
            Color.Verde => 2,
            Color.Azul => 3,
        }
    }
}
funcion main() -> entero {
    sea c = Color.Verde
    retornar c.puntos()
}
"#;
    assert_tw_vm_ok(src, basilisco::ValorNormalizado::Entero(2));
}
