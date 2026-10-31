//! Pruebas del puente FFI básico (ABI C).
//!
//! Compila una biblioteca C de ejemplo con el compilador del sistema y llama
//! funciones reales vía `cargar_ffi`/`llamar_ffi`/`descargar_ffi`. Si no hay
//! compilador C, las pruebas se omiten con un aviso por stderr.

use basilisco::{CausaError, Entorno, ErrorEjecucion, Lexer, Parser, Valor};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const CODIGO_C: &str = r#"
long long sumar(long long a, long long b) { return a + b; }
long long cuadrado(long long x) { return x * x; }
int es_par(long long x) { return x % 2 == 0; }
double media(double a, double b) { return (a + b) / 2.0; }
double area_circulo(double r) { return 3.14159265358979323846 * r * r; }
double mezcla(long long a, double b) { return a + b; }
const char *saludo(void) { return "hola ffi"; }
long long largo(const char *s) { long long n = 0; while (s[n]) n++; return n; }
void nada(void) {}
"#;

fn directorio_temporal(nombre: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("ffi-{nombre}-{}", std::process::id()));
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();
    base
}

/// Compila la biblioteca de ejemplo; devuelve `None` si no hay compilador C.
fn compilar_libreria(nombre: &str) -> Option<String> {
    let base = directorio_temporal(nombre);
    let fuente = base.join("ejemplo.c");
    fs::write(&fuente, CODIGO_C).unwrap();
    let salida = base.join("libbasilisco_ffi.so");
    let estado = Command::new("cc")
        .arg("-shared")
        .arg("-fPIC")
        .arg("-o")
        .arg(&salida)
        .arg(&fuente)
        .output()
        .ok()?;
    if !estado.status.success() {
        eprintln!(
            "aviso: compilación C falló ({}); prueba FFI omitida",
            String::from_utf8_lossy(&estado.stderr).trim()
        );
        return None;
    }
    Some(salida.to_string_lossy().into_owned())
}

fn ejecutar(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida");
    Entorno::nueva().ejecutar_programa(&sentencias)
}

/// Programa que carga la biblioteca y termina en una expresión llamada,
/// para que `ejecutar` devuelva su valor.
fn programa(ruta: &str, ultima_expresion: &str) -> String {
    format!("sea lib = cargar_ffi(\"{ruta}\")\n{ultima_expresion}\n")
}

#[test]
fn enteros() {
    let Some(ruta) = compilar_libreria("enteros") else {
        return;
    };
    let r = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"sumar\", [2, 3], \"entero\")",
    ))
    .unwrap();
    assert_eq!(r, Some(Valor::Entero(5)));

    let r = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"cuadrado\", [7], \"entero\")",
    ))
    .unwrap();
    assert_eq!(r, Some(Valor::Entero(49)));
}

#[test]
fn logico_desde_entero_c() {
    let Some(ruta) = compilar_libreria("logico") else {
        return;
    };
    let r = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"es_par\", [4], \"logico\")",
    ))
    .unwrap();
    assert_eq!(r, Some(Valor::Logico(true)));

    let r = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"es_par\", [3], \"logico\")",
    ))
    .unwrap();
    assert_eq!(r, Some(Valor::Logico(false)));
}

#[test]
fn reales() {
    let Some(ruta) = compilar_libreria("reales") else {
        return;
    };
    let r = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"media\", [2.0, 4.0], \"real\")",
    ))
    .unwrap();
    assert_eq!(r, Some(Valor::Real(3.0)));

    let r = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"area_circulo\", [1.0], \"real\")",
    ))
    .unwrap();
    let Some(Valor::Real(area)) = r else {
        panic!("se esperaba real");
    };
    assert!((area - std::f64::consts::PI).abs() < 1e-9, "{area}");
}

#[test]
fn tipos_mixtos() {
    let Some(ruta) = compilar_libreria("mixtos") else {
        return;
    };
    let r = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"mezcla\", [2, 0.5], \"real\")",
    ))
    .unwrap();
    assert_eq!(r, Some(Valor::Real(2.5)));
}

