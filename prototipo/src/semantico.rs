//! Analizador semántico estático de BASILISCO.
//!
//! Recorre el AST sin ejecutarlo y valida ámbitos, variables, funciones y
//! tipos cuando pueden determinarse estáticamente. Reutiliza `CausaError`
//! y `ErrorEjecucion` del evaluador para emitir diagnósticos uniformes.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::Valor;
use crate::cargador::CargadorModulos;
use crate::evaluador::{CausaError, ErrorEjecucion};
use crate::parser::{Expresion, Sentencia, TipoAnotacion};
use crate::simbodo::{AmbitoId, CategoriaSimbodo, GestorAmbitos, SimbodoId, TablaSimbodo};

/// Identidad semántica estable de un parámetro de tipo genérico (G2.2).
/// Vinculada a su declaración propietaria; compatible con cualificación por módulo.
/// No debe mezclarse con identificadores HIR/MIR.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TipoParamId {
    pub propietario: SimbodoId,
    pub nombre: String,
}

/// Tipo semántico determinado estáticamente.
/// `Desconocido` representa información no disponible.
/// `Param` representa un parámetro de tipo genérico declarado (identidad conocida,
/// opaco sin capacidades/bounds; G2.2). `Param` es distinto de `Desconocido`.
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
        genericos: Vec<TipoParamId>,
    },
    Nulo,
    /// Parámetro de tipo genérico declarado (G2.2), opaco.
    Param(TipoParamId),
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
            TipoSem::Funcion {
                params,
                retorno,
                genericos,
            } => {
                let ps: Vec<String> = params.iter().map(|t| t.nombre()).collect();
                if genericos.is_empty() {
                    format!("función({}) -> {}", ps.join(", "), retorno.nombre())
                } else {
                    let gs: Vec<String> = genericos.iter().map(|g| g.nombre.clone()).collect();
                    format!(
                        "función<{}>({}) -> {}",
                        gs.join(", "),
                        ps.join(", "),
                        retorno.nombre()
                    )
                }
            }
            TipoSem::Nulo => "nulo".to_string(),
            TipoSem::Param(id) => id.nombre.clone(),
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
            TipoSem::Nulo => "nulo",
            TipoSem::Param(_) => "parámetro de tipo",
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
pub(crate) struct ParamTipoDecl {
    pub(crate) nombre: String,
    pub(crate) id: TipoParamId,
}

#[derive(Debug, Clone)]
pub(crate) struct FuncionSig {
    pub(crate) parametros: Vec<(String, Option<TipoSem>)>,
    pub(crate) retorno: Option<TipoSem>,
    // true si `retorno` proviene de anotación explícita en la firma.
    #[allow(dead_code)]
    pub(crate) retorno_anotado: bool,
    // Parámetros de tipo genéricos declarados (G2.2), en orden.
    pub(crate) parametros_tipo: Vec<ParamTipoDecl>,
    // G3: cuerpo y metadatos de plantilla para validación V3/forwarding por aplicación.
    pub(crate) cuerpo: Vec<Sentencia>,
    pub(crate) parametros_nombres: Vec<String>,
}

#[derive(Clone)]
enum ExportadoSem {
    Variable(TipoSem),
    Funcion(FuncionSig),
}

#[derive(Clone)]
struct ModuloInfo {
    /// Símbolos exportados explícitamente (`publico` o `exportar`).
    exports: HashMap<String, ExportadoSem>,
    /// Todos los nombres declarables de nivel superior (incluye privados).
    nombres: HashSet<String>,
    /// Todos los símbolos declarables con tipo/firma conocidos (para `importar` legacy).
    todos: HashMap<String, ExportadoSem>,
}

#[derive(Clone)]
#[allow(dead_code)]
struct C2ResultadoDecl {
    nombre: String,
    tipo: Option<TipoSem>,
}

/// Solicitud de instanciación G4 registrada durante el análisis semántico.
#[derive(Debug, Clone)]
pub(crate) struct SolicitudInstanciaG4 {
    pub(crate) callee_decl_id: SimbodoId,
    pub(crate) callee_nombre: String,
    pub(crate) sigma: HashMap<TipoParamId, TipoSem>,
    pub(crate) linea: usize,
    pub(crate) columna: usize,
}

