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
    Funcion(String),
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
        HirExpresion::FuncionRef(nombre) => MirOperand::Const(MirConst::Funcion(nombre.clone())),
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
            // Cortocircuito: evaluar RHS solo si LHS es verdadero; validar operandos lógicos.
            let la = lower_expr(ctx, a);
            let la_ok = ctx.nuevo_temp(TipoSem::Logico);
            ctx.emitir_stmt(MirStmt::Llamada {
                destino: Some(MirPlace {
                    local: la_ok,
                    projection: vec![],
                }),
                callee: "__bsk_assert_logico".to_string(),
                argumentos: vec![la, MirOperand::Const(MirConst::Texto("y".to_string()))],
            });
            let result = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: result,
                    projection: vec![],
                },
                valor: MirRvalue::Use(MirOperand::Copy(la_ok)),
            });
            let bloque_rhs = ctx.nuevo_bloque();
            let bloque_done = ctx.nuevo_bloque();
            ctx.set_terminator(MirTerminator::SwitchInt {
                discriminante: MirOperand::Copy(la_ok),
                casos: vec![(1, bloque_rhs)],
                por_defecto: bloque_done,
            });
            ctx.cambiar_bloque(bloque_rhs);
            let lb = lower_expr(ctx, b);
            let lb_ok = ctx.nuevo_temp(TipoSem::Logico);
            ctx.emitir_stmt(MirStmt::Llamada {
                destino: Some(MirPlace {
                    local: lb_ok,
                    projection: vec![],
                }),
                callee: "__bsk_assert_logico".to_string(),
                argumentos: vec![lb, MirOperand::Const(MirConst::Texto("y".to_string()))],
            });
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: result,
                    projection: vec![],
                },
                valor: MirRvalue::Use(MirOperand::Copy(lb_ok)),
            });
            if matches!(
                ctx.bloque_mut(ctx.bloque_actual).terminator,
                MirTerminator::Unreachable
            ) {
                ctx.set_terminator(MirTerminator::Goto(bloque_done));
            }
            ctx.cambiar_bloque(bloque_done);
            MirOperand::Copy(result)
        }
        HirExpresion::O(a, b, tipo) => {
            // Cortocircuito: evaluar RHS solo si LHS es falso; validar operandos lógicos.
            let la = lower_expr(ctx, a);
            let la_ok = ctx.nuevo_temp(TipoSem::Logico);
            ctx.emitir_stmt(MirStmt::Llamada {
                destino: Some(MirPlace {
                    local: la_ok,
                    projection: vec![],
                }),
                callee: "__bsk_assert_logico".to_string(),
                argumentos: vec![la, MirOperand::Const(MirConst::Texto("o".to_string()))],
            });
            let result = ctx.nuevo_temp(tipo.clone());
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: result,
                    projection: vec![],
                },
                valor: MirRvalue::Use(MirOperand::Copy(la_ok)),
            });
            let bloque_rhs = ctx.nuevo_bloque();
            let bloque_done = ctx.nuevo_bloque();
            ctx.set_terminator(MirTerminator::SwitchInt {
                discriminante: MirOperand::Copy(la_ok),
                casos: vec![(1, bloque_done)],
                por_defecto: bloque_rhs,
            });
            ctx.cambiar_bloque(bloque_rhs);
            let lb = lower_expr(ctx, b);
            let lb_ok = ctx.nuevo_temp(TipoSem::Logico);
            ctx.emitir_stmt(MirStmt::Llamada {
                destino: Some(MirPlace {
                    local: lb_ok,
                    projection: vec![],
                }),
                callee: "__bsk_assert_logico".to_string(),
                argumentos: vec![lb, MirOperand::Const(MirConst::Texto("o".to_string()))],
            });
            ctx.emitir_stmt(MirStmt::Asignacion {
                destino: MirPlace {
                    local: result,
                    projection: vec![],
                },
                valor: MirRvalue::Use(MirOperand::Copy(lb_ok)),
            });
            if matches!(
                ctx.bloque_mut(ctx.bloque_actual).terminator,
                MirTerminator::Unreachable
            ) {
                ctx.set_terminator(MirTerminator::Goto(bloque_done));
            }
            ctx.cambiar_bloque(bloque_done);
            MirOperand::Copy(result)
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
            capturadas,
            celdas: _,
            capturas_externas,
            retorno,
            cuerpo,
        } => {
            let mut ctx = LoweringCtx::nuevo(if es_modulo {
                std::collections::HashMap::new()
            } else {
                globals.clone()
            });
            // Celdas = capturas externas + capturas anidadas (dedup). Se usa
            // `capturadas`/`capturas_externas` en lugar del campo `celdas` para
            // evitar duplicados que rechaza el verificador HIR (HIR011), sin
            // cambiar el comportamiento de lowering.
            let mut caps: std::collections::HashSet<SimbodoId> =
                capturas_externas.iter().cloned().collect();
            for c in capturadas {
                caps.insert(*c);
            }
            ctx.capturadas = caps;
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

/// Error de verificación de una invariante MIR (contrato comprobable, Fase 03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorVerificacionMir {
    pub codigo: &'static str,
    pub invariante: &'static str,
    pub contexto: String,
    pub mensaje: String,
}

impl std::fmt::Display for ErrorVerificacionMir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} ({}): {}",
            self.codigo, self.invariante, self.contexto, self.mensaje
        )
    }
}

/// Resultado detallado de la verificación MIR.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResultadoVerificacionMir {
    pub errores: Vec<ErrorVerificacionMir>,
}

impl ResultadoVerificacionMir {
    pub fn es_ok(&self) -> bool {
        self.errores.is_empty()
    }
}

/// Verificador MIR: valida invariantes estructurales.
pub struct VerificadorMir;

fn err_mir(
    codigo: &'static str,
    invariante: &'static str,
    contexto: impl Into<String>,
    mensaje: impl Into<String>,
) -> ErrorVerificacionMir {
    ErrorVerificacionMir {
        codigo,
        invariante,
        contexto: contexto.into(),
        mensaje: mensaje.into(),
    }
}

/// Tipo de un operando MIR (si puede inferirse de locales/globals/constantes).
fn tipo_operando(op: &MirOperand, f: &MirFunction, prog: &MirPrograma) -> Option<TipoSem> {
    match op {
        MirOperand::Copy(l) | MirOperand::Move(l) => f.locals.get(l.0).cloned(),
        MirOperand::Global(g) => prog.globals.get(g.0).cloned(),
        MirOperand::Const(c) => Some(match c {
            MirConst::Entero(_) => TipoSem::Entero,
            MirConst::Decimal(_) => TipoSem::Decimal,
            MirConst::Real(_) => TipoSem::Real,
            MirConst::Logico(_) => TipoSem::Logico,
            MirConst::Texto(_) => TipoSem::Texto,
            MirConst::Nulo => TipoSem::Nulo,
            MirConst::Funcion(_) => TipoSem::Desconocido,
        }),
    }
}

/// Compatibilidad conservadora entre tipos MIR (espejo de `semantico::compatible`).
fn tipos_compatibles_mir(esperado: &TipoSem, obtenido: &TipoSem) -> bool {
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
            ne == no && tipos_compatibles_mir(te, to)
        }
        (TipoSem::Lista(te), TipoSem::Lista(to)) => tipos_compatibles_mir(te, to),
        (TipoSem::Arreglo(te, _), TipoSem::Lista(to)) => tipos_compatibles_mir(te, to),
        (TipoSem::Lista(te), TipoSem::Arreglo(to, _)) => tipos_compatibles_mir(te, to),
        (TipoSem::Coleccion(te), TipoSem::Arreglo(to, _)) => tipos_compatibles_mir(te, to),
        (TipoSem::Coleccion(te), TipoSem::Lista(to)) => tipos_compatibles_mir(te, to),
        (TipoSem::Arreglo(te, _), TipoSem::Coleccion(to)) => tipos_compatibles_mir(te, to),
        (TipoSem::Lista(te), TipoSem::Coleccion(to)) => tipos_compatibles_mir(te, to),
        (TipoSem::Coleccion(te), TipoSem::Coleccion(to)) => tipos_compatibles_mir(te, to),
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
                && p1.iter().zip(p2).all(|(a, b)| tipos_compatibles_mir(a, b))
                && tipos_compatibles_mir(r1, r2)
        }
        _ => false,
    }
}