#[test]
fn decimal_escalado() {
    let Some(ruta) = compilar_libreria("decimal") else {
        return;
    };
    let r = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"media\", [2.0, 5.0], \"decimal\")",
    ))
    .unwrap();
    assert_eq!(r, Some(Valor::Decimal(3_500_000_000)));
}

#[test]
fn texto_de_biblioteca() {
    let Some(ruta) = compilar_libreria("texto") else {
        return;
    };
    let r = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"saludo\", [], \"texto\")",
    ))
    .unwrap();
    assert_eq!(r, Some(Valor::Texto("hola ffi".to_string())));

    let r = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"largo\", [\"hola ffi\"], \"entero\")",
    ))
    .unwrap();
    assert_eq!(r, Some(Valor::Entero(8)));
}

#[test]
fn nulo() {
    let Some(ruta) = compilar_libreria("nulo") else {
        return;
    };
    let r = ejecutar(&programa(&ruta, "llamar_ffi(lib, \"nada\", [], \"nulo\")")).unwrap();
    assert_eq!(r, Some(Valor::Nulo));
}

#[test]
fn cargar_ruta_inexistente_es_error() {
    let e = ejecutar("cargar_ffi(\"/no/existe/libreria.so\")").unwrap_err();
    match *e.causa {
        CausaError::FFIInvalido { detalle } => {
            assert!(detalle.contains("no se pudo cargar"), "{detalle}");
        }
        otro => panic!("se esperaba FFIInvalido, se obtuvo {otro:?}"),
    }
}

#[test]
fn simbolo_inexistente_es_error() {
    let Some(ruta) = compilar_libreria("simbolo") else {
        return;
    };
    let e = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"no_existe_nada\", [], \"entero\")",
    ))
    .unwrap_err();
    match *e.causa {
        CausaError::FFIInvalido { detalle } => {
            assert!(detalle.contains("no_existe_nada"), "{detalle}");
        }
        otro => panic!("se esperaba FFIInvalido, se obtuvo {otro:?}"),
    }
}

#[test]
fn tipo_de_retorno_aceptados() {
    let Some(ruta) = compilar_libreria("retorno") else {
        return;
    };
    let e = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"sumar\", [1, 2], \"float\")",
    ))
    .unwrap_err();
    assert!(e.mensaje.contains("tipo de retorno"), "{}", e.mensaje);
}

#[test]
fn descargar_y_llamar_es_error() {
    let Some(ruta) = compilar_libreria("descarga") else {
        return;
    };
    let e = ejecutar(&format!(
        "sea lib = cargar_ffi(\"{ruta}\")\n\
         descargar_ffi(lib)\n\
         llamar_ffi(lib, \"sumar\", [1, 2], \"entero\")\n"
    ))
    .unwrap_err();
    match *e.causa {
        CausaError::FFIInvalido { detalle } => {
            assert!(detalle.contains("no hay biblioteca cargada"), "{detalle}");
        }
        otro => panic!("se esperaba FFIInvalido, se obtuvo {otro:?}"),
    }
}

#[test]
fn descargar_con_id_invalido_es_error() {
    let e = ejecutar("descargar_ffi(999)").unwrap_err();
    match *e.causa {
        CausaError::FFIInvalido { detalle } => {
            assert!(detalle.contains("no hay biblioteca cargada"), "{detalle}");
        }
        otro => panic!("se esperaba FFIInvalido, se obtuvo {otro:?}"),
    }
}

#[test]
fn argumento_de_tipo_no_aceptado_es_error() {
    let Some(ruta) = compilar_libreria("argumento") else {
        return;
    };
    let e = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"sumar\", [verdadero], \"entero\")",
    ))
    .unwrap_err();
    match *e.causa {
        CausaError::FFIInvalido { detalle } => {
            assert!(detalle.contains("no aceptado"), "{detalle}");
            assert!(detalle.contains("lógico"), "{detalle}");
        }
        otro => panic!("se esperaba FFIInvalido, se obtuvo {otro:?}"),
    }
}
