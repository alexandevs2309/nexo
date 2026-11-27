//! Pruebas del puente FFI básico (ABI C).
//!
//! Compila una biblioteca C de ejemplo con el compilador del sistema y llama
//! funciones reales vía `cargar_ffi`/`llamar_ffi`/`descargar_ffi`. Si no hay
//! compilador C, las pruebas se omiten con un aviso por stderr.

use basilisco::{
    CausaError, Comparacion, ContextoSemantico, EfectosObservables, Entorno, ErrorEjecucion, Lexer,
    Parser, ResultadoNormalizado, Valor, comparar_semantica, hir, mir, normalizar_error_tw,
    normalizar_error_vm, normalizar_valor, vm,
};
use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
use std::fs;
use std::path::{Path, PathBuf};
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
const char *texto_nulo(void) { return 0; }
double retorna_nan(void) { return 0.0 / 0.0; }
double retorna_inf(void) { return 1.0 / 0.0; }
double retorna_decimal_grande(void) { return 1e300; }
long long id_entero(long long x) { return x; }
static char eco_buf[256];
const char *eco_texto(const char *s) {
    long long i = 0;
    while (s[i] && i < 255) { eco_buf[i] = s[i]; i++; }
    eco_buf[i] = 0;
    return eco_buf;
}
static long long contador = 0;
void contador_reset(void) { contador = 0; }
long long contador_inc(void) { contador++; return contador; }
long long contador_get(void) { return contador; }
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
fn nombre_libreria() -> String {
    format!("{DLL_PREFIX}basilisco_ffi{DLL_SUFFIX}")
}

fn compilar_libreria(nombre: &str) -> Option<PathBuf> {
    let base = directorio_temporal(nombre);
    let fuente = base.join("ejemplo.c");
    fs::write(&fuente, CODIGO_C).unwrap();
    let salida = base.join(nombre_libreria());
    let mut compilador = Command::new("cc");
    compilador.arg("-shared");
    if !cfg!(windows) {
        compilador.arg("-fPIC");
    }
    let estado = compilador
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
    Some(salida)
}

fn ejecutar(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida");
    Entorno::nueva().ejecutar_programa(&sentencias)
}

fn escapar_ruta(ruta: &Path) -> String {
    ruta.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn declaracion_biblioteca(ruta: &Path) -> String {
    format!("sea lib = cargar_ffi(\"{}\")\n", escapar_ruta(ruta))
}

/// Programa que carga la biblioteca y termina en una expresión llamada,
/// para que `ejecutar` devuelva su valor.
fn programa(ruta: &Path, ultima_expresion: &str) -> String {
    format!("{}{}\n", declaracion_biblioteca(ruta), ultima_expresion)
}

fn vm_resultado(fuente: &str) -> ResultadoNormalizado {
    let tokens = Lexer::nuevo(fuente).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let errores_sem = ctx.analizar_programa(&ss);
    if let Some(e) = errores_sem.first() {
        let relevante = matches!(
            e.causa.as_ref(),
            CausaError::ReasignacionInmutable { .. }
                | CausaError::CondicionNoLogica { .. }
                | CausaError::VariableNoDefinida(_)
        );
        if relevante {
            return ResultadoNormalizado::Err(normalizar_error_tw(e));
        }
    }
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    if let Some(e) = errs_g4.first() {
        return ResultadoNormalizado::Err(normalizar_error_tw(e));
    }
    let hir_p = match hir::lower_programa(&ss, &ctx) {
        Ok(h) => h,
        Err(e) => {
            return ResultadoNormalizado::Err(basilisco::paridad::ErrorNormalizado {
                causa: basilisco::paridad::CausaNormalizada {
                    categoria: basilisco::paridad::CategoriaError::NoSoportado,
                    subcausa: Some("hir_no_implementado".to_string()),
                    datos: {
                        let mut d = std::collections::BTreeMap::new();
                        d.insert("detalle".into(), format!("{e:?}"));
                        d
                    },
                },
            });
        }
    };
    let mir_p = mir::lower_programa(&hir_p);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir_p);
    maquina.registrar_stdlib();
    let Some(idx) = maquina.indice_de("main") else {
        return ResultadoNormalizado::Err(basilisco::paridad::ErrorNormalizado {
            causa: basilisco::paridad::CausaNormalizada {
                categoria: basilisco::paridad::CategoriaError::FuncionNoEncontrada,
                subcausa: None,
                datos: {
                    let mut d = std::collections::BTreeMap::new();
                    d.insert("nombre".into(), "main".into());
                    d
                },
            },
        });
    };
    match maquina.ejecutar_indice(idx, &[]) {
        Ok(v) => ResultadoNormalizado::Ok {
            valor: v.as_ref().map(normalizar_valor),
            efectos: EfectosObservables::default(),
        },
        Err(e) => ResultadoNormalizado::Err(normalizar_error_vm(&e)),
    }
}

