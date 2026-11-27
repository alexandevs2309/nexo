//! Runtime embebido para el backend nativo de BASILISCO.
//!
//! Se incluye verbatim en el código Rust generado por `backend_nativo`.
//! Solo usa `std` (sin crates externos) para que el `.rs` generado compile
//! con `rustc` sin Cargo. Replica la semántica del intérprete (Valor,
//! aritmética, colecciones, E/S) para preservar la paridad VM/Tree-Walk.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub enum Valor {
    Entero(i64),
    Decimal(i128),
    Real(f64),
    Logico(bool),
    Texto(String),
    Arreglo(Vec<Valor>),
    Lista(Vec<Valor>),
    Objeto(BTreeMap<String, Valor>),
    Nulo,
    Funcion(String),
    /// Celda de referencia mutable compartida (capturas mutables en clausuras).
    Celda(Arc<Mutex<Valor>>),
    /// Clausura: nombre de función + valores capturados (celdas/valores).
    Clausura(String, Vec<Valor>),
    /// Estructura nominal (Fase 09 B).
    EstructuraNominal {
        tipo_id: u32,
        campos: Vec<Valor>,
    },
    /// Enumeración nominal (Fase 09 B).
    EnumNominal {
        tipo_id: u32,
        variante: u32,
        payload: Vec<Valor>,
    },
}

impl PartialEq for Valor {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Valor::Entero(a), Valor::Entero(b)) => a == b,
            (Valor::Decimal(a), Valor::Decimal(b)) => a == b,
            (Valor::Real(a), Valor::Real(b)) => a.to_bits() == b.to_bits(),
            (Valor::Logico(a), Valor::Logico(b)) => a == b,
            (Valor::Texto(a), Valor::Texto(b)) => a == b,
            (Valor::Arreglo(a), Valor::Arreglo(b)) | (Valor::Lista(a), Valor::Lista(b)) => a == b,
            (Valor::Arreglo(a), Valor::Lista(b)) | (Valor::Lista(a), Valor::Arreglo(b)) => a == b,
            (Valor::Objeto(a), Valor::Objeto(b)) => a == b,
            (Valor::Nulo, Valor::Nulo) => true,
            (Valor::Funcion(a), Valor::Funcion(b)) => a == b,
            (Valor::Celda(a), Valor::Celda(b)) => Arc::ptr_eq(a, b),
            (Valor::Clausura(na, ca), Valor::Clausura(nb, cb)) => na == nb && ca == cb,
            (
                Valor::EstructuraNominal {
                    tipo_id: a,
                    campos: ca,
                },
                Valor::EstructuraNominal {
                    tipo_id: b,
                    campos: cb,
                },
            ) => a == b && ca == cb,
            (
                Valor::EnumNominal {
                    tipo_id: ta,
                    variante: va,
                    payload: pa,
                },
                Valor::EnumNominal {
                    tipo_id: tb,
                    variante: vb,
                    payload: pb,
                },
            ) => ta == tb && va == vb && pa == pb,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeError(pub String);

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for Valor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Valor::Entero(n) => write!(f, "{}", n),
            Valor::Decimal(m) => {
                let neg = *m < 0;
                let a = m.unsigned_abs();
                let ent = a / (1_000_000_000u128);
                let frac = a % (1_000_000_000u128);
                let mut frac_s = format!("{:09}", frac);
                while frac_s.ends_with('0') && frac_s.len() > 1 {
                    frac_s.pop();
                }
                if neg {
                    write!(f, "-{}.{}", ent, frac_s)
                } else {
                    write!(f, "{}.{}", ent, frac_s)
                }
            }
            Valor::Real(a) => write!(f, "{}", a),
            Valor::Logico(true) => write!(f, "verdadero"),
            Valor::Logico(false) => write!(f, "falso"),
            Valor::Texto(s) => write!(f, "{}", s),
            Valor::Arreglo(v) | Valor::Lista(v) => {
                write!(f, "[")?;
                for (i, e) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            }
            Valor::Objeto(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
            Valor::Nulo => write!(f, "nulo"),
            Valor::Funcion(s) => write!(f, "<función: {}>", s),
            Valor::Celda(_) => write!(f, "<celda>"),
            Valor::Clausura(nombre, _) => write!(f, "<clausura {}>", nombre),
            Valor::EstructuraNominal { tipo_id, campos } => {
                write!(f, "<estructura#{} {{", tipo_id)?;
                for (i, c) in campos.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", c)?;
                }
                write!(f, "}}>")
            }
            Valor::EnumNominal {
                tipo_id,
                variante,
                payload,
            } => {
                write!(f, "<enum#{}::{}", tipo_id, variante)?;
                if !payload.is_empty() {
                    write!(f, "(")?;
                    for (i, c) in payload.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", c)?;
                    }
                    write!(f, ")")?;
                }
                write!(f, ">")
            }
        }
    }
}

