//! Evaluador del prototipo de Nexo.
//!
//! Semántica numérica normativa (ADR-0005, aprobado):
//! - Tres tipos: `entero` (i64), `decimal` (DECIMAL(28,9)), `real` (f64).
//! - Desbordamiento entero = error; división por cero = error en los tres tipos.
//! - `%` de enteros = resto truncado, signo del dividendo; `i64::MIN % -1` = 0.
//! - Decimal: redondeo half-even, exceso de precisión redondea, exceso de rango = error.
//! - Conversiones: solo implícitas sin pérdida (`entero→decimal`, `entero→real`).

use std::collections::HashMap;

use crate::parser::{Expresion, Sentencia};
use crate::valor::{FACTOR_ESCALA, MotivoAritmetico, Valor};

/// Causa tipada de un error de ejecución (ADR-0005 §1.6 y ADR-0006 §1.5, ampliada ADR-0007).
#[derive(Debug, Clone, PartialEq)]
pub enum CausaError {
    /// Motivo aritmético o de tipos del sistema numérico (ADR-0005).
    Aritmetica(MotivoAritmetico),
    /// Referencia o asignación a un nombre no declarado (ADR-0006).
    VariableNoDefinida(String),
    /// `sea nombre` repetido en el mismo ámbito (ADR-0006).
    DeclaracionDuplicada(String),
    /// Asignación con un tipo distinto al declarado y sin conversión implícita.
    AsignacionIncompatible {
        esperado: &'static str,
        obtenido: &'static str,
    },
    /// Condición de `si`/`mientras` no es `lógico` (ADR-0007).
    CondicionNoLogica { obtenido: &'static str },
    /// Operando de `no`/`y`/`o` no es `lógico` (ADR-0007).
    OperandoNoLogico { operador: &'static str, obtenido: &'static str },
    /// Comparación con tipos incompatibles sin conversión implícita aprobada (ADR-0007).
    TiposIncompatibles { izquierdo: &'static str, derecho: &'static str },
    /// Se superó el límite de iteraciones de un `mientras` (protección frente a bucles
    /// infinitos; ADR-0007 §1.5).
    LimiteIteraciones { limite: usize },
}

/// Límite de iteraciones por ciclo `mientras` (protección frente a bucles infinitos).
pub const LIMITE_ITERACIONES: usize = 100_000;

impl std::fmt::Display for CausaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CausaError::Aritmetica(m) => write!(f, "{m}"),
            CausaError::VariableNoDefinida(n) => write!(f, "variable no definida: {n}"),
            CausaError::DeclaracionDuplicada(n) => write!(f, "declaración duplicada: {n}"),
            CausaError::AsignacionIncompatible { esperado, obtenido } => write!(
                f,
                "asignación incompatible: se esperaba {esperado} y se obtuvo {obtenido}"
            ),
            CausaError::CondicionNoLogica { obtenido } => write!(
                f,
                "la condición debe ser lógica, se obtuvo {obtenido}"
            ),
            CausaError::OperandoNoLogico { operador, obtenido } => write!(
                f,
                "el operando de '{operador}' debe ser lógico, se obtuvo {obtenido}"
            ),
            CausaError::TiposIncompatibles { izquierdo, derecho } => write!(
                f,
                "comparación incompatible entre {izquierdo} y {derecho}"
            ),
            CausaError::LimiteIteraciones { limite } => write!(
                f,
                "se superó el límite de {limite} iteraciones: posible bucle infinito"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorEjecucion {
    pub mensaje: String,
    pub causa: CausaError,
    pub linea: usize,
    pub columna: usize,
}

impl std::fmt::Display for ErrorEjecucion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error de ejecución [{}:{}]: {}",
            self.linea, self.columna, self.mensaje
        )
    }
}

fn error(causa: CausaError, linea: usize, columna: usize) -> ErrorEjecucion {
    ErrorEjecucion {
        mensaje: causa.to_string(),
        causa,
        linea,
        columna,
    }
}

/// Entorno de ejecución: pila de ámbitos con las variables (ADR-0006 §1.6).
#[derive(Debug, Default)]
pub struct Entorno {
    ambitos: Vec<HashMap<String, Valor>>,
}

impl Entorno {
    pub fn nueva() -> Self {
        Entorno {
            ambitos: vec![HashMap::new()],
        }
    }

    /// Evalúa una expresión contra este entorno.
    pub fn evaluar_expresion(&mut self, e: &Expresion) -> Result<Valor, ErrorEjecucion> {
        evaluar_con(e, self)
    }

