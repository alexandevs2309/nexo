//! Contrato semántico común y normalización para comparación diferencial TW↔VM.
//!
//! Este módulo **no modifica la semántica de ejecución** de los motores: solo
//! transforma resultados ya producidos (`Valor`, `ErrorEjecucion`, `ErrorVm`)
//! a una representación comparable y define reglas de equivalencia.
//!
//! Principios (política estricta 04.2):
//! - No usa `Debug`/`Display` literales como criterio de igualdad semántica.
//! - Categorías y subcausas diferentes nunca son equivalentes.
//! - Los datos semánticamente relevantes deben coincidir según reglas explícitas.
//! - Las diferencias de representación tolerables solo se aceptan mediante listas
//!   blancas declaradas por categoría/campo; toda diferencia no contemplada produce
//!   divergencia o no verificable (nunca equivalencia automática).
//! - `DiferenciaDiagnostico` solo se emite en casos declarados y justificados.
//! - No introduce tolerancias numéricas arbitrarias: la equivalencia numérica
//!   sigue las conversiones/valores definidos por BASILISCO (`Valor`/ADR-0005).

use crate::evaluador::{CausaError, ErrorEjecucion};
use crate::valor::{MotivoAritmetico, Valor};
use crate::vm::ErrorVm;
use std::collections::BTreeMap;

/// Valor normalizado para comparación semántica.
#[derive(Debug, Clone, PartialEq)]
pub enum ValorNormalizado {
    Entero(i64),
    Decimal(i128),
    RealBits(u64),
    Logico(bool),
    Texto(String),
    Nulo,
    /// Colección secuencial: `Arreglo` y `Lista` conservan su clase observada.
    /// La equivalencia de contenido/orden entre clases distintas solo puede
    /// aceptarse como diferencia de representación/diagnóstico cuando una regla
    /// explícita lo permita (p. ej. GAP-051 en diagnóstico de asignación tipada).
    Secuencia {
        /// "arreglo" | "lista"
        clase: &'static str,
        elementos: Vec<ValorNormalizado>,
    },
    Objeto(BTreeMap<String, ValorNormalizado>),
    FechaHora(String),
    /// Valor opaco: solo comparable si puede demostrarse equivalencia semántica.
    /// `identidad_comprobable` indica si `detalle` es evidencia suficiente.
    Opaco {
        tipo: String,
        detalle: String,
        identidad_comprobable: bool,
    },
    EstructuraNominal {
        tipo_id: u32,
        campos: Vec<ValorNormalizado>,
    },
    EnumNominal {
        tipo_id: u32,
        variante: u32,
        payload: Vec<ValorNormalizado>,
    },
}

/// Categoría de error normalizada.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CategoriaError {
    Aritmetico,
    NombreNoDefinido,
    DeclaracionDuplicada,
    AsignacionIncompatible,
    ReasignacionInmutable,
    CondicionNoLogica,
    OperandoNoLogico,
    TiposIncompatibles,
    LimiteIteraciones,
    FirmaInvalida,
    RetornoAusente,
    TipoDeRetornoIncompatible,
    RetornarFueraDeBloque,
    TipoParametroIncompatible,
    RetornoIncompatibleConAnotacion,
    IteracionSobreNoColeccion,
    RangoTipoInvalido,
    ModificacionDuranteIteracion,
    NombreReservado,
    EntradaInvalida,
    ArchivoNoEncontrado,
    PermisosDenegados,
    ErrorDeArchivo,
    ArchivoYaCerrado,
    IndiceTextoFueraDeRango,
    ConversionTextoInvalida,
    SeparadorVacio,
    ModuloNoEncontrado,
    CicloDeImportacion,
    SimbodoPrivado,
    SimbodoNoExportado,
    ImportacionDuplicada,
    RutaNoPermitida,
    ExtensionNoSoportada,
    ImportacionesNoDisponibles,
    SentenciaNoPermitidaEnModulo,
    FfiInvalido,
    // VM / infraestructura de ejecución
    FuncionNoEncontrada,
    ArgumentosInsuficientes,
    LimitePasosExcedido,
    ProfundidadExcedida,
    BloqueInalcanzable,
    TipoEsperado,
    IndiceFueraDeRango,
    CampoInexistente,
    ConversionInvalido,
    // Fase 08 nominal
    TipoNominalDesconocido,
    TipoNominalDuplicadoOSombreado,
    CampoStructDesconocido,
    CampoStructDuplicado,
    CampoStructFaltante,
    CampoStructSobrante,
    TipoCampoIncompatible,
    ConstruccionNominalInvalida,
    VarianteEnumDesconocida,
    PayloadVarianteIncompatible,
    ConstruccionVarianteInvalida,
    PatronIncompatible,
    MatchNoExhaustivo,
    PatronInalcanzable,
    BindingPatronInvalido,
    UsoPatronInvalido,
    MetodoNoEncontrado,
    FirmaMetodoIncompatibleOColision,
    TraitNoEncontrado,
    TraitNoImplementado,
    ImplIncompleta,
    ImplDuplicadaOCoherencia,
    CapacidadNominalNoSoportadaBackend,
    GenericoNominalNoSoportado,
    IgualdadNominalNoSoportada,
    MiembroNoVisible,
    RecursividadNominalPorValor,
    NoSoportado,
    Desconocido,
}

/// Causa semántica normalizada (independiente de mensaje literal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausaNormalizada {
    pub categoria: CategoriaError,
    /// Subcausa estable cuando aplica (p.ej. division_por_cero, desbordamiento).
    pub subcausa: Option<String>,
    /// Claves semánticas/diagnósticas relevantes cuando disponibles.
    pub datos: BTreeMap<String, String>,
}

/// Error normalizado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorNormalizado {
    pub causa: CausaNormalizada,
}

/// Efectos observables deterministas comparables.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EfectosObservables {
    pub stdout: Vec<String>,
}

/// Resultado normalizado de una ejecución.
#[derive(Debug, Clone, PartialEq)]
pub enum ResultadoNormalizado {
    Ok {
        valor: Option<ValorNormalizado>,
        efectos: EfectosObservables,
    },
    Err(ErrorNormalizado),
}

/// Clasificación de una comparación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparacion {
    /// Semánticamente equivalentes según contrato.
    Equivalente,
    /// Divergencia semántica (valores/errores incompatibles).
    DivergenciaSemantica(String),
    /// Misma causa semántica con diferencia de diagnóstico/representación
    /// explícitamente permitida por lista blanca declarada.
    DiferenciaDiagnostico(String),
    /// Representación distinta sin impacto semántico (absorbida por normalización).
    DiferenciaRepresentacion(String),
    /// Capacidad no soportada (rechazo legítimo) frente a soporte u otro estado.
    CapacidadNoSoportada {
        motor: &'static str,
        detalle: String,
    },
    /// No verificable con la evidencia disponible.
    NoVerificable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CmpValor {
    Equivalente,
    Divergente(String),
    NoVerificable(String),
}

fn norm_real(f: f64) -> u64 {
    // Normalización bitwise: NaN se colapsa a un patrón canónico, +0/-0 se distinguen por bits.
    if f.is_nan() {
        0x7ff8_0000_0000_0000u64
    } else {
        f.to_bits()
    }
}

