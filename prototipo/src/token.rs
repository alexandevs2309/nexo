/// Tipos de token del lexer de BASILISCO.
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
    Arreglo,
    Lista,
    Funcion,
    Retornar,
    Mostrar,
    Si,
    Sino,
    Mientras,
    Coincidir,
    Implementar,
    Rasgo,
    Yo,
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
    /// Corchete izquierdo `[`
    CorcheteIzq,
    /// Corchete derecho `]`
    CorcheteDer,
    /// Punto `.` para acceso a miembro (ADR-0017 D8).
    Punto,
    /// Dos puntos `:` para anotaciones de tipo.
    DosPuntos,
    /// Flecha `->` para anotación de tipo de retorno (ADR-0011).
    Flecha,
    /// Flecha doble `=>` para brazos de `coincidir` (Fase 08 S8.3).
    FlechaDoble,
    /// Palabra clave `nulo` (ADR-0011 §3.5).
    Nulo,
    /// Palabra clave `para` (ADR-0012 §3.1).
    Para,
    /// Palabra clave `cada` (ADR-0012 §3.1).
    Cada,
    /// Palabra clave `en` (ADR-0012 §3.1).
    En,
    /// Palabra clave `romper` (ADR-0012 §3.2).
    Romper,
    /// Palabra clave `continuar` (ADR-0012 §3.2).
    Continuar,
    /// Operador de rango exclusivo `..` (ADR-0012 §3.3).
    RangoDos,
    /// Operador de rango inclusivo `..=` (ADR-0012 §3.3).
    RangoDosCerrado,
    /// Sentencia `mostrar_sin_salto` (ADR-0013 §3.3).
    MostrarSinSalto,
    /// Palabra clave `intentar` (ADR-0014 §3.1).
    Intentar,
    /// Palabra clave `rescatar` (ADR-0014 §3.1).
    Rescatar,
    /// Palabra clave `importar` (módulos básicos).
    Importar,
    /// Palabra clave `exportar` (módulos avanzados: visibilidad).
    Exportar,
    /// Palabra clave `usar` (ADR-0017 D1).
    Usar,
    /// Palabra clave `como` (ADR-0017 D1).
    Como,
    /// Palabra clave `publico` (ADR-0017 D2).
    Publico,
    /// Palabra clave `estructura` (Fase 08 S8.1).
    Estructura,
    /// Palabra clave `enumeracion` (Fase 08 S8.2).
    Enumeracion,
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
    /// Archivo de origen, si está disponible (ADR-0017 D5).
    pub archivo: Option<String>,
}

impl std::fmt::Display for ErrorLexico {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.archivo {
            Some(archivo) => write!(
                f,
                "error léxico [{}:{}:{}]: {}",
                archivo, self.linea, self.columna, self.mensaje
            ),
            None => write!(
                f,
                "error léxico [{}:{}]: {}",
                self.linea, self.columna, self.mensaje
            ),
        }
    }
}
