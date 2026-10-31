//! Cliente HTTP mínimo para el registro de paquetes BASILISCO (R4).

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::paquetes::MetadatosPaquete;

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
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    let mut manifest_text = String::new();
    let mut entries: Vec<(String, u64)> = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().map(|s| s.to_string()).unwrap_or_default();
        if name == "BPK-MANIFEST.toml" {
            file.read_to_string(&mut manifest_text)
                .map_err(|e| e.to_string())?;
        } else if !name.ends_with('/') {
            let relative = name
                .strip_prefix("payload/")
                .ok_or_else(|| format!("archivo fuera de payload/: {name}"))?;
            if relative.is_empty() {
                return Err("ruta vacía en payload".to_string());
            }
            entries.push((relative.to_string(), file.size()));
        }
    }
    if manifest_text.is_empty() {
        return Err("falta BPK-MANIFEST.toml".to_string());
    }
    if !manifest_text.contains("[package]") {
        return Err("manifiesto BPK inválido".to_string());
    }
    if !manifest_text.contains("name =") || !manifest_text.contains("version =") {
        return Err("manifiesto BPK incompleto".to_string());
    }

    let mut declared: Vec<String> = Vec::new();
    for line in manifest_text.lines() {
        let line = line.trim();
        if line.starts_with("path =") {
            let val = line.trim_start_matches("path =").trim().trim_matches('"');
            declared.push(val.to_string());
        }
    }

    for (name, _) in &entries {
        if name.is_empty() || name.contains("..") || name.starts_with('/') || name.contains('\\') {
            return Err("ruta inválida en BPK".to_string());
        }
    }

    for d in &declared {
        if !entries.iter().any(|(n, _)| n == d) {
            return Err(format!("archivo declarado no encontrado: {}", d));
        }
    }

    for (name, _) in &entries {
        if !declared.iter().any(|d| d == name) {
            return Err(format!("archivo no declarado: {}", name));
        }
    }

    Ok(())
}

pub fn extraer_bpk(bytes: &[u8], destino: &Path) -> Result<PathBuf, String> {
    validar_bpk(bytes)?;
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;
    let out_dir = destino.to_path_buf();
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    // Límites contra ZIP bombs.
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
        let name = file.name().map(|s| s.to_string()).unwrap_or_default();
        if name.is_empty() || name.contains("..") || name.starts_with('/') || name.contains('\\') {
            return Err("ruta inválida en BPK".to_string());
        }
        if !seen.insert(name.clone()) {
            return Err("entrada duplicada en BPK".to_string());
        }
        if file.is_dir() && name == "payload/" {
            continue;
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
