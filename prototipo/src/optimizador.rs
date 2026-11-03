//! Optimizador de MIR para BASILISCO.
//!
//! Pases conservadores sobre el MIR ya verificado:
//! 1. **Plegado de constantes** (`plegar_constantes`): evalúa operaciones cuyos
//!    operandos son constantes (o locales con valor constante conocido en el
//!    mismo bloque) y las sustituye por su resultado.
//! 2. **Simplificación de ramas** (`simplificar_ramas`): un `SwitchInt` cuyo
//!    discriminante es constante se convierte en `Goto` al bloque elegido.
//! 3. **Eliminación de código muerto trivial** (`eliminar_nops`): borra
//!    sentencias `Nop`.
//!
//! Todos los pases son idempotentes y conservan el comportamiento observable
//! (validado por las pruebas diferenciales Tree-Walk vs VM).

use crate::mir::{
    MirBinOp, MirConst, MirFunction, MirOperand, MirPrograma, MirRvalue, MirStmt, MirTerminator,
    MirUnOp,
};
use crate::valor::Valor;
use std::collections::HashMap;

/// Punto de entrada: optimiza todas las funciones del programa hasta punto fijo.
pub fn optimizar(programa: &mut MirPrograma) {
    for f in &mut programa.funciones {
        optimizar_funcion(f);
    }
}

/// Optimiza una función hasta punto fijo (máximo 8 iteraciones).
pub fn optimizar_funcion(f: &mut MirFunction) {
    for _ in 0..8 {
        let antes = f.clone();
        plegar_constantes(f);
        simplificar_ramas(f);
        eliminar_nops(f);
        if *f == antes {
            break;
        }
    }
}

/// Plegado de constantes + propagación local dentro de cada bloque.
fn plegar_constantes(f: &mut MirFunction) {
    for bloque in &mut f.bloques {
        // Mapa local -> constante conocida en este bloque.
        let mut conocidos: HashMap<usize, MirConst> = HashMap::new();
        // Reescribimos stmts en sitio.
        for stmt in &mut bloque.stmts {
            match stmt {
                MirStmt::Asignacion { destino, valor } => {
                    // Sustituir operandos Copy por constantes conocidas.
                    sustituir_constantes(valor, &conocidos);
                    // Intentar plegar.
                    if let Some(c) = plegar_rvalue(valor) {
                        *valor = MirRvalue::Use(MirOperand::Const(c.clone()));
                        // Registrar constante del destino si es local simple.
                        if destino.projection.is_empty() {
                            conocidos.insert(destino.local.0, c);
                        }
                    } else if let MirRvalue::Use(MirOperand::Const(c)) = valor {
                        if destino.projection.is_empty() {
                            conocidos.insert(destino.local.0, c.clone());
                        }
                    } else {
                        // El destino deja de ser constante conocida.
                        if destino.projection.is_empty() {
                            conocidos.remove(&destino.local.0);
                        }
                    }
                }
                MirStmt::Llamada { argumentos, .. } => {
                    for a in argumentos {
                        sustituir_operando(a, &conocidos);
                    }
                }
                MirStmt::LlamadaIndirecta {
                    callee, argumentos, ..
                } => {
                    sustituir_operando(callee, &conocidos);
                    for a in argumentos {
                        sustituir_operando(a, &conocidos);
                    }
                }
                MirStmt::Nop => {}
            }
        }
        // Plegar el discriminante de terminadores SwitchInt si es Copy conocido.
        if let MirTerminator::SwitchInt { discriminante, .. } = &mut bloque.terminator {
            sustituir_operando(discriminante, &conocidos);
        }
    }
}

fn sustituir_constantes(r: &mut MirRvalue, conocidos: &HashMap<usize, MirConst>) {
    match r {
        MirRvalue::BinOp { lhs, rhs, .. } => {
            sustituir_operando(lhs, conocidos);
            sustituir_operando(rhs, conocidos);
        }
        MirRvalue::UnOp { operand, .. } => sustituir_operando(operand, conocidos),
        MirRvalue::Agregado { elementos } => {
            for e in elementos {
                sustituir_operando(e, conocidos);
            }
        }
        MirRvalue::Indice { base, indice } => {
            sustituir_operando(base, conocidos);
            sustituir_operando(indice, conocidos);
        }
        MirRvalue::Campo { base, .. } => sustituir_operando(base, conocidos),
        MirRvalue::Longitud { base } => sustituir_operando(base, conocidos),
        MirRvalue::Rango { inicio, fin, .. } => {
            sustituir_operando(inicio, conocidos);
            sustituir_operando(fin, conocidos);
        }
        MirRvalue::Clausura { capturas, .. } => {
            for c in capturas {
                sustituir_operando(c, conocidos);
            }
        }
        MirRvalue::CrearCelda { valor } => sustituir_operando(valor, conocidos),
        MirRvalue::LeerCelda { base } => sustituir_operando(base, conocidos),
        MirRvalue::Use(op) => sustituir_operando(op, conocidos),
    }
}

fn sustituir_operando(op: &mut MirOperand, conocidos: &HashMap<usize, MirConst>) {
    if let MirOperand::Copy(local) = op
        && let Some(c) = conocidos.get(&local.0)
    {
        *op = MirOperand::Const(c.clone());
    }
}

/// Plega un rvalue si todos sus operandos son constantes.
fn plegar_rvalue(r: &MirRvalue) -> Option<MirConst> {
    match r {
        MirRvalue::Use(MirOperand::Const(c)) => Some(c.clone()),
        MirRvalue::BinOp { op, lhs, rhs } => {
            let (MirOperand::Const(a), MirOperand::Const(b)) = (lhs, rhs) else {
                return None;
            };
            plegar_binop(*op, a, b)
        }
        MirRvalue::UnOp { op, operand } => {
            let MirOperand::Const(a) = operand else {
                return None;
            };
            plegar_unop(*op, a)
        }
        _ => None,
    }
}

