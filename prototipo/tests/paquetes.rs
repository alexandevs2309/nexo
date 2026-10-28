use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn raiz(nombre: &str) -> PathBuf {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("paquetes-{nombre}-{}", std::process::id()));
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();
    base
}

fn escribir(base: &Path, ruta: &str, contenido: &str) {
    let destino = base.join(ruta);
    if let Some(padre) = destino.parent() {
        fs::create_dir_all(padre).unwrap();
    }
    fs::write(destino, contenido).unwrap();
}

fn ejecutar(base: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_basilisco"))
        .args(args)
        .current_dir(base)
        .output()
        .expect("no se pudo ejecutar basilisco")
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn crear_dependencia(base: &Path, nombre: &str, version: &str, codigo: &str) {
    escribir(
        base,
        &format!("paquetes/{nombre}/basilisco.toml"),
        &format!("[paquete]\nnombre = \"{nombre}\"\nversion = \"{version}\"\n"),
    );
    escribir(base, &format!("paquetes/{nombre}/lib.bsk"), codigo);
}

#[test]
fn instalar_resuelve_y_escribe_bloqueo() {
    let base = raiz("instalar");
    escribir(
        &base,
        "basilisco.toml",
        "[paquete]\nnombre = \"demo\"\nversion = \"0.1.0\"\n\n[dependencias]\nmate = \"0.1.0\"\n",
    );
    crear_dependencia(
        &base,
        "mate",
        "0.1.0",
        "exportar funcion duplicar(x: entero) -> entero {\n  retornar x * 2\n}\n",
    );

    let salida = ejecutar(&base, &["instalar"]);
    assert!(salida.status.success(), "{}", texto(&salida.stderr));
    let stdout = texto(&salida.stdout);
    assert!(stdout.contains("mate 0.1.0"), "{stdout}");

    let bloqueo = fs::read_to_string(base.join("basilisco.lock")).unwrap();
    assert!(bloqueo.contains("[[dependencia]]"), "{bloqueo}");
    assert!(bloqueo.contains("nombre = \"mate\""), "{bloqueo}");
    assert!(
        bloqueo.contains("ruta = \"paquetes/mate/lib.bsk\""),
        "{bloqueo}"
    );

    fs::remove_dir_all(&base).ok();
}

#[test]
fn instalar_falla_si_falta_dependencia() {
    let base = raiz("faltante");
    escribir(
        &base,
        "basilisco.toml",
        "[paquete]\nnombre = \"demo\"\n\n[dependencias]\nnoexiste = \"1.0.0\"\n",
    );

    let salida = ejecutar(&base, &["instalar"]);
    assert!(!salida.status.success());
    let stderr = texto(&salida.stderr);
    assert!(
        stderr.contains("no se encontró la dependencia 'noexiste'"),
        "{stderr}"
    );

    fs::remove_dir_all(&base).ok();
}

#[test]
fn instalar_rechaza_version_incompatible() {
    let base = raiz("version");
    escribir(
        &base,
        "basilisco.toml",
        "[paquete]\nnombre = \"demo\"\n\n[dependencias]\nmate = \"2.0.0\"\n",
    );
    crear_dependencia(&base, "mate", "0.1.0", "exportar dato = 1\n");

    let salida = ejecutar(&base, &["instalar"]);
    assert_eq!(salida.status.code(), Some(3));
    let stderr = texto(&salida.stderr);
    assert!(stderr.contains("versión incompatible"), "{stderr}");

    fs::remove_dir_all(&base).ok();
}

#[test]
fn ejecutar_importa_dependencia_del_bloqueo() {
    let base = raiz("ejecutar");
    escribir(
        &base,
        "basilisco.toml",
        "[paquete]\nnombre = \"demo\"\n\n[dependencias]\nmate = \"0.1.0\"\n",
    );
    crear_dependencia(
        &base,
        "mate",
        "0.1.0",
        "exportar funcion duplicar(x: entero) -> entero {\n  retornar x * 2\n}\n",
    );
    escribir(
        &base,
        "main.bsk",
        "importar \"mate\"\nmostrar duplicar(21)\n",
    );

    let instalar = ejecutar(&base, &["instalar"]);
    assert!(instalar.status.success(), "{}", texto(&instalar.stderr));

    let salida = ejecutar(&base, &["run", "main.bsk"]);
    assert!(salida.status.success(), "{}", texto(&salida.stderr));
    assert_eq!(texto(&salida.stdout).trim(), "42");

    fs::remove_dir_all(&base).ok();
}

#[test]
fn ejecutar_falla_sin_bloqueo_si_el_paquete_no_esta_en_rutas_convencionales() {
    let base = raiz("sin-bloqueo");
    escribir(&base, "main.bsk", "importar \"mate\"\nmostrar 1\n");

    let salida = ejecutar(&base, &["run", "main.bsk"]);
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("mate"));

    fs::remove_dir_all(&base).ok();
}

