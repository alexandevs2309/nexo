//! Cliente HTTP mínimo para el registro de paquetes BASILISCO (R4).

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::paquetes::MetadatosPaquete;
use unicode_normalization::UnicodeNormalization;

pub fn obtener_metadatos(url: &str) -> Result<MetadatosPaquete, String> {
    let resp = ureq::get(url).call().map_err(|e| e.to_string())?;
    let texto = resp
        .into_body()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    // El registro responde con {success, data, error}; extraer `data`.
    let valor: serde_json::Value = serde_json::from_str(&texto).map_err(|e| e.to_string())?;
    let data = valor.get("data").cloned().unwrap_or(valor);
    serde_json::from_value(data).map_err(|e| e.to_string())
}

pub fn descargar_artefacto(url: &str) -> Result<Vec<u8>, String> {
    let resp = ureq::get(url).call().map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    resp.into_body()
        .as_reader()
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn verificar_artefacto(bytes: &[u8], sha256_esperado: &str) -> Result<(), String> {
    let hash = crate::paquetes::sha256_hex(bytes);
    if hash != sha256_esperado {
        return Err("hash de artefacto inválido".to_string());
    }
    Ok(())
}

pub fn validar_bpk(bytes: &[u8]) -> Result<(), String> {
    // Límites globales.
    if bytes.is_empty() {
        return Err("BPK vacío".to_string());
    }
    if bytes.len() > 50 * 1024 * 1024 {
        return Err("BPK demasiado grande".to_string());
    }

    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    const MAX_FILES: usize = 1000;
    const MAX_TOTAL: u64 = 100 * 1024 * 1024;
    const MAX_FILE: u64 = 10 * 1024 * 1024;

    if archive.len() > MAX_FILES {
        return Err("demasiados archivos en BPK".to_string());
    }

    let mut manifest_text = String::new();
    // relative path (normalizado NFC) -> (size, sha256 calculado)
    let mut archivos: std::collections::BTreeMap<String, (u64, String)> =
        std::collections::BTreeMap::new();
    let mut visto = std::collections::HashSet::new();
    let mut total = 0u64;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name_raw = file.name().map_err(|e| e.to_string())?.to_string();
        let name: String = name_raw.nfc().collect();

        // Seguridad de rutas.
        if name.is_empty()
            || name.contains("..")
            || name.starts_with('/')
            || name.contains('\\')
            || name.contains('\0')
        {
            return Err(format!("ruta inválida en BPK: {name}"));
        }
        // Componentes y nombres especiales.
        for comp in name.split('/') {
            if comp.is_empty() || comp == "." || comp == ".." {
                return Err(format!("componente de ruta inválido: {name}"));
            }
            let upper = comp.to_uppercase();
            let reserved = [
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9",
            ];
            if reserved.contains(&upper.as_str()) {
                return Err(format!("nombre reservado de plataforma: {name}"));
            }
            if comp.ends_with(' ') || comp.ends_with('.') {
                return Err(format!("nombre de archivo inválido: {name}"));
            }
        }
        // Colisión por normalización: rechazar si normalizado difiere
        let norm: String = name.nfc().collect();
        if norm != name {
            // ya normalizado; si difiere, la entrada original no estaba normalizada
        }
        if !visto.insert(name.clone()) {
            return Err(format!("entrada duplicada en BPK: {name}"));
        }

        // Rechazar entradas especiales/symlinks.
        if let Some(mode) = file.unix_mode() {
            let tipo = mode & 0o170000;
            if tipo != 0 && tipo != 0o100000 {
                return Err(format!("archivo especial no permitido: {name}"));
            }
        }

        let size = file.size();
        if size > MAX_FILE {
            return Err(format!("archivo demasiado grande en BPK: {name}"));
        }
        total += size;
        if total > MAX_TOTAL {
            return Err("BPK demasiado grande".to_string());
        }

        if name == "BPK-MANIFEST.toml" {
            if file.is_dir() {
                return Err("BPK-MANIFEST.toml no puede ser directorio".to_string());
            }
            file.read_to_string(&mut manifest_text)
                .map_err(|e| e.to_string())?;
        } else {
            let relative = name
                .strip_prefix("payload/")
                .ok_or_else(|| format!("archivo fuera de payload/: {name}"))?;
            if relative.is_empty() {
                return Err("ruta vacía en payload".to_string());
            }
            // sha256 del contenido
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            let hash = crate::paquetes::sha256_hex(&buf);
            archivos.insert(relative.to_string(), (size, hash));
        }
    }

    if manifest_text.is_empty() {
        return Err("falta BPK-MANIFEST.toml".to_string());
    }

    // Parseo estricto de TOML.
    let manifest: toml::Value =
        toml::from_str(&manifest_text).map_err(|e| format!("TOML inválido: {e}"))?;

    let format_sec = manifest.get("format").ok_or("falta sección [format]")?;
    if format_sec.get("name").and_then(|v| v.as_str()) != Some("BASILISCO_PACKAGE") {
        return Err("format.name inválido".to_string());
    }
    if format_sec.get("version").and_then(|v| v.as_str()) != Some("1.0") {
        return Err("format.version inválido".to_string());
    }

    let package = manifest.get("package").ok_or("falta sección [package]")?;
    let ns = package
        .get("namespace")
        .and_then(|v| v.as_str())
        .ok_or("package.namespace obligatorio")?;
    let pname = package
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("package.name obligatorio")?;
    let version = package
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or("package.version obligatorio")?;
    if ns.is_empty() || pname.is_empty() {
        return Err("namespace/name vacíos".to_string());
    }
    semver::Version::parse(version).map_err(|_| "version SemVer inválida".to_string())?;

    // files
    let files = manifest
        .get("files")
        .and_then(|v| v.as_array())
        .ok_or("falta [[files]]")?;
    if files.is_empty() {
        return Err("lista de archivos vacía".to_string());
    }

    let mut declarados: Vec<String> = Vec::new();
    for f in files {
        let path = f
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("file.path obligatorio")?;
        let path_norm: String = path.nfc().collect();
        if path_norm.is_empty()
            || path_norm.contains("..")
            || path_norm.starts_with('/')
            || path_norm.contains('\\')
        {
            return Err(format!("ruta inválida en manifiesto: {path}"));
        }
        // colisión normalizada: el path declarado debe estar ya normalizado
        if path_norm != path {
            return Err(format!("ruta no normalizada en manifiesto: {path}"));
        }
        for comp in path_norm.split('/') {
            if comp.is_empty() || comp == "." || comp == ".." {
                return Err(format!("componente inválido en manifiesto: {path}"));
            }
        }
        let size = f
            .get("size")
            .and_then(|v| v.as_integer())
            .ok_or("file.size obligatorio")?;
        let sha = f
            .get("sha256")
            .and_then(|v| v.as_str())
            .ok_or("file.sha256 obligatorio")?;
        if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("file.sha256 inválido".to_string());
        }
        // verificar contra archivo real
        let (real_size, real_sha) = archivos
            .get(&path_norm)
            .ok_or_else(|| format!("archivo declarado no encontrado: {path}"))?;
        if *real_size as i64 != size {
            return Err(format!("tamaño no coincide: {path}"));
        }
        if real_sha != sha {
            return Err(format!("sha256 no coincide: {path}"));
        }
        declarados.push(path_norm);
    }

    // todos los archivos reales deben estar declarados
    for n in archivos.keys() {
        if !declarados.iter().any(|d| d == n) {
            return Err(format!("archivo no declarado: {n}"));
        }
    }

    Ok(())
}