/// Normaliza un `Valor` del lenguaje.
pub fn normalizar_valor(v: &Valor) -> ValorNormalizado {
    match v {
        Valor::Entero(n) => ValorNormalizado::Entero(*n),
        Valor::Decimal(n) => ValorNormalizado::Decimal(*n),
        Valor::Real(f) => ValorNormalizado::RealBits(norm_real(*f)),
        Valor::Logico(b) => ValorNormalizado::Logico(*b),
        Valor::Texto(s) => ValorNormalizado::Texto(s.clone()),
        Valor::Nulo => ValorNormalizado::Nulo,
        Valor::EstructuraNominal { tipo_id, campos } => ValorNormalizado::EstructuraNominal {
            tipo_id: *tipo_id,
            campos: campos.iter().map(normalizar_valor).collect(),
        },
        Valor::EnumNominal {
            tipo_id,
            variante,
            payload,
        } => ValorNormalizado::EnumNominal {
            tipo_id: *tipo_id,
            variante: *variante,
            payload: payload.iter().map(normalizar_valor).collect(),
        },
        Valor::Arreglo(elems) => ValorNormalizado::Secuencia {
            clase: "arreglo",
            elementos: elems.iter().map(normalizar_valor).collect(),
        },
        Valor::Lista(elems) => ValorNormalizado::Secuencia {
            clase: "lista",
            elementos: elems.iter().map(normalizar_valor).collect(),
        },
        Valor::Objeto(m) => {
            let mut out = BTreeMap::new();
            for (k, v) in m {
                out.insert(k.clone(), normalizar_valor(v));
            }
            ValorNormalizado::Objeto(out)
        }
        Valor::FechaHora(d) => ValorNormalizado::FechaHora(d.to_rfc3339()),
        Valor::Archivo(_) => ValorNormalizado::Opaco {
            tipo: "archivo".into(),
            detalle: "archivo".into(),
            identidad_comprobable: false,
        },
        Valor::Modulo(m) => ValorNormalizado::Opaco {
            tipo: "modulo".into(),
            detalle: m.ruta.display().to_string(),
            // Ruta canónica disponible: comprobable si coincide.
            identidad_comprobable: true,
        },
        Valor::Funcion(n) => ValorNormalizado::Opaco {
            tipo: "funcion".into(),
            detalle: n.clone(),
            identidad_comprobable: true,
        },
        Valor::FuncionClausura { .. } => ValorNormalizado::Opaco {
            tipo: "clausura".into(),
            detalle: "clausura".into(),
            identidad_comprobable: false,
        },
        Valor::ClausuraMir { nombre, .. } => ValorNormalizado::Opaco {
            tipo: "clausura".into(),
            detalle: nombre.clone(),
            // Sin identidad completa de capturas/instancia: no demostrable.
            identidad_comprobable: false,
        },
        Valor::Celda(_) => ValorNormalizado::Opaco {
            tipo: "celda".into(),
            detalle: "celda".into(),
            identidad_comprobable: false,
        },
    }
}

fn subcausa_arit(m: &MotivoAritmetico) -> &'static str {
    match m {
        MotivoAritmetico::DesbordamientoEntero => "desbordamiento_entero",
        MotivoAritmetico::DivisionPorCero => "division_por_cero",
        MotivoAritmetico::ExcesoRangoDecimal => "exceso_rango_decimal",
        MotivoAritmetico::ConversionIlegal => "conversion_ilegal",
        MotivoAritmetico::TiposIncompatibles => "tipos_incompatibles",
    }
}

fn normalizar_detalle_ffi(s: &str) -> String {
    // Los ids de biblioteca son detalles de runtime, no semántica observable.
    let mut out = s.to_string();
    for prefix in [
        "biblioteca FFI descargada: id ",
        "id de biblioteca FFI inválido: ",
    ] {
        if let Some(pos) = out.find(prefix) {
            let before = &out[..pos + prefix.len()];
            let after = &out[pos + prefix.len()..];
            let rest: String = after.chars().skip_while(|c| c.is_ascii_digit()).collect();
            out = format!("{}<id>{}", before, rest);
        }
    }
    out
}
fn subcausa_concurrencia_nosoportado(mensaje: &str) -> Option<String> {
    if mensaje.starts_with("tarea inexistente:") {
        Some("tarea_inexistente".into())
    } else if mensaje.starts_with("tarea ya esperada:") {
        Some("tarea_ya_esperada".into())
    } else if mensaje.starts_with("tarea falló:") {
        Some("tarea_fallida".into())
    } else if mensaje.starts_with("tarea: función no encontrada:") {
        Some("tarea_funcion_no_encontrada".into())
    } else if mensaje.starts_with("canal inexistente:") {
        Some("canal_inexistente".into())
    } else if mensaje.starts_with("canal cerrado y vacío:") {
        Some("canal_cerrado_vacio".into())
    } else if mensaje.starts_with("canal cerrado:") {
        Some("canal_cerrado".into())
    } else {
        None
    }
}