// ---------- Aritmética ----------

const FACTOR_ESCALA: i128 = 1_000_000_000;
const DIEZ_POT_19: i128 = 10_000_000_000_000_000_000;

/// Tipo destino para conversiones de asignación (replica `convertir_tipo` de vm.rs).
pub enum TipoConv {
    Nada,
    Decimal,
    Real,
    Lista(Box<TipoConv>),
    /// (tipo_elemento, longitud) — la VM solo convierte si `len == n`.
    Arreglo(Box<TipoConv>, usize),
    Coleccion(Box<TipoConv>, usize),
}

impl Valor {
    /// Conversión de asignación implícita (ADR-0005 §1.4), idéntica a
    /// `convertir_tipo` de vm.rs: solo entero→decimal y entero→real sin pérdida,
    /// más normalización de contenedores.
    pub fn convertir_a(self, t: &TipoConv) -> Valor {
        match t {
            TipoConv::Nada => self,
            TipoConv::Decimal => match self {
                Valor::Entero(n) => Valor::Decimal((n as i128) * FACTOR_ESCALA),
                otro => otro,
            },
            TipoConv::Real => match self {
                Valor::Entero(n) => Valor::Real(n as f64),
                otro => otro,
            },
            TipoConv::Lista(elem) => match self {
                Valor::Arreglo(elems) | Valor::Lista(elems) => {
                    Valor::Lista(elems.into_iter().map(|e| e.convertir_a(elem)).collect())
                }
                otro => otro,
            },
            TipoConv::Arreglo(elem, n) => match self {
                Valor::Lista(elems) | Valor::Arreglo(elems) if elems.len() == *n => {
                    Valor::Arreglo(elems.into_iter().map(|e| e.convertir_a(elem)).collect())
                }
                otro => otro,
            },
            TipoConv::Coleccion(elem, n) => match self {
                Valor::Lista(elems) | Valor::Arreglo(elems) if elems.len() == *n => {
                    Valor::Lista(elems.into_iter().map(|e| e.convertir_a(elem)).collect())
                }
                otro => otro,
            },
        }
    }
}

fn err(msg: &str) -> RuntimeError {
    RuntimeError(msg.to_string())
}

/// Redondeo half-even (banquero) al múltiplo de `factor`. Idéntico a
/// `redondear_half_even` de `valor.rs`.
fn redondear_half_even(x: i128, factor: i128) -> i128 {
    if factor <= 1 {
        return x;
    }
    let negativo = x < 0;
    let a = x.unsigned_abs();
    let f = factor as u128;
    let cociente = a / f;
    let resto = a % f;
    let mitad = f / 2;
    let ajuste = if resto > mitad || (resto == mitad && cociente % 2 == 1) {
        1
    } else {
        0
    };
    let r = (cociente + ajuste) * f;
    // Saturación (equivalente a `i128::try_from(r).unwrap_or(i128::MAX)`, pero
    // sin depender de `TryFrom`, que no está en el prelude de la edición 2015
    // con la que compila el código generado por `rustc`).
    let r_i = if r > i128::MAX as u128 {
        i128::MAX
    } else {
        r as i128
    };
    if negativo { -r_i } else { r_i }
}

fn validar_rango_decimal(mantisa: i128) -> Result<(), RuntimeError> {
    let abs = mantisa.unsigned_abs();
    let limite = (DIEZ_POT_19 as u128) * (FACTOR_ESCALA as u128) - 1;
    if abs > limite {
        return Err(err("exceso de rango decimal"));
    }
    Ok(())
}

fn mul_dec(a: i128, b: i128) -> Option<i128> {
    let bruto = a.checked_mul(b)?;
    Some(redondear_half_even(bruto, FACTOR_ESCALA) / FACTOR_ESCALA)
}

