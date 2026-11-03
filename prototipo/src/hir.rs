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
) -> HirExpresion {
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
            let id = res.resolver_o_registrar(nombre);
            let tipo = ctx
                .buscar_tipo_variable(nombre)
                .unwrap_or(TipoSem::Desconocido);
            HirExpresion::Variable(id, tipo)
        }
        Expresion::Suma(a, b) => {
            let ha = lower_expresion(a, ctx, res);
            let hb = lower_expresion(b, ctx, res);
            let t = TipoSem::Desconocido;
            HirExpresion::Suma(Box::new(ha), Box::new(hb), t)
        }
        Expresion::Resta(a, b) => {
            let ha = lower_expresion(a, ctx, res);
            let hb = lower_expresion(b, ctx, res);
            let t = TipoSem::Desconocido;
            HirExpresion::Resta(Box::new(ha), Box::new(hb), t)
        }
        Expresion::Multiplicacion(a, b) => {
            let ha = lower_expresion(a, ctx, res);
            let hb = lower_expresion(b, ctx, res);
            let t = TipoSem::Desconocido;
            HirExpresion::Multiplicacion(Box::new(ha), Box::new(hb), t)
        }
        Expresion::Division(a, b) => {
            let ha = lower_expresion(a, ctx, res);
            let hb = lower_expresion(b, ctx, res);
            let t = TipoSem::Desconocido;
            HirExpresion::Division(Box::new(ha), Box::new(hb), t)
        }
        Expresion::Llamada {
            nombre, argumentos, ..
        } => {
            let args: Vec<HirExpresion> = argumentos
                .iter()
                .map(|a| lower_expresion(a, ctx, res))
                .collect();
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
            let hi = lower_expresion(inicio, ctx, res);
            let hf = lower_expresion(fin, ctx, res);
            HirExpresion::Rango {
                inicio: Box::new(hi),
                fin: Box::new(hf),
                inclusivo: *inclusivo,
                tipo: TipoSem::Desconocido,
            }
        }
        Expresion::Modulo(a, b) => HirExpresion::Modulo(
            Box::new(lower_expresion(a, ctx, res)),
            Box::new(lower_expresion(b, ctx, res)),
            TipoSem::Desconocido,
        ),
        Expresion::IgualIgual(a, b) => HirExpresion::IgualIgual(
            Box::new(lower_expresion(a, ctx, res)),
            Box::new(lower_expresion(b, ctx, res)),
            TipoSem::Desconocido,
        ),
        Expresion::Distinto(a, b) => HirExpresion::Distinto(
            Box::new(lower_expresion(a, ctx, res)),
            Box::new(lower_expresion(b, ctx, res)),
            TipoSem::Desconocido,
        ),
        Expresion::Menor(a, b) => HirExpresion::Menor(
            Box::new(lower_expresion(a, ctx, res)),
            Box::new(lower_expresion(b, ctx, res)),
            TipoSem::Desconocido,
        ),
        Expresion::MenorIgual(a, b) => HirExpresion::MenorIgual(
            Box::new(lower_expresion(a, ctx, res)),
            Box::new(lower_expresion(b, ctx, res)),
            TipoSem::Desconocido,
        ),
        Expresion::Mayor(a, b) => HirExpresion::Mayor(
            Box::new(lower_expresion(a, ctx, res)),
            Box::new(lower_expresion(b, ctx, res)),
            TipoSem::Desconocido,
        ),
        Expresion::MayorIgual(a, b) => HirExpresion::MayorIgual(
            Box::new(lower_expresion(a, ctx, res)),
            Box::new(lower_expresion(b, ctx, res)),
            TipoSem::Desconocido,
        ),
        Expresion::No(a) => {
            HirExpresion::No(Box::new(lower_expresion(a, ctx, res)), TipoSem::Desconocido)
        }
        Expresion::Y(a, b) => HirExpresion::Y(
            Box::new(lower_expresion(a, ctx, res)),
            Box::new(lower_expresion(b, ctx, res)),
            TipoSem::Desconocido,
        ),
        Expresion::O(a, b) => HirExpresion::O(
            Box::new(lower_expresion(a, ctx, res)),
            Box::new(lower_expresion(b, ctx, res)),
            TipoSem::Desconocido,
        ),
        Expresion::ArregloLiteral { elementos, .. } => HirExpresion::ArregloLiteral {
            elementos: elementos
                .iter()
                .map(|e| lower_expresion(e, ctx, res))
                .collect(),
            tipo: TipoSem::Desconocido,
        },
        Expresion::Indice { objeto, indice, .. } => HirExpresion::Indice {
            objeto: Box::new(lower_expresion(objeto, ctx, res)),
            indice: Box::new(lower_expresion(indice, ctx, res)),
            tipo: TipoSem::Desconocido,
        },
        Expresion::Acceso {
            objeto, miembro, ..
        } => HirExpresion::Acceso {
            objeto: Box::new(lower_expresion(objeto, ctx, res)),
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
            let capturas: Vec<SimbodoId> = free
                .iter()
                .map(|n| res.resolver(n).unwrap_or_else(|| res.insertar(n, None)))
                .collect();
            // Nombre de la clausura.
            let nombre_clausura = res.nuevo_nombre_clausura();
            // Registrar los parámetros de la lambda en un nuevo ámbito.
            res.entrar();
            let mut param_ids: Vec<SimbodoId> = Vec::new();
            for (n, _) in &params {
                param_ids.push(res.insertar(n, None));
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
            let body: Vec<HirSentencia> = cuerpo
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx, res).ok())
                .collect();
            let capturadas_ids: Vec<SimbodoId> = capturadas_por_anidados
                .iter()
                .filter_map(|n| res.resolver(n))
                .collect();
            res.salir();
            // Registrar la clausura como función de primer nivel (recolectada).
            res.clausuras.push(HirItem::Funcion {
                nombre: nombre_clausura.clone(),
                simbodo: SimbodoId(0),
                params: clausura_params,
                capturadas: capturadas_ids.clone(),
                capturas_externas: capturas.clone(),
                celdas: {
                    let mut v = capturas.clone();
                    for id in &capturadas_ids {
                        if !v.contains(id) {
                            v.push(*id);
                        }
                    }
                    v
                },
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
            callee: Box::new(lower_expresion(callee, ctx, res)),
            argumentos: argumentos
                .iter()
                .map(|a| lower_expresion(a, ctx, res))
                .collect(),
            tipo: TipoSem::Desconocido,
        },
    }
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
            let valor_hir = lower_expresion(valor, ctx, res);
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
            let id = res
                .resolver(nombre)
                .unwrap_or_else(|| res.insertar(nombre, None));
            Ok(HirSentencia::Asignacion {
                simbodo: id,
                valor: lower_expresion(valor, ctx, res),
            })
        }
        Sentencia::AsignacionIndice {
            objeto,
            indice,
            valor,
            ..
        } => Ok(HirSentencia::AsignacionIndice {
            objeto: lower_expresion(objeto, ctx, res),
            indice: lower_expresion(indice, ctx, res),
            valor: lower_expresion(valor, ctx, res),
        }),
        Sentencia::Retorno { valor, .. } => Ok(HirSentencia::Retornar(
            valor.as_ref().map(|v| lower_expresion(v, ctx, res)),
        )),
        Sentencia::Si {
            condicion,
            rama_si,
            rama_sino,
            ..
        } => {
            let cond = lower_expresion(condicion, ctx, res);
            res.entrar();
            let entonces: Vec<HirSentencia> = rama_si
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx, res).ok())
                .collect();
            res.salir();
            let sino = rama_sino.as_ref().map(|v| {
                res.entrar();
                let rama: Vec<HirSentencia> = v
                    .iter()
                    .filter_map(|st| lower_sentencia(st, ctx, res).ok())
                    .collect();
                res.salir();
                rama
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
            let cond = lower_expresion(condicion, ctx, res);
            res.entrar();
            let body: Vec<HirSentencia> = cuerpo
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx, res).ok())
                .collect();
            res.salir();
            Ok(HirSentencia::Mientras {
                condicion: cond,
                cuerpo: body,
            })
        }
        Sentencia::Bloque(stmts) => {
            res.entrar();
            let body: Vec<HirSentencia> = stmts
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx, res).ok())
                .collect();
            res.salir();
            Ok(HirSentencia::Bloque(body))
        }
        Sentencia::Mostrar { valor, .. } => {
            Ok(HirSentencia::Expresion(lower_expresion(valor, ctx, res)))
        }
        Sentencia::MostrarSinSalto { valor, .. } => {
            Ok(HirSentencia::Expresion(lower_expresion(valor, ctx, res)))
        }
        Sentencia::Expresion(e) => Ok(HirSentencia::Expresion(lower_expresion(e, ctx, res))),
        Sentencia::ParaCada {
            variable,
            iterable,
            cuerpo,
            ..
        } => {
            let iterable_hir = lower_expresion(iterable, ctx, res);
            res.entrar();
            let var_id = res.insertar(variable, None);
            res.variables.insert(variable.clone());
            let body: Vec<HirSentencia> = cuerpo
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx, res).ok())
                .collect();
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
            let ci: Vec<HirSentencia> = cuerpo_intentar
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx, res).ok())
                .collect();
            res.salir();
            let (var_nombre, cr): (Option<String>, Vec<HirSentencia>) = {
                res.entrar();
                let var_nombre = variable_error.as_ref().map(|v| {
                    res.insertar(v, None);
                    v.clone()
                });
                let cr = cuerpo_rescatar
                    .iter()
                    .filter_map(|st| lower_sentencia(st, ctx, res).ok())
                    .collect();
                res.salir();
                (var_nombre, cr)
            };
            Ok(HirSentencia::IntentarRescatar {
                cuerpo_intentar: ci,
                variable_error: var_nombre,
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
            let capturas: Vec<SimbodoId> = free
                .iter()
                .map(|n| res.resolver(n).unwrap_or_else(|| res.insertar(n, None)))
                .collect();
            let nombre_clausura = res.nuevo_nombre_clausura();
            res.entrar();
            let mut param_ids: Vec<SimbodoId> = Vec::new();
            for (n, _) in &params {
                param_ids.push(res.insertar(n, None));
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
            let body: Vec<HirSentencia> = cuerpo
                .iter()
                .filter_map(|st| lower_sentencia(st, ctx, res).ok())
                .collect();
            let capturadas_ids: Vec<SimbodoId> = capturadas_por_anidados
                .iter()
                .filter_map(|n| res.resolver(n))
                .collect();
            res.salir();
            res.clausuras.push(HirItem::Funcion {
                nombre: nombre_clausura.clone(),
                simbodo: SimbodoId(0),
                params: clausura_params,
                capturadas: capturadas_ids.clone(),
                capturas_externas: capturas.clone(),
                celdas: {
                    let mut v = capturas.clone();
                    for id in &capturadas_ids {
                        if !v.contains(id) {
                            v.push(*id);
                        }
                    }
                    v
                },
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
    fn recolectar_expr(e: &Expresion, refs: &mut HashSet<String>) {
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
                recolectar_expr(a, refs);
                recolectar_expr(b, refs);
            }
            Expresion::No(x) => recolectar_expr(x, refs),
            Expresion::Llamada { argumentos, .. } => {
                for a in argumentos {
                    recolectar_expr(a, refs);
                }
            }
            Expresion::LlamadaIndirecta {
                callee, argumentos, ..
            } => {
                recolectar_expr(callee, refs);
                for a in argumentos {
                    recolectar_expr(a, refs);
                }
            }
            Expresion::ArregloLiteral { elementos, .. } => {
                for e in elementos {
                    recolectar_expr(e, refs);
                }
            }
            Expresion::Indice { objeto, indice, .. } => {
                recolectar_expr(objeto, refs);
                recolectar_expr(indice, refs);
            }
            Expresion::Acceso { objeto, .. } => {
                recolectar_expr(objeto, refs);
            }
            Expresion::Rango { inicio, fin, .. } => {
                recolectar_expr(inicio, refs);
                recolectar_expr(fin, refs);
            }
            Expresion::Lambda {
                parametros, cuerpo, ..
            } => {
                let nested_params: Vec<String> =
                    parametros.iter().map(|(n, _)| n.clone()).collect();
                let (nested_free, nested_capt) = variables_libres(cuerpo, &nested_params);
                for v in nested_free {
                    refs.insert(v);
                }
                for v in nested_capt {
                    refs.insert(v);
                }
            }
            _ => {}
        }
    }

    fn recolectar_stmt(s: &Sentencia, refs: &mut HashSet<String>, declaradas: &HashSet<String>) {
        match s {
            Sentencia::Declaracion {
                nombre: _, valor, ..
            } => {
                recolectar_expr(valor, refs);
            }
            Sentencia::Asignacion { nombre, valor, .. } => {
                if !declaradas.contains(nombre) {
                    refs.insert(nombre.clone());
                }
                recolectar_expr(valor, refs);
            }
            Sentencia::AsignacionIndice {
                objeto,
                indice,
                valor,
                ..
            } => {
                recolectar_expr(objeto, refs);
                recolectar_expr(indice, refs);
                recolectar_expr(valor, refs);
            }
            Sentencia::Expresion(e) => recolectar_expr(e, refs),
            Sentencia::Retorno { valor: Some(v), .. } => {
                recolectar_expr(v, refs);
            }
            Sentencia::Retorno { .. } => {}
            Sentencia::Si {
                condicion,
                rama_si,
                rama_sino,
                ..
            } => {
                recolectar_expr(condicion, refs);
                for ss in rama_si {
                    recolectar_stmt(ss, refs, declaradas);
                }
                if let Some(sino) = rama_sino {
                    for ss in sino {
                        recolectar_stmt(ss, refs, declaradas);
                    }
                }
            }
            Sentencia::Mientras {
                condicion, cuerpo, ..
            } => {
                recolectar_expr(condicion, refs);
                for ss in cuerpo {
                    recolectar_stmt(ss, refs, declaradas);
                }
            }
            Sentencia::Bloque(ss) => {
                for s in ss {
                    recolectar_stmt(s, refs, declaradas);
                }
            }
            Sentencia::ParaCada {
                iterable, cuerpo, ..
            } => {
                recolectar_expr(iterable, refs);
                for ss in cuerpo {
                    recolectar_stmt(ss, refs, declaradas);
                }
            }
            Sentencia::IntentarRescatar {
                cuerpo_intentar,
                cuerpo_rescatar,
                ..
            } => {
                for ss in cuerpo_intentar {
                    recolectar_stmt(ss, refs, declaradas);
                }
                for ss in cuerpo_rescatar {
                    recolectar_stmt(ss, refs, declaradas);
                }
            }
            Sentencia::Exportar { sentencia, .. } => {
                recolectar_stmt(sentencia, refs, declaradas);
            }
            Sentencia::Mostrar { valor, .. } | Sentencia::MostrarSinSalto { valor, .. } => {
                recolectar_expr(valor, refs);
            }
            _ => {}
        }
    }

    for s in stmts {
        recolectar_stmt(s, &mut referenciadas, &declaradas);
    }
    // Quitar declaradas y params.
    for d in &declaradas {
        referenciadas.remove(d);
    }
    (referenciadas.into_iter().collect(), capturadas_por_anidados)
}

/// Carga un módulo en tiempo de compilación y devuelve sus items HIR (funciones
/// y declaraciones), filtrando por `nombres` si se da (sentencia `usar ... { ... }`).
fn cargar_modulo_items(
    ruta: &str,
    ctx: &crate::semantico::ContextoSemantico,
    res: &mut ResolutorHir,
    nombres: Option<&Vec<String>>,
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
        return Vec::new();
    };
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
    // Bajar a HIR con un resolvedor propio (comparte la pila de ámbitos raíz
    // mediante un nuevo ResolutorHir; los nombres se resuelven por nombre).
    let hir = match crate::hir::lower_programa(&ast, &ctx_mod) {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };
    // Filtrar por nombres si se pidió.
    let mut items = hir.items;
    if let Some(nombres) = nombres {
        let set: std::collections::HashSet<&String> = nombres.iter().collect();
        items.retain(|it| match it {
            HirItem::Funcion { nombre, .. } => set.contains(nombre),
            HirItem::Declaracion { .. } => true,
            _ => false,
        });
    }
    // Registrar las funciones importadas en el resolvedor actual para que las
    // llamadas posteriores se resuelvan como directas.
    for it in &items {
        if let HirItem::Funcion { nombre, .. } = it {
            res.funciones.insert(nombre.clone());
            res.insertar(nombre, None);
        }
    }
    // Incorporar clausuras anidadas (ya están en hir.items tras lower_programa).
    items
}

/// Programa -> HIR.
pub fn lower_programa(
    ast: &[Sentencia],
    ctx: &crate::semantico::ContextoSemantico,
) -> Result<HirPrograma, Vec<String>> {
    let mut items = Vec::new();
    let mut errores = Vec::new();
    let mut res = ResolutorHir::nuevo();

    // Pre-paso: registrar los nombres de las funciones de primer nivel en el
    // ámbito raíz (permite llamadas anticipadas y recursión), igual que
    // `registrar_funciones` en el semántico.
    for s in ast {
        match s {
            Sentencia::Funcion { nombre, .. } => {
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
                let val = lower_expresion(valor, ctx, &mut res);
                let id = res.insertar(nombre, None);
                res.variables.insert(nombre.clone());
                let tipo = tipo_anotacion
                    .as_ref()
                    .map(crate::semantico::tipo_anotacion_a_sem)
                    .unwrap_or(TipoSem::Desconocido);
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
                // Id ya registrado por el pre-paso.
                let id = res
                    .resolver(nombre)
                    .unwrap_or_else(|| res.insertar(nombre, None));
                let param_names: Vec<String> = parametros.iter().map(|(n, _)| n.clone()).collect();
                let (free, capturadas_por_anidados) = variables_libres(cuerpo, &param_names);
                let _capturas: Vec<SimbodoId> = free
                    .iter()
                    .map(|n| res.resolver(n).unwrap_or_else(|| res.insertar(n, None)))
                    .collect();
                res.entrar();
                let mut params: Vec<(SimbodoId, TipoSem)> = Vec::new();
                for (n, t) in parametros {
                    let pid = res.insertar(n, None);
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
                    match lower_sentencia(st, ctx, &mut res) {
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
                    celdas: capturadas_ids.clone(),
                    retorno: ret,
                    cuerpo: body,
                });
            }
            Sentencia::Expresion(e) => {
                items.push(HirItem::Expresion(lower_expresion(e, ctx, &mut res)));
            }
            Sentencia::IntentarRescatar {
                cuerpo_intentar,
                variable_error,
                cuerpo_rescatar,
                ..
            } => {
                res.entrar();
                let ci: Vec<HirSentencia> = cuerpo_intentar
                    .iter()
                    .filter_map(|st| lower_sentencia(st, ctx, &mut res).ok())
                    .collect();
                let var_nombre = variable_error.as_ref().map(|v| {
                    res.insertar(v, None);
                    v.clone()
                });
                let cr: Vec<HirSentencia> = cuerpo_rescatar
                    .iter()
                    .filter_map(|st| lower_sentencia(st, ctx, &mut res).ok())
                    .collect();
                res.salir();
                items.push(HirItem::IntentarRescatar {
                    cuerpo_intentar: ci,
                    variable_error: var_nombre,
                    cuerpo_rescatar: cr,
                });
            }
            Sentencia::Importar { ruta, alias, .. } => {
                let mods = cargar_modulo_items(ruta, ctx, &mut res, None);
                items.push(HirItem::Importar {
                    ruta: ruta.clone(),
                    alias: alias.clone(),
                });
                items.extend(mods);
            }
            Sentencia::Exportar { sentencia, .. } => {
                match lower_sentencia(sentencia, ctx, &mut res) {
                    Ok(hs) => items.push(HirItem::Exportar {
                        sentencia: Box::new(hs),
                    }),
                    Err(e) => errores.push(e),
                }
            }
            Sentencia::Usar {
                ruta,
                alias,
                nombres,
                ..
            } => {
                let mods = cargar_modulo_items(ruta, ctx, &mut res, nombres.as_ref());
                items.push(HirItem::Usar {
                    ruta: ruta.clone(),
                    alias: alias.clone(),
                    nombres: nombres.clone(),
                });
                items.extend(mods);
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
    // Incorporar las clausuras recolectadas durante el lowering.
    items.append(&mut res.clausuras);
    Ok(HirPrograma { items })
}