fn tw_resultado(fuente: &str) -> ResultadoNormalizado {
    let src = if fuente.contains("funcion main") {
        format!(
            "{}
main()",
            fuente
        )
    } else {
        fuente.to_string()
    };
    match ejecutar(&src) {
        Ok(v) => ResultadoNormalizado::Ok {
            valor: v.as_ref().map(normalizar_valor),
            efectos: EfectosObservables::default(),
        },
        Err(e) => ResultadoNormalizado::Err(normalizar_error_tw(&e)),
    }
}

fn assert_ffi_parity(fuente: &str) {
    let tw = tw_resultado(fuente);
    let vmr = vm_resultado(fuente);
    let cmp = comparar_semantica(&tw, &vmr);
    match cmp {
        Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_) => {}
        other => {
            panic!("divergencia TW↔VM FFI\nSRC:\n{fuente}\nTW: {tw:?}\nVM: {vmr:?}\nCMP: {other:?}")
        }
    }
}

#[test]
fn rutas_windows_se_escapan_como_literales_basilisco() {
    let ruta = PathBuf::from(r#"C:\a\biblioteca "ffi".dll"#);
    let fuente = programa(&ruta, "0");
    let tokens = Lexer::nuevo(&fuente).tokenizar().expect("léxico válido");
    Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida");
    assert!(fuente.contains(r#"C:\\a\\biblioteca \"ffi\".dll"#));
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
    let ruta_inexistente = directorio_temporal("inexistente").join(nombre_libreria());
    let e = ejecutar(&programa(&ruta_inexistente, "0")).unwrap_err();
    match *e.causa {
        CausaError::FFIInvalido { causa } => {
            let detalle = causa.to_string();
            assert!(
                detalle.contains("biblioteca FFI no encontrada")
                    || detalle.contains("biblioteca FFI no cargable"),
                "{detalle}"
            );
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
        CausaError::FFIInvalido { causa } => {
            let detalle = causa.to_string();
            assert!(
                detalle.contains("no_existe_nada") || detalle.contains("símbolo FFI"),
                "{detalle}"
            );
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
    let fuente = format!(
        "{}descargar_ffi(lib)\nllamar_ffi(lib, \"sumar\", [1, 2], \"entero\")\n",
        declaracion_biblioteca(&ruta)
    );
    let e = ejecutar(&fuente).unwrap_err();
    match *e.causa {
        CausaError::FFIInvalido { causa } => {
            let detalle = causa.to_string();
            assert!(
                detalle.contains("biblioteca FFI descargada")
                    || detalle.contains("id de biblioteca FFI inválido"),
                "{detalle}"
            );
        }
        otro => panic!("se esperaba FFIInvalido, se obtuvo {otro:?}"),
    }
}

#[test]
fn descargar_con_id_invalido_es_error() {
    let e = ejecutar("descargar_ffi(999)").unwrap_err();
    match *e.causa {
        CausaError::FFIInvalido { causa } => {
            let detalle = causa.to_string();
            assert!(
                detalle.contains("biblioteca FFI descargada")
                    || detalle.contains("id de biblioteca FFI inválido"),
                "{detalle}"
            );
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
        CausaError::FFIInvalido { causa } => {
            let detalle = causa.to_string();
            assert!(
                detalle.contains("incompatible") || detalle.contains("no aceptado"),
                "{detalle}"
            );
            assert!(detalle.contains("lógico"), "{detalle}");
        }
        otro => panic!("se esperaba FFIInvalido, se obtuvo {otro:?}"),
    }
}

fn programa_main(ruta: &Path, cuerpo: &str, retorno: &str) -> String {
    format!(
        "funcion main() -> {} {{\n  {}\n  retornar {}\n}}\n",
        "texto", /* placeholder, overridden below */
        declaracion_biblioteca(ruta),
        cuerpo
    )
    .replace(
        "funcion main() -> texto {",
        &format!("funcion main() -> {} {{", retorno),
    )
}

fn programa_main_stmt(ruta: &Path, stmts: &str) -> String {
    format!(
        "funcion main() -> entero {{\n  {}\n  {}\n  retornar 0\n}}\n",
        declaracion_biblioteca(ruta),
        stmts
    )
}

#[test]
fn tw_vm_paridad_ffi_basico() {
    let Some(ruta) = compilar_libreria("paridad_basico") else {
        return;
    };
    let src = programa_main(
        &ruta,
        "llamar_ffi(lib, \"sumar\", [20, 22], \"entero\")",
        "entero",
    );
    assert_ffi_parity(&src);
    let src = programa_main(
        &ruta,
        "llamar_ffi(lib, \"media\", [2.0, 4.0], \"real\")",
        "real",
    );
    assert_ffi_parity(&src);
    let src = programa_main(&ruta, "llamar_ffi(lib, \"saludo\", [], \"texto\")", "texto");
    assert_ffi_parity(&src);
}

#[test]
fn tw_vm_paridad_ffi_errores() {
    let Some(ruta) = compilar_libreria("paridad_err") else {
        return;
    };
    let src = programa_main(
        &ruta,
        "llamar_ffi(lib, \"no_existe\", [], \"entero\")",
        "entero",
    );
    assert_ffi_parity(&src);
    let src = programa_main_stmt(
        &ruta,
        "descargar_ffi(lib)\n  llamar_ffi(lib, \"sumar\", [1, 2], \"entero\")",
    );
    assert_ffi_parity(&src);
}

#[test]
fn ffi_id_invalido_en_llamar() {
    let e = ejecutar("llamar_ffi(9999, \"sumar\", [1, 2], \"entero\")").unwrap_err();
    let CausaError::FFIInvalido { causa } = &*e.causa else {
        panic!("esperaba FFIInvalido, got {:?}", e.causa)
    };
    let s = causa.to_string();
    assert!(
        s.contains("descargada") || s.contains("inválido") || s.contains("id"),
        "{s}"
    );
}

#[test]
fn ffi_retorno_invalido_tipo_no_aceptado() {
    let Some(ruta) = compilar_libreria("ret_tipo") else {
        return;
    };
    let e = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"sumar\", [1, 2], \"puntero\")",
    ))
    .unwrap_err();
    let CausaError::FFIInvalido { causa } = &*e.causa else {
        panic!("{:?}", e.causa)
    };
    assert_eq!(causa.codigo(), "E1075");
    assert!(
        causa.to_string().contains("tipo de retorno") || causa.to_string().contains("firma"),
        "{}",
        causa
    );
}

#[test]
fn ffi_texto_nulo_retorno_es_error() {
    let Some(ruta) = compilar_libreria("texto_nulo") else {
        return;
    };
    let e = ejecutar(&programa(
        &ruta,
        "llamar_ffi(lib, \"texto_nulo\", [], \"texto\")",
    ))
    .unwrap_err();
    let CausaError::FFIInvalido { causa } = &*e.causa else {
        panic!("{:?}", e.causa)
    };
    assert_eq!(causa.codigo(), "E1079");
    assert!(causa.to_string().contains("nulo"), "{}", causa);
}

#[test]
fn ffi_decimal_nan_inf_grande_es_error() {
    let Some(ruta) = compilar_libreria("decimal_nan") else {
        return;
    };
    for (sym, label) in [
        ("retorna_nan", "nan"),
        ("retorna_inf", "inf"),
        ("retorna_decimal_grande", "grande"),
    ] {
        let e = ejecutar(&programa(
            &ruta,
            &format!("llamar_ffi(lib, \"{sym}\", [], \"decimal\")"),
        ))
        .unwrap_err();
        let CausaError::FFIInvalido { causa } = &*e.causa else {
            panic!("{label}: {:?}", e.causa)
        };
        assert_eq!(causa.codigo(), "E1078", "{label}: {}", causa);
    }
}

#[test]
fn ffi_argumento_texto_con_nul_interno_es_error() {
    let Some(ruta) = compilar_libreria("nul_texto") else {
        return;
    };
    let id = basilisco::ffi::cargar_libreria(ruta.to_str().unwrap()).unwrap();
    let err = basilisco::ffi::llamar_funcion(id, "largo", &[Valor::Texto("a\0b".into())], "entero")
        .unwrap_err();
    assert_eq!(err.codigo(), "E1077");
    assert!(err.to_string().contains("NUL"), "{}", err);
    let _ = basilisco::ffi::descargar_libreria(id);
}

#[test]
fn ffi_simbolo_con_nul_es_error() {
    let Some(ruta) = compilar_libreria("nul_sym") else {
        return;
    };
    let id = basilisco::ffi::cargar_libreria(ruta.to_str().unwrap()).unwrap();
    let err = basilisco::ffi::llamar_funcion(id, "su\0mar", &[], "entero").unwrap_err();
    assert_eq!(err.codigo(), "E1072");
    let _ = basilisco::ffi::descargar_libreria(id);
}

#[test]
fn ffi_descargar_y_recargar_genera_nuevo_id() {
    let Some(ruta) = compilar_libreria("reload") else {
        return;
    };
    let id1 = basilisco::ffi::cargar_libreria(ruta.to_str().unwrap()).unwrap();
    basilisco::ffi::descargar_libreria(id1).unwrap();
    let err = basilisco::ffi::llamar_funcion(
        id1,
        "sumar",
        &[Valor::Entero(1), Valor::Entero(2)],
        "entero",
    )
    .unwrap_err();
    assert!(matches!(err.codigo(), "E1073" | "E1074"));
    let id2 = basilisco::ffi::cargar_libreria(ruta.to_str().unwrap()).unwrap();
    assert_ne!(id1, id2);
    let v = basilisco::ffi::llamar_funcion(
        id2,
        "sumar",
        &[Valor::Entero(2), Valor::Entero(3)],
        "entero",
    )
    .unwrap();
    assert_eq!(v, Valor::Entero(5));
    let _ = basilisco::ffi::descargar_libreria(id2);
}

#[test]
fn ffi_ciclo_vida_estado_dentro_biblioteca() {
    let Some(ruta) = compilar_libreria("estado") else {
        return;
    };
    let src = format!(
        "{}\nllamar_ffi(lib, \"contador_reset\", [], \"nulo\")\nllamar_ffi(lib, \"contador_inc\", [], \"entero\")\nllamar_ffi(lib, \"contador_inc\", [], \"entero\")\nllamar_ffi(lib, \"contador_get\", [], \"entero\")\n",
        declaracion_biblioteca(&ruta)
    );
    let v = ejecutar(&src).unwrap().unwrap();
    assert_eq!(v, Valor::Entero(2));
}

#[test]
fn ffi_texto_roundtrip_y_boundaries() {
    let Some(ruta) = compilar_libreria("roundtrip") else {
        return;
    };
    let src = format!(
        "{}\nllamar_ffi(lib, \"eco_texto\", [\"hola\"], \"texto\")\n",
        declaracion_biblioteca(&ruta)
    );
    let v = ejecutar(&src).unwrap().unwrap();
    assert_eq!(v, Valor::Texto("hola".into()));
    let src = format!(
        "{}\nllamar_ffi(lib, \"id_entero\", [-9223372036854775808], \"entero\")\n",
        declaracion_biblioteca(&ruta)
    );
    let v = ejecutar(&src).unwrap().unwrap();
    assert_eq!(v, Valor::Entero(i64::MIN));
}

#[test]
fn tw_vm_paridad_ffi_errores_tipados() {
    let Some(ruta) = compilar_libreria("paridad_tipos_err") else {
        return;
    };
    let src = programa_main_stmt(&ruta, "llamar_ffi(lib, \"texto_nulo\", [], \"texto\")");
    assert_ffi_parity(&src);
    let src = programa_main_stmt(&ruta, "llamar_ffi(lib, \"retorna_nan\", [], \"decimal\")");
    assert_ffi_parity(&src);
    let src = programa_main(
        &ruta,
        "llamar_ffi(lib, \"sumar\", [verdadero], \"entero\")",
        "entero",
    );
    assert_ffi_parity(&src);
}

#[test]
fn ffi_error_tiene_codigo_e10xx() {
    let e = ejecutar("descargar_ffi(999999)").unwrap_err();
    let s = e.to_string();
    assert!(s.contains("[E1073]") || s.contains("[E1074]"), "{s}");
}
