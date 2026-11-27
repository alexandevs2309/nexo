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

// ===== FE-02: resolución semántica de miembros vía alias de módulo =====

#[test]
fn fe02_acceso_miembro_publico_existente() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "publico sea valor = 5\n");
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" como lib\nlib.valor\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_acceso_miembro_inexistente_diagnostico_semantico() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "publico sea valor = 5\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nlib.noexiste\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(!e.is_empty(), "debe diagnosticar miembro inexistente");
    assert!(
        e.iter()
            .any(|x| format!("{:?}", x.causa).contains("SimbodoNoExportado")),
        "causa esperada SimbodoNoExportado, errores: {:?}",
        e
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_acceso_miembro_privado_desde_otro_modulo() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "sea secreto = 9\n");
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" como lib\nlib.secreto\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(!e.is_empty(), "debe diagnosticar miembro privado");
    assert!(
        e.iter()
            .any(|x| format!("{:?}", x.causa).contains("SimbodoPrivado")),
        "causa esperada SimbodoPrivado, errores: {:?}",
        e
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_llamada_funcion_publica_por_alias() {
    let dir = temp_dir();
    escribir(
        &dir,
        "lib.bsk",
        "publico funcion suma(a, b) { retornar a + b }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nlib.suma(1, 2)\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_llamada_miembro_inexistente_por_alias() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "publico sea valor = 1\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nlib.noexiste(1)\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(
        e.iter()
            .any(|x| format!("{:?}", x.causa).contains("SimbodoNoExportado")),
        "causa esperada SimbodoNoExportado, errores: {:?}",
        e
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_llamada_miembro_privado_por_alias() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "funcion oculta(x) { retornar x }\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nlib.oculta(1)\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(
        e.iter()
            .any(|x| format!("{:?}", x.causa).contains("SimbodoPrivado")),
        "causa esperada SimbodoPrivado, errores: {:?}",
        e
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_dos_aliases_mismo_modulo_sin_contaminacion() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "publico sea valor = 5\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como a\nusar \"lib.bsk\" como b\na.valor\nb.valor\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_dos_modulos_mismo_nombre_miembro_aislados() {
    let dir = temp_dir();
    escribir(&dir, "a.bsk", "publico sea valor = 1\n");
    escribir(&dir, "b.bsk", "publico sea valor = 2\nsea secreto = 3\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"a.bsk\" como ma\nusar \"b.bsk\" como mb\nma.valor\nmb.valor\nmb.secreto\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(
        e.iter()
            .any(|x| format!("{:?}", x.causa).contains("SimbodoPrivado")
                && x.mensaje.contains("secreto")),
        "debe reportar secreto privado solo en mb, errores: {:?}",
        e
    );
    assert!(
        !e.iter().any(|x| x.mensaje.contains("valor")
            && format!("{:?}", x.causa).contains("SimbodoNoExportado")),
        "no debe contaminar valor entre módulos, errores: {:?}",
        e
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_alias_sombreado_por_variable_local_no_resuelve_como_modulo() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "publico sea valor = 5\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nsi verdadero {\n  sea lib = 1\n  lib\n}\nlib.valor\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_alias_mismo_nombre_en_ambitos_diferentes() {
    let dir = temp_dir();
    escribir(&dir, "a.bsk", "publico sea x = 1\n");
    escribir(&dir, "b.bsk", "publico sea z = 2\n");
    escribir(
        &dir,
        "main.bsk",
        "si verdadero {\n  usar \"a.bsk\" como m\n  m.x\n}\nsi verdadero {\n  usar \"b.bsk\" como m\n  m.z\n}\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_diagnostico_conserva_ubicacion() {
    let dir = temp_dir();
    escribir(&dir, "lib.bsk", "publico sea valor = 5\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nlib.noexiste\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    let err = e
        .iter()
        .find(|x| x.mensaje.contains("noexiste"))
        .expect("debe haber error");
    assert_eq!(err.linea, 2);
    assert_eq!(err.columna, 4);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_miembro_privado_mismo_modulo_via_alias_rechazado_por_contrato_exportacion() {
    // El acceso vía alias de módulo pasa por frontera de exportación: un miembro
    // no exportado es privado incluso si el archivo es el mismo módulo.
    let dir = temp_dir();
    escribir(
        &dir,
        "lib.bsk",
        "sea interno = 1\npublico sea visto = interno\n",
    );
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" como lib\nlib.interno\n");
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(
        e.iter()
            .any(|x| format!("{:?}", x.causa).contains("SimbodoPrivado")),
        "contrato de exportación: privado no accesible vía alias, errores: {:?}",
        e
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_modulos_anidados_aislamiento_alias() {
    let dir = temp_dir();
    escribir(&dir, "c.bsk", "publico sea base = 10\nsea oculto = 99\n");
    escribir(
        &dir,
        "b.bsk",
        "usar \"c.bsk\" como c\npublico funcion f() { retornar c.base }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"b.bsk\" como b\nusar \"c.bsk\" como c\nb.f()\nc.base\nc.oculto\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(
        e.iter()
            .any(|x| format!("{:?}", x.causa).contains("SimbodoPrivado")
                && x.mensaje.contains("oculto")),
        "oculto debe ser privado, errores: {:?}",
        e
    );
    assert!(
        !e.iter().any(|x| x.mensaje.contains("base")
            && !x.mensaje.is_empty()
            && format!("{:?}", x.causa).contains("SimbodoNoExportado")),
        "base público no debe contaminarse, errores: {:?}",
        e
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fe02_acceso_publico_valido_runtime_semantico_coherente() {
    // Confirma que el semántico acepta lo que el runtime ejecuta correctamente.
    let dir = temp_dir();
    escribir(
        &dir,
        "lib.bsk",
        "publico funcion doble(n) { retornar n * 2 }\npublico sea v = 7\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nlib.doble(lib.v)\n",
    );
    let e = analizar_archivo(&dir, "main.bsk");
    assert!(e.is_empty(), "errores: {:?}", e);
    let _ = fs::remove_dir_all(&dir);
}