    /// Ejecuta un programa sentencia a sentencia.
    /// Devuelve el valor de la última sentencia de expresión **del nivel superior**, si la hay.
    /// Las sentencias internas de bloques, ramas o ciclos no producen valor hacia fuera
    /// (ADR-0007 §1.6).
    pub fn ejecutar_programa(
        &mut self,
        sentencias: &[Sentencia],
    ) -> Result<Option<Valor>, ErrorEjecucion> {
        self.ejecutar_sentencias(sentencias)
    }

    /// Ejecuta el cuerpo de un bloque en un ámbito propio (ADR-0007 §1.6).
    /// El valor de las expresiones internas no se propaga al exterior.
    fn ejecutar_bloque(&mut self, sentencias: &[Sentencia]) -> Result<(), ErrorEjecucion> {
        self.ambitos.push(HashMap::new());
        let r = self.ejecutar_sentencias(sentencias);
        self.ambitos.pop();
        r.map(|_| ())
    }

    fn ejecutar_sentencias(
        &mut self,
        sentencias: &[Sentencia],
    ) -> Result<Option<Valor>, ErrorEjecucion> {
        let mut ultimo = None;
        for s in sentencias {
            match s {
                Sentencia::Declaracion {
                    nombre,
                    linea,
                    columna,
                    valor,
                } => {
                    if self.buscar_ambito_actual(nombre).is_some() {
                        return Err(error(
                            CausaError::DeclaracionDuplicada(nombre.clone()),
                            *linea,
                            *columna,
                        ));
                    }
                    let v = evaluar_con(valor, self)?;
                    self.ambitos
                        .last_mut()
                        .expect("siempre hay un ámbito")
                        .insert(nombre.clone(), v);
                }
                Sentencia::Asignacion {
                    nombre,
                    linea,
                    columna,
                    valor,
                } => {
                    let nuevo = evaluar_con(valor, self)?;
                    let Some((idx, viejo)) = self.localizar(nombre) else {
                        return Err(error(
                            CausaError::VariableNoDefinida(nombre.clone()),
                            *linea,
                            *columna,
                        ));
                    };
                    let Some(ajustado) = convertir_para_tipo(viejo, nuevo) else {
                        return Err(error(
                            CausaError::AsignacionIncompatible {
                                esperado: viejo.nombre_tipo(),
                                obtenido: nuevo.nombre_tipo(),
                            },
                            *linea,
                            *columna,
                        ));
                    };
                    self.ambitos[idx].insert(nombre.clone(), ajustado);
                }
                Sentencia::Si {
                    condicion,
                    rama_si,
                    rama_sino,
                    ..
                } => {
                    let c = evaluar_con(condicion, self)?;
                    let es_verdadero = match c {
                        Valor::Logico(b) => b,
                        _ => {
                            let (linea, columna) = ubicacion(condicion);
                            return Err(error(
                                CausaError::CondicionNoLogica {
                                    obtenido: c.nombre_tipo(),
                                },
                                linea,
                                columna,
                            ));
                        }
                    };
                    if es_verdadero {
                        self.ejecutar_bloque(rama_si)?;
                    } else if let Some(rama) = rama_sino {
                        self.ejecutar_bloque(rama)?;
                    }
                }
                Sentencia::Mientras {
                    condicion,
                    cuerpo,
                    ..
                } => {
                    let mut iteraciones = 0usize;
                    loop {
                        let c = evaluar_con(condicion, self)?;
                        let es_verdadero = match c {
                            Valor::Logico(b) => b,
                            _ => {
                                let (linea, columna) = ubicacion(condicion);
                                return Err(error(
                                    CausaError::CondicionNoLogica {
                                        obtenido: c.nombre_tipo(),
                                    },
                                    linea,
                                    columna,
                                ));
                            }
                        };
                        if !es_verdadero {
                            break;
                        }
                        iteraciones += 1;
                        if iteraciones > LIMITE_ITERACIONES {
                            let (linea, columna) = ubicacion(condicion);
                            return Err(error(
                                CausaError::LimiteIteraciones {
                                    limite: LIMITE_ITERACIONES,
                                },
                                linea,
                                columna,
                            ));
                        }
                        self.ejecutar_bloque(cuerpo)?;
                    }
                }
                Sentencia::Bloque(sentencias) => {
                    self.ejecutar_bloque(sentencias)?;
                }
                Sentencia::Expresion(e) => {
                    ultimo = Some(evaluar_con(e, self)?);
                }
            }
        }
        Ok(ultimo)
    }

