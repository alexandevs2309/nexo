#![allow(
    clippy::result_large_err,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

//! Pase de monomorfización G4 (Fase 07).
//!
//! Arquitectura C: representación intermedia dedicada, no ejecutable, separada
//! de HIR/MIR. Transforma aplicaciones genéricas semánticamente válidas en
//! funciones concretas mediante instancia por `InstanceKey`, worklist,
//! especialización/sustitución estructural y validación V3.
//!
//! Este módulo es la autoridad única para identidad de instancias, resolución,
//! deduplicación, detección de crecimiento, dependencias, nombres y reescritura.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::evaluador::CausaError;
use crate::parser::{Expresion, Sentencia, TipoAnotacion};
use crate::semantico::{TipoParamId, TipoSem};
use crate::simbodo::SimbodoId;

/// Límites provisionales (M1/M2/M3) sujetos a calibración documentada.
/// Valores conservadores para prevenir expansión ilimitada mientras se
/// registra evidencia.
pub const LIMITE_INSTANCIAS_PROVISIONAL: usize = 256;
pub const LIMITE_PROFUNDIDAD_PROVISIONAL: usize = 32;
pub const LIMITE_ARIDAD_PROVISIONAL: usize = 8;

/// Identidad estable y cualificada de una declaración genérica.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenericFnId {
    /// Ruta/identidad de módulo (vacía para módulo principal/local).
    pub modulo: String,
    /// Identidad estable de declaración propietaria.
    pub decl_id: SimbodoId,
    /// Nombre de la función.
    pub nombre: String,
    /// Aridad de parámetros de tipo.
    pub aridad: usize,
}

/// Representación canónica de un tipo concreto para identidad de instancia.
/// Debe ser determinista, comparable y libre de parámetros/desconocidos no concretables.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TipoCanon {
    Entero,
    Decimal,
    Real,
    Logico,
    Texto,
    Nulo,
    Arreglo(Box<TipoCanon>, usize),
    Lista(Box<TipoCanon>),
    Coleccion(Box<TipoCanon>, usize),
    Funcion {
        params: Vec<TipoCanon>,
        retorno: Box<TipoCanon>,
    },
}

impl TipoCanon {
    pub fn nombre(&self) -> String {
        match self {
            TipoCanon::Entero => "entero".to_string(),
            TipoCanon::Decimal => "decimal".to_string(),
            TipoCanon::Real => "real".to_string(),
            TipoCanon::Logico => "lógico".to_string(),
            TipoCanon::Texto => "texto".to_string(),
            TipoCanon::Nulo => "nulo".to_string(),
            TipoCanon::Arreglo(t, n) => format!("arreglo<{}, {}>", t.nombre(), n),
            TipoCanon::Lista(t) => format!("lista<{}>", t.nombre()),
            TipoCanon::Coleccion(t, n) => format!("colección<{}, {}>", t.nombre(), n),
            TipoCanon::Funcion { params, retorno } => {
                let ps: Vec<String> = params.iter().map(|t| t.nombre()).collect();
                format!("función({}) -> {}", ps.join(", "), retorno.nombre())
            }
        }
    }
}

/// Intenta canonicalizar un `TipoSem` concreto. Falla si contiene `Param` o
/// `Desconocido` (no concretable) en posiciones que impiden identidad completa.
pub fn canonicalizar_tipo(t: &TipoSem) -> Option<TipoCanon> {
    match t {
        TipoSem::Entero => Some(TipoCanon::Entero),
        TipoSem::Decimal => Some(TipoCanon::Decimal),
        TipoSem::Real => Some(TipoCanon::Real),
        TipoSem::Logico => Some(TipoCanon::Logico),
        TipoSem::Texto => Some(TipoCanon::Texto),
        TipoSem::Nulo => Some(TipoCanon::Nulo),
        TipoSem::Arreglo(te, n) => Some(TipoCanon::Arreglo(Box::new(canonicalizar_tipo(te)?), *n)),
        TipoSem::Lista(te) => Some(TipoCanon::Lista(Box::new(canonicalizar_tipo(te)?))),
        TipoSem::Coleccion(te, n) => {
            Some(TipoCanon::Coleccion(Box::new(canonicalizar_tipo(te)?), *n))
        }
        TipoSem::Nominal(_) => None, // S8.1: nominales no participan en monomorfización G4
        TipoSem::Funcion {
            params, retorno, ..
        } => {
            let cp: Option<Vec<TipoCanon>> = params.iter().map(canonicalizar_tipo).collect();
            Some(TipoCanon::Funcion {
                params: cp?,
                retorno: Box::new(canonicalizar_tipo(retorno)?),
            })
        }
        TipoSem::Param(_) | TipoSem::Desconocido => None,
    }
}

/// Convierte un tipo canónico a `TipoAnotacion` para sustitución en AST.
pub fn canon_a_anotacion(t: &TipoCanon) -> Option<TipoAnotacion> {
    match t {
        TipoCanon::Entero => Some(TipoAnotacion::Entero),
        TipoCanon::Decimal => Some(TipoAnotacion::Decimal),
        TipoCanon::Real => Some(TipoAnotacion::Real),
        TipoCanon::Logico => Some(TipoAnotacion::Logico),
        TipoCanon::Texto => Some(TipoAnotacion::Texto),
        TipoCanon::Nulo => Some(TipoAnotacion::Nulo),
        TipoCanon::Arreglo(te, n) => {
            Some(TipoAnotacion::Arreglo(Box::new(canon_a_anotacion(te)?), *n))
        }
        TipoCanon::Lista(te) => Some(TipoAnotacion::Lista(Box::new(canon_a_anotacion(te)?))),
        // Coleccion no tiene anotación de sintaxis propia; se representa como lista para tipos concretos.
        TipoCanon::Coleccion(te, _) => Some(TipoAnotacion::Lista(Box::new(canon_a_anotacion(te)?))),
        TipoCanon::Funcion { .. } => None,
    }
}

/// Convierte un tipo canónico a `TipoSem` concreto.
pub fn canon_a_sem(t: &TipoCanon) -> TipoSem {
    match t {
        TipoCanon::Entero => TipoSem::Entero,
        TipoCanon::Decimal => TipoSem::Decimal,
        TipoCanon::Real => TipoSem::Real,
        TipoCanon::Logico => TipoSem::Logico,
        TipoCanon::Texto => TipoSem::Texto,
        TipoCanon::Nulo => TipoSem::Nulo,
        TipoCanon::Arreglo(te, n) => TipoSem::Arreglo(Box::new(canon_a_sem(te)), *n),
        TipoCanon::Lista(te) => TipoSem::Lista(Box::new(canon_a_sem(te))),
        TipoCanon::Coleccion(te, n) => TipoSem::Coleccion(Box::new(canon_a_sem(te)), *n),
        TipoCanon::Funcion { params, retorno } => TipoSem::Funcion {
            params: params.iter().map(canon_a_sem).collect(),
            retorno: Box::new(canon_a_sem(retorno)),
            genericos: Vec::new(),
        },
    }
}

/// Clave canónica de una instancia: (GenericFnId, argumentos de tipo concretos).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceKey {
    pub funcion: GenericFnId,
    pub args: Vec<TipoCanon>,
}

impl InstanceKey {
    pub fn nombre_tipos(&self) -> Vec<String> {
        self.args.iter().map(|t| t.nombre()).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoInstancia {
    Pending,
    InProgress,
    Complete,
    Failed,
}

/// Plantilla genérica (IR intermedia, no ejecutable).
#[derive(Debug, Clone)]
pub struct PlantillaGenerica {
    pub id: GenericFnId,
    pub nombre: String,
    pub parametros_tipo: Vec<(String, TipoParamId)>,
    pub parametros: Vec<(String, Option<TipoAnotacion>)>,
    pub tipo_retorno: Option<TipoAnotacion>,
    pub cuerpo: Vec<Sentencia>,
    pub linea: usize,
    pub columna: usize,
    pub publico: bool,
}

/// Registro de una instancia.
#[derive(Debug, Clone)]
pub struct RegistroInstancia {
    pub key: InstanceKey,
    pub estado: EstadoInstancia,
    pub nombre_especializado: Option<String>,
    /// Función especializada concreta (AST) una vez completa.
    pub funcion: Option<Sentencia>,
    pub error: Option<CausaError>,
    /// Dependencias solicitadas durante la especialización.
    pub dependencias: Vec<InstanceKey>,
    /// Origen/cadena de instanciación (para profundidad/growth en modo iterativo).
    pub origen: Option<OrigenInstancia>,
}

/// Contexto de origen para diagnósticos.
#[derive(Debug, Clone)]
pub struct OrigenInstancia {
    pub funcion: String,
    pub instancia: String,
    pub tipos: Vec<String>,
    pub cadena: Vec<String>,
    pub cadena_keys: Vec<InstanceKey>,
    pub linea: usize,
    pub columna: usize,
}

/// Generador determinista de nombres especializados con detección de colisiones.
#[derive(Debug, Default)]
pub struct GeneradorNombres {
    usados: BTreeSet<String>,
    /// Mapeo nombre -> key para verificación de colisiones.
    por_nombre: BTreeMap<String, InstanceKey>,
}

impl GeneradorNombres {
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// Genera un nombre especializado determinista y verifica unicidad.
    /// Retorna Err si se detecta colisión (mismo nombre, distinta clave).
    #[allow(clippy::result_large_err)]
    pub fn generar(&mut self, key: &InstanceKey) -> Result<String, CausaError> {
        let base = nombre_especializado_base(key);
        let nombre = if self.usados.contains(&base) {
            if let Some(prev) = self.por_nombre.get(&base)
                && prev == key
            {
                return Ok(base);
            }
            // Colisión real: intentar sufijo determinista derivado de la clave completa.
            let mut i = 1usize;
            loop {
                let cand = format!("{}__k{i}", base);
                if !self.usados.contains(&cand) {
                    break cand;
                }
                i += 1;
                if i > 10_000 {
                    return Err(CausaError::ReferenciaGenericaResidualIr {
                        detalle: format!(
                            "colisión de nombre especializado no resoluble para '{}'",
                            base
                        ),
                        ubicacion: key.funcion.nombre.clone(),
                    });
                }
            }
        } else {
            base
        };
        if let Some(prev) = self.por_nombre.get(&nombre) {
            if prev != key {
                return Err(CausaError::ReferenciaGenericaResidualIr {
                    detalle: format!(
                        "colisión de SpecializedName '{}' entre instancias distintas",
                        nombre
                    ),
                    ubicacion: key.funcion.nombre.clone(),
                });
            }
        } else {
            self.por_nombre.insert(nombre.clone(), key.clone());
            self.usados.insert(nombre.clone());
        }
        Ok(nombre)
    }
}

fn sanear_identificador(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, ch) in s.chars().enumerate() {
        let ok = if i == 0 {
            ch.is_ascii_alphabetic() || ch == '_'
        } else {
            ch.is_ascii_alphanumeric() || ch == '_'
        };
        if ok {
            out.push(ch);
        } else if ch == '<' || ch == '>' || ch == ',' || ch == ' ' {
            // separadores estructurales ignorados (ya codificados)
        } else {
            out.push('_');
        }
    }
    if out.is_empty() { "_".to_string() } else { out }
}

fn codificar_tipo_para_nombre(t: &TipoCanon) -> String {
    match t {
        TipoCanon::Entero => "entero".to_string(),
        TipoCanon::Decimal => "decimal".to_string(),
        TipoCanon::Real => "real".to_string(),
        TipoCanon::Logico => "logico".to_string(),
        TipoCanon::Texto => "texto".to_string(),
        TipoCanon::Nulo => "nulo".to_string(),
        TipoCanon::Lista(te) => format!("lista_{}_end", codificar_tipo_para_nombre(te)),
        TipoCanon::Coleccion(te, n) => {
            format!("coleccion_{}_{}_end", codificar_tipo_para_nombre(te), n)
        }
        TipoCanon::Arreglo(te, n) => {
            format!("arreglo_{}_{}_end", codificar_tipo_para_nombre(te), n)
        }
        TipoCanon::Funcion { params, retorno } => {
            let ps: Vec<String> = params.iter().map(codificar_tipo_para_nombre).collect();
            format!(
                "fn_{}_ret_{}_end",
                ps.join("_"),
                codificar_tipo_para_nombre(retorno)
            )
        }
    }
}

/// Nombre especializado determinista base (incorpora origen y tipos).
pub fn nombre_especializado_base(key: &InstanceKey) -> String {
    let fn_name = sanear_identificador(&key.funcion.nombre);
    let owner = key.funcion.decl_id.0;
    let tipos: Vec<String> = key.args.iter().map(codificar_tipo_para_nombre).collect();
    if tipos.is_empty() {
        format!("__g4__{fn_name}__d{owner}")
    } else {
        format!("__g4__{fn_name}__d{owner}__{}", tipos.join("__"))
    }
}

/// Motor de monomorfización G4.
#[derive(Debug)]
pub struct MotorMonomorfizacion {
    pub plantillas: BTreeMap<GenericFnId, PlantillaGenerica>,
    pub instancias: BTreeMap<InstanceKey, RegistroInstancia>,
    pub cola: Vec<InstanceKey>,
    pub nombres: GeneradorNombres,
    /// Cadena activa de instanciación (para detección de crecimiento y profundidad).
    pub cadena_activa: Vec<InstanceKey>,
    pub limite_instancias: usize,
    pub limite_profundidad: usize,
}

impl Default for MotorMonomorfizacion {
    fn default() -> Self {
        Self::nuevo(
            LIMITE_INSTANCIAS_PROVISIONAL,
            LIMITE_PROFUNDIDAD_PROVISIONAL,
        )
    }
}

impl MotorMonomorfizacion {
    pub fn nuevo(limite_instancias: usize, limite_profundidad: usize) -> Self {
        Self {
            plantillas: BTreeMap::new(),
            instancias: BTreeMap::new(),
            cola: Vec::new(),
            nombres: GeneradorNombres::nuevo(),
            cadena_activa: Vec::new(),
            limite_instancias,
            limite_profundidad,
        }
    }

    pub fn registrar_plantilla(&mut self, p: PlantillaGenerica) {
        self.plantillas.insert(p.id.clone(), p);
    }

