//! Analizador léxico de BASILISCO.
//!
//! Implementa el borrador aprobado en `02-lenguaje/especificacion-lexica-minima.md`
//! (decisiones del ADR-0003). Contratos:
//! - UTF-8; el texto ya viene como `&str` (Rust garantiza UTF-8). BOM inicial se ignora.
//! - Nada se descarta en silencio: carácter inválido = error con ubicación.
//! - Identificadores ASCII provisional; cadenas admiten Unicode.

use crate::token::{ErrorLexico, TipoToken, Token};

pub struct Lexer {
    entrada: Vec<char>,
    pos: usize,
    linea: usize,
    columna: usize,
    /// Archivo de origen, si está disponible (ADR-0017 D5).
    archivo: Option<String>,
    /// Comentarios de línea `# ...` capturados (línea, texto sin el `#` inicial ni salto).
    comentarios: Vec<(usize, String)>,
}

impl Lexer {
    /// Crea un lexer sobre `fuente`. El BOM UTF-8 inicial se ignora si está presente.
    pub fn nuevo(fuente: &str) -> Self {
        Self::con_archivo(fuente, None)
    }

    /// Crea un lexer sobre `fuente` asociado a un archivo de origen.
    pub fn con_archivo(fuente: &str, archivo: Option<String>) -> Self {
        let sin_bom = fuente.strip_prefix('\u{feff}').unwrap_or(fuente);
        Lexer {
            entrada: sin_bom.chars().collect(),
            pos: 0,
            linea: 1,
            columna: 1,
            archivo,
            comentarios: Vec::new(),
        }
    }

    fn actual(&self) -> Option<char> {
        self.entrada.get(self.pos).copied()
    }

    fn siguiente(&self) -> Option<char> {
        self.entrada.get(self.pos + 1).copied()
    }

    fn avanzar(&mut self) {
        if let Some(c) = self.actual() {
            self.pos += 1;
            if c == '\n' {
                self.linea += 1;
                self.columna = 1;
            } else {
                self.columna += 1;
            }
        }
    }

    fn error(&self, mensaje: impl Into<String>) -> ErrorLexico {
        ErrorLexico {
            mensaje: mensaje.into(),
            linea: self.linea,
            columna: self.columna,
            archivo: self.archivo.clone(),
        }
    }

    /// Produce la lista completa de tokens, terminando siempre con `TipoToken::Fin`.
    pub fn tokenizar(&mut self) -> Result<Vec<Token>, ErrorLexico> {
        let mut tokens = Vec::new();

        loop {
            self.omitir_espacios_y_comentarios();

            let linea = self.linea;
            let columna = self.columna;
            let Some(c) = self.actual() else {
                tokens.push(Token {
                    tipo: TipoToken::Fin,
                    lexema: String::new(),
                    linea,
                    columna,
                });
                return Ok(tokens);
            };

            let inicio = self.pos;
            let tipo = match c {
                '\n' => {
                    self.avanzar();
                    TipoToken::FinDeLinea
                }
                '\r' => {
                    if self.siguiente() == Some('\n') {
                        self.avanzar(); // \r
                        self.avanzar(); // \n
                        TipoToken::FinDeLinea
                    } else {
                        return Err(self.error(
                            "retorno de carro suelto no soportado; use fin de línea \\n o \\r\\n",
                        ));
                    }
                }
                '+' => {
                    self.avanzar();
                    TipoToken::Mas
                }
                '-' => {
                    self.avanzar();
                    if self.actual() == Some('>') {
                        self.avanzar();
                        TipoToken::Flecha
                    } else {
                        TipoToken::Menos
                    }
                }
                '*' => {
                    self.avanzar();
                    TipoToken::Por
                }
                '%' => {
                    self.avanzar();
                    TipoToken::Modulo
                }
                '/' => {
                    self.avanzar();
                    TipoToken::Dividido
                }
                '(' => {
                    self.avanzar();
                    TipoToken::ParentesisIzq
                }
                ')' => {
                    self.avanzar();
                    TipoToken::ParentesisDer
                }
                '{' => {
                    self.avanzar();
                    TipoToken::LlaveIzq
                }
                '}' => {
                    self.avanzar();
                    TipoToken::LlaveDer
                }
                ',' => {
                    self.avanzar();
                    TipoToken::Coma
                }
                ':' => {
                    self.avanzar();
                    TipoToken::DosPuntos
                }
                '[' => {
                    self.avanzar();
                    TipoToken::CorcheteIzq
                }
                ']' => {
                    self.avanzar();
                    TipoToken::CorcheteDer
                }
                '=' => {
                    self.avanzar();
                    if self.actual() == Some('=') {
                        self.avanzar();
                        TipoToken::IgualIgual
                    } else if self.actual() == Some('>') {
                        self.avanzar();
                        TipoToken::FlechaDoble
                    } else {
                        TipoToken::Asignacion
                    }
                }
                '!' => {
                    self.avanzar();
                    if self.actual() == Some('=') {
                        self.avanzar();
                        TipoToken::Distinto
                    } else {
                        return Err(self.error("operador '!' no soportado; use 'no' o '!='"));
                    }
                }
                '<' => {
                    self.avanzar();
                    if self.actual() == Some('=') {
                        self.avanzar();
                        TipoToken::MenorIgual
                    } else {
                        TipoToken::Menor
                    }
                }
                '>' => {
                    self.avanzar();
                    if self.actual() == Some('=') {
                        self.avanzar();
                        TipoToken::MayorIgual
                    } else {
                        TipoToken::Mayor
                    }
                }
                '"' => self.leer_cadena()?,
                '.' => {
                    self.avanzar(); // primer '.'
                    if self.actual() == Some('.') {
                        self.avanzar(); // segundo '.'
                        if self.actual() == Some('=') {
                            self.avanzar(); // '='
                            TipoToken::RangoDosCerrado
                        } else {
                            TipoToken::RangoDos
                        }
                    } else if self
                        .actual()
                        .map(|c| c.is_ascii_alphabetic() || c == '_')
                        .unwrap_or(false)
                    {
                        // Acceso a miembro: `objeto.miembro` (ADR-0017 D8).
                        TipoToken::Punto
                    } else {
                        return Err(self.error(
                            "carácter inesperado: '.' suelto (¿quiso escribir '..' o '..='?)",
                        ));
                    }
                }
                '0'..='9' => self.leer_numero(),
                c if Self::es_inicio_identificador(c) => self.leer_identificador(),
                otro => {
                    return Err(self.error(format!("carácter inesperado: {otro:?}")));
                }
            };

            let lexema: String = self.entrada[inicio..self.pos].iter().collect();

            tokens.push(Token {
                tipo,
                lexema,
                linea,
                columna,
            });
        }
    }

