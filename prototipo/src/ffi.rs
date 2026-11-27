//! Puente FFI (ABI C), compartido por evaluador (Tree-Walk) y VM.
//!
//! Carga bibliotecas dinámicas, resuelve símbolos y los invoca con `libffi`.
//! Los errores se devuelven tipados (`CausaFfi`) para que cada motor los
//! integre en su sistema de diagnósticos.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use crate::valor::{FACTOR_ESCALA, Valor};

/// Causa tipada de error FFI. Cada variante tiene código de diagnóstico estable.
#[derive(Debug, Clone, PartialEq)]
pub enum CausaFfi {
    BibliotecaNoEncontrada {
        ruta: String,
        detalle: String,
    },
    BibliotecaNoCargable {
        ruta: String,
        detalle: String,
    },
    SimboloNoEncontrado {
        simbolo: String,
        detalle: String,
    },
    IdBibliotecaInvalido {
        id: i64,
    },
    BibliotecaDescargada {
        id: i64,
    },
    FirmaInvalida {
        detalle: String,
    },
    AridadIncompatible {
        esperado: usize,
        obtenido: usize,
    },
    TipoArgumentoIncompatible {
        indice: usize,
        obtenido: String,
        detalle: String,
    },
    ConversionFueraDeRango {
        detalle: String,
    },
    ResultadoIncompatible {
        detalle: String,
    },
    ErrorInvocacion {
        detalle: String,
    },
    EstadoRecursoInvalido {
        detalle: String,
    },
    Memoria {
        detalle: String,
    },
}

impl CausaFfi {
    pub fn codigo(&self) -> &'static str {
        match self {
            Self::BibliotecaNoEncontrada { .. } => "E1070",
            Self::BibliotecaNoCargable { .. } => "E1071",
            Self::SimboloNoEncontrado { .. } => "E1072",
            Self::IdBibliotecaInvalido { .. } => "E1073",
            Self::BibliotecaDescargada { .. } => "E1074",
            Self::FirmaInvalida { .. } => "E1075",
            Self::AridadIncompatible { .. } => "E1076",
            Self::TipoArgumentoIncompatible { .. } => "E1077",
            Self::ConversionFueraDeRango { .. } => "E1078",
            Self::ResultadoIncompatible { .. } => "E1079",
            Self::ErrorInvocacion { .. } => "E1080",
            Self::EstadoRecursoInvalido { .. } => "E1081",
            Self::Memoria { .. } => "E1082",
        }
    }
}

impl fmt::Display for CausaFfi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BibliotecaNoEncontrada { ruta, detalle } => {
                write!(f, "biblioteca FFI no encontrada '{ruta}': {detalle}")
            }
            Self::BibliotecaNoCargable { ruta, detalle } => {
                write!(f, "biblioteca FFI no cargable '{ruta}': {detalle}")
            }
            Self::SimboloNoEncontrado { simbolo, detalle } => {
                write!(f, "símbolo FFI '{simbolo}' no encontrado: {detalle}")
            }
            Self::IdBibliotecaInvalido { id } => {
                write!(f, "id de biblioteca FFI inválido: {id}")
            }
            Self::BibliotecaDescargada { id } => {
                write!(f, "biblioteca FFI descargada: id {id}")
            }
            Self::FirmaInvalida { detalle } => write!(f, "firma FFI inválida: {detalle}"),
            Self::AridadIncompatible { esperado, obtenido } => write!(
                f,
                "aridad FFI incompatible: esperado {esperado}, obtenido {obtenido}"
            ),
            Self::TipoArgumentoIncompatible {
                indice,
                obtenido,
                detalle,
            } => write!(
                f,
                "argumento FFI {indice} de tipo '{obtenido}' incompatible: {detalle}"
            ),
            Self::ConversionFueraDeRango { detalle } => {
                write!(f, "conversión FFI fuera de rango: {detalle}")
            }
            Self::ResultadoIncompatible { detalle } => {
                write!(f, "resultado FFI incompatible: {detalle}")
            }
            Self::ErrorInvocacion { detalle } => write!(f, "error de invocación FFI: {detalle}"),
            Self::EstadoRecursoInvalido { detalle } => {
                write!(f, "estado de recurso FFI inválido: {detalle}")
            }
            Self::Memoria { detalle } => write!(f, "error de memoria FFI: {detalle}"),
        }
    }
}

/// Bibliotecas dinámicas cargadas, por id.
static LIBRERIAS_FFI: LazyLock<Mutex<HashMap<i64, Arc<libloading::Library>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Generador de ids de biblioteca.
static SIGUIENTE_ID_FFI: AtomicI64 = AtomicI64::new(0);