    /// Valor de una variable visible (para pruebas e introspección).
    pub fn variable(&self, nombre: &str) -> Option<Valor> {
        self.buscar(nombre).copied()
    }

    fn buscar_ambito_actual(&self, nombre: &str) -> Option<&Valor> {
        self.ambitos.last().and_then(|a| a.get(nombre))
    }

    fn localizar(&self, nombre: &str) -> Option<(usize, &Valor)> {
        self.ambitos
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, a)| a.get(nombre).map(|v| (i, v)))
    }

    fn buscar(&self, nombre: &str) -> Option<&Valor> {
        self.localizar(nombre).map(|(_, v)| v)
    }
}

/// Aplica a `nuevo` las conversiones implícitas permitidas hacia el tipo de la variable
/// (ADR-0005 §1.4 usadas por asignación, ADR-0006 §1.4). `None` = incompatible.
fn convertir_para_tipo(viejo: &Valor, nuevo: Valor) -> Option<Valor> {
    match (viejo, &nuevo) {
        (Valor::Decimal(_), Valor::Entero(n)) => Some(Valor::decimal_desde_entero(*n)),
        (Valor::Real(_), Valor::Entero(n)) => Some(Valor::Real(*n as f64)),
        _ if std::mem::discriminant(viejo) == std::mem::discriminant(&nuevo) => Some(nuevo),
        _ => None,
    }
}
/// Evalúa una expresión con acceso al entorno (función recursiva interna).
fn evaluar_con(e: &Expresion, env: &mut Entorno) -> Result<Valor, ErrorEjecucion> {
    match e {
        Expresion::Literal { valor, .. } => Ok(*valor),
        Expresion::Variable {
            nombre,
            linea,
            columna,
        } => env
            .buscar(nombre)
            .copied()
            .ok_or_else(|| error(CausaError::VariableNoDefinida(nombre.clone()), *linea, *columna)),
        Expresion::Suma(a, b) => binaria(a, b, env, |x, y| x.suma(y)),
        Expresion::Resta(a, b) => binaria(a, b, env, |x, y| x.resta(y)),
        Expresion::Multiplicacion(a, b) => binaria(a, b, env, |x, y| x.multiplicacion(y)),
        Expresion::Division(a, b) => binaria(a, b, env, |x, y| x.division(y)),
        Expresion::Modulo(a, b) => binaria(a, b, env, |x, y| x.modulo(y)),
        Expresion::IgualIgual(a, b) => comparar(a, b, env, |ord| ord == std::cmp::Ordering::Equal),
        Expresion::Distinto(a, b) => comparar(a, b, env, |ord| ord != std::cmp::Ordering::Equal),
        Expresion::Menor(a, b) => comparar(a, b, env, |ord| ord == std::cmp::Ordering::Less),
        Expresion::MenorIgual(a, b) => comparar(a, b, env, |ord| ord != std::cmp::Ordering::Greater),
        Expresion::Mayor(a, b) => comparar(a, b, env, |ord| ord == std::cmp::Ordering::Greater),
        Expresion::MayorIgual(a, b) => comparar(a, b, env, |ord| ord != std::cmp::Ordering::Less),
        Expresion::No(a) => {
            let v = evaluar_con(a, env)?;
            match v {
                Valor::Logico(b) => Ok(Valor::Logico(!b)),
                _ => {
                    let (linea, columna) = ubicacion(e);
                    Err(error(
                        CausaError::OperandoNoLogico {
                            operador: "no",
                            obtenido: v.nombre_tipo(),
                        },
                        linea,
                        columna,
                    ))
                }
            }
        }
        Expresion::Y(a, b) => {
            // Cortocircuito: si el operando izquierdo es `falso`, el derecho NO se evalúa
            // (ADR-0007 §1.2).
            let va = evaluar_con(a, env)?;
            match va {
                Valor::Logico(false) => Ok(Valor::Logico(false)),
                Valor::Logico(true) => {
                    let vb = evaluar_con(b, env)?;
                    match vb {
                        Valor::Logico(x) => Ok(Valor::Logico(x)),
                        _ => {
                            let (linea, columna) = ubicacion(b);
                            Err(error(
                                CausaError::OperandoNoLogico {
                                    operador: "y",
                                    obtenido: vb.nombre_tipo(),
                                },
                                linea,
                                columna,
                            ))
                        }
                    }
                }
                _ => {
                    let (linea, columna) = ubicacion(a);
                    Err(error(
                        CausaError::OperandoNoLogico {
                            operador: "y",
                            obtenido: va.nombre_tipo(),
                        },
                        linea,
                        columna,
                    ))
                }
            }
        }
        Expresion::O(a, b) => {
            // Cortocircuito: si el operando izquierdo es `verdadero`, el derecho NO se evalúa
            // (ADR-0007 §1.2).
            let va = evaluar_con(a, env)?;
            match va {
                Valor::Logico(true) => Ok(Valor::Logico(true)),
                Valor::Logico(false) => {
                    let vb = evaluar_con(b, env)?;
                    match vb {
                        Valor::Logico(x) => Ok(Valor::Logico(x)),
                        _ => {
                            let (linea, columna) = ubicacion(b);
                            Err(error(
                                CausaError::OperandoNoLogico {
                                    operador: "o",
                                    obtenido: vb.nombre_tipo(),
                                },
                                linea,
                                columna,
                            ))
                        }
                    }
                }
                _ => {
                    let (linea, columna) = ubicacion(a);
                    Err(error(
                        CausaError::OperandoNoLogico {
                            operador: "o",
                            obtenido: va.nombre_tipo(),
                        },
                        linea,
                        columna,
                    ))
                }
            }
        }
    }
}

