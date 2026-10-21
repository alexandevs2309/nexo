//! AST y parser del prototipo vertical (Puerta B).
//!
//! Gramática en descenso recursivo según ADR-0004:
//!
//! ```text
//! expresión      → suma
//! suma           → multiplicación ( ("+" | "-") multiplicación )*
//! multiplicación → primaria ( ("*" | "/") primaria )*
//! primaria       → NUMERO | "(" expresión ")"
//! ```
//!
//! Regla contextual de fin de línea (ADR-0003 §3):
//! - Al buscar una *primaria*, los finales de línea se omiten (continuación tras operador o `(`).
//! - Dentro de un grupo `( … )` los finales de línea se omiten también en los operadores.
//! - Fuera de ese contexto, un fin de línea termina la expresión.

use crate::token::{Token, TipoToken};
use crate::valor::{Valor, MotivoAritmetico};

#[derive(Debug, Clone, PartialEq)]
pub enum Expresion {
    /// Literal con su ubicación original (línea/columna) para diagnósticos (ADR-0005 §1.7).
    Literal {
        valor: Valor,
        linea: usize,
        columna: usize,
    },
    /// Referencia a una variable con su ubicación (ADR-0006).
    Variable {
        nombre: String,
        linea: usize,
        columna: usize,
    },
    Suma(Box<Expresion>, Box<Expresion>),
    Resta(Box<Expresion>, Box<Expresion>),
    Multiplicacion(Box<Expresion>, Box<Expresion>),
    Division(Box<Expresion>, Box<Expresion>),
    Modulo(Box<Expresion>, Box<Expresion>),
    IgualIgual(Box<Expresion>, Box<Expresion>),
    Distinto(Box<Expresion>, Box<Expresion>),
    Menor(Box<Expresion>, Box<Expresion>),
    MenorIgual(Box<Expresion>, Box<Expresion>),
    Mayor(Box<Expresion>, Box<Expresion>),
    MayorIgual(Box<Expresion>, Box<Expresion>),
    No(Box<Expresion>),
    Y(Box<Expresion>, Box<Expresion>),
    O(Box<Expresion>, Box<Expresion>),
}

/// Sentencias de un programa (ADR-0006, ampliadas ADR-0007).
#[derive(Debug, Clone, PartialEq)]
pub enum Sentencia {
    /// `sea nombre = expresión`.
    Declaracion {
        nombre: String,
        linea: usize,
        columna: usize,
        valor: Expresion,
    },
    /// `nombre = expresión` sobre una variable existente.
    Asignacion {
        nombre: String,
        linea: usize,
        columna: usize,
        valor: Expresion,
    },
    /// `si condición { ... } (sino { ... } | sino si ... )?`
    Si {
        condicion: Expresion,
        rama_si: Vec<Sentencia>,
        rama_sino: Option<Vec<Sentencia>>,
        linea: usize,
        columna: usize,
    },
    /// `mientras condición { ... }`
    Mientras {
        condicion: Expresion,
        cuerpo: Vec<Sentencia>,
        linea: usize,
        columna: usize,
    },
    /// Bloque autónomo `{ ... }`
    Bloque(Vec<Sentencia>),
    /// Sentencia de expresión: evalúa a un valor.
    Expresion(Expresion),
}

/// Operador de comparación (ADR-0007 §1.3, nivel 4).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Comparador {
    Igual,
    Distinto,
    Menor,
    MenorIgual,
    Mayor,
    MayorIgual,
}

