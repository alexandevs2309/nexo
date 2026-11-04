//! Utilidades de fecha y hora, compartidas por el evaluador y la VM.

use chrono::{DateTime, Utc};

use crate::valor::Valor;

/// Milisegundos desde el epoch Unix (UTC).
pub fn ahora() -> Valor {
    Valor::Entero(Utc::now().timestamp_millis())
}

/// Fecha actual en formato ISO `YYYY-MM-DD` (UTC).
pub fn fecha_actual() -> Valor {
    Valor::Texto(Utc::now().format("%Y-%m-%d").to_string())
}

/// Formatea un instante (milisegundos desde el epoch) con el formato dado
/// (sintaxis de `chrono` strftime). Devuelve texto vacío si el formato falla.
pub fn formatear_fecha(ms: i64, formato: &str) -> Valor {
    match DateTime::from_timestamp_millis(ms) {
        Some(dt) => Valor::Texto(dt.format(formato).to_string()),
        None => Valor::Texto(String::new()),
    }
}
