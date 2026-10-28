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

use crate::token::{TipoToken, Token};
use crate::valor::{MotivoAritmetico, Valor};

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
    /// Llamada a función (ADR-0008).
    Llamada {
        nombre: String,
        argumentos: Vec<Expresion>,
        linea: usize,
        columna: usize,
    },
    /// Literal de arreglo/lista `[...]` (resuelto en evaluación según tipo esperado).
    ArregloLiteral {
        elementos: Vec<Expresion>,
        linea: usize,
        columna: usize,
    },
    /// Indexación `objeto[indice]`.
    Indice {
        objeto: Box<Expresion>,
        indice: Box<Expresion>,
        linea: usize,
        columna: usize,
    },
    /// Acceso a miembro `objeto.miembro` (ADR-0017 D1/D8). Integración en Fase 3.
    Acceso {
        objeto: Box<Expresion>,
        miembro: String,
        linea: usize,
        columna: usize,
    },
    /// Llamada indirecta `objeto.miembro(args)` (ADR-0017 D8). Integración en Fase 3.
    LlamadaIndirecta {
        callee: Box<Expresion>,
        argumentos: Vec<Expresion>,
        linea: usize,
        columna: usize,
    },
    /// Rango `inicio..fin` o `inicio..=fin` (ADR-0012 §3.3).
    /// Solo válido como iterable en `para cada`; no se almacena en variable.
    Rango {
        inicio: Box<Expresion>,
        fin: Box<Expresion>,
        inclusivo: bool,
        linea: usize,
        columna: usize,
    },
}

/// Sentencias de un programa (ADR-0006, ampliadas ADR-0007, ADR-0008).
#[derive(Debug, Clone, PartialEq)]
pub enum Sentencia {
    /// `sea nombre = expresión` | `var nombre: tipo = expresión` | `dato nombre: tipo = expresión` | `const NOMBRE: tipo = expresión`.
    Declaracion {
        nombre: String,
        linea: usize,
        columna: usize,
        valor: Expresion,
        /// Tipo anotado (None para `sea` legacy).
        tipo_anotacion: Option<TipoAnotacion>,
        /// true = mutable (`sea`, `var`), false = inmutable (`dato`, `const`).
        mutable: bool,
        /// true = constante de compilación (`const`).
        es_const: bool,
        /// true = declaración pública (ADR-0017 D2). Fase 2: solo sintaxis; integración en Fase 3.
        publico: bool,
    },
    /// `nombre = expresión` sobre una variable existente.
    Asignacion {
        nombre: String,
        linea: usize,
        columna: usize,
        valor: Expresion,
    },
    /// `objeto[indice] = expresión` asignación por índice.
    AsignacionIndice {
        objeto: Box<Expresion>,
        indice: Box<Expresion>,
        valor: Expresion,
        linea: usize,
        columna: usize,
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
    /// Definición de función (ADR-0008, ampliada ADR-0011).
    Funcion {
        nombre: String,
        /// Parámetros con tipo anotado opcional. Todos tipados o ninguno (ADR-0011 §3.2).
        parametros: Vec<(String, Option<TipoAnotacion>)>,
        /// Tipo de retorno anotado (ADR-0011 §3.4).
        tipo_retorno: Option<TipoAnotacion>,
        cuerpo: Vec<Sentencia>,
        linea: usize,
        columna: usize,
        /// true = función pública (ADR-0017 D2). Fase 2: solo sintaxis; integración en Fase 3.
        publico: bool,
    },
    /// Retorno de función (ADR-0008).
    Retorno {
        valor: Option<Expresion>,
        linea: usize,
        columna: usize,
    },
    /// Sentencia `mostrar expresión`.
    Mostrar {
        valor: Expresion,
        linea: usize,
        columna: usize,
    },
    /// Sentencia `mostrar_sin_salto expresión` (ADR-0013 §3.3).
    MostrarSinSalto {
        valor: Expresion,
        linea: usize,
        columna: usize,
    },
    /// `para cada IDENT en expr_iteracion { ... }` (ADR-0012 §3.1).
    ParaCada {
        variable: String,
        iterable: Expresion,
        cuerpo: Vec<Sentencia>,
        linea: usize,
        columna: usize,
    },
    /// `romper` (ADR-0012 §3.2). Solo válido dentro de un ciclo.
    Romper { linea: usize, columna: usize },
    /// `continuar` (ADR-0012 §3.2). Solo válido dentro de un ciclo.
    Continuar { linea: usize, columna: usize },
    /// `intentar { ... } rescatar [IDENT] { ... }` (ADR-0014 §3.1).
    IntentarRescatar {
        cuerpo_intentar: Vec<Sentencia>,
        variable_error: Option<String>,
        cuerpo_rescatar: Vec<Sentencia>,
        linea: usize,
        columna: usize,
    },
    /// `importar "ruta" [ como alias ]` (módulos básicos/avanzados).
    Importar {
        ruta: String,
        alias: Option<String>,
        linea: usize,
        columna: usize,
    },
    /// `exportar sentencia` (módulos avanzados: visibilidad).
    Exportar {
        sentencia: Box<Sentencia>,
        linea: usize,
        columna: usize,
    },
    /// `usar "ruta" como alias` | `usar "ruta" { nombres }` (ADR-0017 D1). Fase 2: solo sintaxis.
    Usar {
        ruta: String,
        alias: Option<String>,
        nombres: Option<Vec<String>>,
        linea: usize,
        columna: usize,
    },
}

/// Anotación de tipo en declaraciones (var/dato/const).
#[derive(Debug, Clone, PartialEq)]
pub enum TipoAnotacion {
    Entero,
    Decimal,
    Real,
    Logico,
    Texto,
    Arreglo(Box<TipoAnotacion>, usize),
    Lista(Box<TipoAnotacion>),
    /// Tipo nulo (ADR-0011 §3.5). Válido en tipo de retorno `-> nulo`.
    Nulo,
}

impl TipoAnotacion {
    pub fn nombre(&self) -> String {
        match self {
            TipoAnotacion::Entero => "entero".to_string(),
            TipoAnotacion::Decimal => "decimal".to_string(),
            TipoAnotacion::Real => "real".to_string(),
            TipoAnotacion::Logico => "lógico".to_string(),
            TipoAnotacion::Texto => "texto".to_string(),
            TipoAnotacion::Arreglo(t, n) => format!("arreglo<{}, {}>", t.nombre(), n),
            TipoAnotacion::Lista(t) => format!("lista<{}>", t.nombre()),
            TipoAnotacion::Nulo => "nulo".to_string(),
        }
    }
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
    /// Archivo de origen, si está disponible (ADR-0017 D5).
    pub archivo: Option<String>,
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
        match &self.archivo {
            Some(archivo) => write!(
                f,
                "error sintáctico [{}:{}:{}]: {}",
                archivo, self.linea, self.columna, self.mensaje
            ),
            None => write!(
                f,
                "error sintáctico [{}:{}]: {}",
                self.linea, self.columna, self.mensaje
            ),
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Profundidad de agrupación abierta; > 0 significa dentro de `( … )`.
    en_grupo: usize,
    /// Estamos dentro de una definición de función (para rechazar anidación).
    en_funcion: bool,
    /// Profundidad de ciclos anidados (para detectar `romper`/`continuar` fuera de ciclo).
    en_ciclo: usize,
    /// Profundidad de bloques anidados (0 = nivel superior). Para validar `publico`.
    profundidad_bloque: usize,
    /// Archivo de origen, si está disponible (ADR-0017 D5).
    archivo: Option<String>,
}

impl Parser {
    pub fn nuevo(tokens: Vec<Token>) -> Self {
        Self::con_archivo(tokens, None)
    }