/// Normaliza un error de Tree-Walk (`ErrorEjecucion`).
pub fn normalizar_error_tw(e: &ErrorEjecucion) -> ErrorNormalizado {
    let mut datos = BTreeMap::new();
    let (categoria, subcausa) = match e.causa.as_ref() {
        CausaError::Aritmetica(m) => (
            CategoriaError::Aritmetico,
            Some(subcausa_arit(m).to_string()),
        ),
        CausaError::VariableNoDefinida(n) => {
            datos.insert("nombre".into(), n.clone());
            (CategoriaError::NombreNoDefinido, None)
        }
        CausaError::DeclaracionDuplicada(n) => {
            datos.insert("nombre".into(), n.clone());
            (CategoriaError::DeclaracionDuplicada, None)
        }
        CausaError::AsignacionIncompatible { esperado, obtenido } => {
            datos.insert("esperado".into(), esperado.clone());
            datos.insert("obtenido".into(), obtenido.clone());
            (CategoriaError::AsignacionIncompatible, None)
        }
        CausaError::ReasignacionInmutable { nombre } => {
            datos.insert("nombre".into(), nombre.clone());
            (CategoriaError::ReasignacionInmutable, None)
        }
        CausaError::CondicionNoLogica { obtenido } => {
            datos.insert("obtenido".into(), (*obtenido).to_string());
            (CategoriaError::CondicionNoLogica, None)
        }
        CausaError::OperandoNoLogico { operador, obtenido } => {
            datos.insert("operador".into(), (*operador).to_string());
            datos.insert("obtenido".into(), (*obtenido).to_string());
            (CategoriaError::OperandoNoLogico, None)
        }
        CausaError::TiposIncompatibles { izquierdo, derecho } => {
            datos.insert("izquierdo".into(), (*izquierdo).to_string());
            datos.insert("derecho".into(), (*derecho).to_string());
            (CategoriaError::TiposIncompatibles, None)
        }
        CausaError::LimiteIteraciones { limite } => {
            datos.insert("limite".into(), limite.to_string());
            (CategoriaError::LimiteIteraciones, None)
        }
        CausaError::FirmaInvalida {
            funcion,
            esperado,
            obtenido,
        } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("esperado".into(), esperado.to_string());
            datos.insert("obtenido".into(), obtenido.to_string());
            (CategoriaError::FirmaInvalida, None)
        }
        CausaError::RetornoAusente { funcion } => {
            datos.insert("funcion".into(), funcion.clone());
            (CategoriaError::RetornoAusente, None)
        }
        CausaError::TipoDeRetornoIncompatible { esperado, obtenido } => {
            datos.insert("esperado".into(), (*esperado).to_string());
            datos.insert("obtenido".into(), (*obtenido).to_string());
            (CategoriaError::TipoDeRetornoIncompatible, None)
        }
        CausaError::RetornarFueraDeBloque => (CategoriaError::RetornarFueraDeBloque, None),
        CausaError::TipoParametroIncompatible {
            funcion,
            parametro,
            esperado,
            obtenido,
        } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("parametro".into(), parametro.clone());
            datos.insert("esperado".into(), esperado.clone());
            datos.insert("obtenido".into(), obtenido.clone());
            (CategoriaError::TipoParametroIncompatible, None)
        }
        CausaError::RetornoIncompatibleConAnotacion {
            funcion,
            esperado,
            obtenido,
        } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("esperado".into(), esperado.clone());
            datos.insert("obtenido".into(), obtenido.clone());
            (CategoriaError::RetornoIncompatibleConAnotacion, None)
        }
        CausaError::SenalRomper | CausaError::SenalContinuar => {
            (CategoriaError::Desconocido, Some("senal_control".into()))
        }
        CausaError::IteracionSobreNoColeccion { obtenido } => {
            datos.insert("obtenido".into(), (*obtenido).to_string());
            (CategoriaError::IteracionSobreNoColeccion, None)
        }
        CausaError::RangoTipoInvalido { obtenido } => {
            datos.insert("obtenido".into(), (*obtenido).to_string());
            (CategoriaError::RangoTipoInvalido, None)
        }
        CausaError::ModificacionDuranteIteracion { variable } => {
            datos.insert("variable".into(), variable.clone());
            (CategoriaError::ModificacionDuranteIteracion, None)
        }
        CausaError::NombreReservado { nombre } => {
            datos.insert("nombre".into(), nombre.clone());
            (CategoriaError::NombreReservado, None)
        }
        CausaError::EntradaInvalida { esperado, obtenido } => {
            datos.insert("esperado".into(), esperado.clone());
            datos.insert("obtenido".into(), obtenido.clone());
            (CategoriaError::EntradaInvalida, None)
        }
        CausaError::ArchivoNoEncontrado { ruta } => {
            datos.insert("ruta".into(), ruta.clone());
            (CategoriaError::ArchivoNoEncontrado, None)
        }
        CausaError::PermisosDenegados { ruta } => {
            datos.insert("ruta".into(), ruta.clone());
            (CategoriaError::PermisosDenegados, None)
        }
        CausaError::ErrorDeArchivo { ruta, detalle } => {
            datos.insert("ruta".into(), ruta.clone());
            datos.insert("detalle".into(), detalle.clone());
            (CategoriaError::ErrorDeArchivo, None)
        }
        CausaError::ArchivoYaCerrado => (CategoriaError::ArchivoYaCerrado, None),
        CausaError::IndiceTextoFueraDeRango { indice, longitud } => {
            datos.insert("indice".into(), indice.to_string());
            datos.insert("longitud".into(), longitud.to_string());
            (CategoriaError::IndiceTextoFueraDeRango, None)
        }
        CausaError::ConversionTextoInvalida { valor, esperado } => {
            datos.insert("valor".into(), valor.clone());
            datos.insert("esperado".into(), esperado.clone());
            (CategoriaError::ConversionTextoInvalida, None)
        }
        CausaError::SeparadorVacio => (CategoriaError::SeparadorVacio, None),
        CausaError::ModuloNoEncontrado { ruta } => {
            datos.insert("ruta".into(), ruta.clone());
            (CategoriaError::ModuloNoEncontrado, None)
        }
        CausaError::CicloDeImportacion { cadena } => {
            datos.insert("cadena".into(), cadena.join("->"));
            (CategoriaError::CicloDeImportacion, None)
        }
        CausaError::SimbodoPrivado { nombre, modulo } => {
            datos.insert("nombre".into(), nombre.clone());
            datos.insert("modulo".into(), modulo.clone());
            (CategoriaError::SimbodoPrivado, None)
        }
        CausaError::SimbodoNoExportado { nombre, modulo } => {
            datos.insert("nombre".into(), nombre.clone());
            datos.insert("modulo".into(), modulo.clone());
            (CategoriaError::SimbodoNoExportado, None)
        }
        CausaError::ImportacionDuplicada { nombre, modulo } => {
            datos.insert("nombre".into(), nombre.clone());
            datos.insert("modulo".into(), modulo.clone());
            (CategoriaError::ImportacionDuplicada, None)
        }
        CausaError::RutaNoPermitida { ruta } => {
            datos.insert("ruta".into(), ruta.clone());
            (CategoriaError::RutaNoPermitida, None)
        }
        CausaError::ExtensionNoSoportada { extension } => {
            datos.insert("extension".into(), extension.clone());
            (CategoriaError::ExtensionNoSoportada, None)
        }
        CausaError::ImportacionesNoDisponibles => {
            (CategoriaError::ImportacionesNoDisponibles, None)
        }
        CausaError::SentenciaNoPermitidaEnModulo { sentencia } => {
            datos.insert("sentencia".into(), sentencia.clone());
            (CategoriaError::SentenciaNoPermitidaEnModulo, None)
        }
        CausaError::FFIInvalido { causa } => {
            let detalle = normalizar_detalle_ffi(&causa.to_string());
            datos.insert("detalle".into(), detalle);
            datos.insert("codigo".into(), causa.codigo().to_string());
            (
                CategoriaError::FfiInvalido,
                Some(causa.codigo().to_string()),
            )
        }
        CausaError::TipoNominalDesconocido { nombre } => {
            datos.insert("nombre".into(), nombre.clone());
            (
                CategoriaError::TipoNominalDesconocido,
                Some("BAS-7101".into()),
            )
        }
        CausaError::TipoNominalDuplicadoOSombreado { nombre, detalle } => {
            datos.insert("nombre".into(), nombre.clone());
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::TipoNominalDuplicadoOSombreado,
                Some("BAS-7102".into()),
            )
        }
        CausaError::CampoStructDesconocido { campo, tipo } => {
            datos.insert("campo".into(), campo.clone());
            datos.insert("tipo".into(), tipo.clone());
            (
                CategoriaError::CampoStructDesconocido,
                Some("BAS-7103".into()),
            )
        }
        CausaError::CampoStructDuplicado { campo, tipo } => {
            datos.insert("campo".into(), campo.clone());
            datos.insert("tipo".into(), tipo.clone());
            (
                CategoriaError::CampoStructDuplicado,
                Some("BAS-7104".into()),
            )
        }
        CausaError::CampoStructFaltante { campo, tipo } => {
            datos.insert("campo".into(), campo.clone());
            datos.insert("tipo".into(), tipo.clone());
            (CategoriaError::CampoStructFaltante, Some("BAS-7105".into()))
        }
        CausaError::CampoStructSobrante { campo, tipo } => {
            datos.insert("campo".into(), campo.clone());
            datos.insert("tipo".into(), tipo.clone());
            (CategoriaError::CampoStructSobrante, Some("BAS-7106".into()))
        }
        CausaError::TipoCampoIncompatible {
            campo,
            tipo,
            esperado,
            obtenido,
        } => {
            datos.insert("campo".into(), campo.clone());
            datos.insert("tipo".into(), tipo.clone());
            datos.insert("esperado".into(), esperado.clone());
            datos.insert("obtenido".into(), obtenido.clone());
            (
                CategoriaError::TipoCampoIncompatible,
                Some("BAS-7107".into()),
            )
        }
        CausaError::ConstruccionNominalInvalida { tipo, detalle } => {
            datos.insert("tipo".into(), tipo.clone());
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::ConstruccionNominalInvalida,
                Some("BAS-7108".into()),
            )
        }
        CausaError::VarianteEnumDesconocida { variante, tipo } => {
            datos.insert("variante".into(), variante.clone());
            datos.insert("tipo".into(), tipo.clone());
            (
                CategoriaError::VarianteEnumDesconocida,
                Some("BAS-7109".into()),
            )
        }
        CausaError::PayloadVarianteIncompatible {
            variante,
            tipo,
            detalle,
        } => {
            datos.insert("variante".into(), variante.clone());
            datos.insert("tipo".into(), tipo.clone());
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::PayloadVarianteIncompatible,
                Some("BAS-7110".into()),
            )
        }
        CausaError::ConstruccionVarianteInvalida { tipo, detalle } => {
            datos.insert("tipo".into(), tipo.clone());
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::ConstruccionVarianteInvalida,
                Some("BAS-7111".into()),
            )
        }
        CausaError::VarianteEnumDuplicada { variante, tipo } => {
            datos.insert("variante".into(), variante.clone());
            datos.insert("tipo".into(), tipo.clone());
            (
                CategoriaError::ConstruccionVarianteInvalida,
                Some("BAS-7111".into()),
            )
        }
        CausaError::PatronIncompatible { detalle } => {
            datos.insert("detalle".into(), detalle.clone());
            (CategoriaError::PatronIncompatible, Some("BAS-7112".into()))
        }
        CausaError::MatchNoExhaustivo { detalle } => {
            datos.insert("detalle".into(), detalle.clone());
            (CategoriaError::MatchNoExhaustivo, Some("BAS-7113".into()))
        }
        CausaError::PatronInalcanzable { detalle } => {
            datos.insert("detalle".into(), detalle.clone());
            (CategoriaError::PatronInalcanzable, Some("BAS-7114".into()))
        }
        CausaError::BindingPatronInvalido { detalle } => {
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::BindingPatronInvalido,
                Some("BAS-7115".into()),
            )
        }
        CausaError::UsoPatronInvalido { detalle } => {
            datos.insert("detalle".into(), detalle.clone());
            (CategoriaError::UsoPatronInvalido, Some("BAS-7116".into()))
        }
        CausaError::MetodoNoEncontrado { metodo, tipo } => {
            datos.insert("metodo".into(), metodo.clone());
            datos.insert("tipo".into(), tipo.clone());
            (CategoriaError::MetodoNoEncontrado, Some("BAS-7117".into()))
        }
        CausaError::FirmaMetodoIncompatibleOColision { detalle } => {
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::FirmaMetodoIncompatibleOColision,
                Some("BAS-7118".into()),
            )
        }
        CausaError::TraitNoEncontrado { nombre } => {
            datos.insert("nombre".into(), nombre.clone());
            (CategoriaError::TraitNoEncontrado, Some("BAS-7119".into()))
        }
        CausaError::TraitNoImplementado { rasgo, tipo } => {
            datos.insert("rasgo".into(), rasgo.clone());
            datos.insert("tipo".into(), tipo.clone());
            (CategoriaError::TraitNoImplementado, Some("BAS-7120".into()))
        }
        CausaError::ImplIncompleta {
            rasgo,
            tipo,
            detalle,
        } => {
            datos.insert("rasgo".into(), rasgo.clone());
            datos.insert("tipo".into(), tipo.clone());
            datos.insert("detalle".into(), detalle.clone());
            (CategoriaError::ImplIncompleta, Some("BAS-7121".into()))
        }
        CausaError::ImplDuplicadaOCoherencia { detalle } => {
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::ImplDuplicadaOCoherencia,
                Some("BAS-7122".into()),
            )
        }
        CausaError::CapacidadNominalNoSoportadaBackend {
            capacidad,
            backend,
            capa,
        } => {
            datos.insert("capacidad".into(), capacidad.clone());
            datos.insert("backend".into(), backend.clone());
            datos.insert("capa".into(), capa.clone());
            (
                CategoriaError::CapacidadNominalNoSoportadaBackend,
                Some("BAS-7124".into()),
            )
        }
        CausaError::GenericoNominalNoSoportado { tipo } => {
            datos.insert("tipo".into(), tipo.clone());
            (
                CategoriaError::GenericoNominalNoSoportado,
                Some("BAS-7125".into()),
            )
        }
        CausaError::IgualdadNominalNoSoportada { tipo } => {
            datos.insert("tipo".into(), tipo.clone());
            (
                CategoriaError::IgualdadNominalNoSoportada,
                Some("BAS-7126".into()),
            )
        }
        CausaError::MiembroNoVisible { miembro, tipo } => {
            datos.insert("miembro".into(), miembro.clone());
            datos.insert("tipo".into(), tipo.clone());
            (CategoriaError::MiembroNoVisible, Some("BAS-7128".into()))
        }
        CausaError::RecursividadNominalPorValor { tipo, ciclo } => {
            datos.insert("tipo".into(), tipo.clone());
            datos.insert("ciclo".into(), ciclo.clone());
            (
                CategoriaError::RecursividadNominalPorValor,
                Some("BAS-7130".into()),
            )
        }
        CausaError::NoSoportado { mensaje } => {
            let sub = subcausa_concurrencia_nosoportado(mensaje);
            let detalle = if sub.as_deref() == Some("tarea_fallida") {
                "tarea falló".to_string()
            } else {
                mensaje.clone()
            };
            datos.insert("detalle".into(), detalle);
            (CategoriaError::NoSoportado, sub)
        }
        CausaError::InferenciaArgumentosTipoNoDisponible { funcion } => {
            datos.insert("funcion".into(), funcion.clone());
            (
                CategoriaError::NoSoportado,
                Some("inferencia_argumentos_tipo_no_disponible".into()),
            )
        }
        CausaError::ArgumentosTipoEnFuncionNoGenerica { funcion } => {
            datos.insert("funcion".into(), funcion.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("argumentos_tipo_en_funcion_no_generica".into()),
            )
        }
        CausaError::AridadArgumentosTipoInvalida {
            funcion,
            esperado,
            obtenido,
        } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("esperado".into(), esperado.to_string());
            datos.insert("obtenido".into(), obtenido.to_string());
            (
                CategoriaError::FirmaInvalida,
                Some("aridad_argumentos_tipo_invalida".into()),
            )
        }
        CausaError::ArgumentoTipoInvalido { funcion, argumento } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("argumento".into(), argumento.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("argumento_tipo_invalido".into()),
            )
        }
        CausaError::InferenciaArgumentosTipoIncompleta { funcion } => {
            datos.insert("funcion".into(), funcion.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("inferencia_argumentos_tipo_incompleta".into()),
            )
        }
        CausaError::InferenciaArgumentosTipoAmbigua { funcion } => {
            datos.insert("funcion".into(), funcion.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("inferencia_argumentos_tipo_ambigua".into()),
            )
        }
        CausaError::ConflictoTiposInferencia { funcion } => {
            datos.insert("funcion".into(), funcion.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("conflicto_tipos_inferencia".into()),
            )
        }
        CausaError::OperacionProhibidaPorOpacidad { detalle } => {
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("operacion_prohibida_por_opacidad".into()),
            )
        }
        CausaError::IncompatibilidadConcretaGenericos { detalle } => {
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("incompatibilidad_concreta_genericos".into()),
            )
        }
        CausaError::SustitucionIncompleta { funcion } => {
            datos.insert("funcion".into(), funcion.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("sustitucion_incompleta".into()),
            )
        }
        CausaError::ForwardingGenericoInvalido { funcion, detalle } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("forwarding_generico_invalido".into()),
            )
        }
        CausaError::DependenciaGenericaNoResoluble { funcion, detalle } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("dependencia_generica_no_resoluble".into()),
            )
        }
        CausaError::RecursionPolimorficaCreciente {
            funcion,
            instancia,
            tipos,
            cadena,
            detalle,
        } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("instancia".into(), instancia.clone());
            datos.insert("tipos".into(), tipos.join(","));
            if !cadena.is_empty() {
                datos.insert("cadena".into(), cadena.join("->"));
            }
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("recursion_polimorfica_creciente".into()),
            )
        }
        CausaError::ProfundidadInstanciacionExcedida {
            funcion,
            instancia,
            tipos,
            limite,
            cadena,
            detalle,
        } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("instancia".into(), instancia.clone());
            datos.insert("tipos".into(), tipos.join(","));
            datos.insert("limite".into(), limite.to_string());
            if !cadena.is_empty() {
                datos.insert("cadena".into(), cadena.join("->"));
            }
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("profundidad_instanciacion_excedida".into()),
            )
        }
        CausaError::ReferenciaGenericaResidualIr { detalle, ubicacion } => {
            datos.insert("detalle".into(), detalle.clone());
            datos.insert("ubicacion".into(), ubicacion.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("referencia_generica_residual_ir".into()),
            )
        }
        CausaError::LimiteInstanciasExcedido {
            limite,
            solicitada,
            tipos,
            cadena,
            detalle,
        } => {
            datos.insert("limite".into(), limite.to_string());
            datos.insert("solicitada".into(), solicitada.clone());
            datos.insert("tipos".into(), tipos.join(","));
            if !cadena.is_empty() {
                datos.insert("cadena".into(), cadena.join("->"));
            }
            datos.insert("detalle".into(), detalle.clone());
            (
                CategoriaError::TiposIncompatibles,
                Some("limite_instancias_excedido".into()),
            )
        }
    };
    ErrorNormalizado {
        causa: CausaNormalizada {
            categoria,
            subcausa,
            datos,
        },
    }
}

