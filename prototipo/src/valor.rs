//! Valores y tipos del núcleo de BASILISCO (ADR-0005 aprobado; tipo `lógico` ADR-0007 aprobado).
//!
//! Tres tipos numéricos: `entero` (i64), `decimal` (DECIMAL(28,9) escalonado i128) y
//! `real` (f64). Semántica:
//! - Desbordamiento entero = error (`D2`).
//! - División por cero = error en los tres tipos (`D3`).
//! - `%` de enteros = resto truncado, signo del dividendo; `i64::MIN % -1` = 0 (`D4`).
//! - Decimal: escala 9 fija, redondeo half-even, exceso de precisión redondea, exceso de rango = error (`D5`).
//! - Conversiones: solo implícitas sin pérdida (`entero→decimal`, `entero→real`); el resto explícitas (`16`).
//!
//! Tipo adicional (ADR-0007 §1.1): `lógico` (`bool`). **No** es numérico: no hay conversión
//! en ningún sentido con `entero`/`decimal`/`real`; cualquier mezcla es error con ubicación.

use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::{Arc, Mutex};

/// Escala fija del decimal del núcleo: 9 decimales (ADR-0005 §1.2).
pub const ESCALA_DECIMAL: u32 = 9;
/// Precisión total del decimal del núcleo: 28 dígitos significativos.
pub const PRECISION_DECIMAL: u32 = 28;
/// 10^ESCALA_DECIMAL como i128.
pub const FACTOR_ESCALA: i128 = 1_000_000_000;
/// 10^19, para validación de rango del decimal.
const DIEZ_POT_19: i128 = 10_000_000_000_000_000_000;

/// Un valor de BASILISCO con su tipo (tres numéricos + `lógico` + `texto` + `arreglo` + `lista` + `nulo` + `archivo`).
pub enum Valor {
    Entero(i64),
    /// Decimal escalonado: `entero / 10^9`. El entero tiene hasta 19 dígitos.
    Decimal(i128),
    Real(f64),
    /// Valor lógico (ADR-0007). No es numérico: no se convierte ni se compara con números.
    Logico(bool),
    /// Valor de texto (string UTF-8).
    Texto(String),
    /// Arreglo de tamaño fijo: `arreglo<T, N>`.
    Arreglo(Vec<Valor>),
    /// Lista dinámica: `lista<T>`.
    Lista(Vec<Valor>),
    /// Valor nulo (ADR-0011 §3.5). Distinto de falso, 0 y "".
    Nulo,
    /// Archivo abierto (ADR-0013 §3.5). Encapsula el estado de E/S.
    Archivo(Arc<Mutex<ArchivoInterno>>),
    /// Módulo cargado (ADR-0017). Referencia opaca al módulo.
    Modulo(Arc<crate::cargador::ModuloCargado>),
    /// Función de primera clase (referencia por nombre).
    Funcion(String),
}

/// Estado interno de un archivo abierto (ADR-0013 §3.5). Opaco para el código BASILISCO.
#[derive(Debug)]
pub struct ArchivoInterno {
    /// Ruta del archivo (para mensajes de error).
    pub ruta: String,
    /// Modo de apertura: "leer", "escribir", "agregar".
    pub modo: String,
    /// True si el archivo fue cerrado.
    pub cerrado: bool,
    /// Lector con búfer (solo en modo "leer").
    pub lector: Option<BufReader<File>>,
    /// Escritor con búfer (solo en modo "escribir" o "agregar").
    pub escritor: Option<BufWriter<File>>,
}

