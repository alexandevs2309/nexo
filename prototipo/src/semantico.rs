//! Analizador semántico estático de BASILISCO.
//!
//! Recorre el AST sin ejecutarlo y valida ámbitos, variables, funciones y
//! tipos cuando pueden determinarse estáticamente. Reutiliza `CausaError`
//! y `ErrorEjecucion` del evaluador para emitir diagnósticos uniformes.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::Valor;
use crate::cargador::CargadorModulos;
use crate::evaluador::{CausaError, ErrorEjecucion};
use crate::parser::{Expresion, Sentencia, TipoAnotacion};
use crate::simbodo::{AmbitoId, CategoriaSimbodo, GestorAmbitos, SimbodoId, TablaSimbodo};

/// Tipo semántico determinado estáticamente. `Desconocido` representa
/// información no disponible (por ejemplo, llamadas a funciones sin
/// anotación o variables `sea` legacy cuyo tipo se infiere en ejecución).
#[derive(Debug, Clone, PartialEq)]
pub enum TipoSem {
    Entero,
    Decimal,
    Real,
    Logico,
    Texto,
    Arreglo(Box<TipoSem>, usize),
    Lista(Box<TipoSem>),
    /// Tipo estático aproximado para literales `[...]` cuyo contenedor
    /// concreto (arreglo o lista) se resuelve en ejecución.
    Coleccion(Box<TipoSem>),
    Funcion {
        params: Vec<TipoSem>,
        retorno: Box<TipoSem>,
    },
    Nulo,
    Desconocido,
}

impl TipoSem {
    pub fn nombre(&self) -> String {
        match self {
            TipoSem::Entero => "entero".to_string(),
            TipoSem::Decimal => "decimal".to_string(),
            TipoSem::Real => "real".to_string(),
            TipoSem::Logico => "lógico".to_string(),
            TipoSem::Texto => "texto".to_string(),
            TipoSem::Arreglo(t, n) => format!("arreglo<{}, {}>", t.nombre(), n),
            TipoSem::Lista(t) => format!("lista<{}>", t.nombre()),
            TipoSem::Coleccion(t) => format!("colección<{}>", t.nombre()),
            TipoSem::Funcion { params, retorno } => {
                let ps: Vec<String> = params.iter().map(|t| t.nombre()).collect();
                format!("función({}) -> {}", ps.join(", "), retorno.nombre())
            }
            TipoSem::Nulo => "nulo".to_string(),
            TipoSem::Desconocido => "desconocido".to_string(),
        }
    }

    fn estatico(&self) -> &'static str {
        match self {
            TipoSem::Entero => "entero",
            TipoSem::Decimal => "decimal",
            TipoSem::Real => "real",
            TipoSem::Logico => "lógico",
            TipoSem::Texto => "texto",
            _ => "desconocido",
        }
    }
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub tipo: TipoSem,
    mutable: bool,
}

#[derive(Debug, Clone)]
struct FuncionSig {
    parametros: Vec<(String, Option<TipoSem>)>,
    retorno: Option<TipoSem>,
}

#[derive(Clone)]
enum ExportadoSem {
    Variable(TipoSem),
    Funcion(FuncionSig),
}

#[derive(Clone)]
struct ModuloInfo {
    exports: HashMap<String, ExportadoSem>,
    nombres: HashSet<String>,
}

/// Analizador semántico. Independiente del evaluador: no ejecuta código.
pub struct ContextoSemantico {
    pub archivo: Option<String>,
    cargador: Option<Arc<Mutex<CargadorModulos>>>,
    archivo_actual: Option<PathBuf>,
    scopes: Vec<HashMap<String, VarInfo>>,
    tabla: TablaSimbodo,
    gestor: GestorAmbitos,
    funciones: HashMap<String, FuncionSig>,
    errores: Vec<ErrorEjecucion>,
    funcion_actual: Option<String>,
    retorno_esperado: Option<TipoSem>,
    retorno_inferido: Option<TipoSem>,
    en_funcion: bool,
    modulos: HashMap<PathBuf, ModuloInfo>,
    en_carga: HashSet<PathBuf>,
}

impl ContextoSemantico {
    pub fn nuevo() -> Self {
        Self {
            archivo: None,
            cargador: None,
            archivo_actual: None,
            scopes: vec![HashMap::new()],
            tabla: TablaSimbodo::nuevo(),
            gestor: GestorAmbitos::nuevo(),
            funciones: HashMap::new(),
            errores: Vec::new(),
            funcion_actual: None,
            retorno_esperado: None,
            retorno_inferido: None,
            en_funcion: false,
            modulos: HashMap::new(),
            en_carga: HashSet::new(),
        }
    }

    pub fn con_cargador(cargador: Arc<Mutex<CargadorModulos>>, archivo: Option<String>) -> Self {
        let mut ctx = Self::nuevo();
        ctx.cargador = Some(cargador);
        ctx.archivo = archivo.clone();
        ctx.archivo_actual = archivo.map(PathBuf::from);
        ctx
    }

    pub fn analizar_programa(&mut self, sentencias: &[Sentencia]) -> Vec<ErrorEjecucion> {
        // Primer pase: registrar funciones (permite llamadas anticipadas y recursión).
        self.registrar_funciones(sentencias);
        for s in sentencias {
            self.analizar_sentencia(s.clone());
        }
        let mut errs = self.errores.clone();
        for e in &mut errs {
            if e.archivo.is_none() {
                e.archivo = self.archivo.clone();
            }
        }
        errs
    }

    fn error(&mut self, causa: CausaError, linea: usize, columna: usize) {
        let mut e = ErrorEjecucion {
            mensaje: causa.to_string(),
            causa: Box::new(causa),
            linea,
            columna,
            archivo: self.archivo.clone(),
        };
        // Mantener mensaje sincronizado (ya lo está via to_string).
        let _ = &mut e;
        self.errores.push(e);
    }

