//! Backend WASM (prototipo) de BASILISCO.
//!
//! Genera código Rust anotado para `wasm32-unknown-unknown` a partir de MIR
//! (mismo subconjunto que el backend nativo: funciones con tipos enteros).
//! Las funciones se exportan con `#[no_mangle] pub extern "C"` para poder
//! invocarlas desde un host (p. ej. Node.js) y verificar paridad observable.

use crate::backend_nativo::generar_funciones;
use crate::mir::MirPrograma;

pub fn generar_rust_wasm(programa: &MirPrograma) -> String {
    let mut out = String::new();
    out.push_str("// Generado por BASILISCO backend WASM (prototipo)\n");
    out.push_str("#![allow(unused)]\n");
    out.push_str("#![no_main]\n\n");
    // Re-exportar las funciones generadas como símbodos WASM.
    let funciones = generar_funciones(programa);
    // Anotar cada `fn bsk_` con no_mangle/extern C.
    let anotado = funciones.replace("fn bsk_", "#[no_mangle]\npub extern \"C\" fn bsk_");
    out.push_str(&anotado);
    out
}