fn division_decimal(a: i128, b: i128) -> Result<i128, RuntimeError> {
    if b == 0 {
        return Err(err("división entre cero"));
    }
    let numerador = a
        .checked_mul(FACTOR_ESCALA)
        .ok_or_else(|| err("exceso de rango decimal"))?;
    let bruto = numerador
        .checked_div(b)
        .ok_or_else(|| err("exceso de rango decimal"))?;
    let resto = numerador
        .checked_rem(b)
        .ok_or_else(|| err("exceso de rango decimal"))?;
    let doble_resto = resto.unsigned_abs().checked_mul(2);
    let abs_b = b.unsigned_abs();
    let ajuste = match doble_resto {
        Some(dr) if dr > abs_b => 1,
        Some(dr) if dr == abs_b && bruto.unsigned_abs() % 2 == 1 => 1,
        _ => 0,
    };
    let bruto = if (numerador < 0) != (b < 0) {
        bruto - ajuste
    } else {
        bruto + ajuste
    };
    validar_rango_decimal(bruto)?;
    Ok(bruto)
}

impl Valor {
    pub fn suma(&self, other: &Valor) -> Result<Valor, RuntimeError> {
        match (self, other) {
            (Valor::Entero(a), Valor::Entero(b)) => a
                .checked_add(*b)
                .map(Valor::Entero)
                .ok_or_else(|| err("desbordamiento entero")),
            (Valor::Entero(a), Valor::Decimal(b)) => {
                let a = *a as i128 * FACTOR_ESCALA;
                let bruto = a
                    .checked_add(*b)
                    .ok_or_else(|| err("exceso de rango decimal"))?;
                validar_rango_decimal(bruto)?;
                Ok(Valor::Decimal(bruto))
            }
            (Valor::Decimal(a), Valor::Entero(b)) => {
                let b = *b as i128 * FACTOR_ESCALA;
                let bruto = a
                    .checked_add(b)
                    .ok_or_else(|| err("exceso de rango decimal"))?;
                validar_rango_decimal(bruto)?;
                Ok(Valor::Decimal(bruto))
            }
            (Valor::Decimal(a), Valor::Decimal(b)) => {
                let bruto = a
                    .checked_add(*b)
                    .ok_or_else(|| err("exceso de rango decimal"))?;
                validar_rango_decimal(bruto)?;
                Ok(Valor::Decimal(bruto))
            }
            (Valor::Entero(a), Valor::Real(b)) => Ok(Valor::Real(*a as f64 + b)),
            (Valor::Real(a), Valor::Entero(b)) => Ok(Valor::Real(a + *b as f64)),
            (Valor::Real(a), Valor::Real(b)) => Ok(Valor::Real(a + b)),
            (Valor::Decimal(_), Valor::Real(_)) | (Valor::Real(_), Valor::Decimal(_)) => {
                Err(err("conversión ilegal"))
            }
            (Valor::Texto(a), Valor::Texto(b)) => Ok(Valor::Texto(format!("{}{}", a, b))),
            (Valor::Logico(_), _) | (_, Valor::Logico(_)) => Err(err("tipos incompatibles")),
            _ => Err(err("tipos incompatibles")),
        }
    }

    pub fn resta(&self, other: &Valor) -> Result<Valor, RuntimeError> {
        match (self, other) {
            (Valor::Entero(a), Valor::Entero(b)) => a
                .checked_sub(*b)
                .map(Valor::Entero)
                .ok_or_else(|| err("desbordamiento entero")),
            (Valor::Entero(a), Valor::Decimal(b)) => {
                let a = *a as i128 * FACTOR_ESCALA;
                let bruto = a
                    .checked_sub(*b)
                    .ok_or_else(|| err("exceso de rango decimal"))?;
                validar_rango_decimal(bruto)?;
                Ok(Valor::Decimal(bruto))
            }
            (Valor::Decimal(a), Valor::Entero(b)) => {
                let b = *b as i128 * FACTOR_ESCALA;
                let bruto = a
                    .checked_sub(b)
                    .ok_or_else(|| err("exceso de rango decimal"))?;
                validar_rango_decimal(bruto)?;
                Ok(Valor::Decimal(bruto))
            }
            (Valor::Decimal(a), Valor::Decimal(b)) => {
                let bruto = a
                    .checked_sub(*b)
                    .ok_or_else(|| err("exceso de rango decimal"))?;
                validar_rango_decimal(bruto)?;
                Ok(Valor::Decimal(bruto))
            }
            (Valor::Entero(a), Valor::Real(b)) => Ok(Valor::Real(*a as f64 - b)),
            (Valor::Real(a), Valor::Entero(b)) => Ok(Valor::Real(a - *b as f64)),
            (Valor::Real(a), Valor::Real(b)) => Ok(Valor::Real(a - b)),
            (Valor::Decimal(_), Valor::Real(_)) | (Valor::Real(_), Valor::Decimal(_)) => {
                Err(err("conversión ilegal"))
            }
            _ => Err(err("tipos incompatibles")),
        }
    }