/// Normaliza un error de VM (`ErrorVm`).
pub fn normalizar_error_vm(e: &ErrorVm) -> ErrorNormalizado {
    let mut datos = BTreeMap::new();
    let (categoria, subcausa) = match e {
        ErrorVm::FuncionNoEncontrada(n) => {
            datos.insert("nombre".into(), n.clone());
            (CategoriaError::FuncionNoEncontrada, None)
        }
        ErrorVm::ArgumentosInsuficientes { faltantes } => {
            datos.insert("faltantes".into(), faltantes.to_string());
            (CategoriaError::ArgumentosInsuficientes, None)
        }
        ErrorVm::FirmaInvalida {
            funcion,
            esperado,
            obtenido,
        } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("esperado".into(), esperado.to_string());
            datos.insert("obtenido".into(), obtenido.to_string());
            (CategoriaError::FirmaInvalida, None)
        }
        ErrorVm::LimitePasosExcedido => (CategoriaError::LimitePasosExcedido, None),
        ErrorVm::ProfundidadExcedida => (CategoriaError::ProfundidadExcedida, None),
        ErrorVm::BloqueInalcanzable => (CategoriaError::BloqueInalcanzable, None),
        ErrorVm::Aritmetico(m) => (
            CategoriaError::Aritmetico,
            Some(subcausa_arit(m).to_string()),
        ),
        ErrorVm::TiposIncompatibles { izquierdo, derecho } => {
            datos.insert("izquierdo".into(), (*izquierdo).to_string());
            datos.insert("derecho".into(), (*derecho).to_string());
            (CategoriaError::TiposIncompatibles, None)
        }
        ErrorVm::OperandoNoLogico { operador, obtenido } => {
            datos.insert("operador".into(), (*operador).to_string());
            datos.insert("obtenido".into(), (*obtenido).to_string());
            (CategoriaError::OperandoNoLogico, None)
        }
        ErrorVm::CondicionNoLogica { obtenido } => {
            datos.insert("obtenido".into(), (*obtenido).to_string());
            (CategoriaError::CondicionNoLogica, None)
        }
        ErrorVm::TipoEsperado {
            esperado,
            encontrado,
        } => {
            datos.insert("esperado".into(), (*esperado).to_string());
            datos.insert("encontrado".into(), (*encontrado).to_string());
            (CategoriaError::TipoEsperado, None)
        }
        ErrorVm::IndiceFueraDeRango { indice, tamano } => {
            datos.insert("indice".into(), indice.to_string());
            datos.insert("tamano".into(), tamano.to_string());
            (CategoriaError::IndiceFueraDeRango, None)
        }
        ErrorVm::CampoInexistente { campo } => {
            datos.insert("campo".into(), campo.clone());
            (CategoriaError::CampoInexistente, None)
        }
        ErrorVm::ConversionInvalido { valor, esperado } => {
            datos.insert("valor".into(), valor.clone());
            datos.insert("esperado".into(), (*esperado).to_string());
            (CategoriaError::ConversionInvalido, None)
        }
        ErrorVm::AsignacionIncompatible { esperado, obtenido } => {
            datos.insert("esperado".into(), esperado.clone());
            datos.insert("obtenido".into(), obtenido.clone());
            (CategoriaError::AsignacionIncompatible, None)
        }
        ErrorVm::TipoParametroIncompatible {
            funcion,
            parametro,
            esperado,
            obtenido,
        } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("parametro".into(), parametro.clone());
            datos.insert("esperado".into(), esperado.clone());
            datos.insert("obtenido".into(), obtenido.clone());
            (CategoriaError::TipoParametroIncompatible, None)
        }
        ErrorVm::RetornoIncompatible {
            funcion,
            esperado,
            obtenido,
        } => {
            datos.insert("funcion".into(), funcion.clone());
            datos.insert("esperado".into(), esperado.clone());
            datos.insert("obtenido".into(), obtenido.clone());
            (CategoriaError::RetornoIncompatibleConAnotacion, None)
        }
        ErrorVm::FfiInvalido(causa) => {
            let detalle = normalizar_detalle_ffi(&causa.to_string());
            datos.insert("detalle".into(), detalle);
            datos.insert("codigo".into(), causa.codigo().to_string());
            (
                CategoriaError::FfiInvalido,
                Some(causa.codigo().to_string()),
            )
        }
        ErrorVm::NoSoportado(m) => {
            if m.contains("BAS-7126") {
                datos.insert("tipo".into(), "nominal".into());
                (
                    CategoriaError::IgualdadNominalNoSoportada,
                    Some("BAS-7126".into()),
                )
            } else {
                let sub = subcausa_concurrencia_nosoportado(m);
                let detalle = if sub.as_deref() == Some("tarea_fallida") {
                    "tarea falló".to_string()
                } else {
                    m.clone()
                };
                datos.insert("detalle".into(), detalle);
                (CategoriaError::NoSoportado, sub)
            }
        }
    };
    ErrorNormalizado {
        causa: CausaNormalizada {
            categoria,
            subcausa,
            datos,
        },
    }
}

