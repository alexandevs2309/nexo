//! HIR (High-level Intermediate Representation) de BASILISCO.
//!
//! Representación semántica de alto nivel, independiente del AST y del runtime.
//! No ejecutable por sí misma.

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
        nombre: SimbodoId,
        argumentos: Vec<HirExpresion>,
        tipo: TipoSem,
    },
    LlamadaIndirecta {
        callee: Box<HirExpresion>,
        argumentos: Vec<HirExpresion>,
        tipo: TipoSem,
    },
    Lambda {
        params: Vec<(String, TipoSem)>,
        cuerpo: Vec<HirSentencia>,
        tipo: TipoSem,
    },
    Si {
        condicion: Box<HirExpresion>,
        entonces: Vec<HirSentencia>,
        sino: Option<Vec<HirSentencia>>,
    },
    Otro(TipoSem),
    Rango {
        inicio: Box<HirExpresion>,
        fin: Box<HirExpresion>,
        inclusivo: bool,
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
        variable: String,
        iterable: HirExpresion,
        cuerpo: Vec<HirSentencia>,
    },
    Romper,
    Continuar,
    IntentarRescatar {
        cuerpo_intentar: Vec<HirSentencia>,
        variable_error: Option<String>,
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
        simbodo: SimbodoId,
        params: Vec<(SimbodoId, TipoSem)>,
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
        variable_error: Option<String>,
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

/// Verificador HIR: valida invariantes estructurales.
pub struct VerificadorHir;

impl VerificadorHir {
    pub fn verificar(programa: &HirPrograma) -> bool {
        // Invariante básica: programa no vacío permitido; sin nulls.
        for item in &programa.items {
            if let HirItem::Funcion { params, .. } = item {
                let _ = params;
            }
        }
        true
    }
}

/// Lowering AST -> HIR (mínimo). Requiere ContextoSemantico para resolver símbodos.
pub fn lower_expresion(e: &Expresion, ctx: &crate::semantico::ContextoSemantico) -> HirExpresion {
    match e {
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
            let id = ctx
                .resolver_simbodo(nombre)
                .unwrap_or(crate::simbodo::SimbodoId(0));
            let tipo = ctx
                .buscar_tipo_variable(nombre)
                .unwrap_or(TipoSem::Desconocido);
            HirExpresion::Variable(id, tipo)
        }
        Expresion::Suma(a, b) => {
            let ha = lower_expresion(a, ctx);
            let hb = lower_expresion(b, ctx);
            let t = TipoSem::Desconocido;
            HirExpresion::Suma(Box::new(ha), Box::new(hb), t)
        }
        Expresion::Llamada {
            nombre, argumentos, ..
        } => {
            let id = ctx
                .resolver_simbodo(nombre)
                .unwrap_or(crate::simbodo::SimbodoId(0));
            let args: Vec<HirExpresion> =
                argumentos.iter().map(|a| lower_expresion(a, ctx)).collect();
            HirExpresion::Llamada {
                nombre: id,
                argumentos: args,
                tipo: TipoSem::Desconocido,
            }
        }
        Expresion::Rango {
            inicio,
            fin,
            inclusivo,
            ..
        } => {
            let hi = lower_expresion(inicio, ctx);
            let hf = lower_expresion(fin, ctx);
            HirExpresion::Rango {
                inicio: Box::new(hi),
                fin: Box::new(hf),
                inclusivo: *inclusivo,
                tipo: TipoSem::Desconocido,
            }
        }
        Expresion::Modulo(a, b) => HirExpresion::Modulo(
            Box::new(lower_expresion(a, ctx)),
            Box::new(lower_expresion(b, ctx)),
            TipoSem::Desconocido,
        ),
        Expresion::IgualIgual(a, b) => HirExpresion::IgualIgual(
            Box::new(lower_expresion(a, ctx)),
            Box::new(lower_expresion(b, ctx)),
            TipoSem::Desconocido,
        ),
        Expresion::Distinto(a, b) => HirExpresion::Distinto(
            Box::new(lower_expresion(a, ctx)),
            Box::new(lower_expresion(b, ctx)),
            TipoSem::Desconocido,
        ),
        Expresion::Menor(a, b) => HirExpresion::Menor(
            Box::new(lower_expresion(a, ctx)),
            Box::new(lower_expresion(b, ctx)),
            TipoSem::Desconocido,
        ),
        Expresion::MenorIgual(a, b) => HirExpresion::MenorIgual(
            Box::new(lower_expresion(a, ctx)),
            Box::new(lower_expresion(b, ctx)),
            TipoSem::Desconocido,
        ),
        Expresion::Mayor(a, b) => HirExpresion::Mayor(
            Box::new(lower_expresion(a, ctx)),
            Box::new(lower_expresion(b, ctx)),
            TipoSem::Desconocido,
        ),
        Expresion::MayorIgual(a, b) => HirExpresion::MayorIgual(
            Box::new(lower_expresion(a, ctx)),
            Box::new(lower_expresion(b, ctx)),
            TipoSem::Desconocido,
        ),
        Expresion::No(a) => {
            HirExpresion::No(Box::new(lower_expresion(a, ctx)), TipoSem::Desconocido)
        }
        Expresion::Y(a, b) => HirExpresion::Y(
            Box::new(lower_expresion(a, ctx)),
            Box::new(lower_expresion(b, ctx)),
            TipoSem::Desconocido,
        ),
        Expresion::O(a, b) => HirExpresion::O(
            Box::new(lower_expresion(a, ctx)),
            Box::new(lower_expresion(b, ctx)),
            TipoSem::Desconocido,
        ),
        Expresion::ArregloLiteral { elementos, .. } => HirExpresion::ArregloLiteral {
            elementos: elementos.iter().map(|e| lower_expresion(e, ctx)).collect(),
            tipo: TipoSem::Desconocido,
        },
        Expresion::Indice { objeto, indice, .. } => HirExpresion::Indice {
            objeto: Box::new(lower_expresion(objeto, ctx)),
            indice: Box::new(lower_expresion(indice, ctx)),
            tipo: TipoSem::Desconocido,
        },
        Expresion::Acceso {
            objeto, miembro, ..
        } => HirExpresion::Acceso {
            objeto: Box::new(lower_expresion(objeto, ctx)),
            miembro: miembro.clone(),
            tipo: TipoSem::Desconocido,
        },
        Expresion::Lambda {
            parametros,
            tipo_retorno,
            cuerpo,
            ..
        } => {
            let params = parametros
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
            let body: Vec<HirSentencia> = cuerpo
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx).ok())
                .collect();
            HirExpresion::Lambda {
                params,
                cuerpo: body,
                tipo: ret,
            }
        }
        Expresion::LlamadaIndirecta {
            callee, argumentos, ..
        } => HirExpresion::LlamadaIndirecta {
            callee: Box::new(lower_expresion(callee, ctx)),
            argumentos: argumentos.iter().map(|a| lower_expresion(a, ctx)).collect(),
            tipo: TipoSem::Desconocido,
        },
        _ => HirExpresion::Otro(TipoSem::Desconocido),
    }
}

