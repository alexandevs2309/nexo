use basilisco::cargador::CargadorModulos;
use basilisco::{ContextoSemantico, Lexer, Parser};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tid = std::thread::current().id();
    let dir = std::env::temp_dir().join(format!("basilisco-semmod-{n}-{seq}-{tid:?}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn escribir(dir: &std::path::Path, nombre: &str, contenido: &str) {
    fs::write(dir.join(nombre), contenido).unwrap();
}

fn analizar_archivo(root: &std::path::Path, archivo: &str) -> Vec<basilisco::ErrorEjecucion> {
    let ruta = root.join(archivo);
    let fuente = fs::read_to_string(&ruta).unwrap();
    let tokens = Lexer::nuevo(&fuente).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let cargador = Arc::new(Mutex::new(CargadorModulos::nuevo(root.to_path_buf())));
    let mut ctx = ContextoSemantico::con_cargador(cargador, Some(ruta.display().to_string()));
    ctx.analizar_programa(&ss)
}

#[test]
fn mod_import_publico_valido() {
    let dir = temp_dir();
    escribir(
        &dir,
        "lib.bsk",
        "publico sea valor = 5\npublico funcion suma(a, b) { retornar a + b }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" { valor, suma }\nsuma(valor, 1)\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mod_miembro_inexistente() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "publico sea valor = 5\n");
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" { noexiste }\n1\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(!e.is_empty(), "debe fallar");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mod_miembro_privado() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "sea secreto = 5\n");
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" { secreto }\n1\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(!e.is_empty(), "debe fallar");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mod_alias() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "publico sea valor = 5\n");
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" como lib\nlib\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mod_lista_miembros() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "publico sea a = 1\npublico sea b = 2\n");
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" { a, b }\na + b\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mod_transitiva() {
    let dir = temp_dir();
    escribir(&dir, "a.bsk", "publico funcion uno() { retornar 1 }\n");
    escribir(
        &dir,
        "b.bsk",
        "usar \"a.bsk\" { uno }\npublico funcion dos() { retornar uno() + 1 }\n",
    );
    escribir(&dir, "main.bsk", "usar \"b.bsk\" { dos }\ndos()\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mod_ciclo_directo() {
    let dir = temp_dir();
    escribir(&dir, "a.bsk", "usar \"b.bsk\" como b\npublico sea x = 1\n");
    escribir(&dir, "b.bsk", "usar \"a.bsk\" como a\npublico sea y = 2\n");
    escribir(&dir, "main.bsk", "usar \"a.bsk\" como a\n1\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(!e.is_empty(), "debe detectar ciclo");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mod_modulo_inexistente() {
    let dir = temp_dir();
    escribir(&dir, "main.bsk", "usar \"noexiste.bsk\" como lib\n1\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(!e.is_empty(), "debe fallar");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mod_ruta_fuera_raiz() {
    let dir = temp_dir();
    let otra = temp_dir();
    escribir(&otra, "lib.bsk", "publico sea v = 1\n");
    // intentar escapar con ../
    escribir(&dir, "main.bsk", "usar \"../lib.bsk\" como lib\n1\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(!e.is_empty(), "debe fallar");
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&otra);
}

#[test]
fn mod_diagnostico_ubicacion_import() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "publico sea valor = 5\n");
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" { noexiste }\n1\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(!e.is_empty());
    assert_eq!(e[0].linea, 1);
    assert_eq!(e[0].columna, 1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mod_sin_cargador_no_engañoso() {
    let src = "usar \"lib.bsk\" como lib\n1\n";
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let e = ctx.analizar_programa(&ss);
    assert!(!e.is_empty(), "sin cargador debe diagnosticar");
    let _ = e;
}