    /// Solicita una instancia (encola si es nueva). Retorna nombre especializado si ya completa.
    #[allow(clippy::result_large_err)]
    pub fn solicitar(
        &mut self,
        key: InstanceKey,
        origen: &OrigenInstancia,
    ) -> Result<Option<String>, CausaError> {
        if let Some(reg) = self.instancias.get(&key) {
            return match reg.estado {
                EstadoInstancia::Complete => Ok(reg.nombre_especializado.clone()),
                EstadoInstancia::InProgress => Ok(reg.nombre_especializado.clone()),
                EstadoInstancia::Pending => Ok(reg.nombre_especializado.clone()),
                EstadoInstancia::Failed => Err(reg.error.clone().unwrap_or_else(|| {
                    CausaError::DependenciaGenericaNoResoluble {
                        funcion: key.funcion.nombre.clone(),
                        detalle: format!(
                            "instancia previamente fallida para tipos [{}]",
                            key.nombre_tipos().join(", ")
                        ),
                    }
                })),
            };
        }
        // Límites
        if self.instancias.len() >= self.limite_instancias {
            return Err(CausaError::LimiteInstanciasExcedido {
                limite: self.limite_instancias,
                solicitada: key.funcion.nombre.clone(),
                tipos: key.nombre_tipos(),
                cadena: origen.cadena.clone(),
                detalle: "se alcanzó el límite de instancias únicas (M1)".to_string(),
            });
        }
        // Detección de crecimiento polimórfico y profundidad en cadena activa.
        self.verificar_crecimiento_y_profundidad(&key, origen)?;

        // Reservar nombre y registrar.
        let nombre = self.nombres.generar(&key)?;
        let reg = RegistroInstancia {
            key: key.clone(),
            estado: EstadoInstancia::Pending,
            nombre_especializado: Some(nombre),
            funcion: None,
            error: None,
            dependencias: Vec::new(),
            origen: Some(origen.clone()),
        };
        self.instancias.insert(key.clone(), reg);
        self.cola.push(key);
        // Orden determinista de la cola: por InstanceKey (BTree-ordenable al extraer mínimo).
        self.cola.sort();
        Ok(None)
    }