/// Evalúa una expresión con un entorno vacío (compatibilidad con el núcleo previo).
pub fn evaluar(e: &Expresion) -> Result<Valor, ErrorEjecucion> {
    evaluar_con(e, &mut Entorno::nueva())
}

/// Ubicación aproximada de una expresión (literal o variable más a la izquierda).
fn ubicacion(e: &Expresion) -> (usize, usize) {
    match e {
        Expresion::Literal { linea, columna, .. }
        | Expresion::Variable {
            linea, columna, ..
        } => (*linea, *columna),
        Expresion::Suma(a, _)
        | Expresion::Resta(a, _)
        | Expresion::Multiplicacion(a, _)
        | Expresion::Division(a, _)
        | Expresion::Modulo(a, _)
        | Expresion::IgualIgual(a, _)
        | Expresion::Distinto(a, _)
        | Expresion::Menor(a, _)
        | Expresion::MenorIgual(a, _)
        | Expresion::Mayor(a, _)
        | Expresion::MayorIgual(a, _)
        | Expresion::No(a)
        | Expresion::Y(a, _)
        | Expresion::O(a, _) => ubicacion(a),
    }
}

fn binaria(
    a: &Expresion,
    b: &Expresion,
    env: &mut Entorno,
    op: impl Fn(&Valor, &Valor) -> Result<Valor, MotivoAritmetico>,
) -> Result<Valor, ErrorEjecucion> {
    let va = evaluar_con(a, env)?;
    let vb = evaluar_con(b, env)?;
    op(&va, &vb).map_err(|motivo| {
        let (linea, columna) = ubicacion(a);
        error(CausaError::Aritmetica(motivo), linea, columna)
    })
}