    fn omitir_espacios_y_comentarios(&mut self) {
        loop {
            match self.actual() {
                Some(' ') | Some('\t') => self.avanzar(),
                // Comentario de línea: '#' hasta antes del fin de línea.
                Some('#') => {
                    let linea = self.linea;
                    let inicio = self.pos;
                    while let Some(c) = self.actual() {
                        if c == '\n' || c == '\r' {
                            break;
                        }
                        self.avanzar();
                    }
                    let texto: String = self.entrada[inicio..self.pos].iter().collect();
                    self.comentarios.push((linea, texto));
                }
                _ => break,
            }
        }
    }

    /// Comentarios de línea capturados durante la tokenización.
    pub fn comentarios(&self) -> &[(usize, String)] {
        &self.comentarios
    }

    fn leer_numero(&mut self) -> TipoToken {
        while matches!(self.actual(), Some(c) if c.is_ascii_digit()) {
            self.avanzar();
        }
        // Decimal: solo si hay '.' seguido de al menos un dígito.
        if self.actual() == Some('.') && matches!(self.siguiente(), Some(c) if c.is_ascii_digit()) {
            self.avanzar(); // '.'
            while matches!(self.actual(), Some(c) if c.is_ascii_digit()) {
                self.avanzar();
            }
        }
        // Exponencial (real): 'e'/'E' seguido de dígitos u opcionalmente signo y dígitos.
        if matches!(self.actual(), Some('e') | Some('E'))
            && matches!(
                self.siguiente(),
                Some(c) if c.is_ascii_digit()
                    || ((c == '+' || c == '-')
                        && self.entrada.get(self.pos + 2).is_some_and(|d| d.is_ascii_digit()))
            )
        {
            self.avanzar(); // 'e'/'E'
            if matches!(self.actual(), Some('+') | Some('-')) {
                self.avanzar();
            }
            while matches!(self.actual(), Some(c) if c.is_ascii_digit()) {
                self.avanzar();
            }
        }
        TipoToken::Numero
    }

    fn es_inicio_identificador(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_'
    }

