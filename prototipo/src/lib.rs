//! Biblioteca del prototipo de BASILISCO.
//!
//! Fase 2 (Puerta B): lexer → parser → evaluador.
//! Contratos de decisión: ADR-0003 (léxico), ADR-0004 (precedencia).
//! Identidad y CLI: ADR-0009 y ADR-0010.

pub mod backend_entero;
pub mod backend_nativo;
pub mod backend_wasm;
pub mod cargador;
pub mod cli;
pub mod cliente_remoto;
pub mod dap;
pub mod diagnostico;
pub mod evaluador;
pub mod fechas;
pub mod ffi;
pub mod formateador;
pub mod hir;
pub mod http;
pub mod lexer;
pub mod lsp;
pub mod mir;
pub mod optimizador;
pub mod paquetes;
pub mod paridad;
pub mod parser;
pub mod proyecto;
pub mod runtime;
pub mod semantico;
pub mod servidor;
pub mod simbodo;
pub mod token;
pub mod valor;
pub mod vm;

pub use evaluador::{CausaError, Entorno, ErrorEjecucion, evaluar};
pub use lexer::Lexer;
pub use paridad::{
    Backend, Capacidad, ClasificacionMulti, Comparacion, EfectosObservables, EjecucionBackend,
    ResultadoNormalizado, SoporteBackend, ValorNormalizado, capacidades_backend,
    clasificar_multibackend, comparar_semantica, normalizar_error_tw, normalizar_error_vm,
    normalizar_valor, resultado_desde_salida_simple,
};
pub use parser::{ErrorSintactico, Expresion, Parser, Sentencia};
pub use semantico::ContextoSemantico;
pub use simbodo::{AmbitoId, CategoriaSimbodo, GestorAmbitos, Simbodo, SimbodoId, TablaSimbodo};
pub use token::{ErrorLexico, TipoToken, Token};
pub use valor::{ESCALA_DECIMAL, MotivoAritmetico, PRECISION_DECIMAL, Valor};

pub mod monomorfizacion;