    pub fn mul(&self, other: &Valor) -> Result<Valor, RuntimeError> {
        match (self, other) {
            (Valor::Entero(a), Valor::Entero(b)) => a
                .checked_mul(*b)
                .map(Valor::Entero)
                .ok_or_else(|| err("desbordamiento entero")),
            (Valor::Entero(a), Valor::Decimal(b)) => {
                let a = *a as i128 * FACTOR_ESCALA;
                let bruto = mul_dec(a, *b).ok_or_else(|| err("exceso de rango decimal"))?;
                validar_rango_decimal(bruto)?;
                Ok(Valor::Decimal(bruto))
            }
            (Valor::Decimal(a), Valor::Entero(b)) => {
                let b = *b as i128 * FACTOR_ESCALA;
                let bruto = mul_dec(*a, b).ok_or_else(|| err("exceso de rango decimal"))?;
                validar_rango_decimal(bruto)?;
                Ok(Valor::Decimal(bruto))
            }
            (Valor::Decimal(a), Valor::Decimal(b)) => {
                let bruto = mul_dec(*a, *b).ok_or_else(|| err("exceso de rango decimal"))?;
                validar_rango_decimal(bruto)?;
                Ok(Valor::Decimal(bruto))
            }
            (Valor::Entero(a), Valor::Real(b)) => Ok(Valor::Real(*a as f64 * b)),
            (Valor::Real(a), Valor::Entero(b)) => Ok(Valor::Real(a * *b as f64)),
            (Valor::Real(a), Valor::Real(b)) => Ok(Valor::Real(a * b)),
            (Valor::Decimal(_), Valor::Real(_)) | (Valor::Real(_), Valor::Decimal(_)) => {
                Err(err("conversión ilegal"))
            }
            _ => Err(err("tipos incompatibles")),
        }
    }

    pub fn div(&self, other: &Valor) -> Result<Valor, RuntimeError> {
        match (self, other) {
            (Valor::Entero(a), Valor::Entero(b)) => {
                if *b == 0 {
                    return Err(err("división entre cero"));
                }
                if *a == i64::MIN && *b == -1 {
                    return Err(err("desbordamiento entero"));
                }
                Ok(Valor::Entero(a / b))
            }
            (Valor::Entero(a), Valor::Decimal(b)) => {
                let a = *a as i128 * FACTOR_ESCALA;
                division_decimal(a, *b).map(Valor::Decimal)
            }
            (Valor::Decimal(a), Valor::Entero(b)) => {
                let b = *b as i128 * FACTOR_ESCALA;
                division_decimal(*a, b).map(Valor::Decimal)
            }
            (Valor::Decimal(a), Valor::Decimal(b)) => division_decimal(*a, *b).map(Valor::Decimal),
            (Valor::Entero(a), Valor::Real(b)) => {
                if *b == 0.0 {
                    return Err(err("división entre cero"));
                }
                Ok(Valor::Real(*a as f64 / b))
            }
            (Valor::Real(a), Valor::Entero(b)) => {
                if *b == 0 {
                    return Err(err("división entre cero"));
                }
                Ok(Valor::Real(a / *b as f64))
            }
            (Valor::Real(a), Valor::Real(b)) => {
                if *b == 0.0 {
                    return Err(err("división entre cero"));
                }
                Ok(Valor::Real(a / b))
            }
            _ => Err(err("tipos incompatibles")),
        }
    }

    pub fn modulo(&self, other: &Valor) -> Result<Valor, RuntimeError> {
        match (self, other) {
            (Valor::Entero(a), Valor::Entero(b)) => {
                if *b == 0 {
                    return Err(err("división entre cero"));
                }
                if *a == i64::MIN && *b == -1 {
                    return Ok(Valor::Entero(0));
                }
                Ok(Valor::Entero(a % b))
            }
            _ => Err(err("conversión ilegal")),
        }
    }

