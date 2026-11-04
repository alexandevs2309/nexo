//! Cliente HTTP mínimo, compartido por el evaluador y la VM (ureq).

use crate::valor::Valor;

/// `GET url` → cuerpo de la respuesta como texto.
pub fn http_get(url: &str) -> Result<Valor, String> {
    let body = ureq::get(url)
        .call()
        .map_err(|e| format!("http GET '{url}': {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("http GET '{url}': {e}"))?;
    Ok(Valor::Texto(body))
}

/// `POST url` con `cuerpo` → cuerpo de la respuesta como texto.
pub fn http_post(url: &str, cuerpo: &str) -> Result<Valor, String> {
    let body = ureq::post(url)
        .send(cuerpo)
        .map_err(|e| format!("http POST '{url}': {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("http POST '{url}': {e}"))?;
    Ok(Valor::Texto(body))
}
