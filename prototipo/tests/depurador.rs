use std::process::Command;

#[test]
fn depurar_imprime_traza() {
    let dir = std::env::temp_dir().join(format!("bsk_dbg_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("m.bsk"),
        "funcion sumar(a, b) { retornar a + b }\nfuncion main() { mostrar sumar(2, 3) }\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_basilisco"))
        .args(["depurar", dir.join("m.bsk").to_str().unwrap()])
        .output()
        .expect("ejecutar");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("→ sumar(2, 3)"), "stdout: {stdout}");
    assert!(stdout.contains("← sumar = 5"), "stdout: {stdout}");
    assert!(stdout.contains("→ main()"), "stdout: {stdout}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn depurar_filtro_punto_funcion() {
    let dir = std::env::temp_dir().join(format!("bsk_dbg2_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("m.bsk"),
        "funcion sumar(a, b) { retornar a + b }\nfuncion main() { mostrar sumar(2, 3) }\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_basilisco"))
        .args([
            "depurar",
            dir.join("m.bsk").to_str().unwrap(),
            "--punto-funcion=sumar",
        ])
        .output()
        .expect("ejecutar");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("→ sumar(2, 3)"), "stdout: {stdout}");
    assert!(!stdout.contains("→ main()"), "stdout: {stdout}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn depurar_error_reporta() {
    let dir = std::env::temp_dir().join(format!("bsk_dbg3_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("m.bsk"), "funcion main() { mostrar 1 / 0 }\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_basilisco"))
        .args(["depurar", dir.join("m.bsk").to_str().unwrap()])
        .output()
        .expect("ejecutar");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error:"), "stderr: {stderr}");
    std::fs::remove_dir_all(&dir).ok();
}
