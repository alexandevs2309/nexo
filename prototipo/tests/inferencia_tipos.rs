//! Pruebas de regresión para inferencia de tipos (Etapa A, ADR-0017).
//! Documentan el comportamiento actual sin modificar producción.

use std::fs;
use std::process::Command;

fn run_bsk(src: &str) -> (i32, String, String) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static C: AtomicU64 = AtomicU64::new(0);
    let n = C.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("bsk_inf_test_{}", n));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("t.bsk");
    fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect("ejecutar bsk");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (code, stdout, stderr)
}

#[test]
fn primera_llamada_entero_segunda_entero_ok() {
    let (code, _, stderr) = run_bsk("funcion f(x) { retornar x }\nmostrar(f(1))\nmostrar(f(2))\n");
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn primera_llamada_entero_segunda_texto_falla() {
    let (code, _, stderr) =
        run_bsk("funcion f(x) { retornar x }\nmostrar(f(1))\nmostrar(f(\"a\"))\n");
    assert_ne!(code, 0, "debe fallar en runtime");
    assert!(
        stderr.contains("incompatible") || stderr.contains("E1"),
        "stderr: {stderr}"
    );
}

#[test]
fn funcion_anotada_argumento_incompatible_falla() {
    let (code, _, _) = run_bsk("funcion f(x: entero) { retornar x }\nmostrar(f(\"a\"))\n");
    assert_ne!(code, 0);
}

#[test]
fn funcion_sin_retorno_como_sentencia_ok() {
    let (code, _, stderr) = run_bsk("funcion f() { sea x = 1 }\nf()\nmostrar(\"ok\")\n");
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn funcion_sin_retorno_como_expresion_falla_e1010() {
    let (code, _, stderr) = run_bsk("funcion f() { sea x = 1 }\nmostrar(f())\n");
    assert_ne!(code, 0);
    assert!(
        stderr.contains("E1010") || stderr.contains("retorno"),
        "stderr: {stderr}"
    );
}

#[test]
fn recursividad_simple() {
    let (code, _, stderr) = run_bsk(
        "funcion fact(n) {\n  si n <= 1 {\n    retornar 1\n  }\n  retornar n * fact(n - 1)\n}\nmostrar(fact(5))\n",
    );
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn lambda_parametro_basico() {
    let (code, _, stderr) = run_bsk("sea f = funcion(x) { retornar x }\nmostrar(f(7))\n");
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn funcion_nunca_ejecutada_no_valida() {
    // Documenta: sin llamadas, no hay validación de argumentos.
    let (code, _, stderr) = run_bsk("funcion f(x: entero) { retornar x }\nmostrar(\"ok\")\n");
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn recursividad_mutua() {
    let (code, _, stderr) = run_bsk(
        "funcion es_par(n) {\n  si n == 0 { retornar verdadero }\n  retornar es_impar(n - 1)\n}\nfuncion es_impar(n) {\n  si n == 0 { retornar falso }\n  retornar es_par(n - 1)\n}\nmostrar(es_par(4))\nmostrar(es_impar(3))\n",
    );
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn primera_clase_asignacion_e_invocacion_indirecta() {
    let (code, _, stderr) =
        run_bsk("funcion doble(x) { retornar x * 2 }\nsea f = doble\nmostrar(f(21))\n");
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn primera_clase_paso_como_argumento() {
    let (code, _, stderr) = run_bsk(
        "funcion aplicar(g, v) { retornar g(v) }\nfuncion inc(x) { retornar x + 1 }\nmostrar(aplicar(inc, 5))\n",
    );
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn closure_captura_variable() {
    let (code, _, stderr) = run_bsk(
        "funcion crear(x) { retornar funcion (yy) { retornar x + yy } }\nsea c = crear(10)\nmostrar(c(5))\nmostrar(c(7))\n",
    );
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn closure_parametros_tipos_distintos_documenta() {
    let (code, _, _) = run_bsk(
        "funcion crear(x) { retornar funcion (y) { retornar y } }\nsea c = crear(0)\nmostrar(c(1))\nmostrar(c(\"a\"))\n",
    );
    let _ = code;
}

#[test]
fn funcion_nunca_ejecutada_anotada_no_valida() {
    let (code, _, stderr) = run_bsk("funcion f(x: entero) { retornar x }\nmostrar(\"ok\")\n");
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn funcion_nunca_ejecutada_sin_anotacion_no_valida() {
    let (code, _, stderr) = run_bsk("funcion f(x) { retornar x }\nmostrar(\"ok\")\n");
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn analizar_vs_run_aceptado_por_ambos() {
    let src = "funcion f(x: entero) { retornar x }\nmostrar(f(2))\n";
    let dir = std::env::temp_dir().join("bsk_avr");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("t.bsk");
    fs::write(&path, src).unwrap();
    let analizar = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["analizar", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(analizar.status.success());
    let run = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(run.status.success());
}

#[test]
fn analizar_acepta_run_falla() {
    let src = "funcion f(x) { retornar x }\nmostrar(f(1))\nmostrar(f(\"a\"))\n";
    let dir = std::env::temp_dir().join("bsk_avr2");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("t.bsk");
    fs::write(&path, src).unwrap();
    let analizar = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["analizar", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(analizar.status.success(), "analizar debe aceptar");
    let run = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!run.status.success(), "run debe fallar");
}

#[test]
fn analizar_rechaza_incompatibilidad_explicita() {
    let src = "funcion f(x: entero) { retornar x }\nmostrar(f(\"a\"))\n";
    let dir = std::env::temp_dir().join("bsk_avr3");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("t.bsk");
    fs::write(&path, src).unwrap();
    let analizar = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["analizar", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!analizar.status.success(), "analizar debe rechazar");
}