    pub fn neg(&self) -> Result<Valor, RuntimeError> {
        match self {
            Valor::Entero(n) => n
                .checked_neg()
                .map(Valor::Entero)
                .ok_or_else(|| err("desbordamiento entero")),
            Valor::Decimal(n) => Ok(Valor::Decimal(-*n)),
            Valor::Real(n) => Ok(Valor::Real(-*n)),
            _ => Err(RuntimeError(format!(
                "tipos incompatibles: {} y negación",
                self.nombre()
            ))),
        }
    }

    pub fn not(&self) -> Result<Valor, RuntimeError> {
        match self {
            Valor::Logico(b) => Ok(Valor::Logico(!*b)),
            _ => Err(RuntimeError(format!(
                "se esperaba «lógico», se encontró «{}»",
                self.nombre()
            ))),
        }
    }

    // ---------- Comparaciones ----------

    pub fn igual(&self, other: &Valor) -> bool {
        match (self, other) {
            (Valor::Entero(a), Valor::Entero(b)) => a == b,
            (Valor::Entero(a), Valor::Decimal(b)) => (*a as i128 * 1_000_000_000) == *b,
            (Valor::Decimal(a), Valor::Entero(b)) => *a == (*b as i128 * 1_000_000_000),
            (Valor::Entero(a), Valor::Real(b)) => (*a as f64) == *b,
            (Valor::Real(a), Valor::Entero(b)) => *a == (*b as f64),
            (Valor::Decimal(a), Valor::Decimal(b)) => a == b,
            (Valor::Real(a), Valor::Real(b)) => a == b,
            (Valor::Logico(a), Valor::Logico(b)) => a == b,
            (Valor::Texto(a), Valor::Texto(b)) => a == b,
            (Valor::Nulo, Valor::Nulo) => true,
            (Valor::Arreglo(a), Valor::Arreglo(b)) => a == b,
            (Valor::Lista(a), Valor::Lista(b)) => a == b,
            (Valor::Objeto(a), Valor::Objeto(b)) => a == b,
            (Valor::Funcion(a), Valor::Funcion(b)) => a == b,
            (Valor::Celda(a), Valor::Celda(b)) => std::sync::Arc::ptr_eq(a, b),
            (Valor::Clausura(na, ca), Valor::Clausura(nb, cb)) => na == nb && ca == cb,
            (
                Valor::EstructuraNominal {
                    tipo_id: a,
                    campos: ca,
                },
                Valor::EstructuraNominal {
                    tipo_id: b,
                    campos: cb,
                },
            ) => a == b && ca.iter().zip(cb).all(|(x, y)| x.igual(y)),
            (
                Valor::EnumNominal {
                    tipo_id: ta,
                    variante: va,
                    payload: pa,
                },
                Valor::EnumNominal {
                    tipo_id: tb,
                    variante: vb,
                    payload: pb,
                },
            ) => {
                ta == tb
                    && va == vb
                    && pa.len() == pb.len()
                    && pa.iter().zip(pb).all(|(x, y)| x.igual(y))
            }
            _ => false,
        }
    }

    pub fn lt(&self, other: &Valor) -> Result<bool, RuntimeError> {
        match (self, other) {
            (Valor::Entero(a), Valor::Entero(b)) => Ok(a < b),
            (Valor::Entero(a), Valor::Decimal(b)) => Ok((*a as i128 * 1_000_000_000) < *b),
            (Valor::Decimal(a), Valor::Entero(b)) => Ok(*a < (*b as i128 * 1_000_000_000)),
            (Valor::Decimal(a), Valor::Decimal(b)) => Ok(a < b),
            (Valor::Entero(a), Valor::Real(b)) => Ok((*a as f64) < *b),
            (Valor::Real(a), Valor::Entero(b)) => Ok(*a < (*b as f64)),
            (Valor::Real(a), Valor::Real(b)) => Ok(a < b),
            (Valor::Texto(a), Valor::Texto(b)) => Ok(a < b),
            _ => Err(RuntimeError(format!(
                "tipos incompatibles: {} y {}",
                self.nombre(),
                other.nombre()
            ))),
        }
    }