#[test]
fn paquetes_lista_el_bloqueo() {
    let base = raiz("listar");
    escribir(
        &base,
        "basilisco.toml",
        "[paquete]\nnombre = \"demo\"\n\n[dependencias]\nmate = \"0.1.0\"\n",
    );
    crear_dependencia(&base, "mate", "0.1.0", "exportar dato = 1\n");

    assert!(ejecutar(&base, &["instalar"]).status.success());
    let salida = ejecutar(&base, &["paquetes"]);
    assert!(salida.status.success());
    assert!(texto(&salida.stdout).contains("mate 0.1.0"));

    fs::remove_dir_all(&base).ok();
}

#[test]
fn instalar_sin_manifiesto_falla_con_mensaje_claro() {
    let base = raiz("sin-manifiesto");
    let salida = ejecutar(&base, &["instalar"]);
    assert_eq!(salida.status.code(), Some(3));
    assert!(texto(&salida.stderr).contains("basilisco.toml"));
    fs::remove_dir_all(&base).ok();
}

#[test]
fn inicializar_crea_proyecto_instalable() {
    let base = raiz("inicializar");
    let proyecto = base.join("nuevo");
    let salida = ejecutar(&base, &["inicializar", proyecto.to_str().unwrap()]);
    assert!(salida.status.success(), "{}", texto(&salida.stderr));
    assert!(proyecto.join("basilisco.toml").is_file());
    assert!(proyecto.join("paquetes").is_dir());

    let instalar = ejecutar(&proyecto, &["instalar"]);
    assert!(instalar.status.success(), "{}", texto(&instalar.stderr));
    assert!(proyecto.join("basilisco.lock").is_file());

    let listar = ejecutar(&proyecto, &["paquetes"]);
    assert!(listar.status.success());
    assert!(texto(&listar.stdout).contains("sin dependencias"));

    fs::remove_dir_all(&base).ok();
}

#[test]
fn instalar_usa_ruta_explicita_y_ejecuta() {
    let base = raiz("ruta-explicita");
    let proyecto = base.join("proyecto");
    escribir(
        &proyecto,
        "basilisco.toml",
        "[paquete]\nnombre = \"demo\"\n\n[dependencias]\nutil = { version = \"0.2.0\", ruta = \"../compartido/util\" }\n",
    );
    escribir(
        &base,
        "compartido/util/basilisco.toml",
        "[paquete]\nnombre = \"util\"\nversion = \"0.2.0\"\n",
    );
    escribir(
        &base,
        "compartido/util/lib.bsk",
        "exportar funcion saludar() -> texto {\n  retornar \"hola\"\n}\n",
    );
    escribir(
        &proyecto,
        "main.bsk",
        "importar \"util\"\nmostrar saludar()\n",
    );

    let instalar = ejecutar(&proyecto, &["instalar"]);
    assert!(instalar.status.success(), "{}", texto(&instalar.stderr));
    let bloqueo = fs::read_to_string(proyecto.join("basilisco.lock")).unwrap();
    assert!(
        bloqueo.contains("ruta = \"../compartido/util/lib.bsk\""),
        "{bloqueo}"
    );

    let salida = ejecutar(&proyecto, &["run", "main.bsk"]);
    assert!(salida.status.success(), "{}", texto(&salida.stderr));
    assert_eq!(texto(&salida.stdout).trim(), "hola");

    fs::remove_dir_all(&base).ok();
}
