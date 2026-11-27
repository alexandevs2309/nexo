//! Modelo común de diagnósticos para CLI, LSP y REPL.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub archivo: Option<String>,
    pub linea: usize,
    pub columna: usize,
    pub linea_fin: usize,
    pub columna_fin: usize,
}

impl Span {
    pub fn punto(archivo: Option<String>, linea: usize, columna: usize) -> Self {
        Self {
            archivo,
            linea,
            columna,
            linea_fin: linea,
            columna_fin: columna.saturating_add(1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severidad {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostico {
    pub codigo: Option<String>,
    pub severidad: Severidad,
    pub mensaje: String,
    pub span: Option<Span>,
    pub fuente: String,
}

impl Diagnostico {
    pub fn error(mensaje: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            codigo: None,
            severidad: Severidad::Error,
            mensaje: mensaje.into(),
            span,
            fuente: "basilisco".to_string(),
        }
    }
    pub fn a_lsp(&self) -> serde_json::Value {
        let (linea, col, fin_linea, fin_col) = self
            .span
            .as_ref()
            .map(|s| {
                (
                    s.linea.saturating_sub(1),
                    s.columna.saturating_sub(1),
                    s.linea_fin.saturating_sub(1),
                    s.columna_fin.saturating_sub(1),
                )
            })
            .unwrap_or((0, 0, 0, 1));
        let sev = match self.severidad {
            Severidad::Error => 1,
            Severidad::Warning => 2,
            Severidad::Info => 3,
        };
        serde_json::json!({
            "range": {"start": {"line": linea, "character": col}, "end": {"line": fin_linea, "character": fin_col}},
            "severity": sev,
            "source": self.fuente,
            "message": self.mensaje,
            "code": self.codigo
        })
    }
}
