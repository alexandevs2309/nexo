//! Sistema mínimo de paquetes y dependencias.
//!
//! Un proyecto BASILISCO se describe con `basilisco.toml`:
//!
//! ```toml
//! [paquete]
//! nombre = "mi-proyecto"
//! version = "0.1.0"
//!
//! [dependencias]
//! matematica = "0.1.0"
//! util = { version = "0.2.0", ruta = "../util" }
//! ```
//!
//! La resolución es local: no hay descarga desde red. Cada dependencia se
//! busca, en orden, en la ruta explícita, `paquetes/<nombre>`, `vendor/<nombre>`
//! y `<proyecto>/<nombre>`. El resultado se registra en `basilisco.lock` para
//! que las ejecuciones posteriores sean reproducibles.

use std::fs;
use std::path::{Path, PathBuf};

pub const MANIFIESTO: &str = "basilisco.toml";
pub const BLOQUEO: &str = "basilisco.lock";
pub const VERSION_BLOQUEO: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paquete {
    pub nombre: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependencia {
    pub nombre: String,
    /// Versión solicitada; `*` significa "cualquiera".
    pub version: String,
    /// Ruta explícita relativa al manifiesto, si se indicó.
    pub ruta: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifiesto {
    pub paquete: Paquete,
    pub dependencias: Vec<Dependencia>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependenciaResuelta {
    pub nombre: String,
    pub version: String,
    /// Ruta al archivo `.bsk` de entrada del paquete.
    pub ruta: PathBuf,
}

/// Lee y valida `basilisco.toml` en `base`.
pub fn cargar_manifiesto(base: &Path) -> Result<Manifiesto, String> {
    let ruta = base.join(MANIFIESTO);
    let texto = fs::read_to_string(&ruta)
        .map_err(|e| format!("no se pudo leer '{}': {e}", ruta.display()))?;
    parsear_manifiesto(&texto).map_err(|e| format!("{}: {e}", ruta.display()))
}

/// Parsea el contenido de un manifiesto.
pub fn parsear_manifiesto(texto: &str) -> Result<Manifiesto, String> {
    let mut tabla = String::new();
    let mut nombre: Option<String> = None;
    let mut version: Option<String> = None;
    let mut dependencias: Vec<Dependencia> = Vec::new();

    for (numero, cruda) in texto.lines().enumerate() {
        let linea = quitar_comentario(cruda).trim().to_string();
        if linea.is_empty() {
            continue;
        }
        if linea.starts_with('[') {
            if !linea.ends_with(']') || linea.starts_with("[[") {
                return Err(format!("línea {}: tabla inválida", numero + 1));
            }
            tabla = linea[1..linea.len() - 1].trim().to_string();
            match tabla.as_str() {
                "paquete" | "dependencias" => {}
                otra => return Err(format!("línea {}: tabla desconocida '{otra}'", numero + 1)),
            }
            continue;
        }

        let (clave, valor) = linea
            .split_once('=')
            .ok_or_else(|| format!("línea {}: se esperaba 'clave = valor'", numero + 1))?;
        let clave = clave.trim();
        let valor = valor.trim();
        if clave.is_empty() {
            return Err(format!("línea {}: clave vacía", numero + 1));
        }

        match tabla.as_str() {
            "paquete" => match clave {
                "nombre" => {
                    if nombre.is_some() {
                        return Err(format!("línea {}: 'nombre' duplicado", numero + 1));
                    }
                    nombre = Some(
                        parsear_texto(valor).map_err(|e| format!("línea {}: {e}", numero + 1))?,
                    );
                }
                "version" => {
                    if version.is_some() {
                        return Err(format!("línea {}: 'version' duplicado", numero + 1));
                    }
                    version = Some(
                        parsear_texto(valor).map_err(|e| format!("línea {}: {e}", numero + 1))?,
                    );
                }
                _ => {}
            },
            "dependencias" => {
                if dependencias.iter().any(|d| d.nombre == clave) {
                    return Err(format!(
                        "línea {}: dependencia '{clave}' duplicada",
                        numero + 1
                    ));
                }
                let dep = parsear_dependencia(clave, valor)
                    .map_err(|e| format!("línea {}: {e}", numero + 1))?;
                dependencias.push(dep);
            }
            _ => {
                return Err(format!(
                    "línea {}: clave fuera de una tabla ([paquete] o [dependencias])",
                    numero + 1
                ));
            }
        }
    }

    let nombre = nombre.ok_or_else(|| "falta 'nombre' en [paquete]".to_string())?;
    if !nombre_valido(&nombre) {
        return Err(format!("nombre de paquete inválido '{nombre}'"));
    }
    let version = version.unwrap_or_else(|| "0.0.0".to_string());
    if !version_valida(&version) {
        return Err(format!("versión inválida '{version}'"));
    }

    dependencias.sort_by(|a, b| a.nombre.cmp(&b.nombre));
    Ok(Manifiesto {
        paquete: Paquete { nombre, version },
        dependencias,
    })
}

/// Resuelve las dependencias de un manifiesto contra el sistema de archivos.
pub fn resolver_dependencias(
    manifiesto: &Manifiesto,
    base: &Path,
) -> Result<Vec<DependenciaResuelta>, String> {
    let mut resueltas = Vec::with_capacity(manifiesto.dependencias.len());
    for dep in &manifiesto.dependencias {
        resueltas.push(resolver_dependencia(dep, base)?);
    }
    resueltas.sort_by(|a, b| a.nombre.cmp(&b.nombre));
    Ok(resueltas)
}

fn resolver_dependencia(dep: &Dependencia, base: &Path) -> Result<DependenciaResuelta, String> {
    let mut candidatos: Vec<PathBuf> = Vec::new();
    if let Some(ruta) = &dep.ruta {
        candidatos.push(base.join(ruta));
    }
    candidatos.push(base.join("paquetes").join(&dep.nombre));
    candidatos.push(base.join("vendor").join(&dep.nombre));
    candidatos.push(base.join(&dep.nombre));

    let mut buscados = Vec::new();
    for candidato in candidatos {
        if let Some(archivo) = archivo_de_paquete(&candidato) {
            let declarado = manifestacion_de_paquete(&archivo);
            if let Some(otro) = &declarado {
                if otro.nombre != dep.nombre {
                    return Err(format!(
                        "la dependencia '{}' apunta a un paquete llamado '{}'",
                        dep.nombre, otro.nombre
                    ));
                }
                if dep.version != "*" && otro.version != dep.version {
                    return Err(format!(
                        "versión incompatible para '{}': se pidió {} y el paquete declara {}",
                        dep.nombre, dep.version, otro.version
                    ));
                }
            }
            let version = declarado
                .map(|p| p.version)
                .unwrap_or_else(|| dep.version.clone());
            return Ok(DependenciaResuelta {
                nombre: dep.nombre.clone(),
                version,
                ruta: archivo,
            });
        }
        buscados.push(candidato.display().to_string());
    }

    Err(format!(
        "no se encontró la dependencia '{}'. Rutas probadas: {}",
        dep.nombre,
        buscados.join(", ")
    ))
}

/// Devuelve el archivo de entrada si `ruta` es un archivo o un directorio de paquete.
fn archivo_de_paquete(ruta: &Path) -> Option<PathBuf> {
    if ruta.is_file() {
        return Some(ruta.to_path_buf());
    }
    if !ruta.is_dir() {
        return None;
    }
    let nombre = ruta.file_name().map(|n| n.to_string_lossy().to_string());
    let mut opciones = vec![
        ruta.join("lib.bsk"),
        ruta.join("src").join("lib.bsk"),
        ruta.join("principal.bsk"),
    ];
    if let Some(nombre) = nombre {
        opciones.push(ruta.join(format!("{nombre}.bsk")));
    }
    opciones.into_iter().find(|p| p.is_file())
}

/// Lee el `[paquete]` de un manifiesto vecino, si existe y es válido.
fn manifestacion_de_paquete(archivo: &Path) -> Option<Paquete> {
    let mut directorio = archivo.parent()?;
    loop {
        let manifiesto = directorio.join(MANIFIESTO);
        if manifiesto.is_file() {
            let texto = fs::read_to_string(&manifiesto).ok()?;
            let parsed = parsear_manifiesto(&texto).ok()?;
            return Some(parsed.paquete);
        }
        directorio = directorio.parent()?;
    }
}

/// Escribe `basilisco.lock` de forma determinista.
pub fn escribir_bloqueo(base: &Path, resueltas: &[DependenciaResuelta]) -> Result<(), String> {
    let mut salida = String::new();
    salida.push_str("# Generado por basilisco. No editar a mano.\n");
    salida.push_str(&format!("version = {VERSION_BLOQUEO}\n"));
    for dep in resueltas {
        let ruta = dep
            .ruta
            .strip_prefix(base)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| dep.ruta.clone());
        salida.push_str("\n[[dependencia]]\n");
        salida.push_str(&format!("nombre = \"{}\"\n", escapar(&dep.nombre)));
        salida.push_str(&format!("version = \"{}\"\n", escapar(&dep.version)));
        salida.push_str(&format!(
            "ruta = \"{}\"\n",
            escapar(&ruta.to_string_lossy())
        ));
    }
    let ruta = base.join(BLOQUEO);
    fs::write(&ruta, salida).map_err(|e| format!("no se pudo escribir '{}': {e}", ruta.display()))
}

/// Lee `basilisco.lock`; devuelve `None` si no existe.
pub fn leer_bloqueo(base: &Path) -> Result<Option<Vec<DependenciaResuelta>>, String> {
    let ruta = base.join(BLOQUEO);
    if !ruta.is_file() {
        return Ok(None);
    }
    let texto = fs::read_to_string(&ruta)
        .map_err(|e| format!("no se pudo leer '{}': {e}", ruta.display()))?;
    parsear_bloqueo(&texto)
        .map(Some)
        .map_err(|e| format!("{}: {e}", ruta.display()))
}

fn parsear_bloqueo(texto: &str) -> Result<Vec<DependenciaResuelta>, String> {
    let mut resultado: Vec<DependenciaResuelta> = Vec::new();
    let mut actual: Option<(Option<String>, Option<String>, Option<String>)> = None;
    let mut version_bloqueo: Option<u32> = None;

    for (numero, cruda) in texto.lines().enumerate() {
        let linea = quitar_comentario(cruda).trim().to_string();
        if linea.is_empty() {
            continue;
        }
        if linea == "[[dependencia]]" {
            if let Some(prev) = actual.take() {
                resultado.push(construir_dependencia(prev, numero)?);
            }
            actual = Some((None, None, None));
            continue;
        }
        if linea.starts_with('[') {
            return Err(format!("línea {}: sección desconocida", numero + 1));
        }
        let (clave, valor) = linea
            .split_once('=')
            .ok_or_else(|| format!("línea {}: se esperaba 'clave = valor'", numero + 1))?;
        let clave = clave.trim();
        let valor = valor.trim();
        if let Some((nombre, version, ruta)) = actual.as_mut() {
            let texto = parsear_texto(valor).map_err(|e| format!("línea {}: {e}", numero + 1))?;
            match clave {
                "nombre" => *nombre = Some(texto),
                "version" => *version = Some(texto),
                "ruta" => *ruta = Some(texto),
                _ => {}
            }
        } else if clave == "version" {
            let texto = if valor.starts_with('"') {
                parsear_texto(valor).map_err(|e| format!("línea {}: {e}", numero + 1))?
            } else {
                valor.to_string()
            };
            let numero_version: u32 = texto
                .parse()
                .map_err(|_| format!("línea {}: versión de bloqueo inválida", numero + 1))?;
            version_bloqueo = Some(numero_version);
        } else {
            return Err(format!(
                "línea {}: clave fuera de [[dependencia]]",
                numero + 1
            ));
        }
    }

    if let Some(prev) = actual.take() {
        resultado.push(construir_dependencia(prev, 0)?);
    }

    match version_bloqueo {
        Some(v) if v == VERSION_BLOQUEO => {}
        Some(v) => {
            return Err(format!(
                "versión de bloqueo {v} no soportada (se esperaba {VERSION_BLOQUEO})"
            ));
        }
        None => return Err("falta 'version' al inicio del bloqueo".to_string()),
    }

    resultado.sort_by(|a, b| a.nombre.cmp(&b.nombre));
    for par in resultado.windows(2) {
        if par[0].nombre == par[1].nombre {
            return Err(format!(
                "dependencia '{}' duplicada en el bloqueo",
                par[0].nombre
            ));
        }
    }
    Ok(resultado)
}

fn construir_dependencia(
    campos: (Option<String>, Option<String>, Option<String>),
    numero: usize,
) -> Result<DependenciaResuelta, String> {
    let (nombre, version, ruta) = campos;
    let ubicacion = if numero == 0 {
        "al final del bloqueo".to_string()
    } else {
        format!("línea {numero}")
    };
    Ok(DependenciaResuelta {
        nombre: nombre.ok_or_else(|| format!("{ubicacion}: falta 'nombre'"))?,
        version: version.ok_or_else(|| format!("{ubicacion}: falta 'version'"))?,
        ruta: PathBuf::from(ruta.ok_or_else(|| format!("{ubicacion}: falta 'ruta'"))?),
    })
}

/// Resuelve el nombre de un paquete importado usando `basilisco.lock` del
/// directorio de trabajo actual. Devuelve `None` si no hay bloqueo o no figura.
pub fn resolver_desde_bloqueo(nombre: &str) -> Option<PathBuf> {
    let base = std::env::current_dir().ok()?;
    let resueltas = leer_bloqueo(&base).ok()??;
    resueltas
        .into_iter()
        .find(|d| d.nombre == nombre)
        .map(|d| d.ruta)
        .filter(|ruta| ruta.is_file())
}

/// Comprueba si un nombre de paquete es aceptable.
pub fn nombre_valido(nombre: &str) -> bool {
    !nombre.is_empty()
        && nombre.len() <= 64
        && nombre
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && nombre
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
}

fn version_valida(version: &str) -> bool {
    version == "*"
        || (!version.is_empty()
            && version
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '*')))
}

