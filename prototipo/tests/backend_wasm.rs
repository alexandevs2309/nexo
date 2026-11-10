//! Pruebas del backend WASM (subconjunto entero/lógico) — Fase 01.
//!
//! Verifican: generación de código intermedio Rust, compilación real a
//! `wasm32-unknown-unknown` (genera `.wasm`), validez del módulo, y ejecución
//! real en Node.js vía `WebAssembly.Module`/`WebAssembly.Instance`.
//!
//! ## Dependencias para ejecución real (CI/entorno reproducible)
//! - `rustc` (probado con rustc 1.99.1, edition 2024) con target
//!   `wasm32-unknown-unknown` instalado (`rustup target add wasm32-unknown-unknown`).
//! - `node` (probado con v24.x) disponible en `PATH`.
//!
//! Política: **sin skips silenciosos**. Si falta una dependencia requerida, la
//! prueba falla explícitamente como `BLOQUEADA POR DEPENDENCIA` (no pasa).
//! Si la compilación/ejecución falla, la prueba falla.

use basilisco::{ContextoSemantico, Lexer, Parser, backend_wasm, hir, mir, vm};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

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

/// Verifica dependencias requeridas. Falla explícitamente (no skip silencioso)
/// si falta alguna: clasificado como bloqueado por dependencia.
fn exigir_dependencias_wasm() {
    if Command::new("rustc").arg("--version").output().is_err() {
        panic!("BLOQUEADA POR DEPENDENCIA: rustc no disponible en PATH");
    }
    if !wasm32_disponible() {
        panic!(
            "BLOQUEADA POR DEPENDENCIA: target wasm32-unknown-unknown no instalado \
             (ejecutar: rustup target add wasm32-unknown-unknown)"
        );
    }
    if !node_disponible() {
        panic!("BLOQUEADA POR DEPENDENCIA: node no disponible en PATH");
    }
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

fn nuevo_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("wasm-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).expect("no se pudo crear directorio temporal");
    dir
}

/// Compila Rust generado a `wasm32-unknown-unknown`. Falla explícitamente si
/// `rustc` falla (propagación de error de compilación).
fn compilar_a_wasm(src_rust: &str, dir: &Path) -> PathBuf {
    let fuente = dir.join("gen.rs");
    let wasm = dir.join("gen.wasm");
    fs::write(&fuente, src_rust).expect("no se pudo escribir gen.rs");
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
        .expect("no se pudo ejecutar rustc");
    assert!(
        out.status.success(),
        "compilación a wasm32 falló (debe ser error de prueba, no skip):\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        wasm.exists(),
        "rustc reportó éxito pero no se generó el archivo .wasm: {}",
        wasm.display()
    );
    wasm
}

/// Valida el módulo WASM con `WebAssembly.validate` (Node). Falla si inválido.
fn validar_wasm_node(wasm: &Path) {
    let js = wasm
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("validate.js");
    let wasm_path = wasm.display().to_string().replace('\\', "/");
    let js_code = format!(
        "const fs=require('fs');\nconst b=fs.readFileSync('{}');\nif(!WebAssembly.validate(b)){{console.error('INVALID');process.exit(2);}}\nconsole.log('VALID');\n",
        wasm_path
    );
    fs::write(&js, js_code).expect("no se pudo escribir validate.js");
    let run = Command::new("node")
        .arg(&js)
        .output()
        .expect("no se pudo ejecutar node");
    assert!(
        run.status.success(),
        "WebAssembly.validate rechazó el módulo .wasm:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.trim().contains("VALID"),
        "validación inesperada: {}",
        stdout
    );
}

/// Ejecuta el export `bsk_main` vía `WebAssembly.Module`/`Instance` (Node).
/// Falla explícitamente si la ejecución falla (propagación de error).
fn ejecutar_wasm_node(wasm: &Path) -> String {
    let js = wasm
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("run.js");
    let wasm_path = wasm.display().to_string().replace('\\', "/");
    let js_code = format!(
        "const fs=require('fs');\nconst b=fs.readFileSync('{}');\nconst mod=new WebAssembly.Module(b);\nconst inst=new WebAssembly.Instance(mod,{{}});\nconst r=inst.exports.bsk_main();\nconsole.log(r.toString());\n",
        wasm_path
    );
    fs::write(&js, js_code).expect("no se pudo escribir run.js");
    let run = Command::new("node")
        .arg(&js)
        .output()
        .expect("no se pudo ejecutar node");
    assert!(
        run.status.success(),
        "ejecución WASM en Node falló (propagada como error de prueba):\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

/// Pipeline completo real: compila a `.wasm`, valida, ejecuta en Node.
fn compilar_validar_ejecutar(src_rust: &str) -> String {
    let dir = nuevo_dir();
    let wasm = compilar_a_wasm(src_rust, &dir);
    validar_wasm_node(&wasm);
    ejecutar_wasm_node(&wasm)
}

fn assert_ejecucion_real(src_bsk: &str) {
    exigir_dependencias_wasm();
    let rust = generar(src_bsk);
    let salida = compilar_validar_ejecutar(&rust);
    assert_eq!(
        salida,
        vm_run(src_bsk),
        "paridad WASM vs VM para: {src_bsk}"
    );
}

// --- Cobertura existente (ejecución real, sin skips silenciosos) ---

#[test]
fn backend_wasm_aritmetica() {
    let src = "funcion main() -> entero { retornar 2 + 3 * 4 - 1 }";
    assert_ejecucion_real(src);
}

#[test]
fn backend_wasm_recursion_factorial() {
    let src = "funcion factorial(n: entero) -> entero { si n < 2 { retornar 1 } sino { retornar factorial(n - 1) * n } }\nfuncion main() -> entero { retornar factorial(7) }";
    assert_ejecucion_real(src);
}

#[test]
fn backend_wasm_condicional_y_loop() {
    let src = "funcion main() -> entero {\n  sea i = 0\n  sea s = 0\n  mientras i < 5 {\n    i = i + 1\n    s = s + i\n  }\n  si s > 10 { retornar s } sino { retornar 0 }\n}";
    assert_ejecucion_real(src);
}

#[test]
fn backend_wasm_genera_export() {
    // Verifica código intermedio (atributo/export ABI).
    let src = "funcion main() -> entero { retornar 42 }";
    let rust = generar(src);
    assert!(
        rust.contains("#[unsafe(no_mangle)]"),
        "debe usar #[unsafe(no_mangle)] (edition 2024/toolchain soportado)"
    );
    assert!(rust.contains("pub extern \"C\" fn bsk_main"));
}

#[test]
fn backend_wasm_omite_funciones_texto() {
    // Subconjunto WASM entero: `String` no cruza ABI C de exports, se omite.
    let src =
        "funcion saluda() -> texto { retornar \"hola\" }\nfuncion main() -> entero { retornar 7 }";
    let rust = generar(src);
    assert!(rust.contains("pub extern \"C\" fn bsk_main"));
    assert!(!rust.contains("pub extern \"C\" fn bsk_saluda"));
    assert!(rust.contains("omitida"));
}

// --- Verificaciones adicionales Fase 01 (evidencia real) ---

#[test]
fn backend_wasm_compila_genera_archivo_wasm_y_es_valido() {
    exigir_dependencias_wasm();
    let src = "funcion main() -> entero { retornar 55 }";
    let rust = generar(src);
    let dir = nuevo_dir();
    let wasm = compilar_a_wasm(&rust, &dir);
    let meta = fs::metadata(&wasm).expect(".wasm debe existir");
    assert!(meta.len() > 0, ".wasm no debe estar vacío");
    // Cabecera mágica WASM: \0asm
    let bytes = fs::read(&wasm).expect("leer .wasm");
    assert!(
        bytes.starts_with(b"\0asm"),
        "archivo generado no tiene cabecera WASM válida"
    );
    validar_wasm_node(&wasm);
}

#[test]
fn backend_wasm_ejecucion_real_node_module_instance() {
    exigir_dependencias_wasm();
    let src = "funcion main() -> entero {\n  sea i = 0\n  sea s = 0\n  mientras i < 10 { i = i + 1\n    s = s + i }\n  retornar s\n}";
    let rust = generar(src);
    let dir = nuevo_dir();
    let wasm = compilar_a_wasm(&rust, &dir);
    validar_wasm_node(&wasm);
    let salida = ejecutar_wasm_node(&wasm);
    assert_eq!(salida, "55");
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_wasm_resultados_correctos_repetible() {
    exigir_dependencias_wasm();
    let src = "funcion main() -> entero { retornar 2 + 3 * 4 - 1 }";
    let esperado = vm_run(src);
    for _ in 0..3 {
        let rust = generar(src);
        let salida = compilar_validar_ejecutar(&rust);
        assert_eq!(salida, esperado);
    }
}

#[test]
fn backend_wasm_error_compilacion_propagado() {
    // Una compilación fallida debe producir error explícito (panic/assert),
    // nunca un resultado "ok" silencioso.
    exigir_dependencias_wasm();
    let dir = nuevo_dir();
    let fuente = dir.join("gen.rs");
    let wasm = dir.join("gen.wasm");
    fs::write(&fuente, "esto no es rust valido {{{").unwrap();
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
        .expect("ejecutar rustc");
    assert!(
        !out.status.success(),
        "se esperaba fallo de compilación para entrada inválida"
    );
    assert!(
        !wasm.exists(),
        "no debe generarse .wasm ante fallo de compilación"
    );
    // Verifica que nuestro helper propaga (no silencia) el fallo.
    let panico = std::panic::catch_unwind(|| {
        compilar_a_wasm("esto no es rust valido {{{", &dir);
    });
    assert!(
        panico.is_err(),
        "compilar_a_wasm debe propagar fallo de compilación como error (no skip)"
    );
}

#[test]
fn backend_wasm_error_ejecucion_node_propagado() {
    exigir_dependencias_wasm();
    let dir = nuevo_dir();
    let wasm = dir.join("no_existe.wasm");
    let panico = std::panic::catch_unwind(|| {
        let _ = ejecutar_wasm_node(&wasm);
    });
    assert!(
        panico.is_err(),
        "ejecutar_wasm_node debe propagar fallo de ejecución como error"
    );
}

#[test]
fn backend_wasm_codigo_intermedio_contiene_export_valido() {
    let src = "funcion sumar(a: entero, b: entero) -> entero { retornar a + b }\nfuncion main() -> entero { retornar sumar(2, 3) }";
    let rust = generar(src);
    assert!(rust.contains("#![no_main]"));
    assert!(rust.contains("#[unsafe(no_mangle)]"));
    assert!(rust.contains("pub extern \"C\" fn bsk_main"));
    assert!(rust.contains("pub extern \"C\" fn bsk_sumar"));
}
