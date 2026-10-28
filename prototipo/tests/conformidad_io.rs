use basilisco::{Entorno, ErrorEjecucion, Lexer, Parser, Valor};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn ejecutar(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida");
    Entorno::nueva().ejecutar_programa(&sentencias)
}

fn dir_temporal() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let ruta = std::env::temp_dir().join(format!("basilisco-io-{nanos}"));
    fs::create_dir_all(&ruta).unwrap();
    ruta
}

#[test]
fn archivo_escribir_y_leer() {
    let dir = dir_temporal();
    let ruta = dir.join("prueba.txt");
    let ruta_str = ruta.to_string_lossy().replace('\\', "/");
    let fuente = format!(
        "sea f = abrir(\"{ruta}\", \"escribir\")\nescribir_linea(f, \"hola\")\ncerrar(f)\nsea g = abrir(\"{ruta}\", \"leer\")\nsea linea = leer_linea(g)\ncerrar(g)\nlinea",
        ruta = ruta_str
    );
    let v = ejecutar(&fuente).expect("sin error").expect("valor");
    assert_eq!(v, Valor::Texto("hola".into()));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn abrir_inexistente_es_error() {
    let dir = dir_temporal();
    let ruta = dir.join("no_existe.txt");
    let ruta_str = ruta.to_string_lossy().replace('\\', "/");
    let fuente = format!("abrir(\"{ruta}\", \"leer\")", ruta = ruta_str);
    assert!(ejecutar(&fuente).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cerrar_y_usar_es_error() {
    let dir = dir_temporal();
    let ruta = dir.join("prueba2.txt");
    let ruta_str = ruta.to_string_lossy().replace('\\', "/");
    let fuente = format!(
        "sea f = abrir(\"{ruta}\", \"escribir\")\ncerrar(f)\nleer_linea(f)",
        ruta = ruta_str
    );
    assert!(ejecutar(&fuente).is_err());
    let _ = fs::remove_dir_all(&dir);
}