fn clasificar_error_carga(ruta: &str, e: impl fmt::Display) -> CausaFfi {
    let detalle = e.to_string();
    let lower = detalle.to_lowercase();
    if lower.contains("no such file")
        || lower.contains("not found")
        || lower.contains("no existe")
        || lower.contains("cannot open shared object file")
    {
        CausaFfi::BibliotecaNoEncontrada {
            ruta: ruta.to_string(),
            detalle,
        }
    } else {
        CausaFfi::BibliotecaNoCargable {
            ruta: ruta.to_string(),
            detalle,
        }
    }
}

/// Carga una biblioteca dinámica y devuelve su id.
pub fn cargar_libreria(ruta: &str) -> Result<i64, CausaFfi> {
    let libreria =
        unsafe { libloading::Library::new(ruta) }.map_err(|e| clasificar_error_carga(ruta, e))?;
    let id = SIGUIENTE_ID_FFI.fetch_add(1, Ordering::SeqCst);
    LIBRERIAS_FFI.lock().unwrap().insert(id, Arc::new(libreria));
    Ok(id)
}

/// Descarga una biblioteca cargada por id.
pub fn descargar_libreria(id: i64) -> Result<(), CausaFfi> {
    LIBRERIAS_FFI
        .lock()
        .unwrap()
        .remove(&id)
        .map(|_| ())
        .ok_or(CausaFfi::IdBibliotecaInvalido { id })
}