    fn registrar_funciones(&mut self, sentencias: &[Sentencia]) {
        for s in sentencias {
            match s {
                Sentencia::Funcion {
                    nombre,
                    parametros,
                    tipo_retorno,
                    ..
                } => {
                    let params = parametros
                        .iter()
                        .map(|(n, t)| (n.clone(), t.as_ref().map(tipo_anotacion_a_sem)))
                        .collect();
                    self.funciones.insert(
                        nombre.clone(),
                        FuncionSig {
                            parametros: params,
                            retorno: tipo_retorno.as_ref().map(tipo_anotacion_a_sem),
                        },
                    );
                    self.tabla.registrar(
                        nombre.clone(),
                        AmbitoId(0),
                        CategoriaSimbodo::Funcion,
                        false,
                    );
                }
                Sentencia::Exportar { sentencia, .. } => {
                    self.registrar_funciones(std::slice::from_ref(sentencia));
                }
                _ => {}
            }
        }
    }

    fn analizar_sentencia(&mut self, s: Sentencia) {
        match s {
            Sentencia::Declaracion {
                nombre,
                linea,
                columna,
                valor,
                tipo_anotacion,
                mutable,
                ..
            } => {
                if crate::evaluador::NOMBRES_RESERVADOS.contains(&nombre.as_str()) {
                    self.error(
                        CausaError::NombreReservado {
                            nombre: nombre.clone(),
                        },
                        linea,
                        columna,
                    );
                }
                if self.existe_en_ambito_actual(&nombre) {
                    self.error(
                        CausaError::DeclaracionDuplicada(nombre.clone()),
                        linea,
                        columna,
                    );
                }
                let valor_tipo = self.analizar_expresion(&valor);
                let tipo_final = if let Some(ta) = tipo_anotacion {
                    let t = tipo_anotacion_a_sem(&ta);
                    if !compatible(&t, &valor_tipo) && !matches!(valor_tipo, TipoSem::Desconocido) {
                        self.error(
                            CausaError::AsignacionIncompatible {
                                esperado: t.nombre(),
                                obtenido: valor_tipo.nombre(),
                            },
                            linea,
                            columna,
                        );
                    }
                    t
                } else {
                    valor_tipo
                };
                if !self.existe_en_ambito_actual(&nombre) {
                    self.tabla.registrar(
                        nombre.clone(),
                        self.gestor.actual(),
                        CategoriaSimbodo::Variable,
                        mutable,
                    );
                    self.scopes.last_mut().unwrap().insert(
                        nombre,
                        VarInfo {
                            tipo: tipo_final,
                            mutable,
                        },
                    );
                }
            }
            Sentencia::Asignacion {
                nombre,
                linea,
                columna,
                valor,
            } => {
                let info = self.buscar_variable(&nombre);
                match info {
                    None => {
                        self.error(
                            CausaError::VariableNoDefinida(nombre.clone()),
                            linea,
                            columna,
                        );
                        self.analizar_expresion(&valor);
                    }
                    Some(info) => {
                        if !info.mutable {
                            self.error(
                                CausaError::ReasignacionInmutable {
                                    nombre: nombre.clone(),
                                },
                                linea,
                                columna,
                            );
                        }
                        let vt = self.analizar_expresion(&valor);
                        if !compatible(&info.tipo, &vt)
                            && !matches!(vt, TipoSem::Desconocido)
                            && !matches!(info.tipo, TipoSem::Desconocido)
                        {
                            self.error(
                                CausaError::AsignacionIncompatible {
                                    esperado: info.tipo.nombre(),
                                    obtenido: vt.nombre(),
                                },
                                linea,
                                columna,
                            );
                        }
                    }
                }
            }
            Sentencia::AsignacionIndice {
                objeto,
                indice,
                valor,
                linea,
                columna,
            } => {
                self.analizar_expresion(&objeto);
                self.analizar_expresion(&indice);
                self.analizar_expresion(&valor);
                // no comprobaciones adicionales estáticas
                let _ = (linea, columna);
            }
            Sentencia::Si {
                condicion,
                rama_si,
                rama_sino,
                linea,
                columna,
            } => {
                let ct = self.analizar_expresion(&condicion);
                if !matches!(ct, TipoSem::Desconocido | TipoSem::Logico) {
                    self.error(
                        CausaError::CondicionNoLogica {
                            obtenido: ct.estatico(),
                        },
                        linea,
                        columna,
                    );
                }
                self.analizar_bloque(&rama_si);
                if let Some(sino) = rama_sino {
                    self.analizar_bloque(&sino);
                }
            }
            Sentencia::Mientras {
                condicion,
                cuerpo,
                linea,
                columna,
            } => {
                let ct = self.analizar_expresion(&condicion);
                if !matches!(ct, TipoSem::Desconocido | TipoSem::Logico) {
                    self.error(
                        CausaError::CondicionNoLogica {
                            obtenido: ct.estatico(),
                        },
                        linea,
                        columna,
                    );
                }
                self.analizar_bloque(&cuerpo);
            }
            Sentencia::ParaCada {
                variable,
                iterable,
                cuerpo,
                ..
            } => {
                self.analizar_expresion(&iterable);
                self.scopes.push(HashMap::new());
                self.gestor.entrar();
                self.scopes.last_mut().unwrap().insert(
                    variable,
                    VarInfo {
                        tipo: TipoSem::Desconocido,
                        mutable: true,
                    },
                );
                for st in &cuerpo {
                    self.analizar_sentencia(st.clone());
                }
                self.scopes.pop();
                self.gestor.salir();
            }
            Sentencia::Bloque(stmts) => {
                self.analizar_bloque(&stmts);
            }
            Sentencia::Expresion(e) => {
                self.analizar_expresion(&e);
            }
            Sentencia::Funcion {
                nombre,
                parametros,
                tipo_retorno,
                cuerpo,
                linea,
                columna,
                ..
            } => {
                // Consistente con el evaluador: redeclarar una función en el
                // mismo ámbito es error (E1003).
                if self.existe_en_ambito_actual(&nombre) {
                    self.error(
                        CausaError::DeclaracionDuplicada(nombre.clone()),
                        linea,
                        columna,
                    );
                }
                // Registrar la firma en las funciones para validar llamadas y,
                // si es anidada (dentro de otra función), también en el ámbito
                // actual, tal como hace el evaluador al ejecutar la declaración.
                let params: Vec<(String, Option<TipoSem>)> = parametros
                    .iter()
                    .map(|(n, t)| (n.clone(), t.as_ref().map(tipo_anotacion_a_sem)))
                    .collect();
                let firma = FuncionSig {
                    parametros: params.clone(),
                    retorno: tipo_retorno.as_ref().map(tipo_anotacion_a_sem),
                };
                let firma_previa = self.funciones.get(&nombre).cloned();
                self.funciones.insert(nombre.clone(), firma.clone());
                self.tabla.registrar(
                    nombre.clone(),
                    self.gestor.actual(),
                    CategoriaSimbodo::Funcion,
                    false,
                );
                let es_anidada = self.en_funcion;
                self.scopes.last_mut().unwrap().insert(
                    nombre.clone(),
                    VarInfo {
                        tipo: TipoSem::Funcion {
                            params: params
                                .iter()
                                .map(|(_, t)| t.clone().unwrap_or(TipoSem::Desconocido))
                                .collect(),
                            retorno: Box::new(
                                tipo_retorno
                                    .as_ref()
                                    .map(tipo_anotacion_a_sem)
                                    .unwrap_or(TipoSem::Desconocido),
                            ),
                        },
                        mutable: false,
                    },
                );
                self.scopes.push(HashMap::new());
                self.gestor.entrar();
                for (pname, pta) in &parametros {
                    let tipo = pta
                        .as_ref()
                        .map(tipo_anotacion_a_sem)
                        .unwrap_or(TipoSem::Desconocido);
                    self.tabla.registrar(
                        pname.clone(),
                        self.gestor.actual(),
                        CategoriaSimbodo::Parametro,
                        true,
                    );
                    self.scopes.last_mut().unwrap().insert(
                        pname.clone(),
                        VarInfo {
                            tipo,
                            mutable: true,
                        },
                    );
                }
                let prev_func = self.funcion_actual.clone();
                let prev_ret = self.retorno_esperado.clone();
                let prev_inf = self.retorno_inferido.clone();
                let prev_en = self.en_funcion;
                self.funcion_actual = Some(nombre.clone());
                self.retorno_esperado = tipo_retorno.as_ref().map(tipo_anotacion_a_sem);
                self.retorno_inferido = None;
                self.en_funcion = true;
                for st in &cuerpo {
                    self.analizar_sentencia(st.clone());
                }
                if tipo_retorno.is_none()
                    && let Some(inf) = &self.retorno_inferido
                    && let Some(sig) = self.funciones.get_mut(&nombre)
                {
                    sig.retorno = Some(inf.clone());
                }
                self.funcion_actual = prev_func;
                self.retorno_esperado = prev_ret;
                self.retorno_inferido = prev_inf;
                self.en_funcion = prev_en;
                self.scopes.pop();
                self.gestor.salir();
                // Al salir de una función anidada, devolver la firma global
                // anterior: el alcance de una función anidada es el cuerpo de
                // la función que la contiene, no todo el programa.
                if es_anidada {
                    match firma_previa {
                        Some(f) => {
                            self.funciones.insert(nombre, f);
                        }
                        None => {
                            self.funciones.remove(&nombre);
                        }
                    }
                }
            }
            Sentencia::Retorno {
                valor,
                linea,
                columna,
            } => {
                if !self.en_funcion {
                    self.error(CausaError::RetornarFueraDeBloque, linea, columna);
                    if let Some(v) = valor {
                        self.analizar_expresion(&v);
                    }
                    return;
                }
                let vt = match valor {
                    Some(v) => self.analizar_expresion(&v),
                    None => TipoSem::Nulo,
                };
                if let Some(expected) = &self.retorno_esperado
                    && !compatible(expected, &vt)
                    && !matches!(vt, TipoSem::Desconocido)
                    && !matches!(expected, TipoSem::Desconocido)
                {
                    self.error(
                        CausaError::RetornoIncompatibleConAnotacion {
                            funcion: self.funcion_actual.clone().unwrap_or_default(),
                            esperado: expected.nombre(),
                            obtenido: vt.nombre(),
                        },
                        linea,
                        columna,
                    );
                }
                if self.retorno_esperado.is_none() {
                    if let Some(inferred) = &self.retorno_inferido {
                        if !compatible(inferred, &vt)
                            && !matches!(vt, TipoSem::Desconocido)
                            && !matches!(inferred, TipoSem::Desconocido)
                        {
                            self.error(
                                CausaError::TipoDeRetornoIncompatible {
                                    esperado: inferred.estatico(),
                                    obtenido: vt.estatico(),
                                },
                                linea,
                                columna,
                            );
                        }
                    } else if !matches!(vt, TipoSem::Desconocido) {
                        self.retorno_inferido = Some(vt.clone());
                    }
                }
            }
            Sentencia::Mostrar { valor, .. } => {
                self.analizar_expresion(&valor);
            }
            Sentencia::MostrarSinSalto { valor, .. } => {
                self.analizar_expresion(&valor);
            }
            Sentencia::Romper { .. } | Sentencia::Continuar { .. } => {}
            Sentencia::IntentarRescatar {
                cuerpo_intentar,
                variable_error,
                cuerpo_rescatar,
                ..
            } => {
                self.analizar_bloque(&cuerpo_intentar);
                self.scopes.push(HashMap::new());
                self.gestor.entrar();
                if let Some(v) = variable_error {
                    self.scopes.last_mut().unwrap().insert(
                        v,
                        VarInfo {
                            tipo: TipoSem::Desconocido,
                            mutable: true,
                        },
                    );
                }
                for st in &cuerpo_rescatar {
                    self.analizar_sentencia(st.clone());
                }
                self.scopes.pop();
                self.gestor.salir();
            }
            Sentencia::Exportar { sentencia, .. } => {
                self.analizar_sentencia(*sentencia);
            }
            Sentencia::Usar {
                ruta,
                alias,
                nombres,
                linea,
                columna,
            } => {
                let importador = self.archivo_actual.clone();
                let canonical = match self.resolver_y_analizar_modulo(
                    &ruta,
                    importador.as_deref(),
                    linea,
                    columna,
                ) {
                    Some(p) => p,
                    None => return,
                };
                let modulo = match self.modulos.get(&canonical).cloned() {
                    Some(m) => m,
                    None => return,
                };

                if let Some(alias) = alias {
                    if self.existe_en_ambito_actual(&alias) || self.funciones.contains_key(&alias) {
                        self.error(
                            CausaError::ImportacionDuplicada {
                                nombre: alias.clone(),
                                modulo: canonical.display().to_string(),
                            },
                            linea,
                            columna,
                        );
                    } else {
                        self.tabla.registrar(
                            alias.clone(),
                            self.gestor.actual(),
                            CategoriaSimbodo::Variable,
                            false,
                        );
                        self.scopes.last_mut().unwrap().insert(
                            alias.clone(),
                            VarInfo {
                                tipo: TipoSem::Desconocido,
                                mutable: false,
                            },
                        );
                    }
                }

                if let Some(nombres) = nombres {
                    for nombre in nombres {
                        if self.existe_en_ambito_actual(&nombre)
                            || self.funciones.contains_key(&nombre)
                        {
                            self.error(
                                CausaError::ImportacionDuplicada {
                                    nombre: nombre.clone(),
                                    modulo: canonical.display().to_string(),
                                },
                                linea,
                                columna,
                            );
                            continue;
                        }
                        match modulo.exports.get(&nombre) {
                            Some(ExportadoSem::Variable(t)) => {
                                self.tabla.registrar(
                                    nombre.clone(),
                                    self.gestor.actual(),
                                    CategoriaSimbodo::Variable,
                                    false,
                                );
                                self.scopes.last_mut().unwrap().insert(
                                    nombre.clone(),
                                    VarInfo {
                                        tipo: t.clone(),
                                        mutable: false,
                                    },
                                );
                            }
                            Some(ExportadoSem::Funcion(sig)) => {
                                self.tabla.registrar(
                                    nombre.clone(),
                                    self.gestor.actual(),
                                    CategoriaSimbodo::Funcion,
                                    false,
                                );
                                self.funciones.insert(nombre.clone(), sig.clone());
                            }
                            None => {
                                if modulo.nombres.contains(&nombre) {
                                    self.error(
                                        CausaError::SimbodoPrivado {
                                            nombre: nombre.clone(),
                                            modulo: canonical.display().to_string(),
                                        },
                                        linea,
                                        columna,
                                    );
                                } else {
                                    self.error(
                                        CausaError::SimbodoNoExportado {
                                            nombre: nombre.clone(),
                                            modulo: canonical.display().to_string(),
                                        },
                                        linea,
                                        columna,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Sentencia::Importar { .. } => {
                // `importar` es una forma legacy; se deja al runtime.
            }
        }
    }

    fn analizar_bloque(&mut self, stmts: &[Sentencia]) {
        self.scopes.push(HashMap::new());
        self.gestor.entrar();
        for s in stmts {
            self.analizar_sentencia(s.clone());
        }
        self.scopes.pop();
        self.gestor.salir();
    }

    fn analizar_expresion(&mut self, e: &Expresion) -> TipoSem {
        match e {
            Expresion::Literal { valor, .. } => valor_a_sem(valor),
            Expresion::Variable {
                nombre,
                linea,
                columna,
                ..
            } => {
                if let Some(info) = self.buscar_variable(nombre) {
                    info.tipo
                } else if let Some(sig) = self.funciones.get(nombre).cloned() {
                    let params: Vec<TipoSem> = sig
                        .parametros
                        .iter()
                        .map(|(_, t)| t.clone().unwrap_or(TipoSem::Desconocido))
                        .collect();
                    let retorno = Box::new(sig.retorno.clone().unwrap_or(TipoSem::Desconocido));
                    TipoSem::Funcion { params, retorno }
                } else {
                    self.error(
                        CausaError::VariableNoDefinida(nombre.clone()),
                        *linea,
                        *columna,
                    );
                    TipoSem::Desconocido
                }
            }
            Expresion::Suma(a, b) => {
                let ta = self.analizar_expresion(a);
                let tb = self.analizar_expresion(b);
                if matches!(ta, TipoSem::Desconocido) || matches!(tb, TipoSem::Desconocido) {
                    return TipoSem::Desconocido;
                }
                if ta == TipoSem::Texto && tb == TipoSem::Texto {
                    return TipoSem::Texto;
                }
                match tipo_numerico_resultado(&ta, &tb) {
                    Some(t) => t,
                    None => {
                        self.error(
                            CausaError::TiposIncompatibles {
                                izquierdo: ta.estatico(),
                                derecho: tb.estatico(),
                            },
                            1,
                            1,
                        );
                        TipoSem::Desconocido
                    }
                }
            }
            Expresion::Resta(a, b)
            | Expresion::Multiplicacion(a, b)
            | Expresion::Division(a, b)
            | Expresion::Modulo(a, b) => {
                let ta = self.analizar_expresion(a);
                let tb = self.analizar_expresion(b);
                if matches!(ta, TipoSem::Desconocido) || matches!(tb, TipoSem::Desconocido) {
                    return TipoSem::Desconocido;
                }
                match tipo_numerico_resultado(&ta, &tb) {
                    Some(t) => t,
                    None => {
                        self.error(
                            CausaError::TiposIncompatibles {
                                izquierdo: ta.estatico(),
                                derecho: tb.estatico(),
                            },
                            1,
                            1,
                        );
                        TipoSem::Desconocido
                    }
                }
            }
            Expresion::IgualIgual(a, b) | Expresion::Distinto(a, b) => {
                let ta = self.analizar_expresion(a);
                let tb = self.analizar_expresion(b);
                if !matches!(ta, TipoSem::Desconocido)
                    && !matches!(tb, TipoSem::Desconocido)
                    && !comparables(&ta, &tb)
                {
                    self.error(
                        CausaError::TiposIncompatibles {
                            izquierdo: ta.estatico(),
                            derecho: tb.estatico(),
                        },
                        1,
                        1,
                    );
                }
                TipoSem::Logico
            }
            Expresion::Menor(a, b)
            | Expresion::MenorIgual(a, b)
            | Expresion::Mayor(a, b)
            | Expresion::MayorIgual(a, b) => {
                let ta = self.analizar_expresion(a);
                let tb = self.analizar_expresion(b);
                if !matches!(ta, TipoSem::Desconocido)
                    && !matches!(tb, TipoSem::Desconocido)
                    && !ordenables(&ta, &tb)
                {
                    self.error(
                        CausaError::TiposIncompatibles {
                            izquierdo: ta.estatico(),
                            derecho: tb.estatico(),
                        },
                        1,
                        1,
                    );
                }
                TipoSem::Logico
            }
            Expresion::No(x) => {
                let t = self.analizar_expresion(x);
                if !matches!(t, TipoSem::Desconocido | TipoSem::Logico) {
                    self.error(
                        CausaError::OperandoNoLogico {
                            operador: "no",
                            obtenido: t.estatico(),
                        },
                        1,
                        1,
                    );
                }
                TipoSem::Logico
            }
            Expresion::Y(a, b) | Expresion::O(a, b) => {
                let ta = self.analizar_expresion(a);
                let tb = self.analizar_expresion(b);
                if !matches!(ta, TipoSem::Desconocido | TipoSem::Logico) {
                    self.error(
                        CausaError::OperandoNoLogico {
                            operador: if matches!(e, Expresion::Y(_, _)) {
                                "y"
                            } else {
                                "o"
                            },
                            obtenido: ta.estatico(),
                        },
                        1,
                        1,
                    );
                }
                if !matches!(tb, TipoSem::Desconocido | TipoSem::Logico) {
                    self.error(
                        CausaError::OperandoNoLogico {
                            operador: if matches!(e, Expresion::Y(_, _)) {
                                "y"
                            } else {
                                "o"
                            },
                            obtenido: tb.estatico(),
                        },
                        1,
                        1,
                    );
                }
                TipoSem::Logico
            }
            Expresion::Llamada {
                nombre,
                argumentos,
                linea,
                columna,
            } => {
                let sig = self.funciones.get(nombre).cloned();
                let builtin = crate::evaluador::NOMBRES_RESERVADOS.contains(&nombre.as_str());
                let var_existe = self.buscar_variable(nombre).is_some();
                if sig.is_none() && !builtin && !var_existe {
                    self.error(
                        CausaError::VariableNoDefinida(nombre.clone()),
                        *linea,
                        *columna,
                    );
                }
                let mut arg_types = Vec::new();
                for a in argumentos {
                    arg_types.push(self.analizar_expresion(a));
                }
                // Una función valor (anidada asignada en ámbito o lambda) da
                // su firma; validarla como lo hace el evaluador en tiempo real.
                if let Some(sig) = sig {
                    if sig.parametros.len() != argumentos.len() {
                        self.error(
                            CausaError::FirmaInvalida {
                                funcion: nombre.clone(),
                                esperado: sig.parametros.len(),
                                obtenido: argumentos.len(),
                            },
                            *linea,
                            *columna,
                        );
                    }
                    for (i, (pname, ptype)) in sig.parametros.iter().enumerate() {
                        if let (Some(pt), Some(at)) = (ptype, arg_types.get(i))
                            && !compatible(pt, at)
                        {
                            self.error(
                                CausaError::TipoParametroIncompatible {
                                    funcion: nombre.clone(),
                                    parametro: pname.clone(),
                                    esperado: pt.nombre(),
                                    obtenido: at.nombre(),
                                },
                                *linea,
                                *columna,
                            );
                        }
                    }
                    sig.retorno.clone().unwrap_or(TipoSem::Desconocido)
                } else if let Some(VarInfo { tipo, .. }) = self.buscar_variable(nombre)
                    && let TipoSem::Funcion { params, retorno } = tipo
                {
                    if params.len() != argumentos.len() {
                        self.error(
                            CausaError::FirmaInvalida {
                                funcion: nombre.clone(),
                                esperado: params.len(),
                                obtenido: argumentos.len(),
                            },
                            *linea,
                            *columna,
                        );
                    }
                    for (i, (pt, at)) in params.iter().zip(arg_types.iter()).enumerate() {
                        if !compatible(pt, at)
                            && !matches!(pt, TipoSem::Desconocido)
                            && !matches!(at, TipoSem::Desconocido)
                        {
                            self.error(
                                CausaError::TipoParametroIncompatible {
                                    funcion: nombre.clone(),
                                    parametro: format!("arg{i}"),
                                    esperado: pt.nombre(),
                                    obtenido: at.nombre(),
                                },
                                *linea,
                                *columna,
                            );
                        }
                    }
                    *retorno
                } else {
                    TipoSem::Desconocido
                }
            }
            Expresion::Indice { objeto, indice, .. } => {
                let to = self.analizar_expresion(objeto);
                let ti = self.analizar_expresion(indice);
                if !matches!(ti, TipoSem::Desconocido | TipoSem::Entero) {
                    self.error(
                        CausaError::TiposIncompatibles {
                            izquierdo: "entero",
                            derecho: ti.estatico(),
                        },
                        1,
                        1,
                    );
                }
                match to {
                    TipoSem::Arreglo(t, _) => *t,
                    TipoSem::Lista(t) => *t,
                    _ => TipoSem::Desconocido,
                }
            }
            Expresion::Acceso { objeto, .. } => {
                self.analizar_expresion(objeto);
                TipoSem::Desconocido
            }
            Expresion::LlamadaIndirecta {
                callee,
                argumentos,
                linea,
                columna,
            } => {
                let callee_type = self.analizar_expresion(callee);
                let mut arg_types = Vec::new();
                for a in argumentos {
                    arg_types.push(self.analizar_expresion(a));
                }
                match callee_type {
                    TipoSem::Funcion { params, retorno } => {
                        if params.len() != argumentos.len() {
                            self.error(
                                CausaError::FirmaInvalida {
                                    funcion: "<indirecta>".to_string(),
                                    esperado: params.len(),
                                    obtenido: argumentos.len(),
                                },
                                *linea,
                                *columna,
                            );
                        }
                        for (i, (pt, at)) in params.iter().zip(arg_types.iter()).enumerate() {
                            if !compatible(pt, at)
                                && !matches!(pt, TipoSem::Desconocido)
                                && !matches!(at, TipoSem::Desconocido)
                            {
                                self.error(
                                    CausaError::TipoParametroIncompatible {
                                        funcion: "<indirecta>".to_string(),
                                        parametro: format!("arg{}", i),
                                        esperado: pt.nombre(),
                                        obtenido: at.nombre(),
                                    },
                                    *linea,
                                    *columna,
                                );
                            }
                        }
                        *retorno
                    }
                    TipoSem::Desconocido => TipoSem::Desconocido,
                    otro => {
                        self.error(
                            CausaError::AsignacionIncompatible {
                                esperado: "función".to_string(),
                                obtenido: otro.nombre(),
                            },
                            *linea,
                            *columna,
                        );
                        TipoSem::Desconocido
                    }
                }
            }
            Expresion::Lambda {
                parametros,
                tipo_retorno,
                cuerpo,
                ..
            } => {
                // El cuerpo de una lambda es contexto de función: `retornar`
                // es válido y el retorno anotado debe cumplir su anotación.
                let prev_func = self.funcion_actual.clone();
                let prev_ret = self.retorno_esperado.clone();
                let prev_inf = self.retorno_inferido.clone();
                let prev_en = self.en_funcion;
                self.funcion_actual = Some("<lambda>".to_string());
                self.retorno_esperado = tipo_retorno.as_ref().map(tipo_anotacion_a_sem);
                self.retorno_inferido = None;
                self.en_funcion = true;
                self.scopes.push(HashMap::new());
                self.gestor.entrar();
                for (pname, pta) in parametros {
                    let tipo = pta
                        .as_ref()
                        .map(tipo_anotacion_a_sem)
                        .unwrap_or(TipoSem::Desconocido);
                    self.scopes.last_mut().unwrap().insert(
                        pname.clone(),
                        VarInfo {
                            tipo,
                            mutable: true,
                        },
                    );
                }
                for st in cuerpo {
                    self.analizar_sentencia(st.clone());
                }
                self.scopes.pop();
                self.gestor.salir();
                self.funcion_actual = prev_func;
                self.retorno_esperado = prev_ret;
                self.retorno_inferido = prev_inf;
                self.en_funcion = prev_en;
                let params: Vec<TipoSem> = parametros
                    .iter()
                    .map(|(_, t)| {
                        t.as_ref()
                            .map(tipo_anotacion_a_sem)
                            .unwrap_or(TipoSem::Desconocido)
                    })
                    .collect();
                let retorno = tipo_retorno
                    .as_ref()
                    .map(tipo_anotacion_a_sem)
                    .unwrap_or(TipoSem::Desconocido);
                TipoSem::Funcion {
                    params,
                    retorno: Box::new(retorno),
                }
            }
            Expresion::ArregloLiteral { elementos, .. } => {
                if elementos.is_empty() {
                    return TipoSem::Desconocido;
                }
                let mut tipos = Vec::new();
                for e in elementos {
                    tipos.push(self.analizar_expresion(e));
                }
                let mut comun = tipos[0].clone();
                for t in &tipos[1..] {
                    if !compatible(&comun, t) {
                        self.error(
                            CausaError::TiposIncompatibles {
                                izquierdo: comun.estatico(),
                                derecho: t.estatico(),
                            },
                            1,
                            1,
                        );
                        comun = TipoSem::Desconocido;
                        break;
                    }
                    comun = tipo_comun(&comun, t);
                }
                TipoSem::Coleccion(Box::new(comun))
            }
            Expresion::Rango { inicio, fin, .. } => {
                self.analizar_expresion(inicio);
                self.analizar_expresion(fin);
                TipoSem::Desconocido
            }
        }
    }

    fn resolver_y_analizar_modulo(
        &mut self,
        ruta_relativa: &str,
        importador: Option<&Path>,
        linea: usize,
        columna: usize,
    ) -> Option<PathBuf> {
        let canonical = match self.resolver_modulo(ruta_relativa, importador, linea, columna) {
            Ok(p) => p,
            Err(e) => {
                self.errores.push(e);
                return None;
            }
        };

        if let Some(m) = self.modulos.get(&canonical).cloned() {
            let _ = m;
            return Some(canonical);
        }

        if self.en_carga.contains(&canonical) {
            let mut e = ErrorEjecucion {
                mensaje: format!("ciclo de importación: {}", canonical.display()),
                causa: Box::new(CausaError::CicloDeImportacion {
                    cadena: vec![canonical.display().to_string()],
                }),
                linea,
                columna,
                archivo: self.archivo.clone(),
            };
            if e.archivo.is_none() {
                e.archivo = Some(canonical.display().to_string());
            }
            self.errores.push(e);
            return None;
        }

        // Leer archivo.
        let fuente = match std::fs::read_to_string(&canonical) {
            Ok(f) => f,
            Err(err) => {
                let e = ErrorEjecucion {
                    mensaje: format!("no se pudo leer '{}': {err}", canonical.display()),
                    causa: Box::new(CausaError::ArchivoNoEncontrado {
                        ruta: canonical.display().to_string(),
                    }),
                    linea,
                    columna,
                    archivo: Some(canonical.display().to_string()),
                };
                self.errores.push(e);
                return None;
            }
        };

        // Lexar.
        let mut lexer = crate::Lexer::con_archivo(&fuente, Some(canonical.display().to_string()));
        let tokens = match lexer.tokenizar() {
            Ok(t) => t,
            Err(err) => {
                let e = ErrorEjecucion {
                    mensaje: err.mensaje.clone(),
                    causa: Box::new(CausaError::ErrorDeArchivo {
                        ruta: canonical.display().to_string(),
                        detalle: err.mensaje.clone(),
                    }),
                    linea: err.linea,
                    columna: err.columna,
                    archivo: err
                        .archivo
                        .clone()
                        .or(Some(canonical.display().to_string())),
                };
                self.errores.push(e);
                return None;
            }
        };

        // Parsear.
        let mut parser = crate::Parser::con_archivo(tokens, Some(canonical.display().to_string()));
        let ast = match parser.parsear_programa() {
            Ok(a) => a,
            Err(err) => {
                let e = ErrorEjecucion {
                    mensaje: err.mensaje.clone(),
                    causa: Box::new(CausaError::ErrorDeArchivo {
                        ruta: canonical.display().to_string(),
                        detalle: err.mensaje.clone(),
                    }),
                    linea: err.linea,
                    columna: err.columna,
                    archivo: err
                        .archivo
                        .clone()
                        .or(Some(canonical.display().to_string())),
                };
                self.errores.push(e);
                return None;
            }
        };

        // Validar nivel superior.
        for s in &ast {
            match s {
                Sentencia::Declaracion { .. }
                | Sentencia::Funcion { .. }
                | Sentencia::Usar { .. } => {}
                _ => {
                    let (l, c) = ubicacion_sentencia(s);
                    let e = ErrorEjecucion {
                        mensaje: "sentencia no permitida en el nivel superior de un módulo"
                            .to_string(),
                        causa: Box::new(CausaError::SentenciaNoPermitidaEnModulo {
                            sentencia: format!("{:?}", s),
                        }),
                        linea: l,
                        columna: c,
                        archivo: Some(canonical.display().to_string()),
                    };
                    self.errores.push(e);
                    return None;
                }
            }
        }

        self.en_carga.insert(canonical.clone());
        let info = self.analizar_modulo_ast(&canonical, &ast);
        self.en_carga.remove(&canonical);
        self.modulos.insert(canonical.clone(), info);
        Some(canonical)
    }

    fn resolver_modulo(
        &self,
        ruta_relativa: &str,
        importador: Option<&Path>,
        linea: usize,
        columna: usize,
    ) -> Result<PathBuf, ErrorEjecucion> {
        let cargador = self.cargador.as_ref().ok_or_else(|| ErrorEjecucion {
            mensaje: "importaciones no disponibles".to_string(),
            causa: Box::new(CausaError::ImportacionesNoDisponibles),
            linea,
            columna,
            archivo: self.archivo.clone(),
        })?;
        CargadorModulos::resolver_ruta(cargador, ruta_relativa, importador)
    }

    fn analizar_modulo_ast(&mut self, path: &Path, ast: &[Sentencia]) -> ModuloInfo {
        let prev_archivo = self.archivo_actual.clone();
        let prev_funciones = self.funciones.clone();
        let prev_scopes = self.scopes.clone();
        self.archivo_actual = Some(path.to_path_buf());
        self.funciones = HashMap::new();
        self.scopes = vec![HashMap::new()];

        self.registrar_funciones(ast);
        for s in ast {
            self.analizar_sentencia(s.clone());
        }

        let mut exports = HashMap::new();
        let mut nombres = HashSet::new();
        for s in ast {
            match s {
                Sentencia::Declaracion {
                    nombre, publico, ..
                } => {
                    nombres.insert(nombre.clone());
                    if *publico && let Some(info) = self.buscar_variable(nombre) {
                        exports.insert(nombre.clone(), ExportadoSem::Variable(info.tipo));
                    }
                }
                Sentencia::Funcion {
                    nombre, publico, ..
                } => {
                    nombres.insert(nombre.clone());
                    if *publico && let Some(sig) = self.funciones.get(nombre).cloned() {
                        exports.insert(nombre.clone(), ExportadoSem::Funcion(sig));
                    }
                }
                _ => {}
            }
        }

        self.archivo_actual = prev_archivo;
        self.funciones = prev_funciones;
        self.scopes = prev_scopes;
        ModuloInfo { exports, nombres }
    }

    pub fn buscar_variable(&self, nombre: &str) -> Option<VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(nombre) {
                return Some(v.clone());
            }
        }
        None
    }

    /// Resuelve un identificador a su símbodo usando la infraestructura de la
    /// Etapa 1, caminando la pila de ámbitos del gestor en orden léxico.
    pub fn resolver_simbodo(&self, nombre: &str) -> Option<SimbodoId> {
        for ambito in self.gestor.pila().iter().rev() {
            if let Some(id) = self.tabla.buscar(*ambito, nombre) {
                return Some(id);
            }
        }
        None
    }

    fn existe_en_ambito_actual(&self, nombre: &str) -> bool {
        self.scopes.last().is_some_and(|s| s.contains_key(nombre))
    }
}

pub fn tipo_anotacion_a_sem(t: &TipoAnotacion) -> TipoSem {
    match t {
        TipoAnotacion::Entero => TipoSem::Entero,
        TipoAnotacion::Decimal => TipoSem::Decimal,
        TipoAnotacion::Real => TipoSem::Real,
        TipoAnotacion::Logico => TipoSem::Logico,
        TipoAnotacion::Texto => TipoSem::Texto,
        TipoAnotacion::Arreglo(t, n) => TipoSem::Arreglo(Box::new(tipo_anotacion_a_sem(t)), *n),
        TipoAnotacion::Lista(t) => TipoSem::Lista(Box::new(tipo_anotacion_a_sem(t))),
        TipoAnotacion::Nulo => TipoSem::Nulo,
    }
}

fn valor_a_sem(v: &Valor) -> TipoSem {
    match v {
        Valor::Entero(_) => TipoSem::Entero,
        Valor::Decimal(_) => TipoSem::Decimal,
        Valor::Real(_) => TipoSem::Real,
        Valor::Logico(_) => TipoSem::Logico,
        Valor::Texto(_) => TipoSem::Texto,
        Valor::Arreglo(xs) => {
            let t = xs.first().map(valor_a_sem).unwrap_or(TipoSem::Desconocido);
            TipoSem::Arreglo(Box::new(t), xs.len())
        }
        Valor::Lista(xs) => {
            let t = xs.first().map(valor_a_sem).unwrap_or(TipoSem::Desconocido);
            TipoSem::Lista(Box::new(t))
        }
        Valor::Nulo => TipoSem::Nulo,
        _ => TipoSem::Desconocido,
    }
}

fn compatible(a: &TipoSem, b: &TipoSem) -> bool {
    if matches!(a, TipoSem::Desconocido) || matches!(b, TipoSem::Desconocido) {
        return true;
    }
    match (a, b) {
        (TipoSem::Entero, TipoSem::Entero) => true,
        (TipoSem::Decimal, TipoSem::Decimal) | (TipoSem::Decimal, TipoSem::Entero) => true,
        (TipoSem::Real, TipoSem::Real) | (TipoSem::Real, TipoSem::Entero) => true,
        (TipoSem::Logico, TipoSem::Logico) => true,
        (TipoSem::Texto, TipoSem::Texto) => true,
        (TipoSem::Nulo, TipoSem::Nulo) => true,
        (TipoSem::Arreglo(te, ne), TipoSem::Arreglo(to, no)) => ne == no && compatible(te, to),
        (TipoSem::Lista(te), TipoSem::Lista(to)) => compatible(te, to),
        (TipoSem::Arreglo(te, _), TipoSem::Lista(to)) => compatible(te, to),
        (TipoSem::Lista(te), TipoSem::Arreglo(to, _)) => compatible(te, to),
        (TipoSem::Coleccion(te), TipoSem::Arreglo(to, _)) => compatible(te, to),
        (TipoSem::Coleccion(te), TipoSem::Lista(to)) => compatible(te, to),
        (TipoSem::Arreglo(te, _), TipoSem::Coleccion(to)) => compatible(te, to),
        (TipoSem::Lista(te), TipoSem::Coleccion(to)) => compatible(te, to),
        (TipoSem::Coleccion(te), TipoSem::Coleccion(to)) => compatible(te, to),
        (
            TipoSem::Funcion {
                params: p1,
                retorno: r1,
            },
            TipoSem::Funcion {
                params: p2,
                retorno: r2,
            },
        ) => {
            p1.len() == p2.len()
                && p1.iter().zip(p2).all(|(a, b)| compatible(a, b))
                && compatible(r1, r2)
        }
        _ => false,
    }
}

fn comparables(a: &TipoSem, b: &TipoSem) -> bool {
    if matches!(a, TipoSem::Desconocido) || matches!(b, TipoSem::Desconocido) {
        return true;
    }
    if compatible(a, b) {
        return true;
    }
    // permitir comparaciones entre numéricos aunque no haya conversión directa
    (matches!(a, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
        && matches!(b, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real))
}

fn ordenables(a: &TipoSem, b: &TipoSem) -> bool {
    if matches!(a, TipoSem::Desconocido) || matches!(b, TipoSem::Desconocido) {
        return true;
    }
    (matches!(
        a,
        TipoSem::Entero | TipoSem::Decimal | TipoSem::Real | TipoSem::Texto
    ) && matches!(
        b,
        TipoSem::Entero | TipoSem::Decimal | TipoSem::Real | TipoSem::Texto
    ) && (matches!(a, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
        == matches!(b, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
        || (matches!(a, TipoSem::Texto) && matches!(b, TipoSem::Texto))))
}

fn tipo_numerico_resultado(a: &TipoSem, b: &TipoSem) -> Option<TipoSem> {
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

fn ubicacion_sentencia(s: &Sentencia) -> (usize, usize) {
    match s {
        Sentencia::Declaracion { linea, columna, .. }
        | Sentencia::Asignacion { linea, columna, .. }
        | Sentencia::AsignacionIndice { linea, columna, .. }
        | Sentencia::Si { linea, columna, .. }
        | Sentencia::Mientras { linea, columna, .. }
        | Sentencia::Funcion { linea, columna, .. }
        | Sentencia::Retorno { linea, columna, .. }
        | Sentencia::Mostrar { linea, columna, .. }
        | Sentencia::MostrarSinSalto { linea, columna, .. }
        | Sentencia::ParaCada { linea, columna, .. }
        | Sentencia::Romper { linea, columna }
        | Sentencia::Continuar { linea, columna }
        | Sentencia::IntentarRescatar { linea, columna, .. }
        | Sentencia::Importar { linea, columna, .. }
        | Sentencia::Exportar { linea, columna, .. }
        | Sentencia::Usar { linea, columna, .. } => (*linea, *columna),
        Sentencia::Bloque(_) | Sentencia::Expresion(_) => (1, 1),
    }
}

fn tipo_comun(a: &TipoSem, b: &TipoSem) -> TipoSem {
    if matches!(a, TipoSem::Desconocido) {
        return b.clone();
    }
    if matches!(b, TipoSem::Desconocido) {
        return a.clone();
    }
    match (a, b) {
        (TipoSem::Entero, TipoSem::Entero) => TipoSem::Entero,
        (TipoSem::Decimal, TipoSem::Entero)
        | (TipoSem::Entero, TipoSem::Decimal)
        | (TipoSem::Decimal, TipoSem::Decimal) => TipoSem::Decimal,
        (TipoSem::Real, _) | (_, TipoSem::Real)
            if matches!(a, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
                && matches!(b, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real) =>
        {
            TipoSem::Real
        }
        (TipoSem::Texto, TipoSem::Texto) => TipoSem::Texto,
        (TipoSem::Logico, TipoSem::Logico) => TipoSem::Logico,
        _ => TipoSem::Desconocido,
    }
}

impl ContextoSemantico {
    pub fn archivo_actual(&self) -> Option<&PathBuf> {
        self.archivo_actual.as_ref()
    }
    pub fn set_archivo_actual(&mut self, p: Option<PathBuf>) {
        self.archivo_actual = p;
    }
    pub fn cargador(&self) -> Option<&Arc<Mutex<CargadorModulos>>> {
        self.cargador.as_ref()
    }
}