    /// Nombre del tipo (espejo de `Valor::nombre_tipo` de valor.rs).
    pub fn nombre(&self) -> &'static str {
        match self {
            Valor::Entero(_) => "entero",
            Valor::Decimal(_) => "decimal",
            Valor::Real(_) => "real",
            Valor::Logico(_) => "lógico",
            Valor::Texto(_) => "texto",
            Valor::Arreglo(_) => "arreglo",
            Valor::Lista(_) => "lista",
            Valor::Objeto(_) => "objeto",
            Valor::Nulo => "nulo",
            Valor::Funcion(_) => "función",
            Valor::Celda(_) => "celda",
            Valor::Clausura(_, _) => "función",
            Valor::EstructuraNominal { .. } => "estructura",
            Valor::EnumNominal { .. } => "enum",
        }
    }

    pub fn crear_celda(v: Valor) -> Valor {
        Valor::Celda(std::sync::Arc::new(std::sync::Mutex::new(v)))
    }

    pub fn leer_celda(&self) -> Result<Valor, RuntimeError> {
        match self {
            Valor::Celda(c) => Ok(c.lock().unwrap().clone()),
            otro => Err(RuntimeError(format!(
                "se esperaba «celda», se encontró «{}»",
                otro.nombre()
            ))),
        }
    }

    pub fn escribir_celda(&self, v: Valor) -> Result<(), RuntimeError> {
        match self {
            Valor::Celda(c) => {
                *c.lock().unwrap() = v;
                Ok(())
            }
            otro => Err(RuntimeError(format!(
                "se esperaba «celda», se encontró «{}»",
                otro.nombre()
            ))),
        }
    }

    // ---------- Nominales (Fase 09 B) ----------

    pub fn campo_nominal(&self, idx: usize) -> Result<Valor, RuntimeError> {
        match self {
            Valor::EstructuraNominal { campos, .. } => campos.get(idx).cloned().ok_or_else(|| {
                RuntimeError(format!(
                    "campo nominal {} fuera de rango (tamaño {})",
                    idx,
                    campos.len()
                ))
            }),
            otro => Err(RuntimeError(format!(
                "se esperaba «estructura nominal», se encontró «{}»",
                otro.nombre()
            ))),
        }
    }

    pub fn asignar_campo_nominal(&mut self, idx: usize, v: Valor) -> Result<(), RuntimeError> {
        match self {
            Valor::EstructuraNominal { campos, .. } => {
                if idx < campos.len() {
                    campos[idx] = v;
                    Ok(())
                } else {
                    Err(RuntimeError(format!(
                        "campo nominal {} fuera de rango (tamaño {})",
                        idx,
                        campos.len()
                    )))
                }
            }
            otro => Err(RuntimeError(format!(
                "se esperaba «estructura nominal», se encontró «{}»",
                otro.nombre()
            ))),
        }
    }

    pub fn discriminante_enum(&self) -> Result<i64, RuntimeError> {
        match self {
            Valor::EnumNominal { variante, .. } => Ok(*variante as i64),
            otro => Err(RuntimeError(format!(
                "se esperaba «enum», se encontró «{}»",
                otro.nombre()
            ))),
        }
    }

    pub fn payload_enum(&self, idx: usize) -> Result<Valor, RuntimeError> {
        match self {
            Valor::EnumNominal { payload, .. } => payload.get(idx).cloned().ok_or_else(|| {
                RuntimeError(format!(
                    "payload enum {} fuera de rango (tamaño {})",
                    idx,
                    payload.len()
                ))
            }),
            otro => Err(RuntimeError(format!(
                "se esperaba «enum», se encontró «{}»",
                otro.nombre()
            ))),
        }
    }

    // ---------- Colecciones ----------

    pub fn indice(&self, i: i64) -> Result<Valor, RuntimeError> {
        match self {
            Valor::Arreglo(v) | Valor::Lista(v) => {
                if i < 0 || i as usize >= v.len() {
                    return Err(RuntimeError(format!(
                        "índice {} fuera de rango (tamaño {})",
                        i,
                        v.len()
                    )));
                }
                Ok(v[i as usize].clone())
            }
            Valor::Texto(s) => {
                let chars: Vec<char> = s.chars().collect();
                if i < 0 || i as usize >= chars.len() {
                    return Err(RuntimeError(format!(
                        "índice {} fuera de rango (tamaño {})",
                        i,
                        chars.len()
                    )));
                }
                Ok(Valor::Texto(chars[i as usize].to_string()))
            }
            Valor::Objeto(_m) => Err(RuntimeError(
                "se esperaba arreglo, lista o texto".to_string(),
            )),
            _ => Err(RuntimeError(
                "se esperaba arreglo, lista o texto".to_string(),
            )),
        }
    }

    pub fn indice_set(&mut self, i: i64, v: Valor) -> Result<(), RuntimeError> {
        match self {
            Valor::Arreglo(arr) | Valor::Lista(arr) => {
                if i < 0 || i as usize >= arr.len() {
                    return Err(RuntimeError(format!(
                        "índice {} fuera de rango (tamaño {})",
                        i,
                        arr.len()
                    )));
                }
                arr[i as usize] = v;
                Ok(())
            }
            _ => Err(RuntimeError("se esperaba arreglo o lista".to_string())),
        }
    }

    pub fn campo(&self, name: &str) -> Result<Valor, RuntimeError> {
        match self {
            Valor::Objeto(m) => m
                .get(name)
                .cloned()
                .ok_or_else(|| RuntimeError(format!("campo inexistente: {}", name))),
            _ => Err(RuntimeError("se esperaba objeto".to_string())),
        }
    }

    pub fn agregar(&self, x: Valor) -> Valor {
        match self {
            Valor::Arreglo(v) => {
                let mut n = v.clone();
                n.push(x);
                Valor::Arreglo(n)
            }
            Valor::Lista(v) => {
                let mut n = v.clone();
                n.push(x);
                Valor::Lista(n)
            }
            _ => self.clone(),
        }
    }

    pub fn eliminar(&self, i: i64) -> Result<Valor, RuntimeError> {
        match self {
            Valor::Arreglo(v) | Valor::Lista(v) => {
                if i < 0 || i as usize >= v.len() {
                    return Err(RuntimeError(format!(
                        "índice {} fuera de rango (tamaño {})",
                        i,
                        v.len()
                    )));
                }
                let mut n = v.clone();
                n.remove(i as usize);
                if matches!(self, Valor::Arreglo(_)) {
                    Ok(Valor::Arreglo(n))
                } else {
                    Ok(Valor::Lista(n))
                }
            }
            _ => Err(RuntimeError("se esperaba arreglo o lista".to_string())),
        }
    }

    pub fn discriminante(&self) -> i64 {
        match self {
            Valor::Entero(n) => *n,
            Valor::Logico(true) => 1,
            Valor::Logico(false) => 0,
            Valor::Nulo => 0,
            Valor::EnumNominal { variante, .. } => *variante as i64,
            _ => 0,
        }
    }

    pub fn longitud(&self) -> i64 {
        match self {
            Valor::Arreglo(v) | Valor::Lista(v) => v.len() as i64,
            Valor::Texto(s) => s.chars().count() as i64,
            Valor::Objeto(m) => m.len() as i64,
            _ => 0,
        }
    }
}