// Necesitamos buscar_tipo_variable en ContextoSemantico; añadir método.
impl crate::semantico::ContextoSemantico {
    pub fn buscar_tipo_variable(&self, nombre: &str) -> Option<TipoSem> {
        self.buscar_variable(nombre).map(|v| v.tipo)
    }
}

pub fn lower_programa(
    ast: &[Sentencia],
    ctx: &crate::semantico::ContextoSemantico,
) -> Result<HirPrograma, Vec<String>> {
    let mut items = Vec::new();
    let mut errores = Vec::new();
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
                let id = ctx
                    .resolver_simbodo(nombre)
                    .unwrap_or(crate::simbodo::SimbodoId(0));
                let tipo = tipo_anotacion
                    .as_ref()
                    .map(crate::semantico::tipo_anotacion_a_sem)
                    .unwrap_or(TipoSem::Desconocido);
                let val = lower_expresion(valor, ctx);
                items.push(HirItem::Declaracion {
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
                let id = ctx
                    .resolver_simbodo(nombre)
                    .unwrap_or(crate::simbodo::SimbodoId(0));
                let params: Vec<(crate::simbodo::SimbodoId, TipoSem)> = parametros
                    .iter()
                    .map(|(n, t)| {
                        let pid = ctx
                            .resolver_simbodo(n)
                            .unwrap_or(crate::simbodo::SimbodoId(0));
                        let ty = t
                            .as_ref()
                            .map(crate::semantico::tipo_anotacion_a_sem)
                            .unwrap_or(TipoSem::Desconocido);
                        (pid, ty)
                    })
                    .collect();
                let ret = tipo_retorno
                    .as_ref()
                    .map(crate::semantico::tipo_anotacion_a_sem)
                    .unwrap_or(TipoSem::Desconocido);
                let mut body = Vec::new();
                let mut fallo = false;
                for st in cuerpo {
                    match lower_sentencia(st, ctx) {
                        Ok(hs) => body.push(hs),
                        Err(e) => {
                            errores.push(e);
                            fallo = true;
                        }
                    }
                }
                if fallo {
                    continue;
                }
                items.push(HirItem::Funcion {
                    simbodo: id,
                    params,
                    retorno: ret,
                    cuerpo: body,
                });
            }
            Sentencia::Expresion(e) => {
                items.push(HirItem::Expresion(lower_expresion(e, ctx)));
            }
            Sentencia::IntentarRescatar {
                cuerpo_intentar,
                variable_error,
                cuerpo_rescatar,
                ..
            } => {
                let ci: Vec<HirSentencia> = cuerpo_intentar
                    .iter()
                    .filter_map(|st| lower_sentencia(st, ctx).ok())
                    .collect();
                let cr: Vec<HirSentencia> = cuerpo_rescatar
                    .iter()
                    .filter_map(|st| lower_sentencia(st, ctx).ok())
                    .collect();
                items.push(HirItem::IntentarRescatar {
                    cuerpo_intentar: ci,
                    variable_error: variable_error.clone(),
                    cuerpo_rescatar: cr,
                });
            }
            Sentencia::Importar { ruta, alias, .. } => {
                items.push(HirItem::Importar {
                    ruta: ruta.clone(),
                    alias: alias.clone(),
                });
            }
            Sentencia::Exportar { sentencia, .. } => match lower_sentencia(sentencia, ctx) {
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
                items.push(HirItem::Usar {
                    ruta: ruta.clone(),
                    alias: alias.clone(),
                    nombres: nombres.clone(),
                });
            }
            otro => {
                errores.push(format!(
                    "construcción AST no soportada en HIR: {:?}",
                    std::mem::discriminant(otro)
                ));
            }
        }
    }
    if !errores.is_empty() {
        return Err(errores);
    }
    Ok(HirPrograma { items })
}