fn parsear_dependencia(nombre: &str, valor: &str) -> Result<Dependencia, String> {
    if !nombre_valido(nombre) {
        return Err(format!("nombre de dependencia inválido '{nombre}'"));
    }
    if valor.starts_with('{') {
        let interior = valor
            .strip_prefix('{')
            .and_then(|v| v.strip_suffix('}'))
            .ok_or_else(|| "tabla en línea sin '}'".to_string())?
            .trim();
        let mut version = None;
        let mut ruta = None;
        for parte in dividir_por_comas(interior) {
            let (clave, valor) = parte
                .split_once('=')
                .ok_or_else(|| format!("entrada inválida '{parte}' en tabla en línea"))?;
            let clave = clave.trim();
            let texto = parsear_texto(valor.trim())?;
            match clave {
                "version" => version = Some(texto),
                "ruta" | "path" => ruta = Some(texto),
                otra => return Err(format!("clave desconocida '{otra}' en dependencia")),
            }
        }
        let version = version.unwrap_or_else(|| "*".to_string());
        if !version_valida(&version) {
            return Err(format!("versión inválida '{version}'"));
        }
        Ok(Dependencia {
            nombre: nombre.to_string(),
            version,
            ruta,
        })
    } else {
        let version = parsear_texto(valor)?;
        if !version_valida(&version) {
            return Err(format!("versión inválida '{version}'"));
        }
        Ok(Dependencia {
            nombre: nombre.to_string(),
            version,
            ruta: None,
        })
    }
}