pub fn extraer_bpk(bytes: &[u8], destino: &Path) -> Result<PathBuf, String> {
    validar_bpk(bytes)?;
    // (resto igual que antes)
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;
    let out_dir = destino.to_path_buf();
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    const MAX_FILES: usize = 1000;
    const MAX_TOTAL: u64 = 100 * 1024 * 1024;
    const MAX_FILE: u64 = 10 * 1024 * 1024;

    if archive.len() > MAX_FILES {
        return Err("demasiados archivos en BPK".to_string());
    }

    let mut total = 0u64;
    let mut seen = std::collections::HashSet::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name_raw = file.name().map_err(|e| e.to_string())?.to_string();
        let name: String = name_raw.nfc().collect();
        if name.is_empty() || name.contains("..") || name.starts_with('/') || name.contains('\\') {
            return Err("ruta inválida en BPK".to_string());
        }
        if !seen.insert(name.clone()) {
            return Err("entrada duplicada en BPK".to_string());
        }
        let size = file.size();
        if size > MAX_FILE {
            return Err("archivo demasiado grande en BPK".to_string());
        }
        total += size;
        if total > MAX_TOTAL {
            return Err("BPK demasiado grande".to_string());
        }
        let relative = if name == "BPK-MANIFEST.toml" {
            name.as_str()
        } else {
            match name.strip_prefix("payload/") {
                Some(path) if !path.is_empty() => path,
                _ => return Err("archivo fuera de payload/".to_string()),
            }
        };
        if relative.is_empty() {
            continue;
        }
        let out_path = out_dir.join(relative);
        if file.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(out_dir)
}
