//! Evaluador del prototipo de BASILISCO.
//!
//! Semántica numérica normativa (ADR-0005, aprobado):
//! - Tres tipos: `entero` (i64), `decimal` (DECIMAL(28,9)), `real` (f64).
//! - Desbordamiento entero = error; división por cero = error en los tres tipos.
//! - `%` de enteros = resto truncado, signo del dividendo; `i64::MIN % -1` = 0.
//! - Decimal: redondeo half-even, exceso de precisión redondea, exceso de rango = error.
//! - Conversiones: solo implícitas sin pérdida (`entero→decimal`, `entero→real`).

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;

use crate::cargador::{CargadorModulos, SimbodoExportado};
use crate::parser::{Expresion, Sentencia, TipoAnotacion};
use crate::valor::{FACTOR_ESCALA, MotivoAritmetico, Valor};
use crate::{Lexer, Parser};
use std::fs;

/// Causa tipada de un error de ejecución (ADR-0005 §1.6 y ADR-0006 §1.5, ampliada ADR-0007).
#[derive(Debug, Clone, PartialEq)]
pub enum CausaError {
    /// Motivo aritmético o de tipos del sistema numérico (ADR-0005).
    Aritmetica(MotivoAritmetico),
    /// Referencia o asignación a un nombre no declarado (ADR-0006).
    VariableNoDefinida(String),
    /// `sea nombre` repetido en el mismo ámbito (ADR-0006).
    DeclaracionDuplicada(String),
    /// Asignación con un tipo distinto al declarado y sin conversión implícita.
    AsignacionIncompatible { esperado: String, obtenido: String },
    /// Reasignación de una variable inmutable (`dato`/`const`).
    ReasignacionInmutable { nombre: String },
    /// Condición de `si`/`mientras` no es `lógico` (ADR-0007).
    CondicionNoLogica { obtenido: &'static str },
    /// Operando de `no`/`y`/`o` no es `lógico` (ADR-0007).
    OperandoNoLogico {
        operador: &'static str,
        obtenido: &'static str,
    },
    /// Comparación con tipos incompatibles sin conversión implícita aprobada (ADR-0007).
    TiposIncompatibles {
        izquierdo: &'static str,
        derecho: &'static str,
    },
    /// Se superó el límite de iteraciones de un `mientras` (protección frente a bucles
    /// infinitos; ADR-0007 §1.5).
    LimiteIteraciones { limite: usize },
    /// Número incorrecto de argumentos en una llamada (ADR-0008).
    FirmaInvalida {
        funcion: String,
        esperado: usize,
        obtenido: usize,
    },
    /// Se usa el resultado de una función que no ejecuta `retornar` (ADR-0008).
    RetornoAusente { funcion: String },
    /// Tipo de retorno incompatible con el fijado en la primera llamada (ADR-0008).
    TipoDeRetornoIncompatible {
        esperado: &'static str,
        obtenido: &'static str,
    },
    /// `retornar` usado fuera de una función (nivel superior).
    RetornarFueraDeBloque,
    /// Argumento de tipo incompatible en llamada tipada (ADR-0011 §4.3).
    TipoParametroIncompatible {
        funcion: String,
        parametro: String,
        esperado: String,
        obtenido: String,
    },
    /// Retorno incompatible con el tipo anotado (ADR-0011 §4.3).
    RetornoIncompatibleConAnotacion {
        funcion: String,
        esperado: String,
        obtenido: String,
    },
    /// Señal interna: `romper` dentro de ciclo (ADR-0012). Interceptada por ciclos.
    SenalRomper,
    /// Señal interna: `continuar` dentro de ciclo (ADR-0012). Interceptada por ciclos.
    SenalContinuar,
    /// Iteración sobre valor que no es colección ni rango (ADR-0012 §4.4).
    IteracionSobreNoColeccion { obtenido: &'static str },
    /// Extremos del rango no son enteros (ADR-0012 §4.4).
    RangoTipoInvalido { obtenido: &'static str },
    /// Modificación de colección durante iteración (ADR-0012 §4.2).
    ModificacionDuranteIteracion { variable: String },
    /// Nombre reservado de función integrada (ADR-0013 §3.1).
    NombreReservado { nombre: String },
    /// Entrada inválida desde stdin (ADR-0013 §3.2).
    EntradaInvalida { esperado: String, obtenido: String },
    /// Archivo no encontrado (ADR-0013 §3.7).
    ArchivoNoEncontrado { ruta: String },
    /// Permisos denegados (ADR-0013 §3.7).
    PermisosDenegados { ruta: String },
    /// Error genérico de archivo (ADR-0013 §3.7).
    ErrorDeArchivo { ruta: String, detalle: String },
    /// Operación sobre archivo ya cerrado (ADR-0013 §3.7).
    ArchivoYaCerrado,
    /// Índice de texto fuera de rango (ADR-0013 §3.7).
    IndiceTextoFueraDeRango { indice: i64, longitud: usize },
    /// Conversión de texto a número fallida (ADR-0013 §3.7).
    ConversionTextoInvalida { valor: String, esperado: String },
    /// Separador vacío en `dividir` (ADR-0013 §3.7).
    SeparadorVacio,
    /// Módulo no encontrado (ADR-0017 D12).
    ModuloNoEncontrado { ruta: String },
    /// Ciclo de importación detectado (ADR-0017 D12).
    CicloDeImportacion { cadena: Vec<String> },
    /// Símbodo privado (ADR-0017 D12).
    SimbodoPrivado { nombre: String, modulo: String },
    /// Símbodo no exportado (ADR-0017 D12).
    SimbodoNoExportado { nombre: String, modulo: String },
    /// Importación duplicada (ADR-0017 D12).
    ImportacionDuplicada { nombre: String, modulo: String },
    /// Ruta no permitida (fuera de la raíz del proyecto o absoluta) (ADR-0017 D12).
    RutaNoPermitida { ruta: String },
    /// Extensión no soportada en importación (ADR-0017 D12).
    ExtensionNoSoportada { extension: String },
    /// Importaciones no disponibles (sin cargador configurado) (ADR-0017 D12).
    ImportacionesNoDisponibles,
    /// Sentencia no permitida en el nivel superior de un módulo (ADR-0017 D4).
    SentenciaNoPermitidaEnModulo { sentencia: String },
}

/// Límite de iteraciones por ciclo `mientras` (protección frente a bucles infinitos).
pub const LIMITE_ITERACIONES: usize = 100_000;

static IMPORTANDO: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static CACHE_MODULOS: LazyLock<Mutex<HashMap<PathBuf, Entorno>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
#[allow(clippy::type_complexity)]
static TAREAS: LazyLock<Mutex<Vec<Option<thread::JoinHandle<Result<Valor, String>>>>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Nombres reservados que no pueden reutilizarse como funciones de usuario.
pub const NOMBRES_RESERVADOS: &[&str] = &[
    "si",
    "sino",
    "mientras",
    "mostrar",
    "mostrar_sin_salto",
    "retornar",
    "funcion",
    "sea",
    "var",
    "dato",
    "const",
    "para",
    "cada",
    "en",
    "romper",
    "continuar",
    "intentar",
    "rescatar",
    "leer",
    "leer_entero",
    "leer_decimal",
    "leer_real",
    "longitud",
    "contiene",
    "empieza_con",
    "termina_con",
    "es_vacio",
    "mayusculas",
    "minusculas",
    "recortar",
    "reemplazar",
    "subcadena",
    "dividir",
    "unir",
    "a_entero",
    "a_decimal",
    "a_real",
    "texto",
    "abrir",
    "leer_linea",
    "leer_todo",
    "escribir",
    "escribir_linea",
    "cerrar",
    "fin_de_archivo",
    "abs",
    "max",
    "min",
    "raiz",
    "potencia",
    "redondear",
    "map",
    "filtrar",
    "reducir",
    "piso",
    "techo",
    "posicion",
    "repetir",
    "agregar",
    "eliminar",
    "invertir",
    "ordenar",
    "existe_archivo",
    "leer_archivo",
    "escribir_archivo",
    "iniciar_tarea",
    "esperar_tarea",
];

impl CausaError {
    /// Código estable para catálogo de errores (GAP-004).
    pub fn codigo(&self) -> &'static str {
        match self {
            CausaError::Aritmetica(_) => "E1001",
            CausaError::VariableNoDefinida(_) => "E1002",
            CausaError::DeclaracionDuplicada(_) => "E1003",
            CausaError::AsignacionIncompatible { .. } => "E1004",
            CausaError::ReasignacionInmutable { .. } => "E1029",
            CausaError::CondicionNoLogica { .. } => "E1005",
            CausaError::OperandoNoLogico { .. } => "E1006",
            CausaError::TiposIncompatibles { .. } => "E1007",
            CausaError::LimiteIteraciones { .. } => "E1008",
            CausaError::FirmaInvalida { .. } => "E1009",
            CausaError::RetornoAusente { .. } => "E1010",
            CausaError::TipoDeRetornoIncompatible { .. } => "E1011",
            CausaError::RetornarFueraDeBloque => "E1012",
            CausaError::TipoParametroIncompatible { .. } => "E1013",
            CausaError::RetornoIncompatibleConAnotacion { .. } => "E1014",
            CausaError::SenalRomper => "E1015",
            CausaError::SenalContinuar => "E1016",
            CausaError::IteracionSobreNoColeccion { .. } => "E1017",
            CausaError::RangoTipoInvalido { .. } => "E1018",
            CausaError::ModificacionDuranteIteracion { .. } => "E1019",
            CausaError::NombreReservado { .. } => "E1020",
            CausaError::EntradaInvalida { .. } => "E1021",
            CausaError::ArchivoNoEncontrado { .. } => "E1022",
            CausaError::PermisosDenegados { .. } => "E1023",
            CausaError::ErrorDeArchivo { .. } => "E1024",
            CausaError::ArchivoYaCerrado => "E1025",
            CausaError::IndiceTextoFueraDeRango { .. } => "E1026",
            CausaError::ConversionTextoInvalida { .. } => "E1027",
            CausaError::SeparadorVacio => "E1028",
            CausaError::ModuloNoEncontrado { .. } => "E1030",
            CausaError::CicloDeImportacion { .. } => "E1031",
            CausaError::SimbodoPrivado { .. } => "E1032",
            CausaError::SimbodoNoExportado { .. } => "E1033",
            CausaError::ImportacionDuplicada { .. } => "E1034",
            CausaError::ExtensionNoSoportada { .. } => "E1035",
            CausaError::RutaNoPermitida { .. } => "E1036",
            CausaError::SentenciaNoPermitidaEnModulo { .. } => "E1037",
            CausaError::ImportacionesNoDisponibles => "E1038",
        }
    }
}

impl std::fmt::Display for CausaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CausaError::Aritmetica(m) => write!(f, "{m}"),
            CausaError::VariableNoDefinida(n) => write!(f, "variable no definida: {n}"),
            CausaError::DeclaracionDuplicada(n) => write!(f, "declaración duplicada: {n}"),
            CausaError::AsignacionIncompatible { esperado, obtenido } => write!(
                f,
                "asignación incompatible: se esperaba {esperado} y se obtuvo {obtenido}"
            ),
            CausaError::ReasignacionInmutable { nombre } => {
                write!(f, "no se puede reasignar la variable inmutable '{nombre}'")
            }
            CausaError::CondicionNoLogica { obtenido } => {
                write!(f, "la condición debe ser lógica, se obtuvo {obtenido}")
            }
            CausaError::OperandoNoLogico { operador, obtenido } => write!(
                f,
                "el operando de '{operador}' debe ser lógico, se obtuvo {obtenido}"
            ),
            CausaError::TiposIncompatibles { izquierdo, derecho } => {
                write!(f, "comparación incompatible entre {izquierdo} y {derecho}")
            }
            CausaError::LimiteIteraciones { limite } => write!(
                f,
                "se superó el límite de {limite} iteraciones: posible bucle infinito"
            ),
            CausaError::FirmaInvalida {
                funcion,
                esperado,
                obtenido,
            } => write!(
                f,
                "la función '{funcion}' espera {esperado} argumentos y se le pasaron {obtenido}"
            ),
            CausaError::RetornoAusente { funcion } => {
                write!(f, "la función '{funcion}' no ejecuta 'retornar'")
            }
            CausaError::TipoDeRetornoIncompatible { esperado, obtenido } => write!(
                f,
                "el tipo de retorno es incompatible: se esperaba {esperado} y se obtuvo {obtenido}"
            ),
            CausaError::RetornarFueraDeBloque => {
                write!(f, "'retornar' solo puede usarse dentro de una función")
            }
            CausaError::TipoParametroIncompatible {
                funcion,
                parametro,
                esperado,
                obtenido,
            } => write!(
                f,
                "parámetro '{parametro}' de '{funcion}': se esperaba {esperado} y se obtuvo {obtenido}"
            ),
            CausaError::RetornoIncompatibleConAnotacion {
                funcion,
                esperado,
                obtenido,
            } => write!(
                f,
                "retorno de '{funcion}': se esperaba {esperado} y se obtuvo {obtenido}"
            ),
            CausaError::SenalRomper => write!(f, "romper fuera de ciclo (señal interna)"),
            CausaError::SenalContinuar => write!(f, "continuar fuera de ciclo (señal interna)"),
            CausaError::IteracionSobreNoColeccion { obtenido } => {
                write!(f, "no se puede iterar sobre un valor de tipo {obtenido}")
            }
            CausaError::RangoTipoInvalido { obtenido } => write!(
                f,
                "los extremos del rango deben ser enteros, se obtuvo {obtenido}"
            ),
            CausaError::ModificacionDuranteIteracion { variable } => {
                write!(f, "no se puede modificar '{variable}' durante la iteración")
            }
            CausaError::NombreReservado { nombre } => write!(
                f,
                "'{nombre}' es el nombre de una función integrada y no puede redefinirse"
            ),
            CausaError::EntradaInvalida { esperado, obtenido } => write!(
                f,
                "entrada inválida: se esperaba {esperado}, se leyó {obtenido:?}"
            ),
            CausaError::ArchivoNoEncontrado { ruta } => write!(f, "archivo no encontrado: {ruta}"),
            CausaError::PermisosDenegados { ruta } => {
                write!(f, "permisos denegados al acceder a: {ruta}")
            }
            CausaError::ErrorDeArchivo { ruta, detalle } => {
                write!(f, "error de archivo '{ruta}': {detalle}")
            }
            CausaError::ArchivoYaCerrado => write!(f, "el archivo ya fue cerrado"),
            CausaError::IndiceTextoFueraDeRango { indice, longitud } => write!(
                f,
                "índice de texto {indice} fuera de rango (longitud {longitud})"
            ),
            CausaError::ConversionTextoInvalida { valor, esperado } => {
                write!(f, "no se puede convertir \"{valor}\" a {esperado}")
            }
            CausaError::SeparadorVacio => write!(f, "el separador no puede ser vacío"),
            CausaError::ModuloNoEncontrado { ruta } => write!(f, "módulo no encontrado: {ruta}"),
            CausaError::CicloDeImportacion { cadena } => {
                write!(f, "ciclo de importación: {}", cadena.join(" -> "))
            }
            CausaError::SimbodoPrivado { nombre, modulo } => {
                write!(
                    f,
                    "el símbodo '{nombre}' no es público en el módulo '{modulo}'"
                )
            }
            CausaError::SimbodoNoExportado { nombre, modulo } => {
                write!(f, "el símbodo '{nombre}' no existe en el módulo '{modulo}'")
            }
            CausaError::ImportacionDuplicada { nombre, modulo } => {
                write!(
                    f,
                    "importación duplicada: '{nombre}' ya existe (módulo '{modulo}')"
                )
            }
            CausaError::RutaNoPermitida { ruta } => write!(f, "ruta no permitida: {ruta}"),
            CausaError::ExtensionNoSoportada { extension } => {
                write!(f, "extensión no soportada: {extension}")
            }
            CausaError::ImportacionesNoDisponibles => {
                write!(f, "importaciones no disponibles en este contexto")
            }
            CausaError::SentenciaNoPermitidaEnModulo { sentencia } => {
                write!(
                    f,
                    "sentencia no permitida en el nivel superior de un módulo: {sentencia}"
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorEjecucion {
    pub mensaje: String,
    pub causa: Box<CausaError>,
    pub linea: usize,
    pub columna: usize,
    /// Archivo de origen, si está disponible (ADR-0017 D5).
    pub archivo: Option<String>,
}

impl std::fmt::Display for ErrorEjecucion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.archivo {
            Some(archivo) => write!(
                f,
                "error de ejecución [{}] [{}:{}:{}]: {}",
                self.causa.codigo(),
                archivo,
                self.linea,
                self.columna,
                self.mensaje
            ),
            None => write!(
                f,
                "error de ejecución [{}] [{}:{}]: {}",
                self.causa.codigo(),
                self.linea,
                self.columna,
                self.mensaje
            ),
        }
    }
}

fn error(causa: CausaError, linea: usize, columna: usize) -> ErrorEjecucion {
    ErrorEjecucion {
        mensaje: causa.to_string(),
        causa: Box::new(causa),
        linea,
        columna,
        archivo: None,
    }
}

/// Información almacenada para cada variable (valor + metadatos de declaración).
#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub valor: Valor,
    /// Tipo declarado (None = inferido en declaración, como `sea` legacy).
    pub tipo_declarado: Option<TipoAnotacion>,
    /// Si la variable es mutable (`var`/`sea` = true, `dato`/`const` = false).
    pub mutable: bool,
    /// Si es constante de compilación (`const` = true).
    pub es_const: bool,
}

/// Definición de función almacenada en el entorno (ADR-0008, ampliada ADR-0011).
#[derive(Debug, Clone)]
pub struct FuncionDef {
    pub nombre: String,
    /// Parámetros con tipo anotado opcional (ADR-0011 §3.2).
    pub parametros: Vec<(String, Option<TipoAnotacion>)>,
    pub cuerpo: Vec<Sentencia>,
    pub linea: usize,
    pub columna: usize,
    /// Tipo de retorno anotado por el programador (ADR-0011 §3.4).
    pub tipo_retorno_anotado: Option<TipoAnotacion>,
    /// Tipo de retorno inferido en la primera llamada exitosa (ADR-0008).
    /// `None` = aún no se ha llamado o no retornó valor.
    pub tipo_retorno: Option<&'static str>,
    /// Tipos inferidos para parámetros sin anotación, fijados en la primera llamada.
    pub tipos_parametros_inferidos: Option<Vec<Valor>>,
}

/// Entorno de ejecución: pila de ámbitos con las variables (ADR-0006 §1.6),
/// más un mapa global de funciones (ADR-0008).
#[derive(Debug, Default, Clone)]
pub struct Entorno {
    ambitos: Vec<HashMap<String, VariableInfo>>,
    funciones: HashMap<String, FuncionDef>,
    /// Nombres exportados por el módulo en ejecución.
    exportaciones: HashSet<String>,
    /// Cargador de módulos compartido (ADR-0017, Fase 3).
    pub cargador: Option<Arc<Mutex<CargadorModulos>>>,
    /// Archivo de origen asociado a este entorno (para diagnósticos).
    pub archivo: Option<String>,
    /// Indica si se está ejecutando dentro de una función.
    pub en_funcion: bool,
}

impl Entorno {
    pub fn nueva() -> Self {
        Entorno {
            ambitos: vec![HashMap::new()],
            funciones: HashMap::new(),
            exportaciones: HashSet::new(),
            cargador: None,
            archivo: None,
            en_funcion: false,
        }
    }

