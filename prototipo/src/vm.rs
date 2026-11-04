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
use std::sync::{LazyLock, Mutex};

/// Valor de retorno de una tarea: `Ok(valor)` o `Err(mensaje)` (como en `evaluador`).
type ResultadoTarea = Result<Valor, String>;
/// Handle de tarea esperable (`take` + `join`), análogo al del Tree-Walk.
type TareaPendiente = Option<std::thread::JoinHandle<ResultadoTarea>>;

/// Registro global de tareas de la VM (`iniciar_tarea` / `esperar_tarea`),
/// análogo al del evaluador (Tree-Walk).
static TAREAS_VM: LazyLock<Mutex<Vec<TareaPendiente>>> = LazyLock::new(|| Mutex::new(Vec::new()));

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
}

impl std::fmt::Display for ErrorVm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorVm::FuncionNoEncontrada(nombre) => write!(f, "función no encontrada: {nombre}"),
            ErrorVm::ArgumentosInsuficientes { faltantes } => {
                write!(f, "argumentos insuficientes ({faltantes} faltantes)")
            }
            ErrorVm::LimitePasosExcedido => write!(f, "límite de pasos excedido"),
            ErrorVm::ProfundidadExcedida => write!(f, "límite de profundidad excedido"),
            ErrorVm::BloqueInalcanzable => write!(f, "bloque inalcanzable"),
            ErrorVm::Aritmetico(m) => write!(f, "error aritmético: {m}"),
            ErrorVm::TiposIncompatibles { izquierdo, derecho } => {
                write!(f, "tipos incompatibles: {izquierdo} y {derecho}")
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

/// Máquina virtual. Interpreta un `MirPrograma`.
pub struct MaquinaVirtual {
    programa: MirPrograma,
    /// Índice `nombre -> posición en programa.funciones` (la primera gana).
    indice: HashMap<String, usize>,
    /// Funciones nativas (FFI / integradas) registradas en la VM.
    nativos: HashMap<String, FuncionNativa>,
    /// Salida capturada de `mostrar`/`mostrar_sin_salto` (biblioteca estándar).
    salida: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// Pila de regiones try activas.
    try_stack: Vec<(BlockId, BlockId, Option<usize>)>,
    pasos: usize,
    limite_pasos: usize,
    profundidad: usize,
    limite_profundidad: usize,
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
        let mut indice = HashMap::new();
        for (i, f) in programa.funciones.iter().enumerate() {
            indice.entry(f.nombre.clone()).or_insert(i);
        }
        MaquinaVirtual {
            programa,
            indice,
            nativos: HashMap::new(),
            salida: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            try_stack: Vec::new(),
            pasos: 0,
            limite_pasos,
            profundidad: 0,
            limite_profundidad,
        }
    }

    /// Número de funciones del programa.
    pub fn numero_funciones(&self) -> usize {
        self.programa.funciones.len()
    }

    /// Índice de la función por nombre (`f_{simbodo}` según el lowering).
    pub fn indice_de(&self, nombre: &str) -> Option<usize> {
        self.indice.get(nombre).copied()
    }

    /// Registra una función nativa (FFI / integrada) bajo `nombre`.
    /// Tiene prioridad menor que una función MIR del programa con el mismo
    /// nombre (la primera gana en el intérprete).
    pub fn registrar_nativo(&mut self, nombre: impl Into<String>, f: FuncionNativa) {
        self.nativos.entry(nombre.into()).or_insert(f);
    }

    /// Registra la biblioteca estándar mínima (mostrar, mostrar_sin_salto, longitud, etc.).
    pub fn registrar_stdlib(&mut self) {
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
                Ok(Valor::Nulo)
            }),
        );
        self.registrar_nativo(
            "cerrar",
            std::sync::Arc::new(|args: &[Valor]| match args.first() {
                Some(Valor::Archivo(arc)) => {
                    let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
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

    /// Número de funciones nativas registradas.
    pub fn numero_nativos(&self) -> usize {
        self.nativos.len()
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
        // Se clona la función para evitar préstamos cruzados con el estado de la
        // VM (los programas del prototipo son pequeños; en bytecode real el
        // código será compartido, no clonado).
        let Some(funcion) = self.programa.funciones.get(idx).cloned() else {
            return Err(ErrorVm::BloqueInalcanzable);
        };
        let r = self.correr_funcion(&funcion, args);
        self.profundidad -= 1;
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
                if let Err(e) = self.ejecutar_stmt(&mut locals, stmt) {
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
            match self.ejecutar_terminador(&mut locals, &bloque.terminator) {
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
    fn ejecutar_stmt(&mut self, locals: &mut [Valor], stmt: &MirStmt) -> Result<(), ErrorVm> {
        match stmt {
            MirStmt::Asignacion { destino, valor } => {
                let v = self.evaluar_rvalue(locals, valor)?;
                self.escribir_place(locals, destino, v)
            }
            MirStmt::Llamada {
                destino,
                callee,
                argumentos,
            } => {
                let args = self.evaluar_argumentos(locals, argumentos)?;
                let r = self.llamar(callee, &args)?;
                if let Some(dest) = destino {
                    self.escribir_place(locals, dest, r)?;
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
                let mut args = self.evaluar_argumentos(locals, argumentos)?;
                if let Valor::ClausuraMir { capturas, .. } = &callee_val {
                    let mut full = args.clone();
                    full.extend(capturas.clone());
                    args = full;
                }
                let r = self.llamar(&nombre, &args)?;
                if let Some(dest) = destino {
                    self.escribir_place(locals, dest, r)?;
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
                            self.escribir_place(locals, dest, v)?;
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
            let devuelto = self.ejecutar(callee, args)?;
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
            return Err(ErrorVm::FuncionNoEncontrada(func_name));
        }
        let programa = self.programa.clone();
        let handle = std::thread::spawn(move || {
            let mut vm = MaquinaVirtual::nuevo(programa);
            vm.registrar_stdlib();
            vm.ejecutar(&func_name, &[])
                .map(|v| v.unwrap_or(Valor::Nulo))
                .map_err(|e| e.to_string())
        });
        let mut tareas = TAREAS_VM.lock().unwrap();
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
            let mut tareas = TAREAS_VM.lock().unwrap();
            if id >= tareas.len() {
                return Err(ErrorVm::FuncionNoEncontrada(format!("tarea {id}")));
            }
            tareas[id].take()
        };
        match handle {
            Some(h) => match h.join() {
                Ok(Ok(v)) => Ok(v),
                Ok(Err(_)) => Err(ErrorVm::TipoEsperado {
                    esperado: "tarea exitosa",
                    encontrado: "error en tarea",
                }),
                Err(_) => Err(ErrorVm::TipoEsperado {
                    esperado: "tarea exitosa",
                    encontrado: "tarea cancelada",
                }),
            },
            None => Err(ErrorVm::TipoEsperado {
                esperado: "tarea pendiente",
                encontrado: "tarea ya esperada",
            }),
        }
    }

    /// Lee un operando: clona el local o materializa la constante.
    fn operando_a_valor(&self, locals: &[Valor], op: &MirOperand) -> Result<Valor, ErrorVm> {
        match op {
            MirOperand::Copy(local) | MirOperand::Move(local) => {
                locals.get(local.0).cloned().ok_or(ErrorVm::TipoEsperado {
                    esperado: "local",
                    encontrado: "vacío",
                })
            }
            MirOperand::Const(c) => Ok(const_a_valor(c)),
        }
    }

    /// Escribe en un `MirPlace` (local + proyecciones de un solo nivel).
    fn escribir_place(
        &self,
        locals: &mut [Valor],
        place: &MirPlace,
        valor: Valor,
    ) -> Result<(), ErrorVm> {
        let indice = match place.projection.as_slice() {
            [MirProjection::Indice(op)] => {
                let Valor::Entero(i) = self.operando_a_valor(locals, op)? else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "entero",
                        encontrado: "índice no entero",
                    });
                };
                if i < 0 {
                    return Err(ErrorVm::IndiceFueraDeRango {
                        indice: i,
                        tamano: 0,
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
                otro => Err(ErrorVm::TipoEsperado {
                    esperado: "objeto",
                    encontrado: otro.nombre_tipo(),
                }),
            },
            [MirProjection::Indice(_)] => {
                let idx = indice.expect("resuelto arriba");
                match local_val {
                    Valor::Arreglo(elems) | Valor::Lista(elems) => {
                        if idx >= elems.len() {
                            return Err(ErrorVm::IndiceFueraDeRango {
                                indice: idx as i64,
                                tamano: elems.len(),
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
                let a = self.operando_a_valor(locals, lhs)?;
                let b = self.operando_a_valor(locals, rhs)?;
                aplicar_binop(op, &a, &b)
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
                Ok(Valor::Arreglo(elems?))
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
                            return Err(ErrorVm::IndiceFueraDeRango {
                                indice: idx,
                                tamano: elems.len(),
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
        MirBinOp::Igual => Ok(Valor::Logico(a == b)),
        MirBinOp::Distinto => Ok(Valor::Logico(a != b)),
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

/// Comparaciones numéricas/texto (entero↔decimal/real implícito, ADR-0005 §1.4).
fn comparar(
    a: &Valor,
    b: &Valor,
    acepta: impl Fn(std::cmp::Ordering) -> bool,
) -> Result<Valor, ErrorVm> {
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