fn dividir_por_comas(texto: &str) -> Vec<String> {
    let mut partes = Vec::new();
    let mut actual = String::new();
    let mut en_texto = false;
    let mut escape = false;
    for c in texto.chars() {
        if escape {
            actual.push(c);
            escape = false;
            continue;
        }
        match c {
            '\\' if en_texto => {
                actual.push(c);
                escape = true;
            }
            '"' => {
                en_texto = !en_texto;
                actual.push(c);
            }
            ',' if !en_texto => {
                if !actual.trim().is_empty() {
                    partes.push(actual.trim().to_string());
                }
                actual.clear();
            }
            _ => actual.push(c),
        }
    }
    if !actual.trim().is_empty() {
        partes.push(actual.trim().to_string());
    }
    partes
}

fn parsear_texto(valor: &str) -> Result<String, String> {
    let valor = valor.trim();
    if !(valor.starts_with('"') && valor.ends_with('"') && valor.len() >= 2) {
        return Err(format!(
            "se esperaba un texto entre comillas, se encontró '{valor}'"
        ));
    }
    let interior = &valor[1..valor.len() - 1];
    let mut salida = String::new();
    let mut chars = interior.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => salida.push('"'),
                Some('\\') => salida.push('\\'),
                Some('n') => salida.push('\n'),
                Some('t') => salida.push('\t'),
                Some(otro) => {
                    salida.push('\\');
                    salida.push(otro);
                }
                None => return Err("escape incompleto".to_string()),
            }
        } else {
            salida.push(c);
        }
    }
    Ok(salida)
}