/// Promoción numérica MIR (espejo de `tipo_numerico_resultado`).
fn tipos_numericos_compatibles_mir(a: &TipoSem, b: &TipoSem) -> bool {
    matches!(a, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
        && matches!(b, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
}

fn tipo_lugar(p: &MirPlace, f: &MirFunction) -> Option<TipoSem> {
    // Conservador: solo lugares sin proyecciones tienen tipo conocido.
    if p.projection.is_empty() {
        f.locals.get(p.local.0).cloned()
    } else {
        None
    }
}

fn tipo_rvalue(rv: &MirRvalue, f: &MirFunction, prog: &MirPrograma) -> Option<TipoSem> {
    match rv {
        MirRvalue::Use(op) => tipo_operando(op, f, prog),
        MirRvalue::BinOp { op, lhs, rhs } => {
            let tl = tipo_operando(lhs, f, prog)?;
            let tr = tipo_operando(rhs, f, prog)?;
            match op {
                MirBinOp::Suma => {
                    if (matches!(tl, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
                        && matches!(tr, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real))
                    {
                        if matches!(tl, TipoSem::Real) || matches!(tr, TipoSem::Real) {
                            Some(TipoSem::Real)
                        } else if matches!(tl, TipoSem::Decimal) || matches!(tr, TipoSem::Decimal) {
                            Some(TipoSem::Decimal)
                        } else {
                            Some(TipoSem::Entero)
                        }
                    } else if tl == TipoSem::Texto && tr == TipoSem::Texto {
                        Some(TipoSem::Texto)
                    } else {
                        None
                    }
                }
                MirBinOp::Resta
                | MirBinOp::Multiplicacion
                | MirBinOp::Division
                | MirBinOp::Modulo => {
                    if matches!(tl, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
                        && matches!(tr, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
                    {
                        if matches!(tl, TipoSem::Real) || matches!(tr, TipoSem::Real) {
                            Some(TipoSem::Real)
                        } else if matches!(tl, TipoSem::Decimal) || matches!(tr, TipoSem::Decimal) {
                            Some(TipoSem::Decimal)
                        } else {
                            Some(TipoSem::Entero)
                        }
                    } else {
                        None
                    }
                }
                MirBinOp::Igual
                | MirBinOp::Distinto
                | MirBinOp::Menor
                | MirBinOp::MenorIgual
                | MirBinOp::Mayor
                | MirBinOp::MayorIgual => Some(TipoSem::Logico),
                MirBinOp::Y | MirBinOp::O => Some(TipoSem::Logico),
            }
        }
        MirRvalue::UnOp { op, operand } => {
            let t = tipo_operando(operand, f, prog)?;
            match op {
                MirUnOp::Negacion => {
                    if matches!(t, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real) {
                        Some(t)
                    } else {
                        None
                    }
                }
                MirUnOp::No => Some(TipoSem::Logico),
            }
        }
        MirRvalue::Agregado { elementos } => {
            let mut ts: Vec<TipoSem> = elementos
                .iter()
                .filter_map(|e| tipo_operando(e, f, prog))
                .collect();
            if ts.is_empty() {
                return None;
            }
            let first = ts.remove(0);
            if ts.iter().all(|t| tipos_compatibles_mir(&first, t)) {
                Some(first)
            } else {
                None
            }
        }
        MirRvalue::Indice { base, indice: _ } => {
            let tb = tipo_operando(base, f, prog)?;
            match tb {
                TipoSem::Arreglo(te, _) | TipoSem::Lista(te) | TipoSem::Coleccion(te) => Some(*te),
                _ => None,
            }
        }
        MirRvalue::Campo { .. } => None,
        MirRvalue::CrearCelda { valor } => tipo_operando(valor, f, prog),
        MirRvalue::LeerCelda { base } => tipo_operando(base, f, prog),
        MirRvalue::Clausura { .. } => None,
        MirRvalue::Rango { .. } => Some(TipoSem::Lista(Box::new(TipoSem::Entero))),
        MirRvalue::Longitud { .. } => Some(TipoSem::Entero),
        MirRvalue::ObjetoModulo { .. } => None,
    }
}

impl VerificadorMir {
    /// Verificación booleana (compatibilidad). Equivale a `verificar_detallado().es_ok()`.
    pub fn verificar(prog: &MirPrograma) -> bool {
        Self::verificar_detallado(prog).es_ok()
    }

    /// Verificación con contrato detallado (Fase 03): reporta la primera
    /// invariante violada encontrada, con código, contexto y mensaje.
    #[allow(clippy::collapsible_if, clippy::collapsible_match)]
    pub fn verificar_detallado(prog: &MirPrograma) -> ResultadoVerificacionMir {
        let mut nombres_func: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for f in &prog.funciones {
            if !nombres_func.insert(f.nombre.as_str()) {
                return ResultadoVerificacionMir {
                    errores: vec![err_mir(
                        "MIR001",
                        "nombre-funcion-unico",
                        &f.nombre,
                        "nombre de función duplicado",
                    )],
                };
            }
        }
        for f in &prog.funciones {
            if let Err(e) = verificar_funcion_mir(f, prog) {
                return ResultadoVerificacionMir { errores: vec![e] };
            }
        }
        ResultadoVerificacionMir::default()
    }
}

/// Sucesores normales de un terminador (transiciones explícitas del CFG).
fn sucesores_terminador(t: &MirTerminator) -> Vec<BlockId> {
    match t {
        MirTerminator::Goto(d) => vec![*d],
        MirTerminator::SwitchInt {
            casos, por_defecto, ..
        } => {
            let mut v: Vec<BlockId> = casos.iter().map(|(_, b)| *b).collect();
            v.push(*por_defecto);
            v
        }
        MirTerminator::Return(_) | MirTerminator::Unreachable => vec![],
        MirTerminator::Invoke {
            bloque_ok,
            bloque_error,
            ..
        } => vec![*bloque_ok, *bloque_error],
    }
}

/// Bloques alcanzables desde la entrada respetando transiciones reales.
/// Conservador con `EntrarTry`: si un bloque alcanzable contiene `EntrarTry`,
/// su `rescue` se considera alcanzable por la arista excepcional (la VM puede
/// transferir control a `rescue` ante error dentro de la región protegida).
fn bloques_alcanzables(
    f: &MirFunction,
    ids: &std::collections::HashSet<BlockId>,
) -> std::collections::HashSet<BlockId> {
    let mut alcanzables: std::collections::HashSet<BlockId> = std::collections::HashSet::new();
    let mut cola: std::collections::VecDeque<BlockId> = std::collections::VecDeque::new();
    if ids.contains(&f.bloque_entrada) {
        alcanzables.insert(f.bloque_entrada);
        cola.push_back(f.bloque_entrada);
    }
    while let Some(bid) = cola.pop_front() {
        let Some(b) = f.bloques.iter().find(|x| x.id == bid) else {
            continue;
        };
        // Arista excepcional conservadora: try activo en bloque alcanzable.
        for stmt in &b.stmts {
            if let MirStmt::EntrarTry { rescue, .. } = stmt
                && ids.contains(rescue)
                && alcanzables.insert(*rescue)
            {
                cola.push_back(*rescue);
            }
        }
        for s in sucesores_terminador(&b.terminator) {
            if ids.contains(&s) && alcanzables.insert(s) {
                cola.push_back(s);
            }
        }
    }
    alcanzables
}

/// Indica si el tipo de retorno declarado exige un valor en caminos alcanzables.
/// Conservador: `Desconocido` y `Nulo` no exigen valor (evita falsos positivos
/// en funciones void/sintéticas y casos no tipados).
fn retorno_requiere_valor(t: &TipoSem) -> bool {
    !matches!(t, TipoSem::Desconocido | TipoSem::Nulo)
}

fn verificar_funcion_mir(f: &MirFunction, prog: &MirPrograma) -> Result<(), ErrorVerificacionMir> {
    let ctx_f = format!("funcion '{}'", f.nombre);
    if f.bloques.is_empty() {
        return Err(err_mir(
            "MIR010",
            "bloques-no-vacios",
            &ctx_f,
            "la función no tiene bloques",
        ));
    }
    // Ids de bloque únicos y contiguos desde 0.
    let mut ids: std::collections::HashSet<BlockId> = std::collections::HashSet::new();
    for (i, b) in f.bloques.iter().enumerate() {
        if b.id.0 != i {
            return Err(err_mir(
                "MIR011",
                "bloques-contiguos",
                &ctx_f,
                format!("bloque en índice {i} tiene id {}", b.id.0),
            ));
        }
        if !ids.insert(b.id) {
            return Err(err_mir(
                "MIR012",
                "bloque-id-unico",
                &ctx_f,
                format!("id de bloque duplicado {}", b.id.0),
            ));
        }
    }
    if !ids.contains(&f.bloque_entrada) {
        return Err(err_mir(
            "MIR013",
            "bloque-entrada-existe",
            &ctx_f,
            format!("bloque de entrada {} inexistente", f.bloque_entrada.0),
        ));
    }
    for (local, _) in &f.params {
        if local.0 >= f.locals.len() {
            return Err(err_mir(
                "MIR020",
                "param-local-valido",
                &ctx_f,
                format!(
                    "parámetro referencia local {} fuera de rango (locals={})",
                    local.0,
                    f.locals.len()
                ),
            ));
        }
    }

    // Validación estructural, de tipos y de referencias por bloque/stmt/terminador.
    for b in &f.bloques {
        let ctx_b = format!("funcion '{}', bloque {}", f.nombre, b.id.0);
        match &b.terminator {
            MirTerminator::Goto(t) => {
                if !ids.contains(t) {
                    return Err(err_mir(
                        "MIR030",
                        "terminador-destino-valido",
                        &ctx_b,
                        format!("goto a bloque inexistente {}", t.0),
                    ));
                }
            }
            MirTerminator::SwitchInt {
                discriminante,
                casos,
                por_defecto,
            } => {
                operando_valido(discriminante, f, prog, &ctx_b, "switch.discriminante")?;
                for (valor, blk) in casos {
                    let _ = valor; // valor i64: sin restricción estructural
                    if !ids.contains(blk) {
                        return Err(err_mir(
                            "MIR031",
                            "terminador-destino-valido",
                            &ctx_b,
                            format!("switch a bloque inexistente {}", blk.0),
                        ));
                    }
                }
                if !ids.contains(por_defecto) {
                    return Err(err_mir(
                        "MIR031",
                        "terminador-destino-valido",
                        &ctx_b,
                        format!("switch default a bloque inexistente {}", por_defecto.0),
                    ));
                }
            }
            MirTerminator::Return(op) => {
                if let Some(o) = op {
                    operando_valido(o, f, prog, &ctx_b, "return")?;
                    if let Some(to) = tipo_operando(o, f, prog)
                        && !matches!(f.retorno, TipoSem::Desconocido)
                        && !tipos_compatibles_mir(&f.retorno, &to)
                    {
                        return Err(err_mir(
                            "MIR042",
                            "retorno-tipo",
                            &ctx_b,
                            format!(
                                "return de tipo '{}' incompatible con retorno declarado '{}'",
                                to.nombre(),
                                f.retorno.nombre()
                            ),
                        ));
                    }
                }
                // `Return(None)` en caminos alcanzables se valida tras el análisis
                // de alcanzabilidad (CFG) para evitar falsos positivos en bloques
                // sintéticos/inalcanzables.
            }
            MirTerminator::Invoke {
                callee,
                bloque_ok,
                bloque_error,
                destino,
                argumentos,
                ..
            } => {
                if callee.is_empty() {
                    return Err(err_mir(
                        "MIR043",
                        "callee-valido",
                        &ctx_b,
                        "invoke con callee vacio".to_string(),
                    ));
                }
                if !ids.contains(bloque_ok) || !ids.contains(bloque_error) {
                    return Err(err_mir(
                        "MIR032",
                        "terminador-destino-valido",
                        &ctx_b,
                        format!(
                            "invoke a bloque inexistente (ok={}, err={})",
                            bloque_ok.0, bloque_error.0
                        ),
                    ));
                }
                for (i, a) in argumentos.iter().enumerate() {
                    operando_valido(a, f, prog, &ctx_b, &format!("invoke arg {i}"))?;
                }
                if let Some(callee_fn) = prog.funciones.iter().find(|g| g.nombre == *callee) {
                    if callee_fn.params.len() != argumentos.len() {
                        return Err(err_mir(
                            "MIR044",
                            "invoke-arity",
                            &ctx_b,
                            format!(
                                "invoke '{}': se esperaban {} argumentos, se obtuvieron {}",
                                callee,
                                callee_fn.params.len(),
                                argumentos.len()
                            ),
                        ));
                    }
                    for (i, (arg, (_, pt))) in argumentos.iter().zip(&callee_fn.params).enumerate()
                    {
                        if let Some(at) = tipo_operando(arg, f, prog)
                            && !tipos_compatibles_mir(pt, &at)
                        {
                            return Err(err_mir(
                                "MIR042",
                                "invoke-tipos",
                                &ctx_b,
                                format!(
                                    "invoke '{}': argumento {} de tipo '{}' incompatible con parametro '{}'",
                                    callee,
                                    i,
                                    at.nombre(),
                                    pt.nombre()
                                ),
                            ));
                        }
                    }
                }
                if let Some(d) = destino {
                    lugar_valido(d, f, prog, &ctx_b, "invoke destino")?;
                    if let Some(dt) = tipo_lugar(d, f) {
                        let rt = prog
                            .funciones
                            .iter()
                            .find(|g| g.nombre == *callee)
                            .map(|g| g.retorno.clone())
                            .unwrap_or(TipoSem::Desconocido);
                        if !matches!(rt, TipoSem::Desconocido) && !tipos_compatibles_mir(&dt, &rt) {
                            return Err(err_mir(
                                "MIR042",
                                "invoke-tipos",
                                &ctx_b,
                                format!(
                                    "invoke '{}': destino de tipo '{}' incompatible con retorno '{}'",
                                    callee,
                                    dt.nombre(),
                                    rt.nombre()
                                ),
                            ));
                        }
                    }
                }
            }
            MirTerminator::Unreachable => {}
        }
        for (i, stmt) in b.stmts.iter().enumerate() {
            let ctx_s = format!("{ctx_b}, stmt {i}");
            stmt_valido(stmt, f, prog, &ids, &ctx_s)?;
        }
    }

    // CFG/alcanzabilidad (3D.4): rechaza Return(None) solo en bloques alcanzables
    // cuando el tipo declarado exige valor; bloques inalcanzables se ignoran.
    let alcanzables = bloques_alcanzables(f, &ids);
    if retorno_requiere_valor(&f.retorno) {
        for b in &f.bloques {
            if alcanzables.contains(&b.id) && matches!(b.terminator, MirTerminator::Return(None)) {
                let ctx_b = format!("funcion '{}', bloque {}", f.nombre, b.id.0);
                return Err(err_mir(
                    "MIR045",
                    "retorno-requerido",
                    &ctx_b,
                    format!(
                        "retorno vacío en camino alcanzable pero la función declara retorno '{}'",
                        f.retorno.nombre()
                    ),
                ));
            }
        }
    }

    Ok(())
}

#[allow(clippy::collapsible_if)]
fn lugar_valido(
    p: &MirPlace,
    f: &MirFunction,
    prog: &MirPrograma,
    ctx: &str,
    sitio: &str,
) -> Result<(), ErrorVerificacionMir> {
    if p.local.0 >= f.locals.len() {
        return Err(err_mir(
            "MIR040",
            "local-valido",
            ctx,
            format!(
                "{sitio}: local {} fuera de rango (locals={})",
                p.local.0,
                f.locals.len()
            ),
        ));
    }
    for proj in &p.projection {
        match proj {
            MirProjection::Campo(_) => {}
            MirProjection::Indice(op) => {
                operando_valido(op, f, prog, ctx, &format!("{sitio}/indice"))?;
                if let Some(ti) = tipo_operando(op, f, prog)
                    && !matches!(ti, TipoSem::Entero | TipoSem::Desconocido)
                {
                    return Err(err_mir(
                        "MIR042",
                        "proyeccion-indice-tipo",
                        ctx,
                        format!(
                            "{sitio}/indice: se esperaba entero, se obtuvo '{}'",
                            ti.nombre()
                        ),
                    ));
                }
            }
            MirProjection::Celda => {}
        }
    }
    Ok(())
}

fn operando_valido(
    op: &MirOperand,
    f: &MirFunction,
    prog: &MirPrograma,
    ctx: &str,
    sitio: &str,
) -> Result<(), ErrorVerificacionMir> {
    match op {
        MirOperand::Copy(l) | MirOperand::Move(l) => {
            if l.0 >= f.locals.len() {
                Err(err_mir(
                    "MIR040",
                    "local-valido",
                    ctx,
                    format!(
                        "{sitio}: local {} fuera de rango (locals={})",
                        l.0,
                        f.locals.len()
                    ),
                ))
            } else {
                Ok(())
            }
        }
        MirOperand::Const(_) => Ok(()),
        MirOperand::Global(g) => {
            if g.0 >= prog.globals.len() {
                Err(err_mir(
                    "MIR041",
                    "global-valido",
                    ctx,
                    format!(
                        "{sitio}: global {} fuera de rango (globals={})",
                        g.0,
                        prog.globals.len()
                    ),
                ))
            } else {
                Ok(())
            }
        }
    }
}

fn stmt_valido(
    stmt: &MirStmt,
    f: &MirFunction,
    prog: &MirPrograma,
    ids: &std::collections::HashSet<BlockId>,
    ctx: &str,
) -> Result<(), ErrorVerificacionMir> {
    match stmt {
        MirStmt::Asignacion { destino, valor } => {
            lugar_valido(destino, f, prog, ctx, "asignacion.destino")?;
            rvalue_valido(valor, f, prog, ctx, "asignacion.valor")?;
            // Compatibilidad de tipos cuando ambos son conocidos.
            if let (Some(td), Some(tr)) = (tipo_lugar(destino, f), tipo_rvalue(valor, f, prog))
                && !tipos_compatibles_mir(&td, &tr)
            {
                return Err(err_mir(
                    "MIR042",
                    "asignacion-tipos",
                    ctx,
                    format!(
                        "asignacion: destino de tipo '{}' incompatible con valor '{}'",
                        td.nombre(),
                        tr.nombre()
                    ),
                ));
            }
            Ok(())
        }
        MirStmt::AsignacionGlobal { global, valor } => {
            if global.0 >= prog.globals.len() {
                return Err(err_mir(
                    "MIR041",
                    "global-valido",
                    ctx,
                    format!(
                        "asignacion global {} fuera de rango (globals={})",
                        global.0,
                        prog.globals.len()
                    ),
                ));
            }
            rvalue_valido(valor, f, prog, ctx, "asignacion_global.valor")?;
            if let (Some(tg), Some(tr)) = (
                prog.globals.get(global.0).cloned(),
                tipo_rvalue(valor, f, prog),
            ) {
                // Los tipos sintéticos (Funcion) de módulos no se validan aquí.
                if !matches!(tg, TipoSem::Funcion { .. } | TipoSem::Desconocido)
                    && !tipos_compatibles_mir(&tg, &tr)
                {
                    return Err(err_mir(
                        "MIR042",
                        "asignacion-tipos",
                        ctx,
                        format!(
                            "asignacion global: tipo '{}' incompatible con valor '{}'",
                            tg.nombre(),
                            tr.nombre()
                        ),
                    ));
                }
            }
            Ok(())
        }
        MirStmt::Llamada {
            destino,
            callee,
            argumentos,
        } => {
            if callee.is_empty() {
                return Err(err_mir(
                    "MIR043",
                    "callee-valido",
                    ctx,
                    "llamada con callee vacio".to_string(),
                ));
            }
            if let Some(d) = destino {
                lugar_valido(d, f, prog, ctx, "llamada.destino")?;
            }
            for (i, a) in argumentos.iter().enumerate() {
                operando_valido(a, f, prog, ctx, &format!("llamada.arg[{i}]"))?;
            }
            // Validar argumentos/parametros cuando la funcion existe en el programa.
            if let Some(callee_fn) = prog.funciones.iter().find(|g| g.nombre == *callee) {
                if callee_fn.params.len() != argumentos.len() {
                    return Err(err_mir(
                        "MIR044",
                        "llamada-arity",
                        ctx,
                        format!(
                            "llamada '{}': se esperaban {} argumentos, se obtuvieron {}",
                            callee,
                            callee_fn.params.len(),
                            argumentos.len()
                        ),
                    ));
                }
                for (i, (arg, (_, pt))) in argumentos.iter().zip(&callee_fn.params).enumerate() {
                    if let Some(at) = tipo_operando(arg, f, prog)
                        && !tipos_compatibles_mir(pt, &at)
                    {
                        return Err(err_mir(
                            "MIR042",
                            "llamada-tipos",
                            ctx,
                            format!(
                                "llamada '{}': argumento {} de tipo '{}' incompatible con parametro '{}'",
                                callee,
                                i,
                                at.nombre(),
                                pt.nombre()
                            ),
                        ));
                    }
                }
            }
            // Validar tipo de destino si es posible.
            if let Some(d) = destino
                && let Some(dt) = tipo_lugar(d, f)
            {
                // El tipo de retorno de la funcion, si existe.
                let rt = prog
                    .funciones
                    .iter()
                    .find(|g| g.nombre == *callee)
                    .map(|g| g.retorno.clone())
                    .unwrap_or(TipoSem::Desconocido);
                if !matches!(rt, TipoSem::Desconocido) && !tipos_compatibles_mir(&dt, &rt) {
                    return Err(err_mir(
                        "MIR042",
                        "llamada-tipos",
                        ctx,
                        format!(
                            "llamada '{}': destino de tipo '{}' incompatible con retorno '{}'",
                            callee,
                            dt.nombre(),
                            rt.nombre()
                        ),
                    ));
                }
            }
            Ok(())
        }
        MirStmt::LlamadaIndirecta {
            destino,
            callee,
            argumentos,
        } => {
            if let Some(d) = destino {
                lugar_valido(d, f, prog, ctx, "llamada_indirecta.destino")?;
            }
            operando_valido(callee, f, prog, ctx, "llamada_indirecta.callee")?;
            for (i, a) in argumentos.iter().enumerate() {
                operando_valido(a, f, prog, ctx, &format!("llamada_indirecta.arg[{i}]"))?;
            }
            Ok(())
        }
        MirStmt::EntrarTry {
            rescue,
            merge,
            var_local,
        } => {
            // D3 (Fase 3B): rescue/merge deben existir en la función.
            if !ids.contains(rescue) {
                return Err(err_mir(
                    "MIR033",
                    "try-destino-valido",
                    ctx,
                    format!("entrar_try.rescue a bloque inexistente {}", rescue.0),
                ));
            }
            if !ids.contains(merge) {
                return Err(err_mir(
                    "MIR033",
                    "try-destino-valido",
                    ctx,
                    format!("entrar_try.merge a bloque inexistente {}", merge.0),
                ));
            }
            if let Some(vl) = var_local
                && *vl >= f.locals.len()
            {
                return Err(err_mir(
                    "MIR040",
                    "local-valido",
                    ctx,
                    format!(
                        "entrar_try.var_local {} fuera de rango (locals={})",
                        vl,
                        f.locals.len()
                    ),
                ));
            }
            Ok(())
        }
        MirStmt::SalirTry | MirStmt::Nop => Ok(()),
    }
}

fn rvalue_valido(
    r: &MirRvalue,
    f: &MirFunction,
    prog: &MirPrograma,
    ctx: &str,
    sitio: &str,
) -> Result<(), ErrorVerificacionMir> {
    match r {
        MirRvalue::Use(op) => operando_valido(op, f, prog, ctx, sitio),
        MirRvalue::BinOp { op, lhs, rhs } => {
            operando_valido(lhs, f, prog, ctx, &format!("{sitio}.lhs"))?;
            operando_valido(rhs, f, prog, ctx, &format!("{sitio}.rhs"))?;
            // Validación de tipos (conservadora, espejo de semántica).
            if let (Some(tl), Some(tr)) = (tipo_operando(lhs, f, prog), tipo_operando(rhs, f, prog))
            {
                // Si algún tipo es Desconocido, no rechazar (conservador).
                if matches!(tl, TipoSem::Desconocido) || matches!(tr, TipoSem::Desconocido) {
                    return Ok(());
                }
                let ok = match op {
                    MirBinOp::Suma => {
                        tipos_numericos_compatibles_mir(&tl, &tr)
                            || (tl == TipoSem::Texto && tr == TipoSem::Texto)
                    }
                    MirBinOp::Resta
                    | MirBinOp::Multiplicacion
                    | MirBinOp::Division
                    | MirBinOp::Modulo => tipos_numericos_compatibles_mir(&tl, &tr),
                    MirBinOp::Igual
                    | MirBinOp::Distinto
                    | MirBinOp::Menor
                    | MirBinOp::MenorIgual
                    | MirBinOp::Mayor
                    | MirBinOp::MayorIgual => tipos_numericos_compatibles_mir(&tl, &tr) || tl == tr,
                    MirBinOp::Y | MirBinOp::O => tl == TipoSem::Logico && tr == TipoSem::Logico,
                };
                if !ok {
                    return Err(err_mir(
                        "MIR042",
                        "rvalue-tipos-compatibles",
                        ctx,
                        format!(
                            "{sitio}: tipos incompatibles '{}' vs '{}'",
                            tl.nombre(),
                            tr.nombre()
                        ),
                    ));
                }
            }
            Ok(())
        }
        MirRvalue::UnOp { op, operand } => {
            operando_valido(operand, f, prog, ctx, &format!("{sitio}.operand"))?;
            if let Some(t) = tipo_operando(operand, f, prog) {
                let ok = match op {
                    MirUnOp::Negacion => {
                        matches!(t, TipoSem::Entero | TipoSem::Decimal | TipoSem::Real)
                    }
                    MirUnOp::No => t == TipoSem::Logico,
                };
                if !ok {
                    return Err(err_mir(
                        "MIR042",
                        "rvalue-tipos-compatibles",
                        ctx,
                        format!("{sitio}: tipo incompatible para operador unario"),
                    ));
                }
            }
            Ok(())
        }
        MirRvalue::Agregado { elementos } => {
            for (i, e) in elementos.iter().enumerate() {
                operando_valido(e, f, prog, ctx, &format!("{sitio}[{i}]"))?;
            }
            Ok(())
        }
        MirRvalue::Indice { base, indice } => {
            operando_valido(base, f, prog, ctx, &format!("{sitio}.base"))?;
            operando_valido(indice, f, prog, ctx, &format!("{sitio}.indice"))?;
            if let Some(ti) = tipo_operando(indice, f, prog)
                && !matches!(ti, TipoSem::Entero | TipoSem::Desconocido)
            {
                return Err(err_mir(
                    "MIR042",
                    "rvalue-tipos-compatibles",
                    ctx,
                    format!(
                        "{sitio}: indice debe ser entero, se obtuvo '{}'",
                        ti.nombre()
                    ),
                ));
            }
            Ok(())
        }
        MirRvalue::Campo { base, .. } => {
            operando_valido(base, f, prog, ctx, &format!("{sitio}.base"))
        }
        MirRvalue::CrearCelda { valor } => {
            operando_valido(valor, f, prog, ctx, &format!("{sitio}.valor"))
        }
        MirRvalue::LeerCelda { base } => {
            operando_valido(base, f, prog, ctx, &format!("{sitio}.base"))
        }
        MirRvalue::Clausura { capturas, .. } => {
            for (i, c) in capturas.iter().enumerate() {
                operando_valido(c, f, prog, ctx, &format!("{sitio}.captura[{i}]"))?;
            }
            Ok(())
        }
        MirRvalue::Rango { inicio, fin, .. } => {
            operando_valido(inicio, f, prog, ctx, &format!("{sitio}.inicio"))?;
            operando_valido(fin, f, prog, ctx, &format!("{sitio}.fin"))
        }
        MirRvalue::Longitud { base } => {
            operando_valido(base, f, prog, ctx, &format!("{sitio}.base"))
        }
        MirRvalue::ObjetoModulo { .. } => Ok(()),
    }
}

#[cfg(test)]
mod verificacion_3b_tests {
    use super::*;
    use crate::semantico::TipoSem;

    fn func_basica(bloques: Vec<MirBasicBlock>, locals: Vec<TipoSem>) -> MirFunction {
        MirFunction {
            nombre: "f".to_string(),
            params: vec![],
            retorno: TipoSem::Nulo,
            locals,
            bloque_entrada: BlockId(0),
            bloques,
        }
    }

    fn prog(f: MirFunction) -> MirPrograma {
        MirPrograma {
            funciones: vec![f],
            globals: vec![],
        }
    }

    fn bb(id: usize, stmts: Vec<MirStmt>, terminator: MirTerminator) -> MirBasicBlock {
        MirBasicBlock {
            id: BlockId(id),
            stmts,
            terminator,
        }
    }

    // D2 positivo
    #[test]
    fn d2_switchint_discriminante_y_destinos_validos() {
        let f = func_basica(
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::SwitchInt {
                        discriminante: MirOperand::Copy(LocalId(0)),
                        casos: vec![(1, BlockId(1))],
                        por_defecto: BlockId(2),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
                bb(2, vec![], MirTerminator::Return(None)),
            ],
            vec![TipoSem::Entero],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(r.es_ok(), "{r:?}");
    }

    // D2 negativo: discriminante invalido
    #[test]
    fn d2_switchint_discriminante_local_invalido_rechaza_mir040() {
        let f = func_basica(
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::SwitchInt {
                        discriminante: MirOperand::Copy(LocalId(99)),
                        casos: vec![(1, BlockId(1))],
                        por_defecto: BlockId(1),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
            vec![TipoSem::Entero],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR040");
        assert!(r.errores[0].mensaje.contains("switch.discriminante"));
    }

    // D2 negativo: destino invalido (existente, sigue MIR031)
    #[test]
    fn d2_switchint_destino_invalido_rechaza_mir031() {
        let f = func_basica(
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::SwitchInt {
                        discriminante: MirOperand::Copy(LocalId(0)),
                        casos: vec![(1, BlockId(9))],
                        por_defecto: BlockId(1),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
            vec![TipoSem::Entero],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR031");
    }

    // D3 positivo
    #[test]
    fn d3_entrar_try_rescue_merge_validos() {
        let f = func_basica(
            vec![
                bb(
                    0,
                    vec![MirStmt::EntrarTry {
                        rescue: BlockId(1),
                        merge: BlockId(2),
                        var_local: Some(0),
                    }],
                    MirTerminator::Goto(BlockId(2)),
                ),
                bb(1, vec![MirStmt::SalirTry], MirTerminator::Goto(BlockId(2))),
                bb(2, vec![], MirTerminator::Return(None)),
            ],
            vec![TipoSem::Texto],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(r.es_ok(), "{r:?}");
    }

    // D3 negativos
    #[test]
    fn d3_entrar_try_rescue_invalido_rechaza_mir033() {
        let f = func_basica(
            vec![
                bb(
                    0,
                    vec![MirStmt::EntrarTry {
                        rescue: BlockId(99),
                        merge: BlockId(1),
                        var_local: None,
                    }],
                    MirTerminator::Goto(BlockId(1)),
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
            vec![],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR033");
        assert!(r.errores[0].mensaje.contains("rescue"));
    }

    #[test]
    fn d3_entrar_try_merge_invalido_rechaza_mir033() {
        let f = func_basica(
            vec![
                bb(
                    0,
                    vec![MirStmt::EntrarTry {
                        rescue: BlockId(1),
                        merge: BlockId(99),
                        var_local: None,
                    }],
                    MirTerminator::Goto(BlockId(1)),
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
            vec![],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR033");
        assert!(r.errores[0].mensaje.contains("merge"));
    }

    #[test]
    fn d3_entrar_try_var_local_invalido_rechaza_mir040() {
        let f = func_basica(
            vec![
                bb(
                    0,
                    vec![MirStmt::EntrarTry {
                        rescue: BlockId(1),
                        merge: BlockId(1),
                        var_local: Some(5),
                    }],
                    MirTerminator::Goto(BlockId(1)),
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
            vec![],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR040");
    }

    // Compat bool preservada
    #[test]
    fn compat_bool_mir_preservada() {
        let f = func_basica(vec![bb(0, vec![], MirTerminator::Return(None))], vec![]);
        assert!(VerificadorMir::verificar(&prog(f)));
    }

    // Auditoría D2/D3 complementaria
    #[test]
    fn d2_switchint_discriminante_global_valido() {
        let mut f = func_basica(
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::SwitchInt {
                        discriminante: MirOperand::Global(GlobalId(0)),
                        casos: vec![(1, BlockId(1))],
                        por_defecto: BlockId(1),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
            vec![],
        );
        let _ = &mut f;
        let mut p = prog(f);
        p.globals = vec![TipoSem::Entero];
        assert!(VerificadorMir::verificar_detallado(&p).es_ok());
    }

    #[test]
    fn d2_switchint_discriminante_global_invalido_rechaza_mir041() {
        let f = func_basica(
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::SwitchInt {
                        discriminante: MirOperand::Global(GlobalId(9)),
                        casos: vec![(1, BlockId(1))],
                        por_defecto: BlockId(1),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
            vec![],
        );
        let mut p = prog(f);
        p.globals = vec![TipoSem::Entero];
        let r = VerificadorMir::verificar_detallado(&p);
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR041");
    }

    #[test]
    fn d2_proyeccion_indice_valida_en_asignacion() {
        let f = func_basica(
            vec![bb(
                0,
                vec![MirStmt::Asignacion {
                    destino: MirPlace {
                        local: LocalId(0),
                        projection: vec![MirProjection::Indice(MirOperand::Copy(LocalId(1)))],
                    },
                    valor: MirRvalue::Use(MirOperand::Const(MirConst::Entero(1))),
                }],
                MirTerminator::Return(None),
            )],
            vec![TipoSem::Desconocido, TipoSem::Entero],
        );
        assert!(VerificadorMir::verificar_detallado(&prog(f)).es_ok());
    }

    #[test]
    fn d2_proyeccion_indice_invalido_rechaza_mir040() {
        let f = func_basica(
            vec![bb(
                0,
                vec![MirStmt::Asignacion {
                    destino: MirPlace {
                        local: LocalId(0),
                        projection: vec![MirProjection::Indice(MirOperand::Copy(LocalId(99)))],
                    },
                    valor: MirRvalue::Use(MirOperand::Const(MirConst::Entero(1))),
                }],
                MirTerminator::Return(None),
            )],
            vec![TipoSem::Desconocido],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR040");
    }

    #[test]
    fn d3_entrar_try_var_local_en_rango_acepta() {
        let f = func_basica(
            vec![
                bb(
                    0,
                    vec![MirStmt::EntrarTry {
                        rescue: BlockId(1),
                        merge: BlockId(1),
                        var_local: Some(1),
                    }],
                    MirTerminator::Goto(BlockId(1)),
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
            vec![TipoSem::Texto, TipoSem::Texto],
        );
        assert!(VerificadorMir::verificar_detallado(&prog(f)).es_ok());
    }
}

#[cfg(test)]
mod verificacion_3d3_tests {
    use super::*;

    fn bb(id: usize, stmts: Vec<MirStmt>, terminator: MirTerminator) -> MirBasicBlock {
        MirBasicBlock {
            id: BlockId(id),
            stmts,
            terminator,
        }
    }

    fn func(
        nombre: &str,
        params: Vec<(LocalId, TipoSem)>,
        retorno: TipoSem,
        locals: Vec<TipoSem>,
        bloques: Vec<MirBasicBlock>,
    ) -> MirFunction {
        MirFunction {
            nombre: nombre.to_string(),
            params,
            retorno,
            locals,
            bloque_entrada: BlockId(0),
            bloques,
        }
    }

    fn programa(funciones: Vec<MirFunction>, globals: Vec<TipoSem>) -> MirPrograma {
        MirPrograma { funciones, globals }
    }

    fn main_func(
        retorno: TipoSem,
        locals: Vec<TipoSem>,
        bloques: Vec<MirBasicBlock>,
    ) -> MirFunction {
        func("main", vec![], retorno, locals, bloques)
    }

    // --- RValues y operandos: compatibles / incompatibles ---

    #[test]
    fn d3_3_binop_suma_enteros_acepta() {
        let f = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Entero, TipoSem::Entero, TipoSem::Entero],
            vec![bb(
                0,
                vec![MirStmt::Asignacion {
                    destino: MirPlace {
                        local: LocalId(2),
                        projection: vec![],
                    },
                    valor: MirRvalue::BinOp {
                        op: MirBinOp::Suma,
                        lhs: MirOperand::Copy(LocalId(0)),
                        rhs: MirOperand::Copy(LocalId(1)),
                    },
                }],
                MirTerminator::Return(None),
            )],
        );
        assert!(VerificadorMir::verificar_detallado(&programa(vec![f], vec![])).es_ok());
    }

    #[test]
    fn d3_3_binop_suma_entero_texto_rechaza() {
        let f = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Entero, TipoSem::Texto, TipoSem::Desconocido],
            vec![bb(
                0,
                vec![MirStmt::Asignacion {
                    destino: MirPlace {
                        local: LocalId(2),
                        projection: vec![],
                    },
                    valor: MirRvalue::BinOp {
                        op: MirBinOp::Suma,
                        lhs: MirOperand::Copy(LocalId(0)),
                        rhs: MirOperand::Copy(LocalId(1)),
                    },
                }],
                MirTerminator::Return(None),
            )],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![f], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR042");
    }

    #[test]
    fn d3_3_binop_logico_y_enteros_rechaza() {
        let f = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Entero, TipoSem::Entero, TipoSem::Desconocido],
            vec![bb(
                0,
                vec![MirStmt::Asignacion {
                    destino: MirPlace {
                        local: LocalId(2),
                        projection: vec![],
                    },
                    valor: MirRvalue::BinOp {
                        op: MirBinOp::Y,
                        lhs: MirOperand::Copy(LocalId(0)),
                        rhs: MirOperand::Copy(LocalId(1)),
                    },
                }],
                MirTerminator::Return(None),
            )],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![f], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR042");
    }

    #[test]
    fn d3_3_unop_negacion_texto_rechaza() {
        let f = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Texto, TipoSem::Desconocido],
            vec![bb(
                0,
                vec![MirStmt::Asignacion {
                    destino: MirPlace {
                        local: LocalId(1),
                        projection: vec![],
                    },
                    valor: MirRvalue::UnOp {
                        op: MirUnOp::Negacion,
                        operand: MirOperand::Copy(LocalId(0)),
                    },
                }],
                MirTerminator::Return(None),
            )],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![f], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR042");
    }

    #[test]
    fn d3_3_unop_no_logico_acepta() {
        let f = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Logico, TipoSem::Logico],
            vec![bb(
                0,
                vec![MirStmt::Asignacion {
                    destino: MirPlace {
                        local: LocalId(1),
                        projection: vec![],
                    },
                    valor: MirRvalue::UnOp {
                        op: MirUnOp::No,
                        operand: MirOperand::Copy(LocalId(0)),
                    },
                }],
                MirTerminator::Return(None),
            )],
        );
        assert!(VerificadorMir::verificar_detallado(&programa(vec![f], vec![])).es_ok());
    }

    // --- Locales y globales ---

    #[test]
    fn d3_3_asignacion_local_tipo_incompatible_rechaza() {
        let f = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Entero],
            vec![bb(
                0,
                vec![MirStmt::Asignacion {
                    destino: MirPlace {
                        local: LocalId(0),
                        projection: vec![],
                    },
                    valor: MirRvalue::Use(MirOperand::Const(MirConst::Texto("x".into()))),
                }],
                MirTerminator::Return(None),
            )],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![f], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR042");
    }

    #[test]
    fn d3_3_asignacion_local_tipo_compatible_acepta() {
        let f = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Entero],
            vec![bb(
                0,
                vec![MirStmt::Asignacion {
                    destino: MirPlace {
                        local: LocalId(0),
                        projection: vec![],
                    },
                    valor: MirRvalue::Use(MirOperand::Const(MirConst::Entero(1))),
                }],
                MirTerminator::Return(None),
            )],
        );
        assert!(VerificadorMir::verificar_detallado(&programa(vec![f], vec![])).es_ok());
    }

    #[test]
    fn d3_3_asignacion_global_tipo_incompatible_rechaza() {
        let f = main_func(
            TipoSem::Nulo,
            vec![],
            vec![bb(
                0,
                vec![MirStmt::AsignacionGlobal {
                    global: GlobalId(0),
                    valor: MirRvalue::Use(MirOperand::Const(MirConst::Texto("x".into()))),
                }],
                MirTerminator::Return(None),
            )],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![f], vec![TipoSem::Entero]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR042");
    }

    #[test]
    fn d3_3_asignacion_global_tipo_compatible_acepta() {
        let f = main_func(
            TipoSem::Nulo,
            vec![],
            vec![bb(
                0,
                vec![MirStmt::AsignacionGlobal {
                    global: GlobalId(0),
                    valor: MirRvalue::Use(MirOperand::Const(MirConst::Entero(1))),
                }],
                MirTerminator::Return(None),
            )],
        );
        assert!(
            VerificadorMir::verificar_detallado(&programa(vec![f], vec![TipoSem::Entero])).es_ok()
        );
    }

    // --- Proyecciones ---

    #[test]
    fn d3_3_proyeccion_indice_no_entero_rechaza() {
        let f = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Desconocido, TipoSem::Texto],
            vec![bb(
                0,
                vec![MirStmt::Asignacion {
                    destino: MirPlace {
                        local: LocalId(0),
                        projection: vec![MirProjection::Indice(MirOperand::Copy(LocalId(1)))],
                    },
                    valor: MirRvalue::Use(MirOperand::Const(MirConst::Entero(1))),
                }],
                MirTerminator::Return(None),
            )],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![f], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR042");
    }

    // --- Retornos ---

    #[test]
    fn d3_3_retorno_tipo_compatible_acepta() {
        let f = main_func(
            TipoSem::Entero,
            vec![TipoSem::Entero],
            vec![bb(
                0,
                vec![],
                MirTerminator::Return(Some(MirOperand::Copy(LocalId(0)))),
            )],
        );
        assert!(VerificadorMir::verificar_detallado(&programa(vec![f], vec![])).es_ok());
    }

    #[test]
    fn d3_3_retorno_tipo_incompatible_rechaza() {
        let f = main_func(
            TipoSem::Entero,
            vec![TipoSem::Texto],
            vec![bb(
                0,
                vec![],
                MirTerminator::Return(Some(MirOperand::Copy(LocalId(0)))),
            )],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![f], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR042");
    }

    #[test]
    fn d3_3_retorno_vacio_funcion_nulo_acepta() {
        let f = main_func(
            TipoSem::Nulo,
            vec![],
            vec![bb(0, vec![], MirTerminator::Return(None))],
        );
        assert!(VerificadorMir::verificar_detallado(&programa(vec![f], vec![])).es_ok());
    }

    // --- Llamadas directas: existentes/inexistentes, aridad/tipos ---

    #[test]
    fn d3_3_llamada_directa_funcion_existente_ok() {
        let callee = func(
            "sumar",
            vec![(LocalId(0), TipoSem::Entero), (LocalId(1), TipoSem::Entero)],
            TipoSem::Entero,
            vec![TipoSem::Entero, TipoSem::Entero],
            vec![bb(
                0,
                vec![],
                MirTerminator::Return(Some(MirOperand::Copy(LocalId(0)))),
            )],
        );
        let main = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Entero, TipoSem::Entero, TipoSem::Entero],
            vec![bb(
                0,
                vec![MirStmt::Llamada {
                    destino: Some(MirPlace {
                        local: LocalId(2),
                        projection: vec![],
                    }),
                    callee: "sumar".into(),
                    argumentos: vec![MirOperand::Copy(LocalId(0)), MirOperand::Copy(LocalId(1))],
                }],
                MirTerminator::Return(None),
            )],
        );
        assert!(VerificadorMir::verificar_detallado(&programa(vec![main, callee], vec![])).es_ok());
    }

    #[test]
    fn d3_3_llamada_directa_arity_incorrecta_rechaza() {
        let callee = func(
            "sumar",
            vec![(LocalId(0), TipoSem::Entero)],
            TipoSem::Entero,
            vec![TipoSem::Entero],
            vec![bb(
                0,
                vec![],
                MirTerminator::Return(Some(MirOperand::Copy(LocalId(0)))),
            )],
        );
        let main = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Entero, TipoSem::Entero],
            vec![bb(
                0,
                vec![MirStmt::Llamada {
                    destino: None,
                    callee: "sumar".into(),
                    argumentos: vec![MirOperand::Copy(LocalId(0)), MirOperand::Copy(LocalId(0))],
                }],
                MirTerminator::Return(None),
            )],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![main, callee], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR044");
    }

    #[test]
    fn d3_3_llamada_directa_arg_tipo_incompatible_rechaza() {
        let callee = func(
            "sumar",
            vec![(LocalId(0), TipoSem::Entero)],
            TipoSem::Entero,
            vec![TipoSem::Entero],
            vec![bb(
                0,
                vec![],
                MirTerminator::Return(Some(MirOperand::Copy(LocalId(0)))),
            )],
        );
        let main = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Texto],
            vec![bb(
                0,
                vec![MirStmt::Llamada {
                    destino: None,
                    callee: "sumar".into(),
                    argumentos: vec![MirOperand::Copy(LocalId(0))],
                }],
                MirTerminator::Return(None),
            )],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![main, callee], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR042");
    }

    #[test]
    fn d3_3_llamada_directa_funcion_inexistente_en_programa_acepta_conservador() {
        // Callee no resuelto en el programa MIR (p.ej. nativo/externo): conservador, no se rechaza.
        let main = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Entero],
            vec![bb(
                0,
                vec![MirStmt::Llamada {
                    destino: None,
                    callee: "nativo_externo".into(),
                    argumentos: vec![MirOperand::Copy(LocalId(0))],
                }],
                MirTerminator::Return(None),
            )],
        );
        assert!(VerificadorMir::verificar_detallado(&programa(vec![main], vec![])).es_ok());
    }

    // --- Llamadas indirectas ---

    #[test]
    fn d3_3_llamada_indirecta_operandos_validos_acepta() {
        let main = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Desconocido, TipoSem::Entero],
            vec![bb(
                0,
                vec![MirStmt::LlamadaIndirecta {
                    destino: None,
                    callee: MirOperand::Copy(LocalId(0)),
                    argumentos: vec![MirOperand::Copy(LocalId(1))],
                }],
                MirTerminator::Return(None),
            )],
        );
        assert!(VerificadorMir::verificar_detallado(&programa(vec![main], vec![])).es_ok());
    }

    // --- Invoke y argumentos ---

    #[test]
    fn d3_3_invoke_funcion_existente_args_ok() {
        let callee = func(
            "puede_fallar",
            vec![(LocalId(0), TipoSem::Entero)],
            TipoSem::Entero,
            vec![TipoSem::Entero],
            vec![bb(
                0,
                vec![],
                MirTerminator::Return(Some(MirOperand::Copy(LocalId(0)))),
            )],
        );
        let main = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Entero, TipoSem::Entero],
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::Invoke {
                        callee: "puede_fallar".into(),
                        argumentos: vec![MirOperand::Copy(LocalId(0))],
                        destino: Some(MirPlace {
                            local: LocalId(1),
                            projection: vec![],
                        }),
                        bloque_ok: BlockId(1),
                        bloque_error: BlockId(2),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
                bb(2, vec![], MirTerminator::Return(None)),
            ],
        );
        assert!(VerificadorMir::verificar_detallado(&programa(vec![main, callee], vec![])).es_ok());
    }

    #[test]
    fn d3_3_invoke_arg_tipo_incompatible_rechaza() {
        let callee = func(
            "puede_fallar",
            vec![(LocalId(0), TipoSem::Entero)],
            TipoSem::Entero,
            vec![TipoSem::Entero],
            vec![bb(
                0,
                vec![],
                MirTerminator::Return(Some(MirOperand::Copy(LocalId(0)))),
            )],
        );
        let main = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Texto, TipoSem::Entero],
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::Invoke {
                        callee: "puede_fallar".into(),
                        argumentos: vec![MirOperand::Copy(LocalId(0))],
                        destino: Some(MirPlace {
                            local: LocalId(1),
                            projection: vec![],
                        }),
                        bloque_ok: BlockId(1),
                        bloque_error: BlockId(2),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
                bb(2, vec![], MirTerminator::Return(None)),
            ],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![main, callee], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR042");
    }

    #[test]
    fn d3_3_invoke_arity_incorrecta_rechaza() {
        let callee = func(
            "puede_fallar",
            vec![(LocalId(0), TipoSem::Entero)],
            TipoSem::Entero,
            vec![TipoSem::Entero],
            vec![bb(
                0,
                vec![],
                MirTerminator::Return(Some(MirOperand::Copy(LocalId(0)))),
            )],
        );
        let main = main_func(
            TipoSem::Nulo,
            vec![TipoSem::Entero],
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::Invoke {
                        callee: "puede_fallar".into(),
                        argumentos: vec![],
                        destino: None,
                        bloque_ok: BlockId(1),
                        bloque_error: BlockId(2),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
                bb(2, vec![], MirTerminator::Return(None)),
            ],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![main, callee], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR044");
    }

    #[test]
    fn d3_3_invoke_callee_vacio_rechaza() {
        let main = main_func(
            TipoSem::Nulo,
            vec![],
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::Invoke {
                        callee: "".into(),
                        argumentos: vec![],
                        destino: None,
                        bloque_ok: BlockId(1),
                        bloque_error: BlockId(2),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
                bb(2, vec![], MirTerminator::Return(None)),
            ],
        );
        let r = VerificadorMir::verificar_detallado(&programa(vec![main], vec![]));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR043");
    }
}

#[cfg(test)]
mod verificacion_3d4_tests {
    use super::*;

    fn bb(id: usize, stmts: Vec<MirStmt>, terminator: MirTerminator) -> MirBasicBlock {
        MirBasicBlock {
            id: BlockId(id),
            stmts,
            terminator,
        }
    }
    fn func(
        nombre: &str,
        retorno: TipoSem,
        locals: Vec<TipoSem>,
        bloques: Vec<MirBasicBlock>,
    ) -> MirFunction {
        MirFunction {
            nombre: nombre.into(),
            params: vec![],
            retorno,
            locals,
            bloque_entrada: BlockId(0),
            bloques,
        }
    }
    fn prog(f: MirFunction) -> MirPrograma {
        MirPrograma {
            funciones: vec![f],
            globals: vec![],
        }
    }

    #[test]
    fn cfg_bloque_entrada_inexistente_rechaza() {
        let f = MirFunction {
            nombre: "f".into(),
            params: vec![],
            retorno: TipoSem::Nulo,
            locals: vec![],
            bloque_entrada: BlockId(5),
            bloques: vec![bb(0, vec![], MirTerminator::Return(None))],
        };
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR013");
    }

    #[test]
    fn cfg_goto_destino_inexistente_rechaza() {
        let f = func(
            "f",
            TipoSem::Nulo,
            vec![],
            vec![bb(0, vec![], MirTerminator::Goto(BlockId(9)))],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR030");
    }

    #[test]
    fn cfg_switch_destinos_invalidos_rechaza() {
        let f = func(
            "f",
            TipoSem::Nulo,
            vec![TipoSem::Entero],
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::SwitchInt {
                        discriminante: MirOperand::Copy(LocalId(0)),
                        casos: vec![(1, BlockId(2))],
                        por_defecto: BlockId(1),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR031");
    }

    #[test]
    fn cfg_invoke_destinos_invalidos_rechaza() {
        let f = func(
            "f",
            TipoSem::Nulo,
            vec![],
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::Invoke {
                        callee: "g".into(),
                        argumentos: vec![],
                        destino: None,
                        bloque_ok: BlockId(1),
                        bloque_error: BlockId(9),
                    },
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR032");
    }

    #[test]
    fn cfg_unreachable_valido_en_bloque_alcanzable_estructura_ok() {
        // Unreachable es terminador válido estructuralmente.
        let f = func(
            "f",
            TipoSem::Nulo,
            vec![],
            vec![bb(0, vec![], MirTerminator::Unreachable)],
        );
        assert!(VerificadorMir::verificar_detallado(&prog(f)).es_ok());
    }

    #[test]
    fn cfg_return_none_alcanzable_rechaza_si_retorno_tipado() {
        let f = func(
            "f",
            TipoSem::Entero,
            vec![],
            vec![bb(0, vec![], MirTerminator::Return(None))],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR045");
    }

    #[test]
    fn cfg_return_none_alcanzable_permitido_si_retorno_nulo() {
        let f = func(
            "f",
            TipoSem::Nulo,
            vec![],
            vec![bb(0, vec![], MirTerminator::Return(None))],
        );
        assert!(VerificadorMir::verificar_detallado(&prog(f)).es_ok());
    }

    #[test]
    fn cfg_return_none_alcanzable_permitido_si_retorno_desconocido() {
        let f = func(
            "f",
            TipoSem::Desconocido,
            vec![],
            vec![bb(0, vec![], MirTerminator::Return(None))],
        );
        assert!(VerificadorMir::verificar_detallado(&prog(f)).es_ok());
    }

    #[test]
    fn cfg_return_none_inalcanzable_permitido_aun_con_retorno_tipado() {
        // bb1 es inalcanzable (sintético); no debe provocar falso positivo.
        let f = func(
            "f",
            TipoSem::Entero,
            vec![],
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::Return(Some(MirOperand::Const(MirConst::Entero(1)))),
                ),
                bb(1, vec![], MirTerminator::Return(None)),
            ],
        );
        assert!(VerificadorMir::verificar_detallado(&prog(f)).es_ok());
    }

    #[test]
    fn cfg_try_rescue_alcanzable_por_excepcion() {
        // El rescue se considera alcanzable desde un EntrarTry en bloque alcanzable;
        // si el rescue termina con Return(None) y el retorno es tipado, debe rechazarse.
        let f = func(
            "f",
            TipoSem::Entero,
            vec![],
            vec![
                bb(
                    0,
                    vec![MirStmt::EntrarTry {
                        rescue: BlockId(1),
                        merge: BlockId(2),
                        var_local: None,
                    }],
                    MirTerminator::Goto(BlockId(2)),
                ),
                bb(1, vec![], MirTerminator::Return(None)), // rescue alcanzable por arista excepcional
                bb(
                    2,
                    vec![],
                    MirTerminator::Return(Some(MirOperand::Const(MirConst::Entero(0)))),
                ),
            ],
        );
        let r = VerificadorMir::verificar_detallado(&prog(f));
        assert!(!r.es_ok());
        assert_eq!(r.errores[0].codigo, "MIR045");
    }

    #[test]
    fn cfg_try_merge_y_rescue_validos_acepta() {
        let f = func(
            "f",
            TipoSem::Entero,
            vec![],
            vec![
                bb(
                    0,
                    vec![MirStmt::EntrarTry {
                        rescue: BlockId(1),
                        merge: BlockId(2),
                        var_local: None,
                    }],
                    MirTerminator::Goto(BlockId(2)),
                ),
                bb(1, vec![], MirTerminator::Goto(BlockId(2))), // rescue -> merge
                bb(
                    2,
                    vec![],
                    MirTerminator::Return(Some(MirOperand::Const(MirConst::Entero(1)))),
                ),
            ],
        );
        assert!(VerificadorMir::verificar_detallado(&prog(f)).es_ok());
    }

    #[test]
    fn cfg_switch_transiciones_reales_alcanzan_ramas() {
        // Ambas ramas alcanzables retornan valor correcto.
        let f = func(
            "f",
            TipoSem::Entero,
            vec![TipoSem::Logico],
            vec![
                bb(
                    0,
                    vec![],
                    MirTerminator::SwitchInt {
                        discriminante: MirOperand::Copy(LocalId(0)),
                        casos: vec![(1, BlockId(1))],
                        por_defecto: BlockId(2),
                    },
                ),
                bb(
                    1,
                    vec![],
                    MirTerminator::Return(Some(MirOperand::Const(MirConst::Entero(1)))),
                ),
                bb(
                    2,
                    vec![],
                    MirTerminator::Return(Some(MirOperand::Const(MirConst::Entero(2)))),
                ),
            ],
        );
        assert!(VerificadorMir::verificar_detallado(&prog(f)).es_ok());
    }

    #[test]
    fn cfg_referencia_funcion_callee_resuelta_coherente() {
        let callee = func(
            "g",
            TipoSem::Entero,
            vec![],
            vec![bb(
                0,
                vec![],
                MirTerminator::Return(Some(MirOperand::Const(MirConst::Entero(1)))),
            )],
        );
        let mut caller = func(
            "f",
            TipoSem::Entero,
            vec![],
            vec![bb(
                0,
                vec![MirStmt::Llamada {
                    destino: None,
                    callee: "g".into(),
                    argumentos: vec![],
                }],
                MirTerminator::Return(Some(MirOperand::Const(MirConst::Entero(2)))),
            )],
        );
        let _ = &mut caller;
        let p = MirPrograma {
            funciones: vec![caller, callee],
            globals: vec![],
        };
        // destino None compatible; aridad 0 ok
        assert!(VerificadorMir::verificar_detallado(&p).es_ok());
    }
}