// --- E/S y JSON (Etapa C: soporte stdlib nativo) ---

pub fn bsk_leer_archivo(ruta: Valor) -> Result<Valor, RuntimeError> {
    let ruta = match ruta {
        Valor::Texto(s) => s,
        otro => {
            return Err(RuntimeError(format!(
                "leer_archivo: se esperaba texto y se obtuvo {}",
                otro.nombre()
            )));
        }
    };
    std::fs::read_to_string(&ruta)
        .map(Valor::Texto)
        .map_err(|e| RuntimeError(format!("no se pudo leer '{ruta}': {e}")))
}

pub fn bsk_json_parsear(texto: Valor) -> Result<Valor, RuntimeError> {
    let texto = match texto {
        Valor::Texto(s) => s,
        otro => {
            return Err(RuntimeError(format!(
                "json_parsear: se esperaba texto y se obtuvo {}",
                otro.nombre()
            )));
        }
    };
    let mut p = JsonParser::new(&texto);
    let v = p.parse_value()?;
    p.skip_ws();
    if p.pos < p.chars.len() {
        return Err(RuntimeError(
            "JSON inválido: contenido sobrante".to_string(),
        ));
    }
    Ok(v)
}

struct JsonParser<'a> {
    chars: Vec<char>,
    pos: usize,
    _src: &'a str,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            chars: s.chars().collect(),
            pos: 0,
            _src: s,
        }
    }
    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn parse_value(&mut self) -> Result<Valor, RuntimeError> {
        self.skip_ws();
        match self.peek() {
            Some('n') => self.parse_literal("null", Valor::Nulo),
            Some('t') => self.parse_literal("true", Valor::Logico(true)),
            Some('f') => self.parse_literal("false", Valor::Logico(false)),
            Some('"') => self.parse_string().map(Valor::Texto),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            _ => Err(RuntimeError("JSON inválido: valor inesperado".to_string())),
        }
    }
    fn parse_literal(&mut self, lit: &str, v: Valor) -> Result<Valor, RuntimeError> {
        for c in lit.chars() {
            if !self.eat(c) {
                return Err(RuntimeError(format!("JSON inválido: se esperaba {lit}")));
            }
        }
        Ok(v)
    }
    fn parse_string(&mut self) -> Result<String, RuntimeError> {
        if !self.eat('"') {
            return Err(RuntimeError(
                "JSON inválido: se esperaba cadena".to_string(),
            ));
        }
        let mut out = String::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let e = self.peek().ok_or_else(|| {
                        RuntimeError("JSON inválido: escape incompleto".to_string())
                    })?;
                    self.pos += 1;
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000C}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let mut code = 0u32;
                            for _ in 0..4 {
                                let h = self.peek().ok_or_else(|| {
                                    RuntimeError("JSON inválido: \\u incompleto".to_string())
                                })?;
                                self.pos += 1;
                                code = code * 16
                                    + h.to_digit(16).ok_or_else(|| {
                                        RuntimeError("JSON inválido: \\u".to_string())
                                    })?;
                            }
                            if let Some(ch) = char::from_u32(code) {
                                out.push(ch);
                            }
                        }
                        _ => out.push(e),
                    }
                }
                otro => out.push(otro),
            }
        }
        Err(RuntimeError("JSON inválido: cadena sin cerrar".to_string()))
    }
    fn parse_number(&mut self) -> Result<Valor, RuntimeError> {
        let start = self.pos;
        self.eat('-');
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        let mut is_float = false;
        if self.eat('.') {
            is_float = true;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        if !is_float && let Ok(i) = s.parse::<i64>() {
            return Ok(Valor::Entero(i));
        }
        match s.parse::<f64>() {
            Ok(f) => Ok(Valor::Real(f)),
            Err(_) => Err(RuntimeError(format!("JSON inválido: número {s}"))),
        }
    }
    fn parse_array(&mut self) -> Result<Valor, RuntimeError> {
        if !self.eat('[') {
            return Err(RuntimeError("JSON inválido: se esperaba [".to_string()));
        }
        let mut items = Vec::new();
        self.skip_ws();
        if self.eat(']') {
            return Ok(Valor::Lista(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            if self.eat(']') {
                break;
            }
            if !self.eat(',') {
                return Err(RuntimeError("JSON inválido: se esperaba , o ]".to_string()));
            }
        }
        Ok(Valor::Lista(items))
    }
    fn parse_object(&mut self) -> Result<Valor, RuntimeError> {
        if !self.eat('{') {
            return Err(RuntimeError("JSON inválido: se esperaba {".to_string()));
        }
        let mut map = std::collections::BTreeMap::new();
        self.skip_ws();
        if self.eat('}') {
            return Ok(Valor::Objeto(map));
        }
        loop {
            self.skip_ws();
            let k = self.parse_string()?;
            self.skip_ws();
            if !self.eat(':') {
                return Err(RuntimeError("JSON inválido: se esperaba :".to_string()));
            }
            let v = self.parse_value()?;
            map.insert(k, v);
            self.skip_ws();
            if self.eat('}') {
                break;
            }
            if !self.eat(',') {
                return Err(RuntimeError("JSON inválido: se esperaba , o }".to_string()));
            }
        }
        Ok(Valor::Objeto(map))
    }
}

pub fn bsk_assert_logico(v: Valor, op: &str) -> Result<Valor, RuntimeError> {
    match v {
        Valor::Logico(_) => Ok(v),
        ref otro => Err(RuntimeError(format!(
            "el operador «{}» requiere operandos lógicos, se obtuvo «{}»",
            op,
            otro.nombre()
        ))),
    }
}
