//! Biblioteca del prototipo de BASILISCO.
//!
//! Fase 2 (Puerta B): lexer → parser → evaluador.
//! Contratos de decisión: ADR-0003 (léxico), ADR-0004 (precedencia).
//! Identidad y CLI: ADR-0009 y ADR-0010.

pub mod cargador;
pub mod cli;
pub mod evaluador;
pub mod lexer;
pub mod parser;
pub mod proyecto;
pub mod token;
pub mod valor;

pub use evaluador::{CausaError, Entorno, ErrorEjecucion, evaluar};
pub use lexer::Lexer;
pub use parser::{ErrorSintactico, Expresion, Parser, Sentencia};
pub use token::{ErrorLexico, TipoToken, Token};
pub use valor::{ESCALA_DECIMAL, MotivoAritmetico, PRECISION_DECIMAL, Valor};