fn comparar(
    a: &Expresion,
    b: &Expresion,
    env: &mut Entorno,
    pred: impl Fn(std::cmp::Ordering) -> bool,
) -> Result<Valor, ErrorEjecucion> {
    let va = evaluar_con(a, env)?;
    let vb = evaluar_con(b, env)?;
    let (linea, columna) = ubicacion(a);
    if matches!(va, Valor::Logico(_)) || matches!(vb, Valor::Logico(_)) {
        // No se permite comparar lógicos con ==/!= arbitrarios entre numéricos aquí; tipos incompatibles
        return Err(error(
            CausaError::TiposIncompatibles {
                izquierdo: va.nombre_tipo(),
                derecho: vb.nombre_tipo(),
            },
            linea,
            columna,
        ));
    }
    // Conversión implícita aprobada ADR-0005 §1.4
    match (va, vb) {
        (Valor::Entero(x), Valor::Entero(y)) => Ok(Valor::Logico(pred(x.cmp(&y)))),
        (Valor::Entero(x), Valor::Decimal(y)) => {
            Ok(Valor::Logico(pred((x as i128 * FACTOR_ESCALA).cmp(&y))))
        }
        (Valor::Decimal(x), Valor::Entero(y)) => {
            Ok(Valor::Logico(pred(x.cmp(&(y as i128 * FACTOR_ESCALA)))))
        }
        (Valor::Entero(x), Valor::Real(y)) => Ok(Valor::Logico(pred((x as f64).partial_cmp(&y).ok_or_else(
            || error(CausaError::Aritmetica(MotivoAritmetico::ConversionIlegal), linea, columna),
        )?))),
        (Valor::Real(x), Valor::Entero(y)) => Ok(Valor::Logico(pred(x.partial_cmp(&(y as f64)).ok_or_else(
            || error(CausaError::Aritmetica(MotivoAritmetico::ConversionIlegal), linea, columna),
        )?))),
        (Valor::Decimal(x), Valor::Decimal(y)) => Ok(Valor::Logico(pred(x.cmp(&y)))),
        (Valor::Real(x), Valor::Real(y)) => Ok(Valor::Logico(pred(x.partial_cmp(&y).ok_or_else(
            || error(CausaError::Aritmetica(MotivoAritmetico::ConversionIlegal), linea, columna),
        )?))),
        (Valor::Decimal(_), Valor::Real(_)) | (Valor::Real(_), Valor::Decimal(_)) => {
            // Prohibido hasta conversión explícita (ADR-0007).
            Err(error(
                CausaError::TiposIncompatibles {
                    izquierdo: va.nombre_tipo(),
                    derecho: vb.nombre_tipo(),
                },
                linea,
                columna,
            ))
        }
        _ => Err(error(
            CausaError::TiposIncompatibles {
                izquierdo: va.nombre_tipo(),
                derecho: vb.nombre_tipo(),
            },
            linea,
            columna,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lexer, Parser};

    fn eval(fuente: &str) -> Result<Valor, ErrorEjecucion> {
        let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
        let e = Parser::nuevo(tokens)
            .parsear_expresion()
            .expect("sintaxis válida en test");
        evaluar(&e)
    }

    // ---- PN-INT: enteros ----

    #[test]
    fn pn_int_suma_basica() {
        assert_eq!(eval("10 + 5"), Ok(Valor::Entero(15)));
    }

    #[test]
    fn pn_int_division_trunca_hacia_cero() {
        assert_eq!(eval("7 / 2"), Ok(Valor::Entero(3)));
        assert_eq!(eval("-7 / 2"), Ok(Valor::Entero(-3)));
    }

    #[test]
    fn pn_int_desbordamiento_suma_es_error() {
        // PN-INT-OVF-1: MAX + 1 → error, no MIN.
        let r = eval("9223372036854775807 + 1");
        assert!(r.is_err());
        assert!(r.unwrap_err().mensaje.contains("desbordamiento"));
    }

    #[test]
    fn pn_int_desbordamiento_resta_y_mul() {
        assert!(eval("-9223372036854775808 - 1").is_err());
        assert!(eval("9223372036854775807 * 2").is_err());
    }

    #[test]
    fn pn_int_div_min_menos_uno_es_error() {
        // PN-INT-OVF-4.
        let r = eval("-9223372036854775808 / -1");
        assert!(r.is_err());
        assert!(r.unwrap_err().mensaje.contains("desbordamiento"));
    }

    #[test]
    fn pn_int_min_mod_menos_uno_es_cero() {
        // PN-INT-OVF-5: el resto cabe.
        assert_eq!(eval("-9223372036854775808 % -1"), Ok(Valor::Entero(0)));
    }

    #[test]
    fn pn_div_cero_entero_es_error_distinto() {
        // PN-INT-OVF-6: motivo "división entre cero", no desbordamiento.
        let r = eval("10 / 0");
        assert!(r.unwrap_err().mensaje.contains("cero"));
        assert!(eval("10 % 0").unwrap_err().mensaje.contains("cero"));
    }

    // ---- PN-MOD: convenio truncado ----

    #[test]
    fn pn_mod_signo_del_dividendo() {
        assert_eq!(eval("-7 % 3"), Ok(Valor::Entero(-1)));
        assert_eq!(eval("7 % -3"), Ok(Valor::Entero(1)));
        assert_eq!(eval("-7 % -3"), Ok(Valor::Entero(-1)));
        assert_eq!(eval("6 % 3"), Ok(Valor::Entero(0)));
    }

    #[test]
    fn pn_mod_identidad_division() {
        // a == (a/b)*b + a%b
        for (a, b) in [(7, 3), (-7, 3), (7, -3), (-7, -3)] {
            let suma = eval(&format!("({a} / {b}) * {b} + {a} % {b}")).unwrap();
            assert_eq!(suma, Valor::Entero(a), "identidad para {a},{b}");
        }
    }

    // ---- PN-DEC: decimal exacto ----

    #[test]
    fn pn_dec_cero_unos_mas_dos_es_tres() {
        // 0.1 + 0.2 == 0.3 exactamente (nunca 0.30000000000000004).
        assert_eq!(eval("0.1 + 0.2"), Ok(Valor::Decimal(300_000_000)));
    }

    #[test]
    fn pn_dec_tres_unos() {
        // 0.10 + 0.10 + 0.10 == 0.30 (escala preservada).
        let r = eval("0.10 + 0.10 + 0.10").unwrap();
        assert_eq!(r.to_string(), "0.3");
        assert_eq!(r, Valor::Decimal(300_000_000));
    }

    #[test]
    fn pn_dec_division_con_redondeo() {
        // 10.0 / 4.0 = 2.5; 1.0 / 3.0 = 0.333333333 (half-even, escala 9).
        assert_eq!(eval("10.0 / 4.0").unwrap().to_string(), "2.5");
        assert_eq!(eval("1.0 / 3.0").unwrap().to_string(), "0.333333333");
    }

    #[test]
    fn pn_dec_division_por_cero_es_error() {
        assert!(eval("1.0 / 0.0").unwrap_err().mensaje.contains("cero"));
    }

    #[test]
    fn pn_mix_entero_mas_decimal_es_decimal() {
        // Conversión implícita entero→decimal (sin pérdida).
        assert_eq!(eval("2 + 3.5"), Ok(Valor::Decimal(5_500_000_000)));
    }

    #[test]
    fn pn_mix_entero_mas_real_es_real() {
        // Conversión implícita entero→real.
        assert_eq!(eval("2 + 3.5e0"), Ok(Valor::Real(5.5)));
    }

    #[test]
    fn pn_mix_decimal_y_real_requiere_conversion_explicita() {
        // Combinar decimal y real en un operador es error de tipos (decisión 16).
        let r = eval("1.5 + 2.0e0");
        assert!(r.unwrap_err().mensaje.contains("conversión"));
    }

    // ---- PN-REAL ----

    #[test]
    fn pn_real_division_por_cero_es_error() {
        // D3 aprobado: no se genera inf/NaN desde el lenguaje.
        assert!(eval("1.0e0 / 0.0e0").unwrap_err().mensaje.contains("cero"));
        assert!(eval("1.0e0 / 0").unwrap_err().mensaje.contains("cero"));
    }

    #[test]
    fn pn_real_operacion_normal() {
        assert_eq!(eval("1.5e0 * 2.0e0"), Ok(Valor::Real(3.0)));
    }

    // ---- precedencia y contexto (regresión) ----

    #[test]
    fn precedencia_y_parentesis() {
        assert_eq!(eval("2 + 3 * 4"), Ok(Valor::Entero(14)));
        assert_eq!(eval("(2 + 3) * 4"), Ok(Valor::Entero(20)));
        assert_eq!(eval("10 - 3 - 2"), Ok(Valor::Entero(5)));
    }

    #[test]
    fn continuacion_tras_operador() {
        assert_eq!(eval("10 +\n    5"), Ok(Valor::Entero(15)));
    }

    #[test]
    fn modulo_misma_precedencia_que_multiplicacion() {
        assert_eq!(eval("7 + 6 % 4"), Ok(Valor::Entero(9)));
    }

    // ---- entorno de variables (ADR-0006) ----

    fn programa(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
        let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
        let ss = Parser::nuevo(tokens)
            .parsear_programa()
            .expect("sintaxis válida en test");
        Entorno::nueva().ejecutar_programa(&ss)
    }

    fn causa_programa(fuente: &str) -> CausaError {
        programa(fuente).expect_err("se esperaba error de ejecución").causa
    }

    #[test]
    fn declarar_usar_asignar_y_volver_a_usar() {
        let r = programa("sea x = 5\nx + 1").unwrap();
        assert_eq!(r, Some(Valor::Entero(6)));
        // El valor modificado se observa en la última expresión.
        let r = programa("sea x = 5\nx = x * 2\nx + 1").unwrap();
        assert_eq!(r, Some(Valor::Entero(11)));
    }

    #[test]
    fn declaracion_duplicada_es_error_tipeado() {
        assert_eq!(
            causa_programa("sea x = 1\nsea x = 2"),
            CausaError::DeclaracionDuplicada("x".into())
        );
    }

    #[test]
    fn referencia_a_variable_no_definida_es_error_tipeado() {
        assert_eq!(
            causa_programa("zeta + 1"),
            CausaError::VariableNoDefinida("zeta".into())
        );
    }

    #[test]
    fn asignacion_sin_declarar_no_crea_variable() {
        // Restricción del incremento: no hay declaración implícita.
        assert_eq!(
            causa_programa("x = 5"),
            CausaError::VariableNoDefinida("x".into())
        );
        // Y la variable sigue sin existir después del intento fallido.
        assert_eq!(
            causa_programa("x = 5\nx"),
            CausaError::VariableNoDefinida("x".into())
        );
    }

    #[test]
    fn asignacion_incompatible_es_error_tipeado() {
        // variable entero ← decimal: no hay conversión implícita en ese sentido.
        assert_eq!(
            causa_programa("sea x = 5\nx = 2.5"),
            CausaError::AsignacionIncompatible {
                esperado: "entero",
                obtenido: "decimal"
            }
        );
        // variable decimal ← real.
        assert_eq!(
            causa_programa("sea x = 1.5\nx = 2.0e0"),
            CausaError::AsignacionIncompatible {
                esperado: "decimal",
                obtenido: "real"
            }
        );
        // variable entero ← real.
        assert_eq!(
            causa_programa("sea x = 5\nx = 2.0e0"),
            CausaError::AsignacionIncompatible {
                esperado: "entero",
                obtenido: "real"
            }
        );
    }

    #[test]
    fn asignacion_admite_conversiones_implicitas_aprobadas() {
        // entero → decimal (ADR-0005 §1.4): la variable decimal acepta un entero.
        let r = programa("sea x = 1.5\nx = 2\nx").unwrap();
        assert_eq!(r, Some(Valor::Decimal(2_000_000_000)));
        // entero → real.
        let r = programa("sea x = 2.5e0\nx = 3\nx").unwrap();
        assert_eq!(r, Some(Valor::Real(3.0)));
    }

    #[test]
    fn errores_de_variable_llevan_ubicacion() {
        let e = programa("sea x = 1\nx + zeta").expect_err("zeta no existe");
        // 'zeta' ocupa la columna 5 en "x + zeta".
        assert_eq!((e.linea, e.columna), (2, 5));
        let e = programa("sea x = 1\nsea x = 2").expect_err("duplicada");
        assert_eq!((e.linea, e.columna), (2, 5));
        let e = programa("sea x = 1\nx = 2.5").expect_err("incompatible");
        assert_eq!((e.linea, e.columna), (2, 1));
    }

    #[test]
    fn errores_aritmeticos_siguen_funcionando_en_programas() {
        assert_eq!(
            causa_programa("sea x = 10\nx / 0"),
            CausaError::Aritmetica(MotivoAritmetico::DivisionPorCero)
        );
        assert_eq!(
            causa_programa("sea x = 9223372036854775807\nx + 1"),
            CausaError::Aritmetica(MotivoAritmetico::DesbordamientoEntero)
        );
        // La ubicación del error aritmético apunta a la variable izquierda.
        let e = programa("sea x = 10\nx / 0").expect_err("división por cero");
        assert_eq!((e.linea, e.columna), (2, 1));
    }

    #[test]
    fn programa_sin_sentencia_de_expresion_no_produce_valor() {
        assert_eq!(programa("sea x = 5\nx = x + 1").unwrap(), None);
    }

    #[test]
    fn tipo_de_variable_fija_en_declaracion() {
        // La declaración fija el tipo: un entero guardado en variable entero no admite decimales.
        assert_eq!(
            causa_programa("sea x = 5\nx = 7.5"),
            CausaError::AsignacionIncompatible {
                esperado: "entero",
                obtenido: "decimal"
            }
        );
        // Pero la referencia devuelve el valor con su tipo original.
        assert_eq!(programa("sea x = 7.5\nx").unwrap(), Some(Valor::Decimal(7_500_000_000)));
    }

    // ---- control de flujo y lógicos (ADR-0007) ----

    #[test]
    fn las_comparaciones_devuelven_logico() {
        assert_eq!(eval("1 == 1"), Ok(Valor::Logico(true)));
        assert_eq!(eval("1 == 2"), Ok(Valor::Logico(false)));
        assert_eq!(eval("1 != 2"), Ok(Valor::Logico(true)));
        assert_eq!(eval("1 < 2"), Ok(Valor::Logico(true)));
        assert_eq!(eval("2 <= 2"), Ok(Valor::Logico(true)));
        assert_eq!(eval("1 > 2"), Ok(Valor::Logico(false)));
        assert_eq!(eval("2 >= 2"), Ok(Valor::Logico(true)));
    }

    #[test]
    fn los_operadores_logicos_devuelven_logico() {
        assert_eq!(eval("verdadero y verdadero"), Ok(Valor::Logico(true)));
        assert_eq!(eval("verdadero y falso"), Ok(Valor::Logico(false)));
        assert_eq!(eval("falso o verdadero"), Ok(Valor::Logico(true)));
        assert_eq!(eval("falso o falso"), Ok(Valor::Logico(false)));
        assert_eq!(eval("no verdadero"), Ok(Valor::Logico(false)));
        assert_eq!(eval("no falso"), Ok(Valor::Logico(true)));
    }

    #[test]
    fn la_aritmetica_no_acepta_logicos() {
        assert_eq!(
            causa_programa("1 + verdadero"),
            CausaError::Aritmetica(MotivoAritmetico::TiposIncompatibles)
        );
        assert_eq!(
            causa_programa("verdadero + 1"),
            CausaError::Aritmetica(MotivoAritmetico::TiposIncompatibles)
        );
    }

    #[test]
    fn el_cortocircuito_evita_evaluar_el_operando_derecho() {
        // El lado derecho no se evalúa: no hay división por cero.
        assert_eq!(eval("falso y (10 / 0)"), Ok(Valor::Logico(false)));
        assert_eq!(eval("verdadero o (10 / 0)"), Ok(Valor::Logico(true)));
        // Pero si el derecho sí se evalúa, el error aparece.
        assert_eq!(
            causa_programa("verdadero y (10 / 0)"),
            CausaError::Aritmetica(MotivoAritmetico::DivisionPorCero)
        );
    }

    #[test]
    fn la_condicion_debe_ser_logica() {
        assert_eq!(
            causa_programa("si 1 {\n  2\n}"),
            CausaError::CondicionNoLogica { obtenido: "entero" }
        );
        assert_eq!(
            causa_programa("mientras 1 {\n  2\n}"),
            CausaError::CondicionNoLogica { obtenido: "entero" }
        );
    }

    #[test]
    fn el_bloque_no_produce_valor_hacia_fuera() {
        // El valor del programa es el de la última expresión de nivel superior.
        assert_eq!(programa("1 + 1\n{\n  5\n}").unwrap(), Some(Valor::Entero(2)));
    }

    #[test]
    fn el_bloque_usa_un_ambito_propio() {
        // La variable declarada en el bloque no sobrevive al bloque.
        assert_eq!(
            causa_programa("{\n  sea interna = 1\n}\ninterna"),
            CausaError::VariableNoDefinida("interna".into())
        );
        // El sombreado en niveles distintos está permitido.
        assert_eq!(
            programa("sea x = 1\n{\n  sea x = 2\n}\nx").unwrap(),
            Some(Valor::Entero(1))
        );
    }

    #[test]
    fn el_ciclo_se_reevalua_por_iteracion() {
        let r = programa("sea i = 0\nsea n = 0\nmientras i < 4 {\n  i = i + 1\n  n = n + 1\n}\nn").unwrap();
        assert_eq!(r, Some(Valor::Entero(4)));
    }

    #[test]
    fn el_ciclo_con_cero_iteraciones_no_entra_al_cuerpo() {
        let r = programa("sea n = 0\nmientras falso {\n  n = 99\n}\nn").unwrap();
        assert_eq!(r, Some(Valor::Entero(0)));
    }

    #[test]
    fn el_limite_de_iteraciones_protege_de_bucles_infinitos() {
        // Condición siempre verdadera: el guard de iteraciones corta el ciclo.
        assert_eq!(
            causa_programa("mientras verdadero {\n  1\n}"),
            CausaError::LimiteIteraciones {
                limite: LIMITE_ITERACIONES
            }
        );
    }

    #[test]
    fn los_errores_dentro_de_un_bloque_conservan_ubicacion() {
        let e = programa("si verdadero {\n  10 / 0\n}").expect_err("división por cero");
        assert_eq!(e.causa, CausaError::Aritmetica(MotivoAritmetico::DivisionPorCero));
        assert_eq!((e.linea, e.columna), (2, 3));
    }
}
