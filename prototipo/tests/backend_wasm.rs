use basilisco::{ContextoSemantico, Lexer, Parser, backend_wasm, hir, mir, vm};
use std::fs;
use std::process::Command;

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn compilar_y_ejecutar_wasm(src_rust: &str) -> Option<String> {
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("wasm-{}-{}", std::process::id(), n));
    let _ = fs::create_dir_all(&dir);
    let fuente = dir.join("gen.rs");
    let wasm = dir.join("gen.wasm");
    fs::write(&fuente, src_rust).ok()?;
    let out = Command::new("rustc")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("-O")
        .arg(&fuente)
        .arg("-o")
        .arg(&wasm)
        .output()
        .ok()?;
    if !out.status.success() {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&out.stderr));
        return None;
    }
    // Host JS
    let js = dir.join("run.js");
    let js_code = format!(
        r#"const fs = require('fs');
const wasm = fs.readFileSync('{}');
const mod = new WebAssembly.Module(wasm);
const inst = new WebAssembly.Instance(mod, {{}});
console.log(inst.exports.bsk_main().toString());
"#,
        wasm.display()
    );
    fs::write(&js, js_code).ok()?;
    let run = Command::new("node").arg(&js).output().ok()?;
    if !run.status.success() {
        eprintln!("node stderr:\n{}", String::from_utf8_lossy(&run.stderr));
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
    backend_wasm::generar_rust_wasm(&mir)
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
fn backend_wasm_aritmetica() {
    let src = "funcion main() -> entero { retornar 2 + 3 * 4 - 1 }";
    let rust = generar(src);
    let salida = compilar_y_ejecutar_wasm(&rust).expect("compilación wasm");
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_wasm_recursion_factorial() {
    let src = "funcion factorial(n: entero) -> entero { si n < 2 { retornar 1 } sino { retornar factorial(n - 1) * n } }\nfuncion main() -> entero { retornar factorial(7) }";
    let rust = generar(src);
    let salida = compilar_y_ejecutar_wasm(&rust).expect("compilación wasm");
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_wasm_condicional_y_loop() {
    let src = "funcion main() -> entero {\n  sea i = 0\n  sea s = 0\n  mientras i < 5 {\n    i = i + 1\n    s = s + i\n  }\n  si s > 10 { retornar s } sino { retornar 0 }\n}";
    let rust = generar(src);
    let salida = compilar_y_ejecutar_wasm(&rust).expect("compilación wasm");
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_wasm_genera_export() {
    let src = "funcion main() -> entero { retornar 42 }";
    let rust = generar(src);
    assert!(rust.contains("#[no_mangle]"));
    assert!(rust.contains("pub extern \"C\" fn bsk_main"));
}
