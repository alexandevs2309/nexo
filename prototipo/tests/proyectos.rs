use std::fs;
use std::process::Command;

fn tmpdir(nombre: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join("bsk_proj_tests").join(nombre);
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    base
}

fn run_bsk(dir: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_bsk");
    let out = Command::new(bin)
        .args(args)
        .current_dir(dir)
        .output()
        .expect("no se pudo ejecutar");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (code, stdout, stderr)
}

// 1. Creación correcta de un proyecto.
#[test]
fn proyecto_crea_estructura() {
    let base = tmpdir("crea");
    let (code, stdout, stderr) = run_bsk(&base, &["nuevo", "proyecto", "demo"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("Proyecto creado"));
    let proj = base.join("demo");
    assert!(proj.join("basilisco.toml").exists());
    assert!(proj.join("src/main.bsk").exists());
    assert!(proj.join("src/lib.bsk").exists());
    assert!(proj.join("README.md").exists());
    assert!(proj.join(".gitignore").exists());
}

// 2. Nombre inválido.
#[test]
fn proyecto_nombre_invalido() {
    let base = tmpdir("nombre_inv");
    let (code, _stdout, stderr) = run_bsk(&base, &["nuevo", "proyecto", "123bad"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("inválido") || stderr.contains("nombre"),
        "stderr: {stderr}"
    );
}

// 3. Destino ya existente, sin sobrescritura.
#[test]
fn proyecto_no_sobrescribe() {
    let base = tmpdir("no_sobrescribe");
    fs::create_dir_all(base.join("demo")).unwrap();
    let (code, _stdout, stderr) = run_bsk(&base, &["nuevo", "proyecto", "demo"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("ya existe") || stderr.contains("existe"),
        "stderr: {stderr}"
    );
}

// 4. Manifiesto válido.
#[test]
fn proyecto_manifiesto_valido() {
    let base = tmpdir("manif_valido");
    run_bsk(&base, &["nuevo", "proyecto", "demo"]);
    let proj = base.join("demo");
    let contenido = fs::read_to_string(proj.join("basilisco.toml")).unwrap();
    assert!(contenido.contains("nombre = \"demo\""));
    assert!(contenido.contains("version = \"0.1.0\""));
    assert!(contenido.contains("entrada = \"src/main.bsk\""));
}

// 5. Manifiesto inválido — campo obligatorio ausente.
#[test]
fn proyecto_manifiesto_sin_nombre() {
    let base = tmpdir("sin_nombre");
    let proj = base.join("demo");
    fs::create_dir_all(proj.join("src")).unwrap();
    fs::write(
        proj.join("basilisco.toml"),
        "version = \"0.1.0\"\nentrada = \"src/main.bsk\"\n",
    )
    .unwrap();
    fs::write(proj.join("src/main.bsk"), "mostrar 1\n").unwrap();
    let (code, _stdout, stderr) = run_bsk(&proj, &["run"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("nombre") || stderr.contains("falta"),
        "stderr: {stderr}"
    );
}

// 6. Manifiesto con clave duplicada.
#[test]
fn proyecto_manifiesto_clave_duplicada() {
    let base = tmpdir("clave_dup");
    let proj = base.join("demo");
    fs::create_dir_all(proj.join("src")).unwrap();
    fs::write(
        proj.join("basilisco.toml"),
        "nombre = \"demo\"\nnombre = \"otro\"\nversion = \"0.1.0\"\nentrada = \"src/main.bsk\"\n",
    )
    .unwrap();
    fs::write(proj.join("src/main.bsk"), "mostrar 1\n").unwrap();
    let (code, _stdout, stderr) = run_bsk(&proj, &["run"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("duplicada") || stderr.contains("duplicado"),
        "stderr: {stderr}"
    );
}

// 7. Entrada inexistente.
#[test]
fn proyecto_entrada_inexistente() {
    let base = tmpdir("entrada_inex");
    let proj = base.join("demo");
    fs::create_dir_all(proj.join("src")).unwrap();
    fs::write(
        proj.join("basilisco.toml"),
        "nombre = \"demo\"\nversion = \"0.1.0\"\nentrada = \"src/noexiste.bsk\"\n",
    )
    .unwrap();
    let (code, _stdout, stderr) = run_bsk(&proj, &["run"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("no existe") || stderr.contains("entrada"),
        "stderr: {stderr}"
    );
}

// 8. Ejecución del archivo de entrada configurado.
#[test]
fn proyecto_ejecuta_entrada() {
    let base = tmpdir("ejecuta");
    run_bsk(&base, &["nuevo", "proyecto", "demo"]);
    let proj = base.join("demo");
    let (code, stdout, stderr) = run_bsk(&proj, &["run"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "Hola, BASILISCO!");
}

// 9. Importación local dentro del proyecto.
#[test]
fn proyecto_importacion_local() {
    let base = tmpdir("import_local");
    run_bsk(&base, &["nuevo", "proyecto", "demo"]);
    let proj = base.join("demo");
    fs::write(
        proj.join("src/lib.bsk"),
        "publico funcion doble(n) { retornar n * 2 }\n",
    )
    .unwrap();
    fs::write(
        proj.join("src/main.bsk"),
        "usar \"lib.bsk\" como lib\nmostrar lib.doble(7)\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run_bsk(&proj, &["run"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "14");
}

// 10. Intento de escapar de la raíz.
#[test]
fn proyecto_escape_raiz() {
    let base = tmpdir("escape_raiz");
    run_bsk(&base, &["nuevo", "proyecto", "demo"]);
    let proj = base.join("demo");
    // Crear archivo fuera del proyecto.
    fs::write(base.join("secreto.bsk"), "publico sea X = 1\n").unwrap();
    fs::write(
        proj.join("src/main.bsk"),
        "usar \"../../secreto.bsk\" como lib\nmostrar lib.X\n",
    )
    .unwrap();
    let (code, _stdout, stderr) = run_bsk(&proj, &["run"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("ruta no permitida")
            || stderr.contains("no permitida")
            || stderr.contains("módulo no encontrado"),
        "stderr: {stderr}"
    );
}

// 11. Ejecución de archivo individual sigue funcionando.
#[test]
fn proyecto_compatibilidad_archivo() {
    let base = tmpdir("compat");
    fs::write(base.join("solo.bsk"), "mostrar 42\n").unwrap();
    let (code, stdout, stderr) = run_bsk(&base, &["run", "solo.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "42");
}

// 12. Ausencia de manifiesto → código de salida 2.
#[test]
fn proyecto_sin_manifiesto() {
    let base = tmpdir("sin_manif");
    let (code, _stdout, stderr) = run_bsk(&base, &["run"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("manifiesto")
            || stderr.contains("basilisco.toml")
            || stderr.contains("error"),
        "stderr: {stderr}"
    );
}

// 13. Rutas con espacios.
#[test]
fn proyecto_ruta_con_espacios() {
    let base = tmpdir("espacios");
    let (code, stdout, stderr) = run_bsk(&base, &["nuevo", "proyecto", "mi proyecto"]);
    // El nombre con espacio debería ser rechazado (no es identificador válido).
    assert_ne!(code, 0, "nombre con espacio debería ser rechazado");
    let _ = (stdout, stderr);
}

// 14. Versión inválida en manifiesto.
#[test]
fn proyecto_version_invalida() {
    let base = tmpdir("version_inv");
    let proj = base.join("demo");
    fs::create_dir_all(proj.join("src")).unwrap();
    fs::write(
        proj.join("basilisco.toml"),
        "nombre = \"demo\"\nversion = \"1.0\"\nentrada = \"src/main.bsk\"\n",
    )
    .unwrap();
    fs::write(proj.join("src/main.bsk"), "mostrar 1\n").unwrap();
    let (code, _stdout, stderr) = run_bsk(&proj, &["run"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("versión") || stderr.contains("invalida") || stderr.contains("inválida"),
        "stderr: {stderr}"
    );
}