impl Comparador {
    fn construir(self, izq: Box<Expresion>, der: Box<Expresion>) -> Expresion {
        match self {
            Comparador::Igual => Expresion::IgualIgual(izq, der),
            Comparador::Distinto => Expresion::Distinto(izq, der),
            Comparador::Menor => Expresion::Menor(izq, der),
            Comparador::MenorIgual => Expresion::MenorIgual(izq, der),
            Comparador::Mayor => Expresion::Mayor(izq, der),
            Comparador::MayorIgual => Expresion::MayorIgual(izq, der),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorSintactico {
    pub mensaje: String,
    pub linea: usize,
    pub columna: usize,
}

/// Clasifica un literal numérico según ADR-0005 §1.5 (decisión 15):
/// `42` → entero; `42.5` → decimal; `1e10` / `2.5e-3` → real.
pub(crate) fn clasificar_literal(lex: &str) -> Result<Valor, MotivoAritmetico> {
    if lex.contains(['.', 'e', 'E']) {
        if lex.contains('e') || lex.contains('E') {
            Valor::real_desde_lexema(lex)
        } else {
            Valor::decimal_desde_lexema(lex)
        }
    } else {
        Valor::entero_desde_lexema(lex)
    }
}

impl std::fmt::Display for ErrorSintactico {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error sintáctico [{}:{}]: {}",
            self.linea, self.columna, self.mensaje
        )
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Profundidad de agrupación abierta; > 0 significa dentro de `( … )`.
    en_grupo: usize,
}

impl Parser {
    pub fn nuevo(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            en_grupo: 0,
        }
    }

    /// Parsea una única expresión. Después solo se admiten finales de línea y el fin de archivo.
    pub fn parsear_expresion(&mut self) -> Result<Expresion, ErrorSintactico> {
        let e = self.expresion()?;
        self.omitir_finales_de_linea();
        if !matches!(self.actual(), Some(TipoToken::Fin) | None) {
            return Err(self.error("texto inesperado después de la expresión"));
        }
        Ok(e)
    }

    /// Parsea un programa: una secuencia de sentencias separadas por fin de línea
    /// hasta el fin de archivo (ADR-0006).
    pub fn parsear_programa(&mut self) -> Result<Vec<Sentencia>, ErrorSintactico> {
        let mut sentencias = Vec::new();
        loop {
            self.omitir_finales_de_linea();
            match self.actual() {
                Some(TipoToken::Fin) | None => break,
                _ => sentencias.push(self.sentencia()?),
            }
        }
        Ok(sentencias)
    }

    /// Parsea una sentencia: declaración, asignación, control de flujo, bloque o expresión.
    fn sentencia(&mut self) -> Result<Sentencia, ErrorSintactico> {
        if matches!(self.actual(), Some(TipoToken::Sea)) {
            self.pos += 1; // 'sea'
            let (nombre, linea, columna) = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let t = &self.tokens[self.pos];
                    (n.clone(), t.linea, t.columna)
                }
                _ => return Err(self.error("se esperaba un nombre tras 'sea'")),
            };
            self.pos += 1; // identificador
            if !matches!(self.actual(), Some(TipoToken::Asignacion)) {
                return Err(self.error(format!(
                    "se esperaba '=' tras el nombre {nombre:?} en la declaración"
                )));
            }
            self.pos += 1; // '='
            let valor = self.expresion()?;
            self.terminar_sentencia()?;
            Ok(Sentencia::Declaracion {
                nombre,
                linea,
                columna,
                valor,
            })
        } else if matches!(self.actual(), Some(TipoToken::Si)) {
            let s = self.sentencia_si()?;
            self.terminar_sentencia()?;
            Ok(s)
        } else if matches!(self.actual(), Some(TipoToken::Mientras)) {
            let s = self.sentencia_mientras()?;
            self.terminar_sentencia()?;
            Ok(s)
        } else if matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            let bloque = self.bloque()?;
            self.terminar_sentencia()?;
            Ok(Sentencia::Bloque(bloque))
        } else if matches!(self.actual(), Some(TipoToken::Sino)) {
            Err(self.error("'sino' sin un 'si' correspondiente"))
        } else if let Some(TipoToken::Identificador(n)) = self.actual().cloned() {
            // Asignación solo si el siguiente token es '='; si no, es una expresión.
            if matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.tipo),
                Some(TipoToken::Asignacion)
            ) {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                self.pos += 1; // identificador
                self.pos += 1; // '='
                let valor = self.expresion()?;
                self.terminar_sentencia()?;
                Ok(Sentencia::Asignacion {
                    nombre: n,
                    linea,
                    columna,
                    valor,
                })
            } else {
                let e = self.expresion()?;
                self.terminar_sentencia()?;
                Ok(Sentencia::Expresion(e))
            }
        } else {
            let e = self.expresion()?;
            self.terminar_sentencia()?;
            Ok(Sentencia::Expresion(e))
        }
    }

    fn bloque(&mut self) -> Result<Vec<Sentencia>, ErrorSintactico> {
        // '{' debe estar en la misma línea que el encabezado (regla ADR-0007).
        self.pos += 1; // '{'
        self.omitir_finales_de_linea();
        let mut sentencias = Vec::new();
        loop {
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                self.pos += 1; // '}'
                break;
            }
            if matches!(self.actual(), Some(TipoToken::Fin) | None) {
                return Err(self.error("falta '}' para cerrar el bloque"));
            }
            sentencias.push(self.sentencia()?);
        }
        Ok(sentencias)
    }

    /// Parsea una sentencia `si` completa, incluidas las ramas `sino`/`sino si`.
    /// NO consume el terminador de sentencia: de eso se encarga el llamador, para
    /// que `sino si` (que anida otra sentencia `si`) no termine la sentencia dos veces.
    fn sentencia_si(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'si'
        let condicion = self.expresion()?;
        self.exigir_llave_misma_linea(linea, "si")?;
        let rama_si = self.bloque()?;
        // `sino` puede ir en la línea siguiente; se busca sin consumir los saltos de línea
        // si no aparece.
        let guarda = self.pos;
        self.omitir_finales_de_linea();
        let hay_sino = matches!(self.actual(), Some(TipoToken::Sino));
        if !hay_sino {
            self.pos = guarda;
        }
        let rama_sino = if hay_sino {
            let t = &self.tokens[self.pos];
            let linea_sino = t.linea;
            self.pos += 1; // 'sino'
            // Tras `sino` solo se admite `si` o `{`, en la misma línea (ADR-0007 §1.6).
            match self.actual() {
                Some(TipoToken::Si) => {
                    // `sino si`: la sentencia `si` interna se parsea sin consumir el terminador.
                    let s = self.sentencia_si()?;
                    Some(vec![s])
                }
                Some(TipoToken::LlaveIzq) => {
                    self.exigir_llave_misma_linea(linea_sino, "sino")?;
                    Some(self.bloque()?)
                }
                _ => {
                    return Err(self.error(
                        "se esperaba '{' o 'si' en la misma línea que 'sino'",
                    ))
                }
            }
        } else {
            None
        };
        Ok(Sentencia::Si {
            condicion,
            rama_si,
            rama_sino,
            linea,
            columna,
        })
    }

    fn sentencia_mientras(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'mientras'
        let condicion = self.expresion()?;
        self.exigir_llave_misma_linea(linea, "mientras")?;
        let cuerpo = self.bloque()?;
        Ok(Sentencia::Mientras {
            condicion,
            cuerpo,
            linea,
            columna,
        })
    }

    /// Exige el fin de la sentencia: fin de línea (consumido), fin de archivo o
    /// cierre de bloque (`}`), que no se consume aquí.
    fn terminar_sentencia(&mut self) -> Result<(), ErrorSintactico> {
        match self.actual() {
            Some(TipoToken::FinDeLinea) => {
                self.pos += 1;
                Ok(())
            }
            Some(TipoToken::Fin) | None => Ok(()),
            Some(TipoToken::LlaveDer) => Ok(()),
            _ => Err(self.error("texto inesperado después de la sentencia")),
        }
    }

    /// Exige que el token actual sea `{` y que esté en la misma línea que la palabra
    /// clave que lo introduce (regla ADR-0007 §1.6, con diagnóstico inmediato).
    fn exigir_llave_misma_linea(
        &mut self,
        linea_palabra: usize,
        palabra: &str,
    ) -> Result<(), ErrorSintactico> {
        if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            return Err(self.error(format!("se esperaba '{{' en la misma línea que '{palabra}'")));
        }
        match self.linea_actual() {
            Some(l) if l == linea_palabra => Ok(()),
            _ => Err(self.error(format!(
                "se esperaba '{{' en la misma línea que '{palabra}'"
            ))),
        }
    }

    fn actual(&self) -> Option<&TipoToken> {
        self.tokens.get(self.pos).map(|t| &t.tipo)
    }

    /// Línea del token actual (para comprobar la regla de "misma línea" del ADR-0007).
    fn linea_actual(&self) -> Option<usize> {
        self.tokens.get(self.pos).map(|t| t.linea)
    }

    fn error(&self, mensaje: impl Into<String>) -> ErrorSintactico {
        match self.tokens.get(self.pos) {
            Some(t) => ErrorSintactico {
                mensaje: mensaje.into(),
                linea: t.linea,
                columna: t.columna,
            },
            None => ErrorSintactico {
                mensaje: mensaje.into(),
                linea: self.tokens.last().map_or(1, |t| t.linea),
                columna: self.tokens.last().map_or(1, |t| t.columna + 1),
            },
        }
    }

    fn omitir_finales_de_linea(&mut self) {
        while matches!(self.actual(), Some(TipoToken::FinDeLinea)) {
            self.pos += 1;
        }
    }

    fn expresion(&mut self) -> Result<Expresion, ErrorSintactico> {
        self.o()
    }

    fn o(&mut self) -> Result<Expresion, ErrorSintactico> {
        let mut izq = self.y()?;
        loop {
            if self.en_grupo > 0 {
                self.omitir_finales_de_linea();
            }
            if matches!(self.actual(), Some(TipoToken::O)) {
                self.pos += 1;
                let der = self.y()?;
                izq = Expresion::O(Box::new(izq), Box::new(der));
                continue;
            }
            break;
        }
        Ok(izq)
    }

    fn y(&mut self) -> Result<Expresion, ErrorSintactico> {
        let mut izq = self.no_unario()?;
        loop {
            if self.en_grupo > 0 {
                self.omitir_finales_de_linea();
            }
            if matches!(self.actual(), Some(TipoToken::Y)) {
                self.pos += 1;
                let der = self.no_unario()?;
                izq = Expresion::Y(Box::new(izq), Box::new(der));
                continue;
            }
            break;
        }
        Ok(izq)
    }

    fn no_unario(&mut self) -> Result<Expresion, ErrorSintactico> {
        self.omitir_finales_de_linea();
        if matches!(self.actual(), Some(TipoToken::No)) {
            self.pos += 1;
            let e = self.no_unario()?;
            return Ok(Expresion::No(Box::new(e)));
        }
        self.comparacion()
    }

    fn comparacion(&mut self) -> Result<Expresion, ErrorSintactico> {
        let mut izq = self.suma()?;
        loop {
            if self.en_grupo > 0 {
                self.omitir_finales_de_linea();
            }
            let comparador = match self.actual() {
                Some(TipoToken::IgualIgual) => Some(Comparador::Igual),
                Some(TipoToken::Distinto) => Some(Comparador::Distinto),
                Some(TipoToken::Menor) => Some(Comparador::Menor),
                Some(TipoToken::MenorIgual) => Some(Comparador::MenorIgual),
                Some(TipoToken::Mayor) => Some(Comparador::Mayor),
                Some(TipoToken::MayorIgual) => Some(Comparador::MayorIgual),
                _ => None,
            };
            if let Some(c) = comparador {
                self.pos += 1;
                let der = self.suma()?;
                izq = c.construir(Box::new(izq), Box::new(der));
                // Encadenamiento prohibido (ADR-0007): otro comparador al mismo nivel es error
                // sintáctico explícito, para no mixing tipos (lógico vs numérico).
                if self.en_grupo > 0 {
                    self.omitir_finales_de_linea();
                }
                if matches!(
                    self.actual(),
                    Some(TipoToken::IgualIgual)
                        | Some(TipoToken::Distinto)
                        | Some(TipoToken::Menor)
                        | Some(TipoToken::MenorIgual)
                        | Some(TipoToken::Mayor)
                        | Some(TipoToken::MayorIgual)
                ) {
                    return Err(self.error("comparaciones encadenadas no están permitidas"));
                }
                continue;
            }
            break;
        }
        Ok(izq)
    }

    fn suma(&mut self) -> Result<Expresion, ErrorSintactico> {
        let mut izq = self.multiplicacion()?;
        loop {
            if self.en_grupo > 0 {
                self.omitir_finales_de_linea();
            }
            let op = match self.actual() {
                Some(TipoToken::Mas) => Expresion::Suma,
                Some(TipoToken::Menos) => Expresion::Resta,
                _ => break,
            };
            self.pos += 1;
            let der = self.multiplicacion()?;
            izq = op(Box::new(izq), Box::new(der));
        }
        Ok(izq)
    }

    fn multiplicacion(&mut self) -> Result<Expresion, ErrorSintactico> {
        let mut izq = self.primaria()?;
        loop {
            if self.en_grupo > 0 {
                self.omitir_finales_de_linea();
            }
            let op = match self.actual() {
                Some(TipoToken::Por) => Expresion::Multiplicacion,
                Some(TipoToken::Dividido) => Expresion::Division,
                Some(TipoToken::Modulo) => Expresion::Modulo,
                _ => break,
            };
            self.pos += 1;
            let der = self.primaria()?;
            izq = op(Box::new(izq), Box::new(der));
        }
        Ok(izq)
    }

    fn primaria(&mut self) -> Result<Expresion, ErrorSintactico> {
        // Continuación contextual: tras un operador o '(' el salto de línea no termina.
        self.omitir_finales_de_linea();
        match self.actual() {
            // Literal negativo en posición primaria (no es operador unario general:
            // `-x` con x expresión sigue prohibido por ADR-0004). Necesario para la
            // semántica aprobada de D4 (`-7 % 3` = -1).
            Some(TipoToken::Menos)
                if matches!(self.tokens.get(self.pos + 1).map(|t| &t.tipo), Some(TipoToken::Numero)) =>
            {
                let (linea, columna) = {
                    let t = &self.tokens[self.pos];
                    (t.linea, t.columna)
                };
                self.pos += 1; // '-'
                let t = &self.tokens[self.pos];
                let lex = format!("-{}", t.lexema);
                let v = clasificar_literal(&lex).map_err(|m| ErrorSintactico {
                    mensaje: format!("literal numérico inválido {lex:?}: {m}"),
                    linea: t.linea,
                    columna: t.columna,
                })?;
                self.pos += 1;
                Ok(Expresion::Literal {
                    valor: v,
                    linea,
                    columna,
                })
            }
            Some(TipoToken::Numero) => {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                let v = clasificar_literal(&t.lexema).map_err(|m| ErrorSintactico {
                    mensaje: format!("literal numérico inválido {:?}: {m}", t.lexema),
                    linea: t.linea,
                    columna: t.columna,
                })?;
                self.pos += 1;
                Ok(Expresion::Literal {
                    valor: v,
                    linea,
                    columna,
                })
            }
            Some(TipoToken::Verdadero) => {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                self.pos += 1;
                Ok(Expresion::Literal {
                    valor: Valor::Logico(true),
                    linea,
                    columna,
                })
            }
            Some(TipoToken::Falso) => {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                self.pos += 1;
                Ok(Expresion::Literal {
                    valor: Valor::Logico(false),
                    linea,
                    columna,
                })
            }
            Some(TipoToken::Identificador(nombre)) => {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                let nombre = nombre.clone();
                self.pos += 1;
                Ok(Expresion::Variable {
                    nombre,
                    linea,
                    columna,
                })
            }
            Some(TipoToken::ParentesisIzq) => {
                self.pos += 1;
                self.en_grupo += 1;
                let e = self.expresion();
                self.en_grupo -= 1;
                let e = e?;
                self.omitir_finales_de_linea();
                match self.actual() {
                    Some(TipoToken::ParentesisDer) => {
                        self.pos += 1;
                        Ok(e)
                    }
                    _ => Err(self.error("falta ')' de cierre")),
                }
            }
            Some(TipoToken::Fin) | None => {
                Err(self.error("expresión incompleta: falta un operando"))
            }
            _ => Err(self.error("se esperaba un número o '('")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluador::evaluar;
    use crate::Lexer;

    /// Tokeniza, parsea y evalúa; devuelve el valor o el error sintáctico.
    fn valor(fuente: &str) -> Result<Valor, ErrorSintactico> {
        let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido en test");
        Parser::nuevo(tokens).parsear_expresion().map(|e| {
            evaluar(&e).expect("evaluación sin error en tests sin división entre cero")
        })
    }

    fn error_sintactico(fuente: &str) -> ErrorSintactico {
        let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido en test");
        Parser::nuevo(tokens)
            .parsear_expresion()
            .expect_err("se esperaba error sintáctico")
    }

    #[test]
    fn suma_sencilla_de_la_puerta_b() {
        assert_eq!(valor("10 + 5"), Ok(Valor::Entero(15)));
    }

    #[test]
    fn precedencia_multiplicacion_sobre_suma() {
        assert_eq!(valor("2 + 3 * 4"), Ok(Valor::Entero(14)));
        assert_eq!(valor("2 * 3 + 4"), Ok(Valor::Entero(10)));
    }

    #[test]
    fn parentesis_anulan_precedencia() {
        assert_eq!(valor("(2 + 3) * 4"), Ok(Valor::Entero(20)));
        assert_eq!(valor("7"), Ok(Valor::Entero(7)));
        assert_eq!(valor("(7)"), Ok(Valor::Entero(7)));
    }

    #[test]
    fn asociatividad_izquierda() {
        assert_eq!(valor("10 - 3 - 2"), Ok(Valor::Entero(5)));
        assert_eq!(valor("100 / 5 / 2"), Ok(Valor::Entero(10)));
    }

    #[test]
    fn decimal_y_resta() {
        assert_eq!(valor("1.5 + 2.25"), Ok(Valor::Decimal(3_750_000_000)));
        assert_eq!(valor("9 - 4.5"), Ok(Valor::Decimal(4_500_000_000)));
    }

    #[test]
    fn literal_real_con_exponencial() {
        assert_eq!(valor("1e10"), Ok(Valor::Real(1e10)));
        assert_eq!(valor("2.5e-3"), Ok(Valor::Real(2.5e-3)));
    }

    #[test]
    fn clasificacion_de_literales() {
        assert_eq!(clasificar_literal("42"), Ok(Valor::Entero(42)));
        assert_eq!(
            clasificar_literal("42.5"),
            Ok(Valor::Decimal(42_500_000_000))
        );
        assert_eq!(clasificar_literal("1e10"), Ok(Valor::Real(1e10)));
    }

    #[test]
    fn continuacion_tras_operador() {
        // Regla contextual: el '+' al final pide operando, el salto no termina.
        assert_eq!(valor("10 +\n    5"), Ok(Valor::Entero(15)));
        assert_eq!(valor("10 +\n5"), Ok(Valor::Entero(15)));
    }

    #[test]
    fn salto_de_linea_tras_expresion_completa_termina() {
        // '10' está completo: el salto termina y '+ 5' sobra → error sintáctico.
        assert!(valor("10\n+ 5").is_err());
    }

    #[test]
    fn saltos_de_linea_dentro_de_parentesis_no_interrumpen() {
        assert_eq!(valor("(\n10\n+\n5\n)"), Ok(Valor::Entero(15)));
        assert_eq!(valor("(\n2 + 3\n*\n4\n)"), Ok(Valor::Entero(14)));
    }

    #[test]
    fn errores_sintacticos() {
        assert!(valor("2 +").is_err()); // operando faltante
        assert!(valor("+ 5").is_err()); // no hay unario
        assert!(valor("(2 + 3").is_err()); // falta ')'
        assert!(valor("10 + 5 6").is_err()); // texto inesperado
    }

    #[test]
    fn posicion_del_error() {
        let e = error_sintactico("10\n+ 5");
        assert_eq!(e.linea, 2);
        assert!(e.mensaje.contains("inesperado") || e.mensaje.contains("esperaba"));
    }

    // ---- sentencias (ADR-0006) ----

    fn programa(fuente: &str) -> Vec<Sentencia> {
        let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido en test");
        Parser::nuevo(tokens)
            .parsear_programa()
            .expect("sintaxis de programa válida en test")
    }

    #[test]
    fn programa_declaracion_asignacion_expresion() {
        let ss = programa("sea x = 5\nx = x + 1\nx * 2");
        assert_eq!(ss.len(), 3);
        match &ss[0] {
            Sentencia::Declaracion { nombre, .. } => assert_eq!(nombre, "x"),
            otro => panic!("esperaba declaración, obtuvo {otro:?}"),
        }
        match &ss[1] {
            Sentencia::Asignacion { nombre, .. } => assert_eq!(nombre, "x"),
            otro => panic!("esperaba asignación, obtuvo {otro:?}"),
        }
        assert!(matches!(&ss[2], Sentencia::Expresion(_)));
    }

    #[test]
    fn identificador_sin_igual_es_expresion() {
        let ss = programa("x + 1");
        assert_eq!(ss.len(), 1);
        assert!(matches!(&ss[0], Sentencia::Expresion(Expresion::Suma(..))));
    }

    #[test]
    fn declaracion_requiere_nombre_y_igual() {
        let tokens = Lexer::nuevo("sea = 5").tokenizar().unwrap();
        let e = Parser::nuevo(tokens).parsear_programa().unwrap_err();
        assert!(e.mensaje.contains("nombre"));
        let tokens = Lexer::nuevo("sea x 5").tokenizar().unwrap();
        let e = Parser::nuevo(tokens).parsear_programa().unwrap_err();
        assert!(e.mensaje.contains("'='"));
    }

    #[test]
    fn parsear_expresion_no_acepta_sentencias() {
        // Compatibilidad: la entrada de expresión única sigue sin admitir 'sea' ni '='.
        let tokens = Lexer::nuevo("x = 5").tokenizar().unwrap();
        assert!(Parser::nuevo(tokens).parsear_expresion().is_err());
        let tokens = Lexer::nuevo("sea x = 5").tokenizar().unwrap();
        assert!(Parser::nuevo(tokens).parsear_expresion().is_err());
    }

    // ---- control de flujo y lógicos (ADR-0007) ----

    fn error_programa(fuente: &str) -> ErrorSintactico {
        let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido en test");
        Parser::nuevo(tokens)
            .parsear_programa()
            .expect_err("se esperaba error sintáctico")
    }

    #[test]
    fn el_ast_de_si_tiene_condicion_y_ramas() {
        let ss = programa("si x > 1 {\n  1\n} sino {\n  2\n}");
        assert_eq!(ss.len(), 1);
        match &ss[0] {
            Sentencia::Si {
                condicion,
                rama_si,
                rama_sino,
                linea,
                columna,
            } => {
                assert!(matches!(condicion, Expresion::Mayor(..)));
                assert_eq!(rama_si.len(), 1);
                assert_eq!(rama_sino.as_ref().map(|r| r.len()), Some(1));
                assert_eq!((*linea, *columna), (1, 1));
            }
            otro => panic!("esperaba `si`, obtuvo {otro:?}"),
        }
    }

    #[test]
    fn el_ast_de_si_sin_sino_no_tiene_rama_alternativa() {
        let ss = programa("si x > 1 {\n  1\n}");
        match &ss[0] {
            Sentencia::Si { rama_sino, .. } => assert!(rama_sino.is_none()),
            otro => panic!("esperaba `si`, obtuvo {otro:?}"),
        }
    }

    #[test]
    fn el_ast_de_mientras_tiene_condicion_y_cuerpo() {
        let ss = programa("mientras x < 3 {\n  x = x + 1\n}");
        assert_eq!(ss.len(), 1);
        match &ss[0] {
            Sentencia::Mientras {
                condicion, cuerpo, ..
            } => {
                assert!(matches!(condicion, Expresion::Menor(..)));
                assert_eq!(cuerpo.len(), 1);
            }
            otro => panic!("esperaba `mientras`, obtuvo {otro:?}"),
        }
    }

    #[test]
    fn el_ast_de_bloque_lista_las_sentencias_internas() {
        let ss = programa("{\n  sea x = 1\n  x\n}");
        assert_eq!(ss.len(), 1);
        match &ss[0] {
            Sentencia::Bloque(internas) => assert_eq!(internas.len(), 2),
            otro => panic!("esperaba bloque, obtuvo {otro:?}"),
        }
    }

    #[test]
    fn la_llave_debe_estar_en_la_misma_linea_que_la_palabra_clave() {
        for (fuente, esperado) in [
            ("si x\n{\n  1\n}", "misma línea que 'si'"),
            ("mientras x\n{\n  1\n}", "misma línea que 'mientras'"),
            ("si x {\n  1\n} sino\n{\n  2\n}", "misma línea que 'sino'"),
        ] {
            let e = error_programa(fuente);
            assert!(
                e.mensaje.contains(esperado),
                "mensaje inesperado para {fuente:?}: {}",
                e.mensaje
            );
        }
    }

    #[test]
    fn sino_sin_si_es_error_sintactico() {
        let e = error_programa("sino {\n  1\n}");
        assert!(e.mensaje.contains("sino"), "mensaje: {}", e.mensaje);
    }

    #[test]
    fn falta_llave_de_cierre_es_error_sintactico() {
        let e = error_programa("si x {\n  1\n");
        assert!(e.mensaje.contains("falta '}'"), "mensaje: {}", e.mensaje);
    }

    #[test]
    fn las_comparaciones_encadenadas_son_error_sintactico() {
        let e = error_sintactico("1 < 2 < 3");
        assert!(
            e.mensaje.contains("encadenadas"),
            "mensaje: {}",
            e.mensaje
        );
    }

    #[test]
    fn la_estructura_del_ast_refleja_la_precedencia() {
        // `1 + 2 * 3` → Suma(1, Multiplicacion(2, 3))
        let tokens = Lexer::nuevo("1 + 2 * 3").tokenizar().unwrap();
        let e = Parser::nuevo(tokens).parsear_expresion().unwrap();
        assert!(matches!(e, Expresion::Suma(_, b) if matches!(*b, Expresion::Multiplicacion(..))));

        // `no a y b` → Y(No(a), b) (ADR-0007 §1.3)
        let tokens = Lexer::nuevo("no a y b").tokenizar().unwrap();
        let e = Parser::nuevo(tokens).parsear_expresion().unwrap();
        match e {
            Expresion::Y(a, b) => {
                assert!(matches!(*a, Expresion::No(..)));
                assert!(matches!(*b, Expresion::Variable { .. }));
            }
            otro => panic!("esperaba `y`, obtuvo {otro:?}"),
        }

        // `a o b` → O(a, b)
        let tokens = Lexer::nuevo("a o b").tokenizar().unwrap();
        let e = Parser::nuevo(tokens).parsear_expresion().unwrap();
        assert!(matches!(e, Expresion::O(..)));
    }

    #[test]
    fn las_palabras_clave_nuevas_no_son_identificadores() {
        // `y` y `o` son palabras clave: no pueden ser nombres de variable (ADR-0007 §4).
        for fuente in ["sea y = 1", "sea o = 1", "y = 1", "o = 1"] {
            let e = error_programa(fuente);
            assert!(
                !e.mensaje.is_empty(),
                "{fuente:?} debería ser error de sintaxis"
            );
        }
    }
}