impl Clone for Valor {
    fn clone(&self) -> Self {
        match self {
            Valor::Entero(n) => Valor::Entero(*n),
            Valor::Decimal(n) => Valor::Decimal(*n),
            Valor::Real(n) => Valor::Real(*n),
            Valor::Logico(b) => Valor::Logico(*b),
            Valor::Texto(s) => Valor::Texto(s.clone()),
            Valor::Arreglo(v) => Valor::Arreglo(v.clone()),
            Valor::Lista(v) => Valor::Lista(v.clone()),
            Valor::Nulo => Valor::Nulo,
            Valor::Archivo(arc) => Valor::Archivo(Arc::clone(arc)),
            Valor::Modulo(rc) => Valor::Modulo(Arc::clone(rc)),
            Valor::Funcion(s) => Valor::Funcion(s.clone()),
        }
    }
}

impl std::fmt::Debug for Valor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Valor::Entero(n) => write!(f, "Entero({n})"),
            Valor::Decimal(n) => write!(f, "Decimal({n})"),
            Valor::Real(n) => write!(f, "Real({n})"),
            Valor::Logico(b) => write!(f, "Logico({b})"),
            Valor::Texto(s) => write!(f, "Texto({s:?})"),
            Valor::Arreglo(v) => write!(f, "Arreglo({v:?})"),
            Valor::Lista(v) => write!(f, "Lista({v:?})"),
            Valor::Nulo => write!(f, "Nulo"),
            Valor::Archivo(_) => write!(f, "Archivo(<opaco>)"),
            Valor::Modulo(m) => write!(f, "Modulo({})", m.ruta.display()),
            Valor::Funcion(s) => write!(f, "Funcion({s})"),
        }
    }
}

impl PartialEq for Valor {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Valor::Entero(a), Valor::Entero(b)) => a == b,
            (Valor::Decimal(a), Valor::Decimal(b)) => a == b,
            (Valor::Real(a), Valor::Real(b)) => a == b,
            (Valor::Logico(a), Valor::Logico(b)) => a == b,
            (Valor::Texto(a), Valor::Texto(b)) => a == b,
            (Valor::Arreglo(a), Valor::Arreglo(b)) => a == b,
            (Valor::Lista(a), Valor::Lista(b)) => a == b,
            (Valor::Nulo, Valor::Nulo) => true,
            // Archivos se comparan por identidad de puntero (Arc).
            (Valor::Archivo(a), Valor::Archivo(b)) => Arc::ptr_eq(a, b),
            (Valor::Modulo(a), Valor::Modulo(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// Motivos de error aritmético, alineados con `ErrorEjecucion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotivoAritmetico {
    DesbordamientoEntero,
    DivisionPorCero,
    ExcesoRangoDecimal,
    ConversionIlegal,
    /// Operación aritmética entre tipos incompatibles (p. ej. lógico con numérico).
    TiposIncompatibles,
}

impl fmt::Display for MotivoAritmetico {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MotivoAritmetico::DesbordamientoEntero => "desbordamiento entero",
            MotivoAritmetico::DivisionPorCero => "división entre cero",
            MotivoAritmetico::ExcesoRangoDecimal => "exceso de rango decimal",
            MotivoAritmetico::ConversionIlegal => "conversión ilegal",
            MotivoAritmetico::TiposIncompatibles => "tipos incompatibles",
        };
        write!(f, "{s}")
    }
}

// ---------------------------------------------------------------- Constructores

impl Valor {
    /// Literal entero (`42` → Entero). Falla si no cabe en i64.
    pub fn entero_desde_lexema(lex: &str) -> Result<Valor, MotivoAritmetico> {
        lex.parse::<i64>()
            .map(Valor::Entero)
            .map_err(|_| MotivoAritmetico::DesbordamientoEntero)
    }