    /// Crea un entorno con un cargador de módulos compartido.
    pub fn con_cargador(cargador: Arc<Mutex<CargadorModulos>>, archivo: Option<String>) -> Self {
        Entorno {
            ambitos: vec![HashMap::new()],
            funciones: HashMap::new(),
            exportaciones: HashSet::new(),
            cargador: Some(cargador),
            archivo,
            en_funcion: false,
        }
    }

    /// Inserta una variable en el ámbito actual con su metadato completo.
    fn insertar_variable(&mut self, nombre: String, info: VariableInfo) {
        self.ambitos.last_mut().unwrap().insert(nombre, info);
    }

    /// Verifica si una variable existe en el ámbito actual (para declaración duplicada).
    fn existe_en_ambito_actual(&self, nombre: &str) -> bool {
        self.ambitos.last().is_some_and(|a| a.contains_key(nombre))
    }

    /// Evalúa una expresión contra este entorno.
    pub fn evaluar_expresion(&mut self, e: &Expresion) -> Result<Valor, ErrorEjecucion> {
        evaluar_con(e, self)
    }

    /// Llama a una función por nombre con los argumentos dados, reutilizando
    /// la ejecución normal de su cuerpo. Útil para funciones de orden superior.
    pub fn llamar_funcion_por_nombre(
        &mut self,
        nombre: &str,
        args: &[Valor],
        linea: usize,
        columna: usize,
    ) -> Result<Valor, ErrorEjecucion> {
        let fdef = self.funciones.get(nombre).cloned().ok_or_else(|| {
            error(
                CausaError::VariableNoDefinida(nombre.to_string()),
                linea,
                columna,
            )
        })?;
        if args.len() != fdef.parametros.len() {
            return Err(error(
                CausaError::FirmaInvalida {
                    funcion: nombre.to_string(),
                    esperado: fdef.parametros.len(),
                    obtenido: args.len(),
                },
                linea,
                columna,
            ));
        }
        self.ambitos.push(HashMap::new());
        for ((pname, _), val) in fdef.parametros.iter().zip(args.iter()) {
            self.insertar_variable(
                pname.clone(),
                VariableInfo {
                    valor: val.clone(),
                    tipo_declarado: None,
                    mutable: true,
                    es_const: false,
                },
            );
        }
        let r = self.ejecutar_funcion_sentencias(&fdef.cuerpo, nombre);
        self.ambitos.pop();
        match r? {
            Some(v) => Ok(v),
            None => Err(error(
                CausaError::RetornoAusente {
                    funcion: nombre.to_string(),
                },
                linea,
                columna,
            )),
        }
    }

    /// Llama a un valor función (global o clausura).
    pub fn llamar_valor(
        &mut self,
        func_val: Valor,
        args: &[Valor],
        linea: usize,
        columna: usize,
    ) -> Result<Valor, ErrorEjecucion> {
        match func_val {
            Valor::Funcion(nombre) => self.llamar_funcion_por_nombre(&nombre, args, linea, columna),
            Valor::FuncionClausura {
                parametros,
                cuerpo,
                entorno,
            } => {
                if args.len() != parametros.len() {
                    return Err(error(
                        CausaError::FirmaInvalida {
                            funcion: "clausura".to_string(),
                            esperado: parametros.len(),
                            obtenido: args.len(),
                        },
                        linea,
                        columna,
                    ));
                }
                let mut captured = entorno.lock().unwrap();
                captured.ambitos.push(HashMap::new());
                for ((pname, _), val) in parametros.iter().zip(args.iter()) {
                    captured.insertar_variable(
                        pname.clone(),
                        VariableInfo {
                            valor: val.clone(),
                            tipo_declarado: None,
                            mutable: true,
                            es_const: false,
                        },
                    );
                }
                let result = captured.ejecutar_funcion_sentencias(&cuerpo, "clausura");
                captured.ambitos.pop();
                match result? {
                    Some(v) => Ok(v),
                    None => Err(error(
                        CausaError::RetornoAusente {
                            funcion: "clausura".to_string(),
                        },
                        linea,
                        columna,
                    )),
                }
            }
            otro => Err(error(
                CausaError::AsignacionIncompatible {
                    esperado: "función".to_string(),
                    obtenido: otro.nombre_tipo().to_string(),
                },
                linea,
                columna,
            )),
        }
    }

    /// Llama a un valor función (global o clausura).
    pub fn llamar_por_valor(
        &mut self,
        func_val: Valor,
        args: &[Valor],
        linea: usize,
        columna: usize,
    ) -> Result<Valor, ErrorEjecucion> {
        match func_val {
            Valor::Funcion(nombre) => self.llamar_funcion_por_nombre(&nombre, args, linea, columna),
            Valor::FuncionClausura {
                parametros,
                cuerpo,
                entorno,
            } => {
                if args.len() != parametros.len() {
                    return Err(error(
                        CausaError::FirmaInvalida {
                            funcion: "clausura".to_string(),
                            esperado: parametros.len(),
                            obtenido: args.len(),
                        },
                        linea,
                        columna,
                    ));
                }
                let mut captured = entorno.lock().unwrap();
                captured.ambitos.push(HashMap::new());
                for ((pname, _), val) in parametros.iter().zip(args.iter()) {
                    captured.insertar_variable(
                        pname.clone(),
                        VariableInfo {
                            valor: val.clone(),
                            tipo_declarado: None,
                            mutable: true,
                            es_const: false,
                        },
                    );
                }
                let result = captured.ejecutar_funcion_sentencias(&cuerpo, "clausura");
                captured.ambitos.pop();
                match result? {
                    Some(v) => Ok(v),
                    None => Err(error(
                        CausaError::RetornoAusente {
                            funcion: "clausura".to_string(),
                        },
                        linea,
                        columna,
                    )),
                }
            }
            otro => Err(error(
                CausaError::AsignacionIncompatible {
                    esperado: "función".to_string(),
                    obtenido: otro.nombre_tipo().to_string(),
                },
                linea,
                columna,
            )),
        }
    }

    /// Ejecuta un programa sentencia a sentencia.
    /// Devuelve el valor de la última sentencia de expresión **del nivel superior**, si la hay.
    /// Las sentencias internas de bloques, ramas o ciclos no producen valor hacia fuera
    /// (ADR-0007 §1.6).
    pub fn ejecutar_programa(
        &mut self,
        sentencias: &[Sentencia],
    ) -> Result<Option<Valor>, ErrorEjecucion> {
        self.ejecutar_sentencias(sentencias)
    }

    /// Ejecuta el cuerpo de un bloque en un ámbito propio (ADR-0007 §1.6).
    /// El valor de las expresiones internas no se propaga al exterior.
    fn ejecutar_bloque(&mut self, sentencias: &[Sentencia]) -> Result<(), ErrorEjecucion> {
        self.ambitos.push(HashMap::new());
        let r = self.ejecutar_sentencias(sentencias);
        self.ambitos.pop();
        r.map(|_| ())
    }

    fn ejecutar_funcion(
        &mut self,
        sentencias: &[Sentencia],
        nombre_funcion: &str,
    ) -> Result<Option<Valor>, ErrorEjecucion> {
        self.ambitos.push(HashMap::new());
        let prev = self.en_funcion;
        self.en_funcion = true;
        let r = self.ejecutar_funcion_sentencias(sentencias, nombre_funcion);
        self.en_funcion = prev;
        self.ambitos.pop();
        r
    }