/// Analizador semántico. Independiente del evaluador: no ejecuta código.
pub struct ContextoSemantico {
    pub archivo: Option<String>,
    cargador: Option<Arc<Mutex<CargadorModulos>>>,
    archivo_actual: Option<PathBuf>,
    scopes: Vec<HashMap<String, VarInfo>>,
    tabla: TablaSimbodo,
    gestor: GestorAmbitos,
    pub(crate) funciones: HashMap<String, FuncionSig>,
    errores: Vec<ErrorEjecucion>,
    funcion_actual: Option<String>,
    retorno_esperado: Option<TipoSem>,
    retorno_inferido: Option<TipoSem>,
    en_funcion: bool,
    modulos: HashMap<PathBuf, ModuloInfo>,
    en_carga: HashSet<PathBuf>,
    // C.2: resultados de inferencia local conservadora de retornos para
    // funciones nombradas no anotadas. Canal separado del mecanismo legado C.1
    // (FuncionSig.retorno/propagación/fallback): no se usa para resolución de
    // llamadas ni se propaga interprocedimentalmente. Clave = identidad estable
    // de declaración (SimbodoId), no nombre simple. `tipo: None` = Desconocido.
    c2_resultados: HashMap<SimbodoId, C2ResultadoDecl>,
    c2_retorno_tipos: Vec<TipoSem>,
    c2_evidencia_insuficiente: bool,
    c2_activo: bool,
    // G2.2/G3: pila de ámbitos de parámetros de tipo para resolución semántica.
    // Cada función genérica declara su propio conjunto (sin herencia implícita).
    // Valores: `Param(id)` en análisis de plantilla, o tipo sustituido en vista V3.
    parametros_tipo_scope: Vec<HashMap<String, TipoSem>>,
    // G3: dependencias genéricas resueltas/colectadas (deterministas, sin instancias).
    g3_dependencias: BTreeSet<String>,
    // G3: profundidad de validación V3 (vista sustituida) para evitar efectos colaterales.
    g3_v3_depth: usize,
    // G3: aplicaciones en validación V3 para evitar recursión infinita.
    g3_v3_en_progreso: BTreeSet<String>,
    // G4: solicitudes de instanciación concreta recolectadas (seeds y dependencias).
    pub(crate) g4_solicitudes: Vec<SolicitudInstanciaG4>,
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
            c2_resultados: HashMap::new(),
            c2_retorno_tipos: Vec::new(),
            c2_evidencia_insuficiente: false,
            c2_activo: false,
            parametros_tipo_scope: Vec::new(),
            g3_dependencias: BTreeSet::new(),
            g3_v3_depth: 0,
            g3_v3_en_progreso: BTreeSet::new(),
            g4_solicitudes: Vec::new(),
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
                    parametros_tipo,
                    parametros,
                    tipo_retorno,
                    cuerpo,
                    ..
                } => {
                    let decl_id = if let Some(id) = self.tabla.buscar(AmbitoId(0), nombre) {
                        id
                    } else {
                        self.tabla.registrar(
                            nombre.clone(),
                            AmbitoId(0),
                            CategoriaSimbodo::Funcion,
                            false,
                        )
                    };
                    let decls_tp = self.construir_params_tipo_decl(decl_id, parametros_tipo);
                    let scope_tp = Self::scope_mapa_parametros_tipo(&decls_tp);
                    let prev_scope = std::mem::take(&mut self.parametros_tipo_scope);
                    if !scope_tp.is_empty() {
                        self.parametros_tipo_scope.push(scope_tp);
                    }
                    let params = parametros
                        .iter()
                        .map(|(n, t)| (n.clone(), self.resolver_tipo_anotacion_opt(t)))
                        .collect();
                    let retorno_anotado = tipo_retorno.is_some();
                    let retorno = self.resolver_tipo_anotacion_opt(tipo_retorno);
                    self.parametros_tipo_scope = prev_scope;
                    let parametros_nombres = parametros.iter().map(|(n, _)| n.clone()).collect();
                    self.funciones.insert(
                        nombre.clone(),
                        FuncionSig {
                            parametros: params,
                            retorno,
                            retorno_anotado,
                            parametros_tipo: decls_tp,
                            cuerpo: cuerpo.clone(),
                            parametros_nombres,
                        },
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
                    let t = self.resolver_tipo_anotacion(&ta);
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
                        } else if matches!(info.tipo, TipoSem::Desconocido)
                            && !matches!(vt, TipoSem::Desconocido)
                        {
                            // B.2/B.3: refinar tipo local cuando hay evidencia demostrable
                            // en una variable sin tipo conocido, preservando el
                            // comportamiento conservador ante ambigüedad.
                            self.actualizar_tipo_variable(&nombre, vt);
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
                let to = self.analizar_expresion(&objeto);
                let ti = self.analizar_expresion(&indice);
                let tv = self.analizar_expresion(&valor);
                if !matches!(ti, TipoSem::Desconocido | TipoSem::Entero) {
                    self.error(
                        CausaError::TiposIncompatibles {
                            izquierdo: "entero",
                            derecho: ti.estatico(),
                        },
                        linea,
                        columna,
                    );
                }
                let te = tipo_elemento_coleccion(&to);
                if !matches!(te, TipoSem::Desconocido)
                    && !matches!(tv, TipoSem::Desconocido)
                    && !compatible(&te, &tv)
                {
                    self.error(
                        CausaError::AsignacionIncompatible {
                            esperado: te.nombre(),
                            obtenido: tv.nombre(),
                        },
                        linea,
                        columna,
                    );
                }
            }
            Sentencia::Si {
                condicion,
                rama_si,
                rama_sino,
                linea,
                columna,
            } => {
                let ct = self.analizar_expresion(&condicion);
                if es_param_opaco(&ct) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "uso como condición lógica de parámetro de tipo opaco"
                                .to_string(),
                        },
                        linea,
                        columna,
                    );
                } else if !matches!(ct, TipoSem::Desconocido | TipoSem::Logico) {
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
                if es_param_opaco(&ct) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "uso como condición lógica de parámetro de tipo opaco"
                                .to_string(),
                        },
                        linea,
                        columna,
                    );
                } else if !matches!(ct, TipoSem::Desconocido | TipoSem::Logico) {
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
                linea,
                columna,
            } => {
                let tipo_iterable = self.analizar_expresion(&iterable);
                if es_param_opaco(&tipo_iterable) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "iteración sobre parámetro de tipo opaco".to_string(),
                        },
                        linea,
                        columna,
                    );
                }
                let tipo_elemento = tipo_elemento_iterable(&tipo_iterable);
                self.scopes.push(HashMap::new());
                self.gestor.entrar();
                self.scopes.last_mut().unwrap().insert(
                    variable,
                    VarInfo {
                        tipo: tipo_elemento,
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
                parametros_tipo,
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
                let decl_id = self.tabla.registrar(
                    nombre.clone(),
                    self.gestor.actual(),
                    CategoriaSimbodo::Funcion,
                    false,
                );
                let es_anidada = self.en_funcion;
                // G2.2: cada declaración tiene su propio conjunto de parámetros de tipo
                // (sin herencia implícita de funciones exteriores).
                let decls_tp = self.construir_params_tipo_decl(decl_id, &parametros_tipo);
                let scope_tp = Self::scope_mapa_parametros_tipo(&decls_tp);
                let prev_tp_scope = std::mem::take(&mut self.parametros_tipo_scope);
                if !scope_tp.is_empty() {
                    self.parametros_tipo_scope.push(scope_tp);
                }
                // Registrar la firma en las funciones para validar llamadas y,
                // si es anidada (dentro de otra función), también en el ámbito
                // actual, tal como hace el evaluador al ejecutar la declaración.
                let params: Vec<(String, Option<TipoSem>)> = parametros
                    .iter()
                    .map(|(n, t)| (n.clone(), self.resolver_tipo_anotacion_opt(t)))
                    .collect();
                // C.1: el retorno conocido en la firma proviene únicamente de la
                // anotación explícita. No se infiere desde el cuerpo (C.2).
                let retorno_anotado_flag = tipo_retorno.is_some();
                let retorno_firma = self.resolver_tipo_anotacion_opt(&tipo_retorno);
                let parametros_nombres = parametros.iter().map(|(n, _)| n.clone()).collect();
                let firma = FuncionSig {
                    parametros: params.clone(),
                    retorno: retorno_firma.clone(),
                    retorno_anotado: retorno_anotado_flag,
                    parametros_tipo: decls_tp,
                    cuerpo: cuerpo.clone(),
                    parametros_nombres,
                };
                let firma_previa = self.funciones.get(&nombre).cloned();
                self.funciones.insert(nombre.clone(), firma.clone());
                self.scopes.last_mut().unwrap().insert(
                    nombre.clone(),
                    VarInfo {
                        tipo: TipoSem::Funcion {
                            params: params
                                .iter()
                                .map(|(_, t)| t.clone().unwrap_or(TipoSem::Desconocido))
                                .collect(),
                            retorno: Box::new(
                                retorno_firma.clone().unwrap_or(TipoSem::Desconocido),
                            ),
                            genericos: firma.parametros_tipo.iter().map(|p| p.id.clone()).collect(),
                        },
                        mutable: false,
                    },
                );
                self.scopes.push(HashMap::new());
                self.gestor.entrar();
                let mut vistos: std::collections::HashSet<&str> = std::collections::HashSet::new();
                for (pname, pta) in &parametros {
                    if !vistos.insert(pname.as_str()) {
                        self.error(
                            CausaError::DeclaracionDuplicada(pname.clone()),
                            linea,
                            columna,
                        );
                        continue;
                    }
                    let tipo = self
                        .resolver_tipo_anotacion_opt(pta)
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
                let prev_c2_tipos = std::mem::take(&mut self.c2_retorno_tipos);
                let prev_c2_insuf = std::mem::replace(&mut self.c2_evidencia_insuficiente, false);
                let prev_c2_activo = std::mem::replace(&mut self.c2_activo, false);
                self.funcion_actual = Some(nombre.clone());
                self.retorno_esperado = retorno_firma.clone();
                self.retorno_inferido = None;
                self.en_funcion = true;
                // C.2: inferencia local conservadora solo para funciones nombradas
                // sin anotación explícita de retorno; lambdas/closures excluidas.
                self.c2_activo = tipo_retorno.is_none();
                self.c2_retorno_tipos.clear();
                self.c2_evidencia_insuficiente = false;
                for st in &cuerpo {
                    self.analizar_sentencia(st.clone());
                }
                // Finalizar C.2 en canal separado (sin afectar FuncionSig/fallback legado).
                self.c2_finalizar(decl_id, &nombre, &cuerpo, tipo_retorno.is_none());
                // Se preserva propagación preexistente de retorno inferido en la
                // firma registrada para compatibilidad (contratos existentes);
                // C.1 no introduce nueva inferencia de cuerpo y las firmas usadas
                // como valor-función en ámbitos se construyen desde anotaciones.
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
                self.c2_retorno_tipos = prev_c2_tipos;
                self.c2_evidencia_insuficiente = prev_c2_insuf;
                self.c2_activo = prev_c2_activo;
                self.scopes.pop();
                self.gestor.salir();
                self.parametros_tipo_scope = prev_tp_scope;
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
                match valor {
                    Some(v) => {
                        // C.2: evidencia local conservadora y separada (D4/D5).
                        // Se evalúa admisibilidad sin emitir diagnósticos.
                        let c2_admisible = self.c2_activo && self.c2_expr_admisible(&v);
                        let vt = self.analizar_expresion(&v);
                        if self.c2_activo {
                            if c2_admisible && !matches!(vt, TipoSem::Desconocido) {
                                self.c2_registrar_tipo(vt.clone());
                            } else {
                                self.c2_marcar_insuficiente();
                            }
                        }
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
                    None => {
                        // D4: `retornar` sin expresión es evidencia insuficiente para C.2.
                        // Se preserva el tratamiento semántico/diagnóstico legado.
                        self.c2_marcar_insuficiente();
                        let vt = TipoSem::Nulo;
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
            Sentencia::Importar {
                ruta,
                alias,
                linea,
                columna,
            } => {
                let importador = self.archivo_actual.clone();
                let errores_antes = self.errores.len();
                let mut canonical =
                    self.resolver_y_analizar_modulo(&ruta, importador.as_deref(), linea, columna);
                if canonical.is_none() {
                    // Fallback para `importar` legacy: resolución por bloqueo/convenciones
                    // (paquetes instalados, rutas simples) usada por el runtime.
                    self.errores.truncate(errores_antes);
                    let mut candidatos: Vec<std::path::PathBuf> = Vec::new();
                    if let Some(p) = crate::paquetes::resolver_desde_bloqueo(&ruta) {
                        candidatos.push(p);
                    }
                    let base = std::path::PathBuf::from(&ruta);
                    candidatos.extend([
                        std::path::PathBuf::from(format!("{}.bsk", ruta)),
                        base.join("lib.bsk"),
                        base.join("src").join("lib.bsk"),
                        std::path::PathBuf::from("paquetes")
                            .join(&ruta)
                            .join("lib.bsk"),
                        std::path::PathBuf::from("vendor")
                            .join(&ruta)
                            .join("lib.bsk"),
                        base,
                    ]);
                    for cand in candidatos {
                        if !cand.is_file() {
                            continue;
                        }
                        if let Ok(cp) = std::fs::canonicalize(&cand) {
                            if let Some(m) =
                                self.analizar_modulo_desde_canonical(&cp, linea, columna)
                            {
                                canonical = Some(m);
                                break;
                            } else {
                                // Si falla análisis del candidato concreto, limpiar errores parciales
                                // e intentar siguiente.
                                self.errores.truncate(errores_antes);
                            }
                        }
                    }
                }
                let Some(canonical) = canonical else {
                    if self.errores.len() == errores_antes {
                        self.error(
                            CausaError::ModuloNoEncontrado { ruta: ruta.clone() },
                            linea,
                            columna,
                        );
                    }
                    return;
                };
                let modulo = match self.modulos.get(&canonical).cloned() {
                    Some(m) => m,
                    None => return,
                };

                // `importar` (legacy): importa todos los símbolos declarables del módulo.
                for (nombre, exportado) in &modulo.todos {
                    let destino = if let Some(alias) = &alias {
                        format!("{}_{}", alias, nombre)
                    } else {
                        nombre.clone()
                    };
                    if self.existe_en_ambito_actual(&destino)
                        || self.funciones.contains_key(&destino)
                    {
                        // En importación masiva legacy, evitar duplicados ruidosos.
                        continue;
                    }
                    match exportado {
                        ExportadoSem::Variable(t) => {
                            self.tabla.registrar(
                                destino.clone(),
                                self.gestor.actual(),
                                CategoriaSimbodo::Variable,
                                false,
                            );
                            self.scopes.last_mut().unwrap().insert(
                                destino,
                                VarInfo {
                                    tipo: t.clone(),
                                    mutable: false,
                                },
                            );
                        }
                        ExportadoSem::Funcion(sig) => {
                            self.tabla.registrar(
                                destino.clone(),
                                self.gestor.actual(),
                                CategoriaSimbodo::Funcion,
                                false,
                            );
                            self.funciones.insert(destino, sig.clone());
                        }
                    }
                }
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
                    let genericos = sig.parametros_tipo.iter().map(|p| p.id.clone()).collect();
                    TipoSem::Funcion {
                        params,
                        retorno,
                        genericos,
                    }
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
                if es_param_opaco(&ta) || es_param_opaco(&tb) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "aritmética/suma sobre parámetro de tipo opaco".to_string(),
                        },
                        1,
                        1,
                    );
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
                if es_param_opaco(&ta) || es_param_opaco(&tb) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "aritmética sobre parámetro de tipo opaco".to_string(),
                        },
                        1,
                        1,
                    );
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
                if es_param_opaco(&ta) || es_param_opaco(&tb) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "comparación de igualdad sobre parámetro de tipo opaco"
                                .to_string(),
                        },
                        1,
                        1,
                    );
                } else if !matches!(ta, TipoSem::Desconocido)
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
                if es_param_opaco(&ta) || es_param_opaco(&tb) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "comparación de orden sobre parámetro de tipo opaco"
                                .to_string(),
                        },
                        1,
                        1,
                    );
                } else if !matches!(ta, TipoSem::Desconocido)
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
                if es_param_opaco(&t) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "uso lógico sobre parámetro de tipo opaco".to_string(),
                        },
                        1,
                        1,
                    );
                } else if !matches!(t, TipoSem::Desconocido | TipoSem::Logico) {
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
                let op = if matches!(e, Expresion::Y(_, _)) {
                    "y"
                } else {
                    "o"
                };
                if es_param_opaco(&ta) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "uso lógico sobre parámetro de tipo opaco".to_string(),
                        },
                        1,
                        1,
                    );
                } else if !matches!(ta, TipoSem::Desconocido | TipoSem::Logico) {
                    self.error(
                        CausaError::OperandoNoLogico {
                            operador: op,
                            obtenido: ta.estatico(),
                        },
                        1,
                        1,
                    );
                }
                if es_param_opaco(&tb) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "uso lógico sobre parámetro de tipo opaco".to_string(),
                        },
                        1,
                        1,
                    );
                } else if !matches!(tb, TipoSem::Desconocido | TipoSem::Logico) {
                    self.error(
                        CausaError::OperandoNoLogico {
                            operador: op,
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
                argumentos_tipo,
                linea,
                columna,
            } => {
                let builtin = crate::evaluador::NOMBRES_RESERVADOS.contains(&nombre.as_str());
                let var_info = self.buscar_variable(nombre);
                let sig_map = self.funciones.get(nombre).cloned();

                // Resolución léxica conservadora (C.1) para valor/llamada.
                let var_es_funcion = var_info
                    .as_ref()
                    .is_some_and(|v| matches!(v.tipo, TipoSem::Funcion { .. }));
                let sombreado_por_no_funcion = var_info.is_some() && !var_es_funcion;
                let var_genericos: Vec<TipoParamId> = var_info
                    .as_ref()
                    .and_then(|v| match &v.tipo {
                        TipoSem::Funcion { genericos, .. } => Some(genericos.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                let callee_por_variable = if let Some(VarInfo { tipo, .. }) = var_info.clone() {
                    match tipo {
                        TipoSem::Funcion {
                            params, retorno, ..
                        } => Some((params, *retorno)),
                        _ => None,
                    }
                } else {
                    None
                };

                // Firma de función genérica declarada accesible (no sombreada por un valor
                // no-función ni por otra función). Si la variable en ámbito es una función
                // con parámetros genéricos, corresponde al enlace de la declaración.
                let sig_generica_accesible: Option<FuncionSig> = if !sombreado_por_no_funcion {
                    if !var_genericos.is_empty() {
                        sig_map.clone().filter(|s| !s.parametros_tipo.is_empty())
                    } else if var_info.is_none() {
                        sig_map.clone().filter(|s| !s.parametros_tipo.is_empty())
                    } else {
                        None
                    }
                } else {
                    None
                };

                let callee = if let Some(c) = callee_por_variable.clone() {
                    Some(c)
                } else if let Some(sig) = sig_map.clone() {
                    let params = sig
                        .parametros
                        .iter()
                        .map(|(_, t)| t.clone().unwrap_or(TipoSem::Desconocido))
                        .collect();
                    let retorno = sig.retorno.clone().unwrap_or(TipoSem::Desconocido);
                    Some((params, retorno))
                } else {
                    None
                };

                if var_info.is_none() && sig_map.is_none() && !builtin {
                    self.error(
                        CausaError::VariableNoDefinida(nombre.clone()),
                        *linea,
                        *columna,
                    );
                }

                // Analizar argumentos ordinarios primero (necesarios para inferencia).
                let mut arg_types = Vec::new();
                for a in argumentos {
                    arg_types.push(self.analizar_expresion(a));
                }

                // ===== G3: resolución de aplicaciones genéricas =====
                let mut retorno_generico_resuelto: Option<TipoSem> = None;
                if !argumentos_tipo.is_empty() {
                    // Turbofish aplicado: el callee debe ser una función genérica declarada
                    // accesible (no sombreada por un valor/callee no genérico).
                    if let Some(sig_g) = sig_generica_accesible.clone() {
                        // Aridad exacta primero (V1/ligadura estructural).
                        if argumentos_tipo.len() != sig_g.parametros_tipo.len() {
                            self.error(
                                CausaError::AridadArgumentosTipoInvalida {
                                    funcion: nombre.clone(),
                                    esperado: sig_g.parametros_tipo.len(),
                                    obtenido: argumentos_tipo.len(),
                                },
                                *linea,
                                *columna,
                            );
                        } else if let Some(sigma) = self.g3_resolver_args_tipo_explicitos(
                            nombre,
                            &sig_g,
                            argumentos_tipo,
                            *linea,
                            *columna,
                        ) {
                            // σ completa respecto a parámetros del callee.
                            let params_objetivo: Vec<TipoParamId> =
                                sig_g.parametros_tipo.iter().map(|p| p.id.clone()).collect();
                            if let Some(tp) =
                                sigma_crecimiento_polimorfico(&sigma, &params_objetivo)
                            {
                                self.error(
                                    CausaError::RecursionPolimorficaCreciente {
                                        funcion: nombre.clone(),
                                        instancia: nombre.clone(),
                                        tipos: params_objetivo
                                            .iter()
                                            .filter_map(|p| sigma.get(p).map(|t| t.nombre()))
                                            .collect(),
                                        cadena: vec![nombre.clone()],
                                        detalle: format!(
                                            "el parámetro de tipo '{}' se instancia con un tipo que lo contiene estructuralmente: {}",
                                            tp.0, tp.1
                                        ),
                                    },
                                    *linea,
                                    *columna,
                                );
                            } else if sigma_tiene_params_libres_objetivo(&sigma, &params_objetivo) {
                                self.error(
                                    CausaError::SustitucionIncompleta {
                                        funcion: nombre.clone(),
                                    },
                                    *linea,
                                    *columna,
                                );
                            } else {
                                // Aridad ordinaria.
                                if sig_g.parametros.len() != argumentos.len() {
                                    self.error(
                                        CausaError::FirmaInvalida {
                                            funcion: nombre.clone(),
                                            esperado: sig_g.parametros.len(),
                                            obtenido: argumentos.len(),
                                        },
                                        *linea,
                                        *columna,
                                    );
                                }
                                let params_sust = self.g3_sustituir_sig_params(&sig_g, &sigma);
                                let ret_sust = self.g3_sustituir_sig_retorno(&sig_g, &sigma);
                                let sigma_concreto = sigma
                                    .values()
                                    .all(|t| !crate::semantico::contiene_param_generico(t));
                                // V3: compatibilidad concreta de argumentos ordinarios.
                                self.g3_validar_compat_args(
                                    nombre,
                                    &params_sust,
                                    &arg_types,
                                    *linea,
                                    *columna,
                                    true,
                                );
                                // V3: vista semántica sustituida del cuerpo solo para aplicaciones
                                // totalmente concretas; el forwarding/dependencias no concretas se
                                // difieren a la instanciación G4 para evitar recursión no acotada.
                                if sigma_concreto {
                                    self.g3_validar_aplicacion_cuerpo(
                                        &sig_g, &sigma, *linea, *columna,
                                    );
                                }
                                // Dependencias genéricas (deterministas, sin instancias).
                                let decl_id = sig_g
                                    .parametros_tipo
                                    .first()
                                    .map(|p| p.id.propietario)
                                    .unwrap_or(SimbodoId(0));
                                self.g3_registrar_dependencia(decl_id, &sigma, &params_objetivo);
                                self.g4_registrar_solicitud(
                                    nombre,
                                    decl_id,
                                    &sigma,
                                    &params_objetivo,
                                    *linea,
                                    *columna,
                                );
                                retorno_generico_resuelto =
                                    Some(if matches!(ret_sust, TipoSem::Desconocido) {
                                        // Compatibilidad conservadora con firmas sin anotación resoluble.
                                        TipoSem::Desconocido
                                    } else {
                                        ret_sust
                                    });
                            }
                        }
                    } else {
                        // No es una función genérica declarada accesible (incluye no genérica,
                        // builtin, variable/callee sombreante, o inexistente).
                        self.error(
                            CausaError::ArgumentosTipoEnFuncionNoGenerica {
                                funcion: nombre.clone(),
                            },
                            *linea,
                            *columna,
                        );
                    }
                } else if let Some(sig_g) = sig_generica_accesible.clone() {
                    // Llamada sin argumentos de tipo a función genérica declarada: inferencia local G3.
                    if sig_g.parametros.len() != argumentos.len() {
                        // Aridad ordinaria incorrecta: diagnóstico específico, sin inferir.
                        self.error(
                            CausaError::FirmaInvalida {
                                funcion: nombre.clone(),
                                esperado: sig_g.parametros.len(),
                                obtenido: argumentos.len(),
                            },
                            *linea,
                            *columna,
                        );
                    } else {
                        let params_formales: Vec<TipoSem> = sig_g
                            .parametros
                            .iter()
                            .map(|(_, t)| t.clone().unwrap_or(TipoSem::Desconocido))
                            .collect();
                        let params_objetivo: Vec<TipoParamId> =
                            sig_g.parametros_tipo.iter().map(|p| p.id.clone()).collect();
                        match inferir_sigma(&params_formales, &arg_types, &params_objetivo) {
                            Ok(sigma) => {
                                if let Some(tp) =
                                    sigma_crecimiento_polimorfico(&sigma, &params_objetivo)
                                {
                                    self.error(
                                        CausaError::RecursionPolimorficaCreciente {
                                            funcion: nombre.clone(),
                                            instancia: nombre.clone(),
                                            tipos: params_objetivo
                                                .iter()
                                                .filter_map(|p| sigma.get(p).map(|t| t.nombre()))
                                                .collect(),
                                            cadena: vec![nombre.clone()],
                                            detalle: format!(
                                                "el parámetro de tipo '{}' se instancia con un tipo que lo contiene estructuralmente: {}",
                                                tp.0, tp.1
                                            ),
                                        },
                                        *linea,
                                        *columna,
                                    );
                                } else if sigma_tiene_params_libres_objetivo(
                                    &sigma,
                                    &params_objetivo,
                                ) {
                                    self.error(
                                        CausaError::SustitucionIncompleta {
                                            funcion: nombre.clone(),
                                        },
                                        *linea,
                                        *columna,
                                    );
                                } else {
                                    let params_sust = self.g3_sustituir_sig_params(&sig_g, &sigma);
                                    let ret_sust = self.g3_sustituir_sig_retorno(&sig_g, &sigma);
                                    let sigma_concreto = sigma
                                        .values()
                                        .all(|t| !crate::semantico::contiene_param_generico(t));
                                    // V3: compatibilidad concreta de argumentos ordinarios.
                                    self.g3_validar_compat_args(
                                        nombre,
                                        &params_sust,
                                        &arg_types,
                                        *linea,
                                        *columna,
                                        true,
                                    );
                                    // V3: vista semántica sustituida del cuerpo solo para aplicaciones
                                    // totalmente concretas; el forwarding/dependencias no concretas se
                                    // difieren a la instanciación G4 para evitar recursión no acotada.
                                    if sigma_concreto {
                                        self.g3_validar_aplicacion_cuerpo(
                                            &sig_g, &sigma, *linea, *columna,
                                        );
                                    }
                                    let decl_id = sig_g
                                        .parametros_tipo
                                        .first()
                                        .map(|p| p.id.propietario)
                                        .unwrap_or(SimbodoId(0));
                                    self.g3_registrar_dependencia(
                                        decl_id,
                                        &sigma,
                                        &params_objetivo,
                                    );
                                    self.g4_registrar_solicitud(
                                        nombre,
                                        decl_id,
                                        &sigma,
                                        &params_objetivo,
                                        *linea,
                                        *columna,
                                    );
                                    retorno_generico_resuelto =
                                        Some(if matches!(ret_sust, TipoSem::Desconocido) {
                                            TipoSem::Desconocido
                                        } else {
                                            ret_sust
                                        });
                                }
                            }
                            Err(UnificacionError::Incompleta) => {
                                self.error(
                                    CausaError::InferenciaArgumentosTipoIncompleta {
                                        funcion: nombre.clone(),
                                    },
                                    *linea,
                                    *columna,
                                );
                            }
                            Err(UnificacionError::Ambigua) => {
                                self.error(
                                    CausaError::InferenciaArgumentosTipoAmbigua {
                                        funcion: nombre.clone(),
                                    },
                                    *linea,
                                    *columna,
                                );
                            }
                            Err(UnificacionError::Conflicto) => {
                                self.error(
                                    CausaError::ConflictoTiposInferencia {
                                        funcion: nombre.clone(),
                                    },
                                    *linea,
                                    *columna,
                                );
                            }
                        }
                    }
                }

                if let Some(ret_g) = retorno_generico_resuelto {
                    ret_g
                } else if sig_generica_accesible.is_some() {
                    // Aplicación genérica no resuelta: tipo conservador.
                    TipoSem::Desconocido
                } else if let Some((params, mut retorno)) = callee {
                    // Compatibilidad: si el retorno desde ámbito es desconocido,
                    // preservar retorno conocido de la firma registrada (p.ej.
                    // inferido preexistente) para no romper contratos/regresiones.
                    if matches!(retorno, TipoSem::Desconocido)
                        && let Some(sig) = sig_map.as_ref()
                        && let Some(r) = sig.retorno.clone()
                    {
                        retorno = r;
                    }
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
                    retorno
                } else if let Some(VarInfo { tipo, .. }) = var_info {
                    // Variable conocida pero no-función.
                    if es_param_opaco(&tipo) {
                        // V2: invocación de parámetro de tipo opaco.
                        self.error(
                            CausaError::OperacionProhibidaPorOpacidad {
                                detalle: "invocación sobre parámetro de tipo opaco".to_string(),
                            },
                            *linea,
                            *columna,
                        );
                    } else if !matches!(tipo, TipoSem::Desconocido) && !builtin {
                        // Conservador: no forzar error adicional aquí para evitar
                        // falsos positivos en usos no callee conocidos no-función.
                        let _ = tipo;
                    }
                    TipoSem::Desconocido
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
                if es_param_opaco(&to) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "indexación directa sobre parámetro de tipo opaco".to_string(),
                        },
                        1,
                        1,
                    );
                    return TipoSem::Desconocido;
                }
                tipo_elemento_coleccion(&to)
            }
            Expresion::Acceso { objeto, .. } => {
                let to = self.analizar_expresion(objeto);
                if es_param_opaco(&to) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "acceso a miembros sobre parámetro de tipo opaco".to_string(),
                        },
                        1,
                        1,
                    );
                }
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
                if es_param_opaco(&callee_type) {
                    self.error(
                        CausaError::OperacionProhibidaPorOpacidad {
                            detalle: "invocación sobre parámetro de tipo opaco".to_string(),
                        },
                        *linea,
                        *columna,
                    );
                    return TipoSem::Desconocido;
                }
                match callee_type {
                    TipoSem::Funcion {
                        params, retorno, ..
                    } => {
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
                self.retorno_esperado = self.resolver_tipo_anotacion_opt(tipo_retorno);
                self.retorno_inferido = None;
                self.en_funcion = true;
                self.scopes.push(HashMap::new());
                self.gestor.entrar();
                for (pname, pta) in parametros {
                    let tipo = pta
                        .as_ref()
                        .map(|ta| self.resolver_tipo_anotacion(ta))
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
                            .map(|ta| self.resolver_tipo_anotacion(ta))
                            .unwrap_or(TipoSem::Desconocido)
                    })
                    .collect();
                let retorno = tipo_retorno
                    .as_ref()
                    .map(|ta| self.resolver_tipo_anotacion(ta))
                    .unwrap_or(TipoSem::Desconocido);
                TipoSem::Funcion {
                    params,
                    retorno: Box::new(retorno),
                    genericos: Vec::new(),
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
                // B.2: inferencia local de elemento de colección. Si algún elemento
                // carece de tipo conocido no hay evidencia suficiente; si los tipos
                // son heterogéneos sin tipo común demostrable se mantiene Desconocido.
                if tipos.iter().any(|t| matches!(t, TipoSem::Desconocido)) {
                    return TipoSem::Coleccion(Box::new(TipoSem::Desconocido));
                }
                let mut comun = tipos[0].clone();
                for t in &tipos[1..] {
                    let siguiente = tipo_comun(&comun, t);
                    if matches!(siguiente, TipoSem::Desconocido) {
                        comun = TipoSem::Desconocido;
                        break;
                    }
                    comun = siguiente;
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
        self.analizar_modulo_desde_canonical(&canonical, linea, columna)
    }

    fn analizar_modulo_desde_canonical(
        &mut self,
        canonical: &Path,
        linea: usize,
        columna: usize,
    ) -> Option<PathBuf> {
        let canonical = canonical.to_path_buf();
        if self.modulos.contains_key(&canonical) {
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
            let permitida = match s {
                Sentencia::Declaracion { .. }
                | Sentencia::Funcion { .. }
                | Sentencia::Usar { .. }
                | Sentencia::Importar { .. } => true,
                Sentencia::Exportar { sentencia, .. } => matches!(
                    sentencia.as_ref(),
                    Sentencia::Declaracion { .. } | Sentencia::Funcion { .. }
                ),
                _ => false,
            };
            if !permitida {
                let (l, c) = ubicacion_sentencia(s);
                let e = ErrorEjecucion {
                    mensaje: "sentencia no permitida en el nivel superior de un módulo".to_string(),
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
        let mut todos = HashMap::new();

        // Recolectar todos los símbolos declarables y los exportados explícitamente.
        for s in ast {
            let (inner, es_export_wrapper) = match s {
                Sentencia::Exportar { sentencia, .. } => (sentencia.as_ref(), true),
                _ => (s, false),
            };
            let es_publico = matches!(
                inner,
                Sentencia::Declaracion { publico: true, .. }
                    | Sentencia::Funcion { publico: true, .. }
            );
            let es_exportado = es_export_wrapper || es_publico;
            match inner {
                Sentencia::Declaracion { nombre, .. } => {
                    nombres.insert(nombre.clone());
                    if let Some(info) = self.buscar_variable(nombre) {
                        let sym = ExportadoSem::Variable(info.tipo);
                        todos.insert(nombre.clone(), sym.clone());
                        if es_exportado {
                            exports.insert(nombre.clone(), sym);
                        }
                    }
                }
                Sentencia::Funcion { nombre, .. } => {
                    nombres.insert(nombre.clone());
                    if let Some(sig) = self.funciones.get(nombre).cloned() {
                        let sym = ExportadoSem::Funcion(sig);
                        todos.insert(nombre.clone(), sym.clone());
                        if es_exportado {
                            exports.insert(nombre.clone(), sym);
                        }
                    }
                }
                _ => {}
            }
        }

        // Compatibilidad legacy: módulos sin marcadores explícitos (`publico`/`exportar`)
        // exponen todos sus símbolos declarables para `importar`, pero `usar` selectivo
        // debe seguir tratándolos como privados (no en `exports`).
        // No modificamos `exports` aquí para preservar privacidad de `usar`.

        self.archivo_actual = prev_archivo;
        self.funciones = prev_funciones;
        self.scopes = prev_scopes;
        ModuloInfo {
            exports,
            nombres,
            todos,
        }
    }

    pub fn buscar_variable(&self, nombre: &str) -> Option<VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(nombre) {
                return Some(v.clone());
            }
        }
        None
    }

    fn actualizar_tipo_variable(&mut self, nombre: &str, tipo: TipoSem) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(v) = scope.get_mut(nombre) {
                v.tipo = tipo;
                return;
            }
        }
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

    // ===== G2.2/G3: parámetros de tipo genéricos =====

    fn tipo_param_actual(&self, nombre: &str) -> Option<TipoSem> {
        self.parametros_tipo_scope
            .last()
            .and_then(|m| m.get(nombre).cloned())
    }

    fn resolver_tipo_anotacion(&self, t: &TipoAnotacion) -> TipoSem {
        match t {
            TipoAnotacion::Entero => TipoSem::Entero,
            TipoAnotacion::Decimal => TipoSem::Decimal,
            TipoAnotacion::Real => TipoSem::Real,
            TipoAnotacion::Logico => TipoSem::Logico,
            TipoAnotacion::Texto => TipoSem::Texto,
            TipoAnotacion::Arreglo(te, n) => {
                TipoSem::Arreglo(Box::new(self.resolver_tipo_anotacion(te)), *n)
            }
            TipoAnotacion::Lista(te) => TipoSem::Lista(Box::new(self.resolver_tipo_anotacion(te))),
            TipoAnotacion::Nulo => TipoSem::Nulo,
            TipoAnotacion::Parametro(nombre) => self
                .tipo_param_actual(nombre)
                .unwrap_or(TipoSem::Desconocido),
        }
    }

    fn resolver_tipo_anotacion_opt(&self, t: &Option<TipoAnotacion>) -> Option<TipoSem> {
        t.as_ref().map(|ta| self.resolver_tipo_anotacion(ta))
    }

    fn construir_params_tipo_decl(
        &self,
        decl_id: SimbodoId,
        parametros_tipo: &[String],
    ) -> Vec<ParamTipoDecl> {
        parametros_tipo
            .iter()
            .map(|nombre| ParamTipoDecl {
                nombre: nombre.clone(),
                id: TipoParamId {
                    propietario: decl_id,
                    nombre: nombre.clone(),
                },
            })
            .collect()
    }

    fn scope_mapa_parametros_tipo(decls: &[ParamTipoDecl]) -> HashMap<String, TipoSem> {
        decls
            .iter()
            .map(|d| (d.nombre.clone(), TipoSem::Param(d.id.clone())))
            .collect()
    }

    fn scope_mapa_sustituido(
        decls: &[ParamTipoDecl],
        sigma: &HashMap<TipoParamId, TipoSem>,
    ) -> HashMap<String, TipoSem> {
        decls
            .iter()
            .map(|d| {
                (
                    d.nombre.clone(),
                    sigma
                        .get(&d.id)
                        .cloned()
                        .unwrap_or_else(|| TipoSem::Param(d.id.clone())),
                )
            })
            .collect()
    }

    fn tipo_anotacion_tiene_param_invalido(
        t: &TipoAnotacion,
        scope: &HashMap<String, TipoSem>,
    ) -> bool {
        match t {
            TipoAnotacion::Parametro(n) => !scope.contains_key(n),
            TipoAnotacion::Arreglo(te, _) | TipoAnotacion::Lista(te) => {
                Self::tipo_anotacion_tiene_param_invalido(te, scope)
            }
            _ => false,
        }
    }

    fn nombre_tipo_anotacion_arg(t: &TipoAnotacion) -> String {
        match t {
            TipoAnotacion::Parametro(n) => n.clone(),
            TipoAnotacion::Arreglo(te, n) => {
                format!("arreglo<{}, {}>", Self::nombre_tipo_anotacion_arg(te), n)
            }
            TipoAnotacion::Lista(te) => format!("lista<{}>", Self::nombre_tipo_anotacion_arg(te)),
            TipoAnotacion::Entero => "entero".to_string(),
            TipoAnotacion::Decimal => "decimal".to_string(),
            TipoAnotacion::Real => "real".to_string(),
            TipoAnotacion::Logico => "logico".to_string(),
            TipoAnotacion::Texto => "texto".to_string(),
            TipoAnotacion::Nulo => "nulo".to_string(),
        }
    }
}

impl ContextoSemantico {
    fn g3_registrar_dependencia(
        &mut self,
        propietario: SimbodoId,
        sigma: &HashMap<TipoParamId, TipoSem>,
        params: &[TipoParamId],
    ) {
        let mut items: Vec<(String, String)> = Vec::new();
        for p in params {
            if let Some(t) = sigma.get(p) {
                items.push((format!("{}::{}", p.propietario.0, p.nombre), t.nombre()));
            }
        }
        items.sort();
        let key = format!("decl={} args={:?}", propietario.0, items);
        self.g3_dependencias.insert(key);
    }

    fn g4_registrar_solicitud(
        &mut self,
        callee_nombre: &str,
        decl_id: SimbodoId,
        sigma: &HashMap<TipoParamId, TipoSem>,
        params_objetivo: &[TipoParamId],
        linea: usize,
        columna: usize,
    ) {
        // Solo registrar si todos los parámetros objetivo están concretos/canonicalizables.
        let completa = params_objetivo.iter().all(|p| {
            sigma
                .get(p)
                .is_some_and(|t| crate::monomorfizacion::canonicalizar_tipo(t).is_some())
        });
        if !completa {
            return;
        }
        self.g4_solicitudes.push(SolicitudInstanciaG4 {
            callee_decl_id: decl_id,
            callee_nombre: callee_nombre.to_string(),
            sigma: sigma.clone(),
            linea,
            columna,
        });
    }

    fn g3_resolver_args_tipo_explicitos(
        &mut self,
        nombre: &str,
        sig_g: &FuncionSig,
        argumentos_tipo: &[TipoAnotacion],
        linea: usize,
        columna: usize,
    ) -> Option<HashMap<TipoParamId, TipoSem>> {
        if argumentos_tipo.len() != sig_g.parametros_tipo.len() {
            self.error(
                CausaError::AridadArgumentosTipoInvalida {
                    funcion: nombre.to_string(),
                    esperado: sig_g.parametros_tipo.len(),
                    obtenido: argumentos_tipo.len(),
                },
                linea,
                columna,
            );
            return None;
        }
        let scope_actual = self
            .parametros_tipo_scope
            .last()
            .cloned()
            .unwrap_or_default();
        let mut sigma: HashMap<TipoParamId, TipoSem> = HashMap::new();
        let mut ok = true;
        for (ta, decl) in argumentos_tipo.iter().zip(sig_g.parametros_tipo.iter()) {
            if Self::tipo_anotacion_tiene_param_invalido(ta, &scope_actual) {
                let arg_nombre = Self::nombre_tipo_anotacion_arg(ta);
                self.error(
                    CausaError::ArgumentoTipoInvalido {
                        funcion: nombre.to_string(),
                        argumento: arg_nombre,
                    },
                    linea,
                    columna,
                );
                ok = false;
                continue;
            }
            let t = self.resolver_tipo_anotacion(ta);
            if matches!(t, TipoSem::Desconocido) {
                // Argumento no resoluble/concreto en este contexto.
                let arg_nombre = Self::nombre_tipo_anotacion_arg(ta);
                self.error(
                    CausaError::ArgumentoTipoInvalido {
                        funcion: nombre.to_string(),
                        argumento: arg_nombre,
                    },
                    linea,
                    columna,
                );
                ok = false;
            } else {
                sigma.insert(decl.id.clone(), t);
            }
        }
        if ok { Some(sigma) } else { None }
    }

    fn g3_sustituir_sig_params(
        &self,
        sig: &FuncionSig,
        sigma: &HashMap<TipoParamId, TipoSem>,
    ) -> Vec<TipoSem> {
        sig.parametros
            .iter()
            .map(|(_, t)| {
                t.as_ref()
                    .map(|tt| sustituir_tipo(tt, sigma))
                    .unwrap_or(TipoSem::Desconocido)
            })
            .collect()
    }

    fn g3_sustituir_sig_retorno(
        &self,
        sig: &FuncionSig,
        sigma: &HashMap<TipoParamId, TipoSem>,
    ) -> TipoSem {
        sig.retorno
            .as_ref()
            .map(|t| sustituir_tipo(t, sigma))
            .unwrap_or(TipoSem::Desconocido)
    }

    fn g3_validar_compat_args(
        &mut self,
        nombre: &str,
        params_sust: &[TipoSem],
        arg_types: &[TipoSem],
        linea: usize,
        columna: usize,
        como_v3: bool,
    ) {
        for (i, (pt, at)) in params_sust.iter().zip(arg_types.iter()).enumerate() {
            if !compatible(pt, at)
                && !matches!(pt, TipoSem::Desconocido)
                && !matches!(at, TipoSem::Desconocido)
            {
                if como_v3 {
                    self.error(
                        CausaError::IncompatibilidadConcretaGenericos {
                            detalle: format!(
                                "argumento {i} de '{nombre}': se esperaba {} y se obtuvo {}",
                                pt.nombre(),
                                at.nombre()
                            ),
                        },
                        linea,
                        columna,
                    );
                } else {
                    self.error(
                        CausaError::TipoParametroIncompatible {
                            funcion: nombre.to_string(),
                            parametro: format!("arg{i}"),
                            esperado: pt.nombre(),
                            obtenido: at.nombre(),
                        },
                        linea,
                        columna,
                    );
                }
            }
        }
    }

    pub(crate) fn g3_validar_aplicacion_cuerpo(
        &mut self,
        sig: &FuncionSig,
        sigma: &HashMap<TipoParamId, TipoSem>,
        linea_llamada: usize,
        columna_llamada: usize,
    ) {
        // Guardia de recursión: evitar re-entrada infinita en V3 (p.ej. recursión directa/indirecta).
        let decl_id = sig
            .parametros_tipo
            .first()
            .map(|p| p.id.propietario.0)
            .unwrap_or(0);
        let mut key_items: Vec<(String, String)> = sig
            .parametros_tipo
            .iter()
            .filter_map(|p| sigma.get(&p.id).map(|t| (p.nombre.clone(), t.nombre())))
            .collect();
        key_items.sort();
        let v3_key = format!("{}::{:?}", decl_id, key_items);
        if !self.g3_v3_en_progreso.insert(v3_key.clone()) {
            return;
        }
        // Límite defensivo de profundidad V3 para evitar recursión no acotada
        // (p.ej. recursión polimórfica creciente). G4 detecta y diagnostica estos
        // casos durante la monomorfización; V3 se limita a una exploración acotada.
        if self.g3_v3_depth >= 8 {
            self.g3_v3_en_progreso.remove(&v3_key);
            return;
        }

        // Vista semántica sustituida (V3) sin instancias ejecutables.
        // Se analiza el cuerpo con tipos de parámetros/retorno y parámetros de tipo sustituidos.
        let prev_scopes = self.scopes.clone();
        let prev_tabla = self.tabla.clone();
        let prev_gestor = self.gestor.clone();
        let prev_funciones = self.funciones.clone();
        let prev_func = self.funcion_actual.clone();
        let prev_ret = self.retorno_esperado.clone();
        let prev_inf = self.retorno_inferido.clone();
        let prev_en = self.en_funcion;
        let prev_tp_scope = std::mem::take(&mut self.parametros_tipo_scope);
        let prev_c2_tipos = std::mem::take(&mut self.c2_retorno_tipos);
        let prev_c2_insuf = std::mem::replace(&mut self.c2_evidencia_insuficiente, false);
        let prev_c2_activo = std::mem::replace(&mut self.c2_activo, false);
        let prev_c2_resultados = std::mem::take(&mut self.c2_resultados);
        let prev_v3 = self.g3_v3_depth;
        let errores_antes = self.errores.len();

        self.g3_v3_depth += 1;
        self.en_funcion = true;
        self.funcion_actual = Some("<aplicacion generica>".to_string());
        self.retorno_inferido = None;
        self.c2_activo = false;

        // Scope de parámetros de tipo sustituido de la plantilla callee.
        if !sig.parametros_tipo.is_empty() {
            let m = Self::scope_mapa_sustituido(&sig.parametros_tipo, sigma);
            self.parametros_tipo_scope.push(m);
        }

        let params_sust: Vec<(String, Option<TipoSem>)> = sig
            .parametros_nombres
            .iter()
            .zip(sig.parametros.iter())
            .map(|(n, (_, t))| (n.clone(), t.as_ref().map(|tt| sustituir_tipo(tt, sigma))))
            .collect();
        let ret_sust = sig.retorno.as_ref().map(|t| sustituir_tipo(t, sigma));
        self.retorno_esperado = ret_sust;

        // Ámbito de parámetros ordinarios.
        self.scopes.push(HashMap::new());
        self.gestor.entrar();
        for (i, pname) in sig.parametros_nombres.iter().enumerate() {
            let tipo = params_sust
                .get(i)
                .and_then(|(_, t)| t.clone())
                .unwrap_or(TipoSem::Desconocido);
            let _id = self.tabla.registrar(
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
        // Analizar cuerpo (vista sustituida).
        for st in &sig.cuerpo {
            self.analizar_sentencia(st.clone());
        }
        self.gestor.salir();
        self.scopes.pop();

        self.g3_v3_depth = prev_v3;
        self.c2_resultados = prev_c2_resultados;
        self.c2_activo = prev_c2_activo;
        self.c2_evidencia_insuficiente = prev_c2_insuf;
        self.c2_retorno_tipos = prev_c2_tipos;
        self.parametros_tipo_scope = prev_tp_scope;
        self.en_funcion = prev_en;
        self.retorno_inferido = prev_inf;
        self.retorno_esperado = prev_ret;
        self.funcion_actual = prev_func;
        self.funciones = prev_funciones;
        self.gestor = prev_gestor;
        self.tabla = prev_tabla;
        self.scopes = prev_scopes;
        self.g3_v3_en_progreso.remove(&v3_key);

        let _ = (errores_antes, linea_llamada, columna_llamada);
    }

    /// G4: ejecuta validación semántica V3 sobre una instancia concreta ya especializada
    /// (post-sustitución estructural), reutilizando el mecanismo V3 existente.
    /// Devuelve los errores producidos por esta validación sin afectar el flujo de análisis principal.
    pub(crate) fn g4_validar_v3_instancia(
        &mut self,
        sig: &FuncionSig,
        sigma: &HashMap<TipoParamId, TipoSem>,
        linea: usize,
        columna: usize,
    ) -> Vec<ErrorEjecucion> {
        let antes = self.errores.len();
        // Limitar a la instancia actual: las dependencias transitivas reciben su propia
        // validación V3 cuando G4 las especializa/publica.
        let prev_depth = self.g3_v3_depth;
        self.g3_v3_depth = 7;
        self.g3_validar_aplicacion_cuerpo(sig, sigma, linea, columna);
        self.g3_v3_depth = prev_depth;
        if self.errores.len() > antes {
            self.errores.drain(antes..).collect()
        } else {
            Vec::new()
        }
    }
}

// ===== G3: sustitución, inferencia y dependencias genéricas =====

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnificacionError {
    Incompleta,
    #[allow(dead_code)]
    Ambigua,
    Conflicto,
}

pub fn es_tipo_concreto_resoluble(t: &TipoSem) -> bool {
    match t {
        TipoSem::Desconocido => false,
        TipoSem::Param(_) => true, // identidad resuelta (puede ser de plantilla actual en forwarding)
        TipoSem::Arreglo(te, _) | TipoSem::Lista(te) | TipoSem::Coleccion(te) => {
            es_tipo_concreto_resoluble(te)
        }
        TipoSem::Funcion {
            params,
            retorno,
            genericos: _,
        } => params.iter().all(es_tipo_concreto_resoluble) && es_tipo_concreto_resoluble(retorno),
        _ => true,
    }
}

pub fn contiene_param_de(t: &TipoSem, ids: &std::collections::HashSet<TipoParamId>) -> bool {
    match t {
        TipoSem::Param(id) => ids.contains(id),
        TipoSem::Arreglo(te, _) | TipoSem::Lista(te) | TipoSem::Coleccion(te) => {
            contiene_param_de(te, ids)
        }
        TipoSem::Funcion {
            params, retorno, ..
        } => params.iter().any(|p| contiene_param_de(p, ids)) || contiene_param_de(retorno, ids),
        _ => false,
    }
}

/// Retorna true si el tipo contiene algún parámetro genérico (`TipoSem::Param`).
pub fn contiene_param_generico(t: &TipoSem) -> bool {
    match t {
        TipoSem::Param(_) => true,
        TipoSem::Arreglo(te, _) | TipoSem::Lista(te) | TipoSem::Coleccion(te) => {
            contiene_param_generico(te)
        }
        TipoSem::Funcion {
            params,
            retorno,
            genericos,
        } => {
            !genericos.is_empty()
                || params.iter().any(contiene_param_generico)
                || contiene_param_generico(retorno)
        }
        _ => false,
    }
}

pub fn sustituir_tipo(tipo: &TipoSem, sigma: &HashMap<TipoParamId, TipoSem>) -> TipoSem {
    match tipo {
        TipoSem::Param(id) => sigma
            .get(id)
            .cloned()
            .unwrap_or_else(|| TipoSem::Param(id.clone())),
        TipoSem::Arreglo(te, n) => TipoSem::Arreglo(Box::new(sustituir_tipo(te, sigma)), *n),
        TipoSem::Lista(te) => TipoSem::Lista(Box::new(sustituir_tipo(te, sigma))),
        TipoSem::Coleccion(te) => TipoSem::Coleccion(Box::new(sustituir_tipo(te, sigma))),
        TipoSem::Funcion {
            params,
            retorno,
            genericos,
        } => TipoSem::Funcion {
            params: params.iter().map(|p| sustituir_tipo(p, sigma)).collect(),
            retorno: Box::new(sustituir_tipo(retorno, sigma)),
            genericos: genericos.clone(),
        },
        _ => tipo.clone(),
    }
}

fn tipos_iguales_estructura(a: &TipoSem, b: &TipoSem) -> bool {
    a == b
}

pub fn unificar(
    formal: &TipoSem,
    observado: &TipoSem,
    params_objetivo: &std::collections::HashSet<TipoParamId>,
    sigma: &mut HashMap<TipoParamId, TipoSem>,
) -> Result<(), UnificacionError> {
    match formal {
        TipoSem::Param(id) if params_objetivo.contains(id) => {
            if !es_tipo_concreto_resoluble(observado) {
                return Err(UnificacionError::Incompleta);
            }
            if let Some(prev) = sigma.get(id).cloned() {
                // Ambigüedad: mismo parámetro unificado con tipos estructuralmente distintos
                // (p.ej. entero vs decimal). Conflicto: mismo "molde" imposible se detecta
                // también como diferencia estructural; se prioriza conflicto cuando hay
                // imposibilidad evidente? aquí usamos desigualdad estructural -> ambiguo/conflicto
                // según especificación; diferenciamos: si ambos son concretos incompatibles en
                // el mismo sitio de restricción directa -> conflicto/ambiguo no trivial.
                if !tipos_iguales_estructura(&prev, observado) {
                    // Mismo parámetro con dos soluciones concretas distintas -> ambiguo/conflicto.
                    // "Inferencia ambigua" (varias soluciones) vs "conflicto de tipos".
                    // Dos restricciones distintas exigen valores distintos => conflicto.
                    return Err(UnificacionError::Conflicto);
                }
                Ok(())
            } else {
                sigma.insert(id.clone(), observado.clone());
                Ok(())
            }
        }
        TipoSem::Param(_) => {
            // Parámetro de otra plantilla (forwarding/externo al objetivo): debe coincidir
            // estructuralmente con lo observado para ser consistente.
            if tipos_iguales_estructura(formal, observado) {
                Ok(())
            } else {
                Err(UnificacionError::Conflicto)
            }
        }
        TipoSem::Lista(tf) => match observado {
            TipoSem::Lista(to) | TipoSem::Arreglo(to, _) | TipoSem::Coleccion(to) => {
                // Inferencia estructural autorizada para lista<T> (contenedor conocido).
                unificar(tf, to, params_objetivo, sigma)
            }
            TipoSem::Desconocido => Err(UnificacionError::Incompleta),
            _ => Err(UnificacionError::Conflicto),
        },
        TipoSem::Arreglo(tf, nf) => match observado {
            TipoSem::Arreglo(to, no) if nf == no => unificar(tf, to, params_objetivo, sigma),
            // Permitir unificación con Coleccion solo si tamaño no contradice? Coleccion no porta tamaño.
            TipoSem::Coleccion(to) => unificar(tf, to, params_objetivo, sigma),
            TipoSem::Desconocido => Err(UnificacionError::Incompleta),
            _ => Err(UnificacionError::Conflicto),
        },
        TipoSem::Coleccion(tf) => match observado {
            TipoSem::Coleccion(to) | TipoSem::Lista(to) | TipoSem::Arreglo(to, _) => {
                unificar(tf, to, params_objetivo, sigma)
            }
            TipoSem::Desconocido => Err(UnificacionError::Incompleta),
            _ => Err(UnificacionError::Conflicto),
        },
        TipoSem::Funcion {
            params: pf,
            retorno: rf,
            ..
        } => match observado {
            TipoSem::Funcion {
                params: po,
                retorno: ro,
                ..
            } if pf.len() == po.len() => {
                for (a, b) in pf.iter().zip(po.iter()) {
                    unificar(a, b, params_objetivo, sigma)?;
                }
                unificar(rf, ro, params_objetivo, sigma)
            }
            TipoSem::Desconocido => Err(UnificacionError::Incompleta),
            _ => Err(UnificacionError::Conflicto),
        },
        // Primitivos/Nulo: deben coincidir estructuralmente (con compatibilidad numérica no
        // usada para unificación de igualdad estructural; la igualdad estructural es estricta
        // salvo contenedores autorizados arriba).
        _ => {
            if matches!(observado, TipoSem::Desconocido) {
                // Si el formal no depende de params objetivo y observado es desconocido, no hay info.
                if contiene_param_de(formal, params_objetivo) {
                    Err(UnificacionError::Incompleta)
                } else {
                    Ok(())
                }
            } else if tipos_iguales_estructura(formal, observado) {
                Ok(())
            } else {
                // Diferencia estructural en correspondencia formal/observado: conflicto.
                Err(UnificacionError::Conflicto)
            }
        }
    }
}

pub fn inferir_sigma(
    params_formales: &[TipoSem],
    args_observados: &[TipoSem],
    params_tipo: &[TipoParamId],
) -> Result<HashMap<TipoParamId, TipoSem>, UnificacionError> {
    let objetivo: std::collections::HashSet<TipoParamId> = params_tipo.iter().cloned().collect();
    let mut sigma: HashMap<TipoParamId, TipoSem> = HashMap::new();
    for (f, o) in params_formales.iter().zip(args_observados.iter()) {
        unificar(f, o, &objetivo, &mut sigma)?;
    }
    // Completa: todos los parámetros de tipo deben estar asignados.
    for p in params_tipo {
        if !sigma.contains_key(p) {
            return Err(UnificacionError::Incompleta);
        }
    }
    // Sin argumentos parcialmente resueltos / sin Desconocido no concretable.
    for v in sigma.values() {
        if !es_tipo_concreto_resoluble(v) {
            return Err(UnificacionError::Incompleta);
        }
    }
    Ok(sigma)
}

pub fn sigma_tiene_params_libres_objetivo(
    sigma: &HashMap<TipoParamId, TipoSem>,
    objetivo: &[TipoParamId],
) -> bool {
    let set: std::collections::HashSet<TipoParamId> = objetivo.iter().cloned().collect();
    sigma.iter().any(|(k, v)| {
        // Ligadura identidad (forwarding monomórfico p -> p): permitida como completa.
        if let TipoSem::Param(id) = v
            && id == k
            && set.contains(k)
        {
            return false;
        }
        contiene_param_de(v, &set)
    })
}

/// Detecta crecimiento polimórfico en σ: un parámetro objetivo T se liga a un
/// tipo que contiene estructuralmente a T (p.ej. T -> lista<T>), lo que implica
/// recursión polimórfica. La ligadura identidad T -> T no se considera crecimiento.
pub fn sigma_crecimiento_polimorfico(
    sigma: &HashMap<TipoParamId, TipoSem>,
    objetivo: &[TipoParamId],
) -> Option<(String, String)> {
    for p in objetivo {
        if let Some(v) = sigma.get(p) {
            if let TipoSem::Param(id) = v
                && id == p
            {
                continue;
            }
            let mut set = std::collections::HashSet::new();
            set.insert(p.clone());
            if contiene_param_de(v, &set) {
                return Some((p.nombre.clone(), v.nombre()));
            }
        }
    }
    None
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
        // G2: referencias a parámetros de tipo aún sin semántica de genéricos.
        TipoAnotacion::Parametro(_) => TipoSem::Desconocido,
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
        (TipoSem::Param(p1), TipoSem::Param(p2)) => p1 == p2,
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
                genericos: g1,
            },
            TipoSem::Funcion {
                params: p2,
                retorno: r2,
                genericos: g2,
            },
        ) => {
            g1 == g2
                && p1.len() == p2.len()
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
    // G2.2 (opacidad): sin capacidades/bounds no se habilita igualdad sobre Param.
    if matches!(a, TipoSem::Param(_)) || matches!(b, TipoSem::Param(_)) {
        return false;
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
    // G2.2 (opacidad): sin capacidades/bounds no se habilitan comparaciones sobre Param.
    if matches!(a, TipoSem::Param(_)) || matches!(b, TipoSem::Param(_)) {
        return false;
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
        (TipoSem::Param(p1), TipoSem::Param(p2)) if p1 == p2 => TipoSem::Param(p1.clone()),
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

fn es_param_opaco(t: &TipoSem) -> bool {
    matches!(t, TipoSem::Param(_))
}

fn tipo_elemento_coleccion(t: &TipoSem) -> TipoSem {
    match t {
        TipoSem::Arreglo(elem, _) | TipoSem::Lista(elem) | TipoSem::Coleccion(elem) => {
            (**elem).clone()
        }
        _ => TipoSem::Desconocido,
    }
}

fn tipo_elemento_iterable(t: &TipoSem) -> TipoSem {
    tipo_elemento_coleccion(t)
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

    /// Consumidor semántico C.2: recupera el resultado de retorno inferido para
    /// una declaración específica por su identidad estable (`SimbodoId`).
    ///
    /// - `None`: no existe resultado C.2 registrado para esa declaración
    ///   (ausente/no candidata/no registrada como resultado C.2).
    /// - `Some(None)`: resultado C.2 presente y conservadoramente `Desconocido`.
    /// - `Some(Some(t))`: resultado C.2 presente con tipo inferido conocido.
    ///
    /// Canal separado de C.1: no altera `FuncionSig.retorno`, diagnósticos ni
    /// ejecución.
    #[allow(dead_code)]
    fn consultar_c2_retorno(&self, decl_id: SimbodoId) -> Option<Option<TipoSem>> {
        self.c2_resultados.get(&decl_id).map(|r| r.tipo.clone())
    }

    fn c2_registrar_tipo(&mut self, t: TipoSem) {
        if self.c2_activo {
            if Self::c2_tipo_conocido(&t) {
                self.c2_retorno_tipos.push(t);
            } else {
                self.c2_evidencia_insuficiente = true;
            }
        }
    }

    fn c2_tipo_conocido(t: &TipoSem) -> bool {
        match t {
            TipoSem::Desconocido | TipoSem::Param(_) => false,
            TipoSem::Arreglo(te, _) | TipoSem::Lista(te) | TipoSem::Coleccion(te) => {
                Self::c2_tipo_conocido(te)
            }
            TipoSem::Funcion {
                params, retorno, ..
            } => params.iter().all(Self::c2_tipo_conocido) && Self::c2_tipo_conocido(retorno),
            _ => true,
        }
    }

    fn c2_marcar_insuficiente(&mut self) {
        if self.c2_activo {
            self.c2_evidencia_insuficiente = true;
        }
    }

    fn c2_unificar(tipos: &[TipoSem]) -> Option<TipoSem> {
        let mut it = tipos.iter();
        let mut acc = it.next()?.clone();
        for t in it {
            let u = tipo_comun(&acc, t);
            if matches!(u, TipoSem::Desconocido) {
                return None;
            }
            acc = u;
        }
        Some(acc)
    }

    fn c2_finalizar(
        &mut self,
        decl_id: SimbodoId,
        nombre: &str,
        cuerpo: &[Sentencia],
        es_candidato: bool,
    ) {
        if !es_candidato {
            return;
        }
        let resultado = if self.c2_evidencia_insuficiente
            || self.c2_retorno_tipos.is_empty()
            || !Self::bloque_definitivamente_retorna(cuerpo)
        {
            None
        } else {
            Self::c2_unificar(&self.c2_retorno_tipos)
        };
        self.c2_resultados.insert(
            decl_id,
            C2ResultadoDecl {
                nombre: nombre.to_string(),
                tipo: resultado,
            },
        );
    }

    fn bloque_definitivamente_retorna(stmts: &[Sentencia]) -> bool {
        stmts.iter().any(Self::sentencia_definitivamente_retorna)
    }

    fn sentencia_definitivamente_retorna(s: &Sentencia) -> bool {
        match s {
            Sentencia::Retorno { .. } => true,
            Sentencia::Bloque(stmts) => Self::bloque_definitivamente_retorna(stmts),
            Sentencia::Si {
                rama_si,
                rama_sino: Some(sino),
                ..
            } => {
                Self::bloque_definitivamente_retorna(rama_si)
                    && Self::bloque_definitivamente_retorna(sino)
            }
            // Conservador: bucles, intentar/rescatar, si-sin-sino y otras formas
            // no demuestran retorno en todos los caminos relevantes.
            _ => false,
        }
    }

    fn c2_llamada_admisible(&self, nombre: &str) -> bool {
        if let Some(VarInfo { tipo, .. }) = self.buscar_variable(nombre) {
            if let TipoSem::Funcion { retorno, .. } = tipo {
                // En ámbito, el retorno conocido proviene de anotación explícita
                // (la firma de ámbito no se actualiza con inferencia de cuerpo).
                return !matches!(retorno.as_ref(), TipoSem::Desconocido);
            }
            // Sombreado por valor no-función: no hay firma explícita utilizable.
            return false;
        }
        if let Some(sig) = self.funciones.get(nombre) {
            return sig.retorno_anotado;
        }
        false
    }

    fn c2_expr_admisible(&self, e: &Expresion) -> bool {
        match e {
            Expresion::Literal { .. } | Expresion::Variable { .. } | Expresion::Rango { .. } => {
                true
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
            | Expresion::O(a, b) => self.c2_expr_admisible(a) && self.c2_expr_admisible(b),
            Expresion::No(x) => self.c2_expr_admisible(x),
            Expresion::Llamada {
                nombre, argumentos, ..
            } => {
                self.c2_llamada_admisible(nombre)
                    && argumentos.iter().all(|a| self.c2_expr_admisible(a))
            }
            // Llamadas indirectas y accesos: conservador para D5 (origen de la
            // firma/retorno no demostrable como anotación explícita local).
            Expresion::LlamadaIndirecta { .. } | Expresion::Acceso { .. } => false,
            Expresion::Indice { objeto, indice, .. } => {
                self.c2_expr_admisible(objeto) && self.c2_expr_admisible(indice)
            }
            Expresion::ArregloLiteral { elementos, .. } => {
                elementos.iter().all(|x| self.c2_expr_admisible(x))
            }
            // Lambdas excluidas de inferencia C.2; tratadas conservadoramente.
            Expresion::Lambda { .. } => false,
        }
    }

    /// Ejecuta monomorfización G4 (Arquitectura C) sobre un programa ya analizado.
    /// Devuelve el programa especializado y los errores de monomorfización.
    pub fn monomorfizar_programa(
        &mut self,
        programa: &[Sentencia],
    ) -> (Vec<Sentencia>, Vec<ErrorEjecucion>) {
        let modulo = self
            .archivo_actual
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| self.archivo.clone())
            .unwrap_or_default();
        let funciones = self.funciones.clone();
        let solicitudes = self.g4_solicitudes.clone();
        let archivo = self.archivo.clone();
        let mut validador = |sig: &FuncionSig,
                             sigma: &HashMap<TipoParamId, TipoSem>,
                             linea: usize,
                             columna: usize|
         -> Vec<ErrorEjecucion> {
            self.g4_validar_v3_instancia(sig, sigma, linea, columna)
        };
        crate::monomorfizacion::ejecutar_g4(
            programa,
            &funciones,
            &solicitudes,
            archivo.as_deref(),
            &modulo,
            Some(&mut validador),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lexer, Parser};

    fn analizar_ctx(fuente: &str) -> (ContextoSemantico, Vec<ErrorEjecucion>) {
        let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico");
        let ss = Parser::nuevo(tokens).parsear_programa().expect("sintaxis");
        let mut ctx = ContextoSemantico::nuevo();
        let errs = ctx.analizar_programa(&ss);
        (ctx, errs)
    }

    fn c2_entradas(ctx: &ContextoSemantico, nombre: &str) -> Vec<(SimbodoId, Option<TipoSem>)> {
        let mut v: Vec<(SimbodoId, Option<TipoSem>)> = ctx
            .c2_resultados
            .iter()
            .filter(|(_, r)| r.nombre == nombre)
            .map(|(id, _)| {
                // Consumo a través del consumidor semántico C.2 por identidad.
                let res = ctx
                    .consultar_c2_retorno(*id)
                    .expect("entrada C.2 presente debe ser consultable");
                (*id, res)
            })
            .collect();
        v.sort_by_key(|(id, _)| id.0);
        v
    }

    fn c2_unico(ctx: &ContextoSemantico, nombre: &str) -> Option<TipoSem> {
        let v = c2_entradas(ctx, nombre);
        assert_eq!(
            v.len(),
            1,
            "se esperaba 1 resultado C.2 para '{nombre}', encontrados: {:?}",
            v
        );
        v[0].1.clone()
    }

    fn c2(ctx: &ContextoSemantico, nombre: &str) -> Option<TipoSem> {
        c2_unico(ctx, nombre)
    }

    #[test]
    fn c2_retorno_explicito_unico() {
        let (ctx, e) = analizar_ctx("funcion f() { retornar 1 }\nf()");
        assert!(e.is_empty(), "errores: {:?}", e);
        assert_eq!(c2(&ctx, "f"), Some(TipoSem::Entero));
    }

    #[test]
    fn c2_varios_retornos_mismo_tipo_caminos_completos() {
        let (ctx, e) = analizar_ctx(
            "funcion f(x: logico) { si x { retornar 1 } sino { retornar 2 } }\nf(verdadero)",
        );
        assert!(e.is_empty(), "errores: {:?}", e);
        assert_eq!(c2(&ctx, "f"), Some(TipoSem::Entero));
    }

    #[test]
    fn c2_retornos_compatibles_unificables() {
        let (ctx, e) = analizar_ctx(
            "funcion f(x: logico) { si x { retornar 1 } sino { retornar 1.5 } }\nf(verdadero)",
        );
        // Puede existir diagnóstico legado por tipado de retornos (D0: no se altera);
        // C.2 computa unificación conservadora en canal separado.
        let _ = e;
        assert_eq!(c2(&ctx, "f"), Some(TipoSem::Decimal));
    }

    #[test]
    fn c2_callee_anotado_explicito_visible() {
        let (ctx, e) = analizar_ctx(
            "funcion g() -> entero { retornar 1 }\nfuncion f(x: logico) { si x { retornar g() } sino { retornar g() } }\nf(verdadero)",
        );
        assert!(e.is_empty(), "errores: {:?}", e);
        assert_eq!(c2(&ctx, "f"), Some(TipoSem::Entero));
    }

    #[test]
    fn c2_prioridad_anotacion_explicita() {
        let (ctx, e) = analizar_ctx("funcion f() -> entero { retornar 1 }\nf()");
        assert!(e.is_empty(), "errores: {:?}", e);
        assert!(
            c2_entradas(&ctx, "f").is_empty(),
            "función anotada no debe producir resultado C.2 separado"
        );
        let sig = ctx.funciones.get("f").expect("firma");
        assert!(sig.retorno_anotado);
        assert_eq!(sig.retorno, Some(TipoSem::Entero));
    }

    #[test]
    fn c2_caida_implicita_desconocido() {
        let (ctx, e) = analizar_ctx("funcion f() { sea x = 1 }");
        assert!(e.is_empty(), "errores: {:?}", e);
        assert_eq!(c2(&ctx, "f"), None);
    }

    #[test]
    fn c2_retorno_vacio_desconocido() {
        let (ctx, e) = analizar_ctx("funcion f() { retornar }");
        // Se preserva semántica/diagnóstico legado: sin anotación, el retorno
        // vacío no debe generar diagnóstico estático nuevo aquí.
        assert!(e.is_empty(), "errores: {:?}", e);
        assert_eq!(c2(&ctx, "f"), None);
    }

    #[test]
    fn c2_retorno_vacio_mezclado_desconocido() {
        let (ctx, e) =
            analizar_ctx("funcion f(x: logico) { si x { retornar } sino { retornar 1 } }");
        // Se preserva comportamiento/diagnóstico legado preexistente (D0/D4).
        let _ = e;
        assert_eq!(c2(&ctx, "f"), None);
    }

    #[test]
    fn c2_retornos_incompatibles_desconocido() {
        let (ctx, e) =
            analizar_ctx("funcion f(x: logico) { si x { retornar 1 } sino { retornar \"a\" } }");
        // Diagnóstico legado por tipos de retorno incompatibles puede existir;
        // C.2 debe ser conservador.
        let _ = e;
        assert_eq!(c2(&ctx, "f"), None);
    }

    #[test]
    fn c2_expresion_opaca_desconocido() {
        let (ctx, _e) = analizar_ctx("funcion f() { retornar longitud([1, 2]) }\nf()");
        assert_eq!(c2(&ctx, "f"), None);
    }

    #[test]
    fn c2_callee_sin_anotacion_desconocido_d5() {
        let (ctx, e) = analizar_ctx(
            "funcion g() { retornar 1 }\nfuncion f(x: logico) { si x { retornar g() } sino { retornar g() } }\nf(verdadero)",
        );
        assert!(e.is_empty(), "errores: {:?}", e);
        // C.2 no consume retorno inferido del cuerpo de g (D5).
        assert_eq!(c2(&ctx, "f"), None);
        // Compatibilidad legada C.1 preservada: g puede tener retorno inferido en firma.
        let sig_g = ctx.funciones.get("g").expect("firma g");
        assert!(!sig_g.retorno_anotado);
        assert_eq!(sig_g.retorno, Some(TipoSem::Entero));
    }

    #[test]
    fn c2_recursion_sin_evidencia_independiente() {
        let (ctx, _e) = analizar_ctx("funcion f(n: entero) { retornar f(n) }\nf(1)");
        assert_eq!(c2(&ctx, "f"), None);
    }

    #[test]
    fn c2_dependencia_circular_desconocido() {
        let (ctx, e) = analizar_ctx(
            "funcion a(x: logico) { si x { retornar b(x) } sino { retornar b(x) } }\nfuncion b(x: logico) { si x { retornar a(x) } sino { retornar a(x) } }\na(verdadero)",
        );
        assert!(e.is_empty(), "errores: {:?}", e);
        assert_eq!(c2(&ctx, "a"), None);
        assert_eq!(c2(&ctx, "b"), None);
    }

    #[test]
    fn c2_camino_incompleto_si_sin_sino() {
        let (ctx, e) = analizar_ctx("funcion f(x: logico) { si x { retornar 1 } }\nf(verdadero)");
        assert!(e.is_empty(), "errores: {:?}", e);
        assert_eq!(c2(&ctx, "f"), None);
    }

    #[test]
    fn c2_ambiguedad_tipo_coleccion_heterogenea() {
        let (ctx, _e) = analizar_ctx("funcion f() { retornar [1, \"a\"] }\nf()");
        assert_eq!(c2(&ctx, "f"), None);
    }

    #[test]
    fn c2_preserva_propagacion_y_fallback_legado_c1() {
        // Legado: retorno inferido de g (no anotada) se propaga a FuncionSig y
        // puede usarse como fallback en resolución de llamadas.
        let (ctx, e) =
            analizar_ctx("funcion g() { retornar 1 }\nfuncion h() -> entero { retornar g() }\nh()");
        assert!(e.is_empty(), "errores: {:?}", e);
        let sig_g = ctx.funciones.get("g").expect("firma g");
        assert!(!sig_g.retorno_anotado);
        assert_eq!(sig_g.retorno, Some(TipoSem::Entero));
        // h anotada: sin resultado C.2; su análisis debe seguir siendo válido.
        assert!(c2_entradas(&ctx, "h").is_empty());
    }

    #[test]
    fn c2_funcion_anidada_no_fuga_resultado() {
        let (ctx, e) = analizar_ctx(
            "funcion externa() {\n  funcion interna() { retornar 1 }\n  interna()\n}\nexterna()",
        );
        assert!(e.is_empty(), "errores: {:?}", e);
        // interna analizada: C.2 puede registrarla; externa sin retornos explícitos.
        assert_eq!(c2(&ctx, "interna"), Some(TipoSem::Entero));
        assert_eq!(c2(&ctx, "externa"), None);
        // Firma global de interna restaurada/eliminada tras salir del ámbito contenedor.
        assert!(!ctx.funciones.contains_key("interna"));
    }

    fn tipos_por_nombre(ctx: &ContextoSemantico, nombre: &str) -> Vec<Option<TipoSem>> {
        c2_entradas(ctx, nombre)
            .into_iter()
            .map(|(_, t)| t)
            .collect()
    }

    fn ids_por_nombre(ctx: &ContextoSemantico, nombre: &str) -> Vec<SimbodoId> {
        c2_entradas(ctx, nombre)
            .into_iter()
            .map(|(id, _)| id)
            .collect()
    }

    #[test]
    fn c2_identidad_homonimas_anidadas_independientes() {
        // Caso 1 — Funciones anidadas homónimas
        let src = r#"
funcion externa_a() {
  funcion calcular() { retornar 100 }
  calcular()
}
funcion externa_b() {
  funcion calcular() { retornar "texto" }
  calcular()
}
externa_a()
externa_b()
"#;
        let (ctx, e) = analizar_ctx(src);
        assert!(e.is_empty(), "errores: {:?}", e);
        let ids = ids_por_nombre(&ctx, "calcular");
        assert_eq!(ids.len(), 2, "deben existir 2 declaraciones 'calcular'");
        assert_ne!(
            ids[0], ids[1],
            "identidades de declaración deben ser distintas"
        );
        let mut tipos = tipos_por_nombre(&ctx, "calcular");
        tipos.sort_by_key(|t| format!("{:?}", t));
        assert_eq!(
            tipos,
            vec![Some(TipoSem::Entero), Some(TipoSem::Texto)],
            "resultados independientes por declaración"
        );
        assert_eq!(c2(&ctx, "externa_a"), None);
        assert_eq!(c2(&ctx, "externa_b"), None);
    }

    #[test]
    fn c2_identidad_homonimas_orden_inverso() {
        // Caso 2 — Orden inverso
        let src = r#"
funcion externa_b() {
  funcion calcular() { retornar "texto" }
  calcular()
}
funcion externa_a() {
  funcion calcular() { retornar 100 }
  calcular()
}
externa_b()
externa_a()
"#;
        let (ctx, e) = analizar_ctx(src);
        assert!(e.is_empty(), "errores: {:?}", e);
        let ids = ids_por_nombre(&ctx, "calcular");
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
        let mut tipos = tipos_por_nombre(&ctx, "calcular");
        tipos.sort_by_key(|t| format!("{:?}", t));
        assert_eq!(tipos, vec![Some(TipoSem::Entero), Some(TipoSem::Texto)]);
    }

    #[test]
    fn c2_identidad_tres_homonimas_distintas() {
        // Caso 3 — Tres declaraciones homónimas
        let src = "funcion a() {\n  funcion x() { retornar 1 }\n  x()\n}\nfuncion b() {\n  funcion x() { retornar 2 }\n  x()\n}\nfuncion c() {\n  funcion x() { retornar 3 }\n  x()\n}\na()\nb()\nc()\n";
        let (ctx, e) = analizar_ctx(src);
        assert!(e.is_empty(), "errores: {:?}", e);
        let ids = ids_por_nombre(&ctx, "x");
        assert_eq!(ids.len(), 3, "deben existir 3 identidades distintas");
        let uniq: std::collections::HashSet<_> = ids.iter().copied().collect();
        assert_eq!(uniq.len(), 3, "ninguna identidad debe colisionar");
        let mut tipos = tipos_por_nombre(&ctx, "x");
        tipos.sort_by_key(|t| format!("{:?}", t));
        assert_eq!(
            tipos,
            vec![
                Some(TipoSem::Entero),
                Some(TipoSem::Entero),
                Some(TipoSem::Entero)
            ]
        );
    }

    #[test]
    fn c2_identidad_anidacion_multinivel_sin_colisiones() {
        // Caso 4 — Anidación de varios niveles
        let src = "funcion externa1() {\n  funcion f() {\n    funcion g() { retornar 1 }\n    g()\n    retornar 1\n  }\n  retornar f()\n}\nfuncion externa2() {\n  funcion g() { retornar \"ok\" }\n  g()\n}\nexterna1()\nexterna2()\n";
        let (ctx, e) = analizar_ctx(src);
        assert!(e.is_empty(), "errores: {:?}", e);
        let g_ids = ids_por_nombre(&ctx, "g");
        assert_eq!(g_ids.len(), 2, "dos declaraciones 'g' en ámbitos distintos");
        assert_ne!(g_ids[0], g_ids[1]);
        let mut g_tipos = tipos_por_nombre(&ctx, "g");
        g_tipos.sort_by_key(|t| format!("{:?}", t));
        assert_eq!(g_tipos, vec![Some(TipoSem::Entero), Some(TipoSem::Texto)]);
        assert_eq!(c2(&ctx, "f"), Some(TipoSem::Entero));
    }

    #[test]
    fn c2_identidad_desconocido_independiente_no_sobrescribe() {
        // Caso 5 — Resultado desconocido independiente
        let src = r#"
funcion externa_a() {
  funcion h() { retornar 10 }
  h()
}
funcion externa_b() {
  funcion h() { sea x = 1 }
  h()
}
externa_a()
externa_b()
"#;
        let (ctx, e) = analizar_ctx(src);
        assert!(e.is_empty(), "errores: {:?}", e);
        let h_ids = ids_por_nombre(&ctx, "h");
        assert_eq!(h_ids.len(), 2);
        assert_ne!(h_ids[0], h_ids[1]);
        let mut h_tipos = tipos_por_nombre(&ctx, "h");
        h_tipos.sort_by_key(|t| format!("{:?}", t));
        assert_eq!(h_tipos, vec![None, Some(TipoSem::Entero)]);
    }

    #[test]
    fn c2_identidad_independiente_de_restauracion_firma() {
        // Caso 6 — Restauración de ámbito
        let src = r#"
funcion externa() {
  funcion interna() { retornar 7 }
  interna()
}
externa()
"#;
        let (ctx, e) = analizar_ctx(src);
        assert!(e.is_empty(), "errores: {:?}", e);
        // La firma de la anidada se restaura/elimina del mapa global por nombre.
        assert!(!ctx.funciones.contains_key("interna"));
        // El resultado C.2 de esa declaración persiste por identidad propia.
        let interna = c2_entradas(&ctx, "interna");
        assert_eq!(interna.len(), 1);
        assert_eq!(interna[0].1, Some(TipoSem::Entero));
        // Identidad no es la firma global (ya ausente) sino la declaración registrada.
        // Consumidor semántico recupera por identidad de declaración.
        assert_eq!(
            ctx.consultar_c2_retorno(interna[0].0),
            Some(Some(TipoSem::Entero))
        );
    }

    #[test]
    fn c2_regresion_c1_propagacion_y_fallback_intactos() {
        // Caso 7 — Regresión de C.1 (preserva propagación/fallback legado)
        let (ctx, e) = analizar_ctx(
            "funcion g() { retornar 1 }\nfuncion h() -> entero { retornar g() }\nh()\n",
        );
        assert!(e.is_empty(), "errores: {:?}", e);
        let sig_g = ctx.funciones.get("g").expect("firma g");
        assert!(!sig_g.retorno_anotado);
        assert_eq!(
            sig_g.retorno,
            Some(TipoSem::Entero),
            "propagación C.1 preservada"
        );
        // Fallback legado: h anotada consume g (con retorno inferido propagado) sin error.
        let sig_h = ctx.funciones.get("h").expect("firma h");
        assert!(sig_h.retorno_anotado);
        assert_eq!(sig_h.retorno, Some(TipoSem::Entero));
        assert!(c2_entradas(&ctx, "h").is_empty());

        // Anidada: restauración de firma previa preservada.
        let (ctx2, e2) = analizar_ctx(
            "funcion interna() { retornar 9 }\nfuncion externa() {\n  funcion interna() { retornar 1 }\n  interna()\n}\nexterna()\ninterna()\n",
        );
        assert!(e2.is_empty(), "errores: {:?}", e2);
        let sig_interna_global = ctx2.funciones.get("interna").expect("firma global interna");
        assert_eq!(sig_interna_global.retorno, Some(TipoSem::Entero));
    }

    #[test]
    fn c2_regresion_casos_positivos_y_negativos_mision_0618() {
        // Caso 8 — Regresión de C.2 (positivos y negativos de 06.18)
        // Positivos
        let (ctx, e) = analizar_ctx(
            "funcion a() { retornar 1 }\nfuncion b(x: logico) { si x { retornar 1 } sino { retornar 2 } }\nfuncion c(x: logico) { si x { retornar 1 } sino { retornar 1.5 } }\nfuncion g() -> entero { retornar 1 }\nfuncion d(x: logico) { si x { retornar g() } sino { retornar g() } }\na()\nb(verdadero)\nc(verdadero)\nd(verdadero)",
        );
        let _ = e; // D0: c() puede emitir diagnóstico legado; C.2 separado
        assert_eq!(c2(&ctx, "a"), Some(TipoSem::Entero));
        assert_eq!(c2(&ctx, "b"), Some(TipoSem::Entero));
        assert_eq!(c2(&ctx, "c"), Some(TipoSem::Decimal));
        assert_eq!(c2(&ctx, "d"), Some(TipoSem::Entero));

        // Negativos
        let (ctxn, _en) = analizar_ctx(
            "funcion n1() { sea x = 1 }\nfuncion n2() { retornar }\nfuncion n3(x: logico) { si x { retornar } sino { retornar 1 } }\nfuncion n4(x: logico) { si x { retornar 1 } sino { retornar \"a\" } }\nfuncion n5() { retornar longitud([1, 2]) }\nfuncion g2() { retornar 1 }\nfuncion n6(x: logico) { si x { retornar g2() } sino { retornar g2() } }\nfuncion n7(n: entero) { retornar n7(n) }\nfuncion n8(x: logico) { si x { retornar 1 } }\nfuncion n9() { retornar [1, \"a\"] }\nn5()\nn6(verdadero)\nn7(1)\nn8(verdadero)\nn9()",
        );
        assert_eq!(c2(&ctxn, "n1"), None);
        assert_eq!(c2(&ctxn, "n2"), None);
        assert_eq!(c2(&ctxn, "n3"), None);
        assert_eq!(c2(&ctxn, "n4"), None);
        assert_eq!(c2(&ctxn, "n5"), None);
        assert_eq!(c2(&ctxn, "n6"), None);
        assert_eq!(c2(&ctxn, "n7"), None);
        assert_eq!(c2(&ctxn, "n8"), None);
        assert_eq!(c2(&ctxn, "n9"), None);
        // D5 preservado en regresión: g2 tiene C.1 inferido pero no es evidencia C.2.
        let sig_g2 = ctxn.funciones.get("g2").expect("firma g2");
        assert!(!sig_g2.retorno_anotado);
        assert_eq!(sig_g2.retorno, Some(TipoSem::Entero));
    }

    #[test]
    fn c2_identidad_ambito_actual_mismo_nombre_no_colisiona() {
        // Redefinición/homónimo en distinto ámbito secuencial (no anidado): cada
        // declaración analizada recibe identidad propia y resultados independientes.
        let src = "funcion cont1() {\n  funcion k() { retornar 1 }\n  k()\n}\ncont1()\nfuncion cont2() {\n  funcion k() { retornar \"z\" }\n  k()\n}\ncont2()\n";
        let (ctx, e) = analizar_ctx(src);
        assert!(e.is_empty(), "errores: {:?}", e);
        let k = tipos_por_nombre(&ctx, "k");
        assert_eq!(k.len(), 2);
        let mut k = k;
        k.sort_by_key(|t| format!("{:?}", t));
        assert_eq!(k, vec![Some(TipoSem::Entero), Some(TipoSem::Texto)]);
    }

    // ===== MISIÓN 06.20: consumidor semántico C.2 (integración/consulta) =====

    fn c2_ids_unicos(ctx: &ContextoSemantico, nombre: &str) -> Vec<SimbodoId> {
        ids_por_nombre(ctx, nombre)
    }

    #[test]
    fn consumidor_c2_retorno_conocido_por_identidad() {
        let (ctx, e) = analizar_ctx("funcion f() { retornar 42 }\nf()");
        assert!(e.is_empty(), "errores: {:?}", e);
        let ids = c2_ids_unicos(&ctx, "f");
        assert_eq!(ids.len(), 1);
        // Consumo real por identidad de declaración.
        assert_eq!(
            ctx.consultar_c2_retorno(ids[0]),
            Some(Some(TipoSem::Entero))
        );
    }

    #[test]
    fn consumidor_c2_retorno_desconocido_preservado() {
        let (ctx, e) = analizar_ctx("funcion f() { sea x = 1 }");
        assert!(e.is_empty(), "errores: {:?}", e);
        let ids = c2_ids_unicos(&ctx, "f");
        assert_eq!(ids.len(), 1);
        // Presente pero conservadoramente Desconocido para C.2.
        assert_eq!(ctx.consultar_c2_retorno(ids[0]), Some(None));
    }

    #[test]
    fn consumidor_c2_ausente_para_declaracion_no_registrada() {
        let (ctx, e) = analizar_ctx("funcion f() { retornar 1 }\nf()");
        assert!(e.is_empty(), "errores: {:?}", e);
        // SimbodoId inexistente/no registrado como resultado C.2.
        assert_eq!(ctx.consultar_c2_retorno(SimbodoId(9_999_999)), None);
        // Declaración no candidata (anotada) no produce resultado C.2.
        let (ctx2, e2) = analizar_ctx("funcion g() -> entero { retornar 1 }\ng()");
        assert!(e2.is_empty(), "errores: {:?}", e2);
        assert!(c2_ids_unicos(&ctx2, "g").is_empty());
    }

    #[test]
    fn consumidor_c2_recuperacion_correcta_por_identidad_homonimas() {
        let src = "funcion externa_a() {\n  funcion calc() { retornar 1 }\n  calc()\n}\nfuncion externa_b() {\n  funcion calc() { retornar \"t\" }\n  calc()\n}\nexterna_a()\nexterna_b()\n";
        let (ctx, e) = analizar_ctx(src);
        assert!(e.is_empty(), "errores: {:?}", e);
        let ids = c2_ids_unicos(&ctx, "calc");
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
        let r0 = ctx.consultar_c2_retorno(ids[0]);
        let r1 = ctx.consultar_c2_retorno(ids[1]);
        assert!(matches!(r0, Some(Some(_)) | Some(None)));
        assert!(matches!(r1, Some(Some(_)) | Some(None)));
        let mut tipos = vec![r0.unwrap(), r1.unwrap()];
        tipos.sort_by_key(|t| format!("{:?}", t));
        assert_eq!(tipos, vec![Some(TipoSem::Entero), Some(TipoSem::Texto)]);
    }

    #[test]
    fn consumidor_c2_sin_contaminacion_entre_ambitos_y_resultados_ausentes() {
        let src = "funcion a() {\n  funcion z() { retornar 7 }\n  z()\n}\nfuncion b() {\n  funcion z() { retornar }\n  z()\n}\na()\nb()\n";
        let (ctx, e) = analizar_ctx(src);
        let _ = e; // D0: preservar comportamiento legado
        let ids = c2_ids_unicos(&ctx, "z");
        assert_eq!(ids.len(), 2);
        let mut res: Vec<_> = ids.iter().map(|id| ctx.consultar_c2_retorno(*id)).collect();
        res.sort_by_key(|r| format!("{:?}", r));
        assert_eq!(res, vec![Some(None), Some(Some(TipoSem::Entero))]);
        // Ausente sigue ausente para id no relacionado.
        assert_eq!(ctx.consultar_c2_retorno(SimbodoId(u32::MAX)), None);
    }

    #[test]
    fn consumidor_c2_preserva_c1_y_llamadas_anotadas() {
        let (ctx, e) = analizar_ctx(
            "funcion g() { retornar 1 }\nfuncion h() -> entero { retornar g() }\nh()\n",
        );
        assert!(e.is_empty(), "errores: {:?}", e);
        // C.1 preservado: propagación/fallback legado intactos.
        let sig_g = ctx.funciones.get("g").expect("firma g");
        assert!(!sig_g.retorno_anotado);
        assert_eq!(sig_g.retorno, Some(TipoSem::Entero));
        let sig_h = ctx.funciones.get("h").expect("firma h");
        assert!(sig_h.retorno_anotado);
        assert_eq!(sig_h.retorno, Some(TipoSem::Entero));
        // Consumo C.2: g candidata conocida, h anotada sin resultado C.2.
        let gid = c2_ids_unicos(&ctx, "g");
        assert_eq!(gid.len(), 1);
        assert_eq!(
            ctx.consultar_c2_retorno(gid[0]),
            Some(Some(TipoSem::Entero))
        );
        assert!(c2_ids_unicos(&ctx, "h").is_empty());
    }

    #[test]
    fn consumidor_c2_sin_interprocedimental_no_autorizada_y_d5() {
        // f llama a g sin anotación: C.2 no consume retorno inferido de cuerpo ajeno.
        let src = "funcion g() { retornar 1 }\nfuncion f(x: logico) {\n  si x { retornar g() } sino { retornar g() }\n}\nf(verdadero)\n";
        let (ctx, e) = analizar_ctx(src);
        assert!(e.is_empty(), "errores: {:?}", e);
        let fid = c2_ids_unicos(&ctx, "f");
        assert_eq!(fid.len(), 1);
        assert_eq!(ctx.consultar_c2_retorno(fid[0]), Some(None));
        // Pero C.1 legado de g se preserva.
        let sig_g = ctx.funciones.get("g").expect("firma g");
        assert_eq!(sig_g.retorno, Some(TipoSem::Entero));
        assert!(!sig_g.retorno_anotado);
    }

    #[test]
    fn consumidor_c2_excluye_lambdas_y_closures() {
        // Lambdas/closures siguen excluidas de C.2: no generan resultados C.2.
        let src = "sea f = funcion (x) { retornar x }\n";
        let (ctx, e) = analizar_ctx(src);
        assert!(e.is_empty(), "errores: {:?}", e);
        assert!(
            ctx.c2_resultados.is_empty(),
            "sin resultados C.2 para lambdas/closures"
        );
    }

    #[test]
    fn consumidor_c2_sin_cambios_en_diagnosticos_ni_ejecucion_observable() {
        // Casos con comportamiento/diagnóstico legado relevante: C.2 consulta no altera errores.
        let (ctx, e) = analizar_ctx("funcion f() { retornar }");
        assert!(e.is_empty(), "errores: {:?}", e);
        let id = c2_ids_unicos(&ctx, "f")[0];
        assert_eq!(ctx.consultar_c2_retorno(id), Some(None));

        let (ctx2, e2) =
            analizar_ctx("funcion f(x: logico) { si x { retornar 1 } sino { retornar \"a\" } }");
        // Se preserva diagnóstico/resultado legado preexistente (D0).
        let _ = e2;
        let id2 = c2_ids_unicos(&ctx2, "f")[0];
        assert_eq!(ctx2.consultar_c2_retorno(id2), Some(None));
    }
}
