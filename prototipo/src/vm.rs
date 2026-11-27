//! VM (máquina virtual) de BASILISCO.
//!
//! Intérprete directo de MIR: ejecuta funciones MIR bloques-a-bloques con
//! terminadores explícitos, reutilizando `Valor` (la semántica aritmética del
//! núcleo: ADR-0005, `lógico` ADR-0007) y sus métodos (`suma`, `resta`, ...).
//!
//! Alcance de esta versión de andamiaje:
//! - Cada llamada es un marco recursivo con límite de profundidad.
//! - Contador de pasos compartido (`limite_pasos`) para detectar bucles infinitos.
//! - Errores de ejecución en `ErrorVm`; la aritmética reutiliza `MotivoAritmetico`.
//! - Constantes MIR ↔ `Valor` por conversión directa; los tipos ricos (arreglos,
//!   listas, objetos) se materializan como `Valor` completos.
//!
//! Futuro: bytecode real, marcos planos (no recurrentes), llamadas nativas FFI,
//! proyecciones anidadas en `MirPlace`.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

use crate::mir::{
    BlockId, MirBinOp, MirConst, MirOperand, MirPlace, MirPrograma, MirProjection, MirRvalue,
    MirStmt, MirTerminator, MirUnOp,
};
use crate::valor::{FACTOR_ESCALA, Valor};

/// Error de ejecución de la VM.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorVm {
    /// La función llamada no existe en el programa.
    FuncionNoEncontrada(String),
    /// La función se llamó con menos argumentos que parámetros.
    ArgumentosInsuficientes { faltantes: usize },
    /// La función se llamó con una aridad incompatible.
    FirmaInvalida {
        funcion: String,
        esperado: usize,
        obtenido: usize,
    },
    /// Se alcanzó el límite de pasos (posible bucle infinito).
    LimitePasosExcedido,
    /// Se alcanzó el límite de profundidad de llamadas (posible recursión infinita).
    ProfundidadExcedida,
    /// Se llegó a `Unreachable` o a un salto fuera del mapa de bloques.
    BloqueInalcanzable,
    /// Error aritmético reutilizando los motivos del núcleo (ADR-0005 `D2`-`D5`).
    Aritmetico(crate::valor::MotivoAritmetico),
    /// Operación entre tipos incompatibles.
    TiposIncompatibles {
        izquierdo: &'static str,
        derecho: &'static str,
    },
    /// Operando no lógico en operador lógico/cortocircuito (paridad TW E1006).
    OperandoNoLogico {
        operador: &'static str,
        obtenido: &'static str,
    },
    /// Condición no lógica en estructura de control (paridad TW E1005).
    CondicionNoLogica { obtenido: &'static str },
    /// Se esperaba un tipo concreto (p. ej. índice entero).
    TipoEsperado {
        esperado: &'static str,
        encontrado: &'static str,
    },
    /// Acceso fuera de los límites de un arreglo/lista.
    IndiceFueraDeRango { indice: i64, tamano: usize },
    /// El campo no existe en el objeto.
    CampoInexistente { campo: String },
    /// Conversión de texto a número inválida.
    ConversionInvalido {
        valor: String,
        esperado: &'static str,
    },
    /// Asignación/conversión a tipo anotado incompatible (GAP-051, ADR-0005).
    /// Mensaje espejo de E1004 del Tree-Walk para paridad diferencial.
    AsignacionIncompatible { esperado: String, obtenido: String },
    /// Tipo de parámetro incompatible con anotación/inferencia (GAP-051, paridad TW).
    TipoParametroIncompatible {
        funcion: String,
        parametro: String,
        esperado: String,
        obtenido: String,
    },
    /// Retorno incompatible con la anotación de la función (paridad TW).
    RetornoIncompatible {
        funcion: String,
        esperado: String,
        obtenido: String,
    },
    /// Error del puente FFI (carga/llamada de biblioteca dinámica).
    FfiInvalido(crate::ffi::CausaFfi),
    /// Operación/capacidad no soportada (diagnósticos nominales, etc.).
    NoSoportado(String),
}

impl std::fmt::Display for ErrorVm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorVm::FuncionNoEncontrada(nombre) => write!(f, "función no encontrada: {nombre}"),
            ErrorVm::ArgumentosInsuficientes { faltantes } => {
                write!(f, "argumentos insuficientes ({faltantes} faltantes)")
            }
            ErrorVm::FirmaInvalida {
                funcion,
                esperado,
                obtenido,
            } => {
                write!(
                    f,
                    "firma inválida en {funcion}: esperado {esperado}, obtenido {obtenido}"
                )
            }
            ErrorVm::LimitePasosExcedido => write!(f, "límite de pasos excedido"),
            ErrorVm::ProfundidadExcedida => write!(f, "límite de profundidad excedido"),
            ErrorVm::BloqueInalcanzable => write!(f, "bloque inalcanzable"),
            ErrorVm::Aritmetico(m) => write!(f, "error aritmético: {m}"),
            ErrorVm::TiposIncompatibles { izquierdo, derecho } => {
                write!(f, "tipos incompatibles: {izquierdo} y {derecho}")
            }
            ErrorVm::OperandoNoLogico { operador, obtenido } => {
                write!(
                    f,
                    "el operador «{operador}» requiere operandos lógicos, se obtuvo «{obtenido}»"
                )
            }
            ErrorVm::CondicionNoLogica { obtenido } => {
                write!(f, "la condición debe ser lógica, se obtuvo «{obtenido}»")
            }
            ErrorVm::TipoEsperado {
                esperado,
                encontrado,
            } => {
                write!(f, "se esperaba «{esperado}», se encontró «{encontrado}»")
            }
            ErrorVm::IndiceFueraDeRango { indice, tamano } => {
                write!(f, "índice {indice} fuera de rango (tamaño {tamano})")
            }
            ErrorVm::CampoInexistente { campo } => write!(f, "campo inexistente: {campo}"),
            ErrorVm::ConversionInvalido { valor, esperado } => {
                write!(f, "el texto «{valor}» no es un «{esperado}» válido")
            }
            ErrorVm::AsignacionIncompatible { esperado, obtenido } => write!(
                f,
                "asignación incompatible: se esperaba {esperado} y se obtuvo {obtenido}"
            ),
            ErrorVm::TipoParametroIncompatible {
                funcion,
                parametro,
                esperado,
                obtenido,
            } => write!(
                f,
                "parámetro '{parametro}' de '{funcion}': se esperaba {esperado} y se obtuvo {obtenido}"
            ),
            ErrorVm::RetornoIncompatible {
                funcion,
                esperado,
                obtenido,
            } => write!(
                f,
                "retorno de '{funcion}': se esperaba {esperado} y se obtuvo {obtenido}"
            ),
            ErrorVm::FfiInvalido(causa) => write!(f, "{causa}"),
            ErrorVm::NoSoportado(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for ErrorVm {}

/// Límite por defecto de pasos de ejecución (sentencias + terminadores).
pub const LIMITE_PASOS_DEFAULT: usize = 1_000_000;
/// Límite por defecto de profundidad de llamadas.
pub const LIMITE_PROFUNDIDAD_DEFAULT: usize = 1_000;

/// Tipo de una función nativa registrada en la VM: recibe los argumentos
/// evaluados y devuelve un `Valor` (o un `ErrorVm`).
pub type FuncionNativa = std::sync::Arc<dyn Fn(&[Valor]) -> Result<Valor, ErrorVm> + Send + Sync>;

/// Punto de parada para depuración (archivo, línea 1-based).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BreakpointVm {
    pub archivo: String,
    pub linea: usize,
}

/// Modo de paso del depurador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModoPasoVm {
    #[default]
    Ninguno,
    StepIn,
    StepOver {
        profundidad: usize,
    },
    StepOut {
        profundidad: usize,
    },
}

/// Razón de detención de la VM depurable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotivoParadaVm {
    Breakpoint { archivo: String, linea: usize },
    Step,
    Pause,
    Entry,
    Terminado,
    Error(String),
}

/// Snapshot de frame para DAP/UI.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameVm {
    pub funcion: String,
    pub archivo: Option<String>,
    pub linea: Option<usize>,
    pub columna: Option<usize>,
}

/// Snapshot de variable.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableVm {
    pub nombre: String,
    pub valor: String,
    pub tipo: String,
}

/// Estado de depuración aislado por sesión (instancia VM).
#[derive(Debug, Clone, Default)]
pub struct SesionDepuracionVm {
    pub breakpoints: std::collections::HashSet<BreakpointVm>,
    pub modo_paso: ModoPasoVm,
    pub pausa_solicitada: bool,
    pub frames: Vec<FrameVm>,
    pub ultima_linea: Option<(String, usize)>,
}

/// Máquina virtual. Interpreta un `MirPrograma`.
pub struct MaquinaVirtual {
    programa: MirPrograma,
    /// Índice `nombre -> posición en programa.funciones` (la primera gana).
    indice: HashMap<String, usize>,
    /// Funciones nativas (FFI / integradas) registradas en la VM.
    nativos: HashMap<String, FuncionNativa>,
    /// Salida capturada de `mostrar`/`mostrar_sin_salto` (biblioteca estándar).
    salida: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// Valores de las variables globales (inicializados por `__init`).
    globals: Vec<Valor>,
    /// Tipos inferidos por función (para funciones sin anotación de parámetros).
    tipos_inferidos: HashMap<String, Vec<Valor>>,
    /// Funciones sin sentencia `retornar` con valor.
    funciones_sin_retorno: std::collections::HashSet<String>,
    /// Locales de la función actual que provienen de una llamada sin retorno.
    sin_retorno_locals: std::collections::HashSet<usize>,
    /// Si ya se ejecutó `__init`.
    inicializado: bool,
    /// Pila de regiones try activas.
    try_stack: Vec<(BlockId, BlockId, Option<usize>)>,
    pasos: usize,
    limite_pasos: usize,
    profundidad: usize,
    limite_profundidad: usize,
    /// Si la traza del depurador está activa.
    traza: bool,
    /// Salida de la traza del depurador.
    traza_salida: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// Mapa opcional de spans para depuración (func_idx, block_idx, stmt_idx; TERM=usize::MAX) -> (archivo, linea, columna).
    pub debug_spans: HashMap<(usize, usize, usize), (String, usize, usize)>,
    /// Nombres de globals para inspección.
    pub debug_global_names: Vec<String>,
    /// Sesión de depuración aislada por instancia VM.
    pub debug: SesionDepuracionVm,
}

impl MaquinaVirtual {
    /// Crea una VM con los límites por defecto.
    pub fn nuevo(programa: MirPrograma) -> Self {
        MaquinaVirtual::con_limites(programa, LIMITE_PASOS_DEFAULT, LIMITE_PROFUNDIDAD_DEFAULT)
    }

    /// Crea una VM con límites personalizados (útil en tests).
    pub fn con_limites(
        programa: MirPrograma,
        limite_pasos: usize,
        limite_profundidad: usize,
    ) -> Self {
        let n_globals = programa.globals.len();
        let mut indice = HashMap::new();
        for (i, f) in programa.funciones.iter().enumerate() {
            indice.entry(f.nombre.clone()).or_insert(i);
        }
        let funciones_sin_retorno: std::collections::HashSet<String> = programa
            .funciones
            .iter()
            .filter(|f| {
                !f.bloques
                    .iter()
                    .any(|b| matches!(b.terminator, MirTerminator::Return(Some(_))))
            })
            .map(|f| f.nombre.clone())
            .collect();
        let debug_global_names = programa
            .globals
            .iter()
            .enumerate()
            .map(|(i, _)| format!("g{i}"))
            .collect();
        MaquinaVirtual {
            programa,
            indice,
            nativos: HashMap::new(),
            salida: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            globals: vec![Valor::Nulo; n_globals],
            tipos_inferidos: HashMap::new(),
            funciones_sin_retorno,
            sin_retorno_locals: std::collections::HashSet::new(),
            inicializado: false,
            try_stack: Vec::new(),
            pasos: 0,
            limite_pasos,
            profundidad: 0,
            limite_profundidad,
            traza: false,
            traza_salida: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            debug_spans: HashMap::new(),
            debug_global_names,
            debug: SesionDepuracionVm::default(),
        }
    }

    /// Número de funciones del programa.
    pub fn numero_funciones(&self) -> usize {
        self.programa.funciones.len()
    }

    /// Acceso de solo lectura al programa MIR.
    pub fn programa(&self) -> &MirPrograma {
        &self.programa
    }

    /// Índice de la función por nombre (`f_{simbodo}` según el lowering).
    pub fn indice_de(&self, nombre: &str) -> Option<usize> {
        self.indice.get(nombre).copied()
    }

    /// Activa la traza del depurador.
    pub fn activar_traza(&mut self) {
        self.traza = true;
    }

    /// Devuelve la traza capturada hasta ahora.
    pub fn traza(&self) -> Vec<String> {
        self.traza_salida.lock().unwrap().clone()
    }

    /// Registra una función nativa (FFI / integrada) bajo `nombre`.
    /// Tiene prioridad menor que una función MIR del programa con el mismo
    /// nombre (la primera gana en el intérprete).
    pub fn registrar_nativo(&mut self, nombre: impl Into<String>, f: FuncionNativa) {
        self.nativos.entry(nombre.into()).or_insert(f);
    }

