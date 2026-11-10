//! Backend WASM (prototipo, subconjunto entero/lógico) de BASILISCO.
//!
//! Genera código Rust anotado para `wasm32-unknown-unknown` a partir de MIR
//! (subconjunto puro entero/lógico: sin `String` ni E/S, porque `String` no
//! cruza el ABI C de los exports WASM). Las funciones se exportan con
//! `#[unsafe(no_mangle)] pub extern "C"` (forma requerida por rustc/edition 2024
//! para atributos `no_mangle`) para poder invocarlas desde un host (p. ej.
//! Node.js) y verificar paridad observable. No genera `.wasm`/WAT directamente:
//! la compilación a WebAssembly la realiza `rustc`.

use crate::backend_entero::generar_funciones_enteras;
use crate::mir::MirPrograma;

pub fn generar_rust_wasm(programa: &MirPrograma) -> String {
    let mut out = String::new();
    out.push_str("// Generado por BASILISCO backend WASM (prototipo)\n");
    out.push_str("#![allow(unused)]\n");
    out.push_str("#![no_main]\n\n");
    // Re-exportar las funciones generadas como símbolos WASM.
    let funciones = generar_funciones_enteras(programa);
    // Anotar cada `fn bsk_` con no_mangle/extern C (unsafe attribute, ed. 2024).
    let anotado = funciones.replace("fn bsk_", "#[unsafe(no_mangle)]\npub extern \"C\" fn bsk_");
    out.push_str(&anotado);
    out
}