    /// Literal decimal (`42.5` → Decimal escalonado con escala 9). Falla si excede el rango.
    /// Exceso de precisión (>9 decimales) se redondea half-even (ADR-0005 §1.2).
    pub fn decimal_desde_lexema(lex: &str) -> Result<Valor, MotivoAritmetico> {
        let (ent, frac) = lex.split_once('.').unwrap_or((lex, ""));
        let ent: i128 = ent
            .parse()
            .map_err(|_| MotivoAritmetico::ExcesoRangoDecimal)?;
        // Parte fraccionaria: hasta 9 dígitos exactos; si hay más, se redondea half-even.
        let (nueve, cola) = if frac.len() > 9 {
            frac.split_at(9)
        } else {
            (frac, "")
        };
        let mut frac9 = String::with_capacity(9);
        frac9.push_str(nueve);
        while frac9.len() < 9 {
            frac9.push('0');
        }
        let mut frac: i128 = frac9
            .parse()
            .map_err(|_| MotivoAritmetico::ExcesoRangoDecimal)?;
        if !cola.is_empty() {
            let d1 = cola.as_bytes()[0] - b'0';
            let pegajoso = cola[1..].bytes().any(|c| c != b'0');
            if d1 > 5 || (d1 == 5 && (pegajoso || frac % 2 == 1)) {
                frac += 1; // llega como mucho a 10^9; la validación de rango lo detecta.
            }
        }
        let ent = ent
            .checked_abs()
            .ok_or(MotivoAritmetico::ExcesoRangoDecimal)?;
        let mantisa = ent
            .checked_mul(FACTOR_ESCALA)
            .and_then(|e| e.checked_add(frac))
            .ok_or(MotivoAritmetico::ExcesoRangoDecimal)?;
        validar_rango_decimal(mantisa)?;
        let mantisa = if lex.starts_with('-') {
            -mantisa
        } else {
            mantisa
        };
        Ok(Valor::Decimal(mantisa))
    }

    /// Literal real (`1e10`, `2.5e-3` → Real f64).
    pub fn real_desde_lexema(lex: &str) -> Result<Valor, MotivoAritmetico> {
        lex.parse::<f64>()
            .map(Valor::Real)
            .map_err(|_| MotivoAritmetico::ConversionIlegal)
    }

    /// Crea un decimal desde un número entero (para conversiones y pruebas).
    pub fn decimal_desde_entero(v: i64) -> Valor {
        Valor::Decimal((v as i128) * FACTOR_ESCALA)
    }

    /// Nombre del tipo para diagnósticos.
    pub fn nombre_tipo(&self) -> &'static str {
        match self {
            Valor::Entero(_) => "entero",
            Valor::Decimal(_) => "decimal",
            Valor::Real(_) => "real",
            Valor::Logico(_) => "lógico",
            Valor::Texto(_) => "texto",
            Valor::Arreglo(_) => "arreglo",
            Valor::Lista(_) => "lista",
            Valor::Nulo => "nulo",
            Valor::Archivo(_) => "archivo",
            Valor::Modulo(_) => "modulo",
            Valor::Funcion(_) => "función",
        }
    }
}

// ---------------------------------------------------------------- Decimal: redondeo

/// Redondea `x` hacia el par más cercano (half-even / banquero) al múltiplo de `factor`.
/// Precondición: `factor > 0`. Sin desbordamiento en los usos del núcleo.
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
    let r = i128::try_from(r).unwrap_or(i128::MAX);
    if negativo { -r } else { r }
}

// ---------------------------------------------------------------- Operaciones

/// Suma/resta de decimales: mismos operandos con escala 9, resultado escala 9 sin redondeo.
fn combinar_decimales(
    a: i128,
    b: i128,
    op: fn(i128, i128) -> Option<i128>,
) -> Result<i128, MotivoAritmetico> {
    let bruto = op(a, b).ok_or(MotivoAritmetico::ExcesoRangoDecimal)?;
    validar_rango_decimal(bruto)?;
    Ok(bruto)
}

/// Valida que el decimal escalonado quepa en DECIMAL(28,9): |entero| ≤ 10^19 - 1,
/// es decir |mantisa| ≤ 10^28 - 1 (28 dígitos en total).
fn validar_rango_decimal(mantisa: i128) -> Result<(), MotivoAritmetico> {
    let abs = mantisa.unsigned_abs();
    let limite = (DIEZ_POT_19 as u128) * (FACTOR_ESCALA as u128) - 1;
    if abs > limite {
        return Err(MotivoAritmetico::ExcesoRangoDecimal);
    }
    Ok(())
}

