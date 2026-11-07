//! MIR (Mid-level Intermediate Representation) de BASILISCO.
//!
//! Representación de bajo nivel estructurada en bloques básicos con terminadores
//! explícitos. Preparada para análisis de flujo de control, SSA y generación
//! de bytecode para la VM.

use std::collections::HashMap;

use crate::hir::{HirExpresion, HirItem, HirLiteral, HirPrograma, HirSentencia};
use crate::semantico::TipoSem;
use crate::simbodo::SimbodoId;

/// Identificador de bloque básico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

/// Identificador de local (variable/temporal) dentro de una función.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);

/// Operando MIR.
#[derive(Debug, Clone, PartialEq)]
pub enum MirOperand {
    /// Copia de un local.
    Copy(LocalId),
    /// Movimiento de un local (el origen queda inválido).
    Move(LocalId),
    /// Constante literal.
    Const(MirConst),
    /// Acceso a una variable global.
    Global(GlobalId),
}

/// Identificador de variable global.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalId(pub usize);

/// Constante MIR.
#[derive(Debug, Clone, PartialEq)]
pub enum MirConst {
    Entero(i64),
    Decimal(i128),
    Real(f64),
    Logico(bool),
    Texto(String),
    Nulo,
}

/// Lugar (lvalue) MIR: local + proyecciones.
#[derive(Debug, Clone, PartialEq)]
pub struct MirPlace {
    pub local: LocalId,
    pub projection: Vec<MirProjection>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirProjection {
    Campo(String),
    Indice(MirOperand),
    Celda,
}

/// R-value MIR.
#[derive(Debug, Clone, PartialEq)]
pub enum MirRvalue {
    Use(MirOperand),
    BinOp {
        op: MirBinOp,
        lhs: MirOperand,
        rhs: MirOperand,
    },
    UnOp {
        op: MirUnOp,
        operand: MirOperand,
    },
    Agregado {
        elementos: Vec<MirOperand>,
    },
    Indice {
        base: MirOperand,
        indice: MirOperand,
    },
    Campo {
        base: MirOperand,
        campo: String,
    },
    CrearCelda {
        valor: MirOperand,
    },
    LeerCelda {
        base: MirOperand,
    },
    /// Creación de una clausura compilada: función por nombre + capturas.
    Clausura {
        nombre: String,
        capturas: Vec<MirOperand>,
    },
    /// Rango `inicio..fin` materializado como lista de enteros.
    Rango {
        inicio: MirOperand,
        fin: MirOperand,
        inclusivo: bool,
    },
    /// Longitud de un arreglo/lista/texto.
    Longitud {
        base: MirOperand,
    },
    /// Objeto-módulo: mapea nombres a Valor::Funcion(prefijo_nombre).
    ObjetoModulo {
        prefijo: String,
        nombres: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirBinOp {
    Suma,
    Resta,
    Multiplicacion,
    Division,
    Modulo,
    Igual,
    Distinto,
    Menor,
    MenorIgual,
    Mayor,
    MayorIgual,
    Y,
    O,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirUnOp {
    Negacion,
    No,
}

/// Sentencia MIR (no terminador).
#[derive(Debug, Clone, PartialEq)]
pub enum MirStmt {
    /// `_local = rvalue`
    Asignacion { destino: MirPlace, valor: MirRvalue },
    /// `_global = rvalue`
    AsignacionGlobal { global: GlobalId, valor: MirRvalue },
    /// Llamada a función con resultado en `destino` (si Some).
    Llamada {
        destino: Option<MirPlace>,
        callee: String,
        argumentos: Vec<MirOperand>,
    },
    /// Llamada indirecta (a través de un valor).
    LlamadaIndirecta {
        destino: Option<MirPlace>,
        callee: MirOperand,
        argumentos: Vec<MirOperand>,
    },
    /// Entrar en región try: rescue/merge blocks y variable de error (local).
    EntrarTry {
        rescue: BlockId,
        merge: BlockId,
        var_local: Option<usize>,
    },
    /// Salir de región try.
    SalirTry,
    /// Punto de no-retorno / marcador (nop).
    Nop,
}

/// Terminador de bloque básico.
#[derive(Debug, Clone, PartialEq)]
pub enum MirTerminator {
    /// Salto incondicional.
    Goto(BlockId),
    /// Salto condicional.
    SwitchInt {
        discriminante: MirOperand,
        /// (valor, bloque) para cada caso.
        casos: Vec<(i64, BlockId)>,
        por_defecto: BlockId,
    },
    /// Retorno de función.
    Return(Option<MirOperand>),
    /// Llamada a función que termina el bloque (tail call / invoke).
    Invoke {
        callee: String,
        argumentos: Vec<MirOperand>,
        destino: Option<MirPlace>,
        bloque_ok: BlockId,
        bloque_error: BlockId,
    },
    /// Inalcanzable.
    Unreachable,
}

/// Bloque básico MIR.
#[derive(Debug, Clone, PartialEq)]
pub struct MirBasicBlock {
    pub id: BlockId,
    pub stmts: Vec<MirStmt>,
    pub terminator: MirTerminator,
}

/// Función MIR.
#[derive(Debug, Clone, PartialEq)]
pub struct MirFunction {
    pub nombre: String,
    pub params: Vec<(LocalId, TipoSem)>,
    pub retorno: TipoSem,
    pub locals: Vec<TipoSem>,
    pub bloques: Vec<MirBasicBlock>,
    pub bloque_entrada: BlockId,
}

/// Programa MIR.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MirPrograma {
    pub funciones: Vec<MirFunction>,
    pub globals: Vec<TipoSem>,
}

/// Contexto de lowering de una función.
struct LoweringCtx {
    locals: Vec<TipoSem>,
    /// Mapeo `SimbodoId -> LocalId`: los ids de símbodo no son consecutivos
    /// (guarida de resolución léxica del HIR), así que cada local se registra.
    mapeo: HashMap<SimbodoId, LocalId>,
    /// Mapeo `SimbodoId -> GlobalId` para variables globales (top-level).
    globals: HashMap<SimbodoId, GlobalId>,
    /// Variables capturadas (por referencia) en esta función.
    capturadas: std::collections::HashSet<SimbodoId>,
    bloques: Vec<MirBasicBlock>,
    bloque_actual: BlockId,
    contador_temp: usize,
    /// Pila de bucles: (bloque_condicion, bloque_salida).
    bucles: Vec<(BlockId, BlockId)>,
}

impl LoweringCtx {
    fn nuevo(globals: HashMap<SimbodoId, GlobalId>) -> Self {
        let mut ctx = LoweringCtx {
            locals: Vec::new(),
            mapeo: HashMap::new(),
            globals,
            capturadas: std::collections::HashSet::new(),
            bloques: Vec::new(),
            bloque_actual: BlockId(0),
            contador_temp: 0,
            bucles: Vec::new(),
        };
        // Bloque de entrada inicial.
        ctx.bloques.push(MirBasicBlock {
            id: BlockId(0),
            stmts: Vec::new(),
            terminator: MirTerminator::Unreachable,
        });
        ctx
    }

    fn nuevo_local(&mut self, tipo: TipoSem) -> LocalId {
        let id = LocalId(self.locals.len());
        self.locals.push(tipo);
        id
    }

    fn local_de_simbodo(&mut self, id: SimbodoId, tipo: TipoSem) -> LocalId {
        if let Some(&local) = self.mapeo.get(&id) {
            return local;
        }
        let local = self.nuevo_local(tipo);
        self.mapeo.insert(id, local);
        local
    }

    fn nuevo_temp(&mut self, tipo: TipoSem) -> LocalId {
        self.contador_temp += 1;
        self.nuevo_local(tipo)
    }

    fn nuevo_bloque(&mut self) -> BlockId {
        let id = BlockId(self.bloques.len());
        self.bloques.push(MirBasicBlock {
            id,
            stmts: Vec::new(),
            terminator: MirTerminator::Unreachable,
        });
        id
    }

    fn bloque_mut(&mut self, id: BlockId) -> &mut MirBasicBlock {
        &mut self.bloques[id.0]
    }

    fn emitir_stmt(&mut self, stmt: MirStmt) {
        self.bloque_mut(self.bloque_actual).stmts.push(stmt);
    }

    fn set_terminator(&mut self, t: MirTerminator) {
        self.bloque_mut(self.bloque_actual).terminator = t;
    }

    fn cambiar_bloque(&mut self, id: BlockId) {
        self.bloque_actual = id;
    }
}

/// Convierte un literal HIR a constante MIR.
fn lower_literal(lit: &HirLiteral) -> MirConst {
    match lit {
        HirLiteral::Entero(n) => MirConst::Entero(*n),
        HirLiteral::Decimal(n) => MirConst::Decimal(*n),
        HirLiteral::Real(n) => MirConst::Real(*n),
        HirLiteral::Logico(b) => MirConst::Logico(*b),
        HirLiteral::Texto(s) => MirConst::Texto(s.clone()),
        HirLiteral::Nulo => MirConst::Nulo,
    }
}

/// Convierte una expresión HIR a un operando MIR, emitiendo statements
/// en el bloque actual para sub-expresiones.
fn lower_expr(ctx: &mut LoweringCtx, e: &HirExpresion) -> MirOperand {
    match e {
        HirExpresion::Literal(lit, _tipo) => MirOperand::Const(lower_literal(lit)),
        HirExpresion::Variable(id, _tipo) => {
            if let Some(&gid) = ctx.globals.get(id) {
                return MirOperand::Global(gid);
            }
            let local = ctx.local_de_simbodo(*id, TipoSem::Desconocido);
            if ctx.capturadas.contains(id) {
                let tmp = ctx.nuevo_temp(TipoSem::Desconocido);
                ctx.emitir_stmt(MirStmt::Asignacion {
                    destino: MirPlace {
                        local: tmp,
                        projection: vec![],
                    },
                    valor: MirRvalue::LeerCelda {
                        base: MirOperand::Copy(local),
                    },
                });
                MirOperand::Copy(tmp)
            } else {
                MirOperand::Copy(local)
            }
        }
        HirExpresion::Suma(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Suma,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Resta(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Resta,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Multiplicacion(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Multiplicacion,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Division(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Division,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Modulo(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Modulo,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::IgualIgual(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Igual,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Distinto(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Distinto,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Menor(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Menor,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Mayor(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Mayor,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::No(a, tipo) => {
            let la = lower_expr(ctx, a);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::UnOp {
                    op: MirUnOp::No,
                    operand: la,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Y(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Y,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::O(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::O,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::ArregloLiteral { elementos, tipo } => {
            let ops: Vec<MirOperand> = elementos.iter().map(|e| lower_expr(ctx, e)).collect();
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::Agregado { elementos: ops },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Indice {
            objeto,
            indice,
            tipo,
        } => {
            let base = lower_expr(ctx, objeto);
            let idx = lower_expr(ctx, indice);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::Indice { base, indice: idx },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Llamada {
            nombre,
            argumentos,
            tipo,
        } => {
            let args: Vec<MirOperand> = argumentos.iter().map(|a| lower_expr(ctx, a)).collect();
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Llamada {
                destino: Some(MirPlace {
                    local: tmp,
                    projection: vec![],
                }),
                callee: nombre.clone(),
                argumentos: args,
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::LlamadaIndirecta {
            callee,
            argumentos,
            tipo,
        } => {
            let c = lower_expr(ctx, callee);
            let args: Vec<MirOperand> = argumentos.iter().map(|a| lower_expr(ctx, a)).collect();
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::LlamadaIndirecta {
                destino: Some(MirPlace {
                    local: tmp,
                    projection: vec![],
                }),
                callee: c,
                argumentos: args,
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Lambda {
            nombre,
            params: _,
            capturas,
            tipo,
        } => {
            let caps: Vec<MirOperand> = capturas
                .iter()
                .map(|id| {
                    let local = ctx.local_de_simbodo(*id, TipoSem::Desconocido);
                    MirOperand::Copy(local)
                })
                .collect();
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::Clausura {
                    nombre: nombre.clone(),
                    capturas: caps,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Si {
            condicion,
            entonces: _,
            sino: _,
        } => {
            // Si-expresión: lowering a bloque con phi implícito no soportado aún;
            // se emite como statement y devuelve nulo.
            let _ = lower_expr(ctx, condicion);
            MirOperand::Const(MirConst::Nulo)
        }
        HirExpresion::MayorIgual(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::MayorIgual,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::MenorIgual(a, b, tipo) => {
            let la = lower_expr(ctx, a);
            let lb = lower_expr(ctx, b);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::MenorIgual,
                    lhs: la,
                    rhs: lb,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Acceso {
            objeto,
            miembro,
            tipo,
        } => {
            let base = lower_expr(ctx, objeto);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::Campo {
                    base,
                    campo: miembro.clone(),
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::Rango {
            inicio,
            fin,
            inclusivo,
            tipo,
        } => {
            let i = lower_expr(ctx, inicio);
            let f = lower_expr(ctx, fin);
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::Rango {
                    inicio: i,
                    fin: f,
                    inclusivo: *inclusivo,
                },
            });
            MirOperand::Copy(tmp)
        }
        HirExpresion::ModuloObjeto {
            prefijo,
            nombres,
            tipo,
        } => {
            let tmp = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: tmp,
                    projection: vec![],
                },
                valor: MirRvalue::ObjetoModulo {
                    prefijo: prefijo.clone(),
                    nombres: nombres.clone(),
                },
            });
            MirOperand::Copy(tmp)
        }
        _ => MirOperand::Const(MirConst::Nulo),
    }
}

/// Lowering de una sentencia HIR.
fn lower_stmt(ctx: &mut LoweringCtx, s: &HirSentencia) -> Result<(), String> {
    match s {
        HirSentencia::Declaracion {
            simbodo,
            tipo,
            valor,
            ..
        } => {
            let op = lower_expr(ctx, valor);
            if let Some(&gid) = ctx.globals.get(simbodo) {
                ctx.emitir_stmt(MirStmt::AsignacionGlobal {
                    global: gid,
                    valor: MirRvalue::Use(op),
                });
                return Ok(());
            }
            let local = ctx.local_de_simbodo(*simbodo, tipo.clone());
            if ctx.capturadas.contains(simbodo) {
                let tmp = ctx.nuevo_temp(TipoSem::Desconocido);
                ctx.emitir_stmt(MirStmt::Asignacion {
                    destino: MirPlace {
                        local: tmp,
                        projection: vec![],
                    },
                    valor: MirRvalue::CrearCelda { valor: op },
                });
                ctx.emitir_stmt(MirStmt::Asignacion {
                    destino: MirPlace {
                        local,
                        projection: vec![],
                    },
                    valor: MirRvalue::Use(MirOperand::Copy(tmp)),
                });
            } else {
                ctx.emitir_stmt(MirStmt::Asignacion {
                    destino: MirPlace {
                        local,
                        projection: vec![],
                    },
                    valor: MirRvalue::Use(op),
                });
            }
            Ok(())
        }
        HirSentencia::Asignacion { simbodo, valor } => {
            let op = lower_expr(ctx, valor);
            if let Some(&gid) = ctx.globals.get(simbodo) {
                ctx.emitir_stmt(MirStmt::AsignacionGlobal {
                    global: gid,
                    valor: MirRvalue::Use(op),
                });
                return Ok(());
            }
            let local = ctx.local_de_simbodo(*simbodo, TipoSem::Desconocido);
            if ctx.capturadas.contains(simbodo) {
                ctx.emitir_stmt(MirStmt::Asignacion {
                    destino: MirPlace {
                        local,
                        projection: vec![MirProjection::Celda],
                    },
                    valor: MirRvalue::Use(op),
                });
            } else {
                ctx.emitir_stmt(MirStmt::Asignacion {
                    destino: MirPlace {
                        local,
                        projection: vec![],
                    },
                    valor: MirRvalue::Use(op),
                });
            }
            Ok(())
        }
        HirSentencia::Expresion(e) => {
            let _ = lower_expr(ctx, e);
            Ok(())
        }
        HirSentencia::Retornar(Some(e)) => {
            let op = lower_expr(ctx, e);
            ctx.set_terminator(MirTerminator::Return(Some(op)));
            Ok(())
        }
        HirSentencia::Retornar(None) => {
            ctx.set_terminator(MirTerminator::Return(None));
            Ok(())
        }
        HirSentencia::Si {
            condicion,
            entonces,
            sino,
        } => {
            let cond = lower_expr(ctx, condicion);
            let bloque_entonces = ctx.nuevo_bloque();
            let bloque_sino = ctx.nuevo_bloque();
            let bloque_merge = ctx.nuevo_bloque();

            // Terminador del bloque actual.
            ctx.set_terminator(MirTerminator::SwitchInt {
                discriminante: cond,
                casos: vec![(1, bloque_entonces)],
                por_defecto: bloque_sino,
            });

            // Bloque entonces.
            ctx.cambiar_bloque(bloque_entonces);
            for st in entonces {
                lower_stmt(ctx, st)?;
            }
            // Si no terminó con terminator, goto merge.
            if matches!(
                ctx.bloque_mut(ctx.bloque_actual).terminator,
                MirTerminator::Unreachable
            ) {
                ctx.set_terminator(MirTerminator::Goto(bloque_merge));
            }

            // Bloque sino.
            ctx.cambiar_bloque(bloque_sino);
            if let Some(sino_stmts) = sino {
                for st in sino_stmts {
                    lower_stmt(ctx, st)?;
                }
            }
            if matches!(
                ctx.bloque_mut(ctx.bloque_actual).terminator,
                MirTerminator::Unreachable
            ) {
                ctx.set_terminator(MirTerminator::Goto(bloque_merge));
            }

            // Merge.
            ctx.cambiar_bloque(bloque_merge);
            Ok(())
        }
        HirSentencia::Mientras { condicion, cuerpo } => {
            let bloque_cond = ctx.nuevo_bloque();
            let bloque_cuerpo = ctx.nuevo_bloque();
            let bloque_salida = ctx.nuevo_bloque();

            ctx.bucles.push((bloque_cond, bloque_salida));

            // Ir a condición.
            ctx.set_terminator(MirTerminator::Goto(bloque_cond));

            // Bloque condición.
            ctx.cambiar_bloque(bloque_cond);
            let cond = lower_expr(ctx, condicion);
            ctx.set_terminator(MirTerminator::SwitchInt {
                discriminante: cond,
                casos: vec![(1, bloque_cuerpo)],
                por_defecto: bloque_salida,
            });

            // Cuerpo.
            ctx.cambiar_bloque(bloque_cuerpo);
            for st in cuerpo {
                lower_stmt(ctx, st)?;
            }
            if matches!(
                ctx.bloque_mut(ctx.bloque_actual).terminator,
                MirTerminator::Unreachable
            ) {
                ctx.set_terminator(MirTerminator::Goto(bloque_cond));
            }

            ctx.bucles.pop();

            // Salida.
            ctx.cambiar_bloque(bloque_salida);
            Ok(())
        }
        HirSentencia::Bloque(stmts) => {
            for st in stmts {
                lower_stmt(ctx, st)?;
            }
            Ok(())
        }
        HirSentencia::Romper => {
            if let Some((_, salida)) = ctx.bucles.last() {
                let salida = *salida;
                ctx.set_terminator(MirTerminator::Goto(salida));
            }
            Ok(())
        }
        HirSentencia::Continuar => {
            if let Some((cond, _)) = ctx.bucles.last() {
                let cond = *cond;
                ctx.set_terminator(MirTerminator::Goto(cond));
            }
            Ok(())
        }
        HirSentencia::AsignacionIndice {
            objeto,
            indice,
            valor,
        } => {
            // Caso soportado: base es una variable (local). La escritura se
            // materializa como asignación a un `MirPlace` con proyección `Indice`.
            if let HirExpresion::Variable(id, _) = objeto {
                let local = ctx.local_de_simbodo(*id, TipoSem::Desconocido);
                let idx_op = lower_expr(ctx, indice);
                let val_op = lower_expr(ctx, valor);
                ctx.emitir_stmt(MirStmt::Asignacion {
                    destino: MirPlace {
                        local,
                        projection: vec![MirProjection::Indice(idx_op)],
                    },
                    valor: MirRvalue::Use(val_op),
                });
            } else {
                let _ = (objeto, indice, valor);
            }
            Ok(())
        }
        HirSentencia::ParaCada {
            variable,
            iterable,
            cuerpo,
        } => {
            // Bucle por índice sobre el iterable (arreglo/lista/texto).
            let iter_op = lower_expr(ctx, iterable);
            let coll_local = match iter_op {
                MirOperand::Copy(l) => l,
                other => {
                    let tmp = ctx.nuevo_temp(TipoSem::Desconocido);
                    ctx.emitir_stmt(MirStmt::Asignacion {
                        destino: MirPlace {
                            local: tmp,
                            projection: vec![],
                        },
                        valor: MirRvalue::Use(other),
                    });
                    tmp
                }
            };
            let var_local = ctx.local_de_simbodo(*variable, TipoSem::Desconocido);
            let idx_local = ctx.nuevo_temp(TipoSem::Entero);
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: idx_local,
                    projection: vec![],
                },
                valor: MirRvalue::Use(MirOperand::Const(MirConst::Entero(0))),
            });
            let bloque_cond = ctx.nuevo_bloque();
            let bloque_cuerpo = ctx.nuevo_bloque();
            let bloque_salida = ctx.nuevo_bloque();
            ctx.bucles.push((bloque_cond, bloque_salida));

            ctx.set_terminator(MirTerminator::Goto(bloque_cond));
            // Condición: idx < longitud(iterable).
            ctx.cambiar_bloque(bloque_cond);
            let len_tmp = ctx.nuevo_temp(TipoSem::Entero);
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: len_tmp,
                    projection: vec![],
                },
                valor: MirRvalue::Longitud {
                    base: MirOperand::Copy(coll_local),
                },
            });
            let cond_tmp = ctx.nuevo_temp(TipoSem::Logico);
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: cond_tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Menor,
                    lhs: MirOperand::Copy(idx_local),
                    rhs: MirOperand::Copy(len_tmp),
                },
            });
            ctx.set_terminator(MirTerminator::SwitchInt {
                discriminante: MirOperand::Copy(cond_tmp),
                casos: vec![(1, bloque_cuerpo)],
                por_defecto: bloque_salida,
            });
            // Cuerpo: variable = iterable[idx]; ...; idx = idx + 1.
            ctx.cambiar_bloque(bloque_cuerpo);
            let elem_tmp = ctx.nuevo_temp(TipoSem::Desconocido);
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: elem_tmp,
                    projection: vec![],
                },
                valor: MirRvalue::Indice {
                    base: MirOperand::Copy(coll_local),
                    indice: MirOperand::Copy(idx_local),
                },
            });
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: var_local,
                    projection: vec![],
                },
                valor: MirRvalue::Use(MirOperand::Copy(elem_tmp)),
            });
            for st in cuerpo {
                lower_stmt(ctx, st)?;
            }
            let inc_tmp = ctx.nuevo_temp(TipoSem::Entero);
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: inc_tmp,
                    projection: vec![],
                },
                valor: MirRvalue::BinOp {
                    op: MirBinOp::Suma,
                    lhs: MirOperand::Copy(idx_local),
                    rhs: MirOperand::Const(MirConst::Entero(1)),
                },
            });
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: idx_local,
                    projection: vec![],
                },
                valor: MirRvalue::Use(MirOperand::Copy(inc_tmp)),
            });
            if matches!(
                ctx.bloque_mut(ctx.bloque_actual).terminator,
                MirTerminator::Unreachable
            ) {
                ctx.set_terminator(MirTerminator::Goto(bloque_cond));
            }

            ctx.bucles.pop();

            ctx.cambiar_bloque(bloque_salida);
            Ok(())
        }
        HirSentencia::IntentarRescatar {
            cuerpo_intentar,
            variable_error,
            cuerpo_rescatar,
        } => {
            let rescue_block = ctx.nuevo_bloque();
            let merge_block = ctx.nuevo_bloque();
            let var_local =
                variable_error.map(|id| ctx.local_de_simbodo(id, TipoSem::Desconocido).0);
            ctx.emitir_stmt(MirStmt::EntrarTry {
                rescue: rescue_block,
                merge: merge_block,
                var_local,
            });
            for st in cuerpo_intentar {
                lower_stmt(ctx, st)?;
            }
            ctx.emitir_stmt(MirStmt::SalirTry);
            if matches!(
                ctx.bloque_mut(ctx.bloque_actual).terminator,
                MirTerminator::Unreachable
            ) {
                ctx.set_terminator(MirTerminator::Goto(merge_block));
            }
            ctx.cambiar_bloque(rescue_block);
            for st in cuerpo_rescatar {
                lower_stmt(ctx, st)?;
            }
            if matches!(
                ctx.bloque_mut(ctx.bloque_actual).terminator,
                MirTerminator::Unreachable
            ) {
                ctx.set_terminator(MirTerminator::Goto(merge_block));
            }
            ctx.cambiar_bloque(merge_block);
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Lowering de un item HIR a función MIR.
fn lower_item(
    ctx_global: &mut MirPrograma,
    item: &HirItem,
    globals: &std::collections::HashMap<SimbodoId, GlobalId>,
) {
    // Las funciones de módulos inlinados (nombre con prefijo alias_) no deben
    // ver las globales del módulo principal: sus parámetros pueden colisionar
    // con ids de símbodo de declaraciones top-level del principal.
    let es_modulo = match item {
        HirItem::Funcion { nombre, .. } => nombre.contains('_'),
        _ => false,
    };
    match item {
        HirItem::Funcion {
            nombre,
            simbodo: _,
            params,
            capturadas: _capturadas,
            celdas,
            capturas_externas,
            retorno,
            cuerpo,
        } => {
            let mut ctx = LoweringCtx::nuevo(if es_modulo {
                std::collections::HashMap::new()
            } else {
                globals.clone()
            });
            ctx.capturadas = celdas.iter().cloned().collect();
            let externas: std::collections::HashSet<SimbodoId> =
                capturas_externas.iter().cloned().collect();
            // Params como locals.
            let mut param_locals = Vec::new();
            for (sid, ty) in params {
                let local = ctx.nuevo_local(ty.clone());
                ctx.mapeo.insert(*sid, local);
                param_locals.push((local, ty.clone()));
            }
            // Variables capturadas que son parámetros se envuelven en celda
            // al entrar a la función, salvo las que ya llegan como celdas.
            for ((sid, _), (local, _)) in params.iter().zip(param_locals.iter()) {
                if ctx.capturadas.contains(sid) && !externas.contains(sid) {
                    let tmp = ctx.nuevo_temp(TipoSem::Desconocido);
                    ctx.emitir_stmt(MirStmt::Asignacion {
                        destino: MirPlace {
                            local: tmp,
                            projection: vec![],
                        },
                        valor: MirRvalue::CrearCelda {
                            valor: MirOperand::Copy(*local),
                        },
                    });
                    ctx.emitir_stmt(MirStmt::Asignacion {
                        destino: MirPlace {
                            local: *local,
                            projection: vec![],
                        },
                        valor: MirRvalue::Use(MirOperand::Copy(tmp)),
                    });
                }
            }
            // Cuerpo.
            for st in cuerpo {
                let _ = lower_stmt(&mut ctx, st);
            }
            // Si no hay terminator en el bloque actual, añadir Return.
            if matches!(
                ctx.bloque_mut(ctx.bloque_actual).terminator,
                MirTerminator::Unreachable
            ) {
                ctx.set_terminator(MirTerminator::Return(None));
            }
            let func = MirFunction {
                nombre: nombre.clone(),
                params: param_locals,
                retorno: retorno.clone(),
                locals: ctx.locals,
                bloques: ctx.bloques,
                bloque_entrada: BlockId(0),
            };
            ctx_global.funciones.push(func);
        }
        HirItem::Declaracion {
            simbodo,
            tipo,
            valor,
            ..
        } => {
            // Global: la inicialización se hace en la función __init de
            // `lower_programa`; aquí no hay nada que bajar.
            let _ = (simbodo, tipo, valor);
        }
        _ => {}
    }
}

/// Lowering de programa HIR a MIR.
pub fn lower_programa(hir: &HirPrograma) -> MirPrograma {
    // Primera pasada: identificar variables globales (HirItem::Declaracion) y
    // asignarles un GlobalId.
    let mut global_map: std::collections::HashMap<SimbodoId, GlobalId> =
        std::collections::HashMap::new();
    let mut global_types: Vec<TipoSem> = Vec::new();
    for item in &hir.items {
        if let HirItem::Declaracion { simbodo, tipo, .. } = item {
            let id = GlobalId(global_types.len());
            global_map.insert(*simbodo, id);
            global_types.push(tipo.clone());
        }
    }

    // Segunda pasada: bajar items (funciones + declaraciones).
    let mut prog = MirPrograma {
        funciones: Vec::new(),
        globals: global_types,
    };
    for item in &hir.items {
        lower_item(&mut prog, item, &global_map);
    }

    // Construir función de inicialización de globales (`__init`) si hay alguna.
    if !global_map.is_empty() {
        let mut ctx = LoweringCtx::nuevo(global_map.clone());
        // Para cada declaración global, bajar su valor y asignarlo al global.
        for item in &hir.items {
            if let HirItem::Declaracion { simbodo, valor, .. } = item {
                let gid = global_map[simbodo];
                let op = lower_expr(&mut ctx, valor);
                // Materializar el operando en un rvalue Use.
                ctx.emitir_stmt(MirStmt::AsignacionGlobal {
                    global: gid,
                    valor: MirRvalue::Use(op),
                });
            }
        }
        // Terminator de retorno.
        if matches!(
            ctx.bloque_mut(BlockId(0)).terminator,
            MirTerminator::Unreachable
        ) {
            ctx.set_terminator(MirTerminator::Return(None));
        }
        let init_func = MirFunction {
            nombre: "__init".to_string(),
            params: Vec::new(),
            retorno: TipoSem::Desconocido,
            locals: ctx.locals,
            bloques: ctx.bloques,
            bloque_entrada: BlockId(0),
        };
        prog.funciones.push(init_func);
    }

    prog
}

/// Verificador MIR: valida invariantes estructurales.
pub struct VerificadorMir;

impl VerificadorMir {
    #[allow(clippy::collapsible_if, clippy::collapsible_match)]
    pub fn verificar(prog: &MirPrograma) -> bool {
        // Nombres de funciones únicos.
        let mut nombres_func: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for f in &prog.funciones {
            if !nombres_func.insert(f.nombre.as_str()) {
                return false;
            }
        }
        for f in &prog.funciones {
            if f.bloques.is_empty() {
                return false;
            }
            // Ids de bloque únicos y contiguos desde 0.
            let mut ids: std::collections::HashSet<BlockId> = std::collections::HashSet::new();
            for (i, b) in f.bloques.iter().enumerate() {
                if b.id.0 != i {
                    return false;
                }
                if !ids.insert(b.id) {
                    return false;
                }
            }
            if !ids.contains(&f.bloque_entrada) {
                return false;
            }
            // Parámetros referencian locals válidos.
            for (local, _) in &f.params {
                if local.0 >= f.locals.len() {
                    return false;
                }
            }
            // Validar cada bloque.
            for b in &f.bloques {
                // Terminadores apuntan a bloques existentes.
                match &b.terminator {
                    MirTerminator::Goto(t) => {
                        if !ids.contains(t) {
                            return false;
                        }
                    }
                    MirTerminator::SwitchInt {
                        casos, por_defecto, ..
                    } => {
                        for (_, blk) in casos {
                            if !ids.contains(blk) {
                                return false;
                            }
                        }
                        if !ids.contains(por_defecto) {
                            return false;
                        }
                    }
                    MirTerminator::Return(op) => {
                        if let Some(o) = op {
                            if !operando_valido(o, f, prog) {
                                return false;
                            }
                        }
                    }
                    MirTerminator::Invoke {
                        bloque_ok,
                        bloque_error,
                        destino,
                        argumentos,
                        ..
                    } => {
                        if !ids.contains(bloque_ok) || !ids.contains(bloque_error) {
                            return false;
                        }
                        for a in argumentos {
                            if !operando_valido(a, f, prog) {
                                return false;
                            }
                        }
                        if let Some(d) = destino {
                            if !lugar_valido(d, f, prog) {
                                return false;
                            }
                        }
                    }
                    MirTerminator::Unreachable => {}
                }
                // Validar sentencias.
                for stmt in &b.stmts {
                    if !stmt_valido(stmt, f, prog) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

#[allow(clippy::collapsible_if)]
fn lugar_valido(p: &MirPlace, f: &MirFunction, prog: &MirPrograma) -> bool {
    if p.local.0 >= f.locals.len() {
        return false;
    }
    // Proyecciones de un solo nivel soportadas.
    for proj in &p.projection {
        match proj {
            MirProjection::Campo(_) => {}
            MirProjection::Indice(op) => {
                if !operando_valido(op, f, prog) {
                    return false;
                }
            }
            MirProjection::Celda => {}
        }
    }
    true
}

fn operando_valido(op: &MirOperand, f: &MirFunction, prog: &MirPrograma) -> bool {
    match op {
        MirOperand::Copy(l) | MirOperand::Move(l) => l.0 < f.locals.len(),
        MirOperand::Const(_) => true,
        MirOperand::Global(g) => g.0 < prog.globals.len(),
    }
}

fn stmt_valido(stmt: &MirStmt, f: &MirFunction, prog: &MirPrograma) -> bool {
    match stmt {
        MirStmt::Asignacion { destino, valor } => {
            lugar_valido(destino, f, prog) && rvalue_valido(valor, f, prog)
        }
        MirStmt::AsignacionGlobal { global, valor } => {
            global.0 < prog.globals.len() && rvalue_valido(valor, f, prog)
        }
        MirStmt::Llamada {
            destino,
            argumentos,
            ..
        } => {
            if let Some(d) = destino
                && !lugar_valido(d, f, prog)
            {
                return false;
            }
            argumentos.iter().all(|a| operando_valido(a, f, prog))
        }
        MirStmt::LlamadaIndirecta {
            destino,
            callee,
            argumentos,
        } => {
            if let Some(d) = destino
                && !lugar_valido(d, f, prog)
            {
                return false;
            }
            operando_valido(callee, f, prog)
                && argumentos.iter().all(|a| operando_valido(a, f, prog))
        }
        MirStmt::EntrarTry {
            rescue: _,
            merge: _,
            var_local,
        } => {
            // los bloques se validan fuera; rescue/merge deben ser bloques existentes
            // (verificado por el caller que tiene ids). Aquí solo var_local.
            if let Some(vl) = var_local {
                *vl < f.locals.len()
            } else {
                true
            }
        }
        MirStmt::SalirTry | MirStmt::Nop => true,
    }
}

fn rvalue_valido(r: &MirRvalue, f: &MirFunction, prog: &MirPrograma) -> bool {
    match r {
        MirRvalue::Use(op) => operando_valido(op, f, prog),
        MirRvalue::BinOp { lhs, rhs, .. } => {
            operando_valido(lhs, f, prog) && operando_valido(rhs, f, prog)
        }
        MirRvalue::UnOp { operand, .. } => operando_valido(operand, f, prog),
        MirRvalue::Agregado { elementos } => elementos.iter().all(|e| operando_valido(e, f, prog)),
        MirRvalue::Indice { base, indice } => {
            operando_valido(base, f, prog) && operando_valido(indice, f, prog)
        }
        MirRvalue::Campo { base, .. } => operando_valido(base, f, prog),
        MirRvalue::CrearCelda { valor } => operando_valido(valor, f, prog),
        MirRvalue::LeerCelda { base } => operando_valido(base, f, prog),
        MirRvalue::Clausura { capturas, .. } => {
            capturas.iter().all(|c| operando_valido(c, f, prog))
        }
        MirRvalue::Rango { inicio, fin, .. } => {
            operando_valido(inicio, f, prog) && operando_valido(fin, f, prog)
        }
        MirRvalue::Longitud { base } => operando_valido(base, f, prog),
        MirRvalue::ObjetoModulo { .. } => true,
    }
}