    pub fn con_archivo(tokens: Vec<Token>, archivo: Option<String>) -> Self {
        Parser {
            tokens,
            pos: 0,
            en_grupo: 0,
            en_funcion: false,
            en_ciclo: 0,
            profundidad_bloque: 0,
            archivo,
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
        // Modificador `publico` (ADR-0017 D2): solo en nivel superior.
        let publico = if matches!(self.actual(), Some(TipoToken::Publico)) {
            if self.profundidad_bloque > 0 {
                return Err(self.error("'publico' solo tiene sentido en el nivel superior"));
            }
            self.pos += 1; // consume 'publico'
            true
        } else {
            false
        };

        if publico
            && !matches!(
                self.actual(),
                Some(
                    TipoToken::Sea
                        | TipoToken::Var
                        | TipoToken::Dato
                        | TipoToken::Const
                        | TipoToken::Funcion
                )
            )
        {
            return Err(self.error("'publico' solo puede preceder a una declaración o función"));
        }

        if matches!(
            self.actual(),
            Some(TipoToken::Sea | TipoToken::Var | TipoToken::Dato | TipoToken::Const)
        ) {
            return self.sentencia_declaracion(publico);
        } else if matches!(self.actual(), Some(TipoToken::Funcion)) {
            let st = self.sentencia_funcion(publico)?;
            self.terminar_sentencia()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Retornar)) {
            return self.sentencia_retorno();
        } else if matches!(self.actual(), Some(TipoToken::Si)) {
            let st = self.sentencia_si()?;
            self.terminar_sentencia()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Mientras)) {
            let st = self.sentencia_mientras()?;
            self.terminar_sentencia()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Mostrar)) {
            return self.sentencia_mostrar();
        } else if matches!(self.actual(), Some(TipoToken::MostrarSinSalto)) {
            return self.sentencia_mostrar_sin_salto();
        } else if matches!(self.actual(), Some(TipoToken::Para)) {
            let st = self.sentencia_para_cada()?;
            self.terminar_sentencia()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Romper)) {
            let t = &self.tokens[self.pos];
            if self.en_ciclo == 0 {
                return Err(self.error("'romper' fuera de un ciclo"));
            }
            let (linea, columna) = (t.linea, t.columna);
            self.pos += 1;
            self.terminar_sentencia()?;
            return Ok(Sentencia::Romper { linea, columna });
        } else if matches!(self.actual(), Some(TipoToken::Continuar)) {
            let t = &self.tokens[self.pos];
            if self.en_ciclo == 0 {
                return Err(self.error("'continuar' fuera de un ciclo"));
            }
            let (linea, columna) = (t.linea, t.columna);
            self.pos += 1;
            self.terminar_sentencia()?;
            return Ok(Sentencia::Continuar { linea, columna });
        } else if matches!(self.actual(), Some(TipoToken::Importar)) {
            let st = self.sentencia_importar()?;
            self.terminar_sentencia()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Exportar)) {
            let st = self.sentencia_exportar()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Intentar)) {
            let st = self.sentencia_intentar_rescatar()?;
            self.terminar_sentencia()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Usar)) {
            let st = self.sentencia_usar()?;
            self.terminar_sentencia()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Rescatar)) {
            return Err(self.error("'rescatar' sin un 'intentar' correspondiente"));
        } else if matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            let bloque = self.bloque()?;
            self.terminar_sentencia()?;
            return Ok(Sentencia::Bloque(bloque));
        } else if matches!(self.actual(), Some(TipoToken::Sino)) {
            return Err(self.error("'sino' sin un 'si' correspondiente"));
        }

        if matches!(
            self.actual(),
            Some(TipoToken::Identificador(_))
                | Some(TipoToken::CorcheteIzq)
                | Some(TipoToken::ParentesisIzq)
        ) {
            let pos_guardado = self.pos;
            if let Ok(expr_cand) = self.primaria()
                && matches!(self.actual(), Some(TipoToken::Asignacion))
            {
                self.pos += 1; // '='
                let valor = self.expresion()?;
                self.terminar_sentencia()?;
                match expr_cand {
                    Expresion::Variable {
                        nombre,
                        linea,
                        columna,
                    } => {
                        return Ok(Sentencia::Asignacion {
                            nombre,
                            linea,
                            columna,
                            valor,
                        });
                    }
                    Expresion::Indice {
                        objeto,
                        indice,
                        linea,
                        columna,
                    } => {
                        return Ok(Sentencia::AsignacionIndice {
                            objeto,
                            indice,
                            valor,
                            linea,
                            columna,
                        });
                    }
                    _ => return Err(self.error("lado izquierdo de asignación inválido")),
                }
            }
            self.pos = pos_guardado;
        }

        let e = self.expresion()?;
        self.terminar_sentencia()?;
        Ok(Sentencia::Expresion(e))
    }

    fn sentencia_declaracion(&mut self, publico: bool) -> Result<Sentencia, ErrorSintactico> {
        let (keyword, mutable, es_const) = match self.actual() {
            Some(TipoToken::Sea) => ("sea", true, false),
            Some(TipoToken::Var) => ("var", true, false),
            Some(TipoToken::Dato) => ("dato", false, false),
            Some(TipoToken::Const) => ("const", false, true),
            _ => return Err(self.error("palabra clave de declaración no reconocida")),
        };
        self.pos += 1; // keyword
        let (nombre, linea, columna) = match self.actual() {
            Some(TipoToken::Identificador(n)) => {
                let t = &self.tokens[self.pos];
                (n.clone(), t.linea, t.columna)
            }
            _ => return Err(self.error(format!("se esperaba un nombre tras '{}'", keyword))),
        };
        self.pos += 1; // identificador

        // Tipo anotado obligatorio para var/dato/const, opcional para sea
        let tipo_anotacion = if keyword == "sea" {
            None
        } else {
            Some(self.parsear_tipo_anotacion()?)
        };

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
            tipo_anotacion,
            mutable,
            es_const,
            publico,
        })
    }

    /// Parsea una anotación de tipo: entero | decimal | real | logico | texto | nulo
    fn parsear_tipo_anotacion(&mut self) -> Result<TipoAnotacion, ErrorSintactico> {
        if !matches!(self.actual(), Some(TipoToken::DosPuntos)) {
            return Err(self.error("se esperaba ':' para anotación de tipo"));
        }
        self.pos += 1; // ':'
        self.parsear_tipo_anotacion_cuerpo()
    }

    fn parsear_tipo_anotacion_simple(&mut self) -> Result<TipoAnotacion, ErrorSintactico> {
        match self.actual() {
            Some(TipoToken::Identificador(t)) if t == "entero" => {
                self.pos += 1;
                Ok(TipoAnotacion::Entero)
            }
            Some(TipoToken::Identificador(t)) if t == "decimal" => {
                self.pos += 1;
                Ok(TipoAnotacion::Decimal)
            }
            Some(TipoToken::Identificador(t)) if t == "real" => {
                self.pos += 1;
                Ok(TipoAnotacion::Real)
            }
            Some(TipoToken::Identificador(t)) if t == "logico" => {
                self.pos += 1;
                Ok(TipoAnotacion::Logico)
            }
            Some(TipoToken::Identificador(t)) if t == "texto" => {
                self.pos += 1;
                Ok(TipoAnotacion::Texto)
            }
            Some(TipoToken::Nulo) => {
                self.pos += 1;
                Ok(TipoAnotacion::Nulo)
            }
            _ => Err(self.error("tipo desconocido; se esperaba: entero, decimal, real, logico, texto, nulo, arreglo<T, N>, lista<T>")),
        }
    }

    /// Parsea un tipo de anotación sin consumir el `:` previo (para tipo de retorno `-> tipo`).
    fn parsear_tipo_anotacion_cuerpo(&mut self) -> Result<TipoAnotacion, ErrorSintactico> {
        if matches!(self.actual(), Some(TipoToken::Arreglo)) {
            self.pos += 1;
            if !matches!(self.actual(), Some(TipoToken::Menor)) {
                return Err(self.error("se esperaba '<' después de 'arreglo'"));
            }
            self.pos += 1;
            let tipo_elem = self.parsear_tipo_anotacion_simple()?;
            self.omitir_finales_de_linea();
            if !matches!(self.actual(), Some(TipoToken::Coma)) {
                return Err(self.error("se esperaba ',' después del tipo de elemento en arreglo"));
            }
            self.pos += 1;
            self.omitir_finales_de_linea();
            let tamano = match self.actual() {
                Some(TipoToken::Numero) => {
                    let t = &self.tokens[self.pos];
                    let tam: usize = t
                        .lexema
                        .parse()
                        .map_err(|_| self.error("tamaño de arreglo inválido"))?;
                    if tam == 0 {
                        return Err(self.error("el tamaño del arreglo debe ser mayor que 0"));
                    }
                    self.pos += 1;
                    tam
                }
                _ => return Err(self.error("se esperaba un número para el tamaño del arreglo")),
            };
            self.omitir_finales_de_linea();
            if !matches!(self.actual(), Some(TipoToken::Mayor)) {
                return Err(self.error("se esperaba '>' para cerrar el tipo de arreglo"));
            }
            self.pos += 1;
            return Ok(TipoAnotacion::Arreglo(Box::new(tipo_elem), tamano));
        }
        if matches!(self.actual(), Some(TipoToken::Lista)) {
            self.pos += 1;
            if !matches!(self.actual(), Some(TipoToken::Menor)) {
                return Err(self.error("se esperaba '<' después de 'lista'"));
            }
            self.pos += 1;
            let tipo_elem = self.parsear_tipo_anotacion_simple()?;
            self.omitir_finales_de_linea();
            if !matches!(self.actual(), Some(TipoToken::Mayor)) {
                return Err(self.error("se esperaba '>' para cerrar el tipo de lista"));
            }
            self.pos += 1;
            return Ok(TipoAnotacion::Lista(Box::new(tipo_elem)));
        }
        self.parsear_tipo_anotacion_simple()
    }

    fn bloque(&mut self) -> Result<Vec<Sentencia>, ErrorSintactico> {
        // '{' debe estar en la misma línea que el encabezado (regla ADR-0007).
        self.pos += 1; // '{'
        self.omitir_finales_de_linea();
        self.profundidad_bloque += 1;
        let mut sentencias = Vec::new();
        loop {
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                self.pos += 1; // '}'
                break;
            }
            if matches!(self.actual(), Some(TipoToken::Fin) | None) {
                self.profundidad_bloque -= 1;
                return Err(self.error("falta '}' para cerrar el bloque"));
            }
            sentencias.push(self.sentencia()?);
        }
        self.profundidad_bloque -= 1;
        Ok(sentencias)
    }

    fn sentencia_funcion(&mut self, publico: bool) -> Result<Sentencia, ErrorSintactico> {
        if self.en_funcion {
            return Err(self.error("funciones anidadas no están permitidas (ADR-0008)"));
        }
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'funcion'
        self.en_funcion = true;
        let nombre = match self.actual() {
            Some(TipoToken::Identificador(n)) => {
                self.pos += 1;
                n
            }
            _ => return Err(self.error("se esperaba el nombre de la función")),
        };
        if !matches!(self.actual(), Some(TipoToken::ParentesisIzq)) {
            return Err(self.error("se esperaba '(' tras el nombre de la función"));
        }
        self.pos += 1; // '('
        let mut parametros: Vec<(String, Option<TipoAnotacion>)> = Vec::new();
        // Primer parámetro determina si se usan tipos (todo-o-nada, ADR-0011 §3.2).
        let mut primer_tiene_tipo: Option<bool> = None;
        loop {
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                self.pos += 1;
                break;
            }
            let param_nombre = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    self.pos += 1;
                    n
                }
                _ => return Err(self.error("se esperaba un parámetro")),
            };
            // Tipo anotado opcional
            let tipo_param = if matches!(self.actual(), Some(TipoToken::DosPuntos)) {
                let tipo = self.parsear_tipo_anotacion()?;
                Some(tipo)
            } else {
                None
            };
            // Verificar consistencia todo-o-nada
            let este_tiene_tipo = tipo_param.is_some();
            match primer_tiene_tipo {
                None => primer_tiene_tipo = Some(este_tiene_tipo),
                Some(pt) if pt != este_tiene_tipo => {
                    return Err(self.error(format!(
                        "la firma de '{}' mezcla parámetros con y sin tipo; anota todos o ninguno",
                        nombre
                    )));
                }
                _ => {}
            }
            parametros.push((param_nombre, tipo_param));
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::Coma)) {
                self.pos += 1;
                continue;
            }
            if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                self.pos += 1;
                break;
            }
            return Err(self.error("se esperaba ',' o ')'"));
        }
        // Tipo de retorno opcional `-> tipo` (ADR-0011 §3.4)
        let tipo_retorno = if matches!(self.actual(), Some(TipoToken::Flecha)) {
            self.pos += 1; // '->'
            Some(self.parsear_tipo_anotacion_cuerpo()?)
        } else {
            None
        };
        if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            return Err(self.error("se esperaba '{' en la misma línea que la función"));
        }
        let llave = &self.tokens[self.pos];
        if llave.linea != linea {
            return Err(self.error("el '{' debe estar en la misma línea que 'funcion'"));
        }
        let cuerpo = self.bloque()?;
        self.en_funcion = false;
        Ok(Sentencia::Funcion {
            nombre,
            parametros,
            tipo_retorno,
            cuerpo,
            linea,
            columna,
            publico,
        })
    }

    fn sentencia_usar(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'usar'

        // Ruta (cadena)
        let ruta = match self.actual() {
            Some(TipoToken::Cadena(s)) => {
                let r = s.clone();
                self.pos += 1;
                r
            }
            _ => {
                return Err(self.error("se esperaba una cadena con la ruta del módulo tras 'usar'"));
            }
        };

        // Forma 1: `usar "ruta" como alias`
        if matches!(self.actual(), Some(TipoToken::Como)) {
            self.pos += 1; // 'como'
            let alias = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let a = n.clone();
                    self.pos += 1;
                    a
                }
                _ => return Err(self.error("se esperaba un nombre de alias tras 'como'")),
            };
            return Ok(Sentencia::Usar {
                ruta,
                alias: Some(alias),
                nombres: None,
                linea,
                columna,
            });
        }

        // Forma 2: `usar "ruta" { nombre1, nombre2 }`
        if matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            self.pos += 1; // '{'
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                return Err(self.error("la lista de importación no puede estar vacía"));
            }
            let mut nombres = Vec::new();
            loop {
                self.omitir_finales_de_linea();
                match self.actual() {
                    Some(TipoToken::Identificador(n)) => {
                        nombres.push(n.clone());
                        self.pos += 1;
                    }
                    _ => return Err(self.error("se esperaba un nombre en la lista de importación")),
                }
                self.omitir_finales_de_linea();
                if matches!(self.actual(), Some(TipoToken::Coma)) {
                    self.pos += 1;
                    self.omitir_finales_de_linea();
                    continue;
                }
                if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                    self.pos += 1; // '}'
                    break;
                }
                return Err(self.error("se esperaba ',' o '}' en la lista de importación"));
            }
            return Ok(Sentencia::Usar {
                ruta,
                alias: None,
                nombres: Some(nombres),
                linea,
                columna,
            });
        }

        Err(self.error("se esperaba 'como <alias>' o '{ nombres }' tras la ruta"))
    }

    fn sentencia_retorno(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        if !self.en_funcion {
            return Err(self.error("'retornar' solo puede usarse dentro de una función"));
        }
        self.pos += 1; // 'retornar'
        match self.actual() {
            Some(TipoToken::FinDeLinea)
            | Some(TipoToken::Fin)
            | None
            | Some(TipoToken::LlaveDer) => {
                self.terminar_sentencia()?;
                Ok(Sentencia::Retorno {
                    valor: None,
                    linea,
                    columna,
                })
            }
            _ => {
                let expr = self.expresion()?;
                self.terminar_sentencia()?;
                Ok(Sentencia::Retorno {
                    valor: Some(expr),
                    linea,
                    columna,
                })
            }
        }
    }

    /// Parsea una sentencia `mostrar`.
    fn sentencia_mostrar(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'mostrar'
        let expr = self.expresion()?;
        self.terminar_sentencia()?;
        Ok(Sentencia::Mostrar {
            valor: expr,
            linea,
            columna,
        })
    }

    /// Parsea una sentencia `mostrar_sin_salto`.
    fn sentencia_mostrar_sin_salto(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'mostrar_sin_salto'
        let expr = self.expresion()?;
        self.terminar_sentencia()?;
        Ok(Sentencia::MostrarSinSalto {
            valor: expr,
            linea,
            columna,
        })
    }

    /// Parsea `para cada IDENT en expr_iteracion bloque`.
    fn sentencia_para_cada(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'para'
        if !matches!(self.actual(), Some(TipoToken::Cada)) {
            return Err(self.error("se esperaba 'cada' después de 'para'"));
        }
        self.pos += 1; // 'cada'
        let variable = match self.actual() {
            Some(TipoToken::Identificador(n)) => {
                self.pos += 1;
                n
            }
            _ => return Err(self.error("se esperaba el nombre de la variable de iteración")),
        };
        if !matches!(self.actual(), Some(TipoToken::En)) {
            return Err(self.error("se esperaba 'en' después del nombre de la variable"));
        }
        self.pos += 1; // 'en'
        let iterable = self.expr_iteracion()?;
        self.exigir_llave_misma_linea(linea, "para")?;
        self.en_ciclo += 1;
        let cuerpo = self.bloque()?;
        self.en_ciclo -= 1;
        Ok(Sentencia::ParaCada {
            variable,
            iterable,
            cuerpo,
            linea,
            columna,
        })
    }

    /// Parsea una expresión de iteración: puede ser colección o rango `inicio..fin` / `inicio..=fin`.
    fn expr_iteracion(&mut self) -> Result<Expresion, ErrorSintactico> {
        let inicio = self.expresion()?;
        let (linea, columna) = ubicacion_expr(&inicio);
        if matches!(self.actual(), Some(TipoToken::RangoDos)) {
            self.pos += 1;
            let fin = self.expresion()?;
            Ok(Expresion::Rango {
                inicio: Box::new(inicio),
                fin: Box::new(fin),
                inclusivo: false,
                linea,
                columna,
            })
        } else if matches!(self.actual(), Some(TipoToken::RangoDosCerrado)) {
            self.pos += 1;
            let fin = self.expresion()?;
            Ok(Expresion::Rango {
                inicio: Box::new(inicio),
                fin: Box::new(fin),
                inclusivo: true,
                linea,
                columna,
            })
        } else {
            Ok(inicio)
        }
    }

    /// Parsea `intentar { ... } rescatar [IDENT] { ... }`.
    fn sentencia_intentar_rescatar(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'intentar'
        if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            return Err(self.error("se esperaba '{' en la misma línea que 'intentar'"));
        }
        let llave = &self.tokens[self.pos];
        if llave.linea != linea {
            return Err(self.error("el '{' debe estar en la misma línea que 'intentar'"));
        }
        let cuerpo_intentar = self.bloque()?;
        // `rescatar` puede ir en la línea siguiente
        let guarda = self.pos;
        self.omitir_finales_de_linea();
        if !matches!(self.actual(), Some(TipoToken::Rescatar)) {
            self.pos = guarda;
            return Err(self.error("se esperaba 'rescatar' después del bloque 'intentar'"));
        }
        let linea_rescatar = self.tokens[self.pos].linea;
        self.pos += 1; // 'rescatar'
        // Variable de error opcional
        let variable_error = if let Some(TipoToken::Identificador(n)) = self.actual() {
            self.pos += 1;
            Some(n)
        } else {
            None
        };
        if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            return Err(self.error("se esperaba '{' en la misma línea que 'rescatar'"));
        }
        let llave2 = &self.tokens[self.pos];
        if llave2.linea != linea_rescatar {
            return Err(self.error("el '{' debe estar en la misma línea que 'rescatar'"));
        }
        let cuerpo_rescatar = self.bloque()?;
        Ok(Sentencia::IntentarRescatar {
            cuerpo_intentar,
            variable_error,
            cuerpo_rescatar,
            linea,
            columna,
        })
    }

    fn sentencia_importar(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'importar'
        let ruta = match self.actual() {
            Some(TipoToken::Cadena(s)) => {
                let r = s.clone();
                self.pos += 1;
                r
            }
            _ => return Err(self.error("se esperaba una cadena con la ruta del módulo")),
        };
        let alias = if matches!(self.actual(), Some(TipoToken::Como)) {
            self.pos += 1; // 'como'
            match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let a = n.clone();
                    self.pos += 1;
                    Some(a)
                }
                _ => return Err(self.error("se esperaba un nombre de alias tras 'como'")),
            }
        } else {
            None
        };
        Ok(Sentencia::Importar {
            ruta,
            alias,
            linea,
            columna,
        })
    }

    fn sentencia_exportar(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'exportar'
        let inner = self.sentencia()?;
        Ok(Sentencia::Exportar {
            sentencia: Box::new(inner),
            linea,
            columna,
        })
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
                _ => return Err(self.error("se esperaba '{' o 'si' en la misma línea que 'sino'")),
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
        self.en_ciclo += 1;
        let cuerpo = self.bloque()?;
        self.en_ciclo -= 1;
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
            return Err(self.error(format!(
                "se esperaba '{{' en la misma línea que '{palabra}'"
            )));
        }
        match self.linea_actual() {
            Some(l) if l == linea_palabra => Ok(()),
            _ => Err(self.error(format!(
                "se esperaba '{{' en la misma línea que '{palabra}'"
            ))),
        }
    }

    fn actual(&self) -> Option<TipoToken> {
        self.tokens.get(self.pos).map(|t| t.tipo.clone())
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
                archivo: self.archivo.clone(),
            },
            None => ErrorSintactico {
                mensaje: mensaje.into(),
                linea: self.tokens.last().map_or(1, |t| t.linea),
                columna: self.tokens.last().map_or(1, |t| t.columna + 1),
                archivo: self.archivo.clone(),
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

    fn primaria_sin_indice(&mut self) -> Result<Expresion, ErrorSintactico> {
        // Continuación contextual: tras un operador o '(' el salto de línea no termina.
        self.omitir_finales_de_linea();
        match self.actual() {
            // Literal negativo en posición primaria (no es operador unario general:
            // `-x` con x expresión sigue prohibido por ADR-0004). Necesario para la
            // semántica aprobada de D4 (`-7 % 3` = -1).
            Some(TipoToken::Menos)
                if matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.tipo),
                    Some(TipoToken::Numero)
                ) =>
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
                    archivo: self.archivo.clone(),
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
                    archivo: self.archivo.clone(),
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
            Some(TipoToken::Nulo) => {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                self.pos += 1;
                Ok(Expresion::Literal {
                    valor: Valor::Nulo,
                    linea,
                    columna,
                })
            }
            Some(TipoToken::Cadena(s)) => {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                self.pos += 1;
                Ok(Expresion::Literal {
                    valor: Valor::Texto(s.clone()),
                    linea,
                    columna,
                })
            }
            Some(TipoToken::CorcheteIzq) => {
                let t_corchete = &self.tokens[self.pos];
                let (linea, columna) = (t_corchete.linea, t_corchete.columna);
                self.pos += 1; // '['
                self.omitir_finales_de_linea();
                let mut elementos = Vec::new();
                if matches!(self.actual(), Some(TipoToken::CorcheteDer)) {
                    self.pos += 1; // ']'
                } else {
                    loop {
                        let elem = self.expresion()?;
                        elementos.push(elem);
                        self.omitir_finales_de_linea();
                        if matches!(self.actual(), Some(TipoToken::Coma)) {
                            self.pos += 1;
                            self.omitir_finales_de_linea();
                            continue;
                        }
                        if matches!(self.actual(), Some(TipoToken::CorcheteDer)) {
                            self.pos += 1;
                            break;
                        }
                        return Err(self.error("se esperaba ',' o ']'"));
                    }
                }
                Ok(Expresion::ArregloLiteral {
                    elementos,
                    linea,
                    columna,
                })
            }
            Some(TipoToken::Identificador(nombre)) => {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                let nombre = nombre.clone();
                self.pos += 1;
                // Llamada a función: identificador seguido de '('
                if matches!(self.actual(), Some(TipoToken::ParentesisIzq)) {
                    self.pos += 1;
                    let mut args = Vec::new();
                    self.omitir_finales_de_linea();
                    if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                        self.pos += 1;
                    } else {
                        loop {
                            let a = self.expresion()?;
                            args.push(a);
                            self.omitir_finales_de_linea();
                            if matches!(self.actual(), Some(TipoToken::Coma)) {
                                self.pos += 1;
                                self.omitir_finales_de_linea();
                                continue;
                            }
                            if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                                self.pos += 1;
                                break;
                            }
                            return Err(self.error("se esperaba ',' o ')'"));
                        }
                    }
                    return Ok(Expresion::Llamada {
                        nombre,
                        argumentos: args,
                        linea,
                        columna,
                    });
                }
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

    /// Parsea una expresión primaria y maneja indexación posterior `[indice]`.
    fn primaria(&mut self) -> Result<Expresion, ErrorSintactico> {
        let mut expr = self.primaria_sin_indice()?;
        // La indexación es un sufijo de la misma línea: un salto de línea termina
        // la expresión (p. ej. `a` seguido de `[0]` en otra línea son dos sentencias).
        while matches!(self.actual(), Some(TipoToken::CorcheteIzq)) {
            let t_corchete = &self.tokens[self.pos];
            let (linea, columna) = (t_corchete.linea, t_corchete.columna);
            self.pos += 1; // '['
            self.omitir_finales_de_linea();
            let indice = self.expresion()?;
            self.omitir_finales_de_linea();
            if !matches!(self.actual(), Some(TipoToken::CorcheteDer)) {
                return Err(self.error("se esperaba ']'"));
            }
            self.pos += 1; // ']'
            expr = Expresion::Indice {
                objeto: Box::new(expr),
                indice: Box::new(indice),
                linea,
                columna,
            };
        }
        // Acceso a miembro: `objeto.miembro` (ADR-0017 D8). Integración en Fase 3.
        while matches!(self.actual(), Some(TipoToken::Punto)) {
            let t_punto = &self.tokens[self.pos];
            let (linea, columna) = (t_punto.linea, t_punto.columna);
            self.pos += 1; // '.'
            let miembro = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let m = n.clone();
                    self.pos += 1;
                    m
                }
                _ => return Err(self.error("se esperaba un nombre tras '.'")),
            };
            expr = Expresion::Acceso {
                objeto: Box::new(expr),
                miembro,
                linea,
                columna,
            };
        }
        // Llamada indirecta (postfix): `expr(...)` donde expr es un acceso a miembro.
        // Solo se permite cuando expr es un Acceso (alias.func(args)).
        while matches!(self.actual(), Some(TipoToken::ParentesisIzq)) {
            let t = &self.tokens[self.pos];
            let (linea, columna) = (t.linea, t.columna);
            // Solo permitir si expr es un Acceso.
            if !matches!(expr, Expresion::Acceso { .. }) {
                return Err(self.error("llamada indirecta solo válida sobre acceso a miembro"));
            }
            self.pos += 1; // '('
            self.omitir_finales_de_linea();
            let mut args = Vec::new();
            if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                self.pos += 1;
            } else {
                loop {
                    let a = self.expresion()?;
                    args.push(a);
                    self.omitir_finales_de_linea();
                    if matches!(self.actual(), Some(TipoToken::Coma)) {
                        self.pos += 1;
                        self.omitir_finales_de_linea();
                        continue;
                    }
                    if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                        self.pos += 1;
                        break;
                    }
                    return Err(self.error("se esperaba ',' o ')'"));
                }
            }
            expr = Expresion::LlamadaIndirecta {
                callee: Box::new(expr),
                argumentos: args,
                linea,
                columna,
            };
        }
        Ok(expr)
    }
}