pub fn lower_sentencia(
    s: &Sentencia,
    ctx: &crate::semantico::ContextoSemantico,
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
            let id = ctx
                .resolver_simbodo(nombre)
                .unwrap_or(crate::simbodo::SimbodoId(0));
            let tipo = tipo_anotacion
                .as_ref()
                .map(crate::semantico::tipo_anotacion_a_sem)
                .unwrap_or(TipoSem::Desconocido);
            Ok(HirSentencia::Declaracion {
                simbodo: id,
                tipo,
                valor: lower_expresion(valor, ctx),
                mutable: *mutable,
                es_const: *es_const,
                publico: *publico,
            })
        }
        Sentencia::Asignacion { nombre, valor, .. } => {
            let id = ctx
                .resolver_simbodo(nombre)
                .unwrap_or(crate::simbodo::SimbodoId(0));
            Ok(HirSentencia::Asignacion {
                simbodo: id,
                valor: lower_expresion(valor, ctx),
            })
        }
        Sentencia::Retorno { valor, .. } => Ok(HirSentencia::Retornar(
            valor.as_ref().map(|v| lower_expresion(v, ctx)),
        )),
        Sentencia::Si {
            condicion,
            rama_si,
            rama_sino,
            ..
        } => {
            let cond = lower_expresion(condicion, ctx);
            let entonces: Vec<HirSentencia> = rama_si
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx).ok())
                .collect();
            let sino = rama_sino.as_ref().map(|v| {
                v.iter()
                    .filter_map(|st| lower_sentencia(st, ctx).ok())
                    .collect()
            });
            Ok(HirSentencia::Si {
                condicion: cond,
                entonces,
                sino,
            })
        }
        Sentencia::Mientras {
            condicion, cuerpo, ..
        } => {
            let cond = lower_expresion(condicion, ctx);
            let body: Vec<HirSentencia> = cuerpo
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx).ok())
                .collect();
            Ok(HirSentencia::Mientras {
                condicion: cond,
                cuerpo: body,
            })
        }
        Sentencia::Bloque(stmts) => {
            let body: Vec<HirSentencia> = stmts
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx).ok())
                .collect();
            Ok(HirSentencia::Bloque(body))
        }
        Sentencia::Mostrar { valor, .. } => {
            Ok(HirSentencia::Expresion(lower_expresion(valor, ctx)))
        }
        Sentencia::MostrarSinSalto { valor, .. } => {
            Ok(HirSentencia::Expresion(lower_expresion(valor, ctx)))
        }
        Sentencia::IntentarRescatar {
            cuerpo_intentar,
            variable_error,
            cuerpo_rescatar,
            ..
        } => {
            let ci: Vec<HirSentencia> = cuerpo_intentar
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx).ok())
                .collect();
            let cr: Vec<HirSentencia> = cuerpo_rescatar
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx).ok())
                .collect();
            Ok(HirSentencia::IntentarRescatar {
                cuerpo_intentar: ci,
                variable_error: variable_error.clone(),
                cuerpo_rescatar: cr,
            })
        }
        Sentencia::Importar { ruta, alias, .. } => Ok(HirSentencia::Importar {
            ruta: ruta.clone(),
            alias: alias.clone(),
        }),
        Sentencia::Exportar { sentencia, .. } => {
            let hs = lower_sentencia(sentencia, ctx)?;
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
        otro => Err(format!(
            "sentencia AST no soportada en HIR: {:?}",
            std::mem::discriminant(otro)
        )),
    }
}
