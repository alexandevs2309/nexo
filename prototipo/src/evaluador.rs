//! Evaluador del prototipo de BASILISCO.
//!
//! Semántica numérica normativa (ADR-0005, aprobado):
//! - Tres tipos: `entero` (i64), `decimal` (DECIMAL(28,9)), `real` (f64).
//! - Desbordamiento entero = error; división por cero = error en los tres tipos.
//! - `%` de enteros = resto truncado, signo del dividendo; `i64::MIN % -1` = 0.
//! - Decimal: redondeo half-even, exceso de precisión redondea, exceso de rango = error.
//! - Conversiones: solo implícitas sin pérdida (`entero→decimal`, `entero→real`).

use std::collections::HashMap;

use crate::parser::{Expresion, Sentencia, TipoAnotacion};
use crate::valor::{FACTOR_ESCALA, MotivoAritmetico, Valor};

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
}

/// Límite de iteraciones por ciclo `mientras` (protección frente a bucles infinitos).
pub const LIMITE_ITERACIONES: usize = 100_000;

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
    "escribir",
    "abrir",
    "cerrar",
];

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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorEjecucion {
    pub mensaje: String,
    pub causa: Box<CausaError>,
    pub linea: usize,
    pub columna: usize,
}

impl std::fmt::Display for ErrorEjecucion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error de ejecución [{}:{}]: {}",
            self.linea, self.columna, self.mensaje
        )
    }
}

fn error(causa: CausaError, linea: usize, columna: usize) -> ErrorEjecucion {
    ErrorEjecucion {
        mensaje: causa.to_string(),
        causa: Box::new(causa),
        linea,
        columna,
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
#[derive(Debug, Default)]
pub struct Entorno {
    ambitos: Vec<HashMap<String, VariableInfo>>,
    funciones: HashMap<String, FuncionDef>,
}

impl Entorno {
    pub fn nueva() -> Self {
        Entorno {
            ambitos: vec![HashMap::new()],
            funciones: HashMap::new(),
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
        let r = self.ejecutar_funcion_sentencias(sentencias, nombre_funcion);
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
                } => {
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
                            CausaError::AsignacionIncompatible {
                                esperado: "variable inmutable (dato/const)".into(),
                                obtenido: "intento de reasignación".to_string(),
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
                Sentencia::Funcion { .. } => {}
                Sentencia::Mostrar { valor, .. } => {
                    let v = evaluar_con(&valor, self)?;
                    println!("{v}");
                }
                Sentencia::MostrarSinSalto { valor, .. } => {
                    let v = evaluar_con(&valor, self)?;
                    print!("{v}");
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
                            }
                            // Sin variable de captura, se ejecuta el bloque de rescate sin contexto.
                            let r = self.ejecutar_bloque_funcion(&cuerpo_rescatar, nombre_funcion);
                            if let Some(valor) = r? {
                                return Ok(Some(valor));
                            }
                        }
                    }
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
                } => {
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
                            CausaError::AsignacionIncompatible {
                                esperado: "variable inmutable (dato/const)".into(),
                                obtenido: "intento de reasignación".to_string(),
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
        Expresion::Variable {
            nombre,
            linea,
            columna,
        } => env.buscar(nombre).ok_or_else(|| {
            error(
                CausaError::VariableNoDefinida(nombre.clone()),
                *linea,
                *columna,
            )
        }),
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
            let fdef = env.funciones.get(nombre).cloned().ok_or_else(|| {
                error(
                    CausaError::VariableNoDefinida(nombre.clone()),
                    *linea,
                    *columna,
                )
            })?;
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

            let mut valores = Vec::new();
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