/// Invoca un símbolo de una biblioteca cargada.
///
/// Tipos ABI admitidos como argumentos: `entero` (i64 C long long), `real`
/// (f64 C double), `decimal` (f64 C double, escalado /10^9), `texto`
/// (const char* UTF-8 NUL-terminated, propiedad del llamador durante la llamada).
///
/// Tipos de retorno admitidos: `entero`, `logico`, `real`, `decimal`, `texto`,
/// `nulo`/`vacío`.
pub fn llamar_funcion(
    id: i64,
    simbolo: &str,
    argumentos: &[Valor],
    retorno: &str,
) -> Result<Valor, CausaFfi> {
    let libreria = LIBRERIAS_FFI
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or(CausaFfi::BibliotecaDescargada { id })?;

    let simbolo_bytes = CString::new(simbolo).map_err(|_| CausaFfi::SimboloNoEncontrado {
        simbolo: simbolo.to_string(),
        detalle: "el nombre del símbolo contiene byte NUL".to_string(),
    })?;
    let simbolo_ffi = unsafe { libreria.get::<*mut c_void>(simbolo_bytes.as_bytes_with_nul()) }
        .map_err(|e| CausaFfi::SimboloNoEncontrado {
            simbolo: simbolo.to_string(),
            detalle: e.to_string(),
        })?;
    let funcion_ptr = *simbolo_ffi;

    let mut tipos_args = Vec::with_capacity(argumentos.len());
    let mut valores_i64: Vec<i64> = Vec::with_capacity(argumentos.len());
    let mut valores_f64: Vec<f64> = Vec::with_capacity(argumentos.len());
    let mut valores_texto: Vec<CString> = Vec::with_capacity(argumentos.len());

    for (i, a) in argumentos.iter().enumerate() {
        match a {
            Valor::Entero(v) => {
                tipos_args.push(libffi::middle::Type::i64());
                valores_i64.push(*v);
            }
            Valor::Real(v) => {
                tipos_args.push(libffi::middle::Type::f64());
                valores_f64.push(*v);
            }
            Valor::Decimal(v) => {
                tipos_args.push(libffi::middle::Type::f64());
                valores_f64.push(*v as f64 / FACTOR_ESCALA as f64);
            }
            Valor::Texto(s) => match CString::new(s.clone()) {
                Ok(c) => {
                    tipos_args.push(libffi::middle::Type::pointer());
                    valores_texto.push(c);
                }
                Err(e) => {
                    return Err(CausaFfi::TipoArgumentoIncompatible {
                        indice: i + 1,
                        obtenido: "texto".to_string(),
                        detalle: format!("texto con byte NUL interno ({e})"),
                    });
                }
            },
            otro => {
                return Err(CausaFfi::TipoArgumentoIncompatible {
                    indice: i + 1,
                    obtenido: otro.nombre_tipo().to_string(),
                    detalle: "ABI C admite: entero, real, decimal, texto".to_string(),
                });
            }
        }
    }

    let textos_ptr: Vec<*const c_char> = valores_texto.iter().map(|c| c.as_ptr()).collect();
    let mut args_c: Vec<libffi::middle::Arg> = Vec::with_capacity(argumentos.len());
    let mut idx_i64 = 0usize;
    let mut idx_f64 = 0usize;
    let mut idx_texto = 0usize;
    for a in argumentos {
        match a {
            Valor::Entero(_) => {
                args_c.push(libffi::middle::arg(&valores_i64[idx_i64]));
                idx_i64 += 1;
            }
            Valor::Real(_) | Valor::Decimal(_) => {
                args_c.push(libffi::middle::arg(&valores_f64[idx_f64]));
                idx_f64 += 1;
            }
            Valor::Texto(_) => {
                args_c.push(libffi::middle::arg(&textos_ptr[idx_texto]));
                idx_texto += 1;
            }
            _ => unreachable!("tipos no aceptados rechazados antes"),
        }
    }

    let resultado = match retorno {
        "entero" | "logico" => {
            let cif = libffi::middle::Cif::new(tipos_args.clone(), libffi::middle::Type::i64());
            let r = catch_unwind(AssertUnwindSafe(|| unsafe {
                cif.call::<i64>(libffi::low::CodePtr::from_ptr(funcion_ptr), &args_c)
            }))
            .map_err(|_| CausaFfi::ErrorInvocacion {
                detalle: "pánico no controlado en frontera FFI".to_string(),
            })?;
            if retorno == "logico" {
                Valor::Logico(r != 0)
            } else {
                Valor::Entero(r)
            }
        }
        "real" => {
            let cif = libffi::middle::Cif::new(tipos_args.clone(), libffi::middle::Type::f64());
            {
                let r = catch_unwind(AssertUnwindSafe(|| unsafe {
                    cif.call::<f64>(libffi::low::CodePtr::from_ptr(funcion_ptr), &args_c)
                }))
                .map_err(|_| CausaFfi::ErrorInvocacion {
                    detalle: "pánico no controlado en frontera FFI".to_string(),
                })?;
                Valor::Real(r)
            }
        }
        "decimal" => {
            let cif = libffi::middle::Cif::new(tipos_args.clone(), libffi::middle::Type::f64());
            let r = catch_unwind(AssertUnwindSafe(|| unsafe {
                cif.call::<f64>(libffi::low::CodePtr::from_ptr(funcion_ptr), &args_c)
            }))
            .map_err(|_| CausaFfi::ErrorInvocacion {
                detalle: "pánico no controlado en frontera FFI".to_string(),
            })?;
            if !r.is_finite() {
                return Err(CausaFfi::ConversionFueraDeRango {
                    detalle: "el resultado no es representable en 'decimal'".to_string(),
                });
            }
            let escalado = r * FACTOR_ESCALA as f64;
            if escalado.abs() >= i128::MAX as f64 {
                return Err(CausaFfi::ConversionFueraDeRango {
                    detalle: "el resultado se desborda en 'decimal'".to_string(),
                });
            }
            Valor::Decimal(escalado as i128)
        }
        "texto" => {
            let cif = libffi::middle::Cif::new(tipos_args.clone(), libffi::middle::Type::pointer());
            let ptr = catch_unwind(AssertUnwindSafe(|| unsafe {
                cif.call::<*const c_char>(libffi::low::CodePtr::from_ptr(funcion_ptr), &args_c)
            }))
            .map_err(|_| CausaFfi::ErrorInvocacion {
                detalle: "pánico no controlado en frontera FFI".to_string(),
            })?;
            if ptr.is_null() {
                return Err(CausaFfi::ResultadoIncompatible {
                    detalle: "la función devolvió un puntero nulo con retorno 'texto'".to_string(),
                });
            }
            // Contrato: el puntero debe ser válido, NUL-terminated, y legible
            // durante esta conversión. BASILISCO copia el contenido y no libera
            // el puntero; el callee conserva la propiedad/gestión.
            let texto = unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned();
            Valor::Texto(texto)
        }
        "nulo" | "vacío" => {
            let cif = libffi::middle::Cif::new(tipos_args.clone(), libffi::middle::Type::void());
            catch_unwind(AssertUnwindSafe(|| unsafe {
                cif.call::<()>(libffi::low::CodePtr::from_ptr(funcion_ptr), &args_c)
            }))
            .map_err(|_| CausaFfi::ErrorInvocacion {
                detalle: "pánico no controlado en frontera FFI".to_string(),
            })?;
            Valor::Nulo
        }
        otro => {
            return Err(CausaFfi::FirmaInvalida {
                detalle: format!(
                    "tipo de retorno '{otro}' no aceptado (aceptados: entero, logico, real, decimal, texto, nulo)"
                ),
            });
        }
    };
    Ok(resultado)
}
