//! Biblioteca del prototipo de BASILISCO.
//!
//! Fase 2 (Puerta B): lexer → parser → evaluador.
//! Contratos de decisión: ADR-0003 (léxico), ADR-0004 (precedencia).
//! Identidad y CLI: ADR-0009 y ADR-0010.

pub mod cargador;
pub mod cli;
pub mod cliente_remoto;
pub mod evaluador;
pub mod formateador;
pub mod lexer;
pub mod paquetes;
pub mod parser;
pub mod proyecto;
pub mod semantico;
pub mod servidor;
pub mod token;
pub mod valor;

pub use evaluador::{CausaError, Entorno, ErrorEjecucion, evaluar};
pub use lexer::Lexer;
pub use parser::{ErrorSintactico, Expresion, Parser, Sentencia};
pub use semantico::ContextoSemantico;
pub use token::{ErrorLexico, TipoToken, Token};
pub use valor::{ESCALA_DECIMAL, MotivoAritmetico, PRECISION_DECIMAL, Valor};