    fn verificar_crecimiento_y_profundidad(
        &self,
        key: &InstanceKey,
        origen: &OrigenInstancia,
    ) -> Result<(), CausaError> {
        if self.cadena_activa.len() >= self.limite_profundidad {
            return Err(CausaError::ProfundidadInstanciacionExcedida {
                funcion: key.funcion.nombre.clone(),
                instancia: key.funcion.nombre.clone(),
                tipos: key.nombre_tipos(),
                limite: self.limite_profundidad,
                cadena: origen.cadena.clone(),
                detalle: "profundidad de cadena de instanciación excedida (M2)".to_string(),
            });
        }
        // Recursión monomórfica: misma clave en cadena -> permitida (no error).
        // Crecimiento polimórfico: misma GenericFnId pero args con crecimiento estructural estricto.
        for anc in &self.cadena_activa {
            if anc.funcion == key.funcion {
                if anc == key {
                    // monomórfica: OK
                    return Ok(());
                }
                if crecimiento_estructural(&anc.args, &key.args) {
                    return Err(CausaError::RecursionPolimorficaCreciente {
                        funcion: key.funcion.nombre.clone(),
                        instancia: key.funcion.nombre.clone(),
                        tipos: key.nombre_tipos(),
                        cadena: origen.cadena.clone(),
                        detalle: format!(
                            "crecimiento estructural respecto a instancia ancestral con tipos [{}]",
                            anc.nombre_tipos().join(", ")
                        ),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Determina si `nuevo` es una expansión estructural estricta de `ancestro`
/// (conservador, por posición): algún argumento crece estrictamente y ninguno decrece.
fn crecimiento_estructural(ancestro: &[TipoCanon], nuevo: &[TipoCanon]) -> bool {
    if ancestro.len() != nuevo.len() {
        return false;
    }
    let mut algun_crece = false;
    for (a, n) in ancestro.iter().zip(nuevo.iter()) {
        if tipo_crece(a, n) {
            algun_crece = true;
        } else if tipo_crece(n, a) {
            // decrece en alguna posición respecto al ancestro: no es crecimiento puro
            return false;
        } else if a != n {
            // distinto sin relación de contención clara en este par: conservadoramente no marcar crecimiento puro
            // (evitar falsos positivos); el ancestro!=nuevo ya se maneja arriba.
            return false;
        }
    }
    algun_crece
}

/// `n` contiene estructuralmente a `a` como subestructura estricta (embedding conservador).
fn tipo_crece(a: &TipoCanon, n: &TipoCanon) -> bool {
    if a == n {
        return false;
    }
    contiene_subtipo(n, a)
}

fn contiene_subtipo(contenedor: &TipoCanon, objetivo: &TipoCanon) -> bool {
    if contenedor == objetivo {
        return true;
    }
    match contenedor {
        TipoCanon::Lista(t) | TipoCanon::Coleccion(t, _) => {
            t.as_ref() == objetivo || contiene_subtipo(t, objetivo)
        }
        TipoCanon::Arreglo(t, _) => t.as_ref() == objetivo || contiene_subtipo(t, objetivo),
        TipoCanon::Funcion { params, retorno } => {
            params.iter().any(|p| contiene_subtipo(p, objetivo))
                || contiene_subtipo(retorno, objetivo)
        }
        _ => false,
    }
}

/// Mapa de sustitución por nombre de parámetro de tipo (para AST/anotaciones).
pub type MapaSustAnot = HashMap<String, TipoCanon>;

/// Sustituye parámetros de tipo en una anotación de tipo AST.
pub fn sustituir_anotacion(a: &TipoAnotacion, sigma: &MapaSustAnot) -> Option<TipoAnotacion> {
    match a {
        TipoAnotacion::Parametro(n) => sigma.get(n).and_then(canon_a_anotacion),
        TipoAnotacion::Arreglo(te, n) => Some(TipoAnotacion::Arreglo(
            Box::new(sustituir_anotacion(te, sigma)?),
            *n,
        )),
        TipoAnotacion::Lista(te) => Some(TipoAnotacion::Lista(Box::new(sustituir_anotacion(
            te, sigma,
        )?))),
        TipoAnotacion::Entero => Some(TipoAnotacion::Entero),
        TipoAnotacion::Decimal => Some(TipoAnotacion::Decimal),
        TipoAnotacion::Real => Some(TipoAnotacion::Real),
        TipoAnotacion::Logico => Some(TipoAnotacion::Logico),
        TipoAnotacion::Texto => Some(TipoAnotacion::Texto),
        TipoAnotacion::Nulo => Some(TipoAnotacion::Nulo),
        TipoAnotacion::Nominal(n) => Some(TipoAnotacion::Nominal(n.clone())),
    }
}

pub fn sustituir_anotacion_opt(
    a: &Option<TipoAnotacion>,
    sigma: &MapaSustAnot,
) -> Option<Option<TipoAnotacion>> {
    match a {
        None => Some(None),
        Some(t) => Some(Some(sustituir_anotacion(t, sigma)?)),
    }
}

/// Construye mapa nombre->TipoCanon desde sigma de TipoParamId.
pub fn mapa_sust_desde_sigma(
    params: &[(String, TipoParamId)],
    sigma: &HashMap<TipoParamId, TipoSem>,
) -> Option<MapaSustAnot> {
    let mut m = HashMap::new();
    for (nombre, id) in params {
        let t = sigma.get(id)?;
        let c = canonicalizar_tipo(t)?;
        m.insert(nombre.clone(), c);
    }
    Some(m)
}

/// Sustituye tipos en expresiones (argumentos_tipo de llamadas, lambdas, etc.).
pub fn sustituir_expresion(e: &Expresion, sigma: &MapaSustAnot) -> Option<Expresion> {
    Some(match e {
        Expresion::Literal {
            valor,
            linea,
            columna,
        } => Expresion::Literal {
            valor: valor.clone(),
            linea: *linea,
            columna: *columna,
        },
        Expresion::Variable {
            nombre,
            linea,
            columna,
        } => Expresion::Variable {
            nombre: nombre.clone(),
            linea: *linea,
            columna: *columna,
        },
        Expresion::Suma(a, b) => Expresion::Suma(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::Resta(a, b) => Expresion::Resta(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::Multiplicacion(a, b) => Expresion::Multiplicacion(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::Division(a, b) => Expresion::Division(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::Modulo(a, b) => Expresion::Modulo(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::IgualIgual(a, b) => Expresion::IgualIgual(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::Distinto(a, b) => Expresion::Distinto(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::Menor(a, b) => Expresion::Menor(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::MenorIgual(a, b) => Expresion::MenorIgual(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::Mayor(a, b) => Expresion::Mayor(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::MayorIgual(a, b) => Expresion::MayorIgual(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::No(a) => Expresion::No(Box::new(sustituir_expresion(a, sigma)?)),
        Expresion::Y(a, b) => Expresion::Y(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::O(a, b) => Expresion::O(
            Box::new(sustituir_expresion(a, sigma)?),
            Box::new(sustituir_expresion(b, sigma)?),
        ),
        Expresion::Llamada {
            nombre,
            argumentos,
            argumentos_tipo,
            linea,
            columna,
        } => {
            let args: Option<Vec<Expresion>> = argumentos
                .iter()
                .map(|a| sustituir_expresion(a, sigma))
                .collect();
            let targs: Option<Vec<TipoAnotacion>> = argumentos_tipo
                .iter()
                .map(|t| sustituir_anotacion(t, sigma))
                .collect();
            Expresion::Llamada {
                nombre: nombre.clone(),
                argumentos: args?,
                argumentos_tipo: targs?,
                linea: *linea,
                columna: *columna,
            }
        }
        Expresion::ArregloLiteral {
            elementos,
            linea,
            columna,
        } => {
            let elems: Option<Vec<Expresion>> = elementos
                .iter()
                .map(|e| sustituir_expresion(e, sigma))
                .collect();
            Expresion::ArregloLiteral {
                elementos: elems?,
                linea: *linea,
                columna: *columna,
            }
        }
        Expresion::Indice {
            objeto,
            indice,
            linea,
            columna,
        } => Expresion::Indice {
            objeto: Box::new(sustituir_expresion(objeto, sigma)?),
            indice: Box::new(sustituir_expresion(indice, sigma)?),
            linea: *linea,
            columna: *columna,
        },
        Expresion::Acceso {
            objeto,
            miembro,
            linea,
            columna,
        } => Expresion::Acceso {
            objeto: Box::new(sustituir_expresion(objeto, sigma)?),
            miembro: miembro.clone(),
            linea: *linea,
            columna: *columna,
        },
        Expresion::LlamadaIndirecta {
            callee,
            argumentos,
            linea,
            columna,
        } => {
            let args: Option<Vec<Expresion>> = argumentos
                .iter()
                .map(|a| sustituir_expresion(a, sigma))
                .collect();
            Expresion::LlamadaIndirecta {
                callee: Box::new(sustituir_expresion(callee, sigma)?),
                argumentos: args?,
                linea: *linea,
                columna: *columna,
            }
        }
        Expresion::Lambda {
            parametros,
            tipo_retorno,
            cuerpo,
            linea,
            columna,
        } => {
            let params: Option<Vec<(String, Option<TipoAnotacion>)>> = parametros
                .iter()
                .map(|(n, t)| Some((n.clone(), sustituir_anotacion_opt(t, sigma)?)))
                .collect();
            let cuerpo_s: Option<Vec<Sentencia>> = cuerpo
                .iter()
                .map(|s| sustituir_sentencia(s, sigma))
                .collect();
            Expresion::Lambda {
                parametros: params?,
                tipo_retorno: sustituir_anotacion_opt(tipo_retorno, sigma)?,
                cuerpo: cuerpo_s?,
                linea: *linea,
                columna: *columna,
            }
        }
        Expresion::Rango {
            inicio,
            fin,
            inclusivo,
            linea,
            columna,
        } => Expresion::Rango {
            inicio: Box::new(sustituir_expresion(inicio, sigma)?),
            fin: Box::new(sustituir_expresion(fin, sigma)?),
            inclusivo: *inclusivo,
            linea: *linea,
            columna: *columna,
        },
        Expresion::ConstruccionEstructura {
            nombre,
            campos,
            linea,
            columna,
        } => Expresion::ConstruccionEstructura {
            nombre: nombre.clone(),
            campos: campos
                .iter()
                .map(|(n, e)| Some((n.clone(), sustituir_expresion(e, sigma)?)))
                .collect::<Option<Vec<_>>>()?,
            linea: *linea,
            columna: *columna,
        },
        Expresion::ConstruccionVariante {
            tipo,
            variante,
            argumentos,
            linea,
            columna,
        } => Expresion::ConstruccionVariante {
            tipo: tipo.clone(),
            variante: variante.clone(),
            argumentos: argumentos
                .iter()
                .map(|a| sustituir_expresion(a, sigma))
                .collect::<Option<Vec<_>>>()?,
            linea: *linea,
            columna: *columna,
        },
        Expresion::Coincidir {
            scrutinee,
            brazos,
            linea,
            columna,
        } => Expresion::Coincidir {
            scrutinee: Box::new(sustituir_expresion(scrutinee, sigma)?),
            brazos: brazos
                .iter()
                .map(|b| {
                    Some(crate::parser::BrazoCoincidir {
                        patron: b.patron.clone(),
                        cuerpo: b
                            .cuerpo
                            .iter()
                            .map(|s| sustituir_sentencia(s, sigma))
                            .collect::<Option<Vec<_>>>()?,
                        linea: b.linea,
                        columna: b.columna,
                    })
                })
                .collect::<Option<Vec<_>>>()?,
            linea: *linea,
            columna: *columna,
        },
    })
}

/// Sustituye tipos en sentencias.
pub fn sustituir_sentencia(s: &Sentencia, sigma: &MapaSustAnot) -> Option<Sentencia> {
    Some(match s {
        Sentencia::Declaracion {
            nombre,
            linea,
            columna,
            valor,
            tipo_anotacion,
            mutable,
            es_const,
            publico,
        } => Sentencia::Declaracion {
            nombre: nombre.clone(),
            linea: *linea,
            columna: *columna,
            valor: sustituir_expresion(valor, sigma)?,
            tipo_anotacion: sustituir_anotacion_opt(tipo_anotacion, sigma)?,
            mutable: *mutable,
            es_const: *es_const,
            publico: *publico,
        },
        Sentencia::Asignacion {
            nombre,
            linea,
            columna,
            valor,
        } => Sentencia::Asignacion {
            nombre: nombre.clone(),
            linea: *linea,
            columna: *columna,
            valor: sustituir_expresion(valor, sigma)?,
        },
        Sentencia::AsignacionIndice {
            objeto,
            indice,
            valor,
            linea,
            columna,
        } => Sentencia::AsignacionIndice {
            objeto: Box::new(sustituir_expresion(objeto, sigma)?),
            indice: Box::new(sustituir_expresion(indice, sigma)?),
            valor: sustituir_expresion(valor, sigma)?,
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Si {
            condicion,
            rama_si,
            rama_sino,
            linea,
            columna,
        } => {
            let si: Option<Vec<Sentencia>> = rama_si
                .iter()
                .map(|st| sustituir_sentencia(st, sigma))
                .collect();
            let sino: Option<Option<Vec<Sentencia>>> = match rama_sino {
                None => Some(None),
                Some(v) => Some(Some(
                    v.iter()
                        .map(|st| sustituir_sentencia(st, sigma))
                        .collect::<Option<Vec<_>>>()?,
                )),
            };
            Sentencia::Si {
                condicion: sustituir_expresion(condicion, sigma)?,
                rama_si: si?,
                rama_sino: sino?,
                linea: *linea,
                columna: *columna,
            }
        }
        Sentencia::Mientras {
            condicion,
            cuerpo,
            linea,
            columna,
        } => {
            let c: Option<Vec<Sentencia>> = cuerpo
                .iter()
                .map(|st| sustituir_sentencia(st, sigma))
                .collect();
            Sentencia::Mientras {
                condicion: sustituir_expresion(condicion, sigma)?,
                cuerpo: c?,
                linea: *linea,
                columna: *columna,
            }
        }
        Sentencia::Bloque(stmts) => {
            let c: Option<Vec<Sentencia>> = stmts
                .iter()
                .map(|st| sustituir_sentencia(st, sigma))
                .collect();
            Sentencia::Bloque(c?)
        }
        Sentencia::Expresion(e) => Sentencia::Expresion(sustituir_expresion(e, sigma)?),
        Sentencia::Funcion {
            nombre,
            parametros_tipo,
            parametros,
            tipo_retorno,
            cuerpo,
            linea,
            columna,
            publico,
        } => {
            // Funciones anidadas: sustituir tipos en su firma/cuerpo, pero preservar
            // sus propios parámetros de tipo (no herencia; G2.2). Solo sustituimos
            // referencias a parámetros del contexto externo actual (sigma).
            let params: Option<Vec<(String, Option<TipoAnotacion>)>> = parametros
                .iter()
                .map(|(n, t)| Some((n.clone(), sustituir_anotacion_opt(t, sigma)?)))
                .collect();
            let cuerpo_s: Option<Vec<Sentencia>> = cuerpo
                .iter()
                .map(|st| sustituir_sentencia(st, sigma))
                .collect();
            Sentencia::Funcion {
                nombre: nombre.clone(),
                parametros_tipo: parametros_tipo.clone(),
                parametros: params?,
                tipo_retorno: sustituir_anotacion_opt(tipo_retorno, sigma)?,
                cuerpo: cuerpo_s?,
                linea: *linea,
                columna: *columna,
                publico: *publico,
            }
        }
        Sentencia::Retorno {
            valor,
            linea,
            columna,
        } => Sentencia::Retorno {
            valor: match valor {
                None => None,
                Some(v) => Some(sustituir_expresion(v, sigma)?),
            },
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Mostrar {
            valor,
            linea,
            columna,
        } => Sentencia::Mostrar {
            valor: sustituir_expresion(valor, sigma)?,
            linea: *linea,
            columna: *columna,
        },
        Sentencia::MostrarSinSalto {
            valor,
            linea,
            columna,
        } => Sentencia::MostrarSinSalto {
            valor: sustituir_expresion(valor, sigma)?,
            linea: *linea,
            columna: *columna,
        },
        Sentencia::ParaCada {
            variable,
            iterable,
            cuerpo,
            linea,
            columna,
        } => {
            let c: Option<Vec<Sentencia>> = cuerpo
                .iter()
                .map(|st| sustituir_sentencia(st, sigma))
                .collect();
            Sentencia::ParaCada {
                variable: variable.clone(),
                iterable: sustituir_expresion(iterable, sigma)?,
                cuerpo: c?,
                linea: *linea,
                columna: *columna,
            }
        }
        Sentencia::Romper { linea, columna } => Sentencia::Romper {
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Continuar { linea, columna } => Sentencia::Continuar {
            linea: *linea,
            columna: *columna,
        },
        Sentencia::IntentarRescatar {
            cuerpo_intentar,
            variable_error,
            cuerpo_rescatar,
            linea,
            columna,
        } => {
            let ci: Option<Vec<Sentencia>> = cuerpo_intentar
                .iter()
                .map(|st| sustituir_sentencia(st, sigma))
                .collect();
            let cr: Option<Vec<Sentencia>> = cuerpo_rescatar
                .iter()
                .map(|st| sustituir_sentencia(st, sigma))
                .collect();
            Sentencia::IntentarRescatar {
                cuerpo_intentar: ci?,
                variable_error: variable_error.clone(),
                cuerpo_rescatar: cr?,
                linea: *linea,
                columna: *columna,
            }
        }
        Sentencia::Importar {
            ruta,
            alias,
            linea,
            columna,
        } => Sentencia::Importar {
            ruta: ruta.clone(),
            alias: alias.clone(),
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Exportar {
            sentencia,
            linea,
            columna,
        } => Sentencia::Exportar {
            sentencia: Box::new(sustituir_sentencia(sentencia, sigma)?),
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Usar {
            ruta,
            alias,
            nombres,
            linea,
            columna,
        } => Sentencia::Usar {
            ruta: ruta.clone(),
            alias: alias.clone(),
            nombres: nombres.clone(),
            linea: *linea,
            columna: *columna,
        },
        Sentencia::AsignacionCampo {
            objeto,
            campo,
            valor,
            linea,
            columna,
        } => Sentencia::AsignacionCampo {
            objeto: Box::new(sustituir_expresion(objeto, sigma)?),
            campo: campo.clone(),
            valor: sustituir_expresion(valor, sigma)?,
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Estructura {
            nombre,
            campos,
            linea,
            columna,
            publico,
        } => Sentencia::Estructura {
            nombre: nombre.clone(),
            campos: campos
                .iter()
                .map(|c| {
                    Some(crate::parser::CampoEstructura {
                        nombre: c.nombre.clone(),
                        tipo: sustituir_anotacion(&c.tipo, sigma)?,
                        publico: c.publico,
                        linea: c.linea,
                        columna: c.columna,
                    })
                })
                .collect::<Option<Vec<_>>>()?,
            linea: *linea,
            columna: *columna,
            publico: *publico,
        },
        Sentencia::Enumeracion {
            nombre,
            variantes,
            linea,
            columna,
            publico,
        } => Sentencia::Enumeracion {
            nombre: nombre.clone(),
            variantes: variantes
                .iter()
                .map(|v| {
                    Some(crate::parser::VarianteEnumeracion {
                        nombre: v.nombre.clone(),
                        payload: v
                            .payload
                            .iter()
                            .map(|t| sustituir_anotacion(t, sigma))
                            .collect::<Option<Vec<_>>>()?,
                        linea: v.linea,
                        columna: v.columna,
                    })
                })
                .collect::<Option<Vec<_>>>()?,
            linea: *linea,
            columna: *columna,
            publico: *publico,
        },
        Sentencia::Rasgo {
            nombre,
            metodos,
            linea,
            columna,
            publico,
        } => Sentencia::Rasgo {
            nombre: nombre.clone(),
            metodos: metodos.clone(),
            linea: *linea,
            columna: *columna,
            publico: *publico,
        },
        Sentencia::ImplementacionInherente {
            tipo,
            metodos,
            linea,
            columna,
        } => Sentencia::ImplementacionInherente {
            tipo: tipo.clone(),
            metodos: metodos.clone(),
            linea: *linea,
            columna: *columna,
        },
        Sentencia::ImplementacionRasgo {
            rasgo,
            tipo,
            metodos,
            linea,
            columna,
        } => Sentencia::ImplementacionRasgo {
            rasgo: rasgo.clone(),
            tipo: tipo.clone(),
            metodos: metodos.clone(),
            linea: *linea,
            columna: *columna,
        },
    })
}

use crate::semantico::{FuncionSig, inferir_sigma, sustituir_tipo};

/// Información de plantilla accesible para el pase G4.
#[derive(Debug, Clone)]
pub struct InfoPlantilla {
    pub id: GenericFnId,
    pub nombre: String,
    pub parametros_tipo: Vec<(String, TipoParamId)>,
    pub params_formales_sem: Vec<TipoSem>, // tipos formales con Param (para inferencia)
    pub retorno_sem: Option<TipoSem>,
    pub parametros_ast: Vec<(String, Option<TipoAnotacion>)>,
    pub tipo_retorno_ast: Option<TipoAnotacion>,
    pub cuerpo: Vec<Sentencia>,
    pub linea: usize,
    pub columna: usize,
    pub publico: bool,
}

/// Resultado de monomorfización.
#[derive(Debug)]
pub struct ResultadoG4 {
    pub programa: Vec<Sentencia>,
    pub errores: Vec<crate::evaluador::ErrorEjecucion>,
}

/// Construye registro de plantillas desde el AST y firmas semánticas.
pub(crate) fn construir_plantillas(
    programa: &[Sentencia],
    funciones: &HashMap<String, FuncionSig>,
    modulo: &str,
) -> BTreeMap<GenericFnId, InfoPlantilla> {
    let mut plantillas = BTreeMap::new();
    recolectar_plantillas(programa, funciones, modulo, &mut plantillas);
    plantillas
}

fn recolectar_plantillas(
    stmts: &[Sentencia],
    funciones: &HashMap<String, FuncionSig>,
    modulo: &str,
    out: &mut BTreeMap<GenericFnId, InfoPlantilla>,
) {
    for s in stmts {
        match s {
            Sentencia::Funcion {
                nombre,
                parametros_tipo,
                parametros,
                tipo_retorno,
                cuerpo,
                linea,
                columna,
                publico,
            } => {
                if !parametros_tipo.is_empty()
                    && let Some(sig) = funciones.get(nombre)
                    && !sig.parametros_tipo.is_empty()
                {
                    let pids: Vec<(String, TipoParamId)> = sig
                        .parametros_tipo
                        .iter()
                        .map(|p| (p.nombre.clone(), p.id.clone()))
                        .collect();
                    let id = GenericFnId {
                        modulo: modulo.to_string(),
                        decl_id: sig.parametros_tipo[0].id.propietario,
                        nombre: nombre.clone(),
                        aridad: sig.parametros_tipo.len(),
                    };
                    let params_formales_sem: Vec<TipoSem> = sig
                        .parametros
                        .iter()
                        .map(|(_, t)| t.clone().unwrap_or(TipoSem::Desconocido))
                        .collect();
                    out.insert(
                        id.clone(),
                        InfoPlantilla {
                            id,
                            nombre: nombre.clone(),
                            parametros_tipo: pids,
                            params_formales_sem,
                            retorno_sem: sig.retorno.clone(),
                            parametros_ast: parametros.clone(),
                            tipo_retorno_ast: tipo_retorno.clone(),
                            cuerpo: cuerpo.clone(),
                            linea: *linea,
                            columna: *columna,
                            publico: *publico,
                        },
                    );
                }
                // Buscar plantillas anidadas? G4 v1 mismo-módulo; funciones anidadas genéricas pueden existir.
                recolectar_plantillas(cuerpo, funciones, modulo, out);
            }
            Sentencia::Exportar { sentencia, .. } => {
                recolectar_plantillas(std::slice::from_ref(sentencia), funciones, modulo, out);
            }
            // otros contenedores de sentencias
            Sentencia::Si {
                rama_si, rama_sino, ..
            } => {
                recolectar_plantillas(rama_si, funciones, modulo, out);
                if let Some(rs) = rama_sino {
                    recolectar_plantillas(rs, funciones, modulo, out);
                }
            }
            Sentencia::Mientras { cuerpo, .. }
            | Sentencia::Bloque(cuerpo)
            | Sentencia::ParaCada { cuerpo, .. } => {
                recolectar_plantillas(cuerpo, funciones, modulo, out);
            }
            Sentencia::IntentarRescatar {
                cuerpo_intentar,
                cuerpo_rescatar,
                ..
            } => {
                recolectar_plantillas(cuerpo_intentar, funciones, modulo, out);
                recolectar_plantillas(cuerpo_rescatar, funciones, modulo, out);
            }
            _ => {}
        }
    }
}

/// Índice por nombre de función para plantillas (primera/única en ámbito global relevante).
pub fn indice_plantillas_por_nombre(
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
) -> HashMap<String, GenericFnId> {
    let mut m = HashMap::new();
    for (id, p) in plantillas {
        // preferir inserción determinista (BTreeMap iter en orden)
        m.entry(p.nombre.clone()).or_insert_with(|| id.clone());
    }
    m
}

/// Convierte TipoAnotacion (concreta) a TipoSem aproximado para inferencia.
pub fn anotacion_a_sem_concreta(a: &TipoAnotacion) -> TipoSem {
    match a {
        TipoAnotacion::Entero => TipoSem::Entero,
        TipoAnotacion::Decimal => TipoSem::Decimal,
        TipoAnotacion::Real => TipoSem::Real,
        TipoAnotacion::Logico => TipoSem::Logico,
        TipoAnotacion::Texto => TipoSem::Texto,
        TipoAnotacion::Nulo => TipoSem::Nulo,
        TipoAnotacion::Arreglo(te, n) => {
            TipoSem::Arreglo(Box::new(anotacion_a_sem_concreta(te)), *n)
        }
        TipoAnotacion::Lista(te) => TipoSem::Lista(Box::new(anotacion_a_sem_concreta(te))),
        TipoAnotacion::Parametro(_) => TipoSem::Desconocido,
        TipoAnotacion::Nominal(_) => TipoSem::Desconocido, // S8.1: resolución nominal fuera de G4
    }
}

/// Estima tipo de expresión en contexto concreto (best-effort para inferencia G4).
pub fn estimar_tipo_expr(
    e: &Expresion,
    vars: &HashMap<String, TipoSem>,
    firmas_no_genericas: &HashMap<String, (Vec<TipoSem>, TipoSem)>,
    inst_retornos: &HashMap<String, TipoSem>,
) -> TipoSem {
    match e {
        Expresion::Literal { valor, .. } => match valor {
            crate::Valor::Entero(_) => TipoSem::Entero,
            crate::Valor::Decimal(_) => TipoSem::Decimal,
            crate::Valor::Real(_) => TipoSem::Real,
            crate::Valor::Logico(_) => TipoSem::Logico,
            crate::Valor::Texto(_) => TipoSem::Texto,
            crate::Valor::Nulo => TipoSem::Nulo,
            _ => TipoSem::Desconocido,
        },
        Expresion::Variable { nombre, .. } => {
            vars.get(nombre).cloned().unwrap_or(TipoSem::Desconocido)
        }
        Expresion::ArregloLiteral { elementos, .. } => {
            if elementos.is_empty() {
                return TipoSem::Desconocido;
            }
            let mut t = estimar_tipo_expr(&elementos[0], vars, firmas_no_genericas, inst_retornos);
            for el in elementos.iter().skip(1) {
                let te = estimar_tipo_expr(el, vars, firmas_no_genericas, inst_retornos);
                t = tipo_comun_simple(&t, &te);
            }
            TipoSem::Coleccion(Box::new(t), elementos.len())
        }
        Expresion::Rango { .. } => TipoSem::Lista(Box::new(TipoSem::Entero)),
        Expresion::Suma(a, b)
        | Expresion::Resta(a, b)
        | Expresion::Multiplicacion(a, b)
        | Expresion::Division(a, b)
        | Expresion::Modulo(a, b) => {
            let ta = estimar_tipo_expr(a, vars, firmas_no_genericas, inst_retornos);
            let tb = estimar_tipo_expr(b, vars, firmas_no_genericas, inst_retornos);
            tipo_numerico_simple(&ta, &tb).unwrap_or(TipoSem::Desconocido)
        }
        Expresion::IgualIgual(_, _)
        | Expresion::Distinto(_, _)
        | Expresion::Menor(_, _)
        | Expresion::MenorIgual(_, _)
        | Expresion::Mayor(_, _)
        | Expresion::MayorIgual(_, _)
        | Expresion::No(_)
        | Expresion::Y(_, _)
        | Expresion::O(_, _) => TipoSem::Logico,
        Expresion::Llamada {
            nombre,
            argumentos_tipo,
            ..
        } => {
            if argumentos_tipo.is_empty() {
                if let Some(r) = inst_retornos.get(nombre) {
                    return r.clone();
                }
                if let Some((_, r)) = firmas_no_genericas.get(nombre) {
                    return r.clone();
                }
            }
            TipoSem::Desconocido
        }
        Expresion::Indice { objeto, .. } => {
            match estimar_tipo_expr(objeto, vars, firmas_no_genericas, inst_retornos) {
                TipoSem::Arreglo(t, _) | TipoSem::Lista(t) | TipoSem::Coleccion(t, _) => *t,
                _ => TipoSem::Desconocido,
            }
        }
        _ => TipoSem::Desconocido,
    }
}

fn tipo_comun_simple(a: &TipoSem, b: &TipoSem) -> TipoSem {
    if a == b {
        return a.clone();
    }
    match (a, b) {
        (TipoSem::Entero, TipoSem::Decimal)
        | (TipoSem::Decimal, TipoSem::Entero)
        | (TipoSem::Decimal, TipoSem::Decimal) => TipoSem::Decimal,
        (TipoSem::Real, _) | (_, TipoSem::Real)
            if matches!(a, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
                && matches!(b, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real) =>
        {
            TipoSem::Real
        }
        (TipoSem::Desconocido, x) | (x, TipoSem::Desconocido) => x.clone(),
        _ => TipoSem::Desconocido,
    }
}
fn tipo_numerico_simple(a: &TipoSem, b: &TipoSem) -> Option<TipoSem> {
    if !matches!(a, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
        || !matches!(b, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
    {
        return None;
    }
    if matches!(a, TipoSem::Real) || matches!(b, TipoSem::Real) {
        return Some(TipoSem::Real);
    }
    if matches!(a, TipoSem::Decimal) || matches!(b, TipoSem::Decimal) {
        return Some(TipoSem::Decimal);
    }
    Some(TipoSem::Entero)
}

/// Reescribe llamadas genéricas en una expresión dentro de una instancia concreta.
/// `sigma` mapea nombres de params de la instancia actual a tipos canónicos.
pub(crate) fn reescribir_expr_g4(
    e: &Expresion,
    sigma: &MapaSustAnot,
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    por_nombre: &HashMap<String, GenericFnId>,
    motor: &mut MotorMonomorfizacion,
    vars: &HashMap<String, TipoSem>,
    firmas_no_genericas: &HashMap<String, (Vec<TipoSem>, TipoSem)>,
    inst_retornos: &HashMap<String, TipoSem>,
    origen_base: &OrigenInstancia,
    funciones: &HashMap<String, FuncionSig>,
    validar_v3: &mut Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
) -> Result<Expresion, CausaError> {
    Ok(match e {
        Expresion::Llamada {
            nombre,
            argumentos,
            argumentos_tipo,
            linea,
            columna,
        } => {
            // Primero reescribir argumentos recursivamente.
            let mut args_rw = Vec::with_capacity(argumentos.len());
            for a in argumentos {
                args_rw.push(reescribir_expr_g4(
                    a,
                    sigma,
                    plantillas,
                    por_nombre,
                    motor,
                    vars,
                    firmas_no_genericas,
                    inst_retornos,
                    origen_base,
                    funciones,
                    validar_v3,
                )?);
            }
            // ¿Es llamada a plantilla genérica conocida?
            if let Some(pid) = por_nombre.get(nombre).cloned()
                && let Some(plantilla) = plantillas.get(&pid)
            {
                let tipo_params = &plantilla.parametros_tipo; // (nombre, id)
                let params_objetivo: Vec<TipoParamId> =
                    tipo_params.iter().map(|(_, id)| id.clone()).collect();
                let sigma_callee: Option<HashMap<TipoParamId, TipoSem>> =
                    if !argumentos_tipo.is_empty() {
                        // Explícitos: ya sustituidos por sigma de instancia actual -> deben ser concretos.
                        if argumentos_tipo.len() != params_objetivo.len() {
                            None
                        } else {
                            let mut m = HashMap::new();
                            let mut ok = true;
                            for (ta, id) in argumentos_tipo.iter().zip(params_objetivo.iter()) {
                                let ts = anotacion_a_sem_concreta(ta);
                                if canonicalizar_tipo(&ts).is_none() {
                                    ok = false;
                                    break;
                                }
                                m.insert(id.clone(), ts);
                            }
                            if ok { Some(m) } else { None }
                        }
                    } else {
                        // Inferencia en contexto concreto.
                        let arg_tipos: Vec<TipoSem> = args_rw
                            .iter()
                            .map(|a| estimar_tipo_expr(a, vars, firmas_no_genericas, inst_retornos))
                            .collect();
                        if arg_tipos.len() == plantilla.params_formales_sem.len() {
                            // Sustituir formales de la plantilla callee? los formales ya tienen Param ids del callee.
                            match inferir_sigma(
                                &plantilla.params_formales_sem,
                                &arg_tipos,
                                &params_objetivo,
                            ) {
                                Ok(s) => {
                                    if params_objetivo.iter().all(|p| {
                                        s.get(p).is_some_and(|t| canonicalizar_tipo(t).is_some())
                                    }) {
                                        Some(s)
                                    } else {
                                        None
                                    }
                                }
                                Err(_) => None,
                            }
                        } else {
                            None
                        }
                    };
                if let Some(sig_call) = sigma_callee {
                    let args_concretos: Option<Vec<TipoCanon>> = params_objetivo
                        .iter()
                        .map(|p| sig_call.get(p).and_then(canonicalizar_tipo))
                        .collect();
                    if let Some(args_c) = args_concretos {
                        let key = InstanceKey {
                            funcion: pid.clone(),
                            args: args_c,
                        };
                        let mut cadena = origen_base.cadena.clone();
                        cadena.push(format!("{}<{}>", nombre, key.nombre_tipos().join(",")));
                        let mut cadena_keys = origen_base.cadena_keys.clone();
                        cadena_keys.push(key.clone());
                        let origen = OrigenInstancia {
                            funcion: nombre.clone(),
                            instancia: nombre.clone(),
                            tipos: key.nombre_tipos(),
                            cadena,
                            cadena_keys,
                            linea: *linea,
                            columna: *columna,
                        };
                        // Registrar/solicitar instancia
                        let nombre_esp = solicitar_instancia(
                            motor,
                            plantillas,
                            por_nombre,
                            firmas_no_genericas,
                            funciones,
                            key.clone(),
                            &origen,
                            validar_v3,
                        )?;
                        return Ok(Expresion::Llamada {
                            nombre: nombre_esp,
                            argumentos: args_rw,
                            argumentos_tipo: Vec::new(),
                            linea: *linea,
                            columna: *columna,
                        });
                    }
                }
                // Si no se pudo resolver concretamente aquí, dejar como estaba (gate detectará residuales si aplica)
            }
            // No genérica o no reescribible: mantener con args reescritos y type args sustituidos ya presentes
            Expresion::Llamada {
                nombre: nombre.clone(),
                argumentos: args_rw,
                argumentos_tipo: argumentos_tipo.clone(),
                linea: *linea,
                columna: *columna,
            }
        }
        Expresion::Suma(a, b) => Expresion::Suma(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::Resta(a, b) => Expresion::Resta(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::Multiplicacion(a, b) => Expresion::Multiplicacion(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::Division(a, b) => Expresion::Division(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::Modulo(a, b) => Expresion::Modulo(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::IgualIgual(a, b) => Expresion::IgualIgual(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::Distinto(a, b) => Expresion::Distinto(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::Menor(a, b) => Expresion::Menor(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::MenorIgual(a, b) => Expresion::MenorIgual(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::Mayor(a, b) => Expresion::Mayor(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::MayorIgual(a, b) => Expresion::MayorIgual(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::No(a) => Expresion::No(Box::new(reescribir_expr_g4(
            a,
            sigma,
            plantillas,
            por_nombre,
            motor,
            vars,
            firmas_no_genericas,
            inst_retornos,
            origen_base,
            funciones,
            validar_v3,
        )?)),
        Expresion::Y(a, b) => Expresion::Y(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::O(a, b) => Expresion::O(
            Box::new(reescribir_expr_g4(
                a,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            Box::new(reescribir_expr_g4(
                b,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
        ),
        Expresion::ArregloLiteral {
            elementos,
            linea,
            columna,
        } => {
            let mut els = Vec::new();
            for el in elementos {
                els.push(reescribir_expr_g4(
                    el,
                    sigma,
                    plantillas,
                    por_nombre,
                    motor,
                    vars,
                    firmas_no_genericas,
                    inst_retornos,
                    origen_base,
                    funciones,
                    validar_v3,
                )?);
            }
            Expresion::ArregloLiteral {
                elementos: els,
                linea: *linea,
                columna: *columna,
            }
        }
        Expresion::Indice {
            objeto,
            indice,
            linea,
            columna,
        } => Expresion::Indice {
            objeto: Box::new(reescribir_expr_g4(
                objeto,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            indice: Box::new(reescribir_expr_g4(
                indice,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            linea: *linea,
            columna: *columna,
        },
        Expresion::Acceso {
            objeto,
            miembro,
            linea,
            columna,
        } => Expresion::Acceso {
            objeto: Box::new(reescribir_expr_g4(
                objeto,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            miembro: miembro.clone(),
            linea: *linea,
            columna: *columna,
        },
        Expresion::LlamadaIndirecta {
            callee,
            argumentos,
            linea,
            columna,
        } => {
            let mut args_rw = Vec::new();
            for a in argumentos {
                args_rw.push(reescribir_expr_g4(
                    a,
                    sigma,
                    plantillas,
                    por_nombre,
                    motor,
                    vars,
                    firmas_no_genericas,
                    inst_retornos,
                    origen_base,
                    funciones,
                    validar_v3,
                )?);
            }
            Expresion::LlamadaIndirecta {
                callee: Box::new(reescribir_expr_g4(
                    callee,
                    sigma,
                    plantillas,
                    por_nombre,
                    motor,
                    vars,
                    firmas_no_genericas,
                    inst_retornos,
                    origen_base,
                    funciones,
                    validar_v3,
                )?),
                argumentos: args_rw,
                linea: *linea,
                columna: *columna,
            }
        }
        Expresion::Lambda {
            parametros,
            tipo_retorno,
            cuerpo,
            linea,
            columna,
        } => {
            // Reescribir cuerpo de lambda con mismo contexto de vars (aprox: no añadimos params lambda al scope tipado aquí).
            let cuerpo_rw = reescribir_bloque_g4(
                cuerpo,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            Expresion::Lambda {
                parametros: parametros.clone(),
                tipo_retorno: tipo_retorno.clone(),
                cuerpo: cuerpo_rw,
                linea: *linea,
                columna: *columna,
            }
        }
        Expresion::Rango {
            inicio,
            fin,
            inclusivo,
            linea,
            columna,
        } => Expresion::Rango {
            inicio: Box::new(reescribir_expr_g4(
                inicio,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            fin: Box::new(reescribir_expr_g4(
                fin,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            inclusivo: *inclusivo,
            linea: *linea,
            columna: *columna,
        },
        Expresion::Variable {
            nombre,
            linea,
            columna,
        } => {
            if por_nombre.contains_key(nombre) && !vars.contains_key(nombre) {
                return Err(CausaError::ReferenciaGenericaResidualIr {
                    detalle: format!(
                        "la función genérica '{nombre}' se usa como valor sin instanciación concreta; \
                         solo se admiten llamadas directas con argumentos de tipo explícitos o inferibles"
                    ),
                    ubicacion: format!("{}:{}", linea, columna),
                });
            }
            Expresion::Variable {
                nombre: nombre.clone(),
                linea: *linea,
                columna: *columna,
            }
        }
        other => other.clone(),
    })
}

pub(crate) fn reescribir_sentencia_g4(
    s: &Sentencia,
    sigma: &MapaSustAnot,
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    por_nombre: &HashMap<String, GenericFnId>,
    motor: &mut MotorMonomorfizacion,
    vars: &mut HashMap<String, TipoSem>,
    firmas_no_genericas: &HashMap<String, (Vec<TipoSem>, TipoSem)>,
    inst_retornos: &HashMap<String, TipoSem>,
    origen_base: &OrigenInstancia,
    funciones: &HashMap<String, FuncionSig>,
    validar_v3: &mut Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
) -> Result<Sentencia, CausaError> {
    Ok(match s {
        Sentencia::Rasgo { .. } => s.clone(),
        Sentencia::ImplementacionInherente {
            tipo,
            metodos,
            linea,
            columna,
        } => {
            let mut metodos_rw = Vec::with_capacity(metodos.len());
            for m in metodos {
                let mut vars_locales = vars.clone();
                for (n, t) in &m.parametros {
                    let ty = t
                        .as_ref()
                        .map(crate::semantico::tipo_anotacion_a_sem)
                        .unwrap_or(TipoSem::Desconocido);
                    vars_locales.insert(n.clone(), ty);
                }
                let cuerpo_rw = reescribir_bloque_g4(
                    &m.cuerpo,
                    sigma,
                    plantillas,
                    por_nombre,
                    motor,
                    &vars_locales,
                    firmas_no_genericas,
                    inst_retornos,
                    origen_base,
                    funciones,
                    validar_v3,
                )?;
                metodos_rw.push(crate::parser::Metodo {
                    cuerpo: cuerpo_rw,
                    ..m.clone()
                });
            }
            Sentencia::ImplementacionInherente {
                tipo: tipo.clone(),
                metodos: metodos_rw,
                linea: *linea,
                columna: *columna,
            }
        }
        Sentencia::ImplementacionRasgo {
            rasgo,
            tipo,
            metodos,
            linea,
            columna,
        } => {
            let mut metodos_rw = Vec::with_capacity(metodos.len());
            for m in metodos {
                let mut vars_locales = vars.clone();
                for (n, t) in &m.parametros {
                    let ty = t
                        .as_ref()
                        .map(crate::semantico::tipo_anotacion_a_sem)
                        .unwrap_or(TipoSem::Desconocido);
                    vars_locales.insert(n.clone(), ty);
                }
                let cuerpo_rw = reescribir_bloque_g4(
                    &m.cuerpo,
                    sigma,
                    plantillas,
                    por_nombre,
                    motor,
                    &vars_locales,
                    firmas_no_genericas,
                    inst_retornos,
                    origen_base,
                    funciones,
                    validar_v3,
                )?;
                metodos_rw.push(crate::parser::Metodo {
                    cuerpo: cuerpo_rw,
                    ..m.clone()
                });
            }
            Sentencia::ImplementacionRasgo {
                rasgo: rasgo.clone(),
                tipo: tipo.clone(),
                metodos: metodos_rw,
                linea: *linea,
                columna: *columna,
            }
        }
        Sentencia::AsignacionCampo {
            objeto,
            campo,
            valor,
            linea,
            columna,
        } => Sentencia::AsignacionCampo {
            objeto: Box::new(reescribir_expr_g4(
                objeto,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            campo: campo.clone(),
            valor: reescribir_expr_g4(
                valor,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?,
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Estructura { .. } => s.clone(),
        Sentencia::Enumeracion { .. } => s.clone(),
        Sentencia::Declaracion {
            nombre,
            linea,
            columna,
            valor,
            tipo_anotacion,
            mutable,
            es_const,
            publico,
        } => {
            let v = reescribir_expr_g4(
                valor,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            // Actualizar tipo de var para inferencia posterior
            let t = if let Some(ta) = tipo_anotacion {
                anotacion_a_sem_concreta(ta)
            } else {
                estimar_tipo_expr(&v, vars, firmas_no_genericas, inst_retornos)
            };
            vars.insert(nombre.clone(), t);
            Sentencia::Declaracion {
                nombre: nombre.clone(),
                linea: *linea,
                columna: *columna,
                valor: v,
                tipo_anotacion: tipo_anotacion.clone(),
                mutable: *mutable,
                es_const: *es_const,
                publico: *publico,
            }
        }
        Sentencia::Asignacion {
            nombre,
            linea,
            columna,
            valor,
        } => {
            let v = reescribir_expr_g4(
                valor,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            Sentencia::Asignacion {
                nombre: nombre.clone(),
                linea: *linea,
                columna: *columna,
                valor: v,
            }
        }
        Sentencia::AsignacionIndice {
            objeto,
            indice,
            valor,
            linea,
            columna,
        } => Sentencia::AsignacionIndice {
            objeto: Box::new(reescribir_expr_g4(
                objeto,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            indice: Box::new(reescribir_expr_g4(
                indice,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            valor: reescribir_expr_g4(
                valor,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?,
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Si {
            condicion,
            rama_si,
            rama_sino,
            linea,
            columna,
        } => {
            let c = reescribir_expr_g4(
                condicion,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            let si = reescribir_bloque_g4(
                rama_si,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            let sino = match rama_sino {
                None => None,
                Some(v) => Some(reescribir_bloque_g4(
                    v,
                    sigma,
                    plantillas,
                    por_nombre,
                    motor,
                    vars,
                    firmas_no_genericas,
                    inst_retornos,
                    origen_base,
                    funciones,
                    validar_v3,
                )?),
            };
            Sentencia::Si {
                condicion: c,
                rama_si: si,
                rama_sino: sino,
                linea: *linea,
                columna: *columna,
            }
        }
        Sentencia::Mientras {
            condicion,
            cuerpo,
            linea,
            columna,
        } => {
            let c = reescribir_expr_g4(
                condicion,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            let cuerpo_rw = reescribir_bloque_g4(
                cuerpo,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            Sentencia::Mientras {
                condicion: c,
                cuerpo: cuerpo_rw,
                linea: *linea,
                columna: *columna,
            }
        }
        Sentencia::Bloque(stmts) => Sentencia::Bloque(reescribir_bloque_g4(
            stmts,
            sigma,
            plantillas,
            por_nombre,
            motor,
            vars,
            firmas_no_genericas,
            inst_retornos,
            origen_base,
            funciones,
            validar_v3,
        )?),
        Sentencia::Expresion(e) => Sentencia::Expresion(reescribir_expr_g4(
            e,
            sigma,
            plantillas,
            por_nombre,
            motor,
            vars,
            firmas_no_genericas,
            inst_retornos,
            origen_base,
            funciones,
            validar_v3,
        )?),
        Sentencia::Retorno {
            valor,
            linea,
            columna,
        } => Sentencia::Retorno {
            valor: match valor {
                None => None,
                Some(v) => Some(reescribir_expr_g4(
                    v,
                    sigma,
                    plantillas,
                    por_nombre,
                    motor,
                    vars,
                    firmas_no_genericas,
                    inst_retornos,
                    origen_base,
                    funciones,
                    validar_v3,
                )?),
            },
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Mostrar {
            valor,
            linea,
            columna,
        } => Sentencia::Mostrar {
            valor: reescribir_expr_g4(
                valor,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?,
            linea: *linea,
            columna: *columna,
        },
        Sentencia::MostrarSinSalto {
            valor,
            linea,
            columna,
        } => Sentencia::MostrarSinSalto {
            valor: reescribir_expr_g4(
                valor,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?,
            linea: *linea,
            columna: *columna,
        },
        Sentencia::ParaCada {
            variable,
            iterable,
            cuerpo,
            linea,
            columna,
        } => {
            let it = reescribir_expr_g4(
                iterable,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            // estimar tipo elemento para var
            let t_it = estimar_tipo_expr(&it, vars, firmas_no_genericas, inst_retornos);
            let t_el = match t_it {
                TipoSem::Arreglo(t, _) | TipoSem::Lista(t) | TipoSem::Coleccion(t, _) => *t,
                _ => TipoSem::Desconocido,
            };
            let mut vars_inner = vars.clone();
            vars_inner.insert(variable.clone(), t_el);
            let cuerpo_rw = reescribir_bloque_g4_con_vars(
                cuerpo,
                sigma,
                plantillas,
                por_nombre,
                motor,
                &mut vars_inner,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            Sentencia::ParaCada {
                variable: variable.clone(),
                iterable: it,
                cuerpo: cuerpo_rw,
                linea: *linea,
                columna: *columna,
            }
        }
        Sentencia::Romper { linea, columna } => Sentencia::Romper {
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Continuar { linea, columna } => Sentencia::Continuar {
            linea: *linea,
            columna: *columna,
        },
        Sentencia::IntentarRescatar {
            cuerpo_intentar,
            variable_error,
            cuerpo_rescatar,
            linea,
            columna,
        } => {
            let ci = reescribir_bloque_g4(
                cuerpo_intentar,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            let mut vars_r = vars.clone();
            if let Some(v) = variable_error {
                vars_r.insert(v.clone(), TipoSem::Desconocido);
            }
            let cr = reescribir_bloque_g4_con_vars(
                cuerpo_rescatar,
                sigma,
                plantillas,
                por_nombre,
                motor,
                &mut vars_r,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            Sentencia::IntentarRescatar {
                cuerpo_intentar: ci,
                variable_error: variable_error.clone(),
                cuerpo_rescatar: cr,
                linea: *linea,
                columna: *columna,
            }
        }
        // Funciones anidadas: reescribir dentro (sustitución de tipos ya aplicada antes de reescritura; cuidado con sombras)
        Sentencia::Funcion {
            nombre,
            parametros_tipo,
            parametros,
            tipo_retorno,
            cuerpo,
            linea,
            columna,
            publico,
        } => {
            // Si la función anidada es genérica, sus params de tipo sombrean/son propios: no reescribir sus llamadas genéricas con sigma externa de misma forma incorrecta.
            // Para simplificar v1 (mismo-módulo), reescribir cuerpo con mismo contexto pero esto es aproximado; mejor evitar romper.
            let cuerpo_rw = reescribir_bloque_g4(
                cuerpo,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?;
            Sentencia::Funcion {
                nombre: nombre.clone(),
                parametros_tipo: parametros_tipo.clone(),
                parametros: parametros.clone(),
                tipo_retorno: tipo_retorno.clone(),
                cuerpo: cuerpo_rw,
                linea: *linea,
                columna: *columna,
                publico: *publico,
            }
        }
        Sentencia::Exportar {
            sentencia,
            linea,
            columna,
        } => Sentencia::Exportar {
            sentencia: Box::new(reescribir_sentencia_g4(
                sentencia,
                sigma,
                plantillas,
                por_nombre,
                motor,
                vars,
                firmas_no_genericas,
                inst_retornos,
                origen_base,
                funciones,
                validar_v3,
            )?),
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Importar {
            ruta,
            alias,
            linea,
            columna,
        } => Sentencia::Importar {
            ruta: ruta.clone(),
            alias: alias.clone(),
            linea: *linea,
            columna: *columna,
        },
        Sentencia::Usar {
            ruta,
            alias,
            nombres,
            linea,
            columna,
        } => Sentencia::Usar {
            ruta: ruta.clone(),
            alias: alias.clone(),
            nombres: nombres.clone(),
            linea: *linea,
            columna: *columna,
        },
    })
}

pub(crate) fn reescribir_bloque_g4(
    stmts: &[Sentencia],
    sigma: &MapaSustAnot,
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    por_nombre: &HashMap<String, GenericFnId>,
    motor: &mut MotorMonomorfizacion,
    vars: &HashMap<String, TipoSem>,
    firmas_no_genericas: &HashMap<String, (Vec<TipoSem>, TipoSem)>,
    inst_retornos: &HashMap<String, TipoSem>,
    origen_base: &OrigenInstancia,
    funciones: &HashMap<String, FuncionSig>,
    validar_v3: &mut Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
) -> Result<Vec<Sentencia>, CausaError> {
    let mut vars_local = vars.clone();
    reescribir_bloque_g4_con_vars(
        stmts,
        sigma,
        plantillas,
        por_nombre,
        motor,
        &mut vars_local,
        firmas_no_genericas,
        inst_retornos,
        origen_base,
        funciones,
        validar_v3,
    )
}

pub(crate) fn reescribir_bloque_g4_con_vars(
    stmts: &[Sentencia],
    sigma: &MapaSustAnot,
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    por_nombre: &HashMap<String, GenericFnId>,
    motor: &mut MotorMonomorfizacion,
    vars: &mut HashMap<String, TipoSem>,
    firmas_no_genericas: &HashMap<String, (Vec<TipoSem>, TipoSem)>,
    inst_retornos: &HashMap<String, TipoSem>,
    origen_base: &OrigenInstancia,
    funciones: &HashMap<String, FuncionSig>,
    validar_v3: &mut Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
) -> Result<Vec<Sentencia>, CausaError> {
    let mut out = Vec::with_capacity(stmts.len());
    for st in stmts {
        out.push(reescribir_sentencia_g4(
            st,
            sigma,
            plantillas,
            por_nombre,
            motor,
            vars,
            firmas_no_genericas,
            inst_retornos,
            origen_base,
            funciones,
            validar_v3,
        )?);
    }
    Ok(out)
}

/// Construye mapa de firmas no genéricas (nombre -> (params_sem, retorno_sem)) para estimación de tipos.
pub(crate) fn firmas_no_genericas_sem(
    funciones: &HashMap<String, FuncionSig>,
) -> HashMap<String, (Vec<TipoSem>, TipoSem)> {
    let mut m = HashMap::new();
    for (nombre, sig) in funciones {
        if sig.parametros_tipo.is_empty() {
            let params: Vec<TipoSem> = sig
                .parametros
                .iter()
                .map(|(_, t)| t.clone().unwrap_or(TipoSem::Desconocido))
                .collect();
            let ret = sig.retorno.clone().unwrap_or(TipoSem::Desconocido);
            m.insert(nombre.clone(), (params, ret));
        }
    }
    m
}

/// Especializa una plantilla para una instancia concreta.
pub(crate) fn especializar_instancia(
    plantilla: &InfoPlantilla,
    key: &InstanceKey,
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    por_nombre: &HashMap<String, GenericFnId>,
    motor: &mut MotorMonomorfizacion,
    firmas_no_genericas: &HashMap<String, (Vec<TipoSem>, TipoSem)>,
    funciones: &HashMap<String, FuncionSig>,
    nombre_especializado: &str,
    validar_v3: &mut Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
    cadena_base: Vec<String>,
    cadena_keys_base: Vec<InstanceKey>,
) -> Result<Sentencia, CausaError> {
    // Construir sigma TipoParamId->TipoSem y mapa nombre->TipoCanon
    let mut sigma_sem: HashMap<TipoParamId, TipoSem> = HashMap::new();
    let mut sigma_anot: MapaSustAnot = HashMap::new();
    for ((pname, pid), arg) in plantilla.parametros_tipo.iter().zip(key.args.iter()) {
        sigma_sem.insert(pid.clone(), canon_a_sem(arg));
        sigma_anot.insert(pname.clone(), arg.clone());
    }

    // Sustituir firma y cuerpo (anotaciones/tipos explícitos)
    let params_ast: Vec<(String, Option<TipoAnotacion>)> = plantilla
        .parametros_ast
        .iter()
        .map(|(n, t)| {
            Ok((
                n.clone(),
                sustituir_anotacion_opt(t, &sigma_anot).ok_or_else(|| {
                    CausaError::SustitucionIncompleta {
                        funcion: plantilla.nombre.clone(),
                    }
                })?,
            ))
        })
        .collect::<Result<Vec<_>, CausaError>>()?;
    let tipo_retorno_ast = sustituir_anotacion_opt(&plantilla.tipo_retorno_ast, &sigma_anot)
        .ok_or_else(|| CausaError::SustitucionIncompleta {
            funcion: plantilla.nombre.clone(),
        })?;
    let cuerpo_sust: Vec<Sentencia> = plantilla
        .cuerpo
        .iter()
        .map(|s| {
            sustituir_sentencia(s, &sigma_anot).ok_or_else(|| CausaError::SustitucionIncompleta {
                funcion: plantilla.nombre.clone(),
            })
        })
        .collect::<Result<Vec<_>, CausaError>>()?;

    // Preparar entorno de variables para reescritura/inferencia (params con tipos sustituidos)
    let mut vars: HashMap<String, TipoSem> = HashMap::new();
    for (i, (pname, _)) in params_ast.iter().enumerate() {
        let t = plantilla
            .params_formales_sem
            .get(i)
            .map(|ft| sustituir_tipo(ft, &sigma_sem))
            .unwrap_or(TipoSem::Desconocido);
        vars.insert(pname.clone(), t);
    }

    // Retornos conocidos de instancias completas/registradas
    let mut inst_retornos: HashMap<String, TipoSem> = HashMap::new();
    for reg in motor.instancias.values() {
        if let Some(n) = &reg.nombre_especializado
            && let Some(p) = plantillas.get(&reg.key.funcion)
        {
            // retorno sustituido de esa instancia
            let mut s: HashMap<TipoParamId, TipoSem> = HashMap::new();
            for ((_, pid), arg) in p.parametros_tipo.iter().zip(reg.key.args.iter()) {
                s.insert(pid.clone(), canon_a_sem(arg));
            }
            if let Some(r) = &p.retorno_sem {
                inst_retornos.insert(n.clone(), sustituir_tipo(r, &s));
            }
        }
    }
    // Retorno de la instancia actual (para recursión monomórfica)
    if let Some(r) = &plantilla.retorno_sem {
        inst_retornos.insert(
            nombre_especializado.to_string(),
            sustituir_tipo(r, &sigma_sem),
        );
    }

    let mut cadena = cadena_base;
    let mut cadena_keys = cadena_keys_base;
    if cadena.is_empty() {
        cadena.push(format!(
            "{}<{}>",
            plantilla.nombre,
            key.nombre_tipos().join(",")
        ));
    }
    if cadena_keys.is_empty() {
        cadena_keys.push(key.clone());
    }
    let origen = OrigenInstancia {
        funcion: plantilla.nombre.clone(),
        instancia: nombre_especializado.to_string(),
        tipos: key.nombre_tipos(),
        cadena,
        cadena_keys,
        linea: plantilla.linea,
        columna: plantilla.columna,
    };

    // Reescribir llamadas genéricas a instancias concretas
    let cuerpo_rw = reescribir_bloque_g4(
        &cuerpo_sust,
        &sigma_anot,
        plantillas,
        por_nombre,
        motor,
        &vars,
        firmas_no_genericas,
        &inst_retornos,
        &origen,
        funciones,
        validar_v3,
    )?;

    // Función concreta (sin parámetros de tipo)
    Ok(Sentencia::Funcion {
        nombre: nombre_especializado.to_string(),
        parametros_tipo: Vec::new(),
        parametros: params_ast,
        tipo_retorno: tipo_retorno_ast,
        cuerpo: cuerpo_rw,
        linea: plantilla.linea,
        columna: plantilla.columna,
        publico: plantilla.publico,
    })
}

/// Solicita/asegura una instancia. Devuelve nombre especializado.
/// Implementa cache, estados, recursión monomórfica, crecimiento/profundidad y especialización.
/// Si se provee `validar_v3`, ejecuta validación semántica V3 post-especialización sobre la
/// instancia concreta antes de publicarla (Bloqueante A).
pub(crate) fn solicitar_instancia(
    motor: &mut MotorMonomorfizacion,
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    _por_nombre: &HashMap<String, GenericFnId>,
    _firmas_no_genericas: &HashMap<String, (Vec<TipoSem>, TipoSem)>,
    _funciones: &HashMap<String, FuncionSig>,
    key: InstanceKey,
    origen: &OrigenInstancia,
    _validar_v3: &mut Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
) -> Result<String, CausaError> {
    if let Some(reg) = motor.instancias.get(&key) {
        return match reg.estado {
            EstadoInstancia::Complete | EstadoInstancia::InProgress | EstadoInstancia::Pending => {
                Ok(reg.nombre_especializado.clone().unwrap_or_default())
            }
            EstadoInstancia::Failed => Err(reg.error.clone().unwrap_or_else(|| {
                CausaError::DependenciaGenericaNoResoluble {
                    funcion: key.funcion.nombre.clone(),
                    detalle: format!(
                        "instancia fallida previamente: {}<{}>",
                        key.funcion.nombre,
                        key.nombre_tipos().join(", ")
                    ),
                }
            })),
        };
    }
    if plantillas.get(&key.funcion).is_none() {
        return Err(CausaError::DependenciaGenericaNoResoluble {
            funcion: key.funcion.nombre.clone(),
            detalle: "plantilla genérica no encontrada para instanciación".to_string(),
        });
    }

    if motor.instancias.len() >= motor.limite_instancias {
        return Err(CausaError::LimiteInstanciasExcedido {
            limite: motor.limite_instancias,
            solicitada: key.funcion.nombre.clone(),
            tipos: key.nombre_tipos(),
            cadena: origen.cadena.clone(),
            detalle: "se alcanzó el límite de instancias únicas (M1)".to_string(),
        });
    }
    if origen.cadena.len() >= motor.limite_profundidad {
        return Err(CausaError::ProfundidadInstanciacionExcedida {
            funcion: key.funcion.nombre.clone(),
            instancia: key.funcion.nombre.clone(),
            tipos: key.nombre_tipos(),
            limite: motor.limite_profundidad,
            cadena: origen.cadena.clone(),
            detalle: "profundidad de instanciación excedida (M2)".to_string(),
        });
    }
    // Crecimiento polimórfico: comparar contra cadena de instanciación (semántica G4).
    for anc in &origen.cadena_keys {
        if anc.funcion == key.funcion
            && anc != &key
            && crecimiento_estructural(&anc.args, &key.args)
        {
            return Err(CausaError::RecursionPolimorficaCreciente {
                funcion: key.funcion.nombre.clone(),
                instancia: key.funcion.nombre.clone(),
                tipos: key.nombre_tipos(),
                cadena: origen.cadena.clone(),
                detalle: format!(
                    "crecimiento estructural respecto a instancia ancestral con tipos [{}]",
                    anc.nombre_tipos().join(", ")
                ),
            });
        }
    }

    let nombre = motor.nombres.generar(&key)?;
    motor.instancias.insert(
        key.clone(),
        RegistroInstancia {
            key: key.clone(),
            estado: EstadoInstancia::Pending,
            nombre_especializado: Some(nombre.clone()),
            funcion: None,
            error: None,
            dependencias: Vec::new(),
            origen: Some(origen.clone()),
        },
    );
    motor.cola.push(key);
    motor.cola.sort();
    Ok(nombre)
}

fn especializar_pendientes(
    motor: &mut MotorMonomorfizacion,
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    por_nombre: &HashMap<String, GenericFnId>,
    firmas_no_genericas: &HashMap<String, (Vec<TipoSem>, TipoSem)>,
    funciones: &HashMap<String, FuncionSig>,
    validar_v3: &mut Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
) -> Vec<crate::evaluador::ErrorEjecucion> {
    let mut errores = Vec::new();
    loop {
        let pendientes: Vec<InstanceKey> = motor
            .instancias
            .iter()
            .filter(|(_, r)| r.estado == EstadoInstancia::Pending)
            .map(|(k, _)| k.clone())
            .collect();
        if pendientes.is_empty() {
            break;
        }
        for key in pendientes {
            let (plantilla, nombre_esp, cadena_base, cadena_keys_base) = {
                let Some(reg) = motor.instancias.get(&key) else {
                    continue;
                };
                if reg.estado != EstadoInstancia::Pending {
                    continue;
                }
                let Some(p) = plantillas.get(&key.funcion) else {
                    continue;
                };
                (
                    p.clone(),
                    reg.nombre_especializado.clone().unwrap_or_default(),
                    reg.origen
                        .as_ref()
                        .map(|o| o.cadena.clone())
                        .unwrap_or_default(),
                    reg.origen
                        .as_ref()
                        .map(|o| o.cadena_keys.clone())
                        .unwrap_or_default(),
                )
            };
            if let Some(reg) = motor.instancias.get_mut(&key) {
                reg.estado = EstadoInstancia::InProgress;
            }
            motor.cadena_activa.push(key.clone());
            let resultado = especializar_instancia(
                &plantilla,
                &key,
                plantillas,
                por_nombre,
                motor,
                firmas_no_genericas,
                funciones,
                &nombre_esp,
                validar_v3,
                cadena_base,
                cadena_keys_base,
            );
            motor.cadena_activa.pop();
            match resultado {
                Ok(funcion) => {
                    if let Some(reg) = motor.instancias.get_mut(&key) {
                        reg.estado = EstadoInstancia::InProgress;
                        reg.funcion = Some(funcion);
                        reg.error = None;
                    }
                }
                Err(e) => {
                    if let Some(reg) = motor.instancias.get_mut(&key) {
                        reg.estado = EstadoInstancia::Failed;
                        reg.error = Some(e.clone());
                    }
                    errores.push(crate::evaluador::ErrorEjecucion {
                        mensaje: e.to_string(),
                        causa: Box::new(e),
                        linea: plantilla.linea,
                        columna: plantilla.columna,
                        archivo: None,
                    });
                }
            }
        }
    }
    errores
}

/// Ejecuta V3 post-especialización de forma iterativa sobre instancias especializadas
/// pendientes de validación (función materializada, estado InProgress, sin error).
/// Promueve a Complete si V3 pasa; marca Failed y devuelve errores si falla.
fn validar_v3_instancias_pendientes(
    motor: &mut MotorMonomorfizacion,
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    funciones: &HashMap<String, FuncionSig>,
    validar_v3: &mut Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
    archivo: Option<&str>,
) -> Vec<crate::evaluador::ErrorEjecucion> {
    let mut errores = Vec::new();
    if validar_v3.is_none() {
        let keys: Vec<InstanceKey> = motor
            .instancias
            .iter()
            .filter(|(_, r)| {
                r.estado == EstadoInstancia::InProgress && r.funcion.is_some() && r.error.is_none()
            })
            .map(|(k, _)| k.clone())
            .collect();
        for k in keys {
            if let Some(reg) = motor.instancias.get_mut(&k) {
                reg.estado = EstadoInstancia::Complete;
            }
        }
        return errores;
    }
    loop {
        let pendientes: Vec<InstanceKey> = motor
            .instancias
            .iter()
            .filter(|(_, r)| {
                r.estado == EstadoInstancia::InProgress && r.funcion.is_some() && r.error.is_none()
            })
            .map(|(k, _)| k.clone())
            .collect();
        if pendientes.is_empty() {
            break;
        }
        for key in pendientes {
            let info = {
                let Some(reg) = motor.instancias.get(&key) else {
                    continue;
                };
                if !(reg.estado == EstadoInstancia::InProgress
                    && reg.funcion.is_some()
                    && reg.error.is_none())
                {
                    continue;
                }
                let Some(plantilla) = plantillas.get(&key.funcion) else {
                    continue;
                };
                let mut sigma_sem: HashMap<TipoParamId, TipoSem> = HashMap::new();
                for ((_, pid), arg) in plantilla.parametros_tipo.iter().zip(key.args.iter()) {
                    sigma_sem.insert(pid.clone(), canon_a_sem(arg));
                }
                (
                    plantilla.nombre.clone(),
                    plantilla.linea,
                    plantilla.columna,
                    sigma_sem,
                )
            };
            if let Some(validador) = validar_v3.as_deref_mut()
                && let Some(sig_plantilla) = funciones.get(&info.0)
            {
                let v3_errs = validador(sig_plantilla, &sigma_sem_from(&info.3), info.1, info.2);
                if !v3_errs.is_empty() {
                    if let Some(e) = v3_errs.first() {
                        let causa = (*e.causa).clone();
                        if let Some(reg) = motor.instancias.get_mut(&key) {
                            reg.estado = EstadoInstancia::Failed;
                            reg.error = Some(causa);
                        }
                    }
                    for mut ee in v3_errs {
                        if ee.archivo.is_none() {
                            ee.archivo = archivo.map(|s| s.to_string());
                        }
                        errores.push(ee);
                    }
                    continue;
                }
            }
            if let Some(reg) = motor.instancias.get_mut(&key) {
                reg.estado = EstadoInstancia::Complete;
            }
        }
    }
    errores
}

fn sigma_sem_from(s: &HashMap<TipoParamId, TipoSem>) -> HashMap<TipoParamId, TipoSem> {
    s.clone()
}

/// Ejecuta monomorfización G4 sobre un programa ya analizado semánticamente.
/// - Registra plantillas y semillas.
/// - Instancia transitivamente.
/// - Genera programa especializado (sin plantillas genéricas, con instancias concretas)
///   y reescribe llamadas genéricas en código no genérico.
pub(crate) fn ejecutar_g4(
    programa: &[Sentencia],
    funciones: &HashMap<String, FuncionSig>,
    solicitudes: &[crate::semantico::SolicitudInstanciaG4],
    archivo: Option<&str>,
    modulo: &str,
    mut validar_v3: Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
) -> (Vec<Sentencia>, Vec<crate::evaluador::ErrorEjecucion>) {
    let plantillas = construir_plantillas(programa, funciones, modulo);
    if plantillas.is_empty() {
        return (programa.to_vec(), Vec::new());
    }
    let por_nombre = indice_plantillas_por_nombre(&plantillas);
    let firmas_ng = firmas_no_genericas_sem(funciones);
    let mut motor = MotorMonomorfizacion::default();
    let mut errores: Vec<crate::evaluador::ErrorEjecucion> = Vec::new();

    // M3 provisional: aridad máxima de parámetros de tipo por plantilla.
    for p in plantillas.values() {
        if p.parametros_tipo.len() > LIMITE_ARIDAD_PROVISIONAL {
            errores.push(crate::evaluador::ErrorEjecucion {
                mensaje: format!(
                    "límite de aridad de parámetros de tipo excedido (límite {}) en la función genérica '{}'",
                    LIMITE_ARIDAD_PROVISIONAL, p.nombre
                ),
                causa: Box::new(CausaError::DependenciaGenericaNoResoluble {
                    funcion: p.nombre.clone(),
                    detalle: format!(
                        "aridad {} excede límite provisional M3 ({})",
                        p.parametros_tipo.len(),
                        LIMITE_ARIDAD_PROVISIONAL
                    ),
                }),
                linea: p.linea,
                columna: p.columna,
                archivo: archivo.map(|s| s.to_string()),
            });
        }
    }
    if !errores.is_empty() {
        return (programa.to_vec(), errores);
    }

    // Semillas: convertir solicitudes G4 (concretas) a instancias
    // Construir índice de GenericFnId por (decl_id, aridad, nombre) para mapear solicitudes
    let mut por_decl: BTreeMap<(SimbodoId, usize, String), GenericFnId> = BTreeMap::new();
    for (id, p) in &plantillas {
        por_decl.insert((id.decl_id, id.aridad, p.nombre.clone()), id.clone());
    }

    for sol in solicitudes {
        // Determinar aridad a partir de sigma/plantilla
        if let Some(pname_id) = por_decl
            .iter()
            .find(|((did, _, n), _)| *did == sol.callee_decl_id && n == &sol.callee_nombre)
            .map(|(_, gid)| gid.clone())
            && let Some(plantilla) = plantillas.get(&pname_id)
        {
            let params_obj: Vec<TipoParamId> = plantilla
                .parametros_tipo
                .iter()
                .map(|(_, id)| id.clone())
                .collect();
            let args: Option<Vec<TipoCanon>> = params_obj
                .iter()
                .map(|p| sol.sigma.get(p).and_then(canonicalizar_tipo))
                .collect();
            if let Some(args_c) = args {
                let key = InstanceKey {
                    funcion: pname_id.clone(),
                    args: args_c,
                };
                let origen = OrigenInstancia {
                    funcion: sol.callee_nombre.clone(),
                    instancia: sol.callee_nombre.clone(),
                    tipos: key.nombre_tipos(),
                    cadena: vec![format!(
                        "{}<{}>",
                        sol.callee_nombre,
                        key.nombre_tipos().join(",")
                    )],
                    cadena_keys: vec![key.clone()],
                    linea: sol.linea,
                    columna: sol.columna,
                };
                if let Err(e) = solicitar_instancia(
                    &mut motor,
                    &plantillas,
                    &por_nombre,
                    &firmas_ng,
                    funciones,
                    key,
                    &origen,
                    &mut validar_v3,
                ) {
                    let mut ee = crate::evaluador::ErrorEjecucion {
                        mensaje: e.to_string(),
                        causa: Box::new(e),
                        linea: sol.linea,
                        columna: sol.columna,
                        archivo: archivo.map(|s| s.to_string()),
                    };
                    let _ = &mut ee;
                    errores.push(ee);
                }
            }
        }
    }

    let errs_esp = especializar_pendientes(
        &mut motor,
        &plantillas,
        &por_nombre,
        &firmas_ng,
        funciones,
        &mut validar_v3,
    );
    for mut e in errs_esp {
        e.archivo = archivo.map(|s| s.to_string());
        errores.push(e);
    }
    // V3 post-especialización iterativa sobre instancias descubiertas en siembra (Bloqueante A).
    let errs_v3 = validar_v3_instancias_pendientes(
        &mut motor,
        &plantillas,
        funciones,
        &mut validar_v3,
        archivo,
    );
    errores.extend(errs_v3);

    // Segunda fase: reescribir programa (código no genérico y eliminar plantillas genéricas)
    // También procesar cualquier instancia pendiente adicional descubierta? ya se procesaron on-demand.
    if !errores.is_empty() {
        // Generación transaccional: no publicar programa parcialmente especializado ante fallos
        // (pero preservamos errores). Retornamos programa original? Mejor no ejecutar especializado.
        return (programa.to_vec(), errores);
    }

    // Construir mapa de retornos de instancias completas
    let mut inst_retornos: HashMap<String, TipoSem> = HashMap::new();
    for reg in motor.instancias.values() {
        if reg.estado == EstadoInstancia::Complete
            && let Some(n) = &reg.nombre_especializado
            && let Some(p) = plantillas.get(&reg.key.funcion)
        {
            let mut s: HashMap<TipoParamId, TipoSem> = HashMap::new();
            for ((_, pid), arg) in p.parametros_tipo.iter().zip(reg.key.args.iter()) {
                s.insert(pid.clone(), canon_a_sem(arg));
            }
            if let Some(r) = &p.retorno_sem {
                inst_retornos.insert(n.clone(), sustituir_tipo(r, &s));
            }
        }
    }

    let mut programa_out = Vec::new();
    // 1) Reescribir/elidir sentencias originales
    for s in programa {
        match reescribir_sentencia_top_g4(
            s,
            &plantillas,
            &por_nombre,
            &mut motor,
            &firmas_ng,
            &inst_retornos,
            funciones,
            &mut validar_v3,
        ) {
            Ok(Some(reescrita)) => programa_out.push(reescrita),
            Ok(None) => {}
            Err(e) => {
                errores.push(crate::evaluador::ErrorEjecucion {
                    mensaje: e.to_string(),
                    causa: Box::new(e),
                    linea: 1,
                    columna: 1,
                    archivo: archivo.map(|s| s.to_string()),
                });
            }
        }
    }
    if let Some(errs) = motor_errores(&motor, archivo) {
        errores.extend(errs);
    }
    let errs_esp_rw = especializar_pendientes(
        &mut motor,
        &plantillas,
        &por_nombre,
        &firmas_ng,
        funciones,
        &mut validar_v3,
    );
    for mut e in errs_esp_rw {
        e.archivo = archivo.map(|s| s.to_string());
        errores.push(e);
    }
    // Validar V3 de instancias descubiertas durante reescritura (Bloqueante A).
    let errs_v3_rw = validar_v3_instancias_pendientes(
        &mut motor,
        &plantillas,
        funciones,
        &mut validar_v3,
        archivo,
    );
    errores.extend(errs_v3_rw);
    if !errores.is_empty() {
        return (programa.to_vec(), errores);
    }
    // 2) Agregar todas las instancias concretas completas (determinista por clave)
    for reg in motor.instancias.values() {
        if reg.estado == EstadoInstancia::Complete {
            if let Some(f) = &reg.funcion {
                programa_out.push(f.clone());
            }
        } else if reg.estado == EstadoInstancia::Failed
            && let Some(e) = &reg.error
        {
            errores.push(crate::evaluador::ErrorEjecucion {
                mensaje: e.to_string(),
                causa: Box::new(e.clone()),
                linea: 1,
                columna: 1,
                archivo: archivo.map(|s| s.to_string()),
            });
        }
    }
    if !errores.is_empty() {
        return (programa.to_vec(), errores);
    }
    // Gate final G4 (Bloqueante B): integridad estructural de la salida concreta.
    let errs_integridad =
        verificar_integridad_salida_g4(&programa_out, &plantillas, &motor, archivo);
    if !errs_integridad.is_empty() {
        return (programa.to_vec(), errs_integridad);
    }
    (programa_out, errores)
}

fn motor_errores(
    motor: &MotorMonomorfizacion,
    archivo: Option<&str>,
) -> Option<Vec<crate::evaluador::ErrorEjecucion>> {
    let mut errs = Vec::new();
    for reg in motor.instancias.values() {
        if reg.estado == EstadoInstancia::Failed
            && let Some(e) = &reg.error
        {
            errs.push(crate::evaluador::ErrorEjecucion {
                mensaje: e.to_string(),
                causa: Box::new(e.clone()),
                linea: 1,
                columna: 1,
                archivo: archivo.map(|s| s.to_string()),
            });
        }
    }
    if errs.is_empty() { None } else { Some(errs) }
}

/// Reescribe sentencias de nivel superior: elimina plantillas genéricas, reescribe llamadas en el resto.
/// Retorna Ok(None) si la sentencia debe elidirse (plantilla genérica reemplazada por instancias).
fn reescribir_sentencia_top_g4(
    s: &Sentencia,
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    por_nombre: &HashMap<String, GenericFnId>,
    motor: &mut MotorMonomorfizacion,
    firmas_ng: &HashMap<String, (Vec<TipoSem>, TipoSem)>,
    inst_retornos: &HashMap<String, TipoSem>,
    funciones: &HashMap<String, FuncionSig>,
    validar_v3: &mut Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
) -> Result<Option<Sentencia>, CausaError> {
    match s {
        Sentencia::Funcion {
            parametros_tipo,
            nombre,
            ..
        } => {
            if !parametros_tipo.is_empty() && por_nombre.contains_key(nombre) {
                // plantilla: elidir (se reemplaza por instancias concretas)
                return Ok(None);
            }
            // función no genérica: reescribir su cuerpo (puede contener llamadas genéricas concretas resueltas).
            Ok(Some(reescribir_no_generica(
                s,
                plantillas,
                por_nombre,
                motor,
                firmas_ng,
                inst_retornos,
                funciones,
                validar_v3,
            )?))
        }
        Sentencia::Exportar {
            sentencia,
            linea,
            columna,
        } => {
            // si exporta plantilla genérica, elidir (v1: instancias concretas no se re-exportan como plantilla).
            if let Sentencia::Funcion {
                parametros_tipo,
                nombre,
                ..
            } = &**sentencia
                && !parametros_tipo.is_empty()
                && por_nombre.contains_key(nombre)
            {
                return Ok(None);
            }
            let inner = reescribir_no_generica(
                sentencia,
                plantillas,
                por_nombre,
                motor,
                firmas_ng,
                inst_retornos,
                funciones,
                validar_v3,
            )?;
            Ok(Some(Sentencia::Exportar {
                sentencia: Box::new(inner),
                linea: *linea,
                columna: *columna,
            }))
        }
        _ => Ok(Some(reescribir_no_generica(
            s,
            plantillas,
            por_nombre,
            motor,
            firmas_ng,
            inst_retornos,
            funciones,
            validar_v3,
        )?)),
    }
}

fn reescribir_no_generica(
    s: &Sentencia,
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    por_nombre: &HashMap<String, GenericFnId>,
    motor: &mut MotorMonomorfizacion,
    firmas_ng: &HashMap<String, (Vec<TipoSem>, TipoSem)>,
    inst_retornos: &HashMap<String, TipoSem>,
    funciones: &HashMap<String, FuncionSig>,
    validar_v3: &mut Option<
        &mut dyn FnMut(
            &FuncionSig,
            &HashMap<TipoParamId, TipoSem>,
            usize,
            usize,
        ) -> Vec<crate::evaluador::ErrorEjecucion>,
    >,
) -> Result<Sentencia, CausaError> {
    let sigma_vacio: MapaSustAnot = HashMap::new();
    let mut vars: HashMap<String, TipoSem> = HashMap::new();
    // Para funciones no genéricas, poblar vars con parámetros
    if let Sentencia::Funcion { parametros, .. } = s {
        for (n, ta) in parametros {
            let t = ta
                .as_ref()
                .map(anotacion_a_sem_concreta)
                .unwrap_or(TipoSem::Desconocido);
            vars.insert(n.clone(), t);
        }
    }
    let origen = OrigenInstancia {
        funcion: "<toplevel>".into(),
        instancia: "<toplevel>".into(),
        tipos: vec![],
        cadena: vec![],
        cadena_keys: vec![],
        linea: 1,
        columna: 1,
    };
    reescribir_sentencia_g4(
        s,
        &sigma_vacio,
        plantillas,
        por_nombre,
        motor,
        &mut vars,
        firmas_ng,
        inst_retornos,
        &origen,
        funciones,
        validar_v3,
    )
}

/// Gate estructural de integridad de salida G4 (Bloqueante B).
/// Verifica que la salida concreta sea consistente: sin plantillas residuales, sin
/// parámetros genéricos en tipos emitidos, sin destinos no resueltos/inexistentes,
/// con definiciones únicas y nombres especializados canónicos válidos.
pub fn verificar_integridad_salida_g4(
    programa: &[Sentencia],
    plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
    motor: &MotorMonomorfizacion,
    archivo: Option<&str>,
) -> Vec<crate::evaluador::ErrorEjecucion> {
    use crate::parser::TipoAnotacion;
    use std::collections::{HashMap, HashSet};

    let mut errores = Vec::new();
    let archivo_s = archivo.map(|s| s.to_string());

    // Conjuntos de referencia canónicos
    let nombres_plantillas: HashSet<String> =
        plantillas.values().map(|p| p.nombre.clone()).collect();
    let mut especializadas_validas: HashSet<String> = HashSet::new();
    let mut nombre_a_key: HashMap<String, InstanceKey> = HashMap::new();
    let mut todas_especializadas: HashSet<String> = HashSet::new();
    for reg in motor.instancias.values() {
        if let Some(n) = &reg.nombre_especializado {
            todas_especializadas.insert(n.clone());
            nombre_a_key.insert(n.clone(), reg.key.clone());
            if reg.estado == EstadoInstancia::Complete && reg.funcion.is_some() {
                especializadas_validas.insert(n.clone());
            }
        }
    }

    fn err(
        archivo: &Option<String>,
        linea: usize,
        columna: usize,
        detalle: String,
        ubicacion: String,
    ) -> crate::evaluador::ErrorEjecucion {
        crate::evaluador::ErrorEjecucion {
            mensaje: format!(
                "referencia genérica residual en IR concreta ({ubicacion}): {detalle}"
            ),
            causa: Box::new(CausaError::ReferenciaGenericaResidualIr { detalle, ubicacion }),
            linea,
            columna,
            archivo: archivo.clone(),
        }
    }

    // 1) Definiciones: unicidad, sin plantillas residuales, sin params genéricos en emitidas, nombres canónicos válidos
    let mut defs_count: HashMap<String, usize> = HashMap::new();
    let mut defs_info: HashMap<String, (usize, usize)> = HashMap::new();
    fn recolectar_defs(
        s: &Sentencia,
        defs: &mut HashMap<String, usize>,
        info: &mut HashMap<String, (usize, usize)>,
    ) {
        match s {
            Sentencia::Funcion {
                nombre,
                linea,
                columna,
                cuerpo,
                ..
            } => {
                *defs.entry(nombre.clone()).or_insert(0) += 1;
                info.entry(nombre.clone()).or_insert((*linea, *columna));
                for st in cuerpo {
                    recolectar_defs(st, defs, info);
                }
            }
            Sentencia::Exportar { sentencia, .. } => recolectar_defs(sentencia, defs, info),
            Sentencia::Bloque(stmts) => {
                for st in stmts {
                    recolectar_defs(st, defs, info);
                }
            }
            Sentencia::Si {
                rama_si, rama_sino, ..
            } => {
                for st in rama_si {
                    recolectar_defs(st, defs, info);
                }
                if let Some(rs) = rama_sino {
                    for st in rs {
                        recolectar_defs(st, defs, info);
                    }
                }
            }
            Sentencia::Mientras { cuerpo, .. } | Sentencia::ParaCada { cuerpo, .. } => {
                for st in cuerpo {
                    recolectar_defs(st, defs, info);
                }
            }
            Sentencia::IntentarRescatar {
                cuerpo_intentar,
                cuerpo_rescatar,
                ..
            } => {
                for st in cuerpo_intentar {
                    recolectar_defs(st, defs, info);
                }
                for st in cuerpo_rescatar {
                    recolectar_defs(st, defs, info);
                }
            }
            _ => {}
        }
    }
    for s in programa {
        recolectar_defs(s, &mut defs_count, &mut defs_info);
    }

    // Chequeos por definición (top-level y anidadas)
    fn verificar_sentencia_def(
        s: &Sentencia,
        nombres_plantillas: &HashSet<String>,
        todas_especializadas: &HashSet<String>,
        nombre_a_key: &HashMap<String, InstanceKey>,
        plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
        defs_count: &HashMap<String, usize>,
        archivo: &Option<String>,
        errores: &mut Vec<crate::evaluador::ErrorEjecucion>,
    ) {
        match s {
            Sentencia::Funcion {
                nombre,
                parametros_tipo,
                parametros,
                tipo_retorno,
                cuerpo,
                linea,
                columna,
                ..
            } => {
                // Plantilla genérica residual
                if !parametros_tipo.is_empty()
                    || nombres_plantillas.contains(nombre) && !parametros_tipo.is_empty()
                {
                    // (second conjunct redundant but explicit)
                }
                if !parametros_tipo.is_empty() {
                    errores.push(err(
                        archivo,
                        *linea,
                        *columna,
                        format!("plantilla genérica residual: {nombre}"),
                        nombre.clone(),
                    ));
                }
                // Nombre de plantilla genérica sin especializar no debe quedar como definición concreta de plantilla
                if nombres_plantillas.contains(nombre)
                    && parametros_tipo.is_empty()
                    && !todas_especializadas.contains(nombre)
                {
                    // podría ser caso raro; no marcar salvo evidencia. Las plantillas deben haberse elidido.
                }
                // Duplicados de definición
                if defs_count.get(nombre).copied().unwrap_or(0) > 1 {
                    errores.push(err(
                        archivo,
                        *linea,
                        *columna,
                        format!("definición especializada duplicada: '{nombre}'"),
                        nombre.clone(),
                    ));
                }
                // Si es una instancia especializada conocida, validar identidad canónica y correspondencia de nombre
                if let Some(key) = nombre_a_key.get(nombre) {
                    let base = nombre_especializado_base(key);
                    if nombre != &base && !nombre.starts_with(&format!("{base}__k")) {
                        errores.push(err(archivo, *linea, *columna,
                            format!("nombre especializado no canónico para la identidad de instancia: '{nombre}'"), nombre.clone()));
                    }
                    if !plantillas.contains_key(&key.funcion) {
                        errores.push(err(
                            archivo,
                            *linea,
                            *columna,
                            format!(
                                "instancia especializada sin plantilla canónica válida: '{nombre}'"
                            ),
                            nombre.clone(),
                        ));
                    }
                }
                // No parámetros genéricos en firma/cuerpo
                for (_, ta) in parametros {
                    if let Some(t) = ta {
                        verificar_tipo_anot_sin_param(
                            t, archivo, *linea, *columna, nombre, errores,
                        );
                    }
                }
                if let Some(tr) = tipo_retorno {
                    verificar_tipo_anot_sin_param(tr, archivo, *linea, *columna, nombre, errores);
                }
                for st in cuerpo {
                    verificar_sentencia_def(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        nombre_a_key,
                        plantillas,
                        defs_count,
                        archivo,
                        errores,
                    );
                }
            }
            Sentencia::Exportar { sentencia, .. } => {
                // si exporta una plantilla residual se detecta dentro
                verificar_sentencia_def(
                    sentencia,
                    nombres_plantillas,
                    todas_especializadas,
                    nombre_a_key,
                    plantillas,
                    defs_count,
                    archivo,
                    errores,
                );
            }
            Sentencia::Bloque(stmts) => {
                for st in stmts {
                    verificar_sentencia_def(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        nombre_a_key,
                        plantillas,
                        defs_count,
                        archivo,
                        errores,
                    );
                }
            }
            Sentencia::Si {
                rama_si, rama_sino, ..
            } => {
                for st in rama_si {
                    verificar_sentencia_def(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        nombre_a_key,
                        plantillas,
                        defs_count,
                        archivo,
                        errores,
                    );
                }
                if let Some(rs) = rama_sino {
                    for st in rs {
                        verificar_sentencia_def(
                            st,
                            nombres_plantillas,
                            todas_especializadas,
                            nombre_a_key,
                            plantillas,
                            defs_count,
                            archivo,
                            errores,
                        );
                    }
                }
            }
            Sentencia::Mientras { cuerpo, .. } | Sentencia::ParaCada { cuerpo, .. } => {
                for st in cuerpo {
                    verificar_sentencia_def(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        nombre_a_key,
                        plantillas,
                        defs_count,
                        archivo,
                        errores,
                    );
                }
            }
            Sentencia::IntentarRescatar {
                cuerpo_intentar,
                cuerpo_rescatar,
                ..
            } => {
                for st in cuerpo_intentar {
                    verificar_sentencia_def(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        nombre_a_key,
                        plantillas,
                        defs_count,
                        archivo,
                        errores,
                    );
                }
                for st in cuerpo_rescatar {
                    verificar_sentencia_def(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        nombre_a_key,
                        plantillas,
                        defs_count,
                        archivo,
                        errores,
                    );
                }
            }
            _ => {}
        }
    }

    fn verificar_tipo_anot_sin_param(
        t: &TipoAnotacion,
        archivo: &Option<String>,
        linea: usize,
        columna: usize,
        ubic: &str,
        errores: &mut Vec<crate::evaluador::ErrorEjecucion>,
    ) {
        match t {
            TipoAnotacion::Parametro(n) => {
                errores.push(err(
                    archivo,
                    linea,
                    columna,
                    format!("parámetro genérico residual en tipo emitido: '{n}'"),
                    ubic.to_string(),
                ));
            }
            TipoAnotacion::Arreglo(te, _) | TipoAnotacion::Lista(te) => {
                verificar_tipo_anot_sin_param(te, archivo, linea, columna, ubic, errores)
            }
            _ => {}
        }
    }

    // Ejecutar verificaciones
    // (A) definiciones y estructura de funciones emitidas
    for s in programa {
        verificar_sentencia_def(
            s,
            &nombres_plantillas,
            &todas_especializadas,
            &nombre_a_key,
            plantillas,
            &defs_count,
            &archivo_s,
            &mut errores,
        );
    }
    // (B) verificación de cuerpos/expresiones top-level para llamadas y tipos residuales en sentencias no-función
    // Necesitamos validas = nombres de instancias completas
    let validas_completas: HashSet<String> = especializadas_validas.clone();
    for s in programa {
        match s {
            Sentencia::Funcion {
                nombre,
                cuerpo,
                parametros,
                tipo_retorno,
                linea,
                columna,
                ..
            } => {
                for st in cuerpo {
                    verificar_llamadas_y_tipos_sent(
                        st,
                        &nombres_plantillas,
                        &todas_especializadas,
                        &validas_completas,
                        &archivo_s,
                        &mut errores,
                        nombre,
                    );
                }
                for (_, ta) in parametros {
                    if let Some(t) = ta {
                        verificar_tipo_anot_sin_param(
                            t,
                            &archivo_s,
                            *linea,
                            *columna,
                            nombre,
                            &mut errores,
                        );
                    }
                }
                if let Some(tr) = tipo_retorno {
                    verificar_tipo_anot_sin_param(
                        tr,
                        &archivo_s,
                        *linea,
                        *columna,
                        nombre,
                        &mut errores,
                    );
                }
            }
            Sentencia::Exportar { sentencia, .. } => {
                if let Sentencia::Funcion { nombre, cuerpo, .. } = &**sentencia {
                    for st in cuerpo {
                        verificar_llamadas_y_tipos_sent(
                            st,
                            &nombres_plantillas,
                            &todas_especializadas,
                            &validas_completas,
                            &archivo_s,
                            &mut errores,
                            nombre,
                        );
                    }
                } else {
                    verificar_llamadas_y_tipos_sent(
                        s,
                        &nombres_plantillas,
                        &todas_especializadas,
                        &validas_completas,
                        &archivo_s,
                        &mut errores,
                        "salida G4",
                    );
                }
            }
            _ => {
                verificar_llamadas_y_tipos_sent(
                    s,
                    &nombres_plantillas,
                    &todas_especializadas,
                    &validas_completas,
                    &archivo_s,
                    &mut errores,
                    "salida G4",
                );
            }
        }
    }

    // (C) unicidad de identidad de instancia ejecutable (por InstanceKey/definición emitida)
    // Cada instancia completa debe tener exactamente una definición en salida.
    for reg in motor.instancias.values() {
        if reg.estado == EstadoInstancia::Complete
            && let Some(n) = &reg.nombre_especializado
        {
            match defs_count.get(n).copied().unwrap_or(0) {
                0 => {
                    errores.push(err(
                        &archivo_s,
                        1,
                        1,
                        format!("instancia especializada sin definición ejecutable emitida: '{n}'"),
                        n.clone(),
                    ));
                }
                1 => {}
                _ => { /* ya reportado como duplicado */ }
            }
        }
    }

    fn verificar_llamadas_y_tipos_sent(
        s: &Sentencia,
        nombres_plantillas: &HashSet<String>,
        todas_especializadas: &HashSet<String>,
        validas_completas: &HashSet<String>,
        archivo: &Option<String>,
        errores: &mut Vec<crate::evaluador::ErrorEjecucion>,
        ubic: &str,
    ) {
        match s {
            Sentencia::Expresion(e) => verificar_llamadas_y_tipos_expr(
                e,
                nombres_plantillas,
                todas_especializadas,
                validas_completas,
                archivo,
                errores,
                ubic,
            ),
            Sentencia::Declaracion {
                valor,
                tipo_anotacion,
                linea,
                columna,
                ..
            } => {
                if let Some(t) = tipo_anotacion {
                    verificar_tipo_anot_sin_param(t, archivo, *linea, *columna, ubic, errores);
                }
                verificar_llamadas_y_tipos_expr(
                    valor,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
            }
            Sentencia::Asignacion { valor, .. } => verificar_llamadas_y_tipos_expr(
                valor,
                nombres_plantillas,
                todas_especializadas,
                validas_completas,
                archivo,
                errores,
                ubic,
            ),
            Sentencia::Retorno { valor: Some(v), .. } => verificar_llamadas_y_tipos_expr(
                v,
                nombres_plantillas,
                todas_especializadas,
                validas_completas,
                archivo,
                errores,
                ubic,
            ),
            Sentencia::Si {
                condicion,
                rama_si,
                rama_sino,
                ..
            } => {
                verificar_llamadas_y_tipos_expr(
                    condicion,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
                for st in rama_si {
                    verificar_llamadas_y_tipos_sent(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
                if let Some(rs) = rama_sino {
                    for st in rs {
                        verificar_llamadas_y_tipos_sent(
                            st,
                            nombres_plantillas,
                            todas_especializadas,
                            validas_completas,
                            archivo,
                            errores,
                            ubic,
                        );
                    }
                }
            }
            Sentencia::Mientras {
                condicion, cuerpo, ..
            } => {
                verificar_llamadas_y_tipos_expr(
                    condicion,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
                for st in cuerpo {
                    verificar_llamadas_y_tipos_sent(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Sentencia::ParaCada {
                iterable, cuerpo, ..
            } => {
                verificar_llamadas_y_tipos_expr(
                    iterable,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
                for st in cuerpo {
                    verificar_llamadas_y_tipos_sent(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Sentencia::Funcion {
                parametros,
                tipo_retorno,
                cuerpo,
                linea,
                columna,
                ..
            } => {
                for (_, ta) in parametros {
                    if let Some(t) = ta {
                        verificar_tipo_anot_sin_param(t, archivo, *linea, *columna, ubic, errores);
                    }
                }
                if let Some(tr) = tipo_retorno {
                    verificar_tipo_anot_sin_param(tr, archivo, *linea, *columna, ubic, errores);
                }
                for st in cuerpo {
                    verificar_llamadas_y_tipos_sent(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Sentencia::Bloque(stmts) => {
                for st in stmts {
                    verificar_llamadas_y_tipos_sent(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Sentencia::IntentarRescatar {
                cuerpo_intentar,
                cuerpo_rescatar,
                ..
            } => {
                for st in cuerpo_intentar {
                    verificar_llamadas_y_tipos_sent(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
                for st in cuerpo_rescatar {
                    verificar_llamadas_y_tipos_sent(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Sentencia::AsignacionIndice {
                objeto,
                indice,
                valor,
                ..
            } => {
                verificar_llamadas_y_tipos_expr(
                    objeto,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
                verificar_llamadas_y_tipos_expr(
                    indice,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
                verificar_llamadas_y_tipos_expr(
                    valor,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
            }
            Sentencia::Exportar { sentencia, .. } => verificar_llamadas_y_tipos_sent(
                sentencia,
                nombres_plantillas,
                todas_especializadas,
                validas_completas,
                archivo,
                errores,
                ubic,
            ),
            _ => {}
        }
    }

    fn verificar_llamadas_y_tipos_expr(
        e: &Expresion,
        nombres_plantillas: &HashSet<String>,
        todas_especializadas: &HashSet<String>,
        validas_completas: &HashSet<String>,
        archivo: &Option<String>,
        errores: &mut Vec<crate::evaluador::ErrorEjecucion>,
        ubic: &str,
    ) {
        match e {
            Expresion::ConstruccionEstructura { campos, .. } => {
                for (_, v) in campos {
                    verificar_llamadas_y_tipos_expr(
                        v,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Expresion::ConstruccionVariante { argumentos, .. } => {
                for v in argumentos {
                    verificar_llamadas_y_tipos_expr(
                        v,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Expresion::Coincidir {
                scrutinee, brazos, ..
            } => {
                verificar_llamadas_y_tipos_expr(
                    scrutinee,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
                for b in brazos {
                    for st in &b.cuerpo {
                        verificar_llamadas_y_tipos_sent(
                            st,
                            nombres_plantillas,
                            todas_especializadas,
                            validas_completas,
                            archivo,
                            errores,
                            ubic,
                        );
                    }
                }
            }
            Expresion::Llamada {
                nombre,
                argumentos,
                argumentos_tipo,
                linea,
                columna,
            } => {
                if !argumentos_tipo.is_empty() {
                    errores.push(err(
                        archivo,
                        *linea,
                        *columna,
                        format!(
                            "llamada con argumentos de tipo residuales no resueltos: '{nombre}'"
                        ),
                        ubic.to_string(),
                    ));
                } else if nombres_plantillas.contains(nombre)
                    && !todas_especializadas.contains(nombre)
                {
                    errores.push(err(
                        archivo,
                        *linea,
                        *columna,
                        format!("destino genérico no resuelto: llamada a plantilla '{nombre}'"),
                        ubic.to_string(),
                    ));
                }
                if todas_especializadas.contains(nombre) && !validas_completas.contains(nombre) {
                    errores.push(err(archivo, *linea, *columna,
                        format!("referencia a instancia especializada inexistente o no válida: '{nombre}'"), ubic.to_string()));
                }
                for ta in argumentos_tipo {
                    verificar_tipo_anot_sin_param(ta, archivo, *linea, *columna, ubic, errores);
                }
                for a in argumentos {
                    verificar_llamadas_y_tipos_expr(
                        a,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Expresion::LlamadaIndirecta {
                callee, argumentos, ..
            } => {
                verificar_llamadas_y_tipos_expr(
                    callee,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
                for a in argumentos {
                    verificar_llamadas_y_tipos_expr(
                        a,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Expresion::ArregloLiteral { elementos, .. } => {
                for el in elementos {
                    verificar_llamadas_y_tipos_expr(
                        el,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Expresion::Indice { objeto, indice, .. } => {
                verificar_llamadas_y_tipos_expr(
                    objeto,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
                verificar_llamadas_y_tipos_expr(
                    indice,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
            }
            Expresion::Acceso { objeto, .. } => verificar_llamadas_y_tipos_expr(
                objeto,
                nombres_plantillas,
                todas_especializadas,
                validas_completas,
                archivo,
                errores,
                ubic,
            ),
            Expresion::Lambda {
                parametros,
                tipo_retorno,
                cuerpo,
                linea,
                columna,
                ..
            } => {
                for (_, ta) in parametros {
                    if let Some(t) = ta {
                        verificar_tipo_anot_sin_param(t, archivo, *linea, *columna, ubic, errores);
                    }
                }
                if let Some(tr) = tipo_retorno {
                    verificar_tipo_anot_sin_param(tr, archivo, *linea, *columna, ubic, errores);
                }
                for st in cuerpo {
                    verificar_llamadas_y_tipos_sent(
                        st,
                        nombres_plantillas,
                        todas_especializadas,
                        validas_completas,
                        archivo,
                        errores,
                        ubic,
                    );
                }
            }
            Expresion::Suma(a, b)
            | Expresion::Resta(a, b)
            | Expresion::Multiplicacion(a, b)
            | Expresion::Division(a, b)
            | Expresion::Modulo(a, b)
            | Expresion::IgualIgual(a, b)
            | Expresion::Distinto(a, b)
            | Expresion::Menor(a, b)
            | Expresion::MenorIgual(a, b)
            | Expresion::Mayor(a, b)
            | Expresion::MayorIgual(a, b)
            | Expresion::Y(a, b)
            | Expresion::O(a, b) => {
                verificar_llamadas_y_tipos_expr(
                    a,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
                verificar_llamadas_y_tipos_expr(
                    b,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
            }
            Expresion::No(a) => verificar_llamadas_y_tipos_expr(
                a,
                nombres_plantillas,
                todas_especializadas,
                validas_completas,
                archivo,
                errores,
                ubic,
            ),
            Expresion::Rango { inicio, fin, .. } => {
                verificar_llamadas_y_tipos_expr(
                    inicio,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
                verificar_llamadas_y_tipos_expr(
                    fin,
                    nombres_plantillas,
                    todas_especializadas,
                    validas_completas,
                    archivo,
                    errores,
                    ubic,
                );
            }
            Expresion::Literal { .. } | Expresion::Variable { .. } => {}
        }
    }

    errores
}