fn datos_iguales_excepto(
    a: &BTreeMap<String, String>,
    b: &BTreeMap<String, String>,
    ignorar: &[&str],
) -> bool {
    let mut ka: Vec<_> = a
        .keys()
        .filter(|k| !ignorar.contains(&k.as_str()))
        .collect();
    let mut kb: Vec<_> = b
        .keys()
        .filter(|k| !ignorar.contains(&k.as_str()))
        .collect();
    ka.sort();
    kb.sort();
    ka == kb && ka.iter().all(|k| a.get(*k) == b.get(*k))
}

/// Lista blanca explícita de diferencias diagnósticas permitidas.
/// Devuelve `Some(razon)` solo para casos declarados (p.ej. GAP-051).
fn diferencia_diagnostica_permitida(a: &CausaNormalizada, b: &CausaNormalizada) -> Option<String> {
    if a.categoria != b.categoria || a.subcausa != b.subcausa {
        return None;
    }
    match a.categoria {
        // GAP-051 (diagnóstico): en asignación/retorno/parámetro tipado incompatible,
        // la causa semántica normativa se define por categoría + tipo esperado (y, cuando
        // aplica, contexto función/parámetro). Se toleran diferencias diagnósticas acotadas:
        // - `obtenido`: representación de colección/contenedor (p.ej. Lista vs Arreglo).
        // - `parametro`: identificador no disponible en VM/MIR (arquitectónico; no altera
        //   la regla de conversión ni el tipo esperado/obtenido). Justificación técnica
        //   demostrable y delimitada; no autoriza diferencias en `esperado` ni en `funcion`.
        CategoriaError::AsignacionIncompatible
        | CategoriaError::TipoParametroIncompatible
        | CategoriaError::RetornoIncompatibleConAnotacion => {
            let esperado_igual = a.datos.get("esperado") == b.datos.get("esperado");
            let resto_igual = datos_iguales_excepto(&a.datos, &b.datos, &["obtenido", "parametro"]);
            let dif_obtenido = a.datos.get("obtenido") != b.datos.get("obtenido");
            let dif_parametro = a.datos.get("parametro") != b.datos.get("parametro");
            if esperado_igual && resto_igual && (dif_obtenido || dif_parametro) {
                Some(
                    "lista blanca GAP-051: diferencia diagnóstica acotada (obtenido/parametro) con misma causa/esperado"
                        .into(),
                )
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Equivalencias semánticas explícitas entre categorías/mapeos de motores distintos
/// (04.4): solo cuando la causa subyacente es la misma y los datos relevantes coinciden.
fn causas_equivalentes_mapeo(a: &CausaNormalizada, b: &CausaNormalizada) -> bool {
    let par = (&a.categoria, &b.categoria);
    match par {
        // Conversión de texto inválida: misma causa (parseo/conversión de texto fallida).
        (CategoriaError::ConversionTextoInvalida, CategoriaError::ConversionInvalido)
        | (CategoriaError::ConversionInvalido, CategoriaError::ConversionTextoInvalida) => {
            a.subcausa == b.subcausa
                && a.datos.get("valor") == b.datos.get("valor")
                && a.datos.get("esperado") == b.datos.get("esperado")
        }
        // Iteración sobre no colección: TW reporta categoría específica, VM como tipo esperado.
        (CategoriaError::IteracionSobreNoColeccion, CategoriaError::TipoEsperado)
        | (CategoriaError::TipoEsperado, CategoriaError::IteracionSobreNoColeccion) => {
            let (it, te) = if a.categoria == CategoriaError::IteracionSobreNoColeccion {
                (a, b)
            } else {
                (b, a)
            };
            it.subcausa.is_none()
                && te.subcausa.is_none()
                && te.datos.get("esperado").map(String::as_str) == Some("arreglo, lista o texto")
                && it.datos.get("obtenido") == te.datos.get("encontrado")
        }
        // Rango con tipo inválido: misma causa (extremos de rango no enteros).
        (CategoriaError::RangoTipoInvalido, CategoriaError::TipoEsperado)
        | (CategoriaError::TipoEsperado, CategoriaError::RangoTipoInvalido) => {
            let (rt, te) = if a.categoria == CategoriaError::RangoTipoInvalido {
                (a, b)
            } else {
                (b, a)
            };
            rt.subcausa.is_none()
                && te.subcausa.is_none()
                && te.datos.get("esperado").map(String::as_str) == Some("entero")
                && te
                    .datos
                    .get("encontrado")
                    .map(|e| e.contains("rango"))
                    .unwrap_or(false)
        }
        _ => false,
    }
}

/// Compara causas con política estricta 04.2 (con mapeos equivalentes explícitos 04.4).
fn comparar_causas(a: &CausaNormalizada, b: &CausaNormalizada) -> Comparacion {
    // Mapeos de equivalencia semántica explícitos entre motores (misma causa, distinta categoría).
    if a.categoria != b.categoria || a.subcausa != b.subcausa {
        if causas_equivalentes_mapeo(a, b) {
            return Comparacion::Equivalente;
        }
        // No soportado no se equipara a error semántico distinto sin evidencia explícita.
        if matches!(a.categoria, CategoriaError::NoSoportado)
            || matches!(b.categoria, CategoriaError::NoSoportado)
        {
            return Comparacion::NoVerificable(format!(
                "no soportado vs otra causa o capacidades no comparables: {:?} vs {:?}",
                a, b
            ));
        }
        return Comparacion::DivergenciaSemantica(format!(
            "categoría/subcausa distintas: {:?}/{:?} vs {:?}/{:?}",
            a.categoria, a.subcausa, b.categoria, b.subcausa
        ));
    }
    // 3. Datos semánticamente relevantes deben coincidir.
    if a.datos == b.datos {
        return Comparacion::Equivalente;
    }
    // 4. Lista blanca explícita de diferencias diagnósticas.
    if let Some(razon) = diferencia_diagnostica_permitida(a, b) {
        return Comparacion::DiferenciaDiagnostico(razon);
    }
    // 5. Diferencia no contemplada: divergencia (datos distintos no autorizados).
    Comparacion::DivergenciaSemantica(format!(
        "datos relevantes distintos sin lista blanca: {:?} vs {:?}",
        a.datos, b.datos
    ))
}

/// Comparación de valores con distinción equivalente/divergente/no verificable.
pub(crate) fn comparar_valores(a: &ValorNormalizado, b: &ValorNormalizado) -> CmpValor {
    match (a, b) {
        (ValorNormalizado::Entero(x), ValorNormalizado::Entero(y)) if x == y => {
            CmpValor::Equivalente
        }
        (ValorNormalizado::Decimal(x), ValorNormalizado::Decimal(y)) if x == y => {
            CmpValor::Equivalente
        }
        (ValorNormalizado::RealBits(x), ValorNormalizado::RealBits(y)) if x == y => {
            CmpValor::Equivalente
        }
        (ValorNormalizado::Logico(x), ValorNormalizado::Logico(y)) if x == y => {
            CmpValor::Equivalente
        }
        (ValorNormalizado::Texto(x), ValorNormalizado::Texto(y)) if x == y => CmpValor::Equivalente,
        (ValorNormalizado::Nulo, ValorNormalizado::Nulo) => CmpValor::Equivalente,
        (
            ValorNormalizado::Secuencia {
                clase: ca,
                elementos: ea,
            },
            ValorNormalizado::Secuencia {
                clase: cb,
                elementos: eb,
            },
        ) => {
            if ca != cb {
                return CmpValor::Divergente(format!(
                    "clase de secuencia distinta: {} vs {}",
                    ca, cb
                ));
            }
            if ea.len() != eb.len() {
                return CmpValor::Divergente(format!(
                    "longitud de secuencia distinta: {} vs {}",
                    ea.len(),
                    eb.len()
                ));
            }
            for (i, (x, y)) in ea.iter().zip(eb).enumerate() {
                match comparar_valores(x, y) {
                    CmpValor::Equivalente => {}
                    CmpValor::Divergente(d) => {
                        return CmpValor::Divergente(format!("elemento[{i}]: {d}"));
                    }
                    CmpValor::NoVerificable(d) => {
                        return CmpValor::NoVerificable(format!("elemento[{i}]: {d}"));
                    }
                }
            }
            CmpValor::Equivalente
        }
        (ValorNormalizado::Objeto(ma), ValorNormalizado::Objeto(mb)) => {
            if ma.keys().collect::<Vec<_>>() != mb.keys().collect::<Vec<_>>() {
                return CmpValor::Divergente("claves de objeto distintas".into());
            }
            for (k, va) in ma {
                match mb.get(k) {
                    Some(vb) => match comparar_valores(va, vb) {
                        CmpValor::Equivalente => {}
                        CmpValor::Divergente(d) => {
                            return CmpValor::Divergente(format!("campo {k}: {d}"));
                        }
                        CmpValor::NoVerificable(d) => {
                            return CmpValor::NoVerificable(format!("campo {k}: {d}"));
                        }
                    },
                    None => return CmpValor::Divergente(format!("campo ausente: {k}")),
                }
            }
            CmpValor::Equivalente
        }
        (ValorNormalizado::FechaHora(a), ValorNormalizado::FechaHora(b)) if a == b => {
            CmpValor::Equivalente
        }
        (
            ValorNormalizado::Opaco {
                tipo: ta,
                detalle: da,
                identidad_comprobable: ca,
            },
            ValorNormalizado::Opaco {
                tipo: tb,
                detalle: db,
                identidad_comprobable: cb,
            },
        ) => {
            if ta != tb {
                return CmpValor::Divergente(format!("tipo opaco distinto: {ta} vs {tb}"));
            }
            if *ca && *cb {
                if da == db {
                    CmpValor::Equivalente
                } else {
                    CmpValor::Divergente(format!("detalle opaco distinto: {da} vs {db}"))
                }
            } else {
                // Sin equivalencia semántica demostrable.
                CmpValor::NoVerificable(format!(
                    "equivalencia opaca no demostrable ({ta}: {da} vs {db})"
                ))
            }
        }
        (ValorNormalizado::Entero(_), ValorNormalizado::Entero(_))
        | (ValorNormalizado::Decimal(_), ValorNormalizado::Decimal(_))
        | (ValorNormalizado::RealBits(_), ValorNormalizado::RealBits(_))
        | (ValorNormalizado::Logico(_), ValorNormalizado::Logico(_))
        | (ValorNormalizado::Texto(_), ValorNormalizado::Texto(_))
        | (ValorNormalizado::FechaHora(_), ValorNormalizado::FechaHora(_)) => {
            CmpValor::Divergente(format!("valor distinto: {:?} vs {:?}", a, b))
        }
        _ => CmpValor::Divergente(format!(
            "tipos/constructores de valor distintos: {:?} vs {:?}",
            a, b
        )),
    }
}

/// API de conveniencia: equivalencia booleana solo si demostrablemente equivalente.
pub fn valores_equivalentes(a: &ValorNormalizado, b: &ValorNormalizado) -> bool {
    matches!(comparar_valores(a, b), CmpValor::Equivalente)
}

/// Compara causas de error: misma categoría y subcausa.
pub fn causas_semanticamente_equivalentes(a: &CausaNormalizada, b: &CausaNormalizada) -> bool {
    a.categoria == b.categoria && a.subcausa == b.subcausa
}

/// Equivalencia estricta/permitida de causa según política 04.2.
/// `true` solo para equivalente exacta o diferencia diagnóstica explícitamente whitelisted.
pub fn causas_equivalentes_estrictas(a: &CausaNormalizada, b: &CausaNormalizada) -> bool {
    matches!(
        comparar_causas(a, b),
        Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
    )
}

/// Compara resultados normalizados según contrato estricto 04.2.
pub fn comparar_semantica(a: &ResultadoNormalizado, b: &ResultadoNormalizado) -> Comparacion {
    match (a, b) {
        (
            ResultadoNormalizado::Ok {
                valor: va,
                efectos: ea,
            },
            ResultadoNormalizado::Ok {
                valor: vb,
                efectos: eb,
            },
        ) => {
            if ea != eb {
                return Comparacion::DivergenciaSemantica(format!(
                    "efectos distintos: {:?} vs {:?}",
                    ea, eb
                ));
            }
            let veq = match (va, vb) {
                (None, None) => CmpValor::Equivalente,
                (Some(x), Some(y)) => comparar_valores(x, y),
                _ => CmpValor::Divergente(format!(
                    "presencia de valor distinta: {:?} vs {:?}",
                    va, vb
                )),
            };
            match veq {
                CmpValor::Equivalente => Comparacion::Equivalente,
                CmpValor::Divergente(d) => {
                    Comparacion::DivergenciaSemantica(format!("valores: {d}"))
                }
                CmpValor::NoVerificable(d) => Comparacion::NoVerificable(format!("valores: {d}")),
            }
        }
        (ResultadoNormalizado::Err(ea), ResultadoNormalizado::Err(eb)) => {
            comparar_causas(&ea.causa, &eb.causa)
        }
        (ResultadoNormalizado::Err(e), ResultadoNormalizado::Ok { .. })
        | (ResultadoNormalizado::Ok { .. }, ResultadoNormalizado::Err(e)) => {
            if es_no_soportado(e) {
                Comparacion::NoVerificable(format!(
                    "no soportado frente a resultado ejecutable: {:?}",
                    e.causa
                ))
            } else {
                Comparacion::DivergenciaSemantica(format!("ok vs error: {:?} vs {:?}", a, b))
            }
        }
    }
}

/// Utilidad: ¿el error normalizado representa "no soportado" en el contexto dado?
pub fn es_no_soportado(err: &ErrorNormalizado) -> bool {
    matches!(err.causa.categoria, CategoriaError::NoSoportado)
}

// -----------------------------------------------------------------------------
// 04.5 — Contrato diferencial multi-backend (TW/VM/Nativo/WASM)
// -----------------------------------------------------------------------------

/// Backend considerado en el contrato diferencial multi-backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Backend {
    TreeWalk,
    Vm,
    Nativo,
    Wasm,
}

impl Backend {
    pub fn nombre(self) -> &'static str {
        match self {
            Backend::TreeWalk => "tree-walk",
            Backend::Vm => "vm",
            Backend::Nativo => "nativo",
            Backend::Wasm => "wasm",
        }
    }
}

/// Capacidades declaradas para clasificar soporte por subconjunto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capacidad {
    NumericosYLogicosBasicos,
    ControlDeFlujoBasico,
    FuncionesYLlamadasDirectas,
    Recursion,
    TextoYColeccionesBasicas,
    TryRescatar,
    ModulosAliasNominal,
    ClosuresYLambdas,
    ConcurrenciaCanalesTareas,
    Ffi,
    IoArchivos,
    JsonStdlib,
    EfectosStdoutDeterministas,
    ExportEnteroWasm,
}

/// Subconjunto explícito de capacidades soportadas por backend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SoporteBackend {
    pub capacidades: std::collections::BTreeSet<Capacidad>,
    pub limitaciones: Vec<String>,
}