fn suma_dec(a: i128, b: i128) -> Option<i128> {
    a.checked_add(b)
}
fn resta_dec(a: i128, b: i128) -> Option<i128> {
    a.checked_sub(b)
}
fn mul_dec(a: i128, b: i128) -> Option<i128> {
    // (a/10^9) * (b/10^9) → para escala 9: mantisa = round(a·b / 10^9) con half-even.
    // redondear_half_even devuelve el múltiplo de 10^9; el cociente exacto es la mantisa.
    let bruto = a.checked_mul(b)?;
    Some(redondear_half_even(bruto, FACTOR_ESCALA) / FACTOR_ESCALA)
}

/// Divide dos decimales (ambos escalonados a escala 9) → resultado escala 9, half-even.
/// `10.0 / 4.0` = `2.500000000`; `1.0 / 3.0` = `0.333333333`.
fn division_decimal(a: i128, b: i128) -> Result<i128, MotivoAritmetico> {
    if b == 0 {
        return Err(MotivoAritmetico::DivisionPorCero);
    }
    // a/10^9 ÷ b/10^9 = a/b (real). Para escala 9: redondear (a * 10^9) / b.
    let numerador = a
        .checked_mul(FACTOR_ESCALA)
        .ok_or(MotivoAritmetico::ExcesoRangoDecimal)?;
    let bruto = numerador
        .checked_div(b)
        .ok_or(MotivoAritmetico::ExcesoRangoDecimal)?;
    // Redondeo half-even sobre el residuo para corregir el truncado de checked_div.
    // Criterio: comparar 2·|resto| con |b| (|resto| < |b| siempre).
    let resto = numerador
        .checked_rem(b)
        .ok_or(MotivoAritmetico::ExcesoRangoDecimal)?;
    let doble_resto = resto.unsigned_abs().checked_mul(2);
    let abs_b = b.unsigned_abs();
    let ajuste = match doble_resto {
        Some(dr) if dr > abs_b => 1,
        // Empate exacto: half-even sobre la magnitud del cociente truncado.
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
    /// Aritmética de suma según los tipos (D1 §1.4, 16).
    pub fn suma(&self, o: &Valor) -> Result<Valor, MotivoAritmetico> {
        match (self, o) {
            (Valor::Entero(a), Valor::Entero(b)) => a
                .checked_add(*b)
                .map(Valor::Entero)
                .ok_or(MotivoAritmetico::DesbordamientoEntero),
            (Valor::Entero(a), Valor::Decimal(b)) => {
                let a = Valor::decimal_desde_entero(*a);
                let Valor::Decimal(am) = a else {
                    unreachable!()
                };
                combinar_decimales(am, *b, suma_dec).map(Valor::Decimal)
            }
            (Valor::Decimal(a), Valor::Entero(b)) => {
                let b = Valor::decimal_desde_entero(*b);
                let Valor::Decimal(bm) = b else {
                    unreachable!()
                };
                combinar_decimales(*a, bm, suma_dec).map(Valor::Decimal)
            }
            (Valor::Decimal(a), Valor::Decimal(b)) => {
                combinar_decimales(*a, *b, suma_dec).map(Valor::Decimal)
            }
            (Valor::Entero(a), Valor::Real(b)) => Ok(Valor::Real(*a as f64 + *b)),
            (Valor::Real(a), Valor::Entero(b)) => Ok(Valor::Real(*a + *b as f64)),
            (Valor::Real(a), Valor::Real(b)) => Ok(Valor::Real(a + b)),
            (Valor::Decimal(_), Valor::Real(_)) | (Valor::Real(_), Valor::Decimal(_)) => {
                Err(MotivoAritmetico::ConversionIlegal)
            }
            // Concatenación de texto (Texto + Texto).
            (Valor::Texto(a), Valor::Texto(b)) => Ok(Valor::Texto(format!("{a}{b}"))),
            // Lógicos no participan en aritmética (ADR-0007 §1.1).
            _ => Err(MotivoAritmetico::TiposIncompatibles),
        }
    }

    pub fn resta(&self, o: &Valor) -> Result<Valor, MotivoAritmetico> {
        match (self, o) {
            (Valor::Entero(a), Valor::Entero(b)) => a
                .checked_sub(*b)
                .map(Valor::Entero)
                .ok_or(MotivoAritmetico::DesbordamientoEntero),
            (Valor::Entero(a), Valor::Decimal(b)) => {
                let Valor::Decimal(am) = Valor::decimal_desde_entero(*a) else {
                    unreachable!()
                };
                combinar_decimales(am, *b, resta_dec).map(Valor::Decimal)
            }
            (Valor::Decimal(a), Valor::Entero(b)) => {
                let Valor::Decimal(bm) = Valor::decimal_desde_entero(*b) else {
                    unreachable!()
                };
                combinar_decimales(*a, bm, resta_dec).map(Valor::Decimal)
            }
            (Valor::Decimal(a), Valor::Decimal(b)) => {
                combinar_decimales(*a, *b, resta_dec).map(Valor::Decimal)
            }
            (Valor::Entero(a), Valor::Real(b)) => Ok(Valor::Real(*a as f64 - *b)),
            (Valor::Real(a), Valor::Entero(b)) => Ok(Valor::Real(*a - *b as f64)),
            (Valor::Real(a), Valor::Real(b)) => Ok(Valor::Real(a - b)),
            (Valor::Decimal(_), Valor::Real(_)) | (Valor::Real(_), Valor::Decimal(_)) => {
                Err(MotivoAritmetico::ConversionIlegal)
            }
            // Lógicos no participan en aritmética (ADR-0007 §1.1).
            _ => Err(MotivoAritmetico::TiposIncompatibles),
        }
    }

    pub fn multiplicacion(&self, o: &Valor) -> Result<Valor, MotivoAritmetico> {
        match (self, o) {
            (Valor::Entero(a), Valor::Entero(b)) => a
                .checked_mul(*b)
                .map(Valor::Entero)
                .ok_or(MotivoAritmetico::DesbordamientoEntero),
            (Valor::Entero(a), Valor::Decimal(b)) | (Valor::Decimal(b), Valor::Entero(a)) => {
                let Valor::Decimal(am) = Valor::decimal_desde_entero(*a) else {
                    unreachable!()
                };
                combinar_decimales(am, *b, mul_dec).map(Valor::Decimal)
            }
            (Valor::Decimal(a), Valor::Decimal(b)) => {
                combinar_decimales(*a, *b, mul_dec).map(Valor::Decimal)
            }
            (Valor::Entero(a), Valor::Real(b)) => Ok(Valor::Real(*a as f64 * *b)),
            (Valor::Real(a), Valor::Entero(b)) => Ok(Valor::Real(*a * *b as f64)),
            (Valor::Real(a), Valor::Real(b)) => Ok(Valor::Real(a * b)),
            (Valor::Decimal(_), Valor::Real(_)) | (Valor::Real(_), Valor::Decimal(_)) => {
                Err(MotivoAritmetico::ConversionIlegal)
            }
            // Lógicos no participan en aritmética (ADR-0007 §1.1).
            _ => Err(MotivoAritmetico::TiposIncompatibles),
        }
    }

    /// División: `entero/entero` = división entera hacia cero; `decimal` y `real` = división real.
    /// División por cero = error en los tres tipos (D3).
    pub fn division(&self, o: &Valor) -> Result<Valor, MotivoAritmetico> {
        match (self, o) {
            (Valor::Entero(a), Valor::Entero(b)) => {
                if *b == 0 {
                    return Err(MotivoAritmetico::DivisionPorCero);
                }
                if *a == i64::MIN && *b == -1 {
                    return Err(MotivoAritmetico::DesbordamientoEntero);
                }
                Ok(Valor::Entero(a / b))
            }
            (Valor::Entero(a), Valor::Decimal(b)) => {
                let Valor::Decimal(am) = Valor::decimal_desde_entero(*a) else {
                    unreachable!()
                };
                division_decimal(am, *b).map(Valor::Decimal)
            }
            (Valor::Decimal(a), Valor::Entero(b)) => {
                let Valor::Decimal(bm) = Valor::decimal_desde_entero(*b) else {
                    unreachable!()
                };
                division_decimal(*a, bm).map(Valor::Decimal)
            }
            (Valor::Decimal(a), Valor::Decimal(b)) => division_decimal(*a, *b).map(Valor::Decimal),
            (Valor::Entero(a), Valor::Real(b)) => {
                if *b == 0.0 {
                    return Err(MotivoAritmetico::DivisionPorCero);
                }
                Ok(Valor::Real(*a as f64 / *b))
            }
            (Valor::Real(a), Valor::Entero(b)) => {
                if *b == 0 {
                    return Err(MotivoAritmetico::DivisionPorCero);
                }
                Ok(Valor::Real(*a / *b as f64))
            }
            (Valor::Real(a), Valor::Real(b)) => {
                if *b == 0.0 {
                    return Err(MotivoAritmetico::DivisionPorCero);
                }
                Ok(Valor::Real(a / b))
            }
            (Valor::Decimal(_), Valor::Real(_)) | (Valor::Real(_), Valor::Decimal(_)) => {
                Err(MotivoAritmetico::ConversionIlegal)
            }
            // Lógicos no participan en aritmética (ADR-0007 §1.1).
            _ => Err(MotivoAritmetico::TiposIncompatibles),
        }
    }

    /// Módulo entero: resto de la división truncada, signo del dividendo (D4).
    /// `i64::MIN % -1` = 0 (el resto cabe). Divisor cero = error.
    pub fn modulo(&self, o: &Valor) -> Result<Valor, MotivoAritmetico> {
        match (self, o) {
            (Valor::Entero(a), Valor::Entero(b)) => {
                if *b == 0 {
                    return Err(MotivoAritmetico::DivisionPorCero);
                }
                if *a == i64::MIN && *b == -1 {
                    return Ok(Valor::Entero(0));
                }
                Ok(Valor::Entero(a % b))
            }
            _ => Err(MotivoAritmetico::ConversionIlegal),
        }
    }

    /// Igualdad exacta entre valores; requiere tipo común ya resuelto por el operador.
    pub fn es_cero(&self) -> bool {
        match self {
            Valor::Entero(a) => *a == 0,
            Valor::Decimal(a) => *a == 0,
            Valor::Real(a) => *a == 0.0,
            Valor::Logico(_) => false,
            Valor::Texto(_) => false,
            Valor::Arreglo(_) => false,
            Valor::Lista(_) => false,
            Valor::Nulo => false,
            Valor::Archivo(_) => false,
            Valor::Modulo(_) => false,
            Valor::Funcion(_) => false,
        }
    }

    pub fn a_real(&self) -> f64 {
        match self {
            Valor::Entero(a) => *a as f64,
            Valor::Decimal(a) => *a as f64 / FACTOR_ESCALA as f64,
            Valor::Real(a) => *a,
            // Un lógico no tiene representación numérica (ADR-0007 §1.1).
            Valor::Logico(_) => f64::NAN,
            // Texto no tiene representación numérica.
            Valor::Texto(_) => f64::NAN,
            // Arreglos y listas no tienen representación numérica.
            Valor::Arreglo(_) => f64::NAN,
            Valor::Lista(_) => f64::NAN,
            // Nulo no tiene representación numérica (ADR-0011 §3.5).
            Valor::Nulo => f64::NAN,
            // Archivo no tiene representación numérica.
            Valor::Archivo(_) => f64::NAN,
            // Módulo no tiene representación numérica.
            Valor::Modulo(_) => f64::NAN,
            // Función no tiene representación numérica.
            Valor::Funcion(_) => f64::NAN,
        }
    }
}

impl fmt::Display for Valor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Valor::Entero(a) => write!(f, "{a}"),
            Valor::Decimal(m) => {
                // Imprime con escala 9 pero recorta ceros finales, dejando al menos un decimal.
                let neg = *m < 0;
                let a = m.unsigned_abs();
                let ent = a / (FACTOR_ESCALA as u128);
                let frac = a % (FACTOR_ESCALA as u128);
                let mut frac_s = format!("{frac:09}");
                while frac_s.ends_with('0') && frac_s.len() > 1 {
                    frac_s.pop();
                }
                if neg {
                    write!(f, "-{ent}.{frac_s}")
                } else {
                    write!(f, "{ent}.{frac_s}")
                }
            }
            Valor::Real(a) => write!(f, "{a}"),
            Valor::Logico(true) => write!(f, "verdadero"),
            Valor::Logico(false) => write!(f, "falso"),
            Valor::Texto(s) => write!(f, "{s}"),
            Valor::Arreglo(v) => {
                write!(f, "[")?;
                for (i, elem) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "]")
            }
            Valor::Lista(v) => {
                write!(f, "[")?;
                for (i, elem) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "]")
            }
            Valor::Nulo => write!(f, "nulo"),
            Valor::Archivo(_) => write!(f, "<archivo>"),
            Valor::Modulo(m) => write!(f, "<modulo: {}>", m.ruta.display()),
            Valor::Funcion(s) => write!(f, "<función: {s}>"),
        }
    }
}