    fn es_parte_identificador(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    fn leer_identificador(&mut self) -> TipoToken {
        let inicio = self.pos;
        while let Some(c) = self.actual() {
            if Self::es_parte_identificador(c) {
                self.avanzar();
            } else {
                break;
            }
        }
        let texto: String = self.entrada[inicio..self.pos].iter().collect();
        // Las palabras clave son exactas y en minúscula.
        match texto.as_str() {
            "sea" => TipoToken::Sea,
            "var" => TipoToken::Var,
            "dato" => TipoToken::Dato,
            "const" => TipoToken::Const,
            "arreglo" => TipoToken::Arreglo,
            "lista" => TipoToken::Lista,
            "funcion" => TipoToken::Funcion,
            "retornar" => TipoToken::Retornar,
            "mostrar" => TipoToken::Mostrar,
            "si" => TipoToken::Si,
            "sino" => TipoToken::Sino,
            "mientras" => TipoToken::Mientras,
            "y" => TipoToken::Y,
            "o" => TipoToken::O,
            "no" => TipoToken::No,
            "verdadero" => TipoToken::Verdadero,
            "falso" => TipoToken::Falso,
            "nulo" => TipoToken::Nulo,
            "para" => TipoToken::Para,
            "cada" => TipoToken::Cada,
            "en" => TipoToken::En,
            "romper" => TipoToken::Romper,
            "continuar" => TipoToken::Continuar,
            "mostrar_sin_salto" => TipoToken::MostrarSinSalto,
            "intentar" => TipoToken::Intentar,
            "rescatar" => TipoToken::Rescatar,
            "importar" => TipoToken::Importar,
            "exportar" => TipoToken::Exportar,
            "usar" => TipoToken::Usar,
            "como" => TipoToken::Como,
            "publico" => TipoToken::Publico,
            "estructura" => TipoToken::Estructura,
            "enumeracion" => TipoToken::Enumeracion,
            "coincidir" => TipoToken::Coincidir,
            "implementar" => TipoToken::Implementar,
            "rasgo" => TipoToken::Rasgo,
            "yo" => TipoToken::Yo,
            _ => TipoToken::Identificador(texto),
        }
    }

    /// Consume una cadena entre comillas dobles y devuelve el contenido con escapes resueltos.
    fn leer_cadena(&mut self) -> Result<TipoToken, ErrorLexico> {
        self.avanzar(); // comilla de apertura
        let mut contenido = String::new();
        loop {
            let Some(c) = self.actual() else {
                return Err(self.error("cadena sin cerrar: falta la comilla de cierre"));
            };
            match c {
                '"' => {
                    self.avanzar();
                    return Ok(TipoToken::Cadena(contenido));
                }
                '\n' | '\r' => {
                    return Err(self.error("cadena sin cerrar: salto de línea dentro de la cadena"));
                }
                '\\' => {
                    self.avanzar(); // barra
                    let Some(esc) = self.actual() else {
                        return Err(self.error("secuencia de escape incompleta"));
                    };
                    match esc {
                        '\\' => contenido.push('\\'),
                        '"' => contenido.push('"'),
                        'n' => contenido.push('\n'),
                        't' => contenido.push('\t'),
                        otro => {
                            return Err(self.error(format!(
                                "escape no soportado: \\{otro} (soportados: \\\\ \\\" \\n \\t)"
                            )));
                        }
                    }
                    self.avanzar();
                }
                otro => {
                    contenido.push(otro);
                    self.avanzar();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tipos(fuente: &str) -> Vec<TipoToken> {
        Lexer::nuevo(fuente)
            .tokenizar()
            .expect("tokenización válida")
            .into_iter()
            .map(|t| t.tipo)
            .collect()
    }

    #[test]
    fn suma_sencilla() {
        assert_eq!(
            tipos("10 + 5"),
            vec![
                TipoToken::Numero,
                TipoToken::Mas,
                TipoToken::Numero,
                TipoToken::Fin
            ]
        );
    }

    #[test]
    fn lexema_numero_se_preserva() {
        let ts = Lexer::nuevo("123.45").tokenizar().unwrap();
        assert_eq!(ts[0].lexema, "123.45");
        assert_eq!(ts[0].tipo, TipoToken::Numero);
    }

    #[test]
    fn decimal_requiere_digitos_en_ambos_lados() {
        // "123." no es decimal: el punto suelto no es token y debe fallar.
        assert!(Lexer::nuevo("123.").tokenizar().is_err());
    }

    #[test]
    fn identificadores_validos() {
        assert_eq!(
            tipos("numero total_ventas contador2 _init"),
            vec![
                TipoToken::Identificador("numero".into()),
                TipoToken::Identificador("total_ventas".into()),
                TipoToken::Identificador("contador2".into()),
                TipoToken::Identificador("_init".into()),
                TipoToken::Fin
            ]
        );
    }

    #[test]
    fn identificador_no_puede_empezar_con_digito() {
        // "2abc" se tokeniza como número 2 seguido de identificador abc; el parser lo rechaza.
        assert_eq!(
            tipos("2abc"),
            vec![
                TipoToken::Numero,
                TipoToken::Identificador("abc".into()),
                TipoToken::Fin
            ]
        );
    }

    #[test]
    fn palabras_clave_minusculas() {
        assert_eq!(
            tipos("sea var dato const funcion retornar mostrar"),
            vec![
                TipoToken::Sea,
                TipoToken::Var,
                TipoToken::Dato,
                TipoToken::Const,
                TipoToken::Funcion,
                TipoToken::Retornar,
                TipoToken::Mostrar,
                TipoToken::Fin
            ]
        );
        // Con mayúscula inicial no es palabra clave.
        assert_eq!(
            tipos("Funcion"),
            vec![TipoToken::Identificador("Funcion".into()), TipoToken::Fin]
        );
    }

    #[test]
    fn declaracion_y_asignacion_tokenizan() {
        assert_eq!(
            tipos("sea x = 5"),
            vec![
                TipoToken::Sea,
                TipoToken::Identificador("x".into()),
                TipoToken::Asignacion,
                TipoToken::Numero,
                TipoToken::Fin
            ]
        );
        assert_eq!(
            tipos("x = x + 1"),
            vec![
                TipoToken::Identificador("x".into()),
                TipoToken::Asignacion,
                TipoToken::Identificador("x".into()),
                TipoToken::Mas,
                TipoToken::Numero,
                TipoToken::Fin
            ]
        );
    }

    #[test]
    fn cadena_simple_y_con_escapes() {
        assert_eq!(
            tipos(r#""hola""#),
            vec![TipoToken::Cadena("hola".into()), TipoToken::Fin]
        );
        assert_eq!(
            tipos(r#""a\nb\tc\"d\\e""#),
            vec![TipoToken::Cadena("a\nb\tc\"d\\e".into()), TipoToken::Fin]
        );
    }

    #[test]
    fn cadena_admite_unicode() {
        assert_eq!(
            tipos(r#""señor año válido""#),
            vec![TipoToken::Cadena("señor año válido".into()), TipoToken::Fin]
        );
    }

    #[test]
    fn cadena_sin_cerrar_y_escape_invalido() {
        assert!(Lexer::nuevo(r#""hola"#).tokenizar().is_err());
        assert!(Lexer::nuevo(r#""a\xb""#).tokenizar().is_err());
    }

    #[test]
    fn comentarios_se_omiten() {
        assert_eq!(
            tipos("5 # esto es un comentario"),
            vec![TipoToken::Numero, TipoToken::Fin]
        );
        assert_eq!(tipos("# comentario completo"), vec![TipoToken::Fin]);
    }

    #[test]
    fn numerales_finales_de_linea() {
        assert_eq!(
            tipos("a\nb"),
            vec![
                TipoToken::Identificador("a".into()),
                TipoToken::FinDeLinea,
                TipoToken::Identificador("b".into()),
                TipoToken::Fin
            ]
        );
        // CRLF cuenta como un solo fin de línea.
        assert_eq!(
            tipos("a\r\nb"),
            vec![
                TipoToken::Identificador("a".into()),
                TipoToken::FinDeLinea,
                TipoToken::Identificador("b".into()),
                TipoToken::Fin
            ]
        );
        // CR suelto no soportado.
        assert!(Lexer::nuevo("a\rb").tokenizar().is_err());
    }

    #[test]
    fn continuacion_de_expresion_tokeniza_correctamente() {
        // El lexer emite FinDeLinea; el parser decidirá que no termina sentencia tras un '+'.
        assert_eq!(
            tipos("total = 10 +\n    5"),
            vec![
                TipoToken::Identificador("total".into()),
                TipoToken::Asignacion,
                TipoToken::Numero,
                TipoToken::Mas,
                TipoToken::FinDeLinea,
                TipoToken::Numero,
                TipoToken::Fin
            ]
        );
    }

    #[test]
    fn caracter_inesperado_produce_error_con_ubicacion() {
        let err = Lexer::nuevo("a\n@").tokenizar().unwrap_err();
        assert_eq!(err.linea, 2);
        assert_eq!(err.columna, 1);
        assert!(err.mensaje.contains("inesperado"));
    }

    #[test]
    fn bom_inicial_se_ignora() {
        let ts = Lexer::nuevo("\u{feff}10").tokenizar().unwrap();
        assert_eq!(ts[0].tipo, TipoToken::Numero);
        assert_eq!(ts[0].lexema, "10");
    }

    #[test]
    fn posicion_del_token() {
        let ts = Lexer::nuevo("  10 + 5").tokenizar().unwrap();
        assert_eq!((ts[0].linea, ts[0].columna), (1, 3));
        assert_eq!((ts[1].linea, ts[1].columna), (1, 6));
    }

    #[test]
    fn palabras_clave_var_dato_const() {
        let ts = Lexer::nuevo("var dato const").tokenizar().unwrap();
        let tipos: Vec<_> = ts.iter().map(|t| &t.tipo).collect();
        assert!(matches!(tipos[0], TipoToken::Var));
        assert!(matches!(tipos[1], TipoToken::Dato));
        assert!(matches!(tipos[2], TipoToken::Const));
        assert!(matches!(tipos[3], TipoToken::Fin));
    }
}