fn escapar(texto: &str) -> String {
    texto
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn quitar_comentario(linea: &str) -> &str {
    let mut en_texto = false;
    let mut escape = false;
    for (i, c) in linea.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' if en_texto => escape = true,
            '"' => en_texto = !en_texto,
            '#' if !en_texto => return &linea[..i],
            _ => {}
        }
    }
    linea
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsea_manifiesto_con_dependencias() {
        let texto = r#"
# proyecto
[paquete]
nombre = "demo"
version = "0.1.0"

[dependencias]
matematica = "0.1.0"
util = { version = "0.2.0", ruta = "../util" }
"#;
        let m = parsear_manifiesto(texto).unwrap();
        assert_eq!(m.paquete.nombre, "demo");
        assert_eq!(m.paquete.version, "0.1.0");
        assert_eq!(m.dependencias.len(), 2);
        assert_eq!(m.dependencias[0].nombre, "matematica");
        assert_eq!(m.dependencias[0].version, "0.1.0");
        assert_eq!(m.dependencias[1].ruta.as_deref(), Some("../util"));
    }

    #[test]
    fn rechaza_dependencia_duplicada() {
        let texto = "[paquete]\nnombre = \"demo\"\n\n[dependencias]\na = \"1\"\na = \"2\"\n";
        let e = parsear_manifiesto(texto).unwrap_err();
        assert!(e.contains("duplicada"), "{e}");
    }

    #[test]
    fn rechaza_nombre_invalido() {
        let texto = "[paquete]\nnombre = \"demo mal\"\n";
        let e = parsear_manifiesto(texto).unwrap_err();
        assert!(e.contains("inválido"), "{e}");
    }

    #[test]
    fn rechaza_bloqueo_con_version_desconocida() {
        let texto = "version = 99\n";
        let e = parsear_bloqueo(texto).unwrap_err();
        assert!(e.contains("no soportada"), "{e}");
    }

    #[test]
    fn rechaza_bloqueo_sin_version() {
        let texto = "[[dependencia]]\nnombre = \"a\"\nversion = \"1\"\nruta = \"a.bsk\"\n";
        let e = parsear_bloqueo(texto).unwrap_err();
        assert!(e.contains("falta 'version'"), "{e}");
    }

    #[test]
    fn rechaza_dependencia_duplicada_en_bloqueo() {
        let texto = "version = 1\n\n[[dependencia]]\nnombre = \"a\"\nversion = \"1\"\nruta = \"a.bsk\"\n\n[[dependencia]]\nnombre = \"a\"\nversion = \"1\"\nruta = \"a.bsk\"\n";
        let e = parsear_bloqueo(texto).unwrap_err();
        assert!(e.contains("duplicada"), "{e}");
    }

    #[test]
    fn bloqueo_ida_y_vuelta() {
        let resueltas = vec![DependenciaResuelta {
            nombre: "matematica".to_string(),
            version: "0.1.0".to_string(),
            ruta: PathBuf::from("paquetes/matematica/lib.bsk"),
        }];
        let base = std::env::temp_dir().join(format!(
            "basilisco-bloqueo-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&base).unwrap();
        escribir_bloqueo(&base, &resueltas).unwrap();
        let leidas = leer_bloqueo(&base).unwrap().unwrap();
        assert_eq!(leidas, resueltas);
        fs::remove_dir_all(&base).ok();
    }
}
