use basilisco::{ContextoSemantico, Lexer, Parser, backend_nativo, hir, mir, vm};
use std::fs;
use std::process::Command;

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn compilar_y_ejecutar(src_rust: &str) -> Option<String> {
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("bn-{}-{}", std::process::id(), n));
    let _ = fs::create_dir_all(&dir);
    let fuente = dir.join("gen.rs");
    let binario = dir.join("gen");
    fs::write(&fuente, src_rust).ok()?;
    let out = Command::new("rustc")
        .arg("-O")
        .arg(&fuente)
        .arg("-o")
        .arg(&binario)
        .output()
        .ok()?;
    if !out.status.success() {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&out.stderr));
        return None;
    }
    let run = Command::new(&binario).output().ok()?;
    if !run.status.success() {
        eprintln!("bin stderr:\n{}", String::from_utf8_lossy(&run.stderr));
        return None;
    }
    Some(String::from_utf8_lossy(&run.stdout).trim().to_string())
}

fn generar(src_bsk: &str) -> String {
    let tokens = Lexer::nuevo(src_bsk).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    backend_nativo::generar_rust(&mir)
}

fn vm_run(src_bsk: &str) -> String {
    let tokens = Lexer::nuevo(src_bsk).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    let idx = maquina.indice_de("main").unwrap();
    format!("{}", maquina.ejecutar_indice(idx, &[]).unwrap().unwrap())
}

#[test]
fn backend_nativo_aritmetica() {
    let src = "funcion main() -> entero { retornar 2 + 3 * 4 - 1 }";
    let rust = generar(src);
    let salida = compilar_y_ejecutar(&rust).expect("compilación nativa");
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_recursion_factorial() {
    let src = "funcion factorial(n: entero) -> entero { si n < 2 { retornar 1 } sino { retornar factorial(n - 1) * n } }\nfuncion main() -> entero { retornar factorial(7) }";
    let rust = generar(src);
    let salida = compilar_y_ejecutar(&rust).expect("compilación nativa");
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_condicional_y_loop() {
    let src = "funcion main() -> entero {\n  sea i = 0\n  sea s = 0\n  mientras i < 5 {\n    i = i + 1\n    s = s + i\n  }\n  si s > 10 { retornar s } sino { retornar 0 }\n}";
    let rust = generar(src);
    let salida = compilar_y_ejecutar(&rust).expect("compilación nativa");
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_genera_codigo_no_vacio() {
    let src = "funcion main() -> entero { retornar 42 }";
    let rust = generar(src);
    assert!(rust.contains("fn bsk_main"));
    assert!(rust.contains("fn main()"));
}