/// Obtiene la ubicación (linea, columna) de una expresión (para rangos).
fn ubicacion_expr(e: &Expresion) -> (usize, usize) {
    match e {
        Expresion::Literal { linea, columna, .. }
        | Expresion::Variable { linea, columna, .. }
        | Expresion::Llamada { linea, columna, .. }
        | Expresion::ArregloLiteral { linea, columna, .. }
        | Expresion::Indice { linea, columna, .. }
        | Expresion::Acceso { linea, columna, .. }
        | Expresion::LlamadaIndirecta { linea, columna, .. }
        | Expresion::Rango { linea, columna, .. } => (*linea, *columna),
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
        | Expresion::O(a, _) => ubicacion_expr(a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Lexer;
    use crate::evaluador::evaluar;

    /// Tokeniza, parsea y evalúa; devuelve el valor o el error sintáctico.
    fn valor(fuente: &str) -> Result<Valor, ErrorSintactico> {
        let tokens = Lexer::nuevo(fuente)
            .tokenizar()
            .expect("léxico válido en test");
        Parser::nuevo(tokens)
            .parsear_expresion()
            .map(|e| evaluar(&e).expect("evaluación sin error en tests sin división entre cero"))
    }

    fn error_sintactico(fuente: &str) -> ErrorSintactico {
        let tokens = Lexer::nuevo(fuente)
            .tokenizar()
            .expect("léxico válido en test");
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
        let tokens = Lexer::nuevo(fuente)
            .tokenizar()
            .expect("léxico válido en test");
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
        let tokens = Lexer::nuevo(fuente)
            .tokenizar()
            .expect("léxico válido en test");
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
        assert!(e.mensaje.contains("encadenadas"), "mensaje: {}", e.mensaje);
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

    #[test]
    fn declaraciones_tipadas_var_dato_const() {
        let tokens = Lexer::nuevo("var x: entero = 42").tokenizar().unwrap();
        let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
        assert_eq!(ss.len(), 1);
        match &ss[0] {
            Sentencia::Declaracion {
                nombre,
                tipo_anotacion,
                mutable,
                es_const,
                ..
            } => {
                assert_eq!(nombre, "x");
                assert_eq!(*tipo_anotacion, Some(TipoAnotacion::Entero));
                assert!(*mutable);
                assert!(!*es_const);
            }
            _ => panic!("esperaba declaración"),
        }

        let tokens = Lexer::nuevo("dato valor: decimal = 3.14")
            .tokenizar()
            .unwrap();
        let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
        match &ss[0] {
            Sentencia::Declaracion {
                nombre,
                tipo_anotacion,
                mutable,
                es_const,
                ..
            } => {
                assert_eq!(nombre, "valor");
                assert_eq!(*tipo_anotacion, Some(TipoAnotacion::Decimal));
                assert!(!*mutable);
                assert!(!*es_const);
            }
            _ => panic!("esperaba declaración"),
        }

        let tokens = Lexer::nuevo("const PI: real = 3.14159e0")
            .tokenizar()
            .unwrap();
        let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
        match &ss[0] {
            Sentencia::Declaracion {
                nombre,
                tipo_anotacion,
                mutable,
                es_const,
                ..
            } => {
                assert_eq!(nombre, "PI");
                assert_eq!(*tipo_anotacion, Some(TipoAnotacion::Real));
                assert!(!*mutable);
                assert!(*es_const);
            }
            _ => panic!("esperaba declaración"),
        }

        // sea legacy sigue funcionando
        let tokens = Lexer::nuevo("sea z = 10").tokenizar().unwrap();
        let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
        match &ss[0] {
            Sentencia::Declaracion {
                nombre,
                tipo_anotacion,
                mutable,
                es_const,
                ..
            } => {
                assert_eq!(nombre, "z");
                assert_eq!(*tipo_anotacion, None);
                assert!(*mutable);
                assert!(!*es_const);
            }
            _ => panic!("esperaba declaración"),
        }
    }
}