    fn ejecutar_funcion_sentencias(
        &mut self,
        sentencias: &[Sentencia],
        nombre_funcion: &str,
    ) -> Result<Option<Valor>, ErrorEjecucion> {
        for s in sentencias {
            let s = s.clone();
            match s {
                Sentencia::Declaracion {
                    nombre,
                    linea,
                    columna,
                    valor,
                    ref tipo_anotacion,
                    mutable,
                    es_const,
                    publico: _,
                } => {
                    if NOMBRES_RESERVADOS.contains(&nombre.as_str()) {
                        return Err(error(
                            CausaError::NombreReservado {
                                nombre: nombre.clone(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    if self.existe_en_ambito_actual(&nombre) {
                        return Err(error(
                            CausaError::DeclaracionDuplicada(nombre.clone()),
                            linea,
                            columna,
                        ));
                    }
                    let v = evaluar_con(&valor, self)?;
                    // Convertir y validar contra el tipo anotado si existe.
                    let v = if let Some(tipo_anotado) = tipo_anotacion {
                        let tipo_obtenido = v.nombre_tipo();
                        match convertir_a_tipo_anotado(v, tipo_anotado) {
                            Some(convertido) => convertido,
                            None => {
                                return Err(error(
                                    CausaError::AsignacionIncompatible {
                                        esperado: tipo_anotado.nombre(),
                                        obtenido: tipo_obtenido.to_string(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                        }
                    } else {
                        v
                    };
                    self.insertar_variable(
                        nombre.clone(),
                        VariableInfo {
                            valor: v,
                            tipo_declarado: tipo_anotacion.clone(),
                            mutable,
                            es_const,
                        },
                    );
                }
                Sentencia::Asignacion {
                    nombre,
                    linea,
                    columna,
                    valor,
                } => {
                    let nuevo = evaluar_con(&valor, self)?;
                    let nuevo_tipo = nuevo.nombre_tipo();
                    // Buscar la variable y verificar mutabilidad
                    let Some((idx, info)) = self.localizar_info(&nombre) else {
                        return Err(error(
                            CausaError::VariableNoDefinida(nombre.clone()),
                            linea,
                            columna,
                        ));
                    };
                    // Verificar mutabilidad
                    if !info.mutable {
                        return Err(error(
                            CausaError::ReasignacionInmutable {
                                nombre: nombre.clone(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    // Verificar compatibilidad de tipo (según el tipo declarado si lo hay).
                    let Some(ajustado) = convertir_para_variable(info, nuevo) else {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: info.valor.nombre_tipo().into(),
                                obtenido: nuevo_tipo.to_string(),
                            },
                            linea,
                            columna,
                        ));
                    };
                    self.ambitos[idx].get_mut(&nombre).unwrap().valor = ajustado;
                }
                Sentencia::AsignacionIndice {
                    objeto,
                    indice,
                    valor,
                    linea,
                    columna,
                } => {
                    let (var_nombre, base_linea, base_columna) = match objeto.as_ref() {
                        Expresion::Variable {
                            nombre,
                            linea,
                            columna,
                        } => (nombre.clone(), *linea, *columna),
                        _ => {
                            return Err(error(
                                CausaError::AsignacionIncompatible {
                                    esperado: "variable".into(),
                                    obtenido: "expresión no asignable".to_string(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    let Some((idx_amb, info)) = localizar_info_entorno(self, &var_nombre) else {
                        return Err(error(
                            CausaError::VariableNoDefinida(var_nombre.clone()),
                            base_linea,
                            base_columna,
                        ));
                    };
                    if !info.mutable {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: "variable inmutable (dato/const)".into(),
                                obtenido: "intento de reasignación".to_string(),
                            },
                            base_linea,
                            base_columna,
                        ));
                    }
                    let idxv = evaluar_con(&indice, self)?;
                    let i = match idxv {
                        Valor::Entero(n) => n,
                        _ => {
                            return Err(error(
                                CausaError::AsignacionIncompatible {
                                    esperado: "índice entero".into(),
                                    obtenido: idxv.nombre_tipo().to_string(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    if i < 0 {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: "índice válido".into(),
                                obtenido: "índice negativo".to_string(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    let idx_usize = i as usize;
                    let nuevo = evaluar_con(&valor, self)?;
                    let actual = match &info.valor {
                        Valor::Arreglo(v) | Valor::Lista(v) => {
                            if idx_usize >= v.len() {
                                return Err(error(
                                    CausaError::AsignacionIncompatible {
                                        esperado: "índice válido".into(),
                                        obtenido: "fuera de rango".to_string(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                            v[idx_usize].clone()
                        }
                        _ => {
                            return Err(error(
                                CausaError::AsignacionIncompatible {
                                    esperado: "arreglo o lista".into(),
                                    obtenido: info.valor.nombre_tipo().to_string(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    let Some(ajustado) = convertir_para_tipo(&actual, nuevo) else {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: actual.nombre_tipo().into(),
                                obtenido: "tipo incompatible".to_string(),
                            },
                            linea,
                            columna,
                        ));
                    };
                    if let Some(info_mut) = self.ambitos[idx_amb].get_mut(&var_nombre)
                        && let Valor::Arreglo(v) | Valor::Lista(v) = &mut info_mut.valor
                        && idx_usize < v.len()
                    {
                        v[idx_usize] = ajustado;
                    }
                }
                Sentencia::Si {
                    condicion,
                    rama_si,
                    rama_sino,
                    ..
                } => {
                    let c = evaluar_con(&condicion, self)?;
                    let es_verdadero = match c {
                        Valor::Logico(b) => b,
                        _ => {
                            let (linea, columna) = ubicacion(&condicion);
                            return Err(error(
                                CausaError::CondicionNoLogica {
                                    obtenido: c.nombre_tipo(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    if es_verdadero {
                        match self.ejecutar_bloque_funcion(&rama_si, nombre_funcion) {
                            Ok(Some(v)) => return Ok(Some(v)),
                            Ok(None) => {}
                            Err(e) => return Err(e),
                        }
                    } else if let Some(rama) = rama_sino {
                        match self.ejecutar_bloque_funcion(&rama, nombre_funcion) {
                            Ok(Some(v)) => return Ok(Some(v)),
                            Ok(None) => {}
                            Err(e) => return Err(e),
                        }
                    }
                }
                Sentencia::Mientras {
                    condicion, cuerpo, ..
                } => {
                    let mut iteraciones = 0usize;
                    loop {
                        let c = evaluar_con(&condicion, self)?;
                        let es_verdadero = match c {
                            Valor::Logico(b) => b,
                            _ => {
                                let (linea, columna) = ubicacion(&condicion);
                                return Err(error(
                                    CausaError::CondicionNoLogica {
                                        obtenido: c.nombre_tipo(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                        };
                        if !es_verdadero {
                            break;
                        }
                        iteraciones += 1;
                        if iteraciones > LIMITE_ITERACIONES {
                            let (linea, columna) = ubicacion(&condicion);
                            return Err(error(
                                CausaError::LimiteIteraciones {
                                    limite: LIMITE_ITERACIONES,
                                },
                                linea,
                                columna,
                            ));
                        }
                        match self.ejecutar_bloque_funcion(&cuerpo, nombre_funcion) {
                            Ok(Some(v)) => return Ok(Some(v)),
                            Ok(None) => {}
                            Err(e) => match e.causa.as_ref() {
                                CausaError::SenalRomper => break,
                                CausaError::SenalContinuar => {}
                                _ => return Err(e),
                            },
                        }
                    }
                }
                Sentencia::Bloque(sentencias) => {
                    match self.ejecutar_bloque_funcion(&sentencias, nombre_funcion) {
                        Ok(Some(v)) => return Ok(Some(v)),
                        Ok(None) => {}
                        Err(e) => return Err(e),
                    };
                }
                Sentencia::Importar {
                    ruta,
                    alias,
                    linea,
                    columna,
                } => {
                    let ruta_path = resolver_modulo(&ruta);
                    let modulo = {
                        let cache = CACHE_MODULOS.lock().unwrap();
                        if let Some(m) = cache.get(&ruta_path) {
                            m.clone()
                        } else {
                            drop(cache);
                            {
                                let mut guard = IMPORTANDO.lock().unwrap();
                                if !guard.insert(ruta_path.clone()) {
                                    return Err(error(
                                        CausaError::ErrorDeArchivo {
                                            ruta: ruta.clone(),
                                            detalle: "ciclo de importación".to_string(),
                                        },
                                        linea,
                                        columna,
                                    ));
                                }
                            }
                            let fuente = match fs::read_to_string(&ruta_path) {
                                Ok(f) => f,
                                Err(e) => {
                                    IMPORTANDO.lock().unwrap().remove(&ruta_path);
                                    return Err(error(
                                        CausaError::ErrorDeArchivo {
                                            ruta: ruta.clone(),
                                            detalle: e.to_string(),
                                        },
                                        linea,
                                        columna,
                                    ));
                                }
                            };
                            let tokens = match Lexer::nuevo(&fuente).tokenizar() {
                                Ok(t) => t,
                                Err(e) => {
                                    IMPORTANDO.lock().unwrap().remove(&ruta_path);
                                    return Err(error(
                                        CausaError::ErrorDeArchivo {
                                            ruta: ruta.clone(),
                                            detalle: e.to_string(),
                                        },
                                        linea,
                                        columna,
                                    ));
                                }
                            };
                            let sentencias = match Parser::nuevo(tokens).parsear_programa() {
                                Ok(s) => s,
                                Err(e) => {
                                    IMPORTANDO.lock().unwrap().remove(&ruta_path);
                                    return Err(error(
                                        CausaError::ErrorDeArchivo {
                                            ruta: ruta.clone(),
                                            detalle: e.to_string(),
                                        },
                                        linea,
                                        columna,
                                    ));
                                }
                            };
                            let mut env_mod = Entorno::nueva();
                            let resultado = env_mod.ejecutar_sentencias(&sentencias);
                            IMPORTANDO.lock().unwrap().remove(&ruta_path);
                            resultado?;
                            let mut cache = CACHE_MODULOS.lock().unwrap();
                            cache.insert(ruta_path.clone(), env_mod.clone());
                            env_mod
                        }
                    };
                    let exportar_todo = modulo.exportaciones.is_empty();
                    for (nombre, func) in &modulo.funciones {
                        if exportar_todo || modulo.exportaciones.contains(nombre) {
                            let destino = if let Some(alias) = &alias {
                                format!("{}_{}", alias, nombre)
                            } else {
                                nombre.clone()
                            };
                            self.funciones.insert(destino, func.clone());
                        }
                    }
                    if let Some(ambito) = modulo.ambitos.first() {
                        for (nombre, info) in ambito {
                            if exportar_todo || modulo.exportaciones.contains(nombre) {
                                let destino = if let Some(alias) = &alias {
                                    format!("{}_{}", alias, nombre)
                                } else {
                                    nombre.clone()
                                };
                                self.ambitos[0].insert(destino, info.clone());
                            }
                        }
                    }
                }

                Sentencia::Exportar { sentencia, .. } => {
                    let nombre_exportado = match &*sentencia {
                        Sentencia::Declaracion { nombre, .. } => Some(nombre.clone()),
                        Sentencia::Funcion { nombre, .. } => Some(nombre.clone()),
                        _ => None,
                    };
                    let resultado =
                        self.ejecutar_funcion_sentencias(&[*sentencia.clone()], nombre_funcion);
                    if let Some(nombre) = nombre_exportado {
                        self.exportaciones.insert(nombre);
                    }
                    if let Some(v) = resultado? {
                        return Ok(Some(v));
                    }
                }
                Sentencia::Mostrar { valor, .. } => {
                    let v = evaluar_con(&valor, self)?;
                    println!("{v}");
                }
                Sentencia::MostrarSinSalto { valor, .. } => {
                    let v = evaluar_con(&valor, self)?;
                    print!("{v}");
                }
                Sentencia::Funcion {
                    nombre,
                    parametros,
                    tipo_retorno: _,
                    cuerpo,
                    linea,
                    columna,
                    publico: _,
                } => {
                    if self.existe_en_ambito_actual(&nombre) {
                        return Err(error(
                            CausaError::DeclaracionDuplicada(nombre.clone()),
                            linea,
                            columna,
                        ));
                    }
                    let capturado = Arc::new(Mutex::new(self.clone()));
                    self.insertar_variable(
                        nombre.clone(),
                        VariableInfo {
                            valor: Valor::FuncionClausura {
                                parametros: parametros.clone(),
                                cuerpo: cuerpo.clone(),
                                entorno: capturado,
                            },
                            tipo_declarado: None,
                            mutable: true,
                            es_const: false,
                        },
                    );
                }
                Sentencia::Retorno {
                    valor,
                    linea,
                    columna,
                    ..
                } => {
                    if let Some(v) = valor {
                        let mut resultado = evaluar_con(&v, self)?;
                        if let Some(tipo_esperado) = self
                            .funciones
                            .get(nombre_funcion)
                            .and_then(|f| f.tipo_retorno_anotado.as_ref())
                        {
                            let obtenido = resultado.nombre_tipo().to_string();
                            resultado = convertir_a_tipo_anotado(resultado, tipo_esperado)
                                .ok_or_else(|| {
                                    error(
                                        CausaError::RetornoIncompatibleConAnotacion {
                                            funcion: nombre_funcion.into(),
                                            esperado: tipo_esperado.nombre(),
                                            obtenido,
                                        },
                                        linea,
                                        columna,
                                    )
                                })?;
                        }
                        return Ok(Some(resultado));
                    } else {
                        return Err(error(
                            CausaError::RetornoAusente {
                                funcion: nombre_funcion.into(),
                            },
                            linea,
                            columna,
                        ));
                    }
                }
                Sentencia::ParaCada {
                    variable,
                    iterable,
                    cuerpo,
                    linea,
                    columna,
                } => {
                    let variable_iterable = match iterable {
                        Expresion::Variable { ref nombre, .. } => Some(nombre.clone()),
                        _ => None,
                    };
                    let snapshot_iterable = if let Some(nombre) = &variable_iterable {
                        evaluar_con(
                            &Expresion::Variable {
                                nombre: nombre.clone(),
                                linea,
                                columna,
                            },
                            self,
                        )
                        .ok()
                    } else {
                        None
                    };

                    let valores = match evaluar_con(&iterable, self)? {
                        Valor::Arreglo(v) | Valor::Lista(v) => v,
                        Valor::Texto(s) => s.chars().map(|c| Valor::Texto(c.to_string())).collect(),
                        otro => {
                            return Err(error(
                                CausaError::IteracionSobreNoColeccion {
                                    obtenido: otro.nombre_tipo(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };

                    for item in valores {
                        self.ambitos.push(HashMap::new());
                        self.insertar_variable(
                            variable.clone(),
                            VariableInfo {
                                valor: item,
                                tipo_declarado: None,
                                mutable: false,
                                es_const: false,
                            },
                        );
                        let resultado = self.ejecutar_bloque_funcion(&cuerpo, nombre_funcion);

                        if let (Some(nombre), Some(snapshot)) =
                            (&variable_iterable, &snapshot_iterable)
                            && let Ok(actual) = evaluar_con(
                                &Expresion::Variable {
                                    nombre: nombre.clone(),
                                    linea,
                                    columna,
                                },
                                self,
                            )
                            && actual != *snapshot
                        {
                            self.ambitos.pop();
                            return Err(error(
                                CausaError::ModificacionDuranteIteracion {
                                    variable: nombre.clone(),
                                },
                                linea,
                                columna,
                            ));
                        }

                        self.ambitos.pop();
                        match resultado {
                            Ok(Some(v)) => return Ok(Some(v)),
                            Ok(None) => {}
                            Err(e) => match e.causa.as_ref() {
                                CausaError::SenalRomper => break,
                                CausaError::SenalContinuar => continue,
                                _ => {
                                    return Err(e);
                                }
                            },
                        }
                    }
                }
                Sentencia::Romper { linea, columna } => {
                    return Err(error(CausaError::SenalRomper, linea, columna));
                }
                Sentencia::Continuar { linea, columna } => {
                    return Err(error(CausaError::SenalContinuar, linea, columna));
                }
                Sentencia::IntentarRescatar {
                    cuerpo_intentar,
                    variable_error,
                    cuerpo_rescatar,
                    linea: _,
                    columna: _,
                } => {
                    let resultado = self.ejecutar_bloque_funcion(&cuerpo_intentar, nombre_funcion);
                    match resultado {
                        Ok(Some(v)) => return Ok(Some(v)),
                        Ok(None) => {}
                        Err(err) => {
                            if matches!(
                                err.causa.as_ref(),
                                CausaError::SenalRomper | CausaError::SenalContinuar
                            ) {
                                return Err(err);
                            }
                            if let Some(nombre) = variable_error {
                                self.ambitos.push(HashMap::new());
                                self.insertar_variable(
                                    nombre.clone(),
                                    VariableInfo {
                                        valor: Valor::Texto(err.mensaje.clone()),
                                        tipo_declarado: None,
                                        mutable: true,
                                        es_const: false,
                                    },
                                );
                                let r =
                                    self.ejecutar_bloque_funcion(&cuerpo_rescatar, nombre_funcion);
                                self.ambitos.pop();
                                if let Some(valor) = r? {
                                    return Ok(Some(valor));
                                }
                            } else {
                                // Sin variable de captura, se ejecuta el bloque de rescate sin contexto.
                                let r =
                                    self.ejecutar_bloque_funcion(&cuerpo_rescatar, nombre_funcion);
                                if let Some(valor) = r? {
                                    return Ok(Some(valor));
                                }
                            }
                        }
                    }
                }
                Sentencia::Usar { linea, columna, .. } => {
                    return Err(error(
                        CausaError::ImportacionesNoDisponibles,
                        linea,
                        columna,
                    ));
                }
                Sentencia::Expresion(e) => {
                    let _ = evaluar_con(&e, self)?;
                }
            }
        }
        Ok(None)
    }

    fn ejecutar_bloque_funcion(
        &mut self,
        sentencias: &[Sentencia],
        nombre_funcion: &str,
    ) -> Result<Option<Valor>, ErrorEjecucion> {
        self.ambitos.push(HashMap::new());
        let r = self.ejecutar_funcion_sentencias(sentencias, nombre_funcion);
        self.ambitos.pop();
        r
    }

    fn ejecutar_sentencias(
        &mut self,
        sentencias: &[Sentencia],
    ) -> Result<Option<Valor>, ErrorEjecucion> {
        let mut ultimo = None;
        for s in sentencias {
            let s = s.clone();
            match s {
                Sentencia::Declaracion {
                    nombre,
                    linea,
                    columna,
                    valor,
                    ref tipo_anotacion,
                    mutable,
                    es_const,
                    publico: _,
                } => {
                    if NOMBRES_RESERVADOS.contains(&nombre.as_str()) {
                        return Err(error(
                            CausaError::NombreReservado {
                                nombre: nombre.clone(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    if self.existe_en_ambito_actual(&nombre) {
                        return Err(error(
                            CausaError::DeclaracionDuplicada(nombre.clone()),
                            linea,
                            columna,
                        ));
                    }
                    let v = evaluar_con(&valor, self)?;
                    // Convertir y validar contra el tipo anotado si existe.
                    let v = if let Some(tipo_anotado) = tipo_anotacion {
                        let tipo_obtenido = v.nombre_tipo();
                        match convertir_a_tipo_anotado(v, tipo_anotado) {
                            Some(convertido) => convertido,
                            None => {
                                return Err(error(
                                    CausaError::AsignacionIncompatible {
                                        esperado: tipo_anotado.nombre(),
                                        obtenido: tipo_obtenido.to_string(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                        }
                    } else {
                        v
                    };
                    self.insertar_variable(
                        nombre.clone(),
                        VariableInfo {
                            valor: v,
                            tipo_declarado: tipo_anotacion.clone(),
                            mutable,
                            es_const,
                        },
                    );
                }
                Sentencia::Asignacion {
                    nombre,
                    linea,
                    columna,
                    valor,
                } => {
                    let nuevo = evaluar_con(&valor, self)?;
                    let nuevo_tipo = nuevo.nombre_tipo();
                    // Buscar la variable y verificar mutabilidad
                    let Some((idx, info)) = self.localizar_info(&nombre) else {
                        return Err(error(
                            CausaError::VariableNoDefinida(nombre.clone()),
                            linea,
                            columna,
                        ));
                    };
                    // Verificar mutabilidad
                    if !info.mutable {
                        return Err(error(
                            CausaError::ReasignacionInmutable {
                                nombre: nombre.clone(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    // Verificar compatibilidad de tipo (según el tipo declarado si lo hay).
                    let Some(ajustado) = convertir_para_variable(info, nuevo) else {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: info.valor.nombre_tipo().into(),
                                obtenido: nuevo_tipo.to_string(),
                            },
                            linea,
                            columna,
                        ));
                    };
                    self.ambitos[idx].get_mut(&nombre).unwrap().valor = ajustado;
                }
                Sentencia::Funcion {
                    nombre,
                    parametros,
                    tipo_retorno,
                    cuerpo,
                    linea,
                    columna,
                    publico: _,
                } => {
                    // Verificar nombre reservado de builtin (ADR-0013 §3.1)
                    if NOMBRES_RESERVADOS.contains(&nombre.as_str()) {
                        return Err(error(
                            CausaError::NombreReservado {
                                nombre: nombre.clone(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    if self.en_funcion {
                        // Función anidada: crear clausura capturando el entorno actual.
                        if self.existe_en_ambito_actual(&nombre) {
                            return Err(error(
                                CausaError::DeclaracionDuplicada(nombre.clone()),
                                linea,
                                columna,
                            ));
                        }
                        let capturado = Arc::new(Mutex::new(self.clone()));
                        self.insertar_variable(
                            nombre.clone(),
                            VariableInfo {
                                valor: Valor::FuncionClausura {
                                    parametros: parametros.clone(),
                                    cuerpo: cuerpo.clone(),
                                    entorno: capturado,
                                },
                                tipo_declarado: None,
                                mutable: true,
                                es_const: false,
                            },
                        );
                    } else {
                        if self.funciones.contains_key(&nombre) {
                            return Err(error(
                                CausaError::DeclaracionDuplicada(nombre.clone()),
                                linea,
                                columna,
                            ));
                        }
                        self.funciones.insert(
                            nombre.clone(),
                            FuncionDef {
                                nombre: nombre.clone(),
                                parametros: parametros.clone(),
                                tipo_retorno_anotado: tipo_retorno.clone(),
                                cuerpo: cuerpo.clone(),
                                linea,
                                columna,
                                tipo_retorno: None,
                                tipos_parametros_inferidos: None,
                            },
                        );
                    }
                }
                Sentencia::Importar {
                    ruta,
                    alias,
                    linea,
                    columna,
                } => {
                    let ruta_path = resolver_modulo(&ruta);
                    let modulo = {
                        let cache = CACHE_MODULOS.lock().unwrap();
                        if let Some(m) = cache.get(&ruta_path) {
                            m.clone()
                        } else {
                            drop(cache);
                            {
                                let mut guard = IMPORTANDO.lock().unwrap();
                                if !guard.insert(ruta_path.clone()) {
                                    return Err(error(
                                        CausaError::ErrorDeArchivo {
                                            ruta: ruta.clone(),
                                            detalle: "ciclo de importación".to_string(),
                                        },
                                        linea,
                                        columna,
                                    ));
                                }
                            }
                            let fuente = match fs::read_to_string(&ruta_path) {
                                Ok(f) => f,
                                Err(e) => {
                                    IMPORTANDO.lock().unwrap().remove(&ruta_path);
                                    return Err(error(
                                        CausaError::ErrorDeArchivo {
                                            ruta: ruta.clone(),
                                            detalle: e.to_string(),
                                        },
                                        linea,
                                        columna,
                                    ));
                                }
                            };
                            let tokens = match Lexer::nuevo(&fuente).tokenizar() {
                                Ok(t) => t,
                                Err(e) => {
                                    IMPORTANDO.lock().unwrap().remove(&ruta_path);
                                    return Err(error(
                                        CausaError::ErrorDeArchivo {
                                            ruta: ruta.clone(),
                                            detalle: e.to_string(),
                                        },
                                        linea,
                                        columna,
                                    ));
                                }
                            };
                            let sentencias = match Parser::nuevo(tokens).parsear_programa() {
                                Ok(s) => s,
                                Err(e) => {
                                    IMPORTANDO.lock().unwrap().remove(&ruta_path);
                                    return Err(error(
                                        CausaError::ErrorDeArchivo {
                                            ruta: ruta.clone(),
                                            detalle: e.to_string(),
                                        },
                                        linea,
                                        columna,
                                    ));
                                }
                            };
                            let mut env_mod = Entorno::nueva();
                            let resultado = env_mod.ejecutar_sentencias(&sentencias);
                            IMPORTANDO.lock().unwrap().remove(&ruta_path);
                            resultado?;
                            let mut cache = CACHE_MODULOS.lock().unwrap();
                            cache.insert(ruta_path.clone(), env_mod.clone());
                            env_mod
                        }
                    };
                    let exportar_todo = modulo.exportaciones.is_empty();
                    for (nombre, func) in &modulo.funciones {
                        if exportar_todo || modulo.exportaciones.contains(nombre) {
                            let destino = if let Some(alias) = &alias {
                                format!("{}_{}", alias, nombre)
                            } else {
                                nombre.clone()
                            };
                            self.funciones.insert(destino, func.clone());
                        }
                    }
                    if let Some(ambito) = modulo.ambitos.first() {
                        for (nombre, info) in ambito {
                            if exportar_todo || modulo.exportaciones.contains(nombre) {
                                let destino = if let Some(alias) = &alias {
                                    format!("{}_{}", alias, nombre)
                                } else {
                                    nombre.clone()
                                };
                                self.ambitos[0].insert(destino, info.clone());
                            }
                        }
                    }
                }

                Sentencia::Exportar { sentencia, .. } => {
                    let nombre_exportado = match &*sentencia {
                        Sentencia::Declaracion { nombre, .. } => Some(nombre.clone()),
                        Sentencia::Funcion { nombre, .. } => Some(nombre.clone()),
                        _ => None,
                    };
                    let resultado = self.ejecutar_sentencias(&[*sentencia.clone()]);
                    if let Some(nombre) = nombre_exportado {
                        self.exportaciones.insert(nombre);
                    }
                    resultado?;
                }
                Sentencia::Mostrar { valor, .. } => {
                    let v = evaluar_con(&valor, self)?;
                    println!("{v}");
                }
                Sentencia::MostrarSinSalto { valor, .. } => {
                    let v = evaluar_con(&valor, self)?;
                    print!("{v}");
                }
                Sentencia::Retorno { linea, columna, .. } => {
                    return Err(error(CausaError::RetornarFueraDeBloque, linea, columna));
                }
                Sentencia::ParaCada {
                    variable,
                    iterable,
                    cuerpo,
                    linea,
                    columna,
                } => {
                    let variable_iterable = match iterable {
                        Expresion::Variable { ref nombre, .. } => Some(nombre.clone()),
                        _ => None,
                    };
                    let snapshot_iterable = if let Some(nombre) = &variable_iterable {
                        evaluar_con(
                            &Expresion::Variable {
                                nombre: nombre.clone(),
                                linea,
                                columna,
                            },
                            self,
                        )
                        .ok()
                    } else {
                        None
                    };

                    let valores = match evaluar_con(&iterable, self)? {
                        Valor::Arreglo(v) | Valor::Lista(v) => v,
                        Valor::Texto(s) => s.chars().map(|c| Valor::Texto(c.to_string())).collect(),
                        otro => {
                            return Err(error(
                                CausaError::IteracionSobreNoColeccion {
                                    obtenido: otro.nombre_tipo(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };

                    for item in valores {
                        self.ambitos.push(HashMap::new());
                        self.insertar_variable(
                            variable.clone(),
                            VariableInfo {
                                valor: item,
                                tipo_declarado: None,
                                mutable: false,
                                es_const: false,
                            },
                        );
                        let result = self.ejecutar_bloque(&cuerpo);

                        if let (Some(nombre), Some(snapshot)) =
                            (&variable_iterable, &snapshot_iterable)
                            && let Ok(actual) = evaluar_con(
                                &Expresion::Variable {
                                    nombre: nombre.clone(),
                                    linea,
                                    columna,
                                },
                                self,
                            )
                            && actual != *snapshot
                        {
                            self.ambitos.pop();
                            return Err(error(
                                CausaError::ModificacionDuranteIteracion {
                                    variable: nombre.clone(),
                                },
                                linea,
                                columna,
                            ));
                        }

                        self.ambitos.pop();
                        match result {
                            Ok(_) => {}
                            Err(e) => match *e.causa {
                                CausaError::SenalRomper => break,
                                CausaError::SenalContinuar => continue,
                                _ => return Err(e),
                            },
                        }
                    }
                }
                Sentencia::Romper { linea, columna } => {
                    return Err(error(CausaError::SenalRomper, linea, columna));
                }
                Sentencia::Continuar { linea, columna } => {
                    return Err(error(CausaError::SenalContinuar, linea, columna));
                }
                Sentencia::IntentarRescatar {
                    cuerpo_intentar,
                    variable_error,
                    cuerpo_rescatar,
                    linea: _,
                    columna: _,
                } => {
                    let result = self.ejecutar_bloque(&cuerpo_intentar);
                    match result {
                        Ok(_) => {}
                        Err(err) => {
                            if matches!(
                                err.causa.as_ref(),
                                CausaError::SenalRomper | CausaError::SenalContinuar
                            ) {
                                return Err(err);
                            }
                            if let Some(nombre) = variable_error {
                                self.ambitos.push(HashMap::new());
                                self.insertar_variable(
                                    nombre.clone(),
                                    VariableInfo {
                                        valor: Valor::Texto(err.mensaje.clone()),
                                        tipo_declarado: None,
                                        mutable: true,
                                        es_const: false,
                                    },
                                );
                                let r = self.ejecutar_bloque(&cuerpo_rescatar);
                                self.ambitos.pop();
                                r?;
                            } else {
                                self.ejecutar_bloque(&cuerpo_rescatar)?;
                            }
                        }
                    }
                }
                Sentencia::Si {
                    condicion,
                    rama_si,
                    rama_sino,
                    ..
                } => {
                    let c = evaluar_con(&condicion, self)?;
                    let es_verdadero = match c {
                        Valor::Logico(b) => b,
                        _ => {
                            let (linea, columna) = ubicacion(&condicion);
                            return Err(error(
                                CausaError::CondicionNoLogica {
                                    obtenido: c.nombre_tipo(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    if es_verdadero {
                        self.ejecutar_bloque(&rama_si)?;
                    } else if let Some(rama) = rama_sino {
                        self.ejecutar_bloque(&rama)?;
                    }
                }
                Sentencia::Mientras {
                    condicion, cuerpo, ..
                } => {
                    let mut iteraciones = 0usize;
                    loop {
                        let c = evaluar_con(&condicion, self)?;
                        let es_verdadero = match c {
                            Valor::Logico(b) => b,
                            _ => {
                                let (linea, columna) = ubicacion(&condicion);
                                return Err(error(
                                    CausaError::CondicionNoLogica {
                                        obtenido: c.nombre_tipo(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                        };
                        if !es_verdadero {
                            break;
                        }
                        iteraciones += 1;
                        if iteraciones > LIMITE_ITERACIONES {
                            let (linea, columna) = ubicacion(&condicion);
                            return Err(error(
                                CausaError::LimiteIteraciones {
                                    limite: LIMITE_ITERACIONES,
                                },
                                linea,
                                columna,
                            ));
                        }
                        match self.ejecutar_bloque(&cuerpo) {
                            Ok(()) => {}
                            Err(e) => match e.causa.as_ref() {
                                CausaError::SenalRomper => break,
                                CausaError::SenalContinuar => continue,
                                _ => return Err(e),
                            },
                        }
                    }
                }
                Sentencia::Bloque(sentencias) => {
                    self.ejecutar_bloque(&sentencias)?;
                }
                Sentencia::AsignacionIndice {
                    objeto,
                    indice,
                    valor,
                    linea,
                    columna,
                } => {
                    let (var_nombre, base_linea, base_columna) = match objeto.as_ref() {
                        Expresion::Variable {
                            nombre,
                            linea,
                            columna,
                        } => (nombre.clone(), *linea, *columna),
                        _ => {
                            return Err(error(
                                CausaError::AsignacionIncompatible {
                                    esperado: "variable".into(),
                                    obtenido: "expresión no asignable".to_string(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    let Some((idx_amb, info)) = localizar_info_entorno(self, &var_nombre) else {
                        return Err(error(
                            CausaError::VariableNoDefinida(var_nombre.clone()),
                            base_linea,
                            base_columna,
                        ));
                    };
                    if !info.mutable {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: "variable inmutable (dato/const)".into(),
                                obtenido: "intento de reasignación".to_string(),
                            },
                            base_linea,
                            base_columna,
                        ));
                    }
                    let idxv = evaluar_con(indice.as_ref(), self)?;
                    let i = match idxv {
                        Valor::Entero(n) => n,
                        _ => {
                            return Err(error(
                                CausaError::AsignacionIncompatible {
                                    esperado: "índice entero".into(),
                                    obtenido: idxv.nombre_tipo().to_string(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    if i < 0 {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: "índice válido".into(),
                                obtenido: "índice negativo".to_string(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    let idx_usize = i as usize;
                    let nuevo = evaluar_con(&valor, self)?;
                    let actual = match &info.valor {
                        Valor::Arreglo(v) | Valor::Lista(v) => {
                            if idx_usize >= v.len() {
                                return Err(error(
                                    CausaError::AsignacionIncompatible {
                                        esperado: "índice válido".into(),
                                        obtenido: "fuera de rango".to_string(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                            v[idx_usize].clone()
                        }
                        _ => {
                            return Err(error(
                                CausaError::AsignacionIncompatible {
                                    esperado: "arreglo o lista".into(),
                                    obtenido: info.valor.nombre_tipo().to_string(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    let Some(ajustado) = convertir_para_tipo(&actual, nuevo) else {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: actual.nombre_tipo().into(),
                                obtenido: "tipo incompatible".to_string(),
                            },
                            linea,
                            columna,
                        ));
                    };
                    if let Some(info_mut) = self.ambitos[idx_amb].get_mut(&var_nombre)
                        && let Valor::Arreglo(v) | Valor::Lista(v) = &mut info_mut.valor
                        && idx_usize < v.len()
                    {
                        v[idx_usize] = ajustado;
                    }
                }
                Sentencia::Usar {
                    ruta,
                    alias,
                    nombres,
                    linea,
                    columna,
                } => {
                    let cargador = self.cargador.clone().ok_or_else(|| {
                        error(CausaError::ImportacionesNoDisponibles, linea, columna)
                    })?;
                    let importador = self.archivo.as_ref().map(PathBuf::from);
                    let modulo = CargadorModulos::cargar(&cargador, &ruta, importador.as_deref())?;

                    if let Some(alias) = alias {
                        if self.existe_en_ambito_actual(&alias)
                            || self.funciones.contains_key(&alias)
                        {
                            return Err(error(
                                CausaError::ImportacionDuplicada {
                                    nombre: alias.clone(),
                                    modulo: modulo.ruta.display().to_string(),
                                },
                                linea,
                                columna,
                            ));
                        }
                        self.insertar_variable(
                            alias.clone(),
                            VariableInfo {
                                valor: Valor::Modulo(modulo.clone()),
                                tipo_declarado: None,
                                mutable: false,
                                es_const: false,
                            },
                        );
                    }

                    if let Some(nombres) = nombres {
                        for nombre in nombres {
                            if self.existe_en_ambito_actual(&nombre)
                                || self.funciones.contains_key(&nombre)
                            {
                                return Err(error(
                                    CausaError::ImportacionDuplicada {
                                        nombre: nombre.clone(),
                                        modulo: modulo.ruta.display().to_string(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                            if let Some(exportado) = modulo.exportaciones.get(&nombre) {
                                match exportado {
                                    SimbodoExportado::Variable(v) => {
                                        self.insertar_variable(
                                            nombre.clone(),
                                            VariableInfo {
                                                valor: v.clone(),
                                                tipo_declarado: None,
                                                mutable: false,
                                                es_const: false,
                                            },
                                        );
                                    }
                                    SimbodoExportado::Funcion(f) => {
                                        self.funciones.insert(nombre.clone(), f.clone());
                                    }
                                }
                            } else if modulo.entorno.lock().unwrap().variable(&nombre).is_some()
                                || modulo.entorno.lock().unwrap().funcion(&nombre).is_some()
                            {
                                return Err(error(
                                    CausaError::SimbodoPrivado {
                                        nombre: nombre.clone(),
                                        modulo: modulo.ruta.display().to_string(),
                                    },
                                    linea,
                                    columna,
                                ));
                            } else {
                                return Err(error(
                                    CausaError::SimbodoNoExportado {
                                        nombre: nombre.clone(),
                                        modulo: modulo.ruta.display().to_string(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                        }
                    }
                }
                Sentencia::Expresion(e) => {
                    ultimo = Some(evaluar_con(&e, self)?);
                }
            }
        }
        Ok(ultimo)
    }

    /// Valor de una variable visible (para pruebas e introspección).
    pub fn variable(&self, nombre: &str) -> Option<Valor> {
        self.buscar(nombre)
    }

    /// Definición de una función registrada (para el cargador de módulos, ADR-0017).
    pub fn funcion(&self, nombre: &str) -> Option<FuncionDef> {
        self.funciones.get(nombre).cloned()
    }

    fn localizar(&self, nombre: &str) -> Option<(usize, VariableInfo)> {
        self.ambitos
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, a)| a.get(nombre).cloned().map(|v| (i, v)))
    }

    fn localizar_info(&mut self, nombre: &str) -> Option<(usize, &mut VariableInfo)> {
        for (i, ambito) in self.ambitos.iter_mut().enumerate().rev() {
            if let Some(info) = ambito.get_mut(nombre) {
                return Some((i, info));
            }
        }
        None
    }

    fn buscar(&self, nombre: &str) -> Option<Valor> {
        self.localizar(nombre).map(|(_, v)| v.valor)
    }
}

/// Convierte y valida un valor contra una anotación de tipo, aplicando las conversiones
/// implícitas permitidas (entero→decimal, entero→real) y validando el tamaño y los
/// elementos de arreglos y listas. `None` = incompatible.
fn convertir_a_tipo_anotado(valor: Valor, tipo: &TipoAnotacion) -> Option<Valor> {
    match tipo {
        TipoAnotacion::Entero => match valor {
            Valor::Entero(_) => Some(valor),
            _ => None,
        },
        TipoAnotacion::Decimal => match valor {
            Valor::Decimal(_) => Some(valor),
            Valor::Entero(n) => Some(Valor::decimal_desde_entero(n)),
            _ => None,
        },
        TipoAnotacion::Real => match valor {
            Valor::Real(_) => Some(valor),
            Valor::Entero(n) => Some(Valor::Real(n as f64)),
            _ => None,
        },
        TipoAnotacion::Logico => match valor {
            Valor::Logico(_) => Some(valor),
            _ => None,
        },
        TipoAnotacion::Texto => match valor {
            Valor::Texto(_) => Some(valor),
            _ => None,
        },
        TipoAnotacion::Arreglo(tipo_elem, tamano) => {
            let elementos = match valor {
                Valor::Arreglo(v) | Valor::Lista(v) => v,
                _ => return None,
            };
            if elementos.len() != *tamano {
                return None;
            }
            let convertidos = elementos
                .into_iter()
                .map(|e| convertir_a_tipo_anotado(e, tipo_elem))
                .collect::<Option<Vec<_>>>()?;
            Some(Valor::Arreglo(convertidos))
        }
        TipoAnotacion::Lista(tipo_elem) => {
            let elementos = match valor {
                Valor::Lista(v) | Valor::Arreglo(v) => v,
                _ => return None,
            };
            let convertidos = elementos
                .into_iter()
                .map(|e| convertir_a_tipo_anotado(e, tipo_elem))
                .collect::<Option<Vec<_>>>()?;
            Some(Valor::Lista(convertidos))
        }
        TipoAnotacion::Nulo => match valor {
            Valor::Nulo => Some(valor),
            _ => None,
        },
    }
}

/// Convierte `nuevo` al tipo de la variable: usa la anotación declarada si existe y,
/// si no, las conversiones implícitas basadas en el valor actual (ADR-0005 §1.4).
fn convertir_para_variable(info: &VariableInfo, nuevo: Valor) -> Option<Valor> {
    match info.tipo_declarado.as_ref() {
        Some(tipo) => convertir_a_tipo_anotado(nuevo, tipo),
        None => convertir_para_tipo(&info.valor, nuevo),
    }
}

/// Aplica a `nuevo` las conversiones implícitas permitidas hacia el tipo de la variable
/// (ADR-0005 §1.4 usadas por asignación, ADR-0006 §1.4). `None` = incompatible.
fn convertir_para_tipo(viejo: &Valor, nuevo: Valor) -> Option<Valor> {
    match (viejo, &nuevo) {
        (Valor::Decimal(_), Valor::Entero(n)) => Some(Valor::decimal_desde_entero(*n)),
        (Valor::Real(_), Valor::Entero(n)) => Some(Valor::Real(*n as f64)),
        _ if std::mem::discriminant(viejo) == std::mem::discriminant(&nuevo) => Some(nuevo),
        _ => None,
    }
}

/// Localiza la info y el índice del ámbito para una variable.
fn localizar_info_entorno(env: &Entorno, nombre: &str) -> Option<(usize, VariableInfo)> {
    for (i, ambito) in env.ambitos.iter().enumerate().rev() {
        if let Some(info) = ambito.get(nombre) {
            return Some((i, info.clone()));
        }
    }
    None
}

fn evaluar_con(e: &Expresion, env: &mut Entorno) -> Result<Valor, ErrorEjecucion> {
    match e {
        Expresion::Literal { valor, .. } => Ok(valor.clone()),
        Expresion::Lambda {
            parametros,
            tipo_retorno: _,
            cuerpo,
            linea: _,
            columna: _,
        } => {
            let capturado = Arc::new(Mutex::new(env.clone()));
            Ok(Valor::FuncionClausura {
                parametros: parametros.clone(),
                cuerpo: cuerpo.clone(),
                entorno: capturado,
            })
        }
        Expresion::Variable {
            nombre,
            linea,
            columna,
        } => {
            if let Some(v) = env.buscar(nombre) {
                Ok(v)
            } else if env.funciones.contains_key(nombre) {
                Ok(Valor::Funcion(nombre.clone()))
            } else {
                Err(error(
                    CausaError::VariableNoDefinida(nombre.clone()),
                    *linea,
                    *columna,
                ))
            }
        }
        Expresion::Suma(a, b) => binaria(a, b, env, |x, y| x.suma(y)),
        Expresion::Resta(a, b) => binaria(a, b, env, |x, y| x.resta(y)),
        Expresion::Multiplicacion(a, b) => binaria(a, b, env, |x, y| x.multiplicacion(y)),
        Expresion::Division(a, b) => binaria(a, b, env, |x, y| x.division(y)),
        Expresion::Modulo(a, b) => binaria(a, b, env, |x, y| x.modulo(y)),
        Expresion::IgualIgual(a, b) => comparar(a, b, env, true, false, |ord| {
            ord == std::cmp::Ordering::Equal
        }),
        Expresion::Distinto(a, b) => comparar(a, b, env, true, true, |ord| {
            ord != std::cmp::Ordering::Equal
        }),
        Expresion::Menor(a, b) => comparar(a, b, env, false, false, |ord| {
            ord == std::cmp::Ordering::Less
        }),
        Expresion::MenorIgual(a, b) => comparar(a, b, env, false, false, |ord| {
            ord != std::cmp::Ordering::Greater
        }),
        Expresion::Mayor(a, b) => comparar(a, b, env, false, false, |ord| {
            ord == std::cmp::Ordering::Greater
        }),
        Expresion::MayorIgual(a, b) => comparar(a, b, env, false, false, |ord| {
            ord != std::cmp::Ordering::Less
        }),
        Expresion::No(a) => {
            let v = evaluar_con(a, env)?;
            match v {
                Valor::Logico(b) => Ok(Valor::Logico(!b)),
                _ => {
                    let (linea, columna) = ubicacion(e);
                    Err(error(
                        CausaError::OperandoNoLogico {
                            operador: "no",
                            obtenido: v.nombre_tipo(),
                        },
                        linea,
                        columna,
                    ))
                }
            }
        }
        Expresion::Y(a, b) => {
            let va = evaluar_con(a, env)?;
            match va {
                Valor::Logico(false) => Ok(Valor::Logico(false)),
                Valor::Logico(true) => {
                    let vb = evaluar_con(b, env)?;
                    let vb_tipo = vb.nombre_tipo();
                    match vb {
                        Valor::Logico(x) => Ok(Valor::Logico(x)),
                        _ => {
                            let (linea, columna) = ubicacion(b);
                            Err(error(
                                CausaError::OperandoNoLogico {
                                    operador: "y",
                                    obtenido: vb_tipo,
                                },
                                linea,
                                columna,
                            ))
                        }
                    }
                }
                _ => {
                    let (linea, columna) = ubicacion(a);
                    Err(error(
                        CausaError::OperandoNoLogico {
                            operador: "y",
                            obtenido: va.nombre_tipo(),
                        },
                        linea,
                        columna,
                    ))
                }
            }
        }
        Expresion::O(a, b) => {
            let va = evaluar_con(a, env)?;
            match va {
                Valor::Logico(true) => Ok(Valor::Logico(true)),
                Valor::Logico(false) => {
                    let vb = evaluar_con(b, env)?;
                    let vb_tipo = vb.nombre_tipo();
                    match vb {
                        Valor::Logico(x) => Ok(Valor::Logico(x)),
                        _ => {
                            let (linea, columna) = ubicacion(b);
                            Err(error(
                                CausaError::OperandoNoLogico {
                                    operador: "o",
                                    obtenido: vb_tipo,
                                },
                                linea,
                                columna,
                            ))
                        }
                    }
                }
                _ => {
                    let (linea, columna) = ubicacion(a);
                    Err(error(
                        CausaError::OperandoNoLogico {
                            operador: "o",
                            obtenido: va.nombre_tipo(),
                        },
                        linea,
                        columna,
                    ))
                }
            }
        }
        Expresion::Llamada {
            nombre,
            argumentos,
            linea,
            columna,
        } => {
            if es_builtin(nombre) {
                let mut vals = Vec::new();
                for a in argumentos {
                    vals.push(evaluar_con(a, env)?);
                }
                return evaluar_builtin(nombre, &vals, *linea, *columna, env);
            }

            // Clausura almacenada en variable.
            if let Some(Valor::FuncionClausura { .. }) = env.buscar(nombre) {
                let mut vals = Vec::new();
                for a in argumentos {
                    vals.push(evaluar_con(a, env)?);
                }
                let fv = env.buscar(nombre).unwrap();
                return env.llamar_por_valor(fv, &vals, *linea, *columna);
            }

            let fdef = if let Some(f) = env.funciones.get(nombre).cloned() {
                f
            } else if let Some(Valor::Funcion(fn_name)) = env.buscar(nombre) {
                env.funciones.get(&fn_name).cloned().ok_or_else(|| {
                    error(
                        CausaError::VariableNoDefinida(nombre.clone()),
                        *linea,
                        *columna,
                    )
                })?
            } else {
                return Err(error(
                    CausaError::VariableNoDefinida(nombre.clone()),
                    *linea,
                    *columna,
                ));
            };
            let mut vals = Vec::new();
            for a in argumentos {
                vals.push(evaluar_con(a, env)?);
            }
            if vals.len() != fdef.parametros.len() {
                return Err(error(
                    CausaError::FirmaInvalida {
                        funcion: nombre.clone(),
                        esperado: fdef.parametros.len(),
                        obtenido: vals.len(),
                    },
                    *linea,
                    *columna,
                ));
            }
            if fdef.parametros.iter().all(|(_, tipo)| tipo.is_none()) {
                if let Some(tipos_esperados) = &fdef.tipos_parametros_inferidos {
                    for ((parametro, _), (esperado, obtenido)) in fdef
                        .parametros
                        .iter()
                        .zip(tipos_esperados.iter().zip(vals.iter_mut()))
                    {
                        let tipo_obtenido = obtenido.nombre_tipo().to_string();
                        let Some(convertido) = convertir_para_tipo(esperado, obtenido.clone())
                        else {
                            return Err(error(
                                CausaError::TipoParametroIncompatible {
                                    funcion: nombre.clone(),
                                    parametro: parametro.clone(),
                                    esperado: esperado.nombre_tipo().to_string(),
                                    obtenido: tipo_obtenido,
                                },
                                *linea,
                                *columna,
                            ));
                        };
                        *obtenido = convertido;
                    }
                } else if let Some(definicion) = env.funciones.get_mut(nombre) {
                    definicion.tipos_parametros_inferidos = Some(vals.clone());
                }
            }
            let mut nuevo_env = Entorno {
                ambitos: vec![HashMap::new()],
                funciones: env.funciones.clone(),
                exportaciones: HashSet::new(),
                cargador: env.cargador.clone(),
                archivo: env.archivo.clone(),
                en_funcion: false,
            };
            for ((nombre_param, tipo_param), v) in fdef.parametros.iter().zip(vals.iter()) {
                let valor_parametro = if let Some(tipo) = tipo_param {
                    let obtenido = v.nombre_tipo().to_string();
                    convertir_a_tipo_anotado(v.clone(), tipo).ok_or_else(|| {
                        error(
                            CausaError::TipoParametroIncompatible {
                                funcion: nombre.clone(),
                                parametro: nombre_param.clone(),
                                esperado: tipo.nombre(),
                                obtenido,
                            },
                            *linea,
                            *columna,
                        )
                    })?
                } else {
                    v.clone()
                };
                nuevo_env.insertar_variable(
                    nombre_param.clone(),
                    VariableInfo {
                        valor: valor_parametro,
                        tipo_declarado: tipo_param.clone(),
                        mutable: true, // Parámetros son mutables como `var`
                        es_const: false,
                    },
                );
            }
            nuevo_env.en_funcion = true;
            let resultado = match nuevo_env.ejecutar_funcion(&fdef.cuerpo, &fdef.nombre)? {
                Some(valor) => valor,
                None if matches!(fdef.tipo_retorno_anotado, Some(TipoAnotacion::Nulo)) => {
                    Valor::Nulo
                }
                None => {
                    return Err(error(
                        CausaError::RetornoAusente {
                            funcion: fdef.nombre,
                        },
                        *linea,
                        *columna,
                    ));
                }
            };
            if let Some(tipo_esperado) = fdef.tipo_retorno_anotado.as_ref() {
                let obtenido = resultado.nombre_tipo().to_string();
                return convertir_a_tipo_anotado(resultado, tipo_esperado).ok_or_else(|| {
                    error(
                        CausaError::RetornoIncompatibleConAnotacion {
                            funcion: nombre.clone(),
                            esperado: tipo_esperado.nombre(),
                            obtenido,
                        },
                        *linea,
                        *columna,
                    )
                });
            }

            // Validar/guardar tipo de retorno inferido (ADR-0008).
            let tipo_obtenido = resultado.nombre_tipo();
            if let Some(tipo_esperado) = fdef.tipo_retorno {
                if tipo_obtenido != tipo_esperado {
                    return Err(error(
                        CausaError::TipoDeRetornoIncompatible {
                            esperado: tipo_esperado,
                            obtenido: tipo_obtenido,
                        },
                        *linea,
                        *columna,
                    ));
                }
            } else {
                // Primera llamada exitosa: guardar el tipo de retorno en la función original
                if let Some(fdef_mut) = env.funciones.get_mut(nombre) {
                    fdef_mut.tipo_retorno = Some(tipo_obtenido);
                }
            }
            Ok(resultado)
        }
        Expresion::ArregloLiteral { elementos, .. } => {
            let mut valores = Vec::new();
            for elem in elementos {
                valores.push(evaluar_con(elem, env)?);
            }
            // Por defecto, los literales `[...]` crean una Lista (dinámica).
            // Si se usa en contexto de arreglo con tamaño fijo, el evaluador de declaraciones lo convertirá.
            Ok(Valor::Lista(valores))
        }
        Expresion::Rango {
            inicio,
            fin,
            inclusivo,
            linea,
            columna,
        } => {
            let inicio_val = evaluar_con(inicio, env)?;
            let fin_val = evaluar_con(fin, env)?;
            let inicio_int = match inicio_val {
                Valor::Entero(v) => v,
                _ => {
                    return Err(error(
                        CausaError::RangoTipoInvalido {
                            obtenido: inicio_val.nombre_tipo(),
                        },
                        *linea,
                        *columna,
                    ));
                }
            };
            let fin_int = match fin_val {
                Valor::Entero(v) => v,
                _ => {
                    return Err(error(
                        CausaError::RangoTipoInvalido {
                            obtenido: fin_val.nombre_tipo(),
                        },
                        *linea,
                        *columna,
                    ));
                }
            };

            let cardinalidad = if inicio_int > fin_int {
                0usize
            } else {
                let diferencia = (fin_int - inicio_int) as usize;
                diferencia.saturating_add(usize::from(*inclusivo))
            };
            if cardinalidad > LIMITE_ITERACIONES {
                return Err(error(
                    CausaError::LimiteIteraciones {
                        limite: LIMITE_ITERACIONES,
                    },
                    *linea,
                    *columna,
                ));
            }

            let mut valores = Vec::with_capacity(cardinalidad);
            let mut actual = inicio_int;
            while if *inclusivo {
                actual <= fin_int
            } else {
                actual < fin_int
            } {
                valores.push(Valor::Entero(actual));
                if actual == fin_int {
                    break;
                }
                actual += 1;
            }
            Ok(Valor::Lista(valores))
        }
        Expresion::Indice {
            objeto,
            indice,
            linea,
            columna,
        } => {
            let obj = evaluar_con(objeto, env)?;
            let idx = evaluar_con(indice.as_ref(), env)?;
            let idx_val = match idx {
                Valor::Entero(i) => i,
                _ => {
                    return Err(error(
                        CausaError::AsignacionIncompatible {
                            esperado: "entero".into(),
                            obtenido: idx.nombre_tipo().to_string(),
                        },
                        *linea,
                        *columna,
                    ));
                }
            };
            match obj {
                Valor::Arreglo(arr) => {
                    let idx_usize = idx_val as usize;
                    if idx_usize >= arr.len() {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: "índice válido".into(),
                                obtenido: format!(
                                    "índice {} fuera de rango (tamaño {})",
                                    idx_usize,
                                    arr.len()
                                ),
                            },
                            *linea,
                            *columna,
                        ));
                    }
                    Ok(arr[idx_usize].clone())
                }
                Valor::Lista(lst) => {
                    let idx_usize = idx_val as usize;
                    if idx_usize >= lst.len() {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: "índice válido".into(),
                                obtenido: format!(
                                    "índice {} fuera de rango (tamaño {})",
                                    idx_usize,
                                    lst.len()
                                ),
                            },
                            *linea,
                            *columna,
                        ));
                    }
                    Ok(lst[idx_usize].clone())
                }
                _ => Err(error(
                    CausaError::AsignacionIncompatible {
                        esperado: "arreglo o lista".into(),
                        obtenido: obj.nombre_tipo().to_string(),
                    },
                    *linea,
                    *columna,
                )),
            }
        }
        Expresion::Acceso {
            objeto,
            miembro,
            linea,
            columna,
        } => {
            let obj = evaluar_con(objeto, env)?;
            match obj {
                Valor::Modulo(m) => {
                    if let Some(exportado) = m.exportaciones.get(miembro) {
                        match exportado {
                            SimbodoExportado::Variable(v) => Ok(v.clone()),
                            SimbodoExportado::Funcion(_f) => {
                                // Return a callable function reference.
                                // For now, we store it as a function in the env? No, we can't.
                                // Instead, we need a way to call it. Let me think...
                                // Actually, functions are called by name lookup in env.funciones.
                                // A module function accessed via alias.func() needs to be callable.
                                // The simplest: register it as a variable? No, functions aren't variables.
                                //
                                // For Fase 3, let me handle this: if the member is a function,
                                // we need to return something callable. But Valor doesn't have
                                // a "Funcion" variant.
                                //
                                // Hmm, this is a problem. Let me think about how functions are called.
                                // In evaluar_con, Llamada looks up env.funciones by name.
                                // For alias.func(), the expression is Acceso(Acceso(alias, func), Llamada).
                                // Wait, no: alias.func() is Llamada { nombre: "func", ... }? No.
                                // Actually, `alias.func()` parses as Llamada { nombre: ??? }.
                                // Let me check: how does the parser handle `alias.func()`?
                                // primaria() parses `alias` as Variable, then sees `.`, parses `func` as Acceso.
                                // Then the caller `()` makes it... a Llamada? No, Llamada is `identificador(...)`.
                                // `alias.func()` is `(alias.func)()` which is not a Llamada — it's an application of an expression.
                                //
                                // Wait, let me check the parser. primaria() returns an expression. Then the
                                // caller checks if it's followed by `(`. Let me look at how calls are parsed.
                                Err(error(
                                    CausaError::ImportacionesNoDisponibles,
                                    *linea,
                                    *columna,
                                ))
                            }
                        }
                    } else if m.entorno.lock().unwrap().variable(miembro).is_some()
                        || m.entorno.lock().unwrap().funcion(miembro).is_some()
                    {
                        Err(error(
                            CausaError::SimbodoPrivado {
                                nombre: miembro.clone(),
                                modulo: m.ruta.display().to_string(),
                            },
                            *linea,
                            *columna,
                        ))
                    } else {
                        Err(error(
                            CausaError::SimbodoNoExportado {
                                nombre: miembro.clone(),
                                modulo: m.ruta.display().to_string(),
                            },
                            *linea,
                            *columna,
                        ))
                    }
                }
                _ => Err(error(
                    CausaError::AsignacionIncompatible {
                        esperado: "modulo".into(),
                        obtenido: obj.nombre_tipo().to_string(),
                    },
                    *linea,
                    *columna,
                )),
            }
        }
        Expresion::LlamadaIndirecta {
            callee,
            argumentos,
            linea,
            columna,
        } => {
            // Funciones de primera clase (solo si no es acceso a miembro de módulo).
            if !matches!(callee.as_ref(), Expresion::Acceso { .. }) {
                let callee_val = evaluar_con(callee, env)?;
                match callee_val {
                    Valor::Funcion(fn_name) => {
                        let mut vals = Vec::new();
                        for a in argumentos {
                            vals.push(evaluar_con(a, env)?);
                        }
                        return env.llamar_por_valor(
                            Valor::Funcion(fn_name),
                            &vals,
                            *linea,
                            *columna,
                        );
                    }
                    Valor::FuncionClausura { .. } => {
                        let mut vals = Vec::new();
                        for a in argumentos {
                            vals.push(evaluar_con(a, env)?);
                        }
                        return env.llamar_por_valor(callee_val, &vals, *linea, *columna);
                    }
                    _ => {}
                }
            }
            // Evaluate callee: should be Acceso → Modulo.
            let (modulo, miembro) = match callee.as_ref() {
                Expresion::Acceso {
                    objeto, miembro, ..
                } => {
                    let obj = evaluar_con(objeto, env)?;
                    match obj {
                        Valor::Modulo(m) => (m, miembro.clone()),
                        _ => {
                            return Err(error(
                                CausaError::AsignacionIncompatible {
                                    esperado: "modulo".into(),
                                    obtenido: obj.nombre_tipo().to_string(),
                                },
                                *linea,
                                *columna,
                            ));
                        }
                    }
                }
                _ => {
                    return Err(error(
                        CausaError::AsignacionIncompatible {
                            esperado: "acceso a miembro".into(),
                            obtenido: "expresión".into(),
                        },
                        *linea,
                        *columna,
                    ));
                }
            };

            // Look up function in module.
            let fdef =
                if let Some(SimbodoExportado::Funcion(f)) = modulo.exportaciones.get(&miembro) {
                    f.clone()
                } else if modulo.entorno.lock().unwrap().funcion(&miembro).is_some() {
                    return Err(error(
                        CausaError::SimbodoPrivado {
                            nombre: miembro.clone(),
                            modulo: modulo.ruta.display().to_string(),
                        },
                        *linea,
                        *columna,
                    ));
                } else {
                    return Err(error(
                        CausaError::SimbodoNoExportado {
                            nombre: miembro.clone(),
                            modulo: modulo.ruta.display().to_string(),
                        },
                        *linea,
                        *columna,
                    ));
                };

            // Evaluate arguments.
            let mut vals = Vec::new();
            for a in argumentos {
                vals.push(evaluar_con(a, env)?);
            }

            if vals.len() != fdef.parametros.len() {
                return Err(error(
                    CausaError::FirmaInvalida {
                        funcion: miembro.clone(),
                        esperado: fdef.parametros.len(),
                        obtenido: vals.len(),
                    },
                    *linea,
                    *columna,
                ));
            }

            // Type checking for typed params.
            if fdef.parametros.iter().all(|(_, tipo)| tipo.is_none()) {
                // Untyped: skip type inference persistence (simplified for Fase 3).
            } else {
                for ((nombre_param, tipo), (v,)) in
                    fdef.parametros.iter().zip(vals.iter().map(|v| (v,)))
                {
                    if let Some(tipo) = tipo
                        && convertir_a_tipo_anotado(v.clone(), tipo).is_none()
                    {
                        return Err(error(
                            CausaError::TipoParametroIncompatible {
                                funcion: miembro.clone(),
                                parametro: nombre_param.clone(),
                                esperado: tipo.nombre(),
                                obtenido: v.nombre_tipo().to_string(),
                            },
                            *linea,
                            *columna,
                        ));
                    }
                }
            }

            // Execute function body in module's entorno.
            let mut mod_env = modulo.entorno.lock().unwrap().clone();
            // Register the function in the module env (already there).
            // Insert parameters.
            mod_env.ambitos.push(HashMap::new());
            for ((nombre_param, _), v) in fdef.parametros.iter().zip(vals.iter()) {
                mod_env.insertar_variable(
                    nombre_param.clone(),
                    VariableInfo {
                        valor: v.clone(),
                        tipo_declarado: None,
                        mutable: true,
                        es_const: false,
                    },
                );
            }

            // Execute body.
            let result = mod_env.ejecutar_funcion_sentencias(&fdef.cuerpo, &miembro);
            mod_env.ambitos.pop();

            match result {
                Ok(Some(v)) => Ok(v),
                Ok(None) => Err(error(
                    CausaError::RetornoAusente { funcion: miembro },
                    *linea,
                    *columna,
                )),
                Err(e) => Err(e),
            }
        }
    }
}

/// Evalúa una expresión con un entorno vacío (compatibilidad con el núcleo previo).
pub fn evaluar(e: &Expresion) -> Result<Valor, ErrorEjecucion> {
    evaluar_con(e, &mut Entorno::nueva())
}

/// Ubicación aproximada de una expresión (literal o variable más a la izquierda).
fn ubicacion(e: &Expresion) -> (usize, usize) {
    match e {
        Expresion::Literal { linea, columna, .. } | Expresion::Variable { linea, columna, .. } => {
            (*linea, *columna)
        }
        Expresion::Suma(a, _)
        | Expresion::Resta(a, _)
        | Expresion::Multiplicacion(a, _)
        | Expresion::Division(a, _)
        | Expresion::Modulo(a, _)
        | Expresion::IgualIgual(a, _)
        | Expresion::Distinto(a, _)
        | Expresion::Menor(a, _)
        | Expresion::MenorIgual(a, _)
        | Expresion::Mayor(a, _)
        | Expresion::MayorIgual(a, _)
        | Expresion::No(a)
        | Expresion::Y(a, _)
        | Expresion::O(a, _) => ubicacion(a),
        Expresion::Llamada { linea, columna, .. }
        | Expresion::ArregloLiteral { linea, columna, .. }
        | Expresion::Indice { linea, columna, .. }
        | Expresion::Acceso { linea, columna, .. }
        | Expresion::LlamadaIndirecta { linea, columna, .. }
        | Expresion::Lambda { linea, columna, .. }
        | Expresion::Rango { linea, columna, .. } => (*linea, *columna),
    }
}

fn binaria(
    a: &Expresion,
    b: &Expresion,
    env: &mut Entorno,
    op: impl Fn(&Valor, &Valor) -> Result<Valor, MotivoAritmetico>,
) -> Result<Valor, ErrorEjecucion> {
    let va = evaluar_con(a, env)?;
    let vb = evaluar_con(b, env)?;
    op(&va, &vb).map_err(|motivo| {
        let (linea, columna) = ubicacion(a);
        error(CausaError::Aritmetica(motivo), linea, columna)
    })
}

fn comparar(
    a: &Expresion,
    b: &Expresion,
    env: &mut Entorno,
    permitir_igualdad_nulo: bool,
    resultado_nan: bool,
    pred: impl Fn(std::cmp::Ordering) -> bool,
) -> Result<Valor, ErrorEjecucion> {
    let va = evaluar_con(a, env)?;
    let vb = evaluar_con(b, env)?;
    let (linea, columna) = ubicacion(a);
    if matches!(va, Valor::Nulo) || matches!(vb, Valor::Nulo) {
        if permitir_igualdad_nulo && matches!((&va, &vb), (Valor::Nulo, Valor::Nulo)) {
            return Ok(Valor::Logico(pred(std::cmp::Ordering::Equal)));
        }
        return Err(error(
            CausaError::TiposIncompatibles {
                izquierdo: va.nombre_tipo(),
                derecho: vb.nombre_tipo(),
            },
            linea,
            columna,
        ));
    }
    if matches!(va, Valor::Logico(_)) || matches!(vb, Valor::Logico(_)) {
        // No se permite comparar lógicos con ==/!= arbitrarios entre numéricos aquí; tipos incompatibles
        return Err(error(
            CausaError::TiposIncompatibles {
                izquierdo: va.nombre_tipo(),
                derecho: vb.nombre_tipo(),
            },
            linea,
            columna,
        ));
    }
    // Conversión implícita aprobada ADR-0005 §1.4
    match (&va, &vb) {
        (&Valor::Entero(x), &Valor::Entero(y)) => Ok(Valor::Logico(pred(x.cmp(&y)))),
        (&Valor::Entero(x), &Valor::Decimal(y)) => {
            Ok(Valor::Logico(pred((x as i128 * FACTOR_ESCALA).cmp(&y))))
        }
        (&Valor::Decimal(x), &Valor::Entero(y)) => {
            Ok(Valor::Logico(pred(x.cmp(&(y as i128 * FACTOR_ESCALA)))))
        }
        (&Valor::Entero(x), &Valor::Real(y)) => match (x as f64).partial_cmp(&y) {
            Some(orden) => Ok(Valor::Logico(pred(orden))),
            None => Ok(Valor::Logico(resultado_nan)),
        },
        (&Valor::Real(x), &Valor::Entero(y)) => match x.partial_cmp(&(y as f64)) {
            Some(orden) => Ok(Valor::Logico(pred(orden))),
            None => Ok(Valor::Logico(resultado_nan)),
        },
        (&Valor::Decimal(x), &Valor::Decimal(y)) => Ok(Valor::Logico(pred(x.cmp(&y)))),
        (&Valor::Real(x), &Valor::Real(y)) => match x.partial_cmp(&y) {
            Some(orden) => Ok(Valor::Logico(pred(orden))),
            None => Ok(Valor::Logico(resultado_nan)),
        },
        (&Valor::Decimal(_), &Valor::Real(_)) | (&Valor::Real(_), &Valor::Decimal(_)) => {
            // Prohibido hasta conversión explícita (ADR-0007).
            Err(error(
                CausaError::TiposIncompatibles {
                    izquierdo: va.nombre_tipo(),
                    derecho: vb.nombre_tipo(),
                },
                linea,
                columna,
            ))
        }
        // Comparaciones de texto (solo == y !=).
        #[allow(clippy::needless_borrowed_reference)]
        (&Valor::Texto(ref a), &Valor::Texto(ref b)) => Ok(Valor::Logico(pred(a.cmp(b)))),
        _ => Err(error(
            CausaError::TiposIncompatibles {
                izquierdo: va.nombre_tipo(),
                derecho: vb.nombre_tipo(),
            },
            linea,
            columna,
        )),
    }
}

fn es_builtin(nombre: &str) -> bool {
    matches!(
        nombre,
        "leer"
            | "leer_entero"
            | "leer_decimal"
            | "leer_real"
            | "longitud"
            | "contiene"
            | "empieza_con"
            | "termina_con"
            | "es_vacio"
            | "mayusculas"
            | "minusculas"
            | "recortar"
            | "reemplazar"
            | "subcadena"
            | "dividir"
            | "unir"
            | "a_entero"
            | "a_decimal"
            | "a_real"
            | "texto"
            | "abrir"
            | "leer_linea"
            | "leer_todo"
            | "escribir"
            | "escribir_linea"
            | "cerrar"
            | "fin_de_archivo"
            | "abs"
            | "max"
            | "min"
            | "raiz"
            | "potencia"
            | "redondear"
            | "map"
            | "filtrar"
            | "reducir"
            | "piso"
            | "techo"
            | "posicion"
            | "repetir"
            | "agregar"
            | "eliminar"
            | "invertir"
            | "ordenar"
            | "existe_archivo"
            | "leer_archivo"
            | "escribir_archivo"
            | "iniciar_tarea"
            | "esperar_tarea"
    )
}

fn err_builtin(causa: CausaError, linea: usize, columna: usize) -> ErrorEjecucion {
    error(causa, linea, columna)
}

fn evaluar_builtin(
    nombre: &str,
    args: &[Valor],
    linea: usize,
    columna: usize,
    entorno: &mut Entorno,
) -> Result<Valor, ErrorEjecucion> {
    match nombre {
        "leer" => {
            if !args.is_empty() {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 0,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let mut entrada = String::new();
            match io::stdin().lock().read_line(&mut entrada) {
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
                Err(e) => Err(err_builtin(
                    CausaError::ErrorDeArchivo {
                        ruta: "<stdin>".to_string(),
                        detalle: e.to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "leer_entero" => {
            if !args.is_empty() {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 0,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let mut entrada = String::new();
            io::stdin().lock().read_line(&mut entrada).map_err(|e| {
                err_builtin(
                    CausaError::ErrorDeArchivo {
                        ruta: "<stdin>".to_string(),
                        detalle: e.to_string(),
                    },
                    linea,
                    columna,
                )
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
                Err(_) => Err(err_builtin(
                    CausaError::EntradaInvalida {
                        esperado: "entero".to_string(),
                        obtenido: entrada,
                    },
                    linea,
                    columna,
                )),
            }
        }
        "leer_decimal" => {
            if !args.is_empty() {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 0,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let mut entrada = String::new();
            io::stdin().lock().read_line(&mut entrada).map_err(|e| {
                err_builtin(
                    CausaError::ErrorDeArchivo {
                        ruta: "<stdin>".to_string(),
                        detalle: e.to_string(),
                    },
                    linea,
                    columna,
                )
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
                Err(_) => Err(err_builtin(
                    CausaError::EntradaInvalida {
                        esperado: "decimal".to_string(),
                        obtenido: entrada,
                    },
                    linea,
                    columna,
                )),
            }
        }
        "leer_real" => {
            if !args.is_empty() {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 0,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let mut entrada = String::new();
            io::stdin().lock().read_line(&mut entrada).map_err(|e| {
                err_builtin(
                    CausaError::ErrorDeArchivo {
                        ruta: "<stdin>".to_string(),
                        detalle: e.to_string(),
                    },
                    linea,
                    columna,
                )
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
                Err(_) => Err(err_builtin(
                    CausaError::EntradaInvalida {
                        esperado: "real".to_string(),
                        obtenido: entrada,
                    },
                    linea,
                    columna,
                )),
            }
        }
        "longitud" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Texto(t) => Ok(Valor::Entero(t.chars().count() as i64)),
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "empieza_con" => {
            binario_texto_logico(args, nombre, linea, columna, |t, sub| t.starts_with(sub))
        }
        "termina_con" => {
            binario_texto_logico(args, nombre, linea, columna, |t, sub| t.ends_with(sub))
        }
        "es_vacio" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Texto(t) => Ok(Valor::Logico(t.is_empty())),
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "mayusculas" => unario_texto(args, nombre, linea, columna, |t| t.to_uppercase()),
        "minusculas" => unario_texto(args, nombre, linea, columna, |t| t.to_lowercase()),
        "recortar" => unario_texto(args, nombre, linea, columna, |t| t.trim().to_string()),
        "reemplazar" => {
            if args.len() != 3 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 3,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1], &args[2]) {
                (Valor::Texto(t), Valor::Texto(buscar), Valor::Texto(nuevo)) => {
                    Ok(Valor::Texto(t.replace(buscar, nuevo)))
                }
                _ => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto".to_string(),
                        obtenido: args[0].nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "subcadena" => {
            if args.len() != 3 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 3,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1], &args[2]) {
                (Valor::Texto(t), Valor::Entero(inicio), Valor::Entero(fin)) => {
                    let chars: Vec<char> = t.chars().collect();
                    if *inicio < 0 || *fin < 0 || *inicio > *fin || *fin as usize > chars.len() {
                        let indice = if *inicio < 0 {
                            *inicio
                        } else if *fin < 0 {
                            *fin
                        } else if *inicio > *fin {
                            *inicio
                        } else {
                            *fin
                        };
                        return Err(err_builtin(
                            CausaError::IndiceTextoFueraDeRango {
                                indice,
                                longitud: chars.len(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    let s: String = chars[*inicio as usize..*fin as usize].iter().collect();
                    Ok(Valor::Texto(s))
                }
                _ => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto, entero, entero".to_string(),
                        obtenido: args[0].nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "dividir" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1]) {
                (Valor::Texto(t), Valor::Texto(sep)) => {
                    if sep.is_empty() {
                        return Err(err_builtin(CausaError::SeparadorVacio, linea, columna));
                    }
                    let partes: Vec<Valor> =
                        t.split(sep).map(|p| Valor::Texto(p.to_string())).collect();
                    Ok(Valor::Lista(partes))
                }
                _ => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto, texto".to_string(),
                        obtenido: args[0].nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "unir" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1]) {
                (Valor::Lista(items), Valor::Texto(sep)) => {
                    let mut partes = Vec::new();
                    for item in items {
                        match item {
                            Valor::Texto(s) => partes.push(s.clone()),
                            otro => {
                                return Err(err_builtin(
                                    CausaError::ConversionTextoInvalida {
                                        valor: otro.to_string(),
                                        esperado: "texto".to_string(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                        }
                    }
                    Ok(Valor::Texto(partes.join(sep)))
                }
                _ => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "lista<texto>, texto".to_string(),
                        obtenido: args[0].nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "a_entero" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Texto(t) => match t.trim().parse::<i64>() {
                    Ok(v) => Ok(Valor::Entero(v)),
                    Err(_) => Err(err_builtin(
                        CausaError::ConversionTextoInvalida {
                            valor: t.clone(),
                            esperado: "entero".to_string(),
                        },
                        linea,
                        columna,
                    )),
                },
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "a_decimal" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Texto(t) => match Valor::decimal_desde_lexema(t.trim()) {
                    Ok(v) => Ok(v),
                    Err(_) => Err(err_builtin(
                        CausaError::ConversionTextoInvalida {
                            valor: t.clone(),
                            esperado: "decimal".to_string(),
                        },
                        linea,
                        columna,
                    )),
                },
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "a_real" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Texto(t) => match t.trim().parse::<f64>() {
                    Ok(v) => Ok(Valor::Real(v)),
                    Err(_) => Err(err_builtin(
                        CausaError::ConversionTextoInvalida {
                            valor: t.clone(),
                            esperado: "real".to_string(),
                        },
                        linea,
                        columna,
                    )),
                },
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "texto" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            Ok(Valor::Texto(args[0].to_string()))
        }
        "abrir" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1]) {
                (Valor::Texto(ruta), Valor::Texto(modo)) => {
                    let file_result = match modo.as_str() {
                        "leer" => File::open(ruta),
                        "escribir" => OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(ruta),
                        "agregar" => OpenOptions::new().append(true).create(true).open(ruta),
                        _ => {
                            return Err(err_builtin(
                                CausaError::ErrorDeArchivo {
                                    ruta: ruta.clone(),
                                    detalle: format!("modo inválido: {modo}"),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    let archivo = match file_result {
                        Ok(f) => f,
                        Err(e) => {
                            if e.kind() == io::ErrorKind::NotFound {
                                return Err(err_builtin(
                                    CausaError::ArchivoNoEncontrado { ruta: ruta.clone() },
                                    linea,
                                    columna,
                                ));
                            }
                            if e.kind() == io::ErrorKind::PermissionDenied {
                                return Err(err_builtin(
                                    CausaError::PermisosDenegados { ruta: ruta.clone() },
                                    linea,
                                    columna,
                                ));
                            }
                            return Err(err_builtin(
                                CausaError::ErrorDeArchivo {
                                    ruta: ruta.clone(),
                                    detalle: e.to_string(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    let (lector, escritor) = match modo.as_str() {
                        "leer" => (Some(BufReader::new(archivo)), None),
                        "escribir" | "agregar" => (None, Some(BufWriter::new(archivo))),
                        _ => (None, None),
                    };
                    Ok(Valor::Archivo(std::sync::Arc::new(std::sync::Mutex::new(
                        crate::valor::ArchivoInterno {
                            ruta: ruta.clone(),
                            modo: modo.clone(),
                            cerrado: false,
                            lector,
                            escritor,
                        },
                    ))))
                }
                _ => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto, texto".to_string(),
                        obtenido: args[0].nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "leer_linea" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Archivo(arc) => {
                    let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                    if guard.cerrado {
                        return Err(err_builtin(CausaError::ArchivoYaCerrado, linea, columna));
                    }
                    let ruta = guard.ruta.clone();
                    let lector = guard.lector.as_mut().ok_or_else(|| {
                        err_builtin(
                            CausaError::ErrorDeArchivo {
                                ruta: ruta.clone(),
                                detalle: "archivo no abierto en modo lectura".to_string(),
                            },
                            linea,
                            columna,
                        )
                    })?;
                    let mut linea_texto = String::new();
                    match lector.read_line(&mut linea_texto) {
                        Ok(0) => Ok(Valor::Texto(String::new())),
                        Ok(_) => {
                            if linea_texto.ends_with('\n') {
                                linea_texto.pop();
                            }
                            if linea_texto.ends_with('\r') {
                                linea_texto.pop();
                            }
                            Ok(Valor::Texto(linea_texto))
                        }
                        Err(e) => Err(err_builtin(
                            CausaError::ErrorDeArchivo {
                                ruta: ruta.clone(),
                                detalle: e.to_string(),
                            },
                            linea,
                            columna,
                        )),
                    }
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "archivo".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "leer_todo" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Archivo(arc) => {
                    let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                    if guard.cerrado {
                        return Err(err_builtin(CausaError::ArchivoYaCerrado, linea, columna));
                    }
                    let ruta = guard.ruta.clone();
                    let lector = guard.lector.as_mut().ok_or_else(|| {
                        err_builtin(
                            CausaError::ErrorDeArchivo {
                                ruta: ruta.clone(),
                                detalle: "archivo no abierto en modo lectura".to_string(),
                            },
                            linea,
                            columna,
                        )
                    })?;
                    let mut contenido = String::new();
                    match lector.read_to_string(&mut contenido) {
                        Ok(_) => Ok(Valor::Texto(contenido)),
                        Err(e) => Err(err_builtin(
                            CausaError::ErrorDeArchivo {
                                ruta: ruta.clone(),
                                detalle: e.to_string(),
                            },
                            linea,
                            columna,
                        )),
                    }
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "archivo".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "escribir" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1]) {
                (Valor::Archivo(arc), Valor::Texto(t)) => {
                    let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                    if guard.cerrado {
                        return Err(err_builtin(CausaError::ArchivoYaCerrado, linea, columna));
                    }
                    let ruta = guard.ruta.clone();
                    let escritor = guard.escritor.as_mut().ok_or_else(|| {
                        err_builtin(
                            CausaError::ErrorDeArchivo {
                                ruta: ruta.clone(),
                                detalle: "archivo no abierto en modo escritura".to_string(),
                            },
                            linea,
                            columna,
                        )
                    })?;
                    escritor.write_all(t.as_bytes()).map_err(|e| {
                        err_builtin(
                            CausaError::ErrorDeArchivo {
                                ruta: ruta.clone(),
                                detalle: e.to_string(),
                            },
                            linea,
                            columna,
                        )
                    })?;
                    Ok(Valor::Nulo)
                }
                _ => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "archivo, texto".to_string(),
                        obtenido: args[0].nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "escribir_linea" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1]) {
                (Valor::Archivo(arc), Valor::Texto(t)) => {
                    let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                    if guard.cerrado {
                        return Err(err_builtin(CausaError::ArchivoYaCerrado, linea, columna));
                    }
                    let ruta = guard.ruta.clone();
                    let escritor = guard.escritor.as_mut().ok_or_else(|| {
                        err_builtin(
                            CausaError::ErrorDeArchivo {
                                ruta: ruta.clone(),
                                detalle: "archivo no abierto en modo escritura".to_string(),
                            },
                            linea,
                            columna,
                        )
                    })?;
                    writeln!(escritor, "{t}").map_err(|e| {
                        err_builtin(
                            CausaError::ErrorDeArchivo {
                                ruta: ruta.clone(),
                                detalle: e.to_string(),
                            },
                            linea,
                            columna,
                        )
                    })?;
                    Ok(Valor::Nulo)
                }
                _ => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "archivo, texto".to_string(),
                        obtenido: args[0].nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "cerrar" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Archivo(arc) => {
                    let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                    guard.cerrado = true;
                    guard.lector = None;
                    guard.escritor = None;
                    Ok(Valor::Nulo)
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "archivo".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "fin_de_archivo" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Archivo(arc) => {
                    let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
                    if guard.cerrado {
                        return Err(err_builtin(CausaError::ArchivoYaCerrado, linea, columna));
                    }
                    let ruta = guard.ruta.clone();
                    match guard.lector.as_mut() {
                        Some(lector) => {
                            let buf = lector.fill_buf().map_err(|e| {
                                err_builtin(
                                    CausaError::ErrorDeArchivo {
                                        ruta: ruta.clone(),
                                        detalle: e.to_string(),
                                    },
                                    linea,
                                    columna,
                                )
                            })?;
                            Ok(Valor::Logico(buf.is_empty()))
                        }
                        None => Ok(Valor::Logico(false)),
                    }
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "archivo".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "abs" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Entero(v) => match v.checked_abs() {
                    Some(a) => Ok(Valor::Entero(a)),
                    None => Err(err_builtin(
                        CausaError::Aritmetica(MotivoAritmetico::DesbordamientoEntero),
                        linea,
                        columna,
                    )),
                },
                Valor::Decimal(v) => Ok(Valor::Decimal(v.abs())),
                Valor::Real(v) => Ok(Valor::Real(v.abs())),
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "número".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "max" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1]) {
                (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Entero(*a.max(b))),
                (Valor::Decimal(a), Valor::Decimal(b)) => Ok(Valor::Decimal(*a.max(b))),
                (Valor::Real(a), Valor::Real(b)) => Ok(Valor::Real(a.max(*b))),
                (Valor::Entero(a), Valor::Real(b)) => Ok(Valor::Real((*a as f64).max(*b))),
                (Valor::Real(a), Valor::Entero(b)) => Ok(Valor::Real(a.max(*b as f64))),
                _ => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "números compatibles".to_string(),
                        obtenido: args[0].nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "min" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1]) {
                (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Entero(*a.min(b))),
                (Valor::Decimal(a), Valor::Decimal(b)) => Ok(Valor::Decimal(*a.min(b))),
                (Valor::Real(a), Valor::Real(b)) => Ok(Valor::Real(a.min(*b))),
                (Valor::Entero(a), Valor::Real(b)) => Ok(Valor::Real((*a as f64).min(*b))),
                (Valor::Real(a), Valor::Entero(b)) => Ok(Valor::Real(a.min(*b as f64))),
                _ => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "números compatibles".to_string(),
                        obtenido: args[0].nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "raiz" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let v = match &args[0] {
                Valor::Entero(x) => *x as f64,
                Valor::Decimal(x) => *x as f64 / FACTOR_ESCALA as f64,
                Valor::Real(x) => *x,
                otro => {
                    return Err(err_builtin(
                        CausaError::AsignacionIncompatible {
                            esperado: "número".to_string(),
                            obtenido: otro.nombre_tipo().to_string(),
                        },
                        linea,
                        columna,
                    ));
                }
            };
            Ok(Valor::Real(v.sqrt()))
        }
        "potencia" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1]) {
                (Valor::Entero(base), Valor::Entero(exp)) if *exp >= 0 => {
                    match base.checked_pow(*exp as u32) {
                        Some(r) => Ok(Valor::Entero(r)),
                        None => Err(err_builtin(
                            CausaError::Aritmetica(MotivoAritmetico::DesbordamientoEntero),
                            linea,
                            columna,
                        )),
                    }
                }
                _ => {
                    let base = match &args[0] {
                        Valor::Entero(x) => *x as f64,
                        Valor::Decimal(x) => *x as f64 / FACTOR_ESCALA as f64,
                        Valor::Real(x) => *x,
                        _ => {
                            return Err(err_builtin(
                                CausaError::AsignacionIncompatible {
                                    esperado: "número".to_string(),
                                    obtenido: args[0].nombre_tipo().to_string(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    let exp = match &args[1] {
                        Valor::Entero(x) => *x as f64,
                        Valor::Decimal(x) => *x as f64 / FACTOR_ESCALA as f64,
                        Valor::Real(x) => *x,
                        _ => {
                            return Err(err_builtin(
                                CausaError::AsignacionIncompatible {
                                    esperado: "número".to_string(),
                                    obtenido: args[1].nombre_tipo().to_string(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    Ok(Valor::Real(base.powf(exp)))
                }
            }
        }
        "agregar" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Arreglo(v) => {
                    let mut n = v.clone();
                    n.push(args[1].clone());
                    Ok(Valor::Arreglo(n))
                }
                Valor::Lista(v) => {
                    let mut n = v.clone();
                    n.push(args[1].clone());
                    Ok(Valor::Lista(n))
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "arreglo o lista".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "eliminar" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let idx = match &args[1] {
                Valor::Entero(i) if *i >= 0 => *i as usize,
                _ => {
                    return Err(err_builtin(
                        CausaError::AsignacionIncompatible {
                            esperado: "entero no negativo".to_string(),
                            obtenido: args[1].nombre_tipo().to_string(),
                        },
                        linea,
                        columna,
                    ));
                }
            };
            match &args[0] {
                Valor::Arreglo(v) => {
                    if idx >= v.len() {
                        return Err(err_builtin(
                            CausaError::IndiceTextoFueraDeRango {
                                indice: idx as i64,
                                longitud: v.len(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    let mut n = v.clone();
                    n.remove(idx);
                    Ok(Valor::Arreglo(n))
                }
                Valor::Lista(v) => {
                    if idx >= v.len() {
                        return Err(err_builtin(
                            CausaError::IndiceTextoFueraDeRango {
                                indice: idx as i64,
                                longitud: v.len(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    let mut n = v.clone();
                    n.remove(idx);
                    Ok(Valor::Lista(n))
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "arreglo o lista".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "contiene" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Arreglo(v) | Valor::Lista(v) => Ok(Valor::Logico(
                    v.iter().any(|x| valores_iguales(x, &args[1])),
                )),
                Valor::Texto(t) => match &args[1] {
                    Valor::Texto(sub) => Ok(Valor::Logico(t.contains(sub))),
                    _ => Err(err_builtin(
                        CausaError::AsignacionIncompatible {
                            esperado: "texto".to_string(),
                            obtenido: args[1].nombre_tipo().to_string(),
                        },
                        linea,
                        columna,
                    )),
                },
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "arreglo, lista o texto".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "invertir" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Arreglo(v) => {
                    let mut n = v.clone();
                    n.reverse();
                    Ok(Valor::Arreglo(n))
                }
                Valor::Lista(v) => {
                    let mut n = v.clone();
                    n.reverse();
                    Ok(Valor::Lista(n))
                }
                Valor::Texto(t) => {
                    let mut c: Vec<char> = t.chars().collect();
                    c.reverse();
                    Ok(Valor::Texto(c.into_iter().collect()))
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "arreglo, lista o texto".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "ordenar" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Arreglo(v) => {
                    let mut n = v.clone();
                    if !ordenar_valores(&mut n) {
                        return Err(err_builtin(
                            CausaError::AsignacionIncompatible {
                                esperado: "valores ordenables".to_string(),
                                obtenido: "valores no ordenables".to_string(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    Ok(Valor::Arreglo(n))
                }
                Valor::Lista(v) => {
                    let mut n = v.clone();
                    if !ordenar_valores(&mut n) {
                        return Err(err_builtin(
                            CausaError::AsignacionIncompatible {
                                esperado: "valores ordenables".to_string(),
                                obtenido: "valores no ordenables".to_string(),
                            },
                            linea,
                            columna,
                        ));
                    }
                    Ok(Valor::Lista(n))
                }
                Valor::Texto(t) => {
                    let mut c: Vec<char> = t.chars().collect();
                    c.sort();
                    Ok(Valor::Texto(c.into_iter().collect()))
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "arreglo, lista o texto".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "existe_archivo" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Texto(ruta) => {
                    let ruta_segura = validar_ruta_archivo(ruta, entorno, linea, columna)?;
                    Ok(Valor::Logico(ruta_segura.exists()))
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "leer_archivo" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match &args[0] {
                Valor::Texto(ruta) => {
                    let ruta_segura = validar_ruta_archivo(ruta, entorno, linea, columna)?;
                    let ruta_str = ruta_segura.display().to_string();
                    match std::fs::read_to_string(&ruta_segura) {
                        Ok(contenido) => Ok(Valor::Texto(contenido)),
                        Err(e) => {
                            if e.kind() == io::ErrorKind::NotFound {
                                Err(err_builtin(
                                    CausaError::ArchivoNoEncontrado { ruta: ruta_str },
                                    linea,
                                    columna,
                                ))
                            } else if e.kind() == io::ErrorKind::PermissionDenied {
                                Err(err_builtin(
                                    CausaError::PermisosDenegados { ruta: ruta_str },
                                    linea,
                                    columna,
                                ))
                            } else {
                                Err(err_builtin(
                                    CausaError::ErrorDeArchivo {
                                        ruta: ruta_str,
                                        detalle: e.to_string(),
                                    },
                                    linea,
                                    columna,
                                ))
                            }
                        }
                    }
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "escribir_archivo" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            match (&args[0], &args[1]) {
                (Valor::Texto(ruta), Valor::Texto(contenido)) => {
                    let ruta_segura = validar_ruta_archivo(ruta, entorno, linea, columna)?;
                    let ruta_str = ruta_segura.display().to_string();
                    match std::fs::write(&ruta_segura, contenido) {
                        Ok(()) => Ok(Valor::Nulo),
                        Err(e) => {
                            if e.kind() == io::ErrorKind::PermissionDenied {
                                Err(err_builtin(
                                    CausaError::PermisosDenegados { ruta: ruta_str },
                                    linea,
                                    columna,
                                ))
                            } else {
                                Err(err_builtin(
                                    CausaError::ErrorDeArchivo {
                                        ruta: ruta_str,
                                        detalle: e.to_string(),
                                    },
                                    linea,
                                    columna,
                                ))
                            }
                        }
                    }
                }
                _ => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "texto, texto".to_string(),
                        obtenido: args[0].nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "map" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let func_val = args[1].clone();
            match &args[0] {
                Valor::Arreglo(v) => {
                    let mut out = Vec::new();
                    for item in v {
                        let r = entorno.llamar_por_valor(
                            func_val.clone(),
                            std::slice::from_ref(item),
                            linea,
                            columna,
                        )?;
                        out.push(r);
                    }
                    Ok(Valor::Arreglo(out))
                }
                Valor::Lista(v) => {
                    let mut out = Vec::new();
                    for item in v {
                        let r = entorno.llamar_por_valor(
                            func_val.clone(),
                            std::slice::from_ref(item),
                            linea,
                            columna,
                        )?;
                        out.push(r);
                    }
                    Ok(Valor::Lista(out))
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "arreglo o lista".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "filtrar" => {
            if args.len() != 2 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 2,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let func_val = args[1].clone();
            match &args[0] {
                Valor::Arreglo(v) => {
                    let mut out = Vec::new();
                    for item in v {
                        let r = entorno.llamar_por_valor(
                            func_val.clone(),
                            std::slice::from_ref(item),
                            linea,
                            columna,
                        )?;
                        match r {
                            Valor::Logico(true) => out.push(item.clone()),
                            Valor::Logico(false) => {}
                            _ => {
                                return Err(err_builtin(
                                    CausaError::AsignacionIncompatible {
                                        esperado: "lógico".to_string(),
                                        obtenido: r.nombre_tipo().to_string(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                        }
                    }
                    Ok(Valor::Arreglo(out))
                }
                Valor::Lista(v) => {
                    let mut out = Vec::new();
                    for item in v {
                        let r = entorno.llamar_por_valor(
                            func_val.clone(),
                            std::slice::from_ref(item),
                            linea,
                            columna,
                        )?;
                        match r {
                            Valor::Logico(true) => out.push(item.clone()),
                            Valor::Logico(false) => {}
                            _ => {
                                return Err(err_builtin(
                                    CausaError::AsignacionIncompatible {
                                        esperado: "lógico".to_string(),
                                        obtenido: r.nombre_tipo().to_string(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                        }
                    }
                    Ok(Valor::Lista(out))
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "arreglo o lista".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "reducir" => {
            if args.len() != 3 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 3,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let func_val = args[1].clone();
            let mut acc = args[2].clone();
            match &args[0] {
                Valor::Arreglo(v) | Valor::Lista(v) => {
                    for item in v {
                        acc = entorno.llamar_por_valor(
                            func_val.clone(),
                            &[acc.clone(), item.clone()],
                            linea,
                            columna,
                        )?;
                    }
                    Ok(acc)
                }
                otro => Err(err_builtin(
                    CausaError::AsignacionIncompatible {
                        esperado: "arreglo o lista".to_string(),
                        obtenido: otro.nombre_tipo().to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        "iniciar_tarea" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let func_name = match &args[0] {
                Valor::Texto(s) => s.clone(),
                otro => {
                    return Err(err_builtin(
                        CausaError::AsignacionIncompatible {
                            esperado: "texto".to_string(),
                            obtenido: otro.nombre_tipo().to_string(),
                        },
                        linea,
                        columna,
                    ));
                }
            };
            if !entorno.funciones.contains_key(&func_name) {
                return Err(err_builtin(
                    CausaError::VariableNoDefinida(func_name),
                    linea,
                    columna,
                ));
            }
            let env_clone = entorno.clone();
            let handle = thread::spawn(move || {
                let mut env = env_clone;
                let expr = Expresion::Llamada {
                    nombre: func_name.clone(),
                    argumentos: Vec::new(),
                    linea: 0,
                    columna: 0,
                };
                evaluar_con(&expr, &mut env).map_err(|e| e.to_string())
            });
            let mut tareas = TAREAS.lock().unwrap();
            let id = tareas.len();
            tareas.push(Some(handle));
            Ok(Valor::Entero(id as i64))
        }
        "esperar_tarea" => {
            if args.len() != 1 {
                return Err(err_builtin(
                    CausaError::FirmaInvalida {
                        funcion: nombre.to_string(),
                        esperado: 1,
                        obtenido: args.len(),
                    },
                    linea,
                    columna,
                ));
            }
            let id = match &args[0] {
                Valor::Entero(v) => *v as usize,
                otro => {
                    return Err(err_builtin(
                        CausaError::AsignacionIncompatible {
                            esperado: "entero".to_string(),
                            obtenido: otro.nombre_tipo().to_string(),
                        },
                        linea,
                        columna,
                    ));
                }
            };
            let handle = {
                let mut tareas = TAREAS.lock().unwrap();
                if id >= tareas.len() {
                    return Err(err_builtin(
                        CausaError::VariableNoDefinida(format!("tarea {}", id)),
                        linea,
                        columna,
                    ));
                }
                tareas[id].take()
            };
            match handle {
                Some(h) => match h.join() {
                    Ok(Ok(v)) => Ok(v),
                    Ok(Err(e)) => Err(err_builtin(
                        CausaError::ErrorDeArchivo {
                            ruta: "<tarea>".to_string(),
                            detalle: e,
                        },
                        linea,
                        columna,
                    )),
                    Err(_) => Err(err_builtin(
                        CausaError::ErrorDeArchivo {
                            ruta: "<tarea>".to_string(),
                            detalle: "la tarea entró en pánico".to_string(),
                        },
                        linea,
                        columna,
                    )),
                },
                None => Err(err_builtin(
                    CausaError::ErrorDeArchivo {
                        ruta: "<tarea>".to_string(),
                        detalle: "la tarea ya fue esperada".to_string(),
                    },
                    linea,
                    columna,
                )),
            }
        }
        _ => Err(err_builtin(
            CausaError::VariableNoDefinida(nombre.to_string()),
            linea,
            columna,
        )),
    }
}

fn binario_texto_logico<F>(
    args: &[Valor],
    nombre: &str,
    linea: usize,
    columna: usize,
    f: F,
) -> Result<Valor, ErrorEjecucion>
where
    F: Fn(&str, &str) -> bool,
{
    if args.len() != 2 {
        return Err(err_builtin(
            CausaError::FirmaInvalida {
                funcion: nombre.to_string(),
                esperado: 2,
                obtenido: args.len(),
            },
            linea,
            columna,
        ));
    }
    match (&args[0], &args[1]) {
        (Valor::Texto(a), Valor::Texto(b)) => Ok(Valor::Logico(f(a, b))),
        _ => Err(err_builtin(
            CausaError::AsignacionIncompatible {
                esperado: "texto, texto".to_string(),
                obtenido: args[0].nombre_tipo().to_string(),
            },
            linea,
            columna,
        )),
    }
}

fn unario_texto<F>(
    args: &[Valor],
    nombre: &str,
    linea: usize,
    columna: usize,
    f: F,
) -> Result<Valor, ErrorEjecucion>
where
    F: Fn(&str) -> String,
{
    if args.len() != 1 {
        return Err(err_builtin(
            CausaError::FirmaInvalida {
                funcion: nombre.to_string(),
                esperado: 1,
                obtenido: args.len(),
            },
            linea,
            columna,
        ));
    }
    match &args[0] {
        Valor::Texto(t) => Ok(Valor::Texto(f(t))),
        otro => Err(err_builtin(
            CausaError::AsignacionIncompatible {
                esperado: "texto".to_string(),
                obtenido: otro.nombre_tipo().to_string(),
            },
            linea,
            columna,
        )),
    }
}

fn resolver_modulo(ruta: &str) -> PathBuf {
    let base = PathBuf::from(ruta);
    if base.is_file() {
        return base;
    }
    if let Some(resuelto) = crate::paquetes::resolver_desde_bloqueo(ruta) {
        return resuelto;
    }
    let candidatos = [
        PathBuf::from(format!("{}.bsk", ruta)),
        base.join("lib.bsk"),
        base.join("src").join("lib.bsk"),
        PathBuf::from("paquetes").join(ruta).join("lib.bsk"),
        PathBuf::from("vendor").join(ruta).join("lib.bsk"),
    ];
    for c in candidatos {
        if c.is_file() {
            return c;
        }
    }
    base
}

fn valores_iguales(a: &Valor, b: &Valor) -> bool {
    match (a, b) {
        (Valor::Entero(x), Valor::Entero(y)) => x == y,
        (Valor::Decimal(x), Valor::Decimal(y)) => x == y,
        (Valor::Real(x), Valor::Real(y)) => x == y,
        (Valor::Logico(x), Valor::Logico(y)) => x == y,
        (Valor::Texto(x), Valor::Texto(y)) => x == y,
        (Valor::Nulo, Valor::Nulo) => true,
        (Valor::Arreglo(x), Valor::Arreglo(y)) | (Valor::Lista(x), Valor::Lista(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| valores_iguales(a, b))
        }
        _ => false,
    }
}

fn ordenar_valores(valores: &mut [Valor]) -> bool {
    if valores.is_empty() {
        return true;
    }
    let primero = &valores[0];
    for v in &valores[1..] {
        match (primero, v) {
            (Valor::Entero(_), Valor::Entero(_)) => {}
            (Valor::Decimal(_), Valor::Decimal(_)) => {}
            (Valor::Real(_), Valor::Real(_)) => {}
            (Valor::Texto(_), Valor::Texto(_)) => {}
            _ => return false,
        }
    }
    valores.sort_by(|a, b| match (a, b) {
        (Valor::Entero(x), Valor::Entero(y)) => x.cmp(y),
        (Valor::Decimal(x), Valor::Decimal(y)) => x.cmp(y),
        (Valor::Real(x), Valor::Real(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Valor::Texto(x), Valor::Texto(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    true
}

fn validar_ruta_archivo(
    ruta: &str,
    entorno: &Entorno,
    linea: usize,
    columna: usize,
) -> Result<std::path::PathBuf, ErrorEjecucion> {
    let root = match &entorno.cargador {
        Some(c) => c.lock().unwrap().root().to_path_buf(),
        None => std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    };
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    let path = std::path::Path::new(ruta);
    let canonical = if path.exists() {
        std::fs::canonicalize(path).map_err(|e| {
            err_builtin(
                CausaError::ErrorDeArchivo {
                    ruta: ruta.to_string(),
                    detalle: e.to_string(),
                },
                linea,
                columna,
            )
        })?
    } else {
        let parent = path.parent().unwrap_or(std::path::Path::new("."));
        let canon_parent = std::fs::canonicalize(parent).map_err(|e| {
            err_builtin(
                CausaError::ErrorDeArchivo {
                    ruta: ruta.to_string(),
                    detalle: e.to_string(),
                },
                linea,
                columna,
            )
        })?;
        match path.file_name() {
            Some(name) => canon_parent.join(name),
            None => canon_parent,
        }
    };
    if !canonical.starts_with(&root) {
        return Err(err_builtin(
            CausaError::RutaNoPermitida {
                ruta: canonical.display().to_string(),
            },
            linea,
            columna,
        ));
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lexer, Parser};

    fn eval(fuente: &str) -> Result<Valor, ErrorEjecucion> {
        let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
        let e = Parser::nuevo(tokens)
            .parsear_expresion()
            .expect("sintaxis válida en test");
        evaluar(&e)
    }

    // ---- PN-INT: enteros ----

    #[test]
    fn pn_int_suma_basica() {
        assert_eq!(eval("10 + 5"), Ok(Valor::Entero(15)));
    }

    #[test]
    fn pn_int_division_trunca_hacia_cero() {
        assert_eq!(eval("7 / 2"), Ok(Valor::Entero(3)));
        assert_eq!(eval("-7 / 2"), Ok(Valor::Entero(-3)));
    }

    #[test]
    fn pn_int_desbordamiento_suma_es_error() {
        // PN-INT-OVF-1: MAX + 1 → error, no MIN.
        let r = eval("9223372036854775807 + 1");
        assert!(r.is_err());
        assert!(r.unwrap_err().mensaje.contains("desbordamiento"));
    }

    #[test]
    fn pn_int_desbordamiento_resta_y_mul() {
        assert!(eval("-9223372036854775808 - 1").is_err());
        assert!(eval("9223372036854775807 * 2").is_err());
    }

    #[test]
    fn pn_int_div_min_menos_uno_es_error() {
        // PN-INT-OVF-4.
        let r = eval("-9223372036854775808 / -1");
        assert!(r.is_err());
        assert!(r.unwrap_err().mensaje.contains("desbordamiento"));
    }

    #[test]
    fn pn_int_min_mod_menos_uno_es_cero() {
        // PN-INT-OVF-5: el resto cabe.
        assert_eq!(eval("-9223372036854775808 % -1"), Ok(Valor::Entero(0)));
    }

    #[test]
    fn pn_div_cero_entero_es_error_distinto() {
        // PN-INT-OVF-6: motivo "división entre cero", no desbordamiento.
        let r = eval("10 / 0");
        assert!(r.unwrap_err().mensaje.contains("cero"));
        assert!(eval("10 % 0").unwrap_err().mensaje.contains("cero"));
    }

    // ---- PN-MOD: convenio truncado ----

    #[test]
    fn pn_mod_signo_del_dividendo() {
        assert_eq!(eval("-7 % 3"), Ok(Valor::Entero(-1)));
        assert_eq!(eval("7 % -3"), Ok(Valor::Entero(1)));
        assert_eq!(eval("-7 % -3"), Ok(Valor::Entero(-1)));
        assert_eq!(eval("6 % 3"), Ok(Valor::Entero(0)));
    }

    #[test]
    fn pn_mod_identidad_division() {
        // a == (a/b)*b + a%b
        for (a, b) in [(7, 3), (-7, 3), (7, -3), (-7, -3)] {
            let suma = eval(&format!("({a} / {b}) * {b} + {a} % {b}")).unwrap();
            assert_eq!(suma, Valor::Entero(a), "identidad para {a},{b}");
        }
    }

    // ---- PN-DEC: decimal exacto ----

    #[test]
    fn pn_dec_cero_unos_mas_dos_es_tres() {
        // 0.1 + 0.2 == 0.3 exactamente (nunca 0.30000000000000004).
        assert_eq!(eval("0.1 + 0.2"), Ok(Valor::Decimal(300_000_000)));
    }

    #[test]
    fn pn_dec_tres_unos() {
        // 0.10 + 0.10 + 0.10 == 0.30 (escala preservada).
        let r = eval("0.10 + 0.10 + 0.10").unwrap();
        assert_eq!(r.to_string(), "0.3");
        assert_eq!(r, Valor::Decimal(300_000_000));
    }

    #[test]
    fn pn_dec_division_con_redondeo() {
        // 10.0 / 4.0 = 2.5; 1.0 / 3.0 = 0.333333333 (half-even, escala 9).
        assert_eq!(eval("10.0 / 4.0").unwrap().to_string(), "2.5");
        assert_eq!(eval("1.0 / 3.0").unwrap().to_string(), "0.333333333");
    }

    #[test]
    fn pn_dec_division_por_cero_es_error() {
        assert!(eval("1.0 / 0.0").unwrap_err().mensaje.contains("cero"));
    }

    #[test]
    fn pn_mix_entero_mas_decimal_es_decimal() {
        // Conversión implícita entero→decimal (sin pérdida).
        assert_eq!(eval("2 + 3.5"), Ok(Valor::Decimal(5_500_000_000)));
    }

    #[test]
    fn pn_mix_entero_mas_real_es_real() {
        // Conversión implícita entero→real.
        assert_eq!(eval("2 + 3.5e0"), Ok(Valor::Real(5.5)));
    }

    #[test]
    fn pn_mix_decimal_y_real_requiere_conversion_explicita() {
        // Combinar decimal y real en un operador es error de tipos (decisión 16).
        let r = eval("1.5 + 2.0e0");
        assert!(r.unwrap_err().mensaje.contains("conversión"));
    }

    // ---- PN-REAL ----

    #[test]
    fn pn_real_division_por_cero_es_error() {
        // D3 aprobado: no se genera inf/NaN desde el lenguaje.
        assert!(eval("1.0e0 / 0.0e0").unwrap_err().mensaje.contains("cero"));
        assert!(eval("1.0e0 / 0").unwrap_err().mensaje.contains("cero"));
    }

    #[test]
    fn pn_real_operacion_normal() {
        assert_eq!(eval("1.5e0 * 2.0e0"), Ok(Valor::Real(3.0)));
    }

    // ---- precedencia y contexto (regresión) ----

    #[test]
    fn precedencia_y_parentesis() {
        assert_eq!(eval("2 + 3 * 4"), Ok(Valor::Entero(14)));
        assert_eq!(eval("(2 + 3) * 4"), Ok(Valor::Entero(20)));
        assert_eq!(eval("10 - 3 - 2"), Ok(Valor::Entero(5)));
    }

    #[test]
    fn continuacion_tras_operador() {
        assert_eq!(eval("10 +\n    5"), Ok(Valor::Entero(15)));
    }

    #[test]
    fn modulo_misma_precedencia_que_multiplicacion() {
        assert_eq!(eval("7 + 6 % 4"), Ok(Valor::Entero(9)));
    }

    // ---- entorno de variables (ADR-0006) ----

    fn programa(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
        let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
        let ss = Parser::nuevo(tokens)
            .parsear_programa()
            .expect("sintaxis válida en test");
        Entorno::nueva().ejecutar_programa(&ss)
    }

    fn causa_programa(fuente: &str) -> CausaError {
        *programa(fuente)
            .expect_err("se esperaba error de ejecución")
            .causa
    }

    #[test]
    fn declarar_usar_asignar_y_volver_a_usar() {
        let r = programa("sea x = 5\nx + 1").unwrap();
        assert_eq!(r, Some(Valor::Entero(6)));
        // El valor modificado se observa en la última expresión.
        let r = programa("sea x = 5\nx = x * 2\nx + 1").unwrap();
        assert_eq!(r, Some(Valor::Entero(11)));
    }

    #[test]
    fn declaracion_duplicada_es_error_tipeado() {
        assert_eq!(
            causa_programa("sea x = 1\nsea x = 2"),
            CausaError::DeclaracionDuplicada("x".into())
        );
    }

    #[test]
    fn referencia_a_variable_no_definida_es_error_tipeado() {
        assert_eq!(
            causa_programa("zeta + 1"),
            CausaError::VariableNoDefinida("zeta".into())
        );
    }

    #[test]
    fn asignacion_sin_declarar_no_crea_variable() {
        // Restricción del incremento: no hay declaración implícita.
        assert_eq!(
            causa_programa("x = 5"),
            CausaError::VariableNoDefinida("x".into())
        );
        // Y la variable sigue sin existir después del intento fallido.
        assert_eq!(
            causa_programa("x = 5\nx"),
            CausaError::VariableNoDefinida("x".into())
        );
    }

    #[test]
    fn asignacion_incompatible_es_error_tipeado() {
        // variable entero ← decimal: no hay conversión implícita en ese sentido.
        assert_eq!(
            causa_programa("sea x = 5\nx = 2.5"),
            CausaError::AsignacionIncompatible {
                esperado: "entero".to_string(),
                obtenido: "decimal".to_string()
            }
        );
        // variable decimal ← real.
        assert_eq!(
            causa_programa("sea x = 1.5\nx = 2.0e0"),
            CausaError::AsignacionIncompatible {
                esperado: "decimal".to_string(),
                obtenido: "real".to_string()
            }
        );
        // variable entero ← real.
        assert_eq!(
            causa_programa("sea x = 5\nx = 2.0e0"),
            CausaError::AsignacionIncompatible {
                esperado: "entero".to_string(),
                obtenido: "real".to_string()
            }
        );
    }

    #[test]
    fn asignacion_admite_conversiones_implicitas_aprobadas() {
        // entero → decimal (ADR-0005 §1.4): la variable decimal acepta un entero.
        let r = programa("sea x = 1.5\nx = 2\nx").unwrap();
        assert_eq!(r, Some(Valor::Decimal(2_000_000_000)));
        // entero → real.
        let r = programa("sea x = 2.5e0\nx = 3\nx").unwrap();
        assert_eq!(r, Some(Valor::Real(3.0)));
    }

    #[test]
    fn errores_de_variable_llevan_ubicacion() {
        let e = programa("sea x = 1\nx + zeta").expect_err("zeta no existe");
        // 'zeta' ocupa la columna 5 en "x + zeta".
        assert_eq!((e.linea, e.columna), (2, 5));
        let e = programa("sea x = 1\nsea x = 2").expect_err("duplicada");
        assert_eq!((e.linea, e.columna), (2, 5));
        let e = programa("sea x = 1\nx = 2.5").expect_err("incompatible");
        assert_eq!((e.linea, e.columna), (2, 1));
    }

    #[test]
    fn errores_aritmeticos_siguen_funcionando_en_programas() {
        assert_eq!(
            causa_programa("sea x = 10\nx / 0"),
            CausaError::Aritmetica(MotivoAritmetico::DivisionPorCero)
        );
        assert_eq!(
            causa_programa("sea x = 9223372036854775807\nx + 1"),
            CausaError::Aritmetica(MotivoAritmetico::DesbordamientoEntero)
        );
        // La ubicación del error aritmético apunta a la variable izquierda.
        let e = programa("sea x = 10\nx / 0").expect_err("división por cero");
        assert_eq!((e.linea, e.columna), (2, 1));
    }

    #[test]
    fn programa_sin_sentencia_de_expresion_no_produce_valor() {
        assert_eq!(programa("sea x = 5\nx = x + 1").unwrap(), None);
    }

    #[test]
    fn tipo_de_variable_fija_en_declaracion() {
        // La declaración fija el tipo: un entero guardado en variable entero no admite decimales.
        assert_eq!(
            causa_programa("sea x = 5\nx = 7.5"),
            CausaError::AsignacionIncompatible {
                esperado: "entero".to_string(),
                obtenido: "decimal".to_string()
            }
        );
        // Pero la referencia devuelve el valor con su tipo original.
        assert_eq!(
            programa("sea x = 7.5\nx").unwrap(),
            Some(Valor::Decimal(7_500_000_000))
        );
    }

    // ---- control de flujo y lógicos (ADR-0007) ----

    #[test]
    fn las_comparaciones_devuelven_logico() {
        assert_eq!(eval("1 == 1"), Ok(Valor::Logico(true)));
        assert_eq!(eval("1 == 2"), Ok(Valor::Logico(false)));
        assert_eq!(eval("1 != 2"), Ok(Valor::Logico(true)));
        assert_eq!(eval("1 < 2"), Ok(Valor::Logico(true)));
        assert_eq!(eval("2 <= 2"), Ok(Valor::Logico(true)));
        assert_eq!(eval("1 > 2"), Ok(Valor::Logico(false)));
        assert_eq!(eval("2 >= 2"), Ok(Valor::Logico(true)));
    }

    #[test]
    fn los_operadores_logicos_devuelven_logico() {
        assert_eq!(eval("verdadero y verdadero"), Ok(Valor::Logico(true)));
        assert_eq!(eval("verdadero y falso"), Ok(Valor::Logico(false)));
        assert_eq!(eval("falso o verdadero"), Ok(Valor::Logico(true)));
        assert_eq!(eval("falso o falso"), Ok(Valor::Logico(false)));
        assert_eq!(eval("no verdadero"), Ok(Valor::Logico(false)));
        assert_eq!(eval("no falso"), Ok(Valor::Logico(true)));
    }

    #[test]
    fn la_aritmetica_no_acepta_logicos() {
        assert_eq!(
            causa_programa("1 + verdadero"),
            CausaError::Aritmetica(MotivoAritmetico::TiposIncompatibles)
        );
        assert_eq!(
            causa_programa("verdadero + 1"),
            CausaError::Aritmetica(MotivoAritmetico::TiposIncompatibles)
        );
    }

    #[test]
    fn el_cortocircuito_evita_evaluar_el_operando_derecho() {
        // El lado derecho no se evalúa: no hay división por cero.
        assert_eq!(eval("falso y (10 / 0)"), Ok(Valor::Logico(false)));
        assert_eq!(eval("verdadero o (10 / 0)"), Ok(Valor::Logico(true)));
        // Pero si el derecho sí se evalúa, el error aparece.
        assert_eq!(
            causa_programa("verdadero y (10 / 0)"),
            CausaError::Aritmetica(MotivoAritmetico::DivisionPorCero)
        );
    }

    #[test]
    fn la_condicion_debe_ser_logica() {
        assert_eq!(
            causa_programa("si 1 {\n  2\n}"),
            CausaError::CondicionNoLogica { obtenido: "entero" }
        );
        assert_eq!(
            causa_programa("mientras 1 {\n  2\n}"),
            CausaError::CondicionNoLogica { obtenido: "entero" }
        );
    }

    #[test]
    fn el_bloque_no_produce_valor_hacia_fuera() {
        // El valor del programa es el de la última expresión de nivel superior.
        assert_eq!(
            programa("1 + 1\n{\n  5\n}").unwrap(),
            Some(Valor::Entero(2))
        );
    }

    #[test]
    fn el_bloque_usa_un_ambito_propio() {
        // La variable declarada en el bloque no sobrevive al bloque.
        assert_eq!(
            causa_programa("{\n  sea interna = 1\n}\ninterna"),
            CausaError::VariableNoDefinida("interna".into())
        );
        // El sombreado en niveles distintos está permitido.
        assert_eq!(
            programa("sea x = 1\n{\n  sea x = 2\n}\nx").unwrap(),
            Some(Valor::Entero(1))
        );
    }

    #[test]
    fn el_ciclo_se_reevalua_por_iteracion() {
        let r = programa("sea i = 0\nsea n = 0\nmientras i < 4 {\n  i = i + 1\n  n = n + 1\n}\nn")
            .unwrap();
        assert_eq!(r, Some(Valor::Entero(4)));
    }

    #[test]
    fn el_ciclo_con_cero_iteraciones_no_entra_al_cuerpo() {
        let r = programa("sea n = 0\nmientras falso {\n  n = 99\n}\nn").unwrap();
        assert_eq!(r, Some(Valor::Entero(0)));
    }

    #[test]
    fn el_limite_de_iteraciones_protege_de_bucles_infinitos() {
        // Condición siempre verdadera: el guard de iteraciones corta el ciclo.
        assert_eq!(
            causa_programa("mientras verdadero {\n  1\n}"),
            CausaError::LimiteIteraciones {
                limite: LIMITE_ITERACIONES
            }
        );
    }

    #[test]
    fn los_errores_dentro_de_un_bloque_conservan_ubicacion() {
        let e = programa("si verdadero {\n  10 / 0\n}").expect_err("división por cero");
        assert_eq!(
            *e.causa,
            CausaError::Aritmetica(MotivoAritmetico::DivisionPorCero)
        );
        assert_eq!((e.linea, e.columna), (2, 3));
    }

    // ---- INCREMENTO 5: Funciones y retorno (ADR-0008) ----

    #[test]
    fn funcion_error_tipo_retorno_incompatible() {
        // Primera llamada fija el tipo de retorno; segunda con tipo distinto falla
        let e = programa(
            r#"
            funcion id(x) {
                si x == 0 {
                    retornar 1
                }
                retornar 3.14
            }
            id(0)
            id(1)
        "#,
        )
        .expect_err("tipo de retorno incompatible");
        assert_eq!(
            *e.causa,
            CausaError::TipoDeRetornoIncompatible {
                esperado: "entero",
                obtenido: "decimal",
            }
        );
    }

    #[test]
    fn funcion_error_firma_invalida_pocos_args() {
        let e = programa(
            r#"
            funcion suma(a, b) {
                retornar a + b
            }
            suma(1)
        "#,
        )
        .expect_err("firma inválida");
        assert_eq!(
            *e.causa,
            CausaError::FirmaInvalida {
                funcion: "suma".into(),
                esperado: 2,
                obtenido: 1,
            }
        );
    }

    #[test]
    fn funcion_error_firma_invalida_muchos_args() {
        let e = programa(
            r#"
            funcion hola() {
                retornar 1
            }
            hola(1, 2)
        "#,
        )
        .expect_err("firma inválida");
        assert_eq!(
            *e.causa,
            CausaError::FirmaInvalida {
                funcion: "hola".into(),
                esperado: 0,
                obtenido: 2,
            }
        );
    }

    #[test]
    fn funcion_retorno_sin_valor_en_funcion_que_espera_valor() {
        let e = programa(
            r#"
            funcion f() {
                retornar
            }
            f()
        "#,
        )
        .expect_err("retorno sin valor");
        // ADR-0008: se permite retornar sin valor, pero el tipo es "vacío"
        // En este caso, el evaluador devuelve error por no tener valor para usar
        assert_eq!(
            *e.causa,
            CausaError::RetornoAusente {
                funcion: "f".into()
            }
        );
    }

    #[test]
    fn funcion_duplicada_es_error() {
        assert_eq!(
            causa_programa("funcion f() { retornar 1 }\nfuncion f() { retornar 2 }"),
            CausaError::DeclaracionDuplicada("f".into())
        );
    }
}