    /// Registra la biblioteca estándar mínima (mostrar, mostrar_sin_salto, longitud, etc.).
    pub fn registrar_stdlib(&mut self) {
        // Intrínseco interno: valida operando lógico para cortocircuito Y/O (paridad TW).
        self.registrar_nativo(
            "__bsk_assert_logico",
            std::sync::Arc::new(|args: &[Valor]| {
                let v = args.first().cloned().unwrap_or(Valor::Nulo);
                let op = args.get(1).and_then(|x| {
                    if let Valor::Texto(s) = x {
                        Some(s.as_str())
                    } else {
                        None
                    }
                });
                let operador: &'static str = match op {
                    Some("o") => "o",
                    _ => "y",
                };
                match v {
                    Valor::Logico(_) => Ok(v),
                    _ => Err(ErrorVm::OperandoNoLogico {
                        operador,
                        obtenido: v.nombre_tipo(),
                    }),
                }
            }),
        );
        let salida = std::sync::Arc::clone(&self.salida);
        self.registrar_nativo(
            "mostrar",
            std::sync::Arc::new(move |args: &[Valor]| {
                let texto = args.first().map(|v| v.to_string()).unwrap_or_default();
                salida.lock().unwrap().push(format!("{texto}\n"));
                println!("{texto}");
                Ok(Valor::Nulo)
            }),
        );
        let salida = std::sync::Arc::clone(&self.salida);
        self.registrar_nativo(
            "mostrar_sin_salto",
            std::sync::Arc::new(move |args: &[Valor]| {
                let texto = args.first().map(|v| v.to_string()).unwrap_or_default();
                salida.lock().unwrap().push(texto.clone());
                print!("{texto}");
                Ok(Valor::Nulo)
            }),
        );
        self.registrar_nativo(
            "leer",
            std::sync::Arc::new(|_args: &[Valor]| {
                let mut entrada = String::new();
                match std::io::stdin().lock().read_line(&mut entrada) {
                    Ok(0) => Ok(Valor::Texto(String::new())),
                    Ok(_) => {
                        if entrada.ends_with('\n') {
                            entrada.pop();
                        }
                        if entrada.ends_with('\r') {
                            entrada.pop();
                        }
                        Ok(Valor::Texto(entrada))
                    }
                    Err(e) => Err(ErrorVm::FfiInvalido(
                        crate::ffi::CausaFfi::ErrorInvocacion {
                            detalle: format!("<stdin>: {e}"),
                        },
                    )),
                }
            }),
        );
        self.registrar_nativo(
            "leer_entero",
            std::sync::Arc::new(|_args: &[Valor]| {
                let mut entrada = String::new();
                std::io::stdin()
                    .lock()
                    .read_line(&mut entrada)
                    .map_err(|e| {
                        ErrorVm::FfiInvalido(crate::ffi::CausaFfi::ErrorInvocacion {
                            detalle: format!("<stdin>: {e}"),
                        })
                    })?;
                if entrada.ends_with('\n') {
                    entrada.pop();
                }
                if entrada.ends_with('\r') {
                    entrada.pop();
                }
                let texto = entrada.trim();
                match texto.parse::<i64>() {
                    Ok(v) => Ok(Valor::Entero(v)),
                    Err(_) => Err(ErrorVm::ConversionInvalido {
                        valor: texto.to_string(),
                        esperado: "entero",
                    }),
                }
            }),
        );
        self.registrar_nativo(
            "leer_decimal",
            std::sync::Arc::new(|_args: &[Valor]| {
                let mut entrada = String::new();
                std::io::stdin()
                    .lock()
                    .read_line(&mut entrada)
                    .map_err(|e| {
                        ErrorVm::FfiInvalido(crate::ffi::CausaFfi::ErrorInvocacion {
                            detalle: format!("<stdin>: {e}"),
                        })
                    })?;
                if entrada.ends_with('\n') {
                    entrada.pop();
                }
                if entrada.ends_with('\r') {
                    entrada.pop();
                }
                let texto = entrada.trim();
                match Valor::decimal_desde_lexema(texto) {
                    Ok(v) => Ok(v),
                    Err(_) => Err(ErrorVm::ConversionInvalido {
                        valor: texto.to_string(),
                        esperado: "decimal",
                    }),
                }
            }),
        );
        self.registrar_nativo(
            "leer_real",
            std::sync::Arc::new(|_args: &[Valor]| {
                let mut entrada = String::new();
                std::io::stdin()
                    .lock()
                    .read_line(&mut entrada)
                    .map_err(|e| {
                        ErrorVm::FfiInvalido(crate::ffi::CausaFfi::ErrorInvocacion {
                            detalle: format!("<stdin>: {e}"),
                        })
                    })?;
                if entrada.ends_with('\n') {
                    entrada.pop();
                }
                if entrada.ends_with('\r') {
                    entrada.pop();
                }
                let texto = entrada.trim();
                match texto.parse::<f64>() {
                    Ok(v) => Ok(Valor::Real(v)),
                    Err(_) => Err(ErrorVm::ConversionInvalido {
                        valor: texto.to_string(),
                        esperado: "real",
                    }),
                }
            }),
        );
        self.registrar_nativo(
            "ahora",
            std::sync::Arc::new(|_args: &[Valor]| Ok(crate::fechas::ahora())),
        );
        self.registrar_nativo(
            "fecha_actual",
            std::sync::Arc::new(|_args: &[Valor]| Ok(crate::fechas::fecha_actual())),
        );
        self.registrar_nativo(
            "formatear_fecha",
            std::sync::Arc::new(|args: &[Valor]| {
                let ms = match args.first() {
                    Some(Valor::Entero(v)) => *v,
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "entero",
                            encontrado: "otro",
                        });
                    }
                };
                let formato = match args.get(1) {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto",
                            encontrado: "otro",
                        });
                    }
                };
                Ok(crate::fechas::formatear_fecha(ms, &formato))
            }),
        );
        self.registrar_nativo(
            "http_get",
            std::sync::Arc::new(|args: &[Valor]| {
                let url = match args.first() {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto",
                            encontrado: "otro",
                        });
                    }
                };
                crate::http::http_get(&url).map_err(|d| {
                    ErrorVm::FfiInvalido(crate::ffi::CausaFfi::ErrorInvocacion { detalle: d })
                })
            }),
        );
        self.registrar_nativo(
            "http_post",
            std::sync::Arc::new(|args: &[Valor]| {
                let url = match args.first() {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto",
                            encontrado: "otro",
                        });
                    }
                };
                let cuerpo = match args.get(1) {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto",
                            encontrado: "otro",
                        });
                    }
                };
                crate::http::http_post(&url, &cuerpo).map_err(|d| {
                    ErrorVm::FfiInvalido(crate::ffi::CausaFfi::ErrorInvocacion { detalle: d })
                })
            }),
        );
        self.registrar_nativo(
            "longitud",
            std::sync::Arc::new(|args: &[Valor]| {
                let n = match args.first() {
                    Some(Valor::Texto(s)) => s.chars().count() as i64,
                    Some(Valor::Arreglo(e)) | Some(Valor::Lista(e)) => e.len() as i64,
                    Some(Valor::Objeto(m)) => m.len() as i64,
                    Some(_) => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto, arreglo, lista u objeto",
                            encontrado: "otro",
                        });
                    }
                    None => 0,
                };
                Ok(Valor::Entero(n))
            }),
        );
        self.registrar_nativo(
            "abs",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Entero(n)) => Ok(Valor::Entero(n.saturating_abs())),
                Some(Valor::Real(x)) => Ok(Valor::Real(x.abs())),
                Some(Valor::Decimal(m)) => {
                    let abs = if *m < 0 { -m } else { *m };
                    Ok(Valor::Decimal(abs))
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "entero, decimal o real",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "min",
            std::sync::Arc::new(|args: &[Valor]| match (args.first(), args.get(1)) {
                (Some(Valor::Entero(a)), Some(Valor::Entero(b))) => Ok(Valor::Entero((*a).min(*b))),
                (Some(Valor::Real(a)), Some(Valor::Real(b))) => Ok(Valor::Real(a.min(*b))),
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "entero o real",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "max",
            std::sync::Arc::new(|args: &[Valor]| match (args.first(), args.get(1)) {
                (Some(Valor::Entero(a)), Some(Valor::Entero(b))) => Ok(Valor::Entero((*a).max(*b))),
                (Some(Valor::Real(a)), Some(Valor::Real(b))) => Ok(Valor::Real(a.max(*b))),
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "entero o real",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "mayusculas",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Texto(s)) => Ok(Valor::Texto(s.to_uppercase())),
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "minusculas",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Texto(s)) => Ok(Valor::Texto(s.to_lowercase())),
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "es_vacio",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Texto(s)) => Ok(Valor::Logico(s.is_empty())),
                Some(Valor::Arreglo(e)) | Some(Valor::Lista(e)) => Ok(Valor::Logico(e.is_empty())),
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto, arreglo o lista",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "contiene",
            std::sync::Arc::new(|args: &[Valor]| match (args.first(), args.get(1)) {
                (Some(Valor::Texto(s)), Some(Valor::Texto(sub))) => {
                    Ok(Valor::Logico(s.contains(sub.as_str())))
                }
                (Some(Valor::Texto(_)), _) => Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                }),
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "leer_archivo",
            std::sync::Arc::new(|args: &[Valor]| {
                let ruta = match args.first() {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto (ruta)",
                            encontrado: "otro",
                        });
                    }
                };
                std::fs::read_to_string(&ruta)
                    .map(Valor::Texto)
                    .map_err(|_| ErrorVm::TipoEsperado {
                        esperado: "archivo legible",
                        encontrado: "error de lectura",
                    })
            }),
        );
        self.registrar_nativo(
            "escribir_archivo",
            std::sync::Arc::new(|args: &[Valor]| {
                let ruta = match args.first() {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto (ruta)",
                            encontrado: "otro",
                        });
                    }
                };
                let contenido = args.get(1).map(|v| v.to_string()).unwrap_or_default();
                std::fs::write(&ruta, contenido).map_err(|_| ErrorVm::TipoEsperado {
                    esperado: "ruta escribible",
                    encontrado: "error de escritura",
                })?;
                Ok(Valor::Nulo)
            }),
        );
        self.registrar_nativo(
            "existe_archivo",
            std::sync::Arc::new(|args: &[Valor]| {
                let ruta = match args.first() {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto (ruta)",
                            encontrado: "otro",
                        });
                    }
                };
                Ok(Valor::Logico(std::path::Path::new(&ruta).exists()))
            }),
        );
        self.registrar_nativo(
            "json_parsear",
            std::sync::Arc::new(|args: &[Valor]| {
                let texto = match args.first() {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto",
                            encontrado: "otro",
                        });
                    }
                };
                let jv: serde_json::Value =
                    serde_json::from_str(&texto).map_err(|_| ErrorVm::TipoEsperado {
                        esperado: "JSON válido",
                        encontrado: "JSON inválido",
                    })?;
                Ok(json_a_valor(jv))
            }),
        );
        self.registrar_nativo(
            "json_serializar",
            std::sync::Arc::new(|args: &[Valor]| {
                let jv =
                    valor_a_json(&args.first().cloned().unwrap_or(Valor::Nulo)).map_err(|_| {
                        ErrorVm::TipoEsperado {
                            esperado: "valor serializable en JSON",
                            encontrado: "otro",
                        }
                    })?;
                let s = serde_json::to_string(&jv).map_err(|_| ErrorVm::TipoEsperado {
                    esperado: "JSON serializable",
                    encontrado: "otro",
                })?;
                Ok(Valor::Texto(s))
            }),
        );
        self.registrar_nativo(
            "json_obtener",
            std::sync::Arc::new(|args: &[Valor]| {
                let mapa = match args.first() {
                    Some(Valor::Objeto(m)) => m.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "objeto",
                            encontrado: "otro",
                        });
                    }
                };
                let clave = match args.get(1) {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto",
                            encontrado: "otro",
                        });
                    }
                };
                Ok(mapa.get(&clave).cloned().unwrap_or(Valor::Nulo))
            }),
        );
        self.registrar_nativo(
            "termina_con",
            std::sync::Arc::new(|args: &[Valor]| match (args.first(), args.get(1)) {
                (Some(Valor::Texto(s)), Some(Valor::Texto(suf))) => {
                    Ok(Valor::Logico(s.ends_with(suf.as_str())))
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "recortar",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Texto(s)) => Ok(Valor::Texto(s.trim().to_string())),
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "reemplazar",
            std::sync::Arc::new(
                |args: &[Valor]| match (args.first(), args.get(1), args.get(2)) {
                    (Some(Valor::Texto(s)), Some(Valor::Texto(from)), Some(Valor::Texto(to))) => {
                        Ok(Valor::Texto(s.replace(from, to)))
                    }
                    _ => Err(ErrorVm::TipoEsperado {
                        esperado: "texto",
                        encontrado: "otro",
                    }),
                },
            ),
        );
        self.registrar_nativo(
            "subcadena",
            std::sync::Arc::new(
                |args: &[Valor]| match (args.first(), args.get(1), args.get(2)) {
                    (
                        Some(Valor::Texto(s)),
                        Some(Valor::Entero(inicio)),
                        Some(Valor::Entero(fin)),
                    ) => {
                        let chars: Vec<char> = s.chars().collect();
                        let (i, f) = (*inicio, *fin);
                        if i < 0 || f < 0 || i > f || f as usize > chars.len() {
                            return Err(ErrorVm::TipoEsperado {
                                esperado: "rango válido de subcadena",
                                encontrado: "rango inválido",
                            });
                        }
                        Ok(Valor::Texto(
                            chars[i as usize..f as usize].iter().collect::<String>(),
                        ))
                    }
                    _ => Err(ErrorVm::TipoEsperado {
                        esperado: "texto, entero, entero",
                        encontrado: "otro",
                    }),
                },
            ),
        );
        self.registrar_nativo(
            "repetir",
            std::sync::Arc::new(|args: &[Valor]| match (args.first(), args.get(1)) {
                (Some(Valor::Texto(s)), Some(Valor::Entero(n))) => {
                    if *n < 0 {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "entero no negativo",
                            encontrado: "entero negativo",
                        });
                    }
                    Ok(Valor::Texto(s.repeat(*n as usize)))
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto, entero",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "dividir",
            std::sync::Arc::new(|args: &[Valor]| match (args.first(), args.get(1)) {
                (Some(Valor::Texto(s)), Some(Valor::Texto(sep))) => {
                    if sep.is_empty() {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "separador no vacío",
                            encontrado: "texto vacío",
                        });
                    }
                    let partes: Vec<Valor> =
                        s.split(sep).map(|p| Valor::Texto(p.to_string())).collect();
                    Ok(Valor::Lista(partes))
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto, texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "unir",
            std::sync::Arc::new(|args: &[Valor]| match (args.first(), args.get(1)) {
                (Some(Valor::Lista(items)), Some(Valor::Texto(sep))) => {
                    let mut partes = Vec::with_capacity(items.len());
                    for item in items {
                        match item {
                            Valor::Texto(s) => partes.push(s.clone()),
                            otro => {
                                return Err(ErrorVm::TipoEsperado {
                                    esperado: "texto",
                                    encontrado: otro.nombre_tipo(),
                                });
                            }
                        }
                    }
                    Ok(Valor::Texto(partes.join(sep)))
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "lista<texto>, texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "potencia",
            std::sync::Arc::new(|args: &[Valor]| match (args.first(), args.get(1)) {
                (Some(Valor::Entero(base)), Some(Valor::Entero(exp))) if *exp >= 0 => {
                    match base.checked_pow(*exp as u32) {
                        Some(r) => Ok(Valor::Entero(r)),
                        None => Err(ErrorVm::Aritmetico(
                            crate::valor::MotivoAritmetico::DesbordamientoEntero,
                        )),
                    }
                }
                (Some(b), Some(e)) => {
                    let base = match b {
                        Valor::Entero(x) => *x as f64,
                        Valor::Decimal(x) => *x as f64 / crate::valor::FACTOR_ESCALA as f64,
                        Valor::Real(x) => *x,
                        _ => {
                            return Err(ErrorVm::TipoEsperado {
                                esperado: "número",
                                encontrado: "otro",
                            });
                        }
                    };
                    let exp = match e {
                        Valor::Entero(x) => *x as f64,
                        Valor::Decimal(x) => *x as f64 / crate::valor::FACTOR_ESCALA as f64,
                        Valor::Real(x) => *x,
                        _ => {
                            return Err(ErrorVm::TipoEsperado {
                                esperado: "número",
                                encontrado: "otro",
                            });
                        }
                    };
                    Ok(Valor::Real(base.powf(exp)))
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "número, número",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "raiz",
            std::sync::Arc::new(|args: &[Valor]| {
                let valor = match args.first() {
                    Some(Valor::Entero(x)) => *x as f64,
                    Some(Valor::Decimal(x)) => *x as f64 / crate::valor::FACTOR_ESCALA as f64,
                    Some(Valor::Real(x)) => *x,
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "número",
                            encontrado: "otro",
                        });
                    }
                };
                Ok(Valor::Real(valor.sqrt()))
            }),
        );
        for nombre in ["piso", "techo", "redondear"] {
            let nombre_owned = nombre.to_string();
            self.registrar_nativo(
                nombre,
                std::sync::Arc::new(move |args: &[Valor]| {
                    let nombre = nombre_owned.as_str();
                    if args.len() != 1 {
                        return Err(ErrorVm::FirmaInvalida {
                            funcion: nombre.to_string(),
                            esperado: 1,
                            obtenido: args.len(),
                        });
                    }
                    let n: i64 = match args.first() {
                        Some(Valor::Entero(v)) => *v,
                        Some(Valor::Decimal(v)) => {
                            let s = crate::valor::FACTOR_ESCALA;
                            let base = *v / s;
                            let rem = *v % s;
                            let res = match nombre {
                                "piso" => {
                                    if rem != 0 && *v < 0 {
                                        base - 1
                                    } else {
                                        base
                                    }
                                }
                                "techo" => {
                                    if rem != 0 && *v > 0 {
                                        base + 1
                                    } else {
                                        base
                                    }
                                }
                                _ => {
                                    // half away from zero
                                    let sign: i128 = if *v < 0 { -1 } else { 1 };
                                    let av = v.abs();
                                    let abase = av / s;
                                    let arem = av % s;
                                    let arred = if arem * 2 >= s { abase + 1 } else { abase };
                                    sign * arred
                                }
                            };
                            i64::try_from(res).map_err(|_| {
                                ErrorVm::Aritmetico(
                                    crate::valor::MotivoAritmetico::DesbordamientoEntero,
                                )
                            })?
                        }
                        Some(Valor::Real(v)) => {
                            if !v.is_finite() {
                                return Err(ErrorVm::Aritmetico(
                                    crate::valor::MotivoAritmetico::DesbordamientoEntero,
                                ));
                            }
                            let n = match nombre {
                                "piso" => v.floor(),
                                "techo" => v.ceil(),
                                _ => v.round(), // half away from zero
                            };
                            // Intervalo válido i64: [-2^63, 2^63). No usar i64::MAX as f64 (redondea a 2^63).
                            const LIMITE_SUP_EXCLUSIVO: f64 = 9223372036854775808.0_f64; // 2^63
                            if n < i64::MIN as f64 || n >= LIMITE_SUP_EXCLUSIVO {
                                return Err(ErrorVm::Aritmetico(
                                    crate::valor::MotivoAritmetico::DesbordamientoEntero,
                                ));
                            }
                            n as i64
                        }
                        Some(otro) => {
                            return Err(ErrorVm::AsignacionIncompatible {
                                esperado: "número".to_string(),
                                obtenido: otro.nombre_tipo().to_string(),
                            });
                        }
                        None => {
                            return Err(ErrorVm::ArgumentosInsuficientes { faltantes: 1 });
                        }
                    };
                    Ok(Valor::Entero(n))
                }),
            );
        }
        self.registrar_nativo(
            "posicion",
            std::sync::Arc::new(|args: &[Valor]| {
                if args.len() != 2 {
                    return Err(ErrorVm::FirmaInvalida {
                        funcion: "posicion".to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Valor::Texto(t), Valor::Texto(sub)) => {
                        let idx = t
                            .find(sub.as_str())
                            .map(|b| t[..b].chars().count() as i64)
                            .unwrap_or(-1);
                        Ok(Valor::Entero(idx))
                    }
                    (Valor::Arreglo(xs), v) | (Valor::Lista(xs), v) => {
                        let idx = xs
                            .iter()
                            .position(|x| x == v)
                            .map(|i| i as i64)
                            .unwrap_or(-1);
                        Ok(Valor::Entero(idx))
                    }
                    (otro, _) => Err(ErrorVm::AsignacionIncompatible {
                        esperado: "texto, arreglo o lista".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    }),
                }
            }),
        );
        self.registrar_nativo(
            "invertir",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Arreglo(v)) => {
                    let mut n = v.clone();
                    n.reverse();
                    Ok(Valor::Arreglo(n))
                }
                Some(Valor::Lista(v)) => {
                    let mut n = v.clone();
                    n.reverse();
                    Ok(Valor::Lista(n))
                }
                Some(Valor::Texto(t)) => {
                    let mut c: Vec<char> = t.chars().collect();
                    c.reverse();
                    Ok(Valor::Texto(c.into_iter().collect()))
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "arreglo, lista o texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "ordenar",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Arreglo(v)) => {
                    let mut n = v.clone();
                    n.sort_by(ordenar_valor);
                    Ok(Valor::Arreglo(n))
                }
                Some(Valor::Lista(v)) => {
                    let mut n = v.clone();
                    n.sort_by(ordenar_valor);
                    Ok(Valor::Lista(n))
                }
                Some(Valor::Texto(t)) => {
                    let mut c: Vec<char> = t.chars().collect();
                    c.sort();
                    Ok(Valor::Texto(c.into_iter().collect()))
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "arreglo, lista o texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "empieza_con",
            std::sync::Arc::new(|args: &[Valor]| match (args.first(), args.get(1)) {
                (Some(Valor::Texto(s)), Some(Valor::Texto(prefijo))) => {
                    Ok(Valor::Logico(s.starts_with(prefijo.as_str())))
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "a_entero",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Texto(t)) => match t.trim().parse::<i64>() {
                    Ok(v) => Ok(Valor::Entero(v)),
                    Err(_) => Err(ErrorVm::ConversionInvalido {
                        valor: t.clone(),
                        esperado: "entero",
                    }),
                },
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "a_decimal",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Texto(t)) => match Valor::decimal_desde_lexema(t.trim()) {
                    Ok(v) => Ok(v),
                    Err(_) => Err(ErrorVm::ConversionInvalido {
                        valor: t.clone(),
                        esperado: "decimal",
                    }),
                },
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "a_real",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Texto(t)) => match t.trim().parse::<f64>() {
                    Ok(v) => Ok(Valor::Real(v)),
                    Err(_) => Err(ErrorVm::ConversionInvalido {
                        valor: t.clone(),
                        esperado: "real",
                    }),
                },
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "texto",
            std::sync::Arc::new(|args: &[Valor]| {
                if args.len() != 1 {
                    return Err(ErrorVm::FirmaInvalida {
                        funcion: "texto".to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    });
                }
                Ok(Valor::Texto(args[0].to_string()))
            }),
        );
        self.registrar_nativo(
            "agregar",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Arreglo(v)) => {
                    let mut n = v.clone();
                    n.push(args.get(1).cloned().unwrap_or(Valor::Nulo));
                    Ok(Valor::Arreglo(n))
                }
                Some(Valor::Lista(v)) => {
                    let mut n = v.clone();
                    n.push(args.get(1).cloned().unwrap_or(Valor::Nulo));
                    Ok(Valor::Lista(n))
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "arreglo o lista",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "abrir",
            std::sync::Arc::new(|args: &[Valor]| {
                let (ruta, modo) = match (args.first(), args.get(1)) {
                    (Some(Valor::Texto(r)), Some(Valor::Texto(m))) => (r.clone(), m.clone()),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto, texto",
                            encontrado: "otro",
                        });
                    }
                };
                let file_result = match modo.as_str() {
                    "leer" => File::open(&ruta),
                    "escribir" => OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&ruta),
                    "agregar" => OpenOptions::new().append(true).create(true).open(&ruta),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "modo válido (leer, escribir, agregar)",
                            encontrado: "modo inválido",
                        });
                    }
                };
                let archivo = file_result.map_err(|_| ErrorVm::TipoEsperado {
                    esperado: "archivo accesible",
                    encontrado: "error de apertura",
                })?;
                let (lector, escritor) = match modo.as_str() {
                    "leer" => (Some(BufReader::new(archivo)), None),
                    "escribir" | "agregar" => (None, Some(BufWriter::new(archivo))),
                    _ => (None, None),
                };
                Ok(Valor::Archivo(std::sync::Arc::new(std::sync::Mutex::new(
                    crate::valor::ArchivoInterno {
                        ruta,
                        modo,
                        cerrado: false,
                        lector,
                        escritor,
                    },
                ))))
            }),
        );
        self.registrar_nativo(
            "leer_linea",
            std::sync::Arc::new(|args: &[Valor]| {
                let Some(Valor::Archivo(arc)) = args.first() else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "archivo",
                        encontrado: "otro",
                    });
                };
                let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                if guard.cerrado {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "archivo abierto",
                        encontrado: "archivo ya cerrado",
                    });
                }
                let lector = guard.lector.as_mut().ok_or(ErrorVm::TipoEsperado {
                    esperado: "modo lectura",
                    encontrado: "archivo no abierto en modo lectura",
                })?;
                let mut linea_texto = String::new();
                let leidos =
                    lector
                        .read_line(&mut linea_texto)
                        .map_err(|_| ErrorVm::TipoEsperado {
                            esperado: "lectura exitosa",
                            encontrado: "error de lectura",
                        })?;
                if leidos == 0 {
                    return Ok(Valor::Texto(String::new()));
                }
                if linea_texto.ends_with('\n') {
                    linea_texto.pop();
                }
                if linea_texto.ends_with('\r') {
                    linea_texto.pop();
                }
                Ok(Valor::Texto(linea_texto))
            }),
        );
        self.registrar_nativo(
            "leer_todo",
            std::sync::Arc::new(|args: &[Valor]| {
                let Some(Valor::Archivo(arc)) = args.first() else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "archivo",
                        encontrado: "otro",
                    });
                };
                let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                if guard.cerrado {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "archivo abierto",
                        encontrado: "archivo ya cerrado",
                    });
                }
                let lector = guard.lector.as_mut().ok_or(ErrorVm::TipoEsperado {
                    esperado: "modo lectura",
                    encontrado: "archivo no abierto en modo lectura",
                })?;
                let mut contenido = String::new();
                lector
                    .read_to_string(&mut contenido)
                    .map_err(|_| ErrorVm::TipoEsperado {
                        esperado: "lectura exitosa",
                        encontrado: "error de lectura",
                    })?;
                Ok(Valor::Texto(contenido))
            }),
        );
        self.registrar_nativo(
            "escribir",
            std::sync::Arc::new(|args: &[Valor]| {
                let (Some(Valor::Archivo(arc)), valor) = (args.first(), args.get(1)) else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "archivo, texto",
                        encontrado: "otro",
                    });
                };
                let Some(Valor::Texto(t)) = valor else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "archivo, texto",
                        encontrado: "otro",
                    });
                };
                let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                if guard.cerrado {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "archivo abierto",
                        encontrado: "archivo ya cerrado",
                    });
                }
                let escritor = guard.escritor.as_mut().ok_or(ErrorVm::TipoEsperado {
                    esperado: "modo escritura",
                    encontrado: "archivo no abierto en modo escritura",
                })?;
                escritor
                    .write_all(t.as_bytes())
                    .map_err(|_| ErrorVm::TipoEsperado {
                        esperado: "escritura exitosa",
                        encontrado: "error de escritura",
                    })?;
                let _ = escritor.flush();
                Ok(Valor::Nulo)
            }),
        );
        self.registrar_nativo(
            "escribir_linea",
            std::sync::Arc::new(|args: &[Valor]| {
                let (Some(Valor::Archivo(arc)), valor) = (args.first(), args.get(1)) else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "archivo, texto",
                        encontrado: "otro",
                    });
                };
                let Some(Valor::Texto(t)) = valor else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "archivo, texto",
                        encontrado: "otro",
                    });
                };
                let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                if guard.cerrado {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "archivo abierto",
                        encontrado: "archivo ya cerrado",
                    });
                }
                let escritor = guard.escritor.as_mut().ok_or(ErrorVm::TipoEsperado {
                    esperado: "modo escritura",
                    encontrado: "archivo no abierto en modo escritura",
                })?;
                writeln!(escritor, "{t}").map_err(|_| ErrorVm::TipoEsperado {
                    esperado: "escritura exitosa",
                    encontrado: "error de escritura",
                })?;
                let _ = escritor.flush();
                Ok(Valor::Nulo)
            }),
        );
        self.registrar_nativo(
            "cerrar",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Archivo(arc)) => {
                    let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(w) = guard.escritor.as_mut() {
                        let _ = w.flush();
                    }
                    guard.cerrado = true;
                    guard.lector = None;
                    guard.escritor = None;
                    Ok(Valor::Nulo)
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "archivo",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "fin_de_archivo",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Archivo(arc)) => {
                    let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                    if guard.cerrado {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "archivo abierto",
                            encontrado: "archivo ya cerrado",
                        });
                    }
                    match guard.lector.as_mut() {
                        Some(lector) => {
                            let buf = lector.fill_buf().map_err(|_| ErrorVm::TipoEsperado {
                                esperado: "lectura exitosa",
                                encontrado: "error de lectura",
                            })?;
                            Ok(Valor::Logico(buf.is_empty()))
                        }
                        None => Ok(Valor::Logico(false)),
                    }
                }
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "archivo",
                    encontrado: "otro",
                }),
            }),
        );
        self.registrar_nativo(
            "cargar_ffi",
            std::sync::Arc::new(|args: &[Valor]| {
                let ruta = match args.first() {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto",
                            encontrado: "otro",
                        });
                    }
                };
                let id = crate::ffi::cargar_libreria(&ruta).map_err(ErrorVm::FfiInvalido)?;
                Ok(Valor::Entero(id))
            }),
        );
        self.registrar_nativo(
            "descargar_ffi",
            std::sync::Arc::new(|args: &[Valor]| {
                let id = match args.first() {
                    Some(Valor::Entero(v)) => *v,
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "entero",
                            encontrado: "otro",
                        });
                    }
                };
                crate::ffi::descargar_libreria(id).map_err(ErrorVm::FfiInvalido)?;
                Ok(Valor::Nulo)
            }),
        );
        self.registrar_nativo(
            "llamar_ffi",
            std::sync::Arc::new(|args: &[Valor]| {
                let id = match args.first() {
                    Some(Valor::Entero(v)) => *v,
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "entero",
                            encontrado: "otro",
                        });
                    }
                };
                let simbodo = match args.get(1) {
                    Some(Valor::Texto(s)) => s.clone(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto",
                            encontrado: "otro",
                        });
                    }
                };
                let argumentos = match args.get(2) {
                    Some(Valor::Lista(xs)) | Some(Valor::Arreglo(xs)) => xs.clone(),
                    Some(Valor::Nulo) => Vec::new(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "lista",
                            encontrado: "otro",
                        });
                    }
                };
                let retorno = match args.get(3) {
                    Some(Valor::Texto(s)) => s.clone(),
                    None => "entero".to_string(),
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "texto",
                            encontrado: "otro",
                        });
                    }
                };
                crate::ffi::llamar_funcion(id, &simbodo, &argumentos, &retorno)
                    .map_err(ErrorVm::FfiInvalido)
            }),
        );
        self.registrar_nativo(
            "crear_canal",
            std::sync::Arc::new(|_args: &[Valor]| {
                let reg = crate::evaluador::canales_actual();
                let mut canales = reg.lock().unwrap();
                let id = canales.len();
                canales.push(crate::evaluador::crear_canal_nuevo());
                Ok(Valor::Entero(id as i64))
            }),
        );
        self.registrar_nativo(
            "enviar",
            std::sync::Arc::new(|args: &[Valor]| {
                let id = match args.first() {
                    Some(Valor::Entero(v)) => *v as usize,
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "entero (id de canal)",
                            encontrado: "otro",
                        });
                    }
                };
                let valor = args.get(1).cloned().unwrap_or(Valor::Nulo);
                let reg = crate::evaluador::canales_actual();
                let canales = reg.lock().unwrap();
                if id >= canales.len() {
                    return Err(ErrorVm::NoSoportado(format!("canal inexistente: {id}")));
                }
                let canal = std::sync::Arc::clone(&canales[id]);
                drop(canales);
                let (lock, cv) = &*canal;
                let mut estado = lock.lock().unwrap();
                if estado.cerrado {
                    return Err(ErrorVm::NoSoportado(format!("canal cerrado: {id}")));
                }
                estado.buffer.push(valor);
                cv.notify_one();
                Ok(Valor::Nulo)
            }),
        );
        self.registrar_nativo(
            "recibir",
            std::sync::Arc::new(|args: &[Valor]| {
                let id = match args.first() {
                    Some(Valor::Entero(v)) => *v as usize,
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "entero (id de canal)",
                            encontrado: "otro",
                        });
                    }
                };
                let reg = crate::evaluador::canales_actual();
                let canales = reg.lock().unwrap();
                if id >= canales.len() {
                    return Err(ErrorVm::NoSoportado(format!("canal inexistente: {id}")));
                }
                let canal = std::sync::Arc::clone(&canales[id]);
                drop(canales);
                let (lock, cv) = &*canal;
                let mut estado = lock.lock().unwrap();
                loop {
                    if !estado.buffer.is_empty() {
                        return Ok(estado.buffer.remove(0));
                    }
                    if estado.cerrado {
                        return Err(ErrorVm::NoSoportado(format!("canal cerrado y vacío: {id}")));
                    }
                    estado = cv.wait(estado).unwrap();
                }
            }),
        );
        self.registrar_nativo(
            "cerrar_canal",
            std::sync::Arc::new(|args: &[Valor]| {
                let id = match args.first() {
                    Some(Valor::Entero(v)) => *v as usize,
                    _ => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "entero (id de canal)",
                            encontrado: "otro",
                        });
                    }
                };
                let reg = crate::evaluador::canales_actual();
                let canales = reg.lock().unwrap();
                if id >= canales.len() {
                    return Err(ErrorVm::NoSoportado(format!("canal inexistente: {id}")));
                }
                let canal = std::sync::Arc::clone(&canales[id]);
                drop(canales);
                let (lock, cv) = &*canal;
                let mut estado = lock.lock().unwrap();
                estado.cerrado = true;
                cv.notify_all();
                Ok(Valor::Nulo)
            }),
        );
        self.registrar_nativo(
            "eliminar",
            std::sync::Arc::new(|args: &[Valor]| match (args.first(), args.get(1)) {
                (Some(Valor::Arreglo(v)), Some(Valor::Entero(i))) => match (*i).try_into() {
                    Ok(idx) if idx < v.len() => {
                        let mut n = v.clone();
                        n.remove(idx);
                        Ok(Valor::Arreglo(n))
                    }
                    _ => Err(ErrorVm::IndiceFueraDeRango {
                        indice: *i,
                        tamano: v.len(),
                    }),
                },
                (Some(Valor::Lista(v)), Some(Valor::Entero(i))) => match (*i).try_into() {
                    Ok(idx) if idx < v.len() => {
                        let mut n = v.clone();
                        n.remove(idx);
                        Ok(Valor::Lista(n))
                    }
                    _ => Err(ErrorVm::IndiceFueraDeRango {
                        indice: *i,
                        tamano: v.len(),
                    }),
                },
                _ => Err(ErrorVm::TipoEsperado {
                    esperado: "arreglo o lista con índice entero no negativo",
                    encontrado: "otro",
                }),
            }),
        );
    }

    /// Salida capturada de mostrar/mostrar_sin_salto.
    pub fn salida(&self) -> Vec<String> {
        self.salida.lock().unwrap().clone()
    }

    /// Cantidad de globals del programa actual.
    pub fn globals_len(&self) -> usize {
        self.globals.len()
    }

    /// Número de funciones nativas registradas.
    pub fn numero_nativos(&self) -> usize {
        self.nativos.len()
    }

    /// Reemplaza el programa preservando globals/salida/estado de init para REPL.
    /// Ejecuta solo inicializadores de globals nuevos (índices >= globals_previos).
    pub fn reemplazar_programa_repl(
        &mut self,
        programa: MirPrograma,
        globals_previos: usize,
    ) -> Result<(), ErrorVm> {
        let old_globals = std::mem::take(&mut self.globals);
        let old_inicializado = self.inicializado;
        let n_nuevos = programa.globals.len();
        let mut indice = HashMap::new();
        for (i, f) in programa.funciones.iter().enumerate() {
            indice.entry(f.nombre.clone()).or_insert(i);
        }
        let funciones_sin_retorno: std::collections::HashSet<String> = programa
            .funciones
            .iter()
            .filter(|f| {
                !f.bloques
                    .iter()
                    .any(|b| matches!(b.terminator, MirTerminator::Return(Some(_))))
            })
            .map(|f| f.nombre.clone())
            .collect();
        let debug_global_names = programa
            .globals
            .iter()
            .enumerate()
            .map(|(i, _)| format!("g{i}"))
            .collect();
        self.programa = programa;
        self.indice = indice;
        self.funciones_sin_retorno = funciones_sin_retorno;
        self.debug_global_names = debug_global_names;
        self.tipos_inferidos.clear();
        self.sin_retorno_locals.clear();
        self.try_stack.clear();
        self.pasos = 0;
        self.profundidad = 0;
        // Restaurar globals previos; nuevos en Nulo.
        let mut globals = vec![Valor::Nulo; n_nuevos];
        for (i, v) in old_globals.into_iter().enumerate() {
            if i < globals.len() {
                globals[i] = v;
            }
        }
        self.globals = globals;
        self.inicializado = true;
        // Ejecutar init delta solo para globals nuevos.
        if n_nuevos > globals_previos {
            if let Some(init_idx) = self.indice_de("__init") {
                let init_f = self.programa.funciones.get(init_idx).cloned();
                if let Some(f) = init_f {
                    self.ejecutar_init_delta(&f, globals_previos)?;
                }
            }
        } else if !old_inicializado {
            // Primera vez: ejecutar init completo.
            self.inicializado = false;
            if let Some(idx) = self.indice_de("__init") {
                self.inicializado = true;
                let _ = self.ejecutar_indice(idx, &[])?;
            } else {
                self.inicializado = true;
            }
        }
        Ok(())
    }

    fn ejecutar_init_delta(
        &mut self,
        f: &crate::mir::MirFunction,
        globals_previos: usize,
    ) -> Result<(), ErrorVm> {
        // Ejecuta solo la parte de __init que inicializa globals >= globals_previos.
        // El lowering emite globales en orden: stmts..., AsignacionGlobal g0, ..., gN.
        self.profundidad += 1;
        if self.profundidad > self.limite_profundidad {
            self.profundidad -= 1;
            return Err(ErrorVm::ProfundidadExcedida);
        }
        let mut locals: Vec<Valor> = vec![Valor::Nulo; f.locals.len()];
        let mut bloque_actual = f.bloque_entrada;
        let mut vistos_previos = 0usize;
        let mut activo = globals_previos == 0;
        loop {
            self.paso()?;
            if bloque_actual.0 >= f.bloques.len() {
                self.profundidad -= 1;
                return Err(ErrorVm::BloqueInalcanzable);
            }
            let bloque = &f.bloques[bloque_actual.0].clone();
            let mut saltar_a_rescue: Option<BlockId> = None;
            for stmt in &bloque.stmts {
                self.paso()?;
                if !activo {
                    if let MirStmt::AsignacionGlobal { global, .. } = stmt
                        && global.0 < globals_previos
                    {
                        vistos_previos += 1;
                        if vistos_previos >= globals_previos {
                            activo = true;
                        }
                    }
                    continue;
                }
                if let MirStmt::AsignacionGlobal { global, .. } = stmt
                    && global.0 < globals_previos
                {
                    continue;
                }
                if let Err(e) = self.ejecutar_stmt(&mut locals, &f.locals, stmt) {
                    if let Some((rescue, _merge, var_local)) = self.try_stack.last().cloned() {
                        if let Some(vl) = var_local
                            && vl < locals.len()
                        {
                            locals[vl] = Valor::Texto(e.to_string());
                        }
                        self.try_stack.pop();
                        saltar_a_rescue = Some(rescue);
                        break;
                    } else {
                        self.profundidad -= 1;
                        return Err(e);
                    }
                }
            }
            if let Some(rescue) = saltar_a_rescue {
                bloque_actual = rescue;
                continue;
            }
            match self.ejecutar_terminador(&mut locals, f, &bloque.terminator) {
                Ok(Transicion::Salto(nuevo)) => bloque_actual = nuevo,
                Ok(Transicion::Retornar(_)) => break,
                Err(e) => {
                    if let Some((rescue, _merge, var_local)) = self.try_stack.last().cloned() {
                        if let Some(vl) = var_local
                            && vl < locals.len()
                        {
                            locals[vl] = Valor::Texto(e.to_string());
                        }
                        self.try_stack.pop();
                        bloque_actual = rescue;
                    } else {
                        self.profundidad -= 1;
                        return Err(e);
                    }
                }
            }
        }
        self.profundidad -= 1;
        Ok(())
    }

    /// Ejecuta la función `nombre` con `args` y devuelve su valor de retorno.
    pub fn ejecutar(&mut self, nombre: &str, args: &[Valor]) -> Result<Option<Valor>, ErrorVm> {
        let idx = self
            .indice_de(nombre)
            .ok_or_else(|| ErrorVm::FuncionNoEncontrada(nombre.to_string()))?;
        self.ejecutar_indice(idx, args)
    }

    /// Ejecuta la función en la posición `idx` de `programa.funciones`.
    pub fn ejecutar_indice(
        &mut self,
        idx: usize,
        args: &[Valor],
    ) -> Result<Option<Valor>, ErrorVm> {
        // Aislar canales/tareas por ejecución (Fase 02), preservando un
        // contexto compartido si el hilo ya participa en una ejecución (tarea).
        let _ctx = crate::evaluador::asegurar_contexto_ejecucion();
        // Se clona la función para evitar préstamos cruzados con el estado de la
        // VM (los programas del prototipo son pequeños; en bytecode real el
        // código será compartido, no clonado).
        let Some(funcion) = self.programa.funciones.get(idx).cloned() else {
            return Err(ErrorVm::BloqueInalcanzable);
        };
        let es_init = funcion.nombre == "__init";
        if self.traza && !es_init {
            self.traza_salida.lock().unwrap().push(format!(
                "→ {}({})",
                funcion.nombre,
                args.iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let r = self.correr_funcion(&funcion, args);
        self.profundidad -= 1;
        if self.traza && !es_init {
            match &r {
                Ok(Some(v)) => self
                    .traza_salida
                    .lock()
                    .unwrap()
                    .push(format!("← {} = {}", funcion.nombre, v)),
                Ok(None) => self
                    .traza_salida
                    .lock()
                    .unwrap()
                    .push(format!("← {} = (sin retorno)", funcion.nombre)),
                Err(e) => self
                    .traza_salida
                    .lock()
                    .unwrap()
                    .push(format!("← {} ! {}", funcion.nombre, e)),
            }
        }
        r
    }

    /// Registra un paso; falla si se excede el límite.
    fn paso(&mut self) -> Result<(), ErrorVm> {
        self.pasos += 1;
        if self.pasos > self.limite_pasos {
            return Err(ErrorVm::LimitePasosExcedido);
        }
        Ok(())
    }

    /// Bucle principal de la función: recorre bloques hasta un `Return`.
    fn correr_funcion(
        &mut self,
        f: &crate::mir::MirFunction,
        args: &[Valor],
    ) -> Result<Option<Valor>, ErrorVm> {
        self.profundidad += 1;
        if self.profundidad > self.limite_profundidad {
            return Err(ErrorVm::ProfundidadExcedida);
        }
        self.sin_retorno_locals.clear();

        // Inicializar globales una sola vez antes de ejecutar cualquier función.
        if !self.inicializado {
            self.inicializado = true;
            if let Some(idx) = self.indice_de("__init") {
                let _ = self.ejecutar_indice(idx, &[])?;
            }
        }

        // Locales: ceros-nulos y parámetros asignados por posición.
        let mut locals: Vec<Valor> = vec![Valor::Nulo; f.locals.len()];
        if args.len() < f.params.len() {
            return Err(ErrorVm::ArgumentosInsuficientes {
                faltantes: f.params.len() - args.len(),
            });
        }
        for (i, (local, _ty)) in f.params.iter().enumerate() {
            locals[local.0] = args[i].clone();
        }

        let mut bloque_actual = f.bloque_entrada;
        loop {
            self.paso()?;
            if bloque_actual.0 >= f.bloques.len() {
                return Err(ErrorVm::BloqueInalcanzable);
            }
            let bloque = &f.bloques[bloque_actual.0];

            // Sentencias del bloque.
            let mut saltar_a_rescue: Option<BlockId> = None;
            for stmt in &bloque.stmts {
                self.paso()?;
                if let Err(e) = self.ejecutar_stmt(&mut locals, &f.locals, stmt) {
                    if let Some((rescue, _merge, var_local)) = self.try_stack.last().cloned() {
                        if let Some(vl) = var_local
                            && vl < locals.len()
                        {
                            locals[vl] = Valor::Texto(e.to_string());
                        }
                        self.try_stack.pop();
                        saltar_a_rescue = Some(rescue);
                        break;
                    } else {
                        return Err(e);
                    }
                }
            }
            if let Some(rescue) = saltar_a_rescue {
                bloque_actual = rescue;
                continue;
            }

            // Terminador: salto al siguiente bloque o retorno.
            match self.ejecutar_terminador(&mut locals, f, &bloque.terminator) {
                Ok(Transicion::Salto(nuevo)) => bloque_actual = nuevo,
                Ok(Transicion::Retornar(valor)) => return Ok(valor),
                Err(e) => {
                    if let Some((rescue, _merge, var_local)) = self.try_stack.last().cloned() {
                        if let Some(vl) = var_local
                            && vl < locals.len()
                        {
                            locals[vl] = Valor::Texto(e.to_string());
                        }
                        self.try_stack.pop();
                        bloque_actual = rescue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Ejecuta una sentencia no-terminadora.
    fn ejecutar_stmt(
        &mut self,
        locals: &mut [Valor],
        local_types: &[crate::semantico::TipoSem],
        stmt: &MirStmt,
    ) -> Result<(), ErrorVm> {
        match stmt {
            MirStmt::Asignacion { destino, valor } => {
                let v = self.evaluar_rvalue(locals, valor)?;
                self.escribir_place(locals, local_types, destino, v)
            }
            MirStmt::AsignacionGlobal { global, valor } => {
                let v = self.evaluar_rvalue(locals, valor)?;
                if matches!(v, Valor::Celda(_)) {
                    return Err(ErrorVm::NoSoportado(
                        "celda interna no puede escapar a global".to_string(),
                    ));
                }
                if let Some(t) = self.programa.globals.get(global.0) {
                    self.globals[global.0] = convertir_tipo(v, t)?;
                } else if global.0 < self.globals.len() {
                    self.globals[global.0] = v;
                }
                Ok(())
            }
            MirStmt::Llamada {
                destino,
                callee,
                argumentos,
            } => {
                let args = self.evaluar_argumentos(locals, argumentos)?;
                let r = self.llamar(callee, &args)?;
                if let Some(dest) = destino {
                    if self.funciones_sin_retorno.contains(callee) {
                        self.sin_retorno_locals.insert(dest.local.0);
                    }
                    self.escribir_place(locals, local_types, dest, r)?;
                }
                Ok(())
            }
            MirStmt::LlamadaIndirecta {
                destino,
                callee,
                argumentos,
            } => {
                let callee_val = self.operando_a_valor(locals, callee)?;
                let nombre = match &callee_val {
                    Valor::Funcion(n) => n.clone(),
                    Valor::ClausuraMir { nombre, .. } => nombre.clone(),
                    otro => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "función",
                            encontrado: otro.nombre_tipo(),
                        });
                    }
                };
                let args_usuario = self.evaluar_argumentos(locals, argumentos)?;
                let args = if let Valor::ClausuraMir { capturas, .. } = &callee_val {
                    let esperado_usuario = self
                        .indice
                        .get(&nombre)
                        .and_then(|&idx| self.programa.funciones.get(idx))
                        .map(|f| f.params.len().saturating_sub(capturas.len()))
                        .unwrap_or(args_usuario.len());
                    if args_usuario.len() != esperado_usuario {
                        return Err(ErrorVm::FirmaInvalida {
                            funcion: "clausura".to_string(),
                            esperado: esperado_usuario,
                            obtenido: args_usuario.len(),
                        });
                    }
                    let mut full = args_usuario;
                    full.extend(capturas.clone());
                    full
                } else {
                    args_usuario
                };
                let r = self.llamar(&nombre, &args)?;
                if let Some(dest) = destino {
                    if self.funciones_sin_retorno.contains(&nombre) {
                        self.sin_retorno_locals.insert(dest.local.0);
                    }
                    self.escribir_place(locals, local_types, dest, r)?;
                }
                Ok(())
            }
            MirStmt::EntrarTry {
                rescue,
                merge,
                var_local,
            } => {
                self.try_stack.push((*rescue, *merge, *var_local));
                Ok(())
            }
            MirStmt::SalirTry => {
                if !self.try_stack.is_empty() {
                    self.try_stack.pop();
                }
                Ok(())
            }
            MirStmt::Nop => Ok(()),
        }
    }

    /// Evalúa los argumentos de una llamada.
    fn evaluar_argumentos(
        &self,
        locals: &[Valor],
        ops: &[MirOperand],
    ) -> Result<Vec<Valor>, ErrorVm> {
        ops.iter()
            .map(|op| self.operando_a_valor(locals, op))
            .collect()
    }

    /// Ejecuta un terminador y devuelve la transición (salto o retorno).
    fn ejecutar_terminador(
        &mut self,
        locals: &mut [Valor],
        f: &crate::mir::MirFunction,
        t: &MirTerminator,
    ) -> Result<Transicion, ErrorVm> {
        match t {
            MirTerminator::Goto(b) => Ok(Transicion::Salto(*b)),
            MirTerminator::SwitchInt {
                discriminante,
                casos,
                por_defecto,
            } => {
                let v = self.operando_a_valor(locals, discriminante)?;
                // Forma booleana (if/while/para-cada): casos [(1, destino)] y default.
                if casos.len() == 1 && casos[0].0 == 1 {
                    let cond = match v {
                        Valor::Logico(b) => b,
                        otro => {
                            return Err(ErrorVm::CondicionNoLogica {
                                obtenido: otro.nombre_tipo(),
                            });
                        }
                    };
                    return Ok(Transicion::Salto(if cond {
                        casos[0].1
                    } else {
                        *por_defecto
                    }));
                }
                let disc = valor_a_discriminante(&v)?;
                for (valor, b) in casos {
                    if disc == *valor {
                        return Ok(Transicion::Salto(*b));
                    }
                }
                Ok(Transicion::Salto(*por_defecto))
            }
            MirTerminator::Return(op) => match op {
                Some(op) => {
                    let v = self.operando_a_valor(locals, op)?;
                    if matches!(v, Valor::Celda(_)) {
                        return Err(ErrorVm::NoSoportado(
                            "celda interna no puede escapar como valor de retorno".to_string(),
                        ));
                    }
                    let v = if f.retorno == crate::semantico::TipoSem::Desconocido {
                        v
                    } else {
                        convertir_tipo(v, &f.retorno).map_err(|e| match e {
                            ErrorVm::AsignacionIncompatible { esperado, obtenido } => {
                                ErrorVm::RetornoIncompatible {
                                    funcion: f.nombre.clone(),
                                    esperado,
                                    obtenido,
                                }
                            }
                            other => other,
                        })?
                    };
                    Ok(Transicion::Retornar(Some(v)))
                }
                None => Ok(Transicion::Retornar(None)),
            },
            MirTerminator::Invoke {
                callee,
                argumentos,
                destino,
                bloque_ok,
                bloque_error,
            } => {
                let args = self.evaluar_argumentos(locals, argumentos)?;
                match self.llamar(callee, &args) {
                    Ok(v) => {
                        if let Some(dest) = destino {
                            self.escribir_place(locals, &f.locals, dest, v)?;
                        }
                        Ok(Transicion::Salto(*bloque_ok))
                    }
                    Err(_) => Ok(Transicion::Salto(*bloque_error)),
                }
            }
            MirTerminator::Unreachable => Err(ErrorVm::BloqueInalcanzable),
        }
    }

    /// Llama a una función MIR por nombre. Sin retorno MIR equivale a nulo.
    fn llamar(&mut self, callee: &str, args: &[Valor]) -> Result<Valor, ErrorVm> {
        // Funciones MIR del programa tienen prioridad.
        if self.indice.contains_key(callee) {
            // Verificación de tipos en tiempo de ejecución (paridad con Tree-Walk):
            // - Si los parámetros tienen anotación, validar/convertir cada argumento.
            // - Si no, inferir en la primera llamada y validar las siguientes.
            let idx = self.indice[callee];
            let func = self.programa.funciones[idx].clone();
            let anotados = func
                .params
                .iter()
                .any(|(_, t)| *t != crate::semantico::TipoSem::Desconocido);
            // Conversión/validación de parámetros tipados o inferidos (paridad TW, GAP-051).
            let convertidos: Option<Vec<Valor>> = if anotados {
                let mut convertidos: Option<Vec<Valor>> = None;
                for (i, (_, tipo)) in func.params.iter().enumerate() {
                    if *tipo == crate::semantico::TipoSem::Desconocido {
                        continue;
                    }
                    if let Some(arg) = args.get(i) {
                        // Las celdas son detalles internos de captura por referencia;
                        // se pasan tal cual sin validación/conversión de tipo de parámetro.
                        if matches!(arg, Valor::Celda(_)) {
                            continue;
                        }
                        let obtenido_original = arg.nombre_tipo().to_string();
                        match convertir_tipo(arg.clone(), tipo) {
                            Ok(nuevo) => {
                                if nuevo != *arg {
                                    let conv = convertidos.get_or_insert_with(|| args.to_vec());
                                    conv[i] = nuevo;
                                }
                            }
                            Err(ErrorVm::AsignacionIncompatible { esperado, .. }) => {
                                return Err(ErrorVm::TipoParametroIncompatible {
                                    funcion: callee.to_string(),
                                    // MIR no conserva nombres de parámetros; se usa índice 1-based.
                                    parametro: (i + 1).to_string(),
                                    esperado,
                                    obtenido: obtenido_original,
                                });
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
                convertidos
            } else if !func.params.is_empty() {
                // Inferencia: primera llamada guarda tipos; siguientes validan
                // y convierten (paridad con `convertir_para_tipo` del Tree-Walk).
                if let Some(inferidos) = self.tipos_inferidos.get(callee).cloned() {
                    let mut convertidos: Option<Vec<Valor>> = None;
                    for (i, t) in inferidos.iter().enumerate() {
                        if let Some(arg) = args.get(i) {
                            if matches!(arg, Valor::Celda(_)) {
                                continue;
                            }
                            let obtenido_original = arg.nombre_tipo().to_string();
                            match convertir_para_tipo(t, arg) {
                                Some(nuevo) => {
                                    if nuevo != *arg {
                                        let conv = convertidos.get_or_insert_with(|| args.to_vec());
                                        conv[i] = nuevo;
                                    }
                                }
                                None => {
                                    return Err(ErrorVm::TipoParametroIncompatible {
                                        funcion: callee.to_string(),
                                        parametro: (i + 1).to_string(),
                                        esperado: t.nombre_tipo().to_string(),
                                        obtenido: obtenido_original,
                                    });
                                }
                            }
                        }
                    }
                    convertidos
                } else {
                    self.tipos_inferidos
                        .insert(callee.to_string(), args.to_vec());
                    None
                }
            } else {
                None
            };
            let args_final: &[Valor] = convertidos.as_deref().unwrap_or(args);
            let devuelto = self.ejecutar(callee, args_final)?;
            return Ok(devuelto.unwrap_or(Valor::Nulo));
        }
        // Orden superior: map/filtrar/reducir con clausura MIR. La VM es quien
        // puede invocar la clausura, así que intercepta antes del despacho a
        // nativas (semántica idéntica a `evaluador`).
        if args.len() >= 2 && matches!(args[1], Valor::ClausuraMir { .. }) {
            match callee {
                "map" | "filtrar" | "reducir" => return self.llamar_funcional(callee, args),
                _ => {}
            }
        }
        // Tareas: iniciar_tarea("nombre") / esperar_tarea(id). La tarea corre en
        // su propia VM (mismo programa, igual que el Tree-Walk clona el entorno).
        match callee {
            "iniciar_tarea" if !args.is_empty() => return self.iniciar_tarea(args),
            "esperar_tarea" if !args.is_empty() => return self.esperar_tarea(args),
            _ => {}
        }
        // Fallback: función nativa registrada (FFI / integrada).
        if self.nativos.contains_key(callee) {
            self.paso()?;
            let f = self.nativos.get(callee).cloned().unwrap();
            return f(args);
        }
        Err(ErrorVm::FuncionNoEncontrada(callee.to_string()))
    }

    /// `map`/`filtrar`/`reducir` con clausura MIR: invoca la clausura por cada
    /// elemento, replicando la semántica de `evaluador`.
    fn llamar_funcional(&mut self, callee: &str, args: &[Valor]) -> Result<Valor, ErrorVm> {
        self.paso()?;
        let func_val = args[1].clone();
        match callee {
            "map" => match &args[0] {
                Valor::Arreglo(v) => {
                    let mut out = Vec::new();
                    for item in v {
                        let r = self.invocar_cierre(&func_val, std::slice::from_ref(item))?;
                        out.push(r);
                    }
                    Ok(Valor::Arreglo(out))
                }
                Valor::Lista(v) => {
                    let mut out = Vec::new();
                    for item in v {
                        let r = self.invocar_cierre(&func_val, std::slice::from_ref(item))?;
                        out.push(r);
                    }
                    Ok(Valor::Lista(out))
                }
                otro => Err(ErrorVm::TipoEsperado {
                    esperado: "arreglo o lista",
                    encontrado: otro.nombre_tipo(),
                }),
            },
            "filtrar" => match &args[0] {
                Valor::Arreglo(v) => {
                    let mut out = Vec::new();
                    for item in v {
                        let r = self.invocar_cierre(&func_val, std::slice::from_ref(item))?;
                        match r {
                            Valor::Logico(true) => out.push(item.clone()),
                            Valor::Logico(false) => {}
                            _ => {
                                return Err(ErrorVm::TipoEsperado {
                                    esperado: "lógico",
                                    encontrado: r.nombre_tipo(),
                                });
                            }
                        }
                    }
                    Ok(Valor::Arreglo(out))
                }
                Valor::Lista(v) => {
                    let mut out = Vec::new();
                    for item in v {
                        let r = self.invocar_cierre(&func_val, std::slice::from_ref(item))?;
                        match r {
                            Valor::Logico(true) => out.push(item.clone()),
                            Valor::Logico(false) => {}
                            _ => {
                                return Err(ErrorVm::TipoEsperado {
                                    esperado: "lógico",
                                    encontrado: r.nombre_tipo(),
                                });
                            }
                        }
                    }
                    Ok(Valor::Lista(out))
                }
                otro => Err(ErrorVm::TipoEsperado {
                    esperado: "arreglo o lista",
                    encontrado: otro.nombre_tipo(),
                }),
            },
            "reducir" => {
                let mut acc = args.get(2).cloned().unwrap_or(Valor::Nulo);
                match &args[0] {
                    Valor::Arreglo(v) | Valor::Lista(v) => {
                        for item in v {
                            acc = self.invocar_cierre(&func_val, &[acc.clone(), item.clone()])?;
                        }
                        Ok(acc)
                    }
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "arreglo o lista",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            _ => Err(ErrorVm::FuncionNoEncontrada(callee.to_string())),
        }
    }

    /// Invoca un `Valor` que representa función: clausura MIR (con sus capturas)
    /// o referencia por nombre a una función MIR.
    fn invocar_cierre(&mut self, cierre: &Valor, args: &[Valor]) -> Result<Valor, ErrorVm> {
        match cierre {
            Valor::ClausuraMir { nombre, capturas } => {
                let mut full = args.to_vec();
                full.extend(capturas.clone());
                self.llamar(nombre, &full)
            }
            Valor::Funcion(n) => self.llamar(n, args),
            otro => Err(ErrorVm::TipoEsperado {
                esperado: "función",
                encontrado: otro.nombre_tipo(),
            }),
        }
    }

    /// `iniciar_tarea("nombre")`: lanza la función MIR en un hilo con su propia
    /// VM y devuelve el id de la tarea (semántica de `evaluador`).
    fn iniciar_tarea(&mut self, args: &[Valor]) -> Result<Valor, ErrorVm> {
        let func_name = match &args[0] {
            Valor::Texto(s) => s.clone(),
            otro => {
                return Err(ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: otro.nombre_tipo(),
                });
            }
        };
        if !self.indice.contains_key(&func_name) {
            return Err(ErrorVm::NoSoportado(format!(
                "tarea: función no encontrada: {func_name}"
            )));
        }
        let programa = self.programa.clone();
        let globals = self.globals.clone();
        let salida = std::sync::Arc::clone(&self.salida);
        let canales_ctx = crate::evaluador::canales_actual();
        let tareas_ctx = crate::evaluador::tareas_actual();
        let handle = std::thread::spawn(move || {
            crate::evaluador::instalar_registros(canales_ctx, tareas_ctx);
            let mut vm = MaquinaVirtual::nuevo(programa);
            vm.registrar_stdlib();
            // Compartir el estado global (incluye canales) sin re-ejecutar __init.
            vm.globals = globals;
            vm.salida = salida;
            vm.inicializado = true; // no re-ejecutar __init
            vm.ejecutar(&func_name, &[])
                .map(|v| v.unwrap_or(Valor::Nulo))
                .map_err(|e| e.to_string())
        });
        let reg_tareas = crate::evaluador::tareas_actual();
        let mut tareas = reg_tareas.lock().unwrap();
        let id = tareas.len();
        tareas.push(Some(handle));
        Ok(Valor::Entero(id as i64))
    }

    /// `esperar_tarea(id)`: espera (join) la tarea y devuelve su valor.
    fn esperar_tarea(&mut self, args: &[Valor]) -> Result<Valor, ErrorVm> {
        let id = match &args[0] {
            Valor::Entero(v) => *v as usize,
            otro => {
                return Err(ErrorVm::TipoEsperado {
                    esperado: "entero",
                    encontrado: otro.nombre_tipo(),
                });
            }
        };
        let handle = {
            let reg_tareas = crate::evaluador::tareas_actual();
            let mut tareas = reg_tareas.lock().unwrap();
            if id >= tareas.len() {
                return Err(ErrorVm::NoSoportado(format!("tarea inexistente: {id}")));
            }
            tareas[id].take()
        };
        match handle {
            Some(h) => match h.join() {
                Ok(Ok(v)) => Ok(v),
                Ok(Err(e)) => Err(ErrorVm::NoSoportado(format!("tarea falló: {e}"))),
                Err(_) => Err(ErrorVm::NoSoportado(format!(
                    "tarea falló: pánico en tarea {id}"
                ))),
            },
            None => Err(ErrorVm::NoSoportado(format!("tarea ya esperada: {id}"))),
        }
    }

    /// Lee un operando: clona el local o materializa la constante.
    fn operando_a_valor(&self, locals: &[Valor], op: &MirOperand) -> Result<Valor, ErrorVm> {
        match op {
            MirOperand::Copy(local) | MirOperand::Move(local) => {
                if self.sin_retorno_locals.contains(&local.0) {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "valor de retorno",
                        encontrado: "función sin retorno",
                    });
                }
                locals.get(local.0).cloned().ok_or(ErrorVm::TipoEsperado {
                    esperado: "local",
                    encontrado: "vacío",
                })
            }
            MirOperand::Const(c) => Ok(const_a_valor(c)),
            MirOperand::Global(g) => self.globals.get(g.0).cloned().ok_or(ErrorVm::TipoEsperado {
                esperado: "global",
                encontrado: "vacío",
            }),
        }
    }

    /// Escribe en un `MirPlace` (local + proyecciones de un solo nivel).
    fn escribir_place(
        &self,
        locals: &mut [Valor],
        local_types: &[crate::semantico::TipoSem],
        place: &MirPlace,
        valor: Valor,
    ) -> Result<(), ErrorVm> {
        let valor = if place.projection.is_empty() {
            if matches!(valor, Valor::Celda(_)) {
                valor
            } else {
                self.convertir_tipo_opt(valor, local_types.get(place.local.0))?
            }
        } else {
            valor
        };
        let indice = match place.projection.as_slice() {
            [MirProjection::Indice(op)] => {
                let Valor::Entero(i) = self.operando_a_valor(locals, op)? else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "entero",
                        encontrado: "índice no entero",
                    });
                };
                if i < 0 {
                    return Err(ErrorVm::AsignacionIncompatible {
                        esperado: "índice válido".into(),
                        obtenido: format!("índice {} fuera de rango (tamaño 0)", i),
                    });
                }
                Some(i as usize)
            }
            _ => None,
        };
        let Some(local_val) = locals.get_mut(place.local.0) else {
            return Err(ErrorVm::TipoEsperado {
                esperado: "local",
                encontrado: "vacío",
            });
        };
        match place.projection.as_slice() {
            [] => {
                *local_val = valor;
                Ok(())
            }
            [MirProjection::Campo(campo)] => match local_val {
                Valor::Objeto(mapa) => {
                    mapa.insert(campo.clone(), valor);
                    Ok(())
                }
                Valor::EstructuraNominal { .. } => Err(ErrorVm::AsignacionIncompatible {
                    esperado: "objeto dinámico".into(),
                    obtenido: "estructura nominal (use CampoIndice)".into(),
                }),
                otro => Err(ErrorVm::TipoEsperado {
                    esperado: "objeto",
                    encontrado: otro.nombre_tipo(),
                }),
            },
            [MirProjection::CampoIndice(idx)] => match local_val {
                Valor::EstructuraNominal { campos, .. } => {
                    if *idx >= campos.len() {
                        return Err(ErrorVm::AsignacionIncompatible {
                            esperado: "índice de campo válido".into(),
                            obtenido: format!(
                                "campo {} fuera de rango (tamaño {})",
                                idx,
                                campos.len()
                            ),
                        });
                    }
                    campos[*idx] = valor;
                    Ok(())
                }
                otro => Err(ErrorVm::TipoEsperado {
                    esperado: "estructura nominal",
                    encontrado: otro.nombre_tipo(),
                }),
            },
            [MirProjection::Indice(_)] => {
                let idx = indice.expect("resuelto arriba");
                match local_val {
                    Valor::Arreglo(elems) | Valor::Lista(elems) => {
                        if idx >= elems.len() {
                            return Err(ErrorVm::AsignacionIncompatible {
                                esperado: "índice válido".into(),
                                obtenido: format!(
                                    "índice {} fuera de rango (tamaño {})",
                                    idx,
                                    elems.len()
                                ),
                            });
                        }
                        elems[idx] = valor;
                        Ok(())
                    }
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "arreglo o lista",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            [MirProjection::Celda] => match local_val {
                Valor::Celda(c) => {
                    let mut guard = c.lock().unwrap();
                    *guard = valor;
                    Ok(())
                }
                otro => Err(ErrorVm::TipoEsperado {
                    esperado: "celda",
                    encontrado: otro.nombre_tipo(),
                }),
            },
            _ => Err(ErrorVm::TipoEsperado {
                esperado: "proyección simple",
                encontrado: "proyección anidada",
            }),
        }
    }

    /// Evalúa un rvalue.
    fn evaluar_rvalue(&self, locals: &[Valor], r: &MirRvalue) -> Result<Valor, ErrorVm> {
        match r {
            MirRvalue::Use(op) => self.operando_a_valor(locals, op),
            MirRvalue::BinOp { op, lhs, rhs } => {
                // Cortocircuito para Y/O con semántica de operandos lógicos (paridad TW).
                match op {
                    MirBinOp::Y => {
                        let a = self.operando_a_valor(locals, lhs)?;
                        match a {
                            Valor::Logico(false) => Ok(Valor::Logico(false)),
                            Valor::Logico(true) => {
                                let b = self.operando_a_valor(locals, rhs)?;
                                match b {
                                    Valor::Logico(v) => Ok(Valor::Logico(v)),
                                    _ => Err(ErrorVm::OperandoNoLogico {
                                        operador: "y",
                                        obtenido: b.nombre_tipo(),
                                    }),
                                }
                            }
                            _ => Err(ErrorVm::OperandoNoLogico {
                                operador: "y",
                                obtenido: a.nombre_tipo(),
                            }),
                        }
                    }
                    MirBinOp::O => {
                        let a = self.operando_a_valor(locals, lhs)?;
                        match a {
                            Valor::Logico(true) => Ok(Valor::Logico(true)),
                            Valor::Logico(false) => {
                                let b = self.operando_a_valor(locals, rhs)?;
                                match b {
                                    Valor::Logico(v) => Ok(Valor::Logico(v)),
                                    _ => Err(ErrorVm::OperandoNoLogico {
                                        operador: "o",
                                        obtenido: b.nombre_tipo(),
                                    }),
                                }
                            }
                            _ => Err(ErrorVm::OperandoNoLogico {
                                operador: "o",
                                obtenido: a.nombre_tipo(),
                            }),
                        }
                    }
                    _ => {
                        let a = self.operando_a_valor(locals, lhs)?;
                        let b = self.operando_a_valor(locals, rhs)?;
                        aplicar_binop(op, &a, &b)
                    }
                }
            }
            MirRvalue::UnOp { op, operand } => {
                let a = self.operando_a_valor(locals, operand)?;
                aplicar_unop(op, &a)
            }
            MirRvalue::Agregado { elementos } => {
                let elems: Result<Vec<Valor>, _> = elementos
                    .iter()
                    .map(|op| self.operando_a_valor(locals, op))
                    .collect();
                // Literales `[...]` en BASILISCO son Lista por defecto (paridad TW).
                Ok(Valor::Lista(elems?))
            }
            MirRvalue::Indice { base, indice } => {
                let base_v = self.operando_a_valor(locals, base)?;
                let idx_v = self.operando_a_valor(locals, indice)?;
                let Valor::Entero(idx) = idx_v else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "entero",
                        encontrado: "índice no entero",
                    });
                };
                match &base_v {
                    Valor::Arreglo(elems) | Valor::Lista(elems) => {
                        if idx < 0 || idx as usize >= elems.len() {
                            return Err(ErrorVm::AsignacionIncompatible {
                                esperado: "índice válido".into(),
                                obtenido: format!(
                                    "índice {} fuera de rango (tamaño {})",
                                    idx,
                                    elems.len()
                                ),
                            });
                        }
                        Ok(elems[idx as usize].clone())
                    }
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "arreglo o lista",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            MirRvalue::Campo { base, campo } => {
                let base_v = self.operando_a_valor(locals, base)?;
                match &base_v {
                    Valor::Objeto(mapa) => {
                        mapa.get(campo)
                            .cloned()
                            .ok_or_else(|| ErrorVm::CampoInexistente {
                                campo: campo.clone(),
                            })
                    }
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "objeto",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            MirRvalue::CrearCelda { valor } => {
                let v = self.operando_a_valor(locals, valor)?;
                Ok(Valor::Celda(std::sync::Arc::new(std::sync::Mutex::new(v))))
            }
            MirRvalue::LeerCelda { base } => {
                let base_v = self.operando_a_valor(locals, base)?;
                match &base_v {
                    Valor::Celda(c) => {
                        let guard = c.lock().unwrap();
                        Ok(guard.clone())
                    }
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "celda",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            MirRvalue::ConstruirStruct { tipo_id, campos } => {
                let mut vals = Vec::with_capacity(campos.len());
                for op in campos {
                    vals.push(self.operando_a_valor(locals, op)?);
                }
                Ok(Valor::EstructuraNominal {
                    tipo_id: *tipo_id,
                    campos: vals,
                })
            }
            MirRvalue::ConstruirEnum {
                tipo_id,
                variante_idx,
                payload,
            } => {
                let mut vals = Vec::with_capacity(payload.len());
                for op in payload {
                    vals.push(self.operando_a_valor(locals, op)?);
                }
                Ok(Valor::EnumNominal {
                    tipo_id: *tipo_id,
                    variante: *variante_idx,
                    payload: vals,
                })
            }
            MirRvalue::DiscriminanteEnum { base } => {
                let base_v = self.operando_a_valor(locals, base)?;
                match base_v {
                    Valor::EnumNominal { variante, .. } => Ok(Valor::Entero(variante as i64)),
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "enumeracion nominal",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            MirRvalue::PayloadEnum { base, indice } => {
                let base_v = self.operando_a_valor(locals, base)?;
                match &base_v {
                    Valor::EnumNominal { payload, .. } => {
                        payload.get(*indice).cloned().ok_or_else(|| {
                            ErrorVm::AsignacionIncompatible {
                                esperado: "índice de payload válido".into(),
                                obtenido: format!("payload {} fuera de rango", indice),
                            }
                        })
                    }
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "enumeracion nominal",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            MirRvalue::CampoNominal { base, indice } => {
                let base_v = self.operando_a_valor(locals, base)?;
                match &base_v {
                    Valor::EstructuraNominal { campos, .. } => campos
                        .get(*indice)
                        .cloned()
                        .ok_or_else(|| ErrorVm::AsignacionIncompatible {
                            esperado: "índice de campo válido".into(),
                            obtenido: format!("campo {} fuera de rango", indice),
                        }),
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "estructura nominal",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            MirRvalue::Clausura { nombre, capturas } => {
                let caps: Vec<Valor> = capturas
                    .iter()
                    .map(|op| self.operando_a_valor(locals, op))
                    .collect::<Result<_, _>>()?;
                Ok(Valor::ClausuraMir {
                    nombre: nombre.clone(),
                    capturas: caps,
                })
            }
            MirRvalue::ObjetoModulo { prefijo, nombres } => {
                let mut mapa = std::collections::BTreeMap::new();
                for n in nombres {
                    mapa.insert(n.clone(), Valor::Funcion(format!("{}_{}", prefijo, n)));
                }
                Ok(Valor::Objeto(mapa))
            }
            MirRvalue::Longitud { base } => {
                let base_v = self.operando_a_valor(locals, base)?;
                let n = match &base_v {
                    Valor::Arreglo(e) | Valor::Lista(e) => e.len() as i64,
                    Valor::Texto(s) => s.chars().count() as i64,
                    otro => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "arreglo, lista o texto",
                            encontrado: otro.nombre_tipo(),
                        });
                    }
                };
                Ok(Valor::Entero(n))
            }
            MirRvalue::Rango {
                inicio,
                fin,
                inclusivo,
            } => {
                let vi = self.operando_a_valor(locals, inicio)?;
                let vf = self.operando_a_valor(locals, fin)?;
                let (Valor::Entero(a), Valor::Entero(b)) = (vi, vf) else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "entero",
                        encontrado: "rango no entero",
                    });
                };
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                let mut elems = Vec::new();
                if *inclusivo {
                    let mut i = lo;
                    while i <= hi {
                        elems.push(Valor::Entero(i));
                        i += 1;
                    }
                } else {
                    let mut i = lo;
                    while i < hi {
                        elems.push(Valor::Entero(i));
                        i += 1;
                    }
                }
                Ok(Valor::Lista(elems))
            }
        }
    }
}

/// Transición de un terminador: salto a bloque o retorno.
enum Transicion {
    Salto(BlockId),
    Retornar(Option<Valor>),
}

/// Convierte un `MirConst` en `Valor`.
fn const_a_valor(c: &MirConst) -> Valor {
    match c {
        MirConst::Entero(n) => Valor::Entero(*n),
        MirConst::Decimal(n) => Valor::Decimal(*n),
        MirConst::Real(n) => Valor::Real(*n),
        MirConst::Logico(b) => Valor::Logico(*b),
        MirConst::Texto(s) => Valor::Texto(s.clone()),
        MirConst::Nulo => Valor::Nulo,
        MirConst::Funcion(s) => Valor::Funcion(s.clone()),
    }
}

/// Discriminante (i64) para `SwitchInt`: lógicos 0/1, enteros como tal.
fn valor_a_discriminante(v: &Valor) -> Result<i64, ErrorVm> {
    match v {
        Valor::Entero(n) => Ok(*n),
        Valor::Logico(b) => Ok(i64::from(*b)),
        Valor::Nulo => Ok(0),
        otro => Err(ErrorVm::TipoEsperado {
            esperado: "entero o lógico",
            encontrado: otro.nombre_tipo(),
        }),
    }
}

/// Aplica un operador binario MIR con la semántica de `Valor` del núcleo.
fn aplicar_binop(op: &MirBinOp, a: &Valor, b: &Valor) -> Result<Valor, ErrorVm> {
    match op {
        MirBinOp::Suma => a.suma(b).map_err(ErrorVm::Aritmetico),
        MirBinOp::Resta => a.resta(b).map_err(ErrorVm::Aritmetico),
        MirBinOp::Multiplicacion => a.multiplicacion(b).map_err(ErrorVm::Aritmetico),
        MirBinOp::Division => a.division(b).map_err(ErrorVm::Aritmetico),
        MirBinOp::Modulo => a.modulo(b).map_err(ErrorVm::Aritmetico),
        MirBinOp::Igual => comparar_igualdad(a, b, true),
        MirBinOp::Distinto => comparar_igualdad(a, b, false),
        MirBinOp::Menor => comparar(a, b, |o| o.is_lt()),
        MirBinOp::MenorIgual => comparar(a, b, |o| o.is_le()),
        MirBinOp::Mayor => comparar(a, b, |o| o.is_gt()),
        MirBinOp::MayorIgual => comparar(a, b, |o| o.is_ge()),
        MirBinOp::Y | MirBinOp::O => match (a, b) {
            (Valor::Logico(x), Valor::Logico(y)) => {
                let r = if *op == MirBinOp::Y {
                    *x && *y
                } else {
                    *x || *y
                };
                Ok(Valor::Logico(r))
            }
            _ => Err(ErrorVm::TiposIncompatibles {
                izquierdo: a.nombre_tipo(),
                derecho: b.nombre_tipo(),
            }),
        },
    }
}

/// Comparaciones de orden con semántica TW: nulo/lógicos no comparables, conversiones implícitas limitadas.
fn comparar(
    a: &Valor,
    b: &Valor,
    acepta: impl Fn(std::cmp::Ordering) -> bool,
) -> Result<Valor, ErrorVm> {
    if matches!(a, Valor::Nulo) || matches!(b, Valor::Nulo) {
        return Err(ErrorVm::TiposIncompatibles {
            izquierdo: a.nombre_tipo(),
            derecho: b.nombre_tipo(),
        });
    }
    if matches!(a, Valor::Logico(_)) || matches!(b, Valor::Logico(_)) {
        return Err(ErrorVm::TiposIncompatibles {
            izquierdo: a.nombre_tipo(),
            derecho: b.nombre_tipo(),
        });
    }
    let orden = match (a, b) {
        (Valor::Entero(x), Valor::Entero(y)) => x.cmp(y),
        (Valor::Entero(x), Valor::Decimal(y)) => (*x as i128 * FACTOR_ESCALA).cmp(y),
        (Valor::Decimal(x), Valor::Entero(y)) => x.cmp(&(*y as i128 * FACTOR_ESCALA)),
        (Valor::Decimal(x), Valor::Decimal(y)) => x.cmp(y),
        (Valor::Entero(x), Valor::Real(y)) => match (*x as f64).partial_cmp(y) {
            Some(o) => o,
            // NaN: ninguna relación de orden es cierta (semántica de `evaluador`).
            None => return Ok(Valor::Logico(false)),
        },
        (Valor::Real(x), Valor::Entero(y)) => match x.partial_cmp(&(*y as f64)) {
            Some(o) => o,
            None => return Ok(Valor::Logico(false)),
        },
        (Valor::Real(x), Valor::Real(y)) => match x.partial_cmp(y) {
            Some(o) => o,
            // NaN: ninguna relación de orden es cierta (semántica de `evaluador`).
            None => return Ok(Valor::Logico(false)),
        },
        (Valor::Texto(x), Valor::Texto(y)) => x.cmp(y),
        _ => {
            return Err(ErrorVm::TiposIncompatibles {
                izquierdo: a.nombre_tipo(),
                derecho: b.nombre_tipo(),
            });
        }
    };
    Ok(Valor::Logico(acepta(orden)))
}

/// Aplica un operador unario MIR.
fn aplicar_unop(op: &MirUnOp, a: &Valor) -> Result<Valor, ErrorVm> {
    match op {
        MirUnOp::No => match a {
            Valor::Logico(b) => Ok(Valor::Logico(!*b)),
            otro => Err(ErrorVm::TipoEsperado {
                esperado: "lógico",
                encontrado: otro.nombre_tipo(),
            }),
        },
        MirUnOp::Negacion => match a {
            Valor::Entero(n) => n
                .checked_neg()
                .map(Valor::Entero)
                .ok_or(ErrorVm::Aritmetico(
                    crate::valor::MotivoAritmetico::DesbordamientoEntero,
                )),
            Valor::Decimal(m) => Ok(Valor::Decimal(*m * -1)),
            Valor::Real(x) => Ok(Valor::Real(-x)),
            otro => Err(ErrorVm::TiposIncompatibles {
                izquierdo: otro.nombre_tipo(),
                derecho: "negación",
            }),
        },
    }
}

/// Convierte un `serde_json::Value` en un `Valor` de BASILISCO.
fn json_a_valor(v: serde_json::Value) -> Valor {
    match v {
        serde_json::Value::Null => Valor::Nulo,
        serde_json::Value::Bool(b) => Valor::Logico(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Valor::Entero(i)
            } else if let Some(f) = n.as_f64() {
                Valor::Real(f)
            } else {
                Valor::Real(f64::INFINITY)
            }
        }
        serde_json::Value::String(s) => Valor::Texto(s),
        serde_json::Value::Array(arr) => Valor::Lista(arr.into_iter().map(json_a_valor).collect()),
        serde_json::Value::Object(obj) => {
            let mapa = obj.into_iter().map(|(k, v)| (k, json_a_valor(v))).collect();
            Valor::Objeto(mapa)
        }
    }
}

/// Convierte un `Valor` en `serde_json::Value`.
fn valor_a_json(v: &Valor) -> Result<serde_json::Value, String> {
    match v {
        Valor::Nulo => Ok(serde_json::Value::Null),
        Valor::Logico(b) => Ok(serde_json::Value::Bool(*b)),
        Valor::Entero(n) => Ok(serde_json::Value::Number((*n).into())),
        Valor::Real(f) => {
            if f.is_nan() || f.is_infinite() {
                return Err("real no representable en JSON".to_string());
            }
            serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .ok_or_else(|| "real no representable como número JSON".to_string())
        }
        Valor::Decimal(m) => {
            let f = *m as f64 / 1_000_000_000.0;
            serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .ok_or_else(|| "decimal no representable como número JSON".to_string())
        }
        Valor::Texto(s) => Ok(serde_json::Value::String(s.clone())),
        Valor::Lista(elems) | Valor::Arreglo(elems) => {
            let arr: Result<Vec<_>, _> = elems.iter().map(valor_a_json).collect();
            Ok(serde_json::Value::Array(arr?))
        }
        Valor::Objeto(mapa) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in mapa {
                obj.insert(k.clone(), valor_a_json(v)?);
            }
            Ok(serde_json::Value::Object(obj))
        }
        otro => Err(format!(
            "el tipo '{}' no es serializable en JSON",
            otro.nombre_tipo()
        )),
    }
}

/// Comparador homogéneo para `ordenar` (tex (texto), o numéricos en orden de conversión implícita).
fn ordenar_valor(a: &Valor, b: &Valor) -> std::cmp::Ordering {
    fn rango(v: &Valor) -> u8 {
        match v {
            Valor::Entero(_) => 0,
            Valor::Decimal(_) => 1,
            Valor::Real(_) => 2,
            Valor::Logico(_) => 3,
            Valor::Texto(_) => 4,
            _ => 5,
        }
    }
    use std::cmp::Ordering::*;
    let ra = rango(a);
    let rb = rango(b);
    if ra != rb {
        return ra.cmp(&rb);
    }
    match (a, b) {
        (Valor::Entero(x), Valor::Entero(y)) => x.cmp(y),
        (Valor::Decimal(x), Valor::Decimal(y)) => x.cmp(y),
        (Valor::Real(x), Valor::Real(y)) => x.partial_cmp(y).unwrap_or(Equal),
        (Valor::Logico(x), Valor::Logico(y)) => x.cmp(y),
        (Valor::Texto(x), Valor::Texto(y)) => x.cmp(y),
        _ => Equal,
    }
}

/// Compatibilidad entre dos valores para la inferencia de parámetros
/// (paridad con `convertir_para_tipo` del Tree-Walk): mismo discriminante
/// o ambas secuencias (`Arreglo`/`Lista`, tratadas como familia compatible).
fn valor_compat_valor(a: &Valor, b: &Valor) -> bool {
    if matches!(a, Valor::Arreglo(_) | Valor::Lista(_))
        && matches!(b, Valor::Arreglo(_) | Valor::Lista(_))
    {
        return true;
    }
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

/// Convierte un argumento según el tipo observado en la primera llamada
/// (paridad con `convertir_para_tipo` del evaluador Tree-Walk).
fn convertir_para_tipo(viejo: &Valor, nuevo: &Valor) -> Option<Valor> {
    match (viejo, nuevo) {
        (Valor::Decimal(_), Valor::Entero(n)) => Some(Valor::decimal_desde_entero(*n)),
        (Valor::Real(_), Valor::Entero(n)) => Some(Valor::Real(*n as f64)),
        _ if valor_compat_valor(viejo, nuevo) => Some(nuevo.clone()),
        _ => None,
    }
}

/// Igualdad/desigualdad con semántica TW (ADR-0005/0007): sin comparación de
/// lógicos ni de colecciones/objetos por operador, nulo solo con nulo.
fn comparar_igualdad(a: &Valor, b: &Valor, igual: bool) -> Result<Valor, ErrorVm> {
    use crate::valor::FACTOR_ESCALA;
    if matches!(
        a,
        Valor::EstructuraNominal { .. } | Valor::EnumNominal { .. }
    ) || matches!(
        b,
        Valor::EstructuraNominal { .. } | Valor::EnumNominal { .. }
    ) {
        return Err(ErrorVm::NoSoportado(format!(
            "BAS-7126: igualdad nominal no soportada en Fase 08 v1 para '{}'",
            if matches!(
                a,
                Valor::EstructuraNominal { .. } | Valor::EnumNominal { .. }
            ) {
                a.nombre_tipo()
            } else {
                b.nombre_tipo()
            }
        )));
    }
    if matches!(a, Valor::Nulo) || matches!(b, Valor::Nulo) {
        if matches!((a, b), (Valor::Nulo, Valor::Nulo)) {
            return Ok(Valor::Logico(igual));
        }
        return Err(ErrorVm::TiposIncompatibles {
            izquierdo: a.nombre_tipo(),
            derecho: b.nombre_tipo(),
        });
    }
    if matches!(a, Valor::Logico(_)) || matches!(b, Valor::Logico(_)) {
        return match (a, b) {
            (Valor::Logico(x), Valor::Logico(y)) => {
                Ok(Valor::Logico(if igual { x == y } else { x != y }))
            }
            _ => Err(ErrorVm::TiposIncompatibles {
                izquierdo: a.nombre_tipo(),
                derecho: b.nombre_tipo(),
            }),
        };
    }
    let eq = match (a, b) {
        (Valor::Entero(x), Valor::Entero(y)) => x == y,
        (Valor::Entero(x), Valor::Decimal(y)) => (*x as i128 * FACTOR_ESCALA) == *y,
        (Valor::Decimal(x), Valor::Entero(y)) => *x == (*y as i128 * FACTOR_ESCALA),
        (Valor::Entero(x), Valor::Real(y)) => (*x as f64) == *y,
        (Valor::Real(x), Valor::Entero(y)) => *x == (*y as f64),
        (Valor::Decimal(x), Valor::Decimal(y)) => x == y,
        (Valor::Real(x), Valor::Real(y)) => x == y,
        (Valor::Texto(x), Valor::Texto(y)) => x == y,
        _ => {
            return Err(ErrorVm::TiposIncompatibles {
                izquierdo: a.nombre_tipo(),
                derecho: b.nombre_tipo(),
            });
        }
    };
    Ok(Valor::Logico(if igual { eq } else { !eq }))
}

/// Convierte `v` al tipo anotado `t`, aplicando solo las conversiones
/// implícitas permitidas por ADR-0005 §1.4 / spec normativa §3.1 (únicamente
/// `entero→decimal` y `entero→real`). Cualquier otra incompatibilidad produce
/// `AsignacionIncompatible` (GAP-051), espejo estricto de
/// `evaluador::convertir_a_tipo_anotado` para paridad Tree-Walk↔VM.
/// `Funcion`/`Desconocido` se pasan sin restricción (sin anotación estática).
fn convertir_tipo(v: Valor, t: &crate::semantico::TipoSem) -> Result<Valor, ErrorVm> {
    use crate::semantico::TipoSem;
    let esperado = t.nombre();
    let obtenido = v.nombre_tipo().to_string();
    let incompatible = || ErrorVm::AsignacionIncompatible {
        esperado: esperado.clone(),
        obtenido: obtenido.clone(),
    };
    match t {
        TipoSem::Entero => match v {
            Valor::Entero(_) => Ok(v),
            _ => Err(incompatible()),
        },
        TipoSem::Decimal => match v {
            Valor::Decimal(_) => Ok(v),
            Valor::Entero(n) => Ok(Valor::decimal_desde_entero(n)),
            _ => Err(incompatible()),
        },
        TipoSem::Real => match v {
            Valor::Real(_) => Ok(v),
            Valor::Entero(n) => Ok(Valor::Real(n as f64)),
            _ => Err(incompatible()),
        },
        TipoSem::Logico => match v {
            Valor::Logico(_) => Ok(v),
            _ => Err(incompatible()),
        },
        TipoSem::Texto => match v {
            Valor::Texto(_) => Ok(v),
            _ => Err(incompatible()),
        },
        TipoSem::Nulo => match v {
            Valor::Nulo => Ok(v),
            _ => Err(incompatible()),
        },
        TipoSem::Lista(elem) => match v {
            Valor::Lista(elems) | Valor::Arreglo(elems) => {
                // Error de elemento se reporta a nivel contenedor (paridad TW:
                // convertir_a_tipo_anotado devuelve None y el llamador emite
                // E1004 con el tipo declarado completo).
                let elems = convertir_elementos(elems, elem).map_err(|_| incompatible())?;
                Ok(Valor::Lista(elems))
            }
            _ => Err(incompatible()),
        },
        TipoSem::Arreglo(elem, n) => match v {
            Valor::Lista(elems) | Valor::Arreglo(elems) if elems.len() == *n => {
                let elems = convertir_elementos(elems, elem).map_err(|_| incompatible())?;
                Ok(Valor::Arreglo(elems))
            }
            _ => Err(incompatible()),
        },
        TipoSem::Coleccion(elem, n) => match v {
            Valor::Lista(elems) | Valor::Arreglo(elems) if elems.len() == *n => {
                let elems = convertir_elementos(elems, elem).map_err(|_| incompatible())?;
                Ok(Valor::Lista(elems))
            }
            _ => Err(incompatible()),
        },
        TipoSem::Nominal(_) => Ok(v), // S8.1: validación nominal detallada en ejecución/semántica
        TipoSem::Funcion { .. } | TipoSem::Param(_) | TipoSem::Desconocido => Ok(v),
    }
}

fn convertir_elementos(
    elems: Vec<Valor>,
    elem: &crate::semantico::TipoSem,
) -> Result<Vec<Valor>, ErrorVm> {
    elems.into_iter().map(|e| convertir_tipo(e, elem)).collect()
}

impl MaquinaVirtual {
    fn convertir_tipo_opt(
        &self,
        v: Valor,
        t: Option<&crate::semantico::TipoSem>,
    ) -> Result<Valor, ErrorVm> {
        match t {
            Some(t) => convertir_tipo(v, t),
            None => Ok(v),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameDebug {
    func_idx: usize,
    locals: Vec<Valor>,
    block: usize,
    stmt: usize, // usize::MAX = terminador
    caller_dest: Option<MirPlace>,
}

#[derive(Debug, Clone)]
pub struct EstadoDebugVm {
    pub frames: Vec<FrameDebug>,
    pub terminado: bool,
    pub resultado: Option<Result<Option<String>, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventoDepuracionVm {
    Parado(MotivoParadaVm),
    Terminado(Result<Option<String>, String>),
}

impl MaquinaVirtual {
    pub fn debug_establecer_breakpoints(&mut self, bps: Vec<BreakpointVm>) {
        self.debug.breakpoints = bps.into_iter().collect();
    }
    pub fn debug_agregar_breakpoint(&mut self, bp: BreakpointVm) {
        self.debug.breakpoints.insert(bp);
    }
    pub fn debug_limpiar_breakpoints(&mut self) {
        self.debug.breakpoints.clear();
    }
    pub fn debug_pausar(&mut self) {
        self.debug.pausa_solicitada = true;
    }
    pub fn debug_modo_paso(&mut self, modo: ModoPasoVm) {
        self.debug.modo_paso = modo;
    }
    pub fn debug_pila(&self, estado: &EstadoDebugVm) -> Vec<FrameVm> {
        estado
            .frames
            .iter()
            .rev()
            .map(|f| {
                let func = self
                    .programa
                    .funciones
                    .get(f.func_idx)
                    .map(|x| x.nombre.clone())
                    .unwrap_or_else(|| "<desconocida>".to_string());
                let loc = self
                    .debug_spans
                    .get(&(f.func_idx, f.block, f.stmt))
                    .cloned();
                FrameVm {
                    funcion: func,
                    archivo: loc.as_ref().map(|x| x.0.clone()),
                    linea: loc.as_ref().map(|x| x.1),
                    columna: loc.as_ref().map(|x| x.2),
                }
            })
            .collect()
    }
    pub fn debug_variables_locales(
        &self,
        estado: &EstadoDebugVm,
        frame_idx_desde_tope: usize,
    ) -> Vec<VariableVm> {
        let Some(frame) = estado.frames.iter().rev().nth(frame_idx_desde_tope) else {
            return vec![];
        };
        let Some(func) = self.programa.funciones.get(frame.func_idx) else {
            return vec![];
        };
        let mut vars = Vec::new();
        for (i, (local, ty)) in func.params.iter().enumerate() {
            let nombre = format!("param{i}_l{}", local.0);
            if let Some(v) = frame.locals.get(local.0) {
                vars.push(VariableVm {
                    nombre,
                    valor: format!("{v}"),
                    tipo: format!("{:?}", ty),
                });
            }
        }
        for (i, v) in frame.locals.iter().enumerate() {
            if func.params.iter().any(|(l, _)| l.0 == i) {
                continue;
            }
            vars.push(VariableVm {
                nombre: format!("_l{i}"),
                valor: format!("{v}"),
                tipo: v.nombre_tipo().to_string(),
            });
        }
        vars
    }
    pub fn debug_variables_globales(&self) -> Vec<VariableVm> {
        self.globals
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let nombre = self
                    .debug_global_names
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("g{i}"));
                VariableVm {
                    nombre,
                    valor: format!("{v}"),
                    tipo: v.nombre_tipo().to_string(),
                }
            })
            .collect()
    }

    fn debug_ubicacion_actual(&self, estado: &EstadoDebugVm) -> Option<(String, usize)> {
        let f = estado.frames.last()?;
        let loc = self.debug_spans.get(&(f.func_idx, f.block, f.stmt))?;
        Some((loc.0.clone(), loc.1))
    }

    fn debug_debe_parar(&mut self, estado: &EstadoDebugVm) -> Option<MotivoParadaVm> {
        if self.debug.pausa_solicitada {
            self.debug.pausa_solicitada = false;
            return Some(MotivoParadaVm::Pause);
        }
        let (archivo, linea) = self.debug_ubicacion_actual(estado)?;
        if self.debug.ultima_linea.as_ref() == Some(&(archivo.clone(), linea)) {
            return None;
        }
        let bp = BreakpointVm {
            archivo: archivo.clone(),
            linea,
        };
        if self.debug.breakpoints.contains(&bp) {
            self.debug.ultima_linea = Some((archivo.clone(), linea));
            return Some(MotivoParadaVm::Breakpoint { archivo, linea });
        }
        let profundidad = estado.frames.len();
        match self.debug.modo_paso {
            ModoPasoVm::StepIn => {
                self.debug.modo_paso = ModoPasoVm::Ninguno;
                self.debug.ultima_linea = Some((archivo, linea));
                Some(MotivoParadaVm::Step)
            }
            ModoPasoVm::StepOver { profundidad: p } if profundidad <= p => {
                self.debug.modo_paso = ModoPasoVm::Ninguno;
                self.debug.ultima_linea = Some((archivo, linea));
                Some(MotivoParadaVm::Step)
            }
            ModoPasoVm::StepOut { profundidad: p } if profundidad < p => {
                self.debug.modo_paso = ModoPasoVm::Ninguno;
                self.debug.ultima_linea = Some((archivo, linea));
                Some(MotivoParadaVm::Step)
            }
            _ => None,
        }
    }

    pub fn debug_iniciar(
        &mut self,
        func_idx: usize,
        args: &[Valor],
    ) -> Result<EstadoDebugVm, ErrorVm> {
        if self.debug_spans.is_empty() && !self.programa.source_map.is_empty() {
            self.debug_spans = self.programa.source_map.clone();
        }
        self.inicializado = false;
        self.pasos = 0;
        self.profundidad = 0;
        self.try_stack.clear();
        self.sin_retorno_locals.clear();
        self.globals = vec![Valor::Nulo; self.programa.globals.len()];
        let mut estado = EstadoDebugVm {
            frames: Vec::new(),
            terminado: false,
            resultado: None,
        };
        self.debug_push_frame(&mut estado, func_idx, args)?;
        Ok(estado)
    }

    fn debug_push_frame(
        &mut self,
        estado: &mut EstadoDebugVm,
        func_idx: usize,
        args: &[Valor],
    ) -> Result<(), ErrorVm> {
        let Some(f) = self.programa.funciones.get(func_idx).cloned() else {
            return Err(ErrorVm::BloqueInalcanzable);
        };
        self.profundidad += 1;
        if self.profundidad > self.limite_profundidad {
            return Err(ErrorVm::ProfundidadExcedida);
        }
        if !self.inicializado {
            self.inicializado = true;
            if let Some(init_idx) = self.indice_de("__init")
                && init_idx != func_idx
            {
                // ejecutar init con motor normal para simplicidad
                let _ = self.ejecutar_indice(init_idx, &[])?;
            }
        }
        let mut locals = vec![Valor::Nulo; f.locals.len()];
        if args.len() < f.params.len() {
            return Err(ErrorVm::ArgumentosInsuficientes {
                faltantes: f.params.len() - args.len(),
            });
        }
        for (i, (local, _ty)) in f.params.iter().enumerate() {
            locals[local.0] = args[i].clone();
        }
        estado.frames.push(FrameDebug {
            func_idx,
            locals,
            block: f.bloque_entrada.0,
            stmt: 0,
            caller_dest: None,
        });
        Ok(())
    }

    /// Ejecuta un paso de depuración. Devuelve Some(Evento) si se detiene/termina, None si debe continuar.
    pub fn debug_paso(
        &mut self,
        estado: &mut EstadoDebugVm,
    ) -> Result<Option<EventoDepuracionVm>, ErrorVm> {
        if estado.terminado {
            return Ok(Some(EventoDepuracionVm::Terminado(
                estado.resultado.clone().unwrap_or(Ok(None)),
            )));
        }
        if let Some(motivo) = self.debug_debe_parar(estado) {
            return Ok(Some(EventoDepuracionVm::Parado(motivo)));
        }

        let frame_idx = match estado.frames.len().checked_sub(1) {
            Some(i) => i,
            None => {
                estado.terminado = true;
                estado.resultado = Some(Ok(None));
                return Ok(Some(EventoDepuracionVm::Terminado(Ok(None))));
            }
        };
        let (func_idx, block_idx, stmt_idx) = {
            let f = &estado.frames[frame_idx];
            (f.func_idx, f.block, f.stmt)
        };
        let func = self
            .programa
            .funciones
            .get(func_idx)
            .cloned()
            .ok_or(ErrorVm::BloqueInalcanzable)?;
        if block_idx >= func.bloques.len() {
            return Err(ErrorVm::BloqueInalcanzable);
        }
        let bloque = func.bloques[block_idx].clone();

        if stmt_idx < bloque.stmts.len() {
            let stmt = bloque.stmts[stmt_idx].clone();
            // ejecutar stmt en frame actual
            let res = {
                let locals = &mut estado.frames[frame_idx].locals;
                let local_types = &func.locals;
                match &stmt {
                    MirStmt::Llamada {
                        destino,
                        callee,
                        argumentos,
                    } => {
                        let args = self.evaluar_argumentos(locals, argumentos)?;
                        if self.indice.contains_key(callee) && !self.nativos.contains_key(callee) {
                            let callee_idx = self.indice[callee];
                            // guardar destino en frame y empujar nuevo frame
                            estado.frames[frame_idx].stmt = stmt_idx + 1;
                            estado.frames[frame_idx].caller_dest = destino.clone();
                            self.debug_push_frame(estado, callee_idx, &args)?;
                            return Ok(self
                                .debug_debe_parar(estado)
                                .map(EventoDepuracionVm::Parado));
                        } else {
                            let r = self.llamar(callee, &args)?;
                            if let Some(dest) = destino {
                                if self.funciones_sin_retorno.contains(callee) {
                                    self.sin_retorno_locals.insert(dest.local.0);
                                }
                                self.escribir_place(locals, local_types, dest, r)?;
                            }
                            Ok(())
                        }
                    }
                    MirStmt::LlamadaIndirecta {
                        destino,
                        callee,
                        argumentos,
                    } => {
                        let callee_val = self.operando_a_valor(locals, callee)?;
                        let nombre = match &callee_val {
                            Valor::Funcion(n) => n.clone(),
                            Valor::ClausuraMir { nombre, .. } => nombre.clone(),
                            otro => {
                                return Err(ErrorVm::TipoEsperado {
                                    esperado: "función",
                                    encontrado: otro.nombre_tipo(),
                                });
                            }
                        };
                        let args_usuario = self.evaluar_argumentos(locals, argumentos)?;
                        let args = if let Valor::ClausuraMir { capturas, .. } = &callee_val {
                            let mut full = args_usuario;
                            full.extend(capturas.clone());
                            full
                        } else {
                            args_usuario
                        };
                        if self.indice.contains_key(&nombre) && !self.nativos.contains_key(&nombre)
                        {
                            let callee_idx = self.indice[&nombre];
                            estado.frames[frame_idx].stmt = stmt_idx + 1;
                            estado.frames[frame_idx].caller_dest = destino.clone();
                            self.debug_push_frame(estado, callee_idx, &args)?;
                            return Ok(self
                                .debug_debe_parar(estado)
                                .map(EventoDepuracionVm::Parado));
                        } else {
                            let r = self.llamar(&nombre, &args)?;
                            if let Some(dest) = destino {
                                self.escribir_place(locals, local_types, dest, r)?;
                            }
                            Ok(())
                        }
                    }
                    _ => self.ejecutar_stmt(locals, local_types, &stmt),
                }
            };
            if let Err(e) = res {
                if let Some((rescue, _merge, var_local)) = self.try_stack.last().cloned() {
                    if let Some(vl) = var_local
                        && let Some(fr) = estado.frames.get_mut(frame_idx)
                        && vl < fr.locals.len()
                    {
                        fr.locals[vl] = Valor::Texto(e.to_string());
                    }
                    self.try_stack.pop();
                    if let Some(fr) = estado.frames.get_mut(frame_idx) {
                        fr.block = rescue.0;
                        fr.stmt = 0;
                    }
                    return Ok(self
                        .debug_debe_parar(estado)
                        .map(EventoDepuracionVm::Parado));
                }
                estado.terminado = true;
                estado.resultado = Some(Err(e.to_string()));
                return Ok(Some(EventoDepuracionVm::Terminado(Err(e.to_string()))));
            }
            if let Some(f) = estado.frames.get_mut(frame_idx) {
                f.stmt = stmt_idx + 1;
            }
            return Ok(self
                .debug_debe_parar(estado)
                .map(EventoDepuracionVm::Parado));
        }

        // terminador
        let term = bloque.terminator.clone();
        let trans = {
            let locals = &mut estado.frames[frame_idx].locals;
            self.ejecutar_terminador(locals, &func, &term)
        };
        match trans {
            Ok(Transicion::Salto(nuevo)) => {
                if let Some(f) = estado.frames.get_mut(frame_idx) {
                    f.block = nuevo.0;
                    f.stmt = 0;
                }
                Ok(self
                    .debug_debe_parar(estado)
                    .map(EventoDepuracionVm::Parado))
            }
            Ok(Transicion::Retornar(valor)) => {
                let ret = valor.unwrap_or(Valor::Nulo);
                estado.frames.pop();
                self.profundidad = self.profundidad.saturating_sub(1);
                if let Some(caller) = estado.frames.last_mut() {
                    if let Some(dest) = caller.caller_dest.clone() {
                        let caller_func_idx = caller.func_idx;
                        let caller_func = self
                            .programa
                            .funciones
                            .get(caller_func_idx)
                            .cloned()
                            .ok_or(ErrorVm::BloqueInalcanzable)?;
                        let locals = &mut caller.locals;
                        self.escribir_place(locals, &caller_func.locals, &dest, ret)?;
                        caller.caller_dest = None;
                    }
                    Ok(self
                        .debug_debe_parar(estado)
                        .map(EventoDepuracionVm::Parado))
                } else {
                    estado.terminado = true;
                    let out = Some(format!("{}", ret));
                    estado.resultado = Some(Ok(Some(format!("{}", ret))));
                    Ok(Some(EventoDepuracionVm::Terminado(Ok(out))))
                }
            }
            Err(e) => {
                if let Some((rescue, _merge, var_local)) = self.try_stack.last().cloned() {
                    if let Some(vl) = var_local
                        && let Some(fr) = estado.frames.get_mut(frame_idx)
                        && vl < fr.locals.len()
                    {
                        fr.locals[vl] = Valor::Texto(e.to_string());
                    }
                    self.try_stack.pop();
                    if let Some(fr) = estado.frames.get_mut(frame_idx) {
                        fr.block = rescue.0;
                        fr.stmt = 0;
                    }
                    Ok(self
                        .debug_debe_parar(estado)
                        .map(EventoDepuracionVm::Parado))
                } else {
                    estado.terminado = true;
                    estado.resultado = Some(Err(e.to_string()));
                    Ok(Some(EventoDepuracionVm::Terminado(Err(e.to_string()))))
                }
            }
        }
    }

    pub fn debug_continuar(
        &mut self,
        estado: &mut EstadoDebugVm,
    ) -> Result<EventoDepuracionVm, ErrorVm> {
        loop {
            match self.debug_paso(estado)? {
                Some(ev) => return Ok(ev),
                None => continue,
            }
        }
    }
}
