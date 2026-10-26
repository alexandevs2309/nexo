use std::process::{Command, Output};

fn ejecutar(bin: &str, args: &[&str]) -> Output {
    Command::new(bin)
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("no se pudo ejecutar el binario")
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

#[test]
fn version_identifica_basilisco() {
    let salida = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["--version"]);
    assert!(salida.status.success());
    assert!(texto(&salida.stdout).contains("BASILISCO"));
}

#[test]
fn alias_basilisco_funciona() {
    let salida = ejecutar(env!("CARGO_BIN_EXE_basilisco"), &["--version"]);
    assert!(salida.status.success());
    assert!(texto(&salida.stdout).contains("BASILISCO"));
}

#[test]
fn ayuda_muestra_run() {
    let salida = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["--help"]);
    assert!(salida.status.success());
    let stdout = texto(&salida.stdout);
    assert!(stdout.contains("run [<archivo.bsk>]"));
    assert!(stdout.contains(".nexo"));
}

#[test]
fn ejecuta_extension_canonica() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/hola.bsk"],
    );
    assert!(salida.status.success());
    assert_eq!(texto(&salida.stdout).trim(), "Hola BASILISCO");
}

#[test]
fn ejecuta_extension_legado() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/legado.nexo"],
    );
    assert!(salida.status.success());
    assert_eq!(texto(&salida.stdout).trim(), "42");
    assert!(texto(&salida.stderr).contains("legado"));
}

#[test]
fn rechaza_nx() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/legado.nx"],
    );
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains(".nx no está soportado"));
}

#[test]
fn rechaza_extension_desconocida() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/hola.txt"],
    );
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("no soportada"));
}

#[test]
fn rechaza_archivo_sin_extension() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/sin_extension"],
    );
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("extensión"));
}

#[test]
fn archivo_inexistente_falla() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/no-existe.bsk"],
    );
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("no se pudo leer"));
}

#[test]
fn ruta_con_espacios_funciona() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/hola con espacios.bsk"],
    );
    assert!(salida.status.success());
    assert_eq!(texto(&salida.stdout).trim(), "ruta con espacios");
}

#[test]
fn ejecuta_ejemplo_de_funciones() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "examples/funciones_basicas.bsk"],
    );
    assert!(salida.status.success());
    assert_eq!(texto(&salida.stdout), "5\n120\n42\n");
}

#[test]
fn tutorial_inicio_se_ejecuta_por_cli() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "examples/tutorial_inicio.bsk"],
    );
    assert!(salida.status.success());
    assert_eq!(texto(&salida.stdout), "12\n");
}

#[test]
fn aceptacion_v04_ejecuta_el_nucleo_integrado() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "examples/aceptacion_v04.bsk"],
    );
    assert!(salida.status.success(), "stderr: {}", texto(&salida.stderr));
    assert_eq!(texto(&salida.stdout).trim(), "total=9");
}

#[test]
fn aceptacion_v04_diagnostica_errores_con_archivo_y_ubicacion() {
    let ruta = "examples/aceptacion_error_v04.bsk";
    let salida = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["run", ruta]);
    let stderr = texto(&salida.stderr);

    assert!(!salida.status.success());
    assert!(stderr.contains(ruta), "stderr: {stderr}");
    assert!(
        stderr.contains("error de ejecución [examples/aceptacion_error_v04.bsk:2:1]"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("división entre cero"), "stderr: {stderr}");
}

#[test]
fn run_sin_archivo_es_error_de_uso() {
    let salida = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["run"]);
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("uso:"));
}
