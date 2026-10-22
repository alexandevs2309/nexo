/// Tipos de token del lexer de Nexo.
///
/// Contrato: `02-lenguaje/especificacion-lexica-minima.md` (aprobado junto con ADR-0003).
/// El lexer solo tokeniza; el rango numérico y la validez gramatical se resuelven más adelante.
#[derive(Debug, Clone, PartialEq)]
pub enum TipoToken {
    /// Literal numérico. El lexema conserva el texto crudo (`123`, `123.45`).
    Numero,
    /// Cadena de texto. El token lleva el contenido ya sin comillas ni escapes resueltos.
    Cadena(String),
    /// Identificador ASCII. El token lleva el nombre.
    Identificador(String),
    /// Palabra clave `sea` (declaración, ADR-0006).
    Sea,
    /// Palabra clave `var` (declaración mutable tipada).
    Var,
    /// Palabra clave `dato` (declaración inmutable tipada).
    Dato,
    /// Palabra clave `const` (constante de compilación).
    Const,
    Funcion,
    Retornar,
    Mostrar,
    Si,
    Sino,
    Mientras,
    Y,
    O,
    No,
    Verdadero,
    Falso,
    Mas,
    Menos,
    Por,
    Modulo,
    Dividido,
    Asignacion,
    IgualIgual,
    Distinto,
    Menor,
    MenorIgual,
    Mayor,
    MayorIgual,
    ParentesisIzq,
    ParentesisDer,
    LlaveIzq,
    LlaveDer,
    Coma,
    /// Dos puntos `:` para anotaciones de tipo.
    DosPuntos,
    /// Fin de línea (LF o CRLF). El parser decide contextualmente si termina sentencia.
    FinDeLinea,
    Fin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tipo: TipoToken,
    /// Texto crudo tal como aparece en la fuente.
    pub lexema: String,
    pub linea: usize,
    pub columna: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorLexico {
    pub mensaje: String,
    pub linea: usize,
    pub columna: usize,
}

impl std::fmt::Display for ErrorLexico {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error léxico [{}:{}]: {}",
            self.linea, self.columna, self.mensaje
        )
    }
}
