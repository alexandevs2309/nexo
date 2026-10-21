//! Biblioteca del prototipo de Nexo.
//!
//! Fase 2 (Puerta B): lexer → parser → evaluador.
//! Contratos de decisión: ADR-0003 (léxico), ADR-0004 (precedencia).

pub mod evaluador;
pub mod lexer;
pub mod parser;
pub mod token;
pub mod valor;

pub use evaluador::{evaluar, CausaError, Entorno, ErrorEjecucion};
pub use lexer::Lexer;
pub use parser::{ErrorSintactico, Expresion, Parser, Sentencia};
pub use token::{ErrorLexico, Token, TipoToken};
pub use valor::{Valor, MotivoAritmetico, ESCALA_DECIMAL, PRECISION_DECIMAL};
