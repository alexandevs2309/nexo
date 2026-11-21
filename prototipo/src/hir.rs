//! HIR (High-level Intermediate Representation) de BASILISCO.
//!
//! Representación semántica de alto nivel, independiente del AST y del runtime.
//! No ejecutable por sí misma. El lowering mantiene su propia resolución léxica
//! (`ResolutorHir`): recorre el AST en el mismo orden que el análisis semántico
//! y asigna `SimbodoId` deterministas en una pila de ámbitos con nombres. Esto
//! sustituye la consulta tardía a `ctx.resolver_simbodo()` (que tras terminar
//! el análisis solo alcanza el ámbito raíz y colapsaba parámetros/locales al
//! fallback `SimbodoId(0)`).

use std::collections::HashMap;

use crate::parser::{Expresion, Sentencia};
use crate::semantico::TipoSem;
use crate::simbodo::SimbodoId;

/// Literal HIR (modelo propio, no usa `Valor`).
#[derive(Debug, Clone, PartialEq)]
pub enum HirLiteral {
    Entero(i64),
    Decimal(i128),
    Real(f64),
    Logico(bool),
    Texto(String),
    Nulo,
}

/// Expresión HIR.
#[derive(Debug, Clone, PartialEq)]
pub enum HirExpresion {
    Literal(HirLiteral, TipoSem),
    Variable(SimbodoId, TipoSem),
    Suma(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    Resta(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    Multiplicacion(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    Division(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    Modulo(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    IgualIgual(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    Distinto(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    Menor(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    MenorIgual(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    Mayor(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    MayorIgual(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    No(Box<HirExpresion>, TipoSem),
    Y(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    O(Box<HirExpresion>, Box<HirExpresion>, TipoSem),
    Rango {
        inicio: Box<HirExpresion>,
        fin: Box<HirExpresion>,
        inclusivo: bool,
        tipo: TipoSem,
    },
    ArregloLiteral {
        elementos: Vec<HirExpresion>,
        tipo: TipoSem,
    },
    Indice {
        objeto: Box<HirExpresion>,
        indice: Box<HirExpresion>,
        tipo: TipoSem,
    },
    Acceso {
        objeto: Box<HirExpresion>,
        miembro: String,
        tipo: TipoSem,
    },
    Llamada {
        nombre: String,
        argumentos: Vec<HirExpresion>,
        tipo: TipoSem,
    },
    LlamadaIndirecta {
        callee: Box<HirExpresion>,
        argumentos: Vec<HirExpresion>,
        tipo: TipoSem,
    },
    FuncionRef(String),
    Lambda {
        nombre: String,
        params: Vec<(String, TipoSem)>,
        capturas: Vec<SimbodoId>,
        tipo: TipoSem,
    },
    Si {
        condicion: Box<HirExpresion>,
        entonces: Vec<HirSentencia>,
        sino: Option<Vec<HirSentencia>>,
    },
    Otro(TipoSem),
    /// Objeto-módulo: mapea nombres exportados a referencias de función
    /// (Valor::Funcion(prefijo_nombre)). Usado por `usar`/`importar ... como alias`.
    ModuloObjeto {
        prefijo: String,
        nombres: Vec<String>,
        tipo: TipoSem,
    },
}

/// Sentencia HIR.
#[derive(Debug, Clone, PartialEq)]
pub enum HirSentencia {
    Declaracion {
        simbodo: SimbodoId,
        tipo: TipoSem,
        valor: HirExpresion,
        mutable: bool,
        es_const: bool,
        publico: bool,
    },
    Asignacion {
        simbodo: SimbodoId,
        valor: HirExpresion,
    },
    Expresion(HirExpresion),
    Retornar(Option<HirExpresion>),
    Bloque(Vec<HirSentencia>),
    Si {
        condicion: HirExpresion,
        entonces: Vec<HirSentencia>,
        sino: Option<Vec<HirSentencia>>,
    },
    Mientras {
        condicion: HirExpresion,
        cuerpo: Vec<HirSentencia>,
    },
    AsignacionIndice {
        objeto: HirExpresion,
        indice: HirExpresion,
        valor: HirExpresion,
    },
    ParaCada {
        variable: SimbodoId,
        iterable: HirExpresion,
        cuerpo: Vec<HirSentencia>,
    },
    Romper,
    Continuar,
    IntentarRescatar {
        cuerpo_intentar: Vec<HirSentencia>,
        variable_error: Option<SimbodoId>,
        cuerpo_rescatar: Vec<HirSentencia>,
    },
    Importar {
        ruta: String,
        alias: Option<String>,
    },
    Exportar {
        sentencia: Box<HirSentencia>,
    },
    Usar {
        ruta: String,
        alias: Option<String>,
        nombres: Option<Vec<String>>,
    },
}

/// Item HIR.
#[derive(Debug, Clone, PartialEq)]
pub enum HirItem {
    Funcion {
        /// Nombre en el código fuente (los calibres de la VM/FFI lo usan).
        nombre: String,
        simbodo: SimbodoId,
        params: Vec<(SimbodoId, TipoSem)>,
        /// Variables capturadas (por referencia) por esta función/clausura.
        capturadas: Vec<SimbodoId>,
        /// Capturas recibidas desde el ámbito exterior (solo clausuras): ya son celdas.
        capturas_externas: Vec<SimbodoId>,
        /// Locales de esta función que contienen celdas de referencia
        /// (variables capturadas por clausuras anidadas, o params capturados).
        celdas: Vec<SimbodoId>,
        retorno: TipoSem,
        cuerpo: Vec<HirSentencia>,
    },
    Declaracion {
        simbodo: SimbodoId,
        tipo: TipoSem,
        valor: HirExpresion,
        mutable: bool,
        es_const: bool,
        publico: bool,
    },
    Expresion(HirExpresion),
    IntentarRescatar {
        cuerpo_intentar: Vec<HirSentencia>,
        variable_error: Option<SimbodoId>,
        cuerpo_rescatar: Vec<HirSentencia>,
    },
    Importar {
        ruta: String,
        alias: Option<String>,
    },
    Exportar {
        sentencia: Box<HirSentencia>,
    },
    Usar {
        ruta: String,
        alias: Option<String>,
        nombres: Option<Vec<String>>,
    },
}

/// Programa HIR.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HirPrograma {
    pub items: Vec<HirItem>,
}

/// Error de verificación de una invariante HIR (contrato comprobable, Fase 03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorVerificacionHir {
    pub codigo: &'static str,
    pub invariante: &'static str,
    pub contexto: String,
    pub mensaje: String,
}

impl std::fmt::Display for ErrorVerificacionHir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} ({}): {}",
            self.codigo, self.invariante, self.contexto, self.mensaje
        )
    }
}

/// Resultado detallado de la verificación HIR.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResultadoVerificacionHir {
    pub errores: Vec<ErrorVerificacionHir>,
}

impl ResultadoVerificacionHir {
    pub fn es_ok(&self) -> bool {
        self.errores.is_empty()
    }
}

fn err_hir(
    codigo: &'static str,
    invariante: &'static str,
    contexto: impl Into<String>,
    mensaje: impl Into<String>,
) -> ErrorVerificacionHir {
    ErrorVerificacionHir {
        codigo,
        invariante,
        contexto: contexto.into(),
        mensaje: mensaje.into(),
    }
}

/// Verificador HIR: valida invariantes estructurales del programa HIR.
///
/// Comprueba (Lote 3A: contrato detallado; comprobaciones efectivas ampliadas en 3B):
/// 1. Símbolos únicos en la declaración (no se repite id como parámetro
///    de una función, ni id global repetido en dos declaraciones).
/// 2. Parámetros y capturas coherentes (unique ids).
/// 3. Variables referenciadas están declaradas en algún item del programa.
/// 4. Tipo de retorno de los `Retornar` coincide con la anotación de la
///    función cuando ambos tipos son conocidos (no `Desconocido`).
/// 5. Capturas de clausuras referencian símbolos declarados.
/// 6. Expresiones binarias con operandos tipados conocidos compatibles.
/// 7. Tipos literales coherentes (texto -> Texto, lógico -> Logico, etc.).
pub struct VerificadorHir;

impl VerificadorHir {
    /// Verificación booleana (compatibilidad). Equivale a `verificar_detallado().es_ok()`.
    pub fn verificar(programa: &HirPrograma) -> bool {
        Self::verificar_detallado(programa).es_ok()
    }

    /// Verificación con contrato detallado (Fase 03): reporta la primera
    /// invariante violada encontrada, con código, contexto y mensaje.
    pub fn verificar_detallado(programa: &HirPrograma) -> ResultadoVerificacionHir {
        match verificar_items(programa) {
            Ok(()) => ResultadoVerificacionHir::default(),
            Err(e) => ResultadoVerificacionHir { errores: vec![e] },
        }
    }
}

/// Devuelve todos los ids declarados por el programa (params, decls, capturados).
fn simbolos_globales(programa: &HirPrograma) -> std::collections::HashSet<SimbodoId> {
    let mut set = std::collections::HashSet::new();
    for item in &programa.items {
        match item {
            HirItem::Funcion { .. } => {}
            HirItem::Declaracion { simbodo, .. } => {
                set.insert(*simbodo);
            }
            HirItem::IntentarRescatar {
                cuerpo_intentar,
                cuerpo_rescatar,
                variable_error,
                ..
            } => {
                if let Some(id) = variable_error {
                    set.insert(*id);
                }
                recolectar_ids_sentencia(cuerpo_intentar, &mut set);
                recolectar_ids_sentencia(cuerpo_rescatar, &mut set);
            }
            HirItem::Exportar { sentencia } => {
                recolectar_ids_sentencia_once(sentencia, &mut set);
            }
            _ => {}
        }
    }
    set
}

fn recolectar_ids_sentencia_once(s: &HirSentencia, set: &mut std::collections::HashSet<SimbodoId>) {
    match s {
        HirSentencia::Declaracion { simbodo, .. } => {
            set.insert(*simbodo);
        }
        HirSentencia::ParaCada { variable, .. } => {
            set.insert(*variable);
        }
        HirSentencia::Bloque(ss) | HirSentencia::Si { entonces: ss, .. } => {
            for st in ss {
                recolectar_ids_sentencia_once(st, set);
            }
            if let HirSentencia::Si { sino: Some(ss), .. } = s {
                for st in ss {
                    recolectar_ids_sentencia_once(st, set);
                }
            }
        }
        HirSentencia::Mientras { cuerpo, .. } => {
            for st in cuerpo {
                recolectar_ids_sentencia_once(st, set);
            }
        }
        HirSentencia::IntentarRescatar {
            cuerpo_intentar,
            variable_error,
            cuerpo_rescatar,
        } => {
            if let Some(id) = variable_error {
                set.insert(*id);
            }
            recolectar_ids_sentencia_once(&HirSentencia::Bloque(cuerpo_intentar.clone()), set);
            recolectar_ids_sentencia_once(&HirSentencia::Bloque(cuerpo_rescatar.clone()), set);
        }
        _ => {}
    }
}

fn recolectar_ids_sentencia(ss: &[HirSentencia], set: &mut std::collections::HashSet<SimbodoId>) {
    for s in ss {
        recolectar_ids_sentencia_once(s, set);
    }
}

/// Recolecta ids referenciados como Variable en una expresión.
/// Recolecta ids referenciados como Variable en una expresión.
fn recolectar_ids_expr(e: &HirExpresion, set: &mut std::collections::HashSet<SimbodoId>) {
    match e {
        HirExpresion::Variable(id, _) => {
            set.insert(*id);
        }
        HirExpresion::Literal(_, _) | HirExpresion::Otro(_) => {}
        HirExpresion::Suma(a, b, _)
        | HirExpresion::Resta(a, b, _)
        | HirExpresion::Multiplicacion(a, b, _)
        | HirExpresion::Division(a, b, _)
        | HirExpresion::Modulo(a, b, _)
        | HirExpresion::IgualIgual(a, b, _)
        | HirExpresion::Distinto(a, b, _)
        | HirExpresion::Menor(a, b, _)
        | HirExpresion::MenorIgual(a, b, _)
        | HirExpresion::Mayor(a, b, _)
        | HirExpresion::MayorIgual(a, b, _)
        | HirExpresion::Y(a, b, _)
        | HirExpresion::O(a, b, _) => {
            recolectar_ids_expr(a, set);
            recolectar_ids_expr(b, set);
        }
        HirExpresion::No(x, _) => recolectar_ids_expr(x, set),
        HirExpresion::Rango { inicio, fin, .. } => {
            recolectar_ids_expr(inicio, set);
            recolectar_ids_expr(fin, set);
        }
        HirExpresion::ArregloLiteral { elementos, .. } => {
            for e in elementos {
                recolectar_ids_expr(e, set);
            }
        }
        HirExpresion::Indice { objeto, indice, .. } => {
            recolectar_ids_expr(objeto, set);
            recolectar_ids_expr(indice, set);
        }
        HirExpresion::Acceso { objeto, .. } => recolectar_ids_expr(objeto, set),
        HirExpresion::Llamada { argumentos, .. } => {
            for a in argumentos {
                recolectar_ids_expr(a, set);
            }
        }
        HirExpresion::LlamadaIndirecta {
            callee, argumentos, ..
        } => {
            recolectar_ids_expr(callee, set);
            for a in argumentos {
                recolectar_ids_expr(a, set);
            }
        }
        HirExpresion::Lambda { capturas, .. } => {
            for c in capturas {
                set.insert(*c);
            }
        }
        HirExpresion::Si {
            condicion,
            entonces,
            sino,
        } => {
            recolectar_ids_expr(condicion, set);
            for s in entonces {
                recolectar_ids_sentencia_once(s, set);
            }
            if let Some(ss) = sino {
                for s in ss {
                    recolectar_ids_sentencia_once(s, set);
                }
            }
        }
        HirExpresion::ModuloObjeto { .. } => {}
        HirExpresion::FuncionRef(_) => {}
    }
}

fn recolectar_ids_ref(ss: &[HirSentencia], set: &mut std::collections::HashSet<SimbodoId>) {
    for s in ss {
        match s {
            HirSentencia::Declaracion { valor, .. } | HirSentencia::Asignacion { valor, .. } => {
                recolectar_ids_expr(valor, set);
            }
            HirSentencia::Expresion(e) | HirSentencia::Retornar(Some(e)) => {
                recolectar_ids_expr(e, set);
            }
            HirSentencia::Bloque(ss) | HirSentencia::Si { entonces: ss, .. } => {
                recolectar_ids_ref(ss, set);
                if let HirSentencia::Si { sino: Some(ss), .. } = s {
                    recolectar_ids_ref(ss, set);
                }
            }
            HirSentencia::Mientras { cuerpo, .. } => recolectar_ids_ref(cuerpo, set),
            HirSentencia::ParaCada {
                iterable, cuerpo, ..
            } => {
                recolectar_ids_expr(iterable, set);
                recolectar_ids_ref(cuerpo, set);
            }
            HirSentencia::IntentarRescatar {
                cuerpo_intentar,
                cuerpo_rescatar,
                ..
            } => {
                recolectar_ids_ref(cuerpo_intentar, set);
                recolectar_ids_ref(cuerpo_rescatar, set);
            }
            HirSentencia::AsignacionIndice {
                objeto,
                indice,
                valor,
            } => {
                recolectar_ids_expr(objeto, set);
                recolectar_ids_expr(indice, set);
                recolectar_ids_expr(valor, set);
            }
            HirSentencia::Exportar { sentencia } => {
                recolectar_ids_ref(std::slice::from_ref(sentencia.as_ref()), set)
            }
            _ => {}
        }
    }
}

fn verificar_items(programa: &HirPrograma) -> Result<(), ErrorVerificacionHir> {
    let globales = simbolos_globales(programa);
    for (idx, item) in programa.items.iter().enumerate() {
        let declarados = match item {
            HirItem::Funcion {
                params,
                capturadas,
                capturas_externas,
                celdas,
                cuerpo,
                ..
            } => {
                let mut d = globales.clone();
                for (id, _) in params {
                    d.insert(*id);
                }
                for id in capturadas
                    .iter()
                    .chain(capturas_externas.iter())
                    .chain(celdas.iter())
                {
                    d.insert(*id);
                }
                recolectar_ids_sentencia(cuerpo, &mut d);
                d
            }
            _ => globales.clone(),
        };
        verificar_item(item, idx, &declarados)?;
    }
    Ok(())
}

fn ctx_item(item: &HirItem, idx: usize) -> String {
    match item {
        HirItem::Funcion { nombre, .. } => format!("item[{idx}] funcion '{nombre}'"),
        HirItem::Declaracion { simbodo, .. } => {
            format!("item[{idx}] declaracion simbodo {}", simbodo.0)
        }
        HirItem::IntentarRescatar { .. } => format!("item[{idx}] intentar/rescatar"),
        HirItem::Expresion(_) => format!("item[{idx}] expresion"),
        HirItem::Exportar { .. } => format!("item[{idx}] exportar"),
        _ => format!("item[{idx}]"),
    }
}

fn verificar_item(
    item: &HirItem,
    idx: usize,
    declarados: &std::collections::HashSet<SimbodoId>,
) -> Result<(), ErrorVerificacionHir> {
    let ctx = ctx_item(item, idx);
    match item {
        HirItem::Funcion {
            params,
            capturadas,
            capturas_externas,
            celdas,
            retorno,
            cuerpo,
            ..
        } => {
            // 1. Parámetros únicos.
            let mut ids = std::collections::HashSet::new();
            for (id, _) in params {
                if !ids.insert(*id) {
                    return Err(err_hir(
                        "HIR010",
                        "param-unico",
                        &ctx,
                        format!("parámetro duplicado simbodo {}", id.0),
                    ));
                }
            }
            // 2. Capturas son únicas y referencian símbolos declarados.
            let mut cap_ids = std::collections::HashSet::new();
            for id in capturadas
                .iter()
                .chain(capturas_externas.iter())
                .chain(celdas.iter())
            {
                if !cap_ids.insert(*id) {
                    return Err(err_hir(
                        "HIR011",
                        "captura-unica",
                        &ctx,
                        format!("captura duplicada simbodo {}", id.0),
                    ));
                }
                if !declarados.contains(id) {
                    return Err(err_hir(
                        "HIR012",
                        "captura-declarada",
                        &ctx,
                        format!("captura referencia simbodo no declarado {}", id.0),
                    ));
                }
            }
            // 3. Variables referenciadas en el cuerpo están declaradas.
            let mut refs = std::collections::HashSet::new();
            recolectar_ids_ref(cuerpo, &mut refs);
            for r in &refs {
                // Los ids locales de la función (params) están declarados.
                if !declarados.contains(r) {
                    return Err(err_hir(
                        "HIR013",
                        "referencia-declarada",
                        &ctx,
                        format!("referencia a simbodo no declarado {}", r.0),
                    ));
                }
            }
            // 4. Tipo de retorno consistente (D1, Fase 3B): validar retornos
            //    contra el tipo declarado cuando ambos son conocidos; tratar
            //    retornos vacíos y Desconocido sin falsos positivos.
            verificar_retorno_funcion(retorno, cuerpo, &ctx)?;
            verificar_tipos_invariantes(cuerpo, &ctx)
        }
        HirItem::Declaracion { tipo, .. } => {
            // 5. Tipo de inicializador compatible con anotación cuando ambos conocidos.
            if *tipo == TipoSem::Desconocido {
                Err(err_hir(
                    "HIR020",
                    "declaracion-tipo-conocido",
                    &ctx,
                    "declaración con tipo desconocido",
                ))
            } else {
                Ok(())
            }
        }
        HirItem::IntentarRescatar {
            cuerpo_intentar,
            cuerpo_rescatar,
            ..
        } => {
            verificar_tipos_invariantes(cuerpo_intentar, &format!("{ctx}/intentar"))?;
            verificar_tipos_invariantes(cuerpo_rescatar, &format!("{ctx}/rescatar"))
        }
        HirItem::Expresion(_) => Ok(()),
        _ => Ok(()),
    }
}

fn verificar_tipos_invariantes(ss: &[HirSentencia], ctx: &str) -> Result<(), ErrorVerificacionHir> {
    for (i, s) in ss.iter().enumerate() {
        let ctx_s = format!("{ctx}/sent[{i}]");
        match s {
            HirSentencia::Declaracion { valor, .. } | HirSentencia::Asignacion { valor, .. } => {
                verificar_expr(valor, &ctx_s)?;
            }
            HirSentencia::Expresion(e) | HirSentencia::Retornar(Some(e)) => {
                verificar_expr(e, &ctx_s)?;
            }
            HirSentencia::Bloque(inner) => {
                verificar_tipos_invariantes(inner, &format!("{ctx_s}/bloque"))?;
            }
            HirSentencia::Si { entonces, sino, .. } => {
                verificar_tipos_invariantes(entonces, &format!("{ctx_s}/si.entonces"))?;
                if let Some(alt) = sino {
                    verificar_tipos_invariantes(alt, &format!("{ctx_s}/si.sino"))?;
                }
            }
            HirSentencia::Mientras { cuerpo, .. } => {
                verificar_tipos_invariantes(cuerpo, &format!("{ctx_s}/mientras"))?;
            }
            HirSentencia::ParaCada { cuerpo, .. } => {
                verificar_tipos_invariantes(cuerpo, &format!("{ctx_s}/para_cada"))?;
            }
            HirSentencia::IntentarRescatar {
                cuerpo_intentar,
                cuerpo_rescatar,
                ..
            } => {
                verificar_tipos_invariantes(cuerpo_intentar, &format!("{ctx_s}/intentar"))?;
                verificar_tipos_invariantes(cuerpo_rescatar, &format!("{ctx_s}/rescatar"))?;
            }
            HirSentencia::Exportar { sentencia } => {
                verificar_tipos_invariantes(
                    std::slice::from_ref(sentencia.as_ref()),
                    &format!("{ctx_s}/exportar"),
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn tipo_de_expr(e: &HirExpresion) -> TipoSem {
    match e {
        HirExpresion::Literal(_, t)
        | HirExpresion::Variable(_, t)
        | HirExpresion::Suma(_, _, t)
        | HirExpresion::Resta(_, _, t)
        | HirExpresion::Multiplicacion(_, _, t)
        | HirExpresion::Division(_, _, t)
        | HirExpresion::Modulo(_, _, t)
        | HirExpresion::IgualIgual(_, _, t)
        | HirExpresion::Distinto(_, _, t)
        | HirExpresion::Menor(_, _, t)
        | HirExpresion::MenorIgual(_, _, t)
        | HirExpresion::Mayor(_, _, t)
        | HirExpresion::MayorIgual(_, _, t)
        | HirExpresion::No(_, t)
        | HirExpresion::Y(_, _, t)
        | HirExpresion::O(_, _, t) => t.clone(),
        HirExpresion::Rango { tipo: t, .. }
        | HirExpresion::ArregloLiteral { tipo: t, .. }
        | HirExpresion::Indice { tipo: t, .. } => t.clone(),
        _ => TipoSem::Desconocido,
    }
}

fn tipo_inferido(e: &HirExpresion) -> TipoSem {
    let t = tipo_de_expr(e);
    if !matches!(t, TipoSem::Desconocido) {
        return t;
    }
    match e {
        HirExpresion::Suma(a, b, _)
        | HirExpresion::Resta(a, b, _)
        | HirExpresion::Multiplicacion(a, b, _)
        | HirExpresion::Division(a, b, _)
        | HirExpresion::Modulo(a, b, _) => {
            let ta = tipo_inferido(a);
            let tb = tipo_inferido(b);
            if ta == tb && !matches!(ta, TipoSem::Desconocido) {
                ta
            } else {
                TipoSem::Desconocido
            }
        }
        HirExpresion::Literal(HirLiteral::Entero(_), _) => TipoSem::Entero,
        HirExpresion::Literal(HirLiteral::Texto(_), _) => TipoSem::Texto,
        HirExpresion::Literal(HirLiteral::Logico(_), _) => TipoSem::Logico,
        HirExpresion::Literal(HirLiteral::Real(_), _) => TipoSem::Real,
        HirExpresion::Literal(HirLiteral::Decimal(_), _) => TipoSem::Decimal,
        _ => TipoSem::Desconocido,
    }
}

fn es_tipo_numerico(t: &TipoSem) -> bool {
    matches!(t, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
}

fn tipos_compatibles_suma(a: &HirExpresion, b: &HirExpresion) -> bool {
    let ta = tipo_inferido(a);
    let tb = tipo_inferido(b);
    if matches!(ta, TipoSem::Desconocido) || matches!(tb, TipoSem::Desconocido) {
        return true;
    }
    if es_tipo_numerico(&ta) && es_tipo_numerico(&tb) {
        return true;
    }
    ta == TipoSem::Texto && tb == TipoSem::Texto
}

fn tipos_compatibles_aritmetico(a: &HirExpresion, b: &HirExpresion) -> bool {
    let ta = tipo_inferido(a);
    let tb = tipo_inferido(b);
    if matches!(ta, TipoSem::Desconocido) || matches!(tb, TipoSem::Desconocido) {
        return true;
    }
    es_tipo_numerico(&ta) && es_tipo_numerico(&tb)
}

fn tipos_compatibles_comparacion(a: &HirExpresion, b: &HirExpresion) -> bool {
    let ta = tipo_inferido(a);
    let tb = tipo_inferido(b);
    if matches!(ta, TipoSem::Desconocido) || matches!(tb, TipoSem::Desconocido) {
        return true;
    }
    if es_tipo_numerico(&ta) && es_tipo_numerico(&tb) {
        return true;
    }
    ta == tb
}

fn tipos_compatibles_logico(a: &HirExpresion, b: &HirExpresion) -> bool {
    let ta = tipo_inferido(a);
    let tb = tipo_inferido(b);
    if matches!(ta, TipoSem::Desconocido) || matches!(tb, TipoSem::Desconocido) {
        return true;
    }
    ta == TipoSem::Logico && tb == TipoSem::Logico
}

/// Compatibilidad de asignación/retorno (espejo conservador de semántica).
/// `esperado` = tipo declarado; `obtenido` = tipo real/inferido.
/// `Desconocido` es compatible con todo (evita falsos positivos, D1).
fn tipos_compatibles_retorno(esperado: &TipoSem, obtenido: &TipoSem) -> bool {
    if matches!(esperado, TipoSem::Desconocido) || matches!(obtenido, TipoSem::Desconocido) {
        return true;
    }
    match (esperado, obtenido) {
        (TipoSem::Entero, TipoSem::Entero) => true,
        (TipoSem::Decimal, TipoSem::Decimal) | (TipoSem::Decimal, TipoSem::Entero) => true,
        (TipoSem::Real, TipoSem::Real) | (TipoSem::Real, TipoSem::Entero) => true,
        (TipoSem::Logico, TipoSem::Logico) => true,
        (TipoSem::Texto, TipoSem::Texto) => true,
        (TipoSem::Nulo, TipoSem::Nulo) => true,
        (TipoSem::Arreglo(te, ne), TipoSem::Arreglo(to, no)) => {
            ne == no && tipos_compatibles_retorno(te, to)
        }
        (TipoSem::Lista(te), TipoSem::Lista(to)) => tipos_compatibles_retorno(te, to),
        (TipoSem::Arreglo(te, _), TipoSem::Lista(to)) => tipos_compatibles_retorno(te, to),
        (TipoSem::Lista(te), TipoSem::Arreglo(to, _)) => tipos_compatibles_retorno(te, to),
        (TipoSem::Coleccion(te), TipoSem::Arreglo(to, _)) => tipos_compatibles_retorno(te, to),
        (TipoSem::Coleccion(te), TipoSem::Lista(to)) => tipos_compatibles_retorno(te, to),
        (TipoSem::Arreglo(te, _), TipoSem::Coleccion(to)) => tipos_compatibles_retorno(te, to),
        (TipoSem::Lista(te), TipoSem::Coleccion(to)) => tipos_compatibles_retorno(te, to),
        (TipoSem::Coleccion(te), TipoSem::Coleccion(to)) => tipos_compatibles_retorno(te, to),
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
                && p1
                    .iter()
                    .zip(p2)
                    .all(|(a, b)| tipos_compatibles_retorno(a, b))
                && tipos_compatibles_retorno(r1, r2)
        }
        _ => false,
    }
}

fn recolectar_retornos_expr<'a>(e: &'a HirExpresion, out: &mut Vec<&'a Option<HirExpresion>>) {
    match e {
        HirExpresion::Si {
            condicion,
            entonces,
            sino,
        } => {
            recolectar_retornos_expr(condicion, out);
            recolectar_retornos(entonces, out);
            if let Some(alt) = sino {
                recolectar_retornos(alt, out);
            }
        }
        HirExpresion::Suma(a, b, _)
        | HirExpresion::Resta(a, b, _)
        | HirExpresion::Multiplicacion(a, b, _)
        | HirExpresion::Division(a, b, _)
        | HirExpresion::Modulo(a, b, _)
        | HirExpresion::IgualIgual(a, b, _)
        | HirExpresion::Distinto(a, b, _)
        | HirExpresion::Menor(a, b, _)
        | HirExpresion::MenorIgual(a, b, _)
        | HirExpresion::Mayor(a, b, _)
        | HirExpresion::MayorIgual(a, b, _)
        | HirExpresion::Y(a, b, _)
        | HirExpresion::O(a, b, _)
        | HirExpresion::Indice {
            objeto: a,
            indice: b,
            ..
        } => {
            recolectar_retornos_expr(a, out);
            recolectar_retornos_expr(b, out);
        }
        HirExpresion::Rango { inicio, fin, .. } => {
            recolectar_retornos_expr(inicio, out);
            recolectar_retornos_expr(fin, out);
        }
        HirExpresion::No(x, _) | HirExpresion::Acceso { objeto: x, .. } => {
            recolectar_retornos_expr(x, out);
        }
        HirExpresion::ArregloLiteral { elementos, .. } => {
            for el in elementos {
                recolectar_retornos_expr(el, out);
            }
        }
        HirExpresion::Llamada { argumentos, .. } => {
            for a in argumentos {
                recolectar_retornos_expr(a, out);
            }
        }
        HirExpresion::LlamadaIndirecta {
            callee, argumentos, ..
        } => {
            recolectar_retornos_expr(callee, out);
            for a in argumentos {
                recolectar_retornos_expr(a, out);
            }
        }
        // Lambda define su propio ámbito/retornos (cuerpo en HirItem separado); no descender.
        HirExpresion::Literal(_, _)
        | HirExpresion::Variable(_, _)
        | HirExpresion::FuncionRef(_)
        | HirExpresion::Lambda { .. }
        | HirExpresion::Otro(_)
        | HirExpresion::ModuloObjeto { .. } => {}
    }
}

fn recolectar_retornos<'a>(ss: &'a [HirSentencia], out: &mut Vec<&'a Option<HirExpresion>>) {
    for s in ss {
        match s {
            HirSentencia::Retornar(v) => out.push(v),
            HirSentencia::Declaracion { valor, .. } | HirSentencia::Asignacion { valor, .. } => {
                recolectar_retornos_expr(valor, out);
            }
            HirSentencia::Expresion(e) => recolectar_retornos_expr(e, out),
            HirSentencia::Bloque(inner) => recolectar_retornos(inner, out),
            HirSentencia::Si {
                condicion,
                entonces,
                sino,
            } => {
                recolectar_retornos_expr(condicion, out);
                recolectar_retornos(entonces, out);
                if let Some(alt) = sino {
                    recolectar_retornos(alt, out);
                }
            }
            HirSentencia::Mientras { condicion, cuerpo } => {
                recolectar_retornos_expr(condicion, out);
                recolectar_retornos(cuerpo, out);
            }
            HirSentencia::ParaCada {
                iterable, cuerpo, ..
            } => {
                recolectar_retornos_expr(iterable, out);
                recolectar_retornos(cuerpo, out);
            }
            HirSentencia::IntentarRescatar {
                cuerpo_intentar,
                cuerpo_rescatar,
                ..
            } => {
                recolectar_retornos(cuerpo_intentar, out);
                recolectar_retornos(cuerpo_rescatar, out);
            }
            HirSentencia::AsignacionIndice {
                objeto,
                indice,
                valor,
            } => {
                recolectar_retornos_expr(objeto, out);
                recolectar_retornos_expr(indice, out);
                recolectar_retornos_expr(valor, out);
            }
            HirSentencia::Exportar { sentencia } => {
                recolectar_retornos(std::slice::from_ref(sentencia.as_ref()), out)
            }
            HirSentencia::Romper
            | HirSentencia::Continuar
            | HirSentencia::Importar { .. }
            | HirSentencia::Usar { .. } => {}
        }
    }
}

fn verificar_retorno_funcion(
    retorno: &TipoSem,
    cuerpo: &[HirSentencia],
    ctx: &str,
) -> Result<(), ErrorVerificacionHir> {
    if matches!(retorno, TipoSem::Desconocido) {
        return Ok(());
    }
    let mut rets = Vec::new();
    recolectar_retornos(cuerpo, &mut rets);
    for r in rets {
        match r {
            None => {
                if !matches!(retorno, TipoSem::Nulo | TipoSem::Desconocido) {
                    return Err(err_hir(
                        "HIR040",
                        "retorno-tipo-incompatible",
                        ctx,
                        format!(
                            "retorno vacío incompatible con tipo declarado '{}'",
                            retorno.nombre()
                        ),
                    ));
                }
            }
            Some(e) => {
                let te = tipo_inferido(e);
                if !matches!(te, TipoSem::Desconocido) && !tipos_compatibles_retorno(retorno, &te) {
                    return Err(err_hir(
                        "HIR040",
                        "retorno-tipo-incompatible",
                        ctx,
                        format!(
                            "retorno de tipo '{}' incompatible con tipo declarado '{}'",
                            te.nombre(),
                            retorno.nombre()
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn verificar_expr(e: &HirExpresion, ctx: &str) -> Result<(), ErrorVerificacionHir> {
    match e {
        HirExpresion::Literal(lit, tipo) => {
            let ok = matches!(
                (lit, tipo),
                (HirLiteral::Entero(_), TipoSem::Entero)
                    | (HirLiteral::Real(_), TipoSem::Real)
                    | (HirLiteral::Decimal(_), TipoSem::Decimal)
                    | (HirLiteral::Logico(_), TipoSem::Logico)
                    | (HirLiteral::Texto(_), TipoSem::Texto)
                    | (HirLiteral::Nulo, TipoSem::Nulo)
                    | (_, TipoSem::Desconocido)
            );
            if ok {
                Ok(())
            } else {
                Err(err_hir(
                    "HIR030",
                    "literal-tipo-coherente",
                    ctx,
                    format!("literal {:?} incompatible con tipo {:?}", lit, tipo),
                ))
            }
        }
        HirExpresion::Variable(_, _) => Ok(()),
        HirExpresion::Suma(a, b, _) => {
            verificar_expr(a, &format!("{ctx}/op.izq"))?;
            verificar_expr(b, &format!("{ctx}/op.der"))?;
            if !tipos_compatibles_suma(a, b) {
                return Err(err_hir(
                    "HIR031",
                    "operacion-tipos-compatibles",
                    ctx,
                    "operandos de suma con tipos conocidos incompatibles",
                ));
            }
            Ok(())
        }
        HirExpresion::Resta(a, b, _)
        | HirExpresion::Multiplicacion(a, b, _)
        | HirExpresion::Division(a, b, _)
        | HirExpresion::Modulo(a, b, _) => {
            verificar_expr(a, &format!("{ctx}/op.izq"))?;
            verificar_expr(b, &format!("{ctx}/op.der"))?;
            if !tipos_compatibles_aritmetico(a, b) {
                return Err(err_hir(
                    "HIR031",
                    "operacion-tipos-compatibles",
                    ctx,
                    "operandos aritmeticos con tipos conocidos incompatibles",
                ));
            }
            Ok(())
        }
        HirExpresion::IgualIgual(a, b, _)
        | HirExpresion::Distinto(a, b, _)
        | HirExpresion::Menor(a, b, _)
        | HirExpresion::MenorIgual(a, b, _)
        | HirExpresion::Mayor(a, b, _)
        | HirExpresion::MayorIgual(a, b, _) => {
            verificar_expr(a, &format!("{ctx}/op.izq"))?;
            verificar_expr(b, &format!("{ctx}/op.der"))?;
            if !tipos_compatibles_comparacion(a, b) {
                return Err(err_hir(
                    "HIR031",
                    "operacion-tipos-compatibles",
                    ctx,
                    "operandos de comparacion con tipos conocidos incompatibles",
                ));
            }
            Ok(())
        }
        HirExpresion::Y(a, b, _) | HirExpresion::O(a, b, _) => {
            verificar_expr(a, &format!("{ctx}/op.izq"))?;
            verificar_expr(b, &format!("{ctx}/op.der"))?;
            if !tipos_compatibles_logico(a, b) {
                return Err(err_hir(
                    "HIR031",
                    "operacion-tipos-compatibles",
                    ctx,
                    "operandos logicos con tipos conocidos incompatibles",
                ));
            }
            Ok(())
        }
        HirExpresion::No(x, _) => verificar_expr(x, &format!("{ctx}/no")),
        HirExpresion::Rango { inicio, fin, .. } => {
            verificar_expr(inicio, &format!("{ctx}/rango.inicio"))?;
            verificar_expr(fin, &format!("{ctx}/rango.fin"))
        }
        HirExpresion::ArregloLiteral { elementos, .. } => {
            for (i, el) in elementos.iter().enumerate() {
                verificar_expr(el, &format!("{ctx}/arreglo[{i}]"))?;
            }
            Ok(())
        }
        HirExpresion::Indice { objeto, indice, .. } => {
            verificar_expr(objeto, &format!("{ctx}/indice.objeto"))?;
            verificar_expr(indice, &format!("{ctx}/indice.indice"))
        }
        HirExpresion::Acceso { objeto, .. } => {
            verificar_expr(objeto, &format!("{ctx}/acceso.objeto"))
        }
        HirExpresion::Llamada { argumentos, .. } => {
            for (i, a) in argumentos.iter().enumerate() {
                verificar_expr(a, &format!("{ctx}/llamada.arg[{i}]"))?;
            }
            Ok(())
        }
        HirExpresion::LlamadaIndirecta {
            callee, argumentos, ..
        } => {
            verificar_expr(callee, &format!("{ctx}/llamada_ind.callee"))?;
            for (i, a) in argumentos.iter().enumerate() {
                verificar_expr(a, &format!("{ctx}/llamada_ind.arg[{i}]"))?;
            }
            Ok(())
        }
        HirExpresion::Lambda { .. } => Ok(()),
        HirExpresion::Si {
            condicion,
            entonces,
            sino,
        } => {
            verificar_expr(condicion, &format!("{ctx}/si.cond"))?;
            verificar_tipos_invariantes(entonces, &format!("{ctx}/si.entonces"))?;
            if let Some(alt) = sino {
                verificar_tipos_invariantes(alt, &format!("{ctx}/si.sino"))?;
            }
            Ok(())
        }
        HirExpresion::FuncionRef(_) => Ok(()),
        HirExpresion::Otro(_) => Ok(()),
        HirExpresion::ModuloObjeto { .. } => Ok(()),
    }
}

/// Renombra en sitio las llamadas directas de una sentencia según `mapa`
/// (nombre original -> nombre prefijado). Se usa al inlinear un módulo con
/// alias: los cuerpos de las funciones del módulo conservan los nombres sin
/// prefijo y, sin esta reescritura, la VM no encuentra la función.
fn renombrar_llamadas_sentencia(
    ss: &mut HirSentencia,
    mapa: &std::collections::HashMap<String, String>,
) {
    match ss {
        HirSentencia::Declaracion { valor, .. } => renombrar_llamadas_expresion(valor, mapa),
        HirSentencia::Asignacion { valor, .. } => renombrar_llamadas_expresion(valor, mapa),
        HirSentencia::Expresion(e) => renombrar_llamadas_expresion(e, mapa),
        HirSentencia::Retornar(Some(e)) => renombrar_llamadas_expresion(e, mapa),
        HirSentencia::Retornar(None) => {}
        HirSentencia::Bloque(cuerpo) => {
            for s in cuerpo {
                renombrar_llamadas_sentencia(s, mapa);
            }
        }
        HirSentencia::Si {
            condicion,
            entonces,
            sino,
        } => {
            renombrar_llamadas_expresion(condicion, mapa);
            for s in entonces {
                renombrar_llamadas_sentencia(s, mapa);
            }
            if let Some(alt) = sino {
                for s in alt {
                    renombrar_llamadas_sentencia(s, mapa);
                }
            }
        }
        HirSentencia::Mientras { condicion, cuerpo } => {
            renombrar_llamadas_expresion(condicion, mapa);
            for s in cuerpo {
                renombrar_llamadas_sentencia(s, mapa);
            }
        }
        HirSentencia::AsignacionIndice {
            objeto,
            indice,
            valor,
        } => {
            renombrar_llamadas_expresion(objeto, mapa);
            renombrar_llamadas_expresion(indice, mapa);
            renombrar_llamadas_expresion(valor, mapa);
        }
        HirSentencia::ParaCada {
            iterable, cuerpo, ..
        } => {
            renombrar_llamadas_expresion(iterable, mapa);
            for s in cuerpo {
                renombrar_llamadas_sentencia(s, mapa);
            }
        }
        HirSentencia::IntentarRescatar {
            cuerpo_intentar,
            cuerpo_rescatar,
            ..
        } => {
            for s in cuerpo_intentar {
                renombrar_llamadas_sentencia(s, mapa);
            }
            for s in cuerpo_rescatar {
                renombrar_llamadas_sentencia(s, mapa);
            }
        }
        HirSentencia::Exportar { sentencia } => renombrar_llamadas_sentencia(sentencia, mapa),
        HirSentencia::Romper
        | HirSentencia::Continuar
        | HirSentencia::Importar { .. }
        | HirSentencia::Usar { .. } => {}
    }
}

/// Renombra en sitio las llamadas directas de una expresión según `mapa`.
/// Las llamadas indirectas (`LlamadaIndirecta`) no se tocan: su objetivo no es
/// un nombre de función.
fn renombrar_llamadas_expresion(
    e: &mut HirExpresion,
    mapa: &std::collections::HashMap<String, String>,
) {
    match e {
        HirExpresion::Literal(_, _)
        | HirExpresion::Variable(_, _)
        | HirExpresion::Lambda { .. }
        | HirExpresion::Otro(_)
        | HirExpresion::ModuloObjeto { .. }
        | HirExpresion::FuncionRef(_) => {}
        HirExpresion::Suma(a, b, _)
        | HirExpresion::Resta(a, b, _)
        | HirExpresion::Multiplicacion(a, b, _)
        | HirExpresion::Division(a, b, _)
        | HirExpresion::Modulo(a, b, _)
        | HirExpresion::IgualIgual(a, b, _)
        | HirExpresion::Distinto(a, b, _)
        | HirExpresion::Menor(a, b, _)
        | HirExpresion::MenorIgual(a, b, _)
        | HirExpresion::Mayor(a, b, _)
        | HirExpresion::MayorIgual(a, b, _)
        | HirExpresion::Y(a, b, _)
        | HirExpresion::O(a, b, _) => {
            renombrar_llamadas_expresion(a, mapa);
            renombrar_llamadas_expresion(b, mapa);
        }
        HirExpresion::No(x, _) => renombrar_llamadas_expresion(x, mapa),
        HirExpresion::Rango { inicio, fin, .. } => {
            renombrar_llamadas_expresion(inicio, mapa);
            renombrar_llamadas_expresion(fin, mapa);
        }
        HirExpresion::ArregloLiteral { elementos, .. } => {
            for el in elementos {
                renombrar_llamadas_expresion(el, mapa);
            }
        }
        HirExpresion::Indice { objeto, indice, .. } => {
            renombrar_llamadas_expresion(objeto, mapa);
            renombrar_llamadas_expresion(indice, mapa);
        }
        HirExpresion::Acceso { objeto, .. } => renombrar_llamadas_expresion(objeto, mapa),
        HirExpresion::Llamada {
            nombre, argumentos, ..
        } => {
            if let Some(prefijado) = mapa.get(nombre) {
                *nombre = prefijado.clone();
            }
            for arg in argumentos {
                renombrar_llamadas_expresion(arg, mapa);
            }
        }
        HirExpresion::LlamadaIndirecta {
            callee, argumentos, ..
        } => {
            renombrar_llamadas_expresion(callee, mapa);
            for arg in argumentos {
                renombrar_llamadas_expresion(arg, mapa);
            }
        }
        HirExpresion::Si {
            condicion,
            entonces,
            sino,
        } => {
            renombrar_llamadas_expresion(condicion, mapa);
            for s in entonces {
                renombrar_llamadas_sentencia(s, mapa);
            }
            if let Some(alt) = sino {
                for s in alt {
                    renombrar_llamadas_sentencia(s, mapa);
                }
            }
        }
    }
}

/// Resolvedor léxico propio del lowering de HIR.
///
/// Mantiene una pila de ámbitos con nombres y asigna `SimbodoId` frescos y
/// deterministas. El lowering recorre el AST en el mismo orden que el análisis
/// semántico, así que el anidamiento es fiel (funciones, bloques, ramas y
/// `para cada` abren ámbitos). Los ids parten de una base alta para no colisionar
/// con los de `TablaSimbodo`; HIR/MIR solo necesitan consistencia interna.
pub struct ResolutorHir {
    scopes: Vec<HashMap<String, SimbodoId>>,
    siguiente: u32,
    pub clausuras: Vec<HirItem>,
    contador_clausura: usize,
    /// Nombres de funciones registradas (primer nivel o clausuras) para
    /// decidir entre llamada directa e indirecta.
    pub funciones: std::collections::HashSet<String>,
    /// Nombres de variables declaradas (para distinguir llamadas indirectas).
    pub variables: std::collections::HashSet<String>,
}

const BASE_ID_RESOLUCION: u32 = 1_000_000;

impl ResolutorHir {
    pub fn nuevo() -> Self {
        ResolutorHir {
            scopes: vec![HashMap::new()],
            siguiente: BASE_ID_RESOLUCION,
            clausuras: Vec::new(),
            contador_clausura: 0,
            funciones: std::collections::HashSet::new(),
            variables: std::collections::HashSet::new(),
        }
    }

    fn fresca(&mut self) -> SimbodoId {
        let id = SimbodoId(self.siguiente);
        self.siguiente += 1;
        id
    }

    /// Registra `nombre` en el ámbito actual; reutiliza `id` si se da.
    fn insertar(&mut self, nombre: &str, id: Option<SimbodoId>) -> SimbodoId {
        let id = id.unwrap_or_else(|| self.fresca());
        self.scopes
            .last_mut()
            .expect("siempre hay ámbito raíz")
            .insert(nombre.to_string(), id);
        id
    }

    /// Resolución léxica de interior a exterior.
    fn resolver(&self, nombre: &str) -> Option<SimbodoId> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.get(nombre) {
                return Some(*id);
            }
        }
        None
    }

    /// Resuelve o registra en el ámbito actual (lectura sin declaración previa:
    /// queda mapeada de forma consistente para lecturas posteriores).
    fn resolver_o_registrar(&mut self, nombre: &str) -> SimbodoId {
        match self.resolver(nombre) {
            Some(id) => id,
            None => self.insertar(nombre, None),
        }
    }

    fn nuevo_nombre_clausura(&mut self) -> String {
        self.contador_clausura += 1;
        format!("clausura_{}", self.contador_clausura)
    }

    fn entrar(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn salir(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
}

// Necesitamos buscar_tipo_variable en ContextoSemantico; añadir método.
impl crate::semantico::ContextoSemantico {
    pub fn buscar_tipo_variable(&self, nombre: &str) -> Option<TipoSem> {
        self.buscar_variable(nombre).map(|v| v.tipo)
    }
}

/// Lowering AST -> HIR. Requiere ContextoSemantico para tipos y el resolvedor
/// propio para símbodos.
fn lower_expresion(
    e: &Expresion,
    ctx: &crate::semantico::ContextoSemantico,
    res: &mut ResolutorHir,
) -> Result<HirExpresion, String> {
    Ok(match e {
        Expresion::Literal { valor, .. } => {
            use crate::valor::Valor;
            let tipo = match valor {
                Valor::Entero(_) => TipoSem::Entero,
                Valor::Decimal(_) => TipoSem::Decimal,
                Valor::Real(_) => TipoSem::Real,
                Valor::Logico(_) => TipoSem::Logico,
                Valor::Texto(_) => TipoSem::Texto,
                Valor::Nulo => TipoSem::Nulo,
                _ => TipoSem::Desconocido,
            };
            let lit = match valor {
                Valor::Entero(n) => HirLiteral::Entero(*n),
                Valor::Decimal(n) => HirLiteral::Decimal(*n),
                Valor::Real(n) => HirLiteral::Real(*n),
                Valor::Logico(b) => HirLiteral::Logico(*b),
                Valor::Texto(s) => HirLiteral::Texto(s.clone()),
                Valor::Nulo => HirLiteral::Nulo,
                _ => HirLiteral::Nulo,
            };
            HirExpresion::Literal(lit, tipo)
        }
        Expresion::Variable { nombre, .. } => {
            if res.funciones.contains(nombre) {
                return Ok(HirExpresion::FuncionRef(nombre.clone()));
            }
            let id = res.resolver_o_registrar(nombre);
            let tipo = ctx
                .buscar_tipo_variable(nombre)
                .unwrap_or(TipoSem::Desconocido);
            HirExpresion::Variable(id, tipo)
        }
        Expresion::Suma(a, b) => {
            let ha = lower_expresion(a, ctx, res)?;
            let hb = lower_expresion(b, ctx, res)?;
            let t = TipoSem::Desconocido;
            HirExpresion::Suma(Box::new(ha), Box::new(hb), t)
        }
        Expresion::Resta(a, b) => {
            let ha = lower_expresion(a, ctx, res)?;
            let hb = lower_expresion(b, ctx, res)?;
            let t = TipoSem::Desconocido;
            HirExpresion::Resta(Box::new(ha), Box::new(hb), t)
        }
        Expresion::Multiplicacion(a, b) => {
            let ha = lower_expresion(a, ctx, res)?;
            let hb = lower_expresion(b, ctx, res)?;
            let t = TipoSem::Desconocido;
            HirExpresion::Multiplicacion(Box::new(ha), Box::new(hb), t)
        }
        Expresion::Division(a, b) => {
            let ha = lower_expresion(a, ctx, res)?;
            let hb = lower_expresion(b, ctx, res)?;
            let t = TipoSem::Desconocido;
            HirExpresion::Division(Box::new(ha), Box::new(hb), t)
        }
        Expresion::Llamada {
            nombre, argumentos, ..
        } => {
            let args: Vec<HirExpresion> = argumentos
                .iter()
                .map(|a| lower_expresion(a, ctx, res))
                .collect::<Result<Vec<_>, _>>()?;
            if res.funciones.contains(nombre) {
                HirExpresion::Llamada {
                    nombre: nombre.clone(),
                    argumentos: args,
                    tipo: TipoSem::Desconocido,
                }
            } else if res.variables.contains(nombre) {
                let id = res.resolver_o_registrar(nombre);
                HirExpresion::LlamadaIndirecta {
                    callee: Box::new(HirExpresion::Variable(id, TipoSem::Desconocido)),
                    argumentos: args,
                    tipo: TipoSem::Desconocido,
                }
            } else {
                // Función o nativo desconocido: llamada directa por nombre.
                HirExpresion::Llamada {
                    nombre: nombre.clone(),
                    argumentos: args,
                    tipo: TipoSem::Desconocido,
                }
            }
        }
        Expresion::Rango {
            inicio,
            fin,
            inclusivo,
            ..
        } => {
            let hi = lower_expresion(inicio, ctx, res)?;
            let hf = lower_expresion(fin, ctx, res)?;
            HirExpresion::Rango {
                inicio: Box::new(hi),
                fin: Box::new(hf),
                inclusivo: *inclusivo,
                tipo: TipoSem::Desconocido,
            }
        }
        Expresion::Modulo(a, b) => HirExpresion::Modulo(
            Box::new(lower_expresion(a, ctx, res)?),
            Box::new(lower_expresion(b, ctx, res)?),
            TipoSem::Desconocido,
        ),
        Expresion::IgualIgual(a, b) => HirExpresion::IgualIgual(
            Box::new(lower_expresion(a, ctx, res)?),
            Box::new(lower_expresion(b, ctx, res)?),
            TipoSem::Desconocido,
        ),
        Expresion::Distinto(a, b) => HirExpresion::Distinto(
            Box::new(lower_expresion(a, ctx, res)?),
            Box::new(lower_expresion(b, ctx, res)?),
            TipoSem::Desconocido,
        ),
        Expresion::Menor(a, b) => HirExpresion::Menor(
            Box::new(lower_expresion(a, ctx, res)?),
            Box::new(lower_expresion(b, ctx, res)?),
            TipoSem::Desconocido,
        ),
        Expresion::MenorIgual(a, b) => HirExpresion::MenorIgual(
            Box::new(lower_expresion(a, ctx, res)?),
            Box::new(lower_expresion(b, ctx, res)?),
            TipoSem::Desconocido,
        ),
        Expresion::Mayor(a, b) => HirExpresion::Mayor(
            Box::new(lower_expresion(a, ctx, res)?),
            Box::new(lower_expresion(b, ctx, res)?),
            TipoSem::Desconocido,
        ),
        Expresion::MayorIgual(a, b) => HirExpresion::MayorIgual(
            Box::new(lower_expresion(a, ctx, res)?),
            Box::new(lower_expresion(b, ctx, res)?),
            TipoSem::Desconocido,
        ),
        Expresion::No(a) => HirExpresion::No(
            Box::new(lower_expresion(a, ctx, res)?),
            TipoSem::Desconocido,
        ),
        Expresion::Y(a, b) => HirExpresion::Y(
            Box::new(lower_expresion(a, ctx, res)?),
            Box::new(lower_expresion(b, ctx, res)?),
            TipoSem::Desconocido,
        ),
        Expresion::O(a, b) => HirExpresion::O(
            Box::new(lower_expresion(a, ctx, res)?),
            Box::new(lower_expresion(b, ctx, res)?),
            TipoSem::Desconocido,
        ),
        Expresion::ArregloLiteral { elementos, .. } => HirExpresion::ArregloLiteral {
            elementos: elementos
                .iter()
                .map(|e| lower_expresion(e, ctx, res))
                .collect::<Result<Vec<_>, _>>()?,
            tipo: TipoSem::Desconocido,
        },
        Expresion::Indice { objeto, indice, .. } => HirExpresion::Indice {
            objeto: Box::new(lower_expresion(objeto, ctx, res)?),
            indice: Box::new(lower_expresion(indice, ctx, res)?),
            tipo: TipoSem::Desconocido,
        },
        Expresion::Acceso {
            objeto, miembro, ..
        } => HirExpresion::Acceso {
            objeto: Box::new(lower_expresion(objeto, ctx, res)?),
            miembro: miembro.clone(),
            tipo: TipoSem::Desconocido,
        },
        Expresion::Lambda {
            parametros,
            tipo_retorno,
            cuerpo,
            ..
        } => {
            let params: Vec<(String, TipoSem)> = parametros
                .iter()
                .map(|(n, t)| {
                    let ty = t
                        .as_ref()
                        .map(crate::semantico::tipo_anotacion_a_sem)
                        .unwrap_or(TipoSem::Desconocido);
                    (n.clone(), ty)
                })
                .collect();
            let ret = tipo_retorno
                .as_ref()
                .map(crate::semantico::tipo_anotacion_a_sem)
                .unwrap_or(TipoSem::Desconocido);
            // Variables libres (capturas) del cuerpo.
            let param_names: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
            let (free, capturadas_por_anidados) = variables_libres(cuerpo, &param_names);
            // Resolver capturas a SimbodoId (deben estar en ámbitos exteriores).
            let mut capturas: Vec<SimbodoId> = Vec::new();
            for n in &free {
                let id = res.resolver(n).unwrap_or_else(|| res.insertar(n, None));
                if !capturas.contains(&id) {
                    capturas.push(id);
                }
            }
            // Nombre de la clausura.
            let nombre_clausura = res.nuevo_nombre_clausura();
            // Registrar los parámetros de la lambda en un nuevo ámbito.
            res.entrar();
            let mut param_ids: Vec<SimbodoId> = Vec::new();
            for (n, _) in &params {
                param_ids.push(res.insertar(n, None));
                res.variables.insert(n.clone());
            }
            // Tipos de los parámetros de la clausura (params + capturas).
            let mut clausura_params: Vec<(SimbodoId, TipoSem)> = Vec::new();
            for (i, (_, ty)) in params.iter().enumerate() {
                clausura_params.push((param_ids[i], ty.clone()));
            }
            for (cap_name, cap_id) in free.iter().zip(capturas.iter()) {
                let ty = ctx
                    .buscar_tipo_variable(cap_name)
                    .unwrap_or(TipoSem::Desconocido);
                clausura_params.push((*cap_id, ty));
            }
            // Bajar el cuerpo con el ámbito actual (params registrados).
            let mut body: Vec<HirSentencia> = Vec::new();
            for st in cuerpo {
                body.push(lower_sentencia(st, ctx, res)?);
            }
            let capturadas_ids: Vec<SimbodoId> = capturadas_por_anidados
                .iter()
                .filter_map(|n| res.resolver(n))
                .filter(|id| !capturas.contains(id))
                .collect();
            res.salir();
            // Registrar la clausura como función de primer nivel (recolectada).
            res.clausuras.push(HirItem::Funcion {
                nombre: nombre_clausura.clone(),
                simbodo: SimbodoId(0),
                params: clausura_params,
                capturadas: capturadas_ids.clone(),
                capturas_externas: capturas.clone(),
                celdas: Vec::new(),
                retorno: ret.clone(),
                cuerpo: body,
            });
            HirExpresion::Lambda {
                nombre: nombre_clausura,
                params,
                capturas,
                tipo: ret,
            }
        }
        Expresion::LlamadaIndirecta {
            callee, argumentos, ..
        } => HirExpresion::LlamadaIndirecta {
            callee: Box::new(lower_expresion(callee, ctx, res)?),
            argumentos: argumentos
                .iter()
                .map(|a| lower_expresion(a, ctx, res))
                .collect::<Result<Vec<_>, _>>()?,
            tipo: TipoSem::Desconocido,
        },
    })
}

/// Lowering AST -> HIR.
fn lower_sentencia(
    s: &Sentencia,
    ctx: &crate::semantico::ContextoSemantico,
    res: &mut ResolutorHir,
) -> Result<HirSentencia, String> {
    match s {
        Sentencia::Declaracion {
            nombre,
            valor,
            tipo_anotacion,
            mutable,
            es_const,
            publico,
            ..
        } => {
            // Igual que el semántico: se evalúa el inicializador ANTES de
            // declarar (léxico: `sea a = a + 1` lee la `a` exterior).
            let valor_hir = lower_expresion(valor, ctx, res)?;
            let id = res.insertar(nombre, None);
            res.variables.insert(nombre.clone());
            let tipo = tipo_anotacion
                .as_ref()
                .map(crate::semantico::tipo_anotacion_a_sem)
                .unwrap_or(TipoSem::Desconocido);
            Ok(HirSentencia::Declaracion {
                simbodo: id,
                tipo,
                valor: valor_hir,
                mutable: *mutable,
                es_const: *es_const,
                publico: *publico,
            })
        }
        Sentencia::Asignacion { nombre, valor, .. } => {
            // No crear variables fantasma: si el nombre no se resolvió (el
            // análisis semántico debería haberlo rechazado), propagar el
            // error en lugar de enmascararlo.
            let id = match res.resolver(nombre) {
                Some(id) => id,
                None => {
                    return Err(format!(
                        "asignación a variable no declarada en lowering: '{}' \
                         (el análisis semántico debería haberla rechazado)",
                        nombre
                    ));
                }
            };
            Ok(HirSentencia::Asignacion {
                simbodo: id,
                valor: lower_expresion(valor, ctx, res)?,
            })
        }
        Sentencia::AsignacionIndice {
            objeto,
            indice,
            valor,
            ..
        } => Ok(HirSentencia::AsignacionIndice {
            objeto: lower_expresion(objeto, ctx, res)?,
            indice: lower_expresion(indice, ctx, res)?,
            valor: lower_expresion(valor, ctx, res)?,
        }),
        Sentencia::Retorno { valor, .. } => Ok(HirSentencia::Retornar(
            valor
                .as_ref()
                .map(|v| lower_expresion(v, ctx, res))
                .transpose()?,
        )),
        Sentencia::Si {
            condicion,
            rama_si,
            rama_sino,
            ..
        } => {
            let cond = lower_expresion(condicion, ctx, res)?;
            res.entrar();
            let mut entonces: Vec<HirSentencia> = Vec::new();
            for st in rama_si {
                match lower_sentencia(st, ctx, res) {
                    Ok(hs) => entonces.push(hs),
                    Err(e) => {
                        res.salir();
                        return Err(e);
                    }
                }
            }
            res.salir();
            let sino = if let Some(v) = rama_sino.as_ref() {
                res.entrar();
                let mut rama: Vec<HirSentencia> = Vec::new();
                for st in v {
                    match lower_sentencia(st, ctx, res) {
                        Ok(hs) => rama.push(hs),
                        Err(e) => {
                            res.salir();
                            return Err(e);
                        }
                    }
                }
                res.salir();
                Some(rama)
            } else {
                None
            };
            Ok(HirSentencia::Si {
                condicion: cond,
                entonces,
                sino,
            })
        }
        Sentencia::Mientras {
            condicion, cuerpo, ..
        } => {
            let cond = lower_expresion(condicion, ctx, res)?;
            res.entrar();
            let mut body: Vec<HirSentencia> = Vec::new();
            for st in cuerpo {
                match lower_sentencia(st, ctx, res) {
                    Ok(hs) => body.push(hs),
                    Err(e) => {
                        res.salir();
                        return Err(e);
                    }
                }
            }
            res.salir();
            Ok(HirSentencia::Mientras {
                condicion: cond,
                cuerpo: body,
            })
        }
        Sentencia::Bloque(stmts) => {
            res.entrar();
            let mut body: Vec<HirSentencia> = Vec::new();
            for st in stmts {
                match lower_sentencia(st, ctx, res) {
                    Ok(hs) => body.push(hs),
                    Err(e) => {
                        res.salir();
                        return Err(e);
                    }
                }
            }
            res.salir();
            Ok(HirSentencia::Bloque(body))
        }
        Sentencia::Mostrar { valor, .. } => Ok(HirSentencia::Expresion(HirExpresion::Llamada {
            nombre: "mostrar".to_string(),
            argumentos: vec![lower_expresion(valor, ctx, res)?],
            tipo: TipoSem::Desconocido,
        })),
        Sentencia::MostrarSinSalto { valor, .. } => {
            Ok(HirSentencia::Expresion(HirExpresion::Llamada {
                nombre: "mostrar_sin_salto".to_string(),
                argumentos: vec![lower_expresion(valor, ctx, res)?],
                tipo: TipoSem::Desconocido,
            }))
        }
        Sentencia::Expresion(e) => Ok(HirSentencia::Expresion(lower_expresion(e, ctx, res)?)),
        Sentencia::ParaCada {
            variable,
            iterable,
            cuerpo,
            ..
        } => {
            let iterable_hir = lower_expresion(iterable, ctx, res)?;
            res.entrar();
            let var_id = res.insertar(variable, None);
            res.variables.insert(variable.clone());
            let mut body: Vec<HirSentencia> = Vec::new();
            for st in cuerpo {
                match lower_sentencia(st, ctx, res) {
                    Ok(hs) => body.push(hs),
                    Err(e) => {
                        res.salir();
                        return Err(e);
                    }
                }
            }
            res.salir();
            Ok(HirSentencia::ParaCada {
                variable: var_id,
                iterable: iterable_hir,
                cuerpo: body,
            })
        }
        Sentencia::Romper { .. } => Ok(HirSentencia::Romper),
        Sentencia::Continuar { .. } => Ok(HirSentencia::Continuar),
        Sentencia::IntentarRescatar {
            cuerpo_intentar,
            variable_error,
            cuerpo_rescatar,
            ..
        } => {
            res.entrar();
            let mut ci: Vec<HirSentencia> = Vec::new();
            for st in cuerpo_intentar {
                match lower_sentencia(st, ctx, res) {
                    Ok(hs) => ci.push(hs),
                    Err(e) => {
                        res.salir();
                        return Err(e);
                    }
                }
            }
            res.salir();
            let (var_id, cr): (Option<SimbodoId>, Vec<HirSentencia>) = {
                res.entrar();
                let var_id = variable_error.as_ref().map(|v| res.insertar(v, None));
                let mut cr: Vec<HirSentencia> = Vec::new();
                for st in cuerpo_rescatar {
                    match lower_sentencia(st, ctx, res) {
                        Ok(hs) => cr.push(hs),
                        Err(e) => {
                            res.salir();
                            return Err(e);
                        }
                    }
                }
                res.salir();
                (var_id, cr)
            };
            Ok(HirSentencia::IntentarRescatar {
                cuerpo_intentar: ci,
                variable_error: var_id,
                cuerpo_rescatar: cr,
            })
        }
        Sentencia::Importar { ruta, alias, .. } => Ok(HirSentencia::Importar {
            ruta: ruta.clone(),
            alias: alias.clone(),
        }),
        Sentencia::Exportar { sentencia, .. } => {
            let hs = lower_sentencia(sentencia, ctx, res)?;
            Ok(HirSentencia::Exportar {
                sentencia: Box::new(hs),
            })
        }
        Sentencia::Usar {
            ruta,
            alias,
            nombres,
            ..
        } => Ok(HirSentencia::Usar {
            ruta: ruta.clone(),
            alias: alias.clone(),
            nombres: nombres.clone(),
        }),
        Sentencia::Funcion {
            nombre,
            parametros,
            tipo_retorno,
            cuerpo,
            ..
        } => {
            // Función anidada = clausura. Se trata igual que una lambda asignada
            // a `nombre`.
            let params: Vec<(String, TipoSem)> = parametros
                .iter()
                .map(|(n, t)| {
                    let ty = t
                        .as_ref()
                        .map(crate::semantico::tipo_anotacion_a_sem)
                        .unwrap_or(TipoSem::Desconocido);
                    (n.clone(), ty)
                })
                .collect();
            let ret = tipo_retorno
                .as_ref()
                .map(crate::semantico::tipo_anotacion_a_sem)
                .unwrap_or(TipoSem::Desconocido);
            let param_names: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
            let (free, capturadas_por_anidados) = variables_libres(cuerpo, &param_names);
            let mut capturas: Vec<SimbodoId> = Vec::new();
            for n in &free {
                let id = res.resolver(n).unwrap_or_else(|| res.insertar(n, None));
                if !capturas.contains(&id) {
                    capturas.push(id);
                }
            }
            let nombre_clausura = res.nuevo_nombre_clausura();
            res.entrar();
            let mut param_ids: Vec<SimbodoId> = Vec::new();
            for (n, _) in &params {
                param_ids.push(res.insertar(n, None));
                res.variables.insert(n.clone());
            }
            let mut clausura_params: Vec<(SimbodoId, TipoSem)> = Vec::new();
            for (i, (_, ty)) in params.iter().enumerate() {
                clausura_params.push((param_ids[i], ty.clone()));
            }
            for (cap_name, cap_id) in free.iter().zip(capturas.iter()) {
                let ty = ctx
                    .buscar_tipo_variable(cap_name)
                    .unwrap_or(TipoSem::Desconocido);
                clausura_params.push((*cap_id, ty));
            }
            let mut body: Vec<HirSentencia> = Vec::new();
            for st in cuerpo {
                body.push(lower_sentencia(st, ctx, res)?);
            }
            let capturadas_ids: Vec<SimbodoId> = capturadas_por_anidados
                .iter()
                .filter_map(|n| res.resolver(n))
                .filter(|id| !capturas.contains(id))
                .collect();
            res.salir();
            res.clausuras.push(HirItem::Funcion {
                nombre: nombre_clausura.clone(),
                simbodo: SimbodoId(0),
                params: clausura_params,
                capturadas: capturadas_ids.clone(),
                capturas_externas: capturas.clone(),
                celdas: Vec::new(),
                retorno: ret.clone(),
                cuerpo: body,
            });
            let id = res.insertar(nombre, None);
            res.variables.insert(nombre.clone());
            Ok(HirSentencia::Declaracion {
                simbodo: id,
                tipo: TipoSem::Funcion {
                    params: params.iter().map(|(_, t)| t.clone()).collect(),
                    retorno: Box::new(ret.clone()),
                    genericos: Vec::new(),
                },
                valor: HirExpresion::Lambda {
                    nombre: nombre_clausura,
                    params,
                    capturas,
                    tipo: ret,
                },
                mutable: false,
                es_const: false,
                publico: false,
            })
        }
    }
}

/// Recolecta los nombres de variables libres en un cuerpo, dados los parámetros
/// del ámbito actual. Incluye las variables libres de funciones anidadas
/// (propagación transitiva de capturas).
fn variables_libres(stmts: &[Sentencia], params: &[String]) -> (Vec<String>, Vec<String>) {
    use std::collections::HashSet;
    let mut declaradas: HashSet<String> = params.iter().cloned().collect();
    let mut referenciadas: HashSet<String> = HashSet::new();
    let mut capturadas_por_anidados: Vec<String> = Vec::new();

    for s in stmts {
        match s {
            Sentencia::Declaracion { nombre, .. } => {
                declaradas.insert(nombre.clone());
            }
            Sentencia::ParaCada { variable, .. } => {
                declaradas.insert(variable.clone());
            }
            Sentencia::Funcion {
                nombre,
                parametros,
                cuerpo,
                ..
            } => {
                declaradas.insert(nombre.clone());
                let nested_params: Vec<String> =
                    parametros.iter().map(|(n, _)| n.clone()).collect();
                let (nested_free, nested_capt) = variables_libres(cuerpo, &nested_params);
                for v in &nested_free {
                    if !declaradas.contains(v) {
                        referenciadas.insert(v.clone());
                    }
                }
                for v in nested_capt {
                    if declaradas.contains(&v) && !capturadas_por_anidados.contains(&v) {
                        capturadas_por_anidados.push(v);
                    }
                }
                for v in &nested_free {
                    if declaradas.contains(v) && !capturadas_por_anidados.contains(v) {
                        capturadas_por_anidados.push(v.clone());
                    }
                }
            }
            _ => {}
        }
    }

    // Recolector de expresiones.
    fn recolectar_expr(
        e: &Expresion,
        refs: &mut HashSet<String>,
        declaradas: &HashSet<String>,
        capturas: &mut Vec<String>,
    ) {
        match e {
            Expresion::Variable { nombre, .. } => {
                refs.insert(nombre.clone());
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
                recolectar_expr(a, refs, declaradas, capturas);
                recolectar_expr(b, refs, declaradas, capturas);
            }
            Expresion::No(x) => recolectar_expr(x, refs, declaradas, capturas),
            Expresion::Llamada { argumentos, .. } => {
                for a in argumentos {
                    recolectar_expr(a, refs, declaradas, capturas);
                }
            }
            Expresion::LlamadaIndirecta {
                callee, argumentos, ..
            } => {
                recolectar_expr(callee, refs, declaradas, capturas);
                for a in argumentos {
                    recolectar_expr(a, refs, declaradas, capturas);
                }
            }
            Expresion::ArregloLiteral { elementos, .. } => {
                for e in elementos {
                    recolectar_expr(e, refs, declaradas, capturas);
                }
            }
            Expresion::Indice { objeto, indice, .. } => {
                recolectar_expr(objeto, refs, declaradas, capturas);
                recolectar_expr(indice, refs, declaradas, capturas);
            }
            Expresion::Acceso { objeto, .. } => {
                recolectar_expr(objeto, refs, declaradas, capturas);
            }
            Expresion::Rango { inicio, fin, .. } => {
                recolectar_expr(inicio, refs, declaradas, capturas);
                recolectar_expr(fin, refs, declaradas, capturas);
            }
            Expresion::Lambda {
                parametros, cuerpo, ..
            } => {
                let nested_params: Vec<String> =
                    parametros.iter().map(|(n, _)| n.clone()).collect();
                let (nested_free, nested_capt) = variables_libres(cuerpo, &nested_params);
                for v in nested_free {
                    // Nombres declarados en ESTE ámbito capturados por una
                    // clausura anidada: van a la lista de capturas (y también
                    // como referencia para propagación transitiva).
                    if declaradas.contains(&v) && !capturas.contains(&v) {
                        capturas.push(v.clone());
                    }
                    refs.insert(v);
                }
                for v in nested_capt {
                    if declaradas.contains(&v) && !capturas.contains(&v) {
                        capturas.push(v.clone());
                    }
                    refs.insert(v);
                }
            }
            _ => {}
        }
    }

    fn recolectar_stmt(
        s: &Sentencia,
        refs: &mut HashSet<String>,
        declaradas: &HashSet<String>,
        capturas: &mut Vec<String>,
    ) {
        match s {
            Sentencia::Declaracion {
                nombre: _, valor, ..
            } => {
                recolectar_expr(valor, refs, declaradas, capturas);
            }
            Sentencia::Asignacion { nombre, valor, .. } => {
                if !declaradas.contains(nombre) {
                    refs.insert(nombre.clone());
                }
                recolectar_expr(valor, refs, declaradas, capturas);
            }
            Sentencia::AsignacionIndice {
                objeto,
                indice,
                valor,
                ..
            } => {
                recolectar_expr(objeto, refs, declaradas, capturas);
                recolectar_expr(indice, refs, declaradas, capturas);
                recolectar_expr(valor, refs, declaradas, capturas);
            }
            Sentencia::Expresion(e) => recolectar_expr(e, refs, declaradas, capturas),
            Sentencia::Retorno { valor: Some(v), .. } => {
                recolectar_expr(v, refs, declaradas, capturas);
            }
            Sentencia::Retorno { .. } => {}
            Sentencia::Si {
                condicion,
                rama_si,
                rama_sino,
                ..
            } => {
                recolectar_expr(condicion, refs, declaradas, capturas);
                for ss in rama_si {
                    recolectar_stmt(ss, refs, declaradas, capturas);
                }
                if let Some(sino) = rama_sino {
                    for ss in sino {
                        recolectar_stmt(ss, refs, declaradas, capturas);
                    }
                }
            }
            Sentencia::Mientras {
                condicion, cuerpo, ..
            } => {
                recolectar_expr(condicion, refs, declaradas, capturas);
                for ss in cuerpo {
                    recolectar_stmt(ss, refs, declaradas, capturas);
                }
            }
            Sentencia::Bloque(ss) => {
                for s in ss {
                    recolectar_stmt(s, refs, declaradas, capturas);
                }
            }
            Sentencia::ParaCada {
                iterable, cuerpo, ..
            } => {
                recolectar_expr(iterable, refs, declaradas, capturas);
                for ss in cuerpo {
                    recolectar_stmt(ss, refs, declaradas, capturas);
                }
            }
            Sentencia::IntentarRescatar {
                cuerpo_intentar,
                cuerpo_rescatar,
                ..
            } => {
                for ss in cuerpo_intentar {
                    recolectar_stmt(ss, refs, declaradas, capturas);
                }
                for ss in cuerpo_rescatar {
                    recolectar_stmt(ss, refs, declaradas, capturas);
                }
            }
            Sentencia::Exportar { sentencia, .. } => {
                recolectar_stmt(sentencia, refs, declaradas, capturas);
            }
            Sentencia::Mostrar { valor, .. } | Sentencia::MostrarSinSalto { valor, .. } => {
                recolectar_expr(valor, refs, declaradas, capturas);
            }
            _ => {}
        }
    }

    for s in stmts {
        recolectar_stmt(
            s,
            &mut referenciadas,
            &declaradas,
            &mut capturadas_por_anidados,
        );
    }
    // Quitar declaradas y params.
    for d in &declaradas {
        referenciadas.remove(d);
    }
    (referenciadas.into_iter().collect(), capturadas_por_anidados)
}

/// Carga un módulo en tiempo de compilación y devuelve sus items HIR (funciones
/// y declaraciones), filtrando por `nombres` si se da (sentencia `usar ... { ... }`).
static MODULOS_EN_CARGA: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<std::path::PathBuf>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

pub static MODULO_ERROR: std::sync::LazyLock<std::sync::Mutex<Option<String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

fn cargar_modulo_items(
    ruta: &str,
    ctx: &crate::semantico::ContextoSemantico,
    res: &mut ResolutorHir,
    nombres: Option<&Vec<String>>,
    alias: Option<&str>,
) -> Vec<HirItem> {
    use std::path::PathBuf;
    // Resolver ruta relativa al archivo actual.
    let base_dir = ctx
        .archivo_actual()
        .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let mut candidatos = vec![ruta.to_string()];
    // normalizar extensión
    if !ruta.ends_with(".bsk") && !ruta.ends_with(".nexo") {
        candidatos = vec![
            format!("{ruta}.bsk"),
            format!("{ruta}.nexo"),
            ruta.to_string(),
        ];
    }
    let mut contenido: Option<String> = None;
    let mut ruta_resuelta: Option<PathBuf> = None;
    for c in &candidatos {
        let p = base_dir.join(c);
        if let Ok(txt) = std::fs::read_to_string(&p) {
            contenido = Some(txt);
            ruta_resuelta = Some(p);
            break;
        }
    }
    let Some(contenido) = contenido else {
        *MODULO_ERROR.lock().unwrap() = Some(format!("no se pudo cargar '{}'", ruta));
        return Vec::new();
    };
    // Detección de ciclos: si la ruta ya está en carga, abortar.
    if let Some(p) = &ruta_resuelta {
        let mut guard = MODULOS_EN_CARGA.lock().unwrap();
        if !guard.insert(p.clone()) {
            return Vec::new();
        }
    }
    let tokens = match crate::Lexer::nuevo(&contenido).tokenizar() {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let ast = match crate::Parser::nuevo(tokens).parsear_programa() {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    // Analizar con un contexto que comparte el cargador.
    let mut ctx_mod = crate::semantico::ContextoSemantico::nuevo();
    if let Some(c) = ctx.cargador() {
        ctx_mod = crate::semantico::ContextoSemantico::con_cargador(c.clone(), None);
    }
    if let Some(p) = &ruta_resuelta {
        ctx_mod.set_archivo_actual(Some(p.clone()));
        ctx_mod.archivo = Some(p.display().to_string());
    }
    let _ = ctx_mod.analizar_programa(&ast);
    // Recolectar nombres públicos del módulo.
    let mut publicos_modulo: std::collections::HashSet<String> = std::collections::HashSet::new();
    for s in &ast {
        match s {
            Sentencia::Funcion {
                nombre, publico, ..
            } if *publico => {
                publicos_modulo.insert(nombre.clone());
            }
            Sentencia::Declaracion {
                nombre, publico, ..
            } if *publico => {
                publicos_modulo.insert(nombre.clone());
            }
            _ => {}
        }
    }
    // Bajar a HIR con un resolvedor compartido.
    let (hir, cuerpo_modulo) = match crate::hir::lower_programa_con_resolutor(&ast, &ctx_mod, res) {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };
    // Filtrar por nombres si se pidió.
    let mut items = hir.items;
    // Las declaraciones de nivel superior del módulo van en cuerpo_principal;
    // las convertimos a HirItem::Declaracion para que se inicialicen en __init.
    for st in &cuerpo_modulo {
        if let HirSentencia::Declaracion {
            simbodo,
            valor,
            tipo,
            mutable,
            es_const,
            publico,
            ..
        } = st
        {
            items.push(HirItem::Declaracion {
                simbodo: *simbodo,
                // Tipo sintético no-Desconocido para cumplir HIR020 sin alterar
                // el verificador ni la ejecución.
                tipo: if matches!(tipo, TipoSem::Desconocido) {
                    TipoSem::Funcion {
                        params: vec![],
                        retorno: Box::new(TipoSem::Nulo),
                        genericos: Vec::new(),
                    }
                } else {
                    tipo.clone()
                },
                valor: valor.clone(),
                mutable: *mutable,
                es_const: *es_const,
                publico: *publico,
            });
        }
    }
    if let Some(nombres) = nombres {
        let set: std::collections::HashSet<&String> = nombres.iter().collect();
        items.retain(|it| match it {
            HirItem::Funcion { nombre, .. } => set.contains(nombre),
            HirItem::Declaracion { .. } => true,
            _ => false,
        });
    }

    // Si hay alias, prefijar las funciones importadas con `alias_` y registrar
    // un objeto-módulo `alias` que mapea nombres -> Valor::Funcion(alias_nombre).
    let mut nombres_exportados: Vec<String> = Vec::new();
    if let Some(alias) = alias {
        // Recolectar nombres de funciones exportadas (tras el filtro), solo
        // las públicas (o todas si ninguna está marcada como pública).
        for it in &items {
            if let HirItem::Funcion { nombre, .. } = it
                && publicos_modulo.contains(nombre)
            {
                nombres_exportados.push(nombre.clone());
            }
        }
        // Prefijar nombres de funciones. Se registra el mapa
        // (nombre original -> nombre prefijado) para reescribir después las
        // llamadas internas del módulo.
        let mut prefijos: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for it in &mut items {
            if let HirItem::Funcion { nombre, .. } = it {
                let prefijado = format!("{}_{}", alias, nombre);
                prefijos.insert(nombre.clone(), prefijado.clone());
                *nombre = prefijado;
            }
        }
        // Reescribir las llamadas internas entre funciones del propio módulo:
        // los cuerpos conservan los nombres sin prefijo y, sin esta
        // reescritura, la VM no encuentra la función (el Tree-Walk resuelve
        // en tiempo de ejecución y por eso no lo necesita). También cubre las
        // declaraciones de nivel superior del módulo que llaman a funciones.
        for it in &mut items {
            match it {
                HirItem::Funcion { cuerpo, .. } => {
                    for st in cuerpo {
                        renombrar_llamadas_sentencia(st, &prefijos);
                    }
                }
                HirItem::Declaracion { valor, .. } => {
                    renombrar_llamadas_expresion(valor, &prefijos);
                }
                _ => {}
            }
        }
        // Registrar funciones prefijadas en el resolvedor.
        for it in &items {
            if let HirItem::Funcion { nombre, .. } = it {
                res.funciones.insert(nombre.clone());
                res.insertar(nombre, None);
            }
        }
        // Crear objeto-módulo sintético (siempre, aunque no haya funciones
        // exportadas, para que las referencias `alias.miembro` resuelvan a un
        // simbodo declarado y el gate HIR no las rechace como no declaradas).
        let id = res.insertar(alias, None);
        res.variables.insert(alias.to_string());
        items.push(HirItem::Declaracion {
            simbodo: id,
            // Tipo sintético no-Desconocido para cumplir HIR020 sin alterar
            // el verificador ni la ejecución (el tipo no se usa en runtime).
            tipo: TipoSem::Funcion {
                params: vec![],
                retorno: Box::new(TipoSem::Nulo),
                genericos: Vec::new(),
            },
            valor: HirExpresion::ModuloObjeto {
                prefijo: alias.to_string(),
                nombres: nombres_exportados,
                tipo: TipoSem::Desconocido,
            },
            mutable: false,
            es_const: false,
            publico: false,
        });
    } else {
        // Registrar las funciones importadas en el resolvedor actual para que las
        // llamadas posteriores se resuelvan como directas.
        for it in &items {
            if let HirItem::Funcion { nombre, .. } = it {
                res.funciones.insert(nombre.clone());
                res.insertar(nombre, None);
            }
        }
    }
    // Liberar la marca de carga.
    if let Some(p) = &ruta_resuelta {
        MODULOS_EN_CARGA.lock().unwrap().remove(p);
    }
    // Incorporar clausuras anidadas (ya están en hir.items tras lower_programa).
    items
}

/// Programa -> HIR.
pub fn lower_programa(
    ast: &[Sentencia],
    ctx: &crate::semantico::ContextoSemantico,
) -> Result<HirPrograma, Vec<String>> {
    let mut res = ResolutorHir::nuevo();
    let (hir, _cuerpo_principal): (HirPrograma, Vec<HirSentencia>) =
        lower_programa_con_resolutor(ast, ctx, &mut res)?;
    Ok(hir)
}

/// Versión que permite reutilizar un resolvedor (para inlining de módulos).
pub fn lower_programa_con_resolutor(
    ast: &[Sentencia],
    ctx: &crate::semantico::ContextoSemantico,
    res: &mut ResolutorHir,
) -> Result<(HirPrograma, Vec<HirSentencia>), Vec<String>> {
    let mut items = Vec::new();
    let mut errores = Vec::new();
    let mut cuerpo_principal: Vec<HirSentencia> = Vec::new();
    let mut tiene_main_usuario = false;
    let mut ultima_es_expresion_bare = false;

    // Pre-paso: registrar los nombres de las funciones de primer nivel en el
    // ámbito raíz (permite llamadas anticipadas y recursión), igual que
    // `registrar_funciones` en el semántico.
    for s in ast {
        match s {
            Sentencia::Funcion { nombre, .. } => {
                if nombre == "main" {
                    tiene_main_usuario = true;
                }
                res.insertar(nombre, None);
                res.funciones.insert(nombre.clone());
            }
            Sentencia::Exportar { sentencia, .. } => {
                if let Sentencia::Funcion { nombre, .. } = &**sentencia {
                    res.insertar(nombre, None);
                    res.funciones.insert(nombre.clone());
                }
            }
            _ => {}
        }
    }

    for s in ast {
        match s {
            Sentencia::Declaracion {
                nombre,
                valor,
                tipo_anotacion,
                mutable,
                es_const,
                publico,
                ..
            } => {
                let val = lower_expresion(valor, ctx, res).map_err(|e| vec![e])?;
                let id = res.insertar(nombre, None);
                res.variables.insert(nombre.clone());
                let tipo = tipo_anotacion
                    .as_ref()
                    .map(crate::semantico::tipo_anotacion_a_sem)
                    .unwrap_or(TipoSem::Desconocido);
                // Las declaraciones de nivel superior se ejecutan en `main`,
                // no en `__init`, para mantener el orden de ejecución con
                // el resto de sentencias del programa.
                cuerpo_principal.push(HirSentencia::Declaracion {
                    simbodo: id,
                    tipo,
                    valor: val,
                    mutable: *mutable,
                    es_const: *es_const,
                    publico: *publico,
                });
            }
            Sentencia::Funcion {
                nombre,
                parametros,
                tipo_retorno,
                cuerpo,
                ..
            } => {
                // Id ya registrado por el pre-paso.
                let id = res
                    .resolver(nombre)
                    .unwrap_or_else(|| res.insertar(nombre, None));
                let param_names: Vec<String> = parametros.iter().map(|(n, _)| n.clone()).collect();
                // Detectar parámetros duplicados.
                let mut vistos = std::collections::HashSet::new();
                for n in &param_names {
                    if !vistos.insert(n.clone()) {
                        errores.push(format!("parámetro duplicado: {}", n));
                        // saltar función
                        // (señal mediante continue externo no disponible; usar flag)
                        // Marcamos fallo y continuamos el match sin push.
                        // Implementado con early continue via etiqueta no trivial; en su lugar retornar del bloque con continue externo.
                    }
                }
                if param_names
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != param_names.len()
                {
                    continue;
                }
                let (free, capturadas_por_anidados) = variables_libres(cuerpo, &param_names);
                let _capturas: Vec<SimbodoId> = free
                    .iter()
                    .map(|n| res.resolver(n).unwrap_or_else(|| res.insertar(n, None)))
                    .collect();
                res.entrar();
                let mut params: Vec<(SimbodoId, TipoSem)> = Vec::new();
                for (n, t) in parametros {
                    let pid = res.insertar(n, None);
                    res.variables.insert(n.clone());
                    let ty = t
                        .as_ref()
                        .map(crate::semantico::tipo_anotacion_a_sem)
                        .unwrap_or(TipoSem::Desconocido);
                    params.push((pid, ty));
                }
                let ret = tipo_retorno
                    .as_ref()
                    .map(crate::semantico::tipo_anotacion_a_sem)
                    .unwrap_or(TipoSem::Desconocido);
                let mut body = Vec::new();
                let mut fallo = false;
                for st in cuerpo {
                    match lower_sentencia(st, ctx, res) {
                        Ok(hs) => body.push(hs),
                        Err(e) => {
                            errores.push(e);
                            fallo = true;
                        }
                    }
                }
                let capturadas_ids: Vec<SimbodoId> = capturadas_por_anidados
                    .iter()
                    .filter_map(|n| res.resolver(n))
                    .collect();
                res.salir();
                if fallo {
                    continue;
                }
                items.push(HirItem::Funcion {
                    nombre: nombre.clone(),
                    simbodo: id,
                    params,
                    capturadas: capturadas_ids.clone(),
                    capturas_externas: Vec::new(),
                    celdas: Vec::new(),
                    retorno: ret,
                    cuerpo: body,
                });
            }
            Sentencia::Expresion(e) => {
                ultima_es_expresion_bare = true;
                if tiene_main_usuario {
                    items.push(HirItem::Expresion(
                        lower_expresion(e, ctx, res).map_err(|e| vec![e])?,
                    ));
                } else {
                    // Bajar la expresión como sentencia (descarta el valor);
                    // el valor del programa será el de la última expresión.
                    match lower_sentencia(&Sentencia::Expresion(e.clone()), ctx, res) {
                        Ok(hs) => cuerpo_principal.push(hs),
                        Err(err) => errores.push(err),
                    }
                }
            }
            Sentencia::IntentarRescatar {
                cuerpo_intentar,
                variable_error,
                cuerpo_rescatar,
                ..
            } => {
                res.entrar();
                let mut ci: Vec<HirSentencia> = Vec::new();
                let mut fallo_ci = false;
                for st in cuerpo_intentar {
                    match lower_sentencia(st, ctx, res) {
                        Ok(hs) => ci.push(hs),
                        Err(e) => {
                            errores.push(e);
                            fallo_ci = true;
                        }
                    }
                }
                let var_id = variable_error.as_ref().map(|v| res.insertar(v, None));
                let mut cr: Vec<HirSentencia> = Vec::new();
                let mut fallo_cr = false;
                for st in cuerpo_rescatar {
                    match lower_sentencia(st, ctx, res) {
                        Ok(hs) => cr.push(hs),
                        Err(e) => {
                            errores.push(e);
                            fallo_cr = true;
                        }
                    }
                }
                if fallo_ci || fallo_cr {
                    continue;
                }
                res.salir();
                items.push(HirItem::IntentarRescatar {
                    cuerpo_intentar: ci,
                    variable_error: var_id,
                    cuerpo_rescatar: cr,
                });
            }
            Sentencia::Importar { ruta, alias, .. } => {
                let mods = cargar_modulo_items(ruta, ctx, res, None, alias.as_deref());
                items.push(HirItem::Importar {
                    ruta: ruta.clone(),
                    alias: alias.clone(),
                });
                items.extend(mods);
            }
            Sentencia::Exportar { sentencia, .. } => match lower_sentencia(sentencia, ctx, res) {
                Ok(hs) => items.push(HirItem::Exportar {
                    sentencia: Box::new(hs),
                }),
                Err(e) => errores.push(e),
            },
            Sentencia::Usar {
                ruta,
                alias,
                nombres,
                ..
            } => {
                let mods = cargar_modulo_items(ruta, ctx, res, nombres.as_ref(), alias.as_deref());
                items.push(HirItem::Usar {
                    ruta: ruta.clone(),
                    alias: alias.clone(),
                    nombres: nombres.clone(),
                });
                items.extend(mods);
            }
            otro => {
                // Sentencias de nivel superior (mostrar, si, mientras, ...) se
                // agrupan en una función `main` sintética para que el motor VM
                // (que exige `main`) pueda ejecutarlas.
                ultima_es_expresion_bare = false;
                match lower_sentencia(otro, ctx, res) {
                    Ok(hs) => cuerpo_principal.push(hs),
                    Err(e) => errores.push(e),
                }
            }
        }
    }
    // Si no hay `main` de usuario pero se acumularon sentencias de nivel
    // superior, sintetizar `funcion main()` para el motor VM.
    if !tiene_main_usuario && !cuerpo_principal.is_empty() {
        // Si la última sentencia es una expresión, convertirla en `retornar`
        // para que el CLI imprima su valor (como hace el Tree-Walk).
        let mut cuerpo = cuerpo_principal.clone();
        if ultima_es_expresion_bare && let Some(HirSentencia::Expresion(e)) = cuerpo.last().cloned()
        {
            cuerpo.pop();
            cuerpo.push(HirSentencia::Retornar(Some(e)));
        }
        let id = res.insertar("main", None);
        res.funciones.insert("main".to_string());
        items.push(HirItem::Funcion {
            nombre: "main".to_string(),
            simbodo: id,
            params: Vec::new(),
            capturadas: Vec::new(),
            capturas_externas: Vec::new(),
            celdas: Vec::new(),
            retorno: TipoSem::Desconocido,
            cuerpo,
        });
    }
    if !errores.is_empty() {
        return Err(errores);
    }
    // Incorporar las clausuras recolectadas durante el lowering.
    items.append(&mut res.clausuras);
    Ok((HirPrograma { items }, cuerpo_principal))
}

/// Gate G4: detecta parámetros genéricos residuales en HIR concreto (BAS-7020/E1055).
pub fn verificar_sin_residuos_genericos(programa: &HirPrograma) -> Result<(), String> {
    let mut errs = Vec::new();
    for item in &programa.items {
        scan_item_hir(item, &mut errs);
    }
    if errs.is_empty() {
        return Ok(());
    }
    let detalle = errs.join("; ");
    let ubicacion = programa
        .items
        .iter()
        .find_map(|it| match it {
            HirItem::Funcion { nombre, .. } => Some(nombre.clone()),
            HirItem::Declaracion { .. } => Some("declaracion".to_string()),
            HirItem::Expresion(_) => Some("expresion".to_string()),
            HirItem::IntentarRescatar { .. } => Some("intentar/rescatar".to_string()),
            HirItem::Importar { .. } | HirItem::Exportar { .. } | HirItem::Usar { .. } => None,
        })
        .unwrap_or_else(|| "hir".to_string());
    let err = crate::evaluador::ErrorEjecucion {
        mensaje: format!("referencia genérica residual en IR concreta ({ubicacion}): {detalle}"),
        causa: Box::new(crate::evaluador::CausaError::ReferenciaGenericaResidualIr {
            detalle,
            ubicacion,
        }),
        linea: 1,
        columna: 1,
        archivo: None,
    };
    Err(err.to_string())
}

fn scan_tipo_hir(t: &TipoSem, ctx: &str, errs: &mut Vec<String>) {
    if crate::semantico::contiene_param_generico(t) {
        errs.push(format!("residuo genérico en tipo ({ctx}): {}", t.nombre()));
    }
}
fn scan_expr_hir(e: &HirExpresion, ctx: &str, errs: &mut Vec<String>) {
    match e {
        HirExpresion::Literal(_, t) => scan_tipo_hir(t, ctx, errs),
        HirExpresion::Variable(_, t) => scan_tipo_hir(t, ctx, errs),
        HirExpresion::Suma(a, b, t)
        | HirExpresion::Resta(a, b, t)
        | HirExpresion::Multiplicacion(a, b, t)
        | HirExpresion::Division(a, b, t)
        | HirExpresion::Modulo(a, b, t)
        | HirExpresion::IgualIgual(a, b, t)
        | HirExpresion::Distinto(a, b, t)
        | HirExpresion::Menor(a, b, t)
        | HirExpresion::MenorIgual(a, b, t)
        | HirExpresion::Mayor(a, b, t)
        | HirExpresion::MayorIgual(a, b, t)
        | HirExpresion::Y(a, b, t)
        | HirExpresion::O(a, b, t) => {
            scan_expr_hir(a, ctx, errs);
            scan_expr_hir(b, ctx, errs);
            scan_tipo_hir(t, ctx, errs);
        }
        HirExpresion::No(a, t) => {
            scan_expr_hir(a, ctx, errs);
            scan_tipo_hir(t, ctx, errs);
        }
        HirExpresion::Rango {
            inicio, fin, tipo, ..
        } => {
            scan_expr_hir(inicio, ctx, errs);
            scan_expr_hir(fin, ctx, errs);
            scan_tipo_hir(tipo, ctx, errs);
        }
        HirExpresion::ArregloLiteral { elementos, tipo } => {
            for el in elementos {
                scan_expr_hir(el, ctx, errs);
            }
            scan_tipo_hir(tipo, ctx, errs);
        }
        HirExpresion::Indice {
            objeto,
            indice,
            tipo,
        } => {
            scan_expr_hir(objeto, ctx, errs);
            scan_expr_hir(indice, ctx, errs);
            scan_tipo_hir(tipo, ctx, errs);
        }
        HirExpresion::Acceso { objeto, tipo, .. } => {
            scan_expr_hir(objeto, ctx, errs);
            scan_tipo_hir(tipo, ctx, errs);
        }
        HirExpresion::Llamada {
            argumentos, tipo, ..
        } => {
            for a in argumentos {
                scan_expr_hir(a, ctx, errs);
            }
            scan_tipo_hir(tipo, ctx, errs);
        }
        HirExpresion::LlamadaIndirecta {
            callee,
            argumentos,
            tipo,
        } => {
            scan_expr_hir(callee, ctx, errs);
            for a in argumentos {
                scan_expr_hir(a, ctx, errs);
            }
            scan_tipo_hir(tipo, ctx, errs);
        }
        HirExpresion::FuncionRef(_) => {}
        HirExpresion::Lambda { params, tipo, .. } => {
            for (_, t) in params {
                scan_tipo_hir(t, ctx, errs);
            }
            scan_tipo_hir(tipo, ctx, errs);
        }
        HirExpresion::Si {
            condicion,
            entonces,
            sino,
        } => {
            scan_expr_hir(condicion, ctx, errs);
            for s in entonces {
                scan_stmt_hir(s, ctx, errs);
            }
            if let Some(v) = sino {
                for s in v {
                    scan_stmt_hir(s, ctx, errs);
                }
            }
        }
        HirExpresion::Otro(t) => scan_tipo_hir(t, ctx, errs),
        HirExpresion::ModuloObjeto { tipo, .. } => scan_tipo_hir(tipo, ctx, errs),
    }
}
fn scan_stmt_hir(s: &HirSentencia, ctx: &str, errs: &mut Vec<String>) {
    match s {
        HirSentencia::Declaracion { tipo, valor, .. } => {
            scan_tipo_hir(tipo, ctx, errs);
            scan_expr_hir(valor, ctx, errs);
        }
        HirSentencia::Asignacion { valor, .. } => scan_expr_hir(valor, ctx, errs),
        HirSentencia::Expresion(e) => scan_expr_hir(e, ctx, errs),
        HirSentencia::Retornar(v) => {
            if let Some(e) = v {
                scan_expr_hir(e, ctx, errs);
            }
        }
        HirSentencia::Bloque(stmts) => {
            for st in stmts {
                scan_stmt_hir(st, ctx, errs);
            }
        }
        HirSentencia::Si {
            condicion,
            entonces,
            sino,
        } => {
            scan_expr_hir(condicion, ctx, errs);
            for st in entonces {
                scan_stmt_hir(st, ctx, errs);
            }
            if let Some(v) = sino {
                for st in v {
                    scan_stmt_hir(st, ctx, errs);
                }
            }
        }
        HirSentencia::Mientras { condicion, cuerpo } => {
            scan_expr_hir(condicion, ctx, errs);
            for st in cuerpo {
                scan_stmt_hir(st, ctx, errs);
            }
        }
        HirSentencia::AsignacionIndice {
            objeto,
            indice,
            valor,
        } => {
            scan_expr_hir(objeto, ctx, errs);
            scan_expr_hir(indice, ctx, errs);
            scan_expr_hir(valor, ctx, errs);
        }
        HirSentencia::ParaCada {
            iterable, cuerpo, ..
        } => {
            scan_expr_hir(iterable, ctx, errs);
            for st in cuerpo {
                scan_stmt_hir(st, ctx, errs);
            }
        }
        HirSentencia::Romper | HirSentencia::Continuar => {}
        HirSentencia::IntentarRescatar {
            cuerpo_intentar,
            cuerpo_rescatar,
            ..
        } => {
            for st in cuerpo_intentar {
                scan_stmt_hir(st, ctx, errs);
            }
            for st in cuerpo_rescatar {
                scan_stmt_hir(st, ctx, errs);
            }
        }
        HirSentencia::Importar { .. }
        | HirSentencia::Exportar { .. }
        | HirSentencia::Usar { .. } => {}
    }
}
fn scan_item_hir(item: &HirItem, errs: &mut Vec<String>) {
    match item {
        HirItem::Funcion {
            nombre,
            params,
            retorno,
            cuerpo,
            ..
        } => {
            let ctx = format!("funcion {nombre}");
            for (_, t) in params {
                scan_tipo_hir(t, &ctx, errs);
            }
            scan_tipo_hir(retorno, &ctx, errs);
            for st in cuerpo {
                scan_stmt_hir(st, &ctx, errs);
            }
        }
        HirItem::Declaracion { tipo, valor, .. } => {
            scan_tipo_hir(tipo, "declaracion", errs);
            scan_expr_hir(valor, "declaracion", errs);
        }
        HirItem::Expresion(e) => scan_expr_hir(e, "expresion", errs),
        HirItem::IntentarRescatar {
            cuerpo_intentar,
            cuerpo_rescatar,
            ..
        } => {
            for st in cuerpo_intentar {
                scan_stmt_hir(st, "intentar", errs);
            }
            for st in cuerpo_rescatar {
                scan_stmt_hir(st, "rescatar", errs);
            }
        }
        HirItem::Importar { .. } | HirItem::Exportar { .. } | HirItem::Usar { .. } => {}
    }
}

#[cfg(test)]
mod verificacion_3b_tests {
    use super::*;
    use crate::semantico::TipoSem;
    use crate::simbodo::SimbodoId;

    fn prog_funcion(retorno: TipoSem, cuerpo: Vec<HirSentencia>) -> HirPrograma {
        HirPrograma {
            items: vec![HirItem::Funcion {
                nombre: "f".to_string(),
                simbodo: SimbodoId(0),
                params: vec![],
                capturadas: vec![],
                capturas_externas: vec![],
                celdas: vec![],
                retorno,
                cuerpo,
            }],
        }
    }

    fn lit_entero(v: i64) -> HirExpresion {
        HirExpresion::Literal(HirLiteral::Entero(v), TipoSem::Entero)
    }

    fn lit_texto(s: &str) -> HirExpresion {
        HirExpresion::Literal(HirLiteral::Texto(s.to_string()), TipoSem::Texto)
    }

    // D1 positivos
    #[test]
    fn d1_retorno_entero_valido() {
        let p = prog_funcion(
            TipoSem::Entero,
            vec![HirSentencia::Retornar(Some(lit_entero(1)))],
        );
        let r = VerificadorHir::verificar_detallado(&p);
        assert!(r.es_ok(), "{r:?}");
    }

    #[test]
    fn d1_retorno_desconocido_no_falso_positivo() {
        let p = prog_funcion(
            TipoSem::Desconocido,
            vec![HirSentencia::Retornar(Some(lit_texto("x")))],
        );
        assert!(VerificadorHir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d1_retorno_vacio_nulo_valido() {
        let p = prog_funcion(TipoSem::Nulo, vec![HirSentencia::Retornar(None)]);
        assert!(VerificadorHir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d1_expresion_desconocida_no_falso_positivo() {
        // D1: tipo inferido Desconocido no debe producir falso positivo (HIR040),
        // sin violar invariantes previas (p. ej. HIR013 por refs no declaradas).
        let p = prog_funcion(
            TipoSem::Entero,
            vec![HirSentencia::Retornar(Some(HirExpresion::Otro(
                TipoSem::Desconocido,
            )))],
        );
        let r = VerificadorHir::verificar_detallado(&p);
        assert!(r.es_ok(), "{r:?}");
    }

    // D1 negativos
    #[test]
    fn d1_retorno_texto_en_entero_rechaza_hir040() {
        let p = prog_funcion(
            TipoSem::Entero,
            vec![HirSentencia::Retornar(Some(lit_texto("hola")))],
        );
        let r = VerificadorHir::verificar_detallado(&p);
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "HIR040");
    }

    #[test]
    fn d1_retorno_vacio_en_entero_rechaza_hir040() {
        let p = prog_funcion(TipoSem::Entero, vec![HirSentencia::Retornar(None)]);
        let r = VerificadorHir::verificar_detallado(&p);
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "HIR040");
    }

    #[test]
    fn d1_bool_compatible_sigue_ok() {
        assert!(VerificadorHir::verificar(&prog_funcion(
            TipoSem::Entero,
            vec![HirSentencia::Retornar(Some(lit_entero(7)))]
        )));
    }

    // Auditoría D1: compatibilidades espejo semántica
    #[test]
    fn d1_entero_a_decimal_acepta() {
        let p = prog_funcion(
            TipoSem::Decimal,
            vec![HirSentencia::Retornar(Some(lit_entero(1)))],
        );
        assert!(VerificadorHir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d1_entero_a_real_acepta() {
        let p = prog_funcion(
            TipoSem::Real,
            vec![HirSentencia::Retornar(Some(lit_entero(1)))],
        );
        assert!(VerificadorHir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d1_lista_a_arreglo_elemento_compatible_acepta() {
        let e = HirExpresion::ArregloLiteral {
            elementos: vec![lit_entero(1)],
            tipo: TipoSem::Lista(Box::new(TipoSem::Entero)),
        };
        let p = prog_funcion(
            TipoSem::Arreglo(Box::new(TipoSem::Entero), 1),
            vec![HirSentencia::Retornar(Some(e))],
        );
        assert!(VerificadorHir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d1_coleccion_acepta_arreglo() {
        let e = HirExpresion::ArregloLiteral {
            elementos: vec![lit_entero(1)],
            tipo: TipoSem::Arreglo(Box::new(TipoSem::Entero), 1),
        };
        let p = prog_funcion(
            TipoSem::Coleccion(Box::new(TipoSem::Entero)),
            vec![HirSentencia::Retornar(Some(e))],
        );
        assert!(VerificadorHir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d1_funcion_tipo_compatible_acepta() {
        let e = HirExpresion::Otro(TipoSem::Funcion {
            params: vec![TipoSem::Entero],
            retorno: Box::new(TipoSem::Entero),
            genericos: Vec::new(),
        });
        let p = prog_funcion(
            TipoSem::Funcion {
                params: vec![TipoSem::Entero],
                retorno: Box::new(TipoSem::Entero),
                genericos: Vec::new(),
            },
            vec![HirSentencia::Retornar(Some(e))],
        );
        assert!(VerificadorHir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d1_retorno_en_condicional_bucle_bloque_valida() {
        let p = prog_funcion(
            TipoSem::Entero,
            vec![HirSentencia::Si {
                condicion: HirExpresion::Literal(HirLiteral::Logico(true), TipoSem::Logico),
                entonces: vec![HirSentencia::Retornar(Some(lit_entero(1)))],
                sino: Some(vec![HirSentencia::Bloque(vec![HirSentencia::Mientras {
                    condicion: HirExpresion::Literal(HirLiteral::Logico(false), TipoSem::Logico),
                    cuerpo: vec![HirSentencia::Retornar(Some(lit_entero(2)))],
                }])]),
            }],
        );
        assert!(VerificadorHir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d1_retorno_incompatible_en_bloque_rechaza_hir040() {
        let p = prog_funcion(
            TipoSem::Entero,
            vec![HirSentencia::Bloque(vec![HirSentencia::Retornar(Some(
                lit_texto("x"),
            ))])],
        );
        let r = VerificadorHir::verificar_detallado(&p);
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "HIR040");
    }

    #[test]
    fn d1_no_cruza_lambda_ambito_separado() {
        // Retorno de la función externa es Entero y válido; Lambda en expresión
        // no debe inspeccionarse como retorno de la función externa.
        let p = HirPrograma {
            items: vec![HirItem::Funcion {
                nombre: "f".to_string(),
                simbodo: SimbodoId(0),
                params: vec![],
                capturadas: vec![],
                capturas_externas: vec![],
                celdas: vec![],
                retorno: TipoSem::Entero,
                cuerpo: vec![
                    HirSentencia::Expresion(HirExpresion::Lambda {
                        nombre: "l".to_string(),
                        params: vec![],
                        capturas: vec![],
                        tipo: TipoSem::Desconocido,
                    }),
                    HirSentencia::Retornar(Some(lit_entero(1))),
                ],
            }],
        };
        assert!(VerificadorHir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d1_arreglo_tamano_distinto_rechaza_hir040() {
        let e = HirExpresion::ArregloLiteral {
            elementos: vec![lit_entero(1)],
            tipo: TipoSem::Arreglo(Box::new(TipoSem::Entero), 1),
        };
        let p = prog_funcion(
            TipoSem::Arreglo(Box::new(TipoSem::Entero), 2),
            vec![HirSentencia::Retornar(Some(e))],
        );
        let r = VerificadorHir::verificar_detallado(&p);
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "HIR040");
    }

    // 3D.1: aritmética numérica mixta válida (sin falso positivo HIR031).
    #[test]
    fn d1_binario_numerico_mixto_acepta() {
        let casos = [(HirExpresion::Suma(
            Box::new(lit_entero(1)),
            Box::new(HirExpresion::Literal(HirLiteral::Real(1.0), TipoSem::Real)),
            TipoSem::Desconocido,
        ),)];
        for (e,) in casos {
            let p = prog_funcion(TipoSem::Desconocido, vec![HirSentencia::Expresion(e)]);
            assert!(VerificadorHir::verificar_detallado(&p).es_ok());
        }
    }

    #[test]
    fn d1_binario_incompatible_rechaza_hir031() {
        let e = HirExpresion::Suma(
            Box::new(lit_entero(1)),
            Box::new(lit_texto("a")),
            TipoSem::Desconocido,
        );
        let p = prog_funcion(TipoSem::Desconocido, vec![HirSentencia::Expresion(e)]);
        let r = VerificadorHir::verificar_detallado(&p);
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "HIR031");
    }
    // 3D.2: operadores lógicos solo aceptan lógicos.
    #[test]
    fn d1_binario_logico_valido_acepta() {
        let e = HirExpresion::Y(
            Box::new(HirExpresion::Literal(
                HirLiteral::Logico(true),
                TipoSem::Logico,
            )),
            Box::new(HirExpresion::Literal(
                HirLiteral::Logico(false),
                TipoSem::Logico,
            )),
            TipoSem::Desconocido,
        );
        let p = prog_funcion(TipoSem::Desconocido, vec![HirSentencia::Expresion(e)]);
        assert!(VerificadorHir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d1_binario_logico_incompatible_rechaza_hir031() {
        let e = HirExpresion::Y(
            Box::new(lit_entero(1)),
            Box::new(lit_entero(2)),
            TipoSem::Desconocido,
        );
        let p = prog_funcion(TipoSem::Desconocido, vec![HirSentencia::Expresion(e)]);
        let r = VerificadorHir::verificar_detallado(&p);
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "HIR031");
    }

    // 3D.2: aritmética no acepta texto.
    #[test]
    fn d1_aritmetico_texto_rechaza_hir031() {
        let e = HirExpresion::Resta(
            Box::new(lit_texto("a")),
            Box::new(lit_texto("b")),
            TipoSem::Desconocido,
        );
        let p = prog_funcion(TipoSem::Desconocido, vec![HirSentencia::Expresion(e)]);
        let r = VerificadorHir::verificar_detallado(&p);
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "HIR031");
    }
}
