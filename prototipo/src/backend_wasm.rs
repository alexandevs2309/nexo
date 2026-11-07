//! Backend WASM (prototipo) de BASILISCO.
//!
//! Genera código Rust anotado para `wasm32-unknown-unknown` a partir de MIR
//! (subconjunto puro entero/lógico: sin `String` ni E/S, porque `String` no
//! cruza el ABI C de los exports WASM). Las funciones se exportan con
//! `#[no_mangle] pub extern "C"` para poder invocarlas desde un host
//! (p. ej. Node.js) y verificar paridad observable.

use crate::backend_nativo::generar_funciones_enteras;
use crate::mir::MirPrograma;

pub fn generar_rust_wasm(programa: &MirPrograma) -> String {
    let mut out = String::new();
    out.push_str("// Generado por BASILISCO backend WASM (prototipo)\n");
    out.push_str("#![allow(unused)]\n");
    out.push_str("#![no_main]\n\n");
    // Re-exportar las funciones generadas como símbodos WASM.
    let funciones = generar_funciones_enteras(programa);
    // Anotar cada `fn bsk_` con no_mangle/extern C.
    let anotado = funciones.replace("fn bsk_", "#[no_mangle]\npub extern \"C\" fn bsk_");
    out.push_str(&anotado);
    out
}
