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
        /// Argumentos de tipo explícitos (turbofish, G2). Vacío si no se especifican.
        argumentos_tipo: Vec<TipoAnotacion>,
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
    /// Función anónima (lambda) (Fase 2).
    Lambda {
        parametros: Vec<(String, Option<TipoAnotacion>)>,
        tipo_retorno: Option<TipoAnotacion>,
        cuerpo: Vec<Sentencia>,
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
    /// Construcción de estructura nominal `Tipo { campo: expr }` (Fase 08 S8.1).
    ConstruccionEstructura {
        nombre: String,
        campos: Vec<(String, Expresion)>,
        linea: usize,
        columna: usize,
    },
    /// Construcción de variante enum nominal `Tipo.Variante` o `Tipo.Variante(args)` (Fase 08 S8.2).
    ConstruccionVariante {
        tipo: String,
        variante: String,
        argumentos: Vec<Expresion>,
        linea: usize,
        columna: usize,
    },
    /// Expresión `coincidir` (Fase 08 S8.3).
    Coincidir {
        scrutinee: Box<Expresion>,
        brazos: Vec<BrazoCoincidir>,
        linea: usize,
        columna: usize,
    },
}

/// Patrón de `coincidir` (Fase 08 S8.3, v1).
#[derive(Debug, Clone, PartialEq)]
pub enum Patron {
    Wildcard {
        linea: usize,
        columna: usize,
    },
    Binding {
        nombre: String,
        linea: usize,
        columna: usize,
    },
    Literal {
        valor: Valor,
        linea: usize,
        columna: usize,
    },
    Variante {
        tipo: String,
        variante: String,
        campos: Vec<Patron>,
        linea: usize,
        columna: usize,
    },
    Estructura {
        tipo: String,
        campos: Vec<(String, Patron)>,
        linea: usize,
        columna: usize,
    },
}

/// Brazo de `coincidir` (Fase 08 S8.3).
#[derive(Debug, Clone, PartialEq)]
pub struct BrazoCoincidir {
    pub patron: Patron,
    pub cuerpo: Vec<Sentencia>,
    pub linea: usize,
    pub columna: usize,
}