impl SoporteBackend {
    fn con(caps: &[Capacidad], limitaciones: &[&str]) -> Self {
        Self {
            capacidades: caps.iter().copied().collect(),
            limitaciones: limitaciones.iter().map(|s| (*s).to_string()).collect(),
        }
    }
    pub fn soporta(&self, cap: Capacidad) -> bool {
        self.capacidades.contains(&cap)
    }
    pub fn soporta_todas(&self, caps: &[Capacidad]) -> bool {
        caps.iter().all(|c| self.capacidades.contains(c))
    }
}

/// Matriz declarada de capacidades por backend (subconjuntos reales observados, 04.5).
pub fn capacidades_backend(b: Backend) -> SoporteBackend {
    match b {
        Backend::TreeWalk | Backend::Vm => SoporteBackend::con(
            &[
                Capacidad::NumericosYLogicosBasicos,
                Capacidad::ControlDeFlujoBasico,
                Capacidad::FuncionesYLlamadasDirectas,
                Capacidad::Recursion,
                Capacidad::TextoYColeccionesBasicas,
                Capacidad::TryRescatar,
                Capacidad::ModulosAliasNominal,
                Capacidad::ClosuresYLambdas,
                Capacidad::ConcurrenciaCanalesTareas,
                Capacidad::Ffi,
                Capacidad::IoArchivos,
                Capacidad::JsonStdlib,
                Capacidad::EfectosStdoutDeterministas,
            ],
            &["Motores de referencia TW↔VM dentro del subconjunto común del contrato (Fase 04)."],
        ),
        Backend::Nativo => SoporteBackend::con(
            &[
                Capacidad::NumericosYLogicosBasicos,
                Capacidad::ControlDeFlujoBasico,
                Capacidad::FuncionesYLlamadasDirectas,
                Capacidad::Recursion,
                Capacidad::TextoYColeccionesBasicas,
                Capacidad::ModulosAliasNominal,
                Capacidad::JsonStdlib,
                Capacidad::EfectosStdoutDeterministas,
            ],
            &[
                "Requiere rustc; ejecución por compilación de Rust generado (salida stdout).",
                "No soporta try/catch/invoke, closures/celdas/rangos ni stdlib fuera del runtime embebido.",
                "No se declara soporte para concurrencia/canales/tareas, FFI ni E/S de archivos en este contrato diferencial.",
            ],
        ),
        Backend::Wasm => SoporteBackend::con(
            &[
                Capacidad::NumericosYLogicosBasicos,
                Capacidad::ControlDeFlujoBasico,
                Capacidad::FuncionesYLlamadasDirectas,
                Capacidad::Recursion,
                Capacidad::ExportEnteroWasm,
            ],
            &[
                "Subconjunto entero/lógico: export `bsk_main` ejecutado en Node (WebAssembly).",
                "No soporta texto, colecciones, try, módulos, closures, concurrencia, FFI, IO ni JSON en este contrato.",
            ],
        ),
    }
}