fn plegar_binop(op: MirBinOp, a: &MirConst, b: &MirConst) -> Option<MirConst> {
    // Convertir a Valor para reutilizar la semántica del núcleo.
    let va = const_a_valor(a);
    let vb = const_a_valor(b);
    let vr = match op {
        MirBinOp::Suma => va.suma(&vb).ok()?,
        MirBinOp::Resta => va.resta(&vb).ok()?,
        MirBinOp::Multiplicacion => va.multiplicacion(&vb).ok()?,
        MirBinOp::Division => va.division(&vb).ok()?,
        MirBinOp::Modulo => va.modulo(&vb).ok()?,
        MirBinOp::Igual => Valor::Logico(va == vb),
        MirBinOp::Distinto => Valor::Logico(va != vb),
        MirBinOp::Menor => {
            let o = va.partial_cmp_like(&vb)?;
            Valor::Logico(matches!(o, std::cmp::Ordering::Less))
        }
        MirBinOp::MenorIgual => {
            let o = va.partial_cmp_like(&vb)?;
            Valor::Logico(matches!(
                o,
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal
            ))
        }
        MirBinOp::Mayor => {
            let o = va.partial_cmp_like(&vb)?;
            Valor::Logico(matches!(o, std::cmp::Ordering::Greater))
        }
        MirBinOp::MayorIgual => {
            let o = va.partial_cmp_like(&vb)?;
            Valor::Logico(matches!(
                o,
                std::cmp::Ordering::Greater | std::cmp::Ordering::Equal
            ))
        }
        MirBinOp::Y => match (va, vb) {
            (Valor::Logico(x), Valor::Logico(y)) => Valor::Logico(x && y),
            _ => return None,
        },
        MirBinOp::O => match (va, vb) {
            (Valor::Logico(x), Valor::Logico(y)) => Valor::Logico(x || y),
            _ => return None,
        },
    };
    Some(valor_a_const(&vr))
}

fn plegar_unop(op: MirUnOp, a: &MirConst) -> Option<MirConst> {
    let va = const_a_valor(a);
    let vr = match op {
        MirUnOp::No => match va {
            Valor::Logico(b) => Valor::Logico(!b),
            _ => return None,
        },
        MirUnOp::Negacion => match va {
            Valor::Entero(n) => Valor::Entero(n.checked_neg()?),
            Valor::Decimal(m) => Valor::Decimal(-m),
            Valor::Real(x) => Valor::Real(-x),
            _ => return None,
        },
    };
    Some(valor_a_const(&vr))
}

trait PartialCmpLike {
    fn partial_cmp_like(&self, other: &Valor) -> Option<std::cmp::Ordering>;
}

impl PartialCmpLike for Valor {
    fn partial_cmp_like(&self, other: &Valor) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Valor::Entero(x), Valor::Entero(y)) => Some(x.cmp(y)),
            (Valor::Entero(x), Valor::Decimal(y)) => Some((*x as i128 * 1_000_000_000).cmp(y)),
            (Valor::Decimal(x), Valor::Entero(y)) => Some(x.cmp(&(*y as i128 * 1_000_000_000))),
            (Valor::Decimal(x), Valor::Decimal(y)) => Some(x.cmp(y)),
            (Valor::Real(x), Valor::Real(y)) => x.partial_cmp(y),
            (Valor::Texto(x), Valor::Texto(y)) => Some(x.cmp(y)),
            _ => None,
        }
    }
}

fn const_a_valor(c: &MirConst) -> Valor {
    match c {
        MirConst::Entero(n) => Valor::Entero(*n),
        MirConst::Decimal(n) => Valor::Decimal(*n),
        MirConst::Real(n) => Valor::Real(*n),
        MirConst::Logico(b) => Valor::Logico(*b),
        MirConst::Texto(s) => Valor::Texto(s.clone()),
        MirConst::Nulo => Valor::Nulo,
    }
}

fn valor_a_const(v: &Valor) -> MirConst {
    match v {
        Valor::Entero(n) => MirConst::Entero(*n),
        Valor::Decimal(n) => MirConst::Decimal(*n),
        Valor::Real(n) => MirConst::Real(*n),
        Valor::Logico(b) => MirConst::Logico(*b),
        Valor::Texto(s) => MirConst::Texto(s.clone()),
        _ => MirConst::Nulo,
    }
}

/// Simplificación de ramas: SwitchInt con discriminante constante → Goto.
fn simplificar_ramas(f: &mut MirFunction) {
    for bloque in &mut f.bloques {
        // Necesitamos leer el discriminante y luego reemplazar; clonamos.
        let nuevo = match &bloque.terminator {
            MirTerminator::SwitchInt {
                discriminante: MirOperand::Const(c),
                casos,
                por_defecto,
            } => {
                let disc = match c {
                    MirConst::Entero(n) => *n,
                    MirConst::Logico(b) => i64::from(*b),
                    _ => {
                        // No sabemos elegir; dejamos como está.
                        continue;
                    }
                };
                let destino = casos
                    .iter()
                    .find(|(v, _)| *v == disc)
                    .map(|(_, b)| *b)
                    .unwrap_or(*por_defecto);
                Some(MirTerminator::Goto(destino))
            }
            _ => None,
        };
        if let Some(t) = nuevo {
            bloque.terminator = t;
        }
    }
}

/// Eliminación trivial de `Nop`.
fn eliminar_nops(f: &mut MirFunction) {
    for bloque in &mut f.bloques {
        bloque.stmts.retain(|s| !matches!(s, MirStmt::Nop));
    }
}