/// Definición de campo en una estructura nominal (Fase 08 S8.1).
#[derive(Debug, Clone, PartialEq)]
pub struct CampoEstructura {
    pub nombre: String,
    pub tipo: TipoAnotacion,
    pub publico: bool,
    pub linea: usize,
    pub columna: usize,
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
    /// `objeto.campo = expresión` asignación de campo nominal (Fase 08 S8.1, C2).
    AsignacionCampo {
        objeto: Box<Expresion>,
        campo: String,
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
        /// Parámetros de tipo genéricos sintácticos (G2). Vacío si no se declaran.
        parametros_tipo: Vec<String>,
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
    /// Declaración de estructura nominal (Fase 08 S8.1).
    Estructura {
        nombre: String,
        campos: Vec<CampoEstructura>,
        linea: usize,
        columna: usize,
        publico: bool,
    },
    /// Declaración de enumeración nominal (Fase 08 S8.2).
    Enumeracion {
        nombre: String,
        variantes: Vec<VarianteEnumeracion>,
        linea: usize,
        columna: usize,
        publico: bool,
    },
    /// Declaración de rasgo (Fase 08 S8.4b).
    Rasgo {
        nombre: String,
        metodos: Vec<FirmaMetodo>,
        linea: usize,
        columna: usize,
        publico: bool,
    },
    /// Implementación inherente `implementar Tipo { ... }` (Fase 08 S8.4a).
    ImplementacionInherente {
        tipo: String,
        metodos: Vec<Metodo>,
        linea: usize,
        columna: usize,
    },
    /// Implementación de rasgo `implementar Rasgo para Tipo { ... }` (Fase 08 S8.4b).
    ImplementacionRasgo {
        rasgo: String,
        tipo: String,
        metodos: Vec<Metodo>,
        linea: usize,
        columna: usize,
    },
}

/// Variante de enumeración nominal (Fase 08 S8.2).
#[derive(Debug, Clone, PartialEq)]
pub struct VarianteEnumeracion {
    pub nombre: String,
    /// Payload posicional. Vacío para variante unitaria.
    pub payload: Vec<TipoAnotacion>,
    pub linea: usize,
    pub columna: usize,
}

/// Receptor de método (Fase 08 S8.4).
#[derive(Debug, Clone, PartialEq)]
pub enum ReceptorMetodo {
    Ninguno,
    Yo { mutable: bool },
}

/// Firma de método en rasgo (Fase 08 S8.4b).
#[derive(Debug, Clone, PartialEq)]
pub struct FirmaMetodo {
    pub nombre: String,
    pub receptor: ReceptorMetodo,
    pub parametros: Vec<(String, Option<TipoAnotacion>)>,
    pub tipo_retorno: Option<TipoAnotacion>,
    pub linea: usize,
    pub columna: usize,
}

/// Método en implementación (Fase 08 S8.4).
#[derive(Debug, Clone, PartialEq)]
pub struct Metodo {
    pub nombre: String,
    pub receptor: ReceptorMetodo,
    pub parametros: Vec<(String, Option<TipoAnotacion>)>,
    pub tipo_retorno: Option<TipoAnotacion>,
    pub cuerpo: Vec<Sentencia>,
    pub linea: usize,
    pub columna: usize,
    pub publico: bool,
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
    /// Referencia sintáctica a un parámetro de tipo (G2, Fase 07).
    /// No implica validación semántica en G2.
    Parametro(String),
    /// Tipo nominal declarado por el usuario (Fase 08 S8.1).
    Nominal(String),
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
            TipoAnotacion::Parametro(n) => n.clone(),
            TipoAnotacion::Nominal(n) => n.clone(),
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
    /// Parámetros de tipo en alcance sintáctico (G2.1). Pila por función genérica.
    parametros_tipo_alcance: Vec<std::collections::HashSet<String>>,
    /// Archivo de origen, si está disponible (ADR-0017 D5).
    archivo: Option<String>,
    /// C1 (Fase 08 R3): bloquea literal struct directo `Ident { ... }` en cabeza de control.
    bloquear_literal_struct_cabeza: bool,
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
            parametros_tipo_alcance: Vec::new(),
            archivo,
            bloquear_literal_struct_cabeza: false,
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
                        | TipoToken::Estructura
                        | TipoToken::Enumeracion
                        | TipoToken::Rasgo
                )
            )
        {
            return Err(self.error(
                "'publico' solo puede preceder a una declaración, función, estructura, enumeración o rasgo",
            ));
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
        } else if matches!(self.actual(), Some(TipoToken::Estructura)) {
            let st = self.sentencia_estructura(publico)?;
            self.terminar_sentencia()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Enumeracion)) {
            let st = self.sentencia_enumeracion(publico)?;
            self.terminar_sentencia()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Rasgo)) {
            let st = self.sentencia_rasgo(publico)?;
            self.terminar_sentencia()?;
            return Ok(st);
        } else if matches!(self.actual(), Some(TipoToken::Implementar)) {
            if publico {
                return Err(self.error("'publico' no puede preceder a 'implementar'"));
            }
            let st = self.sentencia_implementar()?;
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
                | Some(TipoToken::Yo)
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
                        // C2: solo se admite id[indice] = expr en v1.
                        if !matches!(objeto.as_ref(), Expresion::Variable { .. }) {
                            return Err(self.error(
                                "forma de lvalue no soportada en Fase 08 v1; se admite solo `id[indice] = expr`",
                            ));
                        }
                        return Ok(Sentencia::AsignacionIndice {
                            objeto,
                            indice,
                            valor,
                            linea,
                            columna,
                        });
                    }
                    Expresion::Acceso {
                        objeto,
                        miembro,
                        linea,
                        columna,
                    } => {
                        // C2: solo se admite id.campo = expr en v1.
                        if !matches!(objeto.as_ref(), Expresion::Variable { .. }) {
                            return Err(self.error(
                                "forma de lvalue no soportada en Fase 08 v1; se admite solo `id.campo = expr`",
                            ));
                        }
                        return Ok(Sentencia::AsignacionCampo {
                            objeto,
                            campo: miembro,
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

    fn tipo_param_en_alcance(&self, nombre: &str) -> bool {
        self.parametros_tipo_alcance
            .last()
            .is_some_and(|scope| scope.contains(nombre))
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
            Some(TipoToken::Identificador(t)) => {
                let nombre = t.clone();
                self.pos += 1;
                if self.tipo_param_en_alcance(&nombre) {
                    Ok(TipoAnotacion::Parametro(nombre))
                } else {
                    Ok(TipoAnotacion::Nominal(nombre))
                }
            }
            Some(TipoToken::Nulo) => {
                self.pos += 1;
                Ok(TipoAnotacion::Nulo)
            }
            _ => Err(self.error(
                "tipo desconocido; se esperaba: entero, decimal, real, logico, texto, nulo, arreglo<T, N>, lista<T>",
            )),
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
            let tipo_elem = self.parsear_tipo_anotacion_cuerpo()?; // permite anidados (G2)
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
            let tipo_elem = self.parsear_tipo_anotacion_cuerpo()?; // permite anidados (G2)
            self.omitir_finales_de_linea();
            if !matches!(self.actual(), Some(TipoToken::Mayor)) {
                return Err(self.error("se esperaba '>' para cerrar el tipo de lista"));
            }
            self.pos += 1;
            return Ok(TipoAnotacion::Lista(Box::new(tipo_elem)));
        }
        self.parsear_tipo_anotacion_simple()
    }

    fn es_nombre_tipo_param_prohibido(nombre: &str) -> bool {
        matches!(nombre, "entero" | "decimal" | "real" | "logico" | "texto")
    }

    /// Parsea la lista de parámetros de tipo de una declaración: `<T, U>`.
    /// Requiere que el token actual sea `<`.
    fn parsear_parametros_tipo_decl(&mut self) -> Result<Vec<String>, ErrorSintactico> {
        if !matches!(self.actual(), Some(TipoToken::Menor)) {
            return Ok(Vec::new());
        }
        self.pos += 1; // '<'
        self.omitir_finales_de_linea();
        let mut params: Vec<String> = Vec::new();
        let mut vistos: std::collections::HashSet<String> = std::collections::HashSet::new();
        if matches!(self.actual(), Some(TipoToken::Mayor)) {
            return Err(self.error("se esperaba al menos un parámetro de tipo"));
        }
        loop {
            self.omitir_finales_de_linea();
            let nombre = match self.actual() {
                Some(TipoToken::Identificador(n)) => n.clone(),
                Some(
                    TipoToken::Sea
                    | TipoToken::Var
                    | TipoToken::Dato
                    | TipoToken::Const
                    | TipoToken::Arreglo
                    | TipoToken::Lista
                    | TipoToken::Funcion
                    | TipoToken::Retornar
                    | TipoToken::Mostrar
                    | TipoToken::MostrarSinSalto
                    | TipoToken::Si
                    | TipoToken::Sino
                    | TipoToken::Mientras
                    | TipoToken::Para
                    | TipoToken::Cada
                    | TipoToken::En
                    | TipoToken::Romper
                    | TipoToken::Continuar
                    | TipoToken::Intentar
                    | TipoToken::Rescatar
                    | TipoToken::Importar
                    | TipoToken::Exportar
                    | TipoToken::Usar
                    | TipoToken::Como
                    | TipoToken::Publico
                    | TipoToken::Y
                    | TipoToken::O
                    | TipoToken::No
                    | TipoToken::Verdadero
                    | TipoToken::Falso
                    | TipoToken::Nulo,
                ) => {
                    let lex = self.tokens[self.pos].lexema.clone();
                    return Err(self.error(format!(
                        "el nombre '{lex}' no puede usarse como parámetro de tipo"
                    )));
                }
                _ => {
                    return Err(self.error("se esperaba un parámetro de tipo"));
                }
            };
            self.pos += 1;
            if Self::es_nombre_tipo_param_prohibido(&nombre) {
                return Err(self.error(format!(
                    "el nombre '{nombre}' no puede usarse como parámetro de tipo"
                )));
            }
            if !vistos.insert(nombre.clone()) {
                return Err(self.error(format!("el parámetro de tipo '{nombre}' está duplicado")));
            }
            params.push(nombre);
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::Coma)) {
                self.pos += 1;
                self.omitir_finales_de_linea();
                if matches!(self.actual(), Some(TipoToken::Mayor)) {
                    self.pos += 1; // coma final
                    return Ok(params);
                }
                continue;
            }
            if matches!(self.actual(), Some(TipoToken::Mayor)) {
                self.pos += 1;
                return Ok(params);
            }
            return Err(self.error("se esperaba ',' o '>' en la lista de parámetros de tipo"));
        }
    }

    /// Si hay `:: < ... >` en posición de turbofish (dos DosPuntos seguidos de `<`), lo parsea.
    /// Retorna argumentos de tipo y consume tokens; si no hay turbofish, retorna Vec vacío sin consumir.
    fn intentar_parsear_argumentos_tipo_turbofish(
        &mut self,
    ) -> Result<Option<Vec<TipoAnotacion>>, ErrorSintactico> {
        let es_turbofish = matches!(self.actual(), Some(TipoToken::DosPuntos))
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.tipo),
                Some(TipoToken::DosPuntos)
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|t| &t.tipo),
                Some(TipoToken::Menor)
            );
        if !es_turbofish {
            return Ok(None);
        }
        self.pos += 2; // '::'
        self.pos += 1; // '<'
        self.omitir_finales_de_linea();
        if matches!(self.actual(), Some(TipoToken::Mayor)) {
            return Err(self.error("se esperaba al menos un argumento de tipo"));
        }
        let mut args: Vec<TipoAnotacion> = Vec::new();
        loop {
            self.omitir_finales_de_linea();
            let ty = self
                .parsear_tipo_anotacion_cuerpo()
                .map_err(|_| self.error("argumento de tipo mal formado"))?;
            args.push(ty);
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::Coma)) {
                self.pos += 1;
                self.omitir_finales_de_linea();
                if matches!(self.actual(), Some(TipoToken::Mayor)) {
                    self.pos += 1; // coma final
                    return Ok(Some(args));
                }
                continue;
            }
            if matches!(self.actual(), Some(TipoToken::Mayor)) {
                self.pos += 1;
                return Ok(Some(args));
            }
            return Err(self.error("se esperaba ',' o '>' en la lista de argumentos de tipo"));
        }
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
        // Las funciones anidadas están permitidas para closures (Fase 2).
        let t = &self.tokens[self.pos];
        let (_linea_kw, _columna_kw) = (t.linea, t.columna);
        self.pos += 1; // 'funcion'
        let prev_en_funcion = self.en_funcion;
        self.en_funcion = true;
        let (nombre, linea, columna) = match self.actual() {
            Some(TipoToken::Identificador(n)) => {
                let tn = &self.tokens[self.pos];
                let (ln, cn) = (tn.linea, tn.columna);
                let nom = n.clone();
                self.pos += 1;
                (nom, ln, cn)
            }
            _ => return Err(self.error("se esperaba el nombre de la función")),
        };
        let parametros_tipo = if matches!(self.actual(), Some(TipoToken::Menor)) {
            self.parsear_parametros_tipo_decl()?
        } else {
            Vec::new()
        };
        // G2.2 (3.3): cada declaración de función tiene su propio conjunto de
        // parámetros de tipo (sin herencia implícita de funciones exteriores).
        let set: std::collections::HashSet<String> = parametros_tipo.iter().cloned().collect();
        self.parametros_tipo_alcance.push(set);
        let resultado = (|| {
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
            Ok((parametros, tipo_retorno, cuerpo))
        })();
        self.parametros_tipo_alcance.pop();
        let (parametros, tipo_retorno, cuerpo) = resultado?;
        self.en_funcion = prev_en_funcion;
        Ok(Sentencia::Funcion {
            nombre,
            parametros_tipo,
            parametros,
            tipo_retorno,
            cuerpo,
            linea,
            columna,
            publico,
        })
    }

    fn sentencia_estructura(&mut self, publico: bool) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'estructura'

        let nombre = match self.actual() {
            Some(TipoToken::Identificador(n)) => {
                let n = n.clone();
                self.pos += 1;
                n
            }
            _ => {
                return Err(self.error("se esperaba un nombre de estructura tras 'estructura'"));
            }
        };

        self.omitir_finales_de_linea();
        if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            return Err(self.error(format!(
                "se esperaba '{{' tras el nombre de estructura '{nombre}'"
            )));
        }
        self.pos += 1; // '{'
        self.omitir_finales_de_linea();

        let mut campos = Vec::new();
        let mut nombres_vistos = std::collections::HashSet::new();
        while !matches!(
            self.actual(),
            Some(TipoToken::LlaveDer) | Some(TipoToken::Fin) | None
        ) {
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                break;
            }

            let mut campo_publico = false;
            if matches!(self.actual(), Some(TipoToken::Publico)) {
                campo_publico = true;
                self.pos += 1;
                self.omitir_finales_de_linea();
            }

            let (c_linea, c_columna) = self
                .tokens
                .get(self.pos)
                .map(|tk| (tk.linea, tk.columna))
                .unwrap_or((linea, columna));
            let nombre_campo = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let n = n.clone();
                    self.pos += 1;
                    n
                }
                _ => {
                    return Err(self.error("se esperaba un nombre de campo en la estructura"));
                }
            };

            if !nombres_vistos.insert(nombre_campo.clone()) {
                return Err(ErrorSintactico {
                    mensaje: format!(
                        "el campo '{nombre_campo}' está duplicado en la estructura '{nombre}'"
                    ),
                    linea: c_linea,
                    columna: c_columna,
                    archivo: self.archivo.clone(),
                });
            }

            if !matches!(self.actual(), Some(TipoToken::DosPuntos)) {
                return Err(self.error(format!(
                    "se esperaba ':' tras el nombre del campo '{nombre_campo}'"
                )));
            }
            let tipo = self.parsear_tipo_anotacion()?;
            campos.push(CampoEstructura {
                nombre: nombre_campo,
                tipo,
                publico: campo_publico,
                linea: c_linea,
                columna: c_columna,
            });

            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::Coma)) {
                self.pos += 1;
                self.omitir_finales_de_linea();
                continue;
            }
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                break;
            }
            // permitir salto de línea como separador
            if matches!(self.actual(), Some(TipoToken::FinDeLinea)) {
                self.omitir_finales_de_linea();
                continue;
            }
            return Err(
                self.error("se esperaba ',', nueva línea o '}' tras un campo de estructura")
            );
        }

        if !matches!(self.actual(), Some(TipoToken::LlaveDer)) {
            return Err(self.error(format!(
                "se esperaba '}}' para cerrar la estructura '{nombre}'"
            )));
        }
        self.pos += 1; // '}'

        Ok(Sentencia::Estructura {
            nombre,
            campos,
            linea,
            columna,
            publico,
        })
    }

    fn sentencia_enumeracion(&mut self, publico: bool) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'enumeracion'

        let nombre = match self.actual() {
            Some(TipoToken::Identificador(n)) => {
                let n = n.clone();
                self.pos += 1;
                n
            }
            _ => {
                return Err(self.error("se esperaba un nombre de enumeración tras 'enumeracion'"));
            }
        };

        self.omitir_finales_de_linea();
        if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            return Err(self.error(format!(
                "se esperaba '{{' tras el nombre de enumeración '{nombre}'"
            )));
        }
        self.pos += 1; // '{'
        self.omitir_finales_de_linea();

        let mut variantes = Vec::new();
        let mut nombres_vistos = std::collections::HashSet::new();
        while !matches!(
            self.actual(),
            Some(TipoToken::LlaveDer) | Some(TipoToken::Fin) | None
        ) {
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                break;
            }
            let (v_linea, v_columna) = self
                .tokens
                .get(self.pos)
                .map(|tk| (tk.linea, tk.columna))
                .unwrap_or((linea, columna));
            let nombre_variante = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let n = n.clone();
                    self.pos += 1;
                    n
                }
                _ => {
                    return Err(self.error("se esperaba un nombre de variante en la enumeración"));
                }
            };
            nombres_vistos.insert(nombre_variante.clone());

            let mut payload = Vec::new();
            if matches!(self.actual(), Some(TipoToken::ParentesisIzq)) {
                self.pos += 1; // '('
                self.omitir_finales_de_linea();
                if !matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                    loop {
                        let tipo = self.parsear_tipo_anotacion_cuerpo()?;
                        payload.push(tipo);
                        self.omitir_finales_de_linea();
                        if matches!(self.actual(), Some(TipoToken::Coma)) {
                            self.pos += 1;
                            self.omitir_finales_de_linea();
                            if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
                if !matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                    return Err(self.error(format!(
                        "se esperaba ')' tras el payload de la variante '{nombre_variante}'"
                    )));
                }
                self.pos += 1; // ')'
            }

            variantes.push(VarianteEnumeracion {
                nombre: nombre_variante,
                payload,
                linea: v_linea,
                columna: v_columna,
            });

            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::Coma)) {
                self.pos += 1;
                self.omitir_finales_de_linea();
                continue;
            }
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                break;
            }
            if matches!(self.actual(), Some(TipoToken::FinDeLinea)) {
                self.omitir_finales_de_linea();
                continue;
            }
            return Err(
                self.error("se esperaba ',', nueva línea o '}' tras una variante de enumeración")
            );
        }

        if !matches!(self.actual(), Some(TipoToken::LlaveDer)) {
            return Err(self.error(format!(
                "se esperaba '}}' para cerrar la enumeración '{nombre}'"
            )));
        }
        self.pos += 1; // '}'

        Ok(Sentencia::Enumeracion {
            nombre,
            variantes,
            linea,
            columna,
            publico,
        })
    }

    fn sentencia_rasgo(&mut self, publico: bool) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'rasgo'
        let nombre = match self.actual() {
            Some(TipoToken::Identificador(n)) => {
                let n = n.clone();
                self.pos += 1;
                n
            }
            _ => return Err(self.error("se esperaba el nombre del rasgo")),
        };
        self.omitir_finales_de_linea();
        if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            return Err(self.error(format!(
                "se esperaba '{{' tras el nombre del rasgo '{nombre}'"
            )));
        }
        self.pos += 1; // '{'
        self.omitir_finales_de_linea();
        let mut metodos = Vec::new();
        while !matches!(
            self.actual(),
            Some(TipoToken::LlaveDer) | Some(TipoToken::Fin) | None
        ) {
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                break;
            }
            if !matches!(self.actual(), Some(TipoToken::Funcion)) {
                return Err(self.error("se esperaba 'funcion' en declaración de rasgo"));
            }
            let mt = &self.tokens[self.pos];
            let (ml, mc) = (mt.linea, mt.columna);
            self.pos += 1; // 'funcion'
            let mnombre = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let n = n.clone();
                    self.pos += 1;
                    n
                }
                _ => return Err(self.error("se esperaba el nombre del método en rasgo")),
            };
            let (receptor, parametros) = self.parsear_params_metodo(&mnombre)?;
            let tipo_retorno = if matches!(self.actual(), Some(TipoToken::Flecha)) {
                self.pos += 1;
                Some(self.parsear_tipo_anotacion_cuerpo()?)
            } else {
                None
            };
            metodos.push(FirmaMetodo {
                nombre: mnombre,
                receptor,
                parametros,
                tipo_retorno,
                linea: ml,
                columna: mc,
            });
            self.omitir_finales_de_linea();
            // permitir fin de línea o coma opcional
            if matches!(self.actual(), Some(TipoToken::Coma)) {
                self.pos += 1;
                self.omitir_finales_de_linea();
            }
        }
        if !matches!(self.actual(), Some(TipoToken::LlaveDer)) {
            return Err(self.error(format!("se esperaba '}}' para cerrar el rasgo '{nombre}'")));
        }
        self.pos += 1;
        Ok(Sentencia::Rasgo {
            nombre,
            metodos,
            linea,
            columna,
            publico,
        })
    }

    fn sentencia_implementar(&mut self) -> Result<Sentencia, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'implementar'
        let nombre1 = match self.actual() {
            Some(TipoToken::Identificador(n)) => {
                let n = n.clone();
                self.pos += 1;
                n
            }
            _ => return Err(self.error("se esperaba nombre de tipo o rasgo tras 'implementar'")),
        };
        self.omitir_finales_de_linea();
        // implementar Rasgo para Tipo { ... } or implementar Tipo { ... }
        if matches!(self.actual(), Some(TipoToken::Para)) {
            self.pos += 1; // 'para'
            let tipo = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let n = n.clone();
                    self.pos += 1;
                    n
                }
                _ => return Err(self.error("se esperaba nombre de tipo tras 'para'")),
            };
            let metodos = self.parsear_metodos_impl()?;
            Ok(Sentencia::ImplementacionRasgo {
                rasgo: nombre1,
                tipo,
                metodos,
                linea,
                columna,
            })
        } else {
            let metodos = self.parsear_metodos_impl()?;
            Ok(Sentencia::ImplementacionInherente {
                tipo: nombre1,
                metodos,
                linea,
                columna,
            })
        }
    }

    fn parsear_metodos_impl(&mut self) -> Result<Vec<Metodo>, ErrorSintactico> {
        self.omitir_finales_de_linea();
        if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            return Err(self.error("se esperaba '{' en implementación"));
        }
        self.pos += 1;
        self.omitir_finales_de_linea();
        let mut metodos = Vec::new();
        while !matches!(
            self.actual(),
            Some(TipoToken::LlaveDer) | Some(TipoToken::Fin) | None
        ) {
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                break;
            }
            let mut publico = false;
            if matches!(self.actual(), Some(TipoToken::Publico)) {
                publico = true;
                self.pos += 1;
                self.omitir_finales_de_linea();
            }
            if !matches!(self.actual(), Some(TipoToken::Funcion)) {
                return Err(self.error("se esperaba 'funcion' en implementación"));
            }
            let mt = &self.tokens[self.pos];
            let (ml, mc) = (mt.linea, mt.columna);
            self.pos += 1; // 'funcion'
            let mnombre = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let n = n.clone();
                    self.pos += 1;
                    n
                }
                _ => return Err(self.error("se esperaba el nombre del método")),
            };
            let (receptor, parametros) = self.parsear_params_metodo(&mnombre)?;
            let tipo_retorno = if matches!(self.actual(), Some(TipoToken::Flecha)) {
                self.pos += 1;
                Some(self.parsear_tipo_anotacion_cuerpo()?)
            } else {
                None
            };
            self.omitir_finales_de_linea();
            if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
                return Err(self.error(format!(
                    "se esperaba '{{' para el cuerpo del método '{mnombre}'"
                )));
            }
            let prev_en_funcion = self.en_funcion;
            self.en_funcion = true;
            let cuerpo = self.bloque()?;
            self.en_funcion = prev_en_funcion;
            metodos.push(Metodo {
                nombre: mnombre,
                receptor,
                parametros,
                tipo_retorno,
                cuerpo,
                linea: ml,
                columna: mc,
                publico,
            });
            self.omitir_finales_de_linea();
        }
        if !matches!(self.actual(), Some(TipoToken::LlaveDer)) {
            return Err(self.error("se esperaba '}' para cerrar la implementación"));
        }
        self.pos += 1;
        Ok(metodos)
    }

    #[allow(clippy::type_complexity)]
    fn parsear_params_metodo(
        &mut self,
        _mnombre: &str,
    ) -> Result<(ReceptorMetodo, Vec<(String, Option<TipoAnotacion>)>), ErrorSintactico> {
        if !matches!(self.actual(), Some(TipoToken::ParentesisIzq)) {
            return Err(self.error("se esperaba '(' tras el nombre del método"));
        }
        self.pos += 1; // '('
        let mut receptor = ReceptorMetodo::Ninguno;
        let mut parametros: Vec<(String, Option<TipoAnotacion>)> = Vec::new();
        let mut primer_tiene_tipo: Option<bool> = None;
        let mut primero = true;
        loop {
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                self.pos += 1;
                break;
            }
            // receptor yo / yo var
            if primero && matches!(self.actual(), Some(TipoToken::Yo)) {
                self.pos += 1; // 'yo'
                let mutable = if matches!(self.actual(), Some(TipoToken::Var)) {
                    self.pos += 1;
                    true
                } else {
                    false
                };
                receptor = ReceptorMetodo::Yo { mutable };
                primero = false;
                self.omitir_finales_de_linea();
                if matches!(self.actual(), Some(TipoToken::Coma)) {
                    self.pos += 1;
                    continue;
                }
                if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                    self.pos += 1;
                    break;
                }
                return Err(self.error("se esperaba ',' o ')' tras receptor 'yo'"));
            }
            primero = false;
            let param_nombre = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let n = n.clone();
                    self.pos += 1;
                    n
                }
                Some(TipoToken::Yo) => {
                    return Err(self.error("'yo' solo puede ser el primer parámetro del método"));
                }
                _ => return Err(self.error("se esperaba un parámetro")),
            };
            let tipo_param = if matches!(self.actual(), Some(TipoToken::DosPuntos)) {
                let tipo = self.parsear_tipo_anotacion()?;
                Some(tipo)
            } else {
                None
            };
            let este_tiene_tipo = tipo_param.is_some();
            match primer_tiene_tipo {
                None => primer_tiene_tipo = Some(este_tiene_tipo),
                Some(pt) if pt != este_tiene_tipo => {
                    return Err(self.error("la firma del método mezcla parámetros con y sin tipo; anota todos o ninguno"));
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
        Ok((receptor, parametros))
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
        let prev = self.bloquear_literal_struct_cabeza;
        self.bloquear_literal_struct_cabeza = true;
        let r = (|| {
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
        })();
        self.bloquear_literal_struct_cabeza = prev;
        r
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
        let condicion = self.expresion_cabeza_control()?;
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
        let condicion = self.expresion_cabeza_control()?;
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

    /// C1 (R3): expresión de cabeza de control. Prohíbe literal struct directo `Ident { ... }`.
    fn expresion_cabeza_control(&mut self) -> Result<Expresion, ErrorSintactico> {
        let prev = self.bloquear_literal_struct_cabeza;
        self.bloquear_literal_struct_cabeza = true;
        let r = self.expresion();
        self.bloquear_literal_struct_cabeza = prev;
        r
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

    fn parsear_coincidir(&mut self) -> Result<Expresion, ErrorSintactico> {
        let t = &self.tokens[self.pos];
        let (linea, columna) = (t.linea, t.columna);
        self.pos += 1; // 'coincidir'
        let prev = self.bloquear_literal_struct_cabeza;
        self.bloquear_literal_struct_cabeza = true;
        let scrutinee = self.expresion()?;
        self.bloquear_literal_struct_cabeza = prev;
        self.omitir_finales_de_linea();
        if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            return Err(self.error("se esperaba '{' tras la expresión de 'coincidir'"));
        }
        self.pos += 1; // '{'
        self.omitir_finales_de_linea();
        let mut brazos = Vec::new();
        while !matches!(
            self.actual(),
            Some(TipoToken::LlaveDer) | Some(TipoToken::Fin) | None
        ) {
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                break;
            }
            let patron = self.parsear_patron()?;
            self.omitir_finales_de_linea();
            if !matches!(self.actual(), Some(TipoToken::FlechaDoble)) {
                return Err(self.error("se esperaba '=>' en brazo de 'coincidir'"));
            }
            let flecha = &self.tokens[self.pos];
            let (bl, bc) = (flecha.linea, flecha.columna);
            self.pos += 1; // '=>'
            self.omitir_finales_de_linea();
            let cuerpo = if matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
                self.bloque()?
            } else {
                let expr = self.expresion()?;
                vec![Sentencia::Expresion(expr)]
            };
            brazos.push(BrazoCoincidir {
                patron,
                cuerpo,
                linea: bl,
                columna: bc,
            });
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::Coma)) {
                self.pos += 1;
                self.omitir_finales_de_linea();
            }
        }
        if !matches!(self.actual(), Some(TipoToken::LlaveDer)) {
            return Err(self.error("se esperaba '}' para cerrar 'coincidir'"));
        }
        self.pos += 1; // '}'
        Ok(Expresion::Coincidir {
            scrutinee: Box::new(scrutinee),
            brazos,
            linea,
            columna,
        })
    }

    fn parsear_patron(&mut self) -> Result<Patron, ErrorSintactico> {
        self.omitir_finales_de_linea();
        let t = self
            .tokens
            .get(self.pos)
            .cloned()
            .ok_or_else(|| self.error("se esperaba un patrón"))?;
        let (linea, columna) = (t.linea, t.columna);
        match &t.tipo {
            TipoToken::Identificador(nombre) if nombre == "_" => {
                self.pos += 1;
                Ok(Patron::Wildcard { linea, columna })
            }
            TipoToken::Verdadero => {
                self.pos += 1;
                Ok(Patron::Literal {
                    valor: Valor::Logico(true),
                    linea,
                    columna,
                })
            }
            TipoToken::Falso => {
                self.pos += 1;
                Ok(Patron::Literal {
                    valor: Valor::Logico(false),
                    linea,
                    columna,
                })
            }
            TipoToken::Nulo => {
                self.pos += 1;
                Ok(Patron::Literal {
                    valor: Valor::Nulo,
                    linea,
                    columna,
                })
            }
            TipoToken::Cadena(s) => {
                let s = s.clone();
                self.pos += 1;
                Ok(Patron::Literal {
                    valor: Valor::Texto(s),
                    linea,
                    columna,
                })
            }
            TipoToken::Numero => {
                let v = clasificar_literal(&t.lexema).map_err(|m| ErrorSintactico {
                    mensaje: format!("literal numérico inválido {:?}: {m}", t.lexema),
                    linea: t.linea,
                    columna: t.columna,
                    archivo: self.archivo.clone(),
                })?;
                self.pos += 1;
                Ok(Patron::Literal {
                    valor: v,
                    linea,
                    columna,
                })
            }
            TipoToken::Menos
                if matches!(
                    self.tokens.get(self.pos + 1).map(|x| &x.tipo),
                    Some(TipoToken::Numero)
                ) =>
            {
                self.pos += 1; // -
                let nt = &self.tokens[self.pos];
                let lex = format!("-{}", nt.lexema);
                let v = clasificar_literal(&lex).map_err(|m| ErrorSintactico {
                    mensaje: format!("literal numérico inválido {lex:?}: {m}"),
                    linea: nt.linea,
                    columna: nt.columna,
                    archivo: self.archivo.clone(),
                })?;
                self.pos += 1;
                Ok(Patron::Literal {
                    valor: v,
                    linea,
                    columna,
                })
            }
            TipoToken::Identificador(nombre) => {
                let nombre = nombre.clone();
                self.pos += 1;
                // Variant pattern: Tipo.Variante ...
                if matches!(self.actual(), Some(TipoToken::Punto)) {
                    self.pos += 1; // '.'
                    let variante = match self.actual() {
                        Some(TipoToken::Identificador(v)) => {
                            let v = v.clone();
                            self.pos += 1;
                            v
                        }
                        _ => {
                            return Err(
                                self.error("se esperaba nombre de variante tras '.' en patrón")
                            );
                        }
                    };
                    let mut campos = Vec::new();
                    if matches!(self.actual(), Some(TipoToken::ParentesisIzq)) {
                        self.pos += 1; // '('
                        self.omitir_finales_de_linea();
                        if !matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                            loop {
                                campos.push(self.parsear_patron()?);
                                self.omitir_finales_de_linea();
                                if matches!(self.actual(), Some(TipoToken::Coma)) {
                                    self.pos += 1;
                                    self.omitir_finales_de_linea();
                                    if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                                        break;
                                    }
                                    continue;
                                }
                                break;
                            }
                        }
                        if !matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                            return Err(self.error("se esperaba ')' en patrón de variante"));
                        }
                        self.pos += 1;
                    }
                    return Ok(Patron::Variante {
                        tipo: nombre,
                        variante,
                        campos,
                        linea,
                        columna,
                    });
                }
                // Struct pattern: Tipo { campo: patron }
                if matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
                    // Disambiguate: struct pattern must have `ident :` or `}` (empty allowed?)
                    let mut j = self.pos + 1;
                    while matches!(
                        self.tokens.get(j).map(|x| &x.tipo),
                        Some(TipoToken::FinDeLinea)
                    ) {
                        j += 1;
                    }
                    let es_struct_pat = match self.tokens.get(j).map(|x| &x.tipo) {
                        Some(TipoToken::LlaveDer) => true,
                        Some(TipoToken::Identificador(_)) => matches!(
                            self.tokens.get(j + 1).map(|x| &x.tipo),
                            Some(TipoToken::DosPuntos)
                        ),
                        _ => false,
                    };
                    if es_struct_pat {
                        self.pos += 1; // '{'
                        self.omitir_finales_de_linea();
                        let mut campos = Vec::new();
                        while !matches!(
                            self.actual(),
                            Some(TipoToken::LlaveDer) | Some(TipoToken::Fin) | None
                        ) {
                            self.omitir_finales_de_linea();
                            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                                break;
                            }
                            let (cl, cc) = self
                                .tokens
                                .get(self.pos)
                                .map(|x| (x.linea, x.columna))
                                .unwrap_or((linea, columna));
                            let campo = match self.actual() {
                                Some(TipoToken::Identificador(n)) => {
                                    let n = n.clone();
                                    self.pos += 1;
                                    n
                                }
                                _ => {
                                    return Err(
                                        self.error("se esperaba nombre de campo en patrón struct")
                                    );
                                }
                            };
                            self.omitir_finales_de_linea();
                            if !matches!(self.actual(), Some(TipoToken::DosPuntos)) {
                                return Err(self.error(
                                    "se esperaba ':' tras nombre de campo en patrón struct",
                                ));
                            }
                            self.pos += 1; // ':'
                            self.omitir_finales_de_linea();
                            let p = self.parsear_patron()?;
                            let _ = (cl, cc);
                            campos.push((campo, p));
                            self.omitir_finales_de_linea();
                            if matches!(self.actual(), Some(TipoToken::Coma)) {
                                self.pos += 1;
                                self.omitir_finales_de_linea();
                                continue;
                            }
                            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                                break;
                            }
                        }
                        if !matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                            return Err(self.error("se esperaba '}' en patrón struct"));
                        }
                        self.pos += 1;
                        return Ok(Patron::Estructura {
                            tipo: nombre,
                            campos,
                            linea,
                            columna,
                        });
                    }
                }
                Ok(Patron::Binding {
                    nombre,
                    linea,
                    columna,
                })
            }
            _ => Err(self.error("patrón no soportado en 'coincidir' v1")),
        }
    }

    fn parsear_construccion_estructura(
        &mut self,
        nombre: String,
        linea: usize,
        columna: usize,
    ) -> Result<Expresion, ErrorSintactico> {
        if !matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
            return Err(self.error("se esperaba '{' en la construcción de estructura"));
        }
        self.pos += 1; // '{'
        let prev_bloqueo = self.bloquear_literal_struct_cabeza;
        self.bloquear_literal_struct_cabeza = false;
        self.omitir_finales_de_linea();

        let mut campos = Vec::new();
        let mut nombres_vistos = std::collections::HashSet::new();
        while !matches!(
            self.actual(),
            Some(TipoToken::LlaveDer) | Some(TipoToken::Fin) | None
        ) {
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                break;
            }
            let (c_linea, c_columna) = self
                .tokens
                .get(self.pos)
                .map(|tk| (tk.linea, tk.columna))
                .unwrap_or((linea, columna));
            let nombre_campo = match self.actual() {
                Some(TipoToken::Identificador(n)) => {
                    let n = n.clone();
                    self.pos += 1;
                    n
                }
                _ => {
                    self.bloquear_literal_struct_cabeza = prev_bloqueo;
                    return Err(self
                        .error("se esperaba un nombre de campo en la construcción de estructura"));
                }
            };
            if !nombres_vistos.insert(nombre_campo.clone()) {
                self.bloquear_literal_struct_cabeza = prev_bloqueo;
                return Err(ErrorSintactico {
                    mensaje: format!(
                        "el campo '{nombre_campo}' está duplicado en la construcción de '{nombre}'"
                    ),
                    linea: c_linea,
                    columna: c_columna,
                    archivo: self.archivo.clone(),
                });
            }
            if !matches!(self.actual(), Some(TipoToken::DosPuntos)) {
                self.bloquear_literal_struct_cabeza = prev_bloqueo;
                return Err(self.error(format!("se esperaba ':' tras el campo '{nombre_campo}'")));
            }
            self.pos += 1; // ':'
            let valor = match self.expresion() {
                Ok(v) => v,
                Err(e) => {
                    self.bloquear_literal_struct_cabeza = prev_bloqueo;
                    return Err(e);
                }
            };
            campos.push((nombre_campo, valor));
            self.omitir_finales_de_linea();
            if matches!(self.actual(), Some(TipoToken::Coma)) {
                self.pos += 1;
                self.omitir_finales_de_linea();
                continue;
            }
            if matches!(self.actual(), Some(TipoToken::LlaveDer)) {
                break;
            }
            if matches!(self.actual(), Some(TipoToken::FinDeLinea)) {
                self.omitir_finales_de_linea();
                continue;
            }
            self.bloquear_literal_struct_cabeza = prev_bloqueo;
            return Err(
                self.error("se esperaba ',', nueva línea o '}' en la construcción de estructura")
            );
        }

        if !matches!(self.actual(), Some(TipoToken::LlaveDer)) {
            self.bloquear_literal_struct_cabeza = prev_bloqueo;
            return Err(self.error(format!(
                "se esperaba '}}' para cerrar la construcción de '{nombre}'"
            )));
        }
        self.pos += 1; // '}'
        self.bloquear_literal_struct_cabeza = prev_bloqueo;

        Ok(Expresion::ConstruccionEstructura {
            nombre,
            campos,
            linea,
            columna,
        })
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
            Some(TipoToken::Coincidir) => self.parsear_coincidir(),
            Some(TipoToken::Yo) => {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                self.pos += 1;
                Ok(Expresion::Variable {
                    nombre: "yo".to_string(),
                    linea,
                    columna,
                })
            }
            Some(TipoToken::Identificador(nombre)) => {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                let nombre = nombre.clone();
                self.pos += 1;
                // Construcción de estructura nominal: `Tipo { campo: expr }` (Fase 08 S8.1).
                if matches!(self.actual(), Some(TipoToken::LlaveIzq)) {
                    if self.bloquear_literal_struct_cabeza {
                        // C1: no interpretar como struct literal en cabeza de control.
                        // Solo error si parece claramente un literal struct (Ident ':' / '}' / 'publico').
                        let parece_struct = {
                            let mut j = self.pos + 1; // tras '{'
                            while matches!(
                                self.tokens.get(j).map(|t| &t.tipo),
                                Some(TipoToken::FinDeLinea)
                            ) {
                                j += 1;
                            }
                            match self.tokens.get(j).map(|t| &t.tipo) {
                                Some(TipoToken::Publico) => true,
                                Some(TipoToken::Identificador(_)) => {
                                    matches!(
                                        self.tokens.get(j + 1).map(|t| &t.tipo),
                                        Some(TipoToken::DosPuntos)
                                    )
                                }
                                // `{}` vacío es ambiguo con bloque vacío de control; no rechazar como struct literal.
                                Some(TipoToken::LlaveDer) => false,
                                _ => false,
                            }
                        };
                        if parece_struct {
                            return Err(self.error(
                                "literal struct directo no permitido en cabeza de control; envuelve la expresión entre paréntesis, p. ej. `si (Tipo { ... }).campo { ... }`",
                            ));
                        }
                        // Si no parece struct literal, tratar como variable y dejar '{' al bloque de control.
                    } else {
                        return self.parsear_construccion_estructura(nombre, linea, columna);
                    }
                }
                // Turbofish en llamada directa por identificador (G2): `id::<T>(...)`
                if let Some(argumentos_tipo) = self.intentar_parsear_argumentos_tipo_turbofish()? {
                    if !matches!(self.actual(), Some(TipoToken::ParentesisIzq)) {
                        return Err(self.error(
                            "los argumentos de tipo solo se permiten en llamadas directas por identificador",
                        ));
                    }
                    self.pos += 1; // '('
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
                        argumentos_tipo,
                        linea,
                        columna,
                    });
                }
                // Detectar `:: <` fuera de contexto admitido en G2
                if matches!(self.actual(), Some(TipoToken::DosPuntos))
                    && matches!(
                        self.tokens.get(self.pos + 1).map(|t| &t.tipo),
                        Some(TipoToken::DosPuntos)
                    )
                {
                    return Err(self.error(
                        "los argumentos de tipo solo se permiten en llamadas directas por identificador",
                    ));
                }
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
                        argumentos_tipo: Vec::new(),
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
                let prev_bloqueo = self.bloquear_literal_struct_cabeza;
                self.bloquear_literal_struct_cabeza = false;
                let e = self.expresion();
                self.en_grupo -= 1;
                self.bloquear_literal_struct_cabeza = prev_bloqueo;
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
            Some(TipoToken::Funcion) => {
                let t = &self.tokens[self.pos];
                let (linea, columna) = (t.linea, t.columna);
                self.pos += 1; // 'funcion'
                if !matches!(self.actual(), Some(TipoToken::ParentesisIzq)) {
                    return Err(self.error("se esperaba '(' tras 'funcion' en lambda"));
                }
                self.pos += 1; // '('
                let mut parametros: Vec<(String, Option<TipoAnotacion>)> = Vec::new();
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
                    let tipo_param = if matches!(self.actual(), Some(TipoToken::DosPuntos)) {
                        let tipo = self.parsear_tipo_anotacion()?;
                        Some(tipo)
                    } else {
                        None
                    };
                    let este_tiene_tipo = tipo_param.is_some();
                    match primer_tiene_tipo {
                        None => primer_tiene_tipo = Some(este_tiene_tipo),
                        Some(pt) if pt != este_tiene_tipo => {
                            return Err(self.error(
                                "la firma de lambda mezcla parámetros con y sin tipo".to_string(),
                            ));
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
                    if matches!(self.actual(), Some(TipoToken::ParentesisDer)) {
                        self.pos += 1;
                        break;
                    }
                    return Err(self.error("se esperaba ',' o ')'"));
                }
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
                let prev = self.en_funcion;
                self.en_funcion = true;
                let cuerpo = self.bloque()?;
                self.en_funcion = prev;
                Ok(Expresion::Lambda {
                    parametros,
                    tipo_retorno,
                    cuerpo,
                    linea,
                    columna,
                })
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
            // Permitir llamada sobre cualquier expresión que evalúe a una función.
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
        | Expresion::Lambda { linea, columna, .. }
        | Expresion::Rango { linea, columna, .. }
        | Expresion::ConstruccionEstructura { linea, columna, .. }
        | Expresion::ConstruccionVariante { linea, columna, .. }
        | Expresion::Coincidir { linea, columna, .. } => (*linea, *columna),
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