/// Clasificación obligatoria multi-backend (04.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClasificacionMulti {
    /// Comportamiento equivalente dentro del subconjunto soportado.
    Equivalent,
    /// Comportamiento diferente donde ambos backends declaran soporte.
    Divergence(String),
    /// Funcionalidad no soportada por uno de los backends.
    Unsupported(String),
    /// No existe evidencia suficiente para determinar equivalencia.
    Unverifiable(String),
}

/// Clasifica resultados de dos backends según soporte declarado y comparación semántica.
///
/// - Si el caso no está soportado por alguno, clasifica `Unsupported` (nunca divergencia).
/// - Si falta evidencia fiable, clasifica `Unverifiable`.
/// - Si ambos soportan y son comparables, reutiliza `comparar_semantica`.
pub fn clasificar_multibackend(
    a: &ResultadoNormalizado,
    b: &ResultadoNormalizado,
    caso_soportado_a: bool,
    caso_soportado_b: bool,
    backend_a: Backend,
    backend_b: Backend,
) -> ClasificacionMulti {
    if !caso_soportado_a || !caso_soportado_b {
        let quien = if !caso_soportado_a && !caso_soportado_b {
            format!("{}/{}", backend_a.nombre(), backend_b.nombre())
        } else if !caso_soportado_a {
            backend_a.nombre().to_string()
        } else {
            backend_b.nombre().to_string()
        };
        return ClasificacionMulti::Unsupported(format!(
            "funcionalidad no soportada por {quien} (subconjunto declarado)"
        ));
    }
    match comparar_semantica(a, b) {
        Comparacion::Equivalente
        | Comparacion::DiferenciaDiagnostico(_)
        | Comparacion::DiferenciaRepresentacion(_) => ClasificacionMulti::Equivalent,
        Comparacion::DivergenciaSemantica(m) => ClasificacionMulti::Divergence(m),
        Comparacion::CapacidadNoSoportada { motor, detalle } => {
            ClasificacionMulti::Unsupported(format!("{motor}: {detalle}"))
        }
        Comparacion::NoVerificable(m) => ClasificacionMulti::Unverifiable(m),
    }
}