impl Valor {
    /// Auxiliar de test: extrae la mantisa decimal.
    #[cfg(test)]
    fn unwrap_decimal(&self) -> i128 {
        match self {
            Valor::Decimal(m) => *m,
            _ => panic!("no es decimal"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literales_tipados() {
        assert_eq!(Valor::entero_desde_lexema("42"), Ok(Valor::Entero(42)));
        assert_eq!(
            Valor::decimal_desde_lexema("42.5"),
            Ok(Valor::Decimal(42_500_000_000))
        );
        assert_eq!(
            Valor::decimal_desde_lexema("0.10"),
            Ok(Valor::Decimal(100_000_000))
        );
        assert_eq!(Valor::real_desde_lexema("1e10"), Ok(Valor::Real(1e10)));
    }

    #[test]
    fn decimal_suma_exacta() {
        // 0.1 + 0.2 == 0.3 exactamente (nunca 0.30000000000000004).
        let a = Valor::decimal_desde_lexema("0.1").unwrap();
        let b = Valor::decimal_desde_lexema("0.2").unwrap();
        let s = a.suma(&b).unwrap();
        assert_eq!(s, Valor::Decimal(300_000_000));
        assert_eq!(s.to_string(), "0.3");
    }

    #[test]
    fn decimal_conserva_escala() {
        let a = Valor::decimal_desde_lexema("0.10").unwrap();
        let s = a.suma(&a).unwrap().unwrap_decimal();
        assert_eq!(s, 200_000_000); // 0.20
    }

    #[test]
    fn division_decimal_media() {
        // 10.0 / 4.0 = 2.5
        let a = Valor::decimal_desde_lexema("10.0").unwrap();
        let b = Valor::decimal_desde_lexema("4.0").unwrap();
        assert_eq!(a.division(&b).unwrap().to_string(), "2.5");
    }

    #[test]
    fn multiplicacion_decimal_escala_correcta() {
        // Regresión del defecto D-5: mul_dec devolvía el múltiplo de 10^9 en vez
        // del cociente, inflando el resultado en 10^9.
        let a = Valor::decimal_desde_lexema("2.5").unwrap();
        let b = Valor::decimal_desde_lexema("4.0").unwrap();
        assert_eq!(a.multiplicacion(&b).unwrap().to_string(), "10.0");
        // Redondeo half-even en el resultado: 0.000000003 * 0.5 = 0.0000000015 → 0.000000002.
        let a = Valor::decimal_desde_lexema("0.000000003").unwrap();
        let b = Valor::decimal_desde_lexema("0.5").unwrap();
        assert_eq!(a.multiplicacion(&b).unwrap().to_string(), "0.000000002");
        // Empate con cociente par: 0.000000005 * 0.5 = 0.0000000025 → 0.000000002.
        let a = Valor::decimal_desde_lexema("0.000000005").unwrap();
        assert_eq!(a.multiplicacion(&b).unwrap().to_string(), "0.000000002");
    }

    #[test]
    fn division_por_cero_errores() {
        let e = Valor::Entero(10);
        let cero_e = Valor::Entero(0);
        assert_eq!(e.division(&cero_e), Err(MotivoAritmetico::DivisionPorCero));
        let cero_d = Valor::Decimal(0);
        assert_eq!(
            Valor::Decimal(1).division(&cero_d),
            Err(MotivoAritmetico::DivisionPorCero)
        );
        let cero_r = Valor::Real(0.0);
        assert_eq!(
            Valor::Real(1.0).division(&cero_r),
            Err(MotivoAritmetico::DivisionPorCero)
        );
    }

    #[test]
    fn modulo_signo_del_dividendo() {
        assert_eq!(
            Valor::Entero(-7).modulo(&Valor::Entero(3)),
            Ok(Valor::Entero(-1))
        );
        assert_eq!(
            Valor::Entero(7).modulo(&Valor::Entero(-3)),
            Ok(Valor::Entero(1))
        );
        assert_eq!(
            Valor::Entero(-7).modulo(&Valor::Entero(-3)),
            Ok(Valor::Entero(-1))
        );
        assert_eq!(
            Valor::Entero(6).modulo(&Valor::Entero(3)),
            Ok(Valor::Entero(0))
        );
    }

    #[test]
    fn min_menos_uno_modulo_es_cero() {
        assert_eq!(
            Valor::Entero(i64::MIN).modulo(&Valor::Entero(-1)),
            Ok(Valor::Entero(0))
        );
        assert_eq!(
            Valor::Entero(i64::MIN).division(&Valor::Entero(-1)),
            Err(MotivoAritmetico::DesbordamientoEntero)
        );
    }

    #[test]
    fn desbordamiento_entero_es_error() {
        assert_eq!(
            Valor::Entero(i64::MAX).suma(&Valor::Entero(1)),
            Err(MotivoAritmetico::DesbordamientoEntero)
        );
        assert_eq!(
            Valor::Entero(i64::MIN).resta(&Valor::Entero(1)),
            Err(MotivoAritmetico::DesbordamientoEntero)
        );
        assert_eq!(
            Valor::Entero(i64::MAX).multiplicacion(&Valor::Entero(2)),
            Err(MotivoAritmetico::DesbordamientoEntero)
        );
    }

    #[test]
    fn decimal_real_no_se_combinan() {
        assert_eq!(
            Valor::Decimal(1).suma(&Valor::Real(1.0)),
            Err(MotivoAritmetico::ConversionIlegal)
        );
    }

    #[test]
    fn entero_a_decimal_es_implicito() {
        // 2 + 3.5 = 5.5 (entero se convierte a decimal sin pérdida).
        let r = Valor::Entero(2).suma(&Valor::decimal_desde_lexema("3.5").unwrap());
        assert_eq!(r.unwrap().to_string(), "5.5");
    }
}
