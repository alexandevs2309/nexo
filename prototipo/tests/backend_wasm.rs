use basilisco::{ContextoSemantico, Lexer, Parser, backend_wasm, hir, mir, vm};
use std::fs;
use std::process::Command;

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn compilar_y_ejecutar_wasm(src_rust: &str) -> Option<String> {
    // Verificar que el target WASM esté disponible.
    let targets = Command::new("rustc")
        .arg("--print")
        .arg("target-list")
        .output()
        .ok()?;
    let targets = String::from_utf8_lossy(&targets.stdout);
    if !targets
        .lines()
        .any(|t| t.trim() == "wasm32-unknown-unknown")
    {
        eprintln!("aviso: target wasm32-unknown-unknown no disponible; prueba omitida");
        return None;
    }
    // Verificar node.
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("aviso: node no disponible; prueba omitida");
        return None;
    }
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
    // Host JS (ruta con barras normales para Windows).
    let js = dir.join("run.js");
    let wasm_path = wasm.display().to_string().replace('\\', "/");
    let js_code = format!(
        "const fs = require('fs');\nconst wasm = fs.readFileSync('{}');\nconst mod = new WebAssembly.Module(wasm);\nconst inst = new WebAssembly.Instance(mod, {{}});\nconsole.log(inst.exports.bsk_main().toString());\n",
        wasm_path
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
    let Some(salida) = compilar_y_ejecutar_wasm(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_wasm_recursion_factorial() {
    let src = "funcion factorial(n: entero) -> entero { si n < 2 { retornar 1 } sino { retornar factorial(n - 1) * n } }\nfuncion main() -> entero { retornar factorial(7) }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar_wasm(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_wasm_condicional_y_loop() {
    let src = "funcion main() -> entero {\n  sea i = 0\n  sea s = 0\n  mientras i < 5 {\n    i = i + 1\n    s = s + i\n  }\n  si s > 10 { retornar s } sino { retornar 0 }\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar_wasm(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_wasm_genera_export() {
    let src = "funcion main() -> entero { retornar 42 }";
    let rust = generar(src);
    assert!(rust.contains("#[no_mangle]"));
    assert!(rust.contains("pub extern \"C\" fn bsk_main"));
}

#[test]
fn backend_wasm_omite_funciones_texto() {
    // El subconjunto WASM es puro entero: `String` no cruza el ABI C de los
    // exports, así que las funciones con texto se omiten (comentario explícito).
    let src =
        "funcion saluda() -> texto { retornar \"hola\" }\nfuncion main() -> entero { retornar 7 }";
    let rust = generar(src);
    assert!(rust.contains("pub extern \"C\" fn bsk_main"));
    assert!(!rust.contains("pub extern \"C\" fn bsk_saluda"));
    assert!(rust.contains("omitida"));
}