/// Construye resultado comparable desde salida textual entera/booleana simple.
/// Devuelve `None` si no es interpretable con fiabilidad (evidencia insuficiente).
pub fn resultado_desde_salida_simple(stdout: &str) -> Option<ResultadoNormalizado> {
    let t = stdout.trim();
    if t.is_empty() {
        return None;
    }
    if t.lines().count() == 1 {
        if let Ok(n) = t.parse::<i64>() {
            return Some(ResultadoNormalizado::Ok {
                valor: Some(ValorNormalizado::Entero(n)),
                efectos: EfectosObservables::default(),
            });
        }
        if t == "verdadero" || t == "true" {
            return Some(ResultadoNormalizado::Ok {
                valor: Some(ValorNormalizado::Logico(true)),
                efectos: EfectosObservables::default(),
            });
        }
        if t == "falso" || t == "false" {
            return Some(ResultadoNormalizado::Ok {
                valor: Some(ValorNormalizado::Logico(false)),
                efectos: EfectosObservables::default(),
            });
        }
    }
    None
}

/// Resultado de proceso backend: éxito con stdout, fallo con stderr, o indisponible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EjecucionBackend {
    Ok { stdout: String },
    Fallo { stderr: String, stdout: String },
    Indisponible { motivo: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    fn causa(cat: CategoriaError, sub: Option<&str>, datos: &[(&str, &str)]) -> CausaNormalizada {
        let mut m = BTreeMap::new();
        for (k, v) in datos {
            m.insert((*k).to_string(), (*v).to_string());
        }
        CausaNormalizada {
            categoria: cat,
            subcausa: sub.map(|s| s.to_string()),
            datos: m,
        }
    }
    fn err(cat: CategoriaError, sub: Option<&str>, datos: &[(&str, &str)]) -> ResultadoNormalizado {
        ResultadoNormalizado::Err(ErrorNormalizado {
            causa: causa(cat, sub, datos),
        })
    }
    fn ok_entero(n: i64) -> ResultadoNormalizado {
        ResultadoNormalizado::Ok {
            valor: Some(ValorNormalizado::Entero(n)),
            efectos: EfectosObservables::default(),
        }
    }

    #[test]
    fn neg_aritmetico_vs_asignacion_diferentes() {
        let a = err(CategoriaError::Aritmetico, Some("division_por_cero"), &[]);
        let b = err(
            CategoriaError::AsignacionIncompatible,
            None,
            &[("esperado", "entero"), ("obtenido", "texto")],
        );
        assert!(matches!(
            comparar_semantica(&a, &b),
            Comparacion::DivergenciaSemantica(_)
        ));
    }

    #[test]
    fn neg_tipos_incompatibles_vs_condicion_no_logica_diferentes() {
        let a = err(
            CategoriaError::TiposIncompatibles,
            None,
            &[("izquierdo", "entero"), ("derecho", "texto")],
        );
        let b = err(
            CategoriaError::CondicionNoLogica,
            None,
            &[("obtenido", "entero")],
        );
        assert!(matches!(
            comparar_semantica(&a, &b),
            Comparacion::DivergenciaSemantica(_)
        ));
    }

    #[test]
    fn neg_misma_categoria_subcausa_diferente() {
        let a = err(CategoriaError::Aritmetico, Some("division_por_cero"), &[]);
        let b = err(
            CategoriaError::Aritmetico,
            Some("desbordamiento_entero"),
            &[],
        );
        assert!(matches!(
            comparar_semantica(&a, &b),
            Comparacion::DivergenciaSemantica(_)
        ));
    }

    #[test]
    fn neg_misma_categoria_subcausa_datos_criticos_diferentes() {
        let a = err(
            CategoriaError::IndiceFueraDeRango,
            None,
            &[("indice", "5"), ("tamano", "2")],
        );
        let b = err(
            CategoriaError::IndiceFueraDeRango,
            None,
            &[("indice", "1"), ("tamano", "2")],
        );
        assert!(matches!(
            comparar_semantica(&a, &b),
            Comparacion::DivergenciaSemantica(_)
        ));
    }

    #[test]
    fn wl_diferencia_representacion_permitida_diagnostico() {
        // GAP-051: misma causa/esperado, obtenido difiere por representación Lista/Arreglo.
        let a = err(
            CategoriaError::AsignacionIncompatible,
            None,
            &[("esperado", "lista<entero>"), ("obtenido", "Lista")],
        );
        let b = err(
            CategoriaError::AsignacionIncompatible,
            None,
            &[("esperado", "lista<entero>"), ("obtenido", "Arreglo")],
        );
        assert!(matches!(
            comparar_semantica(&a, &b),
            Comparacion::DiferenciaDiagnostico(_)
        ));
    }

    #[test]
    fn neg_diferencia_representacion_no_autorizada_divergencia() {
        let a = err(
            CategoriaError::AsignacionIncompatible,
            None,
            &[("esperado", "entero"), ("obtenido", "texto")],
        );
        let b = err(
            CategoriaError::AsignacionIncompatible,
            None,
            &[("esperado", "real"), ("obtenido", "texto")],
        );
        assert!(matches!(
            comparar_semantica(&a, &b),
            Comparacion::DivergenciaSemantica(_)
        ));
    }

    #[test]
    fn neg_ok_vs_error_divergencia() {
        let a = ok_entero(1);
        let b = err(CategoriaError::Aritmetico, Some("division_por_cero"), &[]);
        assert!(matches!(
            comparar_semantica(&a, &b),
            Comparacion::DivergenciaSemantica(_)
        ));
    }

    #[test]
    fn neg_error_vs_no_soportado_no_equivalentes() {
        let a = err(CategoriaError::Aritmetico, Some("division_por_cero"), &[]);
        let b = err(
            CategoriaError::NoSoportado,
            Some("hir_no_implementado"),
            &[],
        );
        let cmp = comparar_semantica(&a, &b);
        assert!(matches!(cmp, Comparacion::NoVerificable(_)));
        assert!(!matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ));
    }

    #[test]
    fn val_opaco_no_demostrable_es_no_verificable() {
        let a = ValorNormalizado::Opaco {
            tipo: "clausura".into(),
            detalle: "x".into(),
            identidad_comprobable: false,
        };
        let b = ValorNormalizado::Opaco {
            tipo: "clausura".into(),
            detalle: "x".into(),
            identidad_comprobable: false,
        };
        assert!(matches!(
            comparar_valores(&a, &b),
            CmpValor::NoVerificable(_)
        ));
    }

    #[test]
    fn val_secuencia_clase_distinta_diverge() {
        let a = ValorNormalizado::Secuencia {
            clase: "arreglo",
            elementos: vec![ValorNormalizado::Entero(1)],
        };
        let b = ValorNormalizado::Secuencia {
            clase: "lista",
            elementos: vec![ValorNormalizado::Entero(1)],
        };
        assert!(matches!(comparar_valores(&a, &b), CmpValor::Divergente(_)));
    }

    #[test]
    fn val_nan_canonico_equivalente() {
        let a = ValorNormalizado::RealBits(0x7ff8_0000_0000_0000);
        let b = ValorNormalizado::RealBits(0x7ff8_0000_0000_0000);
        assert!(matches!(comparar_valores(&a, &b), CmpValor::Equivalente));
    }
}
