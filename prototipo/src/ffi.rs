//! Puente FFI básico (ABI C), compartido por el evaluador (Tree-Walk) y la VM.
//!
//! Carga bibliotecas dinámicas (`cargar_ffi`), busca símbolos y los invoca con
//! `libffi` (`llamar_ffi`), y los descarga (`descargar_ffi`). Los errores se
//! devuelven como `String` con el detalle; cada motor los traduce a su tipo
//! (`CausaError::FFIInvalido` en el Tree-Walk, `ErrorVm` en la VM).

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use crate::valor::{FACTOR_ESCALA, Valor};

/// Bibliotecas dinámicas cargadas, por id.
static LIBRERIAS_FFI: LazyLock<Mutex<HashMap<i64, Arc<libloading::Library>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Generador de ids de biblioteca.
static SIGUIENTE_ID_FFI: AtomicI64 = AtomicI64::new(0);

/// Carga una biblioteca dinámica y devuelve su id.
pub fn cargar_libreria(ruta: &str) -> Result<i64, String> {
    let libreria = unsafe { libloading::Library::new(ruta) }
        .map_err(|e| format!("no se pudo cargar '{ruta}': {e}"))?;
    let id = SIGUIENTE_ID_FFI.fetch_add(1, Ordering::SeqCst);
    LIBRERIAS_FFI.lock().unwrap().insert(id, Arc::new(libreria));
    Ok(id)
}

/// Descarga una biblioteca cargada por id.
pub fn descargar_libreria(id: i64) -> Result<(), String> {
    LIBRERIAS_FFI
        .lock()
        .unwrap()
        .remove(&id)
        .map(|_| ())
        .ok_or_else(|| format!("no hay biblioteca cargada con id {id}"))
}

/// Invoca un símbolo de una biblioteca cargada.
///
/// Los argumentos admitidos por la ABI C son `entero`, `real`, `decimal`
/// (convertido a `f64`) y `texto` (puntero C). El tipo de retorno admitido:
/// `entero` (por defecto), `logico`, `real`, `decimal`, `texto`, `nulo`/`vacío`.
pub fn llamar_funcion(
    id: i64,
    simbolo: &str,
    argumentos: &[Valor],
    retorno: &str,
) -> Result<Valor, String> {
    // Cargar y buscar el símbolo. El Arc mantiene viva la biblioteca
    // durante la llamada aunque otro hilo la descargue.
    let libreria = LIBRERIAS_FFI
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("no hay biblioteca cargada con id {id}"))?;
    let simbolo_ffi = unsafe { libreria.get::<*mut std::ffi::c_void>(simbolo.as_bytes()) }
        .map_err(|e| format!("símbolo '{simbolo}' no encontrado: {e}"))?;
    let funcion_ptr = *simbolo_ffi;

    // Tipos ABI de los argumentos y valores C correspondientes. Los
    // CString deben sobrevivir a la llamada completa.
    let mut tipos_args = Vec::with_capacity(argumentos.len());
    let mut valores_i64: Vec<i64> = Vec::with_capacity(argumentos.len());
    let mut valores_f64: Vec<f64> = Vec::with_capacity(argumentos.len());
    let mut valores_texto: Vec<std::ffi::CString> = Vec::with_capacity(argumentos.len());
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
            Valor::Texto(s) => match std::ffi::CString::new(s.clone()) {
                Ok(c) => {
                    tipos_args.push(libffi::middle::Type::pointer());
                    valores_texto.push(c);
                }
                Err(e) => {
                    return Err(format!(
                        "argumento {}: texto con byte NUL interno ({e})",
                        i + 1
                    ));
                }
            },
            otro => {
                return Err(format!(
                    "argumento {}: tipo '{}' no aceptado por la ABI C (aceptados: entero, real, decimal, texto)",
                    i + 1,
                    otro.nombre_tipo()
                ));
            }
        }
    }

    // Punteros C estables para argumentos de texto (la cadena vive
    // hasta el final de la llamada gracias a `valores_texto`).
    let textos_ptr: Vec<*const std::ffi::c_char> =
        valores_texto.iter().map(|c| c.as_ptr()).collect();

    // Construir los Arg's una sola vez, con cursores por tipo.
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
            _ => unreachable!("los tipos no aceptados se rechazan antes"),
        }
    }

    // La firma final depende del tipo indicado de retorno.
    let resultado: Valor = match retorno {
        "entero" | "logico" => {
            let cif = libffi::middle::Cif::new(tipos_args.clone(), libffi::middle::Type::i64());
            let r =
                unsafe { cif.call::<i64>(libffi::low::CodePtr::from_ptr(funcion_ptr), &args_c) };
            if retorno == "logico" {
                Valor::Logico(r != 0)
            } else {
                Valor::Entero(r)
            }
        }
        "real" => {
            let cif = libffi::middle::Cif::new(tipos_args.clone(), libffi::middle::Type::f64());
            Valor::Real(unsafe {
                cif.call::<f64>(libffi::low::CodePtr::from_ptr(funcion_ptr), &args_c)
            })
        }
        "decimal" => {
            let cif = libffi::middle::Cif::new(tipos_args.clone(), libffi::middle::Type::f64());
            let r =
                unsafe { cif.call::<f64>(libffi::low::CodePtr::from_ptr(funcion_ptr), &args_c) };
            if !r.is_finite() {
                return Err("el resultado no es representable en 'decimal'".to_string());
            }
            let escalado = r * FACTOR_ESCALA as f64;
            // `decimal` es un entero escalado (i128); un resultado
            // mayor que i128::MAX no es representable.
            if escalado.abs() >= i128::MAX as f64 {
                return Err("el resultado se desborda en 'decimal'".to_string());
            }
            Valor::Decimal(escalado as i128)
        }
        "texto" => {
            let cif = libffi::middle::Cif::new(tipos_args.clone(), libffi::middle::Type::pointer());
            let ptr = unsafe {
                cif.call::<*const std::ffi::c_char>(
                    libffi::low::CodePtr::from_ptr(funcion_ptr),
                    &args_c,
                )
            };
            if ptr.is_null() {
                return Err("la función devolvió un puntero nulo con retorno 'texto'".to_string());
            }
            let texto = unsafe { std::ffi::CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned();
            Valor::Texto(texto)
        }
        "nulo" | "vacío" => {
            let cif = libffi::middle::Cif::new(tipos_args.clone(), libffi::middle::Type::void());
            unsafe { cif.call::<()>(libffi::low::CodePtr::from_ptr(funcion_ptr), &args_c) };
            Valor::Nulo
        }
        otro => {
            return Err(format!(
                "tipo de retorno '{otro}' no aceptado (aceptados: entero, logico, real, decimal, texto, nulo)"
            ));
        }
    };
    Ok(resultado)
}
