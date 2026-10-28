use basilisco::CausaError;
use basilisco::cargador::{CargadorModulos, SimbodoExportado};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn temp_dir(nombre: &str) -> PathBuf {
    let base = std::env::temp_dir()
        .join("basilisco_cargador_tests")
        .join(nombre);
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    base
}

fn escribir(dir: &Path, nombre: &str, contenido: &str) -> PathBuf {
    let ruta = dir.join(nombre);
    if let Some(parent) = ruta.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&ruta, contenido).unwrap();
    ruta
}

#[test]
fn carga_modulo_valido() {
    let dir = temp_dir("valido");
    escribir(
        &dir,
        "lib.bsk",
        "publico sea PI = 3.14\npublico funcion sumar(a, b) { retornar a + b }\n",
    );
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let modulo = CargadorModulos::cargar(&cargador, "lib.bsk", None).expect("carga válida");
    assert_eq!(modulo.ruta, fs::canonicalize(dir.join("lib.bsk")).unwrap());
    assert!(modulo.exportaciones.contains_key("PI"));
    assert!(modulo.exportaciones.contains_key("sumar"));
    match modulo.exportaciones.get("PI").unwrap() {
        SimbodoExportado::Variable(v) => assert_eq!(v.to_string(), "3.14"),
        _ => panic!("PI debería ser variable"),
    }
}

#[test]
fn cache_reutiliza_modulo() {
    let dir = temp_dir("cache");
    escribir(&dir, "lib.bsk", "sea X = 42\n");
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let m1 = CargadorModulos::cargar(&cargador, "lib.bsk", None).unwrap();
    let m2 = CargadorModulos::cargar(&cargador, "lib.bsk", None).unwrap();
    assert!(
        std::rc::Rc::ptr_eq(&m1, &m2),
        "debería reutilizar el mismo Rc"
    );
}

#[test]
fn modulo_inexistente() {
    let dir = temp_dir("inexistente");
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let err = CargadorModulos::cargar(&cargador, "no_existe.bsk", None).unwrap_err();
    assert!(
        err.mensaje.contains("módulo no encontrado") || err.mensaje.contains("no se pudo leer")
    );
    assert!(err.archivo.is_some());
    match *err.causa {
        CausaError::ModuloNoEncontrado { .. } | CausaError::ArchivoNoEncontrado { .. } => {}
        other => panic!("causa inesperada: {other:?}"),
    }
}

#[test]
fn ruta_fuera_de_root() {
    let dir = temp_dir("root");
    let fuera = temp_dir("fuera_root");
    escribir(&fuera, "secreto.bsk", "sea X = 1\n");
    // Crear un archivo dentro del root para que sea el importador.
    escribir(&dir, "main.bsk", "");
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let importador = dir.join("main.bsk");
    let err = CargadorModulos::cargar(&cargador, "../fuera_root/secreto.bsk", Some(&importador))
        .unwrap_err();
    // El archivo existe fuera de la raíz: la causa debe ser exclusivamente RutaNoPermitida.
    match *err.causa {
        CausaError::RutaNoPermitida { .. } => {}
        other => panic!("causa inesperada (se esperaba RutaNoPermitida): {other:?}"),
    }
}

#[test]
fn sentencia_no_permitida_en_modulo() {
    let dir = temp_dir("sentencia");
    escribir(&dir, "lib.bsk", "mostrar \"hola\"\n");
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let err = CargadorModulos::cargar(&cargador, "lib.bsk", None).unwrap_err();
    assert!(err.mensaje.contains("sentencia no permitida") || err.mensaje.contains("sentencia"));
    match *err.causa {
        CausaError::SentenciaNoPermitidaEnModulo { .. } | CausaError::ErrorDeArchivo { .. } => {}
        other => panic!("causa inesperada: {other:?}"),
    }
}

#[test]
fn resolucion_relativa_desde_importador() {
    let dir = temp_dir("relativa");
    let sub = dir.join("sub");
    fs::create_dir_all(&sub).unwrap();
    escribir(&sub, "helper.bsk", "publico sea VALOR = 99\n");
    escribir(&dir, "main.bsk", "");
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let importador = dir.join("main.bsk");
    let modulo = CargadorModulos::cargar(&cargador, "sub/helper.bsk", Some(&importador))
        .expect("carga relativa");
    assert!(modulo.exportaciones.contains_key("VALOR"));
}

#[test]
fn diagnostico_lleva_ruta_del_modulo() {
    let dir = temp_dir("diag");
    escribir(&dir, "lib.bsk", "sea X = 1 / 0\n");
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let err = CargadorModulos::cargar(&cargador, "lib.bsk", None).unwrap_err();
    let ruta = fs::canonicalize(dir.join("lib.bsk")).unwrap();
    assert_eq!(err.archivo.as_deref(), Some(ruta.to_str().unwrap()));
}

#[test]
fn ciclo_detectado() {
    let dir = temp_dir("ciclo");
    escribir(&dir, "a.bsk", "sea X = 1\n");
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let canonical = fs::canonicalize(dir.join("a.bsk")).unwrap();
    // Simular que a.bsk está en carga.
    cargador.borrow_mut().en_carga_insert(canonical.clone());
    let err = CargadorModulos::cargar(&cargador, "a.bsk", None).unwrap_err();
    assert!(err.mensaje.contains("ciclo de importación"));
    match *err.causa {
        CausaError::CicloDeImportacion { .. } => {}
        other => panic!("causa inesperada: {other:?}"),
    }
}

#[test]
fn modulo_con_funcion_exportada() {
    let dir = temp_dir("func");
    escribir(
        &dir,
        "lib.bsk",
        "publico funcion doble(n) { retornar n * 2 }\n",
    );
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let modulo = CargadorModulos::cargar(&cargador, "lib.bsk", None).unwrap();
    assert!(modulo.exportaciones.contains_key("doble"));
    match modulo.exportaciones.get("doble").unwrap() {
        SimbodoExportado::Funcion(f) => assert_eq!(f.nombre, "doble"),
        _ => panic!("doble debería ser función"),
    }
}

#[test]
fn modulo_con_error_de_ejecucion() {
    let dir = temp_dir("exec_err");
    escribir(&dir, "lib.bsk", "sea X = 1 / 0\n");
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let err = CargadorModulos::cargar(&cargador, "lib.bsk", None).unwrap_err();
    assert!(err.mensaje.contains("división entre cero"));
    let ruta = fs::canonicalize(dir.join("lib.bsk")).unwrap();
    assert_eq!(err.archivo.as_deref(), Some(ruta.to_str().unwrap()));
}

#[test]
fn raiz_relativa_con_puntos() {
    // Una raíz con componentes `..` que resuelve a un directorio válido.
    let dir = temp_dir("raiz_puntos");
    escribir(&dir, "lib.bsk", "publico sea PI = 3.14\n");
    let raiz_con_puntos = dir.join("sub").join("..");
    fs::create_dir_all(dir.join("sub")).unwrap();
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(raiz_con_puntos)));
    let modulo = CargadorModulos::cargar(&cargador, "lib.bsk", None).expect("raíz con .. válida");
    assert!(modulo.exportaciones.contains_key("PI"));
}

#[test]
fn raiz_inexistente_error_explicito() {
    // Una raíz que no existe debe producir un error explícito, no un pánico.
    let inexistente = std::env::temp_dir().join("basilisco_no_existe_xyz_123");
    let _ = fs::remove_dir_all(&inexistente);
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(inexistente.clone())));
    let err = CargadorModulos::cargar(&cargador, "lib.bsk", None).unwrap_err();
    // Debe ser RutaNoPermitida (root canonicalization failed).
    match *err.causa {
        CausaError::RutaNoPermitida { .. } => {}
        other => panic!("causa inesperada (se esperaba RutaNoPermitida): {other:?}"),
    }
    assert!(
        err.mensaje.contains("no se pudo canonicalizar") || err.mensaje.contains("canonicalizar")
    );
}

#[test]
fn importador_nombre_simple_sin_directorio() {
    // Un importador dado como nombre de archivo sin directorio.
    let dir = temp_dir("importador_simple");
    let sub = dir.join("sub");
    fs::create_dir_all(&sub).unwrap();
    escribir(&sub, "helper.bsk", "publico sea VALOR = 42\n");
    // importador como nombre simple (no existe como archivo, fallback a root).
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let modulo = CargadorModulos::cargar(&cargador, "sub/helper.bsk", Some(Path::new("main.bsk")))
        .expect("resolución con importador simple");
    assert!(modulo.exportaciones.contains_key("VALOR"));
}

#[test]
fn archivo_existente_fuera_de_raiz_rechazado() {
    let dir = temp_dir("fuera_raiz2");
    let fuera = temp_dir("fuera_raiz2_ext");
    escribir(&fuera, "secreto.bsk", "sea X = 1\n");
    escribir(&dir, "main.bsk", "");
    let cargador = Rc::new(RefCell::new(CargadorModulos::nuevo(dir.clone())));
    let importador = dir.join("main.bsk");
    let err = CargadorModulos::cargar(
        &cargador,
        "../fuera_raiz2_ext/secreto.bsk",
        Some(&importador),
    )
    .unwrap_err();
    match *err.causa {
        CausaError::RutaNoPermitida { .. } => {}
        other => panic!("causa inesperada (se esperaba RutaNoPermitida): {other:?}"),
    }
}
