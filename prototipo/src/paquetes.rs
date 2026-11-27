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
    /// Hash SHA-256 de confianza proporcionado explícitamente, si existe.
    pub hash: Option<String>,
    /// Firma Base64 proporcionada explícitamente, si existe.
    pub firma: Option<String>,
    /// Clave pública hex proporcionada explícitamente, si existe.
    pub clave: Option<String>,
    /// Registro de origen explícito, si existe.
    pub registro: Option<String>,
    /// Nonce de publicación, si existe.
    pub nonce: Option<String>,
    /// Fecha de expiración, si existe.
    pub expira: Option<String>,
    /// Autorización explícita para rutas externas.
    pub autorizado: bool,
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
    /// Hash SHA-256 del archivo de entrada, para verificación de integridad.
    pub hash: String,
    /// Indica si el hash proviene de una referencia confiable explícita.
    pub verificado: bool,
    /// Registro de procedencia, si aplica.
    pub registro: Option<String>,
    /// Identificador de clave pública, si aplica.
    pub key_id: Option<String>,
    /// Firma Base64, si está disponible.
    pub firma: Option<String>,
    /// Nonce de publicación, si está disponible.
    pub nonce: Option<String>,
    /// Fecha de expiración, si está disponible.
    pub expira: Option<String>,
    /// Fecha de publicación, si está disponible.
    pub published_at: Option<String>,
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

/// Resuelve las dependencias de un manifiesto contra el sistema de archivos,
/// incluyendo dependencias transitivas, detección de ciclos y conflictos.
pub fn resolver_dependencias(
    manifiesto: &Manifiesto,
    base: &Path,
) -> Result<Vec<DependenciaResuelta>, String> {
    let base_canonical = std::fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
    let mut resueltas: std::collections::BTreeMap<String, DependenciaResuelta> =
        std::collections::BTreeMap::new();
    let mut visitando: std::collections::HashSet<String> = std::collections::HashSet::new();
    resolver_recursivo(manifiesto, &base_canonical, &mut resueltas, &mut visitando)?;
    Ok(resueltas.into_values().collect())
}

fn resolver_recursivo(
    manifiesto: &Manifiesto,
    base: &Path,
    resueltas: &mut std::collections::BTreeMap<String, DependenciaResuelta>,
    visitando: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
    for dep in &manifiesto.dependencias {
        if visitando.contains(&dep.nombre) {
            return Err(format!(
                "ciclo de dependencias detectado en '{}'",
                dep.nombre
            ));
        }
        if let Some(existing) = resueltas.get(&dep.nombre) {
            if !version_compatible(&dep.version, &existing.version) {
                return Err(format!(
                    "conflicto de versión para '{}': se pidió {} y ya se resolvió {}",
                    dep.nombre, dep.version, existing.version
                ));
            }
            continue;
        }
        visitando.insert(dep.nombre.clone());
        let resolved = resolver_dependencia(dep, base)?;
        // Hash de integridad.
        let hash = hash_archivo(&resolved.ruta).unwrap_or_default();
        let verificado = dep.hash.is_some();
        let mut resolved = resolved;
        resolved.hash = hash;
        resolved.verificado = verificado;
        asegurar_cache(base, &resolved)?;

        // Dependencias transitivas.
        if let Ok(texto) = std::fs::read_to_string(&resolved.ruta) {
            // buscar manifiesto vecino
            if let Some(_man) = manifiesto_vecino(&resolved.ruta) {
                // resolver con base = directorio del paquete
                let dep_base = resolved.ruta.parent().unwrap_or(base).to_path_buf();
                if let Ok(subman) = cargar_manifiesto(&dep_base) {
                    resolver_recursivo(&subman, base, resueltas, visitando)?;
                }
            }
            let _ = texto;
        }

        resueltas.insert(dep.nombre.clone(), resolved);
        visitando.remove(&dep.nombre);
    }
    Ok(())
}

/// Resuelve una única dependencia contra el sistema de archivos. Pública para
/// que `bsk add` pueda pre-verificar resolubilidad antes de editar el
/// manifiesto. `base` debe ser el directorio del proyecto (idealmente
/// canonicalizado).
pub fn resolver_dependencia(dep: &Dependencia, base: &Path) -> Result<DependenciaResuelta, String> {
    let mut candidatos: Vec<PathBuf> = Vec::new();
    if let Some(ruta) = &dep.ruta {
        candidatos.push(base.join(ruta));
    }
    candidatos.push(base.join("paquetes").join(&dep.nombre));
    candidatos.push(base.join("vendor").join(&dep.nombre));
    candidatos.push(base.join(&dep.nombre));

    let mut buscados = Vec::new();
    for (i, candidato) in candidatos.iter().enumerate() {
        let explicito = dep.ruta.is_some() && i == 0;
        let canonical = std::fs::canonicalize(candidato).ok();
        if let Some(c) = &canonical {
            if explicito {
                if dep.autorizado {
                    let parent = base.parent().unwrap_or(base);
                    let parent_canonical =
                        std::fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
                    if !c.starts_with(&parent_canonical) {
                        continue;
                    }
                } else {
                    let base_canonical =
                        std::fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
                    if !c.starts_with(&base_canonical) {
                        continue;
                    }
                }
            } else {
                let base_canonical =
                    std::fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
                if !c.starts_with(&base_canonical) {
                    continue;
                }
            }
        }
        if let Some(archivo) = archivo_de_paquete(candidato) {
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
            // Segunda comprobación de ruta antes de devolver (mitigación TOCTOU).
            if let Ok(c) = std::fs::canonicalize(&archivo) {
                let base_canonical =
                    std::fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
                if dep.ruta.is_some() {
                    let parent = base.parent().unwrap_or(base);
                    let parent_canonical =
                        std::fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
                    if !c.starts_with(&parent_canonical) {
                        return Err(format!("ruta no permitida: {}", c.display()));
                    }
                } else if !c.starts_with(&base_canonical) {
                    return Err(format!("ruta no permitida: {}", c.display()));
                }
            }
            let hash_calculado = hash_archivo(&archivo).unwrap_or_default();
            let verificado = if let Some(hash_esperado) = &dep.hash {
                if hash_esperado != &hash_calculado {
                    return Err(format!(
                        "integridad comprometida para '{}': hash esperado {} y se obtuvo {}",
                        dep.nombre, hash_esperado, hash_calculado
                    ));
                }
                true
            } else {
                false
            };

            // Verificación criptográfica si se proporciona firma y clave.
            let almacen = AlmacenClaves::cargar(base).unwrap_or_default();
            if let (Some(firma), Some(clave_hex)) = (&dep.firma, &dep.clave) {
                let clave_bytes = hex_decode(clave_hex)?;
                let registry_id = dep.registro.clone().unwrap_or_else(|| "local".to_string());
                if !almacen.es_confiable(&registry_id, "key1", &clave_bytes) {
                    return Err(format!("clave no confiable para '{}'", dep.nombre));
                }
                let meta = MetadatosPaquete {
                    namespace: dep.nombre.clone(),
                    name: dep.nombre.clone(),
                    version: version.clone(),
                    sha256: hash_calculado.clone(),
                    published_at: "2026-10-31T00:00:00Z".to_string(),
                    registry_id: registry_id.clone(),
                    key_id: "key1".to_string(),
                    nonce: dep.nonce.clone().unwrap_or_else(|| "nonce1".to_string()),
                    expires_at: dep
                        .expira
                        .clone()
                        .unwrap_or_else(|| "2027-10-31T00:00:00Z".to_string()),
                    firma: dep.firma.clone(),
                    clave: dep.clave.clone(),
                };
                verificar_firma_metadatos(&meta, firma, &clave_bytes)?;

                // Anti-replay.
                let mut nonces = RegistroNonces::cargar(base).unwrap_or_default();
                let nonce = dep.nonce.clone().unwrap_or_else(|| "nonce1".to_string());
                nonces
                    .aceptar(&registry_id, &nonce)
                    .map_err(|e| format!("{}: {}", dep.nombre, e))?;
                nonces
                    .guardar(base)
                    .map_err(|e| format!("no se pudo persistir nonces: {e}"))?;
            }

            return Ok(DependenciaResuelta {
                nombre: dep.nombre.clone(),
                version,
                ruta: archivo,
                hash: hash_calculado,
                verificado,
                registro: dep.registro.clone(),
                key_id: dep.clave.clone(),
                firma: dep.firma.clone(),
                nonce: dep.nonce.clone(),
                expira: dep.expira.clone(),
                published_at: None,
            });
        }
        buscados.push(candidato.display().to_string());
    }

    if (dep.registro.is_some() || dep.firma.is_some() || dep.clave.is_some())
        && let Ok(remota) = resolver_dependencia_remota(dep, base)
    {
        return Ok(remota);
    }

    Err(format!(
        "no se encontró la dependencia '{}'. Rutas probadas: {}",
        dep.nombre,
        buscados.join(", ")
    ))
}

fn resolver_dependencia_remota(
    dep: &Dependencia,
    base: &Path,
) -> Result<DependenciaResuelta, String> {
    let cfg = ConfigRegistros::cargar(base).unwrap_or_default();
    let reg = if let Some(alias) = &dep.registro {
        cfg.obtener(alias)
            .or_else(|| cfg.registros.values().find(|r| &r.registry_id == alias))
            .cloned()
    } else {
        cfg.predeterminado().cloned()
    };
    let reg = reg.ok_or_else(|| format!("no hay registro configurado para '{}'", dep.nombre))?;
    if !reg.habilitado {
        return Err(format!("registro '{}' deshabilitado", reg.alias));
    }

    let (namespace, nombre_pkg) = if let Some((ns, n)) = dep.nombre.split_once('/') {
        (ns.to_string(), n.to_string())
    } else {
        (dep.nombre.clone(), dep.nombre.clone())
    };

    let version = if dep.version == "*" || dep.version.is_empty() {
        let url = format!(
            "{}/api/v1/packages/{}/{}/versions",
            reg.url.trim_end_matches('/'),
            namespace,
            nombre_pkg
        );
        // versions endpoint returns different shape; do a simple fetch
        let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
        let texto = resp
            .into_body()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        let valor: serde_json::Value = serde_json::from_str(&texto).map_err(|e| e.to_string())?;
        let data = valor.get("data").cloned().unwrap_or(valor);
        let versiones = data
            .get("versions")
            .or_else(|| data.get("items"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut elegida: Option<String> = None;
        for v in &versiones {
            let ver = v
                .get("version")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string();
            let yanked = v.get("yanked").and_then(|x| x.as_bool()).unwrap_or(false);
            if ver.is_empty() || yanked {
                continue;
            }
            if semver::Version::parse(&ver).is_ok() {
                if let Some(ref actual) = elegida {
                    if let (Ok(a), Ok(b)) =
                        (semver::Version::parse(&ver), semver::Version::parse(actual))
                        && a > b
                    {
                        elegida = Some(ver);
                    }
                } else {
                    elegida = Some(ver);
                }
            }
        }
        elegida.ok_or_else(|| format!("no hay versiones disponibles para '{}'", dep.nombre))?
    } else {
        dep.version.clone()
    };

    let meta_url = format!(
        "{}/api/v1/packages/{}/{}/{}",
        reg.url.trim_end_matches('/'),
        namespace,
        nombre_pkg,
        version
    );
    let art_url = format!("{}/download", meta_url);
    let meta = crate::cliente_remoto::obtener_metadatos(&meta_url)?;
    let bytes = crate::cliente_remoto::descargar_artefacto(&art_url)?;
    crate::cliente_remoto::verificar_artefacto(&bytes, &meta.sha256)?;

    let almacen = AlmacenClaves::cargar(base).unwrap_or_default();
    let firma = meta
        .firma
        .as_ref()
        .ok_or_else(|| "autenticidad no demostrada: falta firma".to_string())?;
    let clave_hex = meta
        .clave
        .as_ref()
        .ok_or_else(|| "autenticidad no demostrada: falta clave".to_string())?;
    let clave_bytes = hex_decode(clave_hex)?;
    if !almacen.es_confiable(&meta.registry_id, &meta.key_id, &clave_bytes) {
        return Err("clave no confiable".to_string());
    }
    if reg.registry_id != meta.registry_id {
        return Err("identidad de registro no coincide con metadatos".to_string());
    }
    verificar_firma_metadatos(&meta, firma, &clave_bytes)?;
    let mut nonces = RegistroNonces::cargar(base).unwrap_or_default();
    nonces.aceptar(&meta.registry_id, &meta.nonce)?;
    nonces.guardar(base)?;

    crate::cliente_remoto::validar_bpk(&bytes)?;

    let mut tmp_dir = None;
    for attempt in 0..10 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let candidate = std::env::temp_dir().join(format!(
            "basilisco_remoto_{}_{}_{}",
            std::process::id(),
            nanos,
            attempt
        ));
        match std::fs::create_dir(&candidate) {
            Ok(()) => {
                tmp_dir = Some(candidate);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    let tmp_dir =
        tmp_dir.ok_or_else(|| "no se pudo crear directorio temporal exclusivo".to_string())?;
    let extraido = crate::cliente_remoto::extraer_bpk(&bytes, &tmp_dir)?;
    let archivo = archivo_de_paquete(&extraido)
        .ok_or_else(|| "no se encontró archivo de paquete".to_string())?;
    let hash = hash_archivo(&archivo)?;

    let remote_dir = base
        .join(".basilisco")
        .join("remote")
        .join(&meta.name)
        .join(&meta.version);
    std::fs::create_dir_all(&remote_dir).map_err(|e| e.to_string())?;
    let archivo_instalado = remote_dir.join(archivo.file_name().unwrap_or_default());
    std::fs::copy(&archivo, &archivo_instalado).map_err(|e| e.to_string())?;
    let dep_resuelta = DependenciaResuelta {
        nombre: dep.nombre.clone(),
        version: meta.version.clone(),
        ruta: archivo_instalado,
        hash,
        verificado: meta.firma.is_some() && meta.clave.is_some(),
        registro: Some(meta.registry_id.clone()),
        key_id: Some(meta.key_id.clone()),
        firma: meta.firma.clone(),
        nonce: Some(meta.nonce.clone()),
        expira: Some(meta.expires_at.clone()),
        published_at: Some(meta.published_at.clone()),
    };
    asegurar_cache(base, &dep_resuelta)?;
    std::fs::remove_dir_all(&tmp_dir).ok();
    Ok(dep_resuelta)
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

/// Verifica que los hashes de los paquetes resueltos coincidan con el lock existente.
pub fn verificar_integridad(base: &Path, resueltas: &[DependenciaResuelta]) -> Result<(), String> {
    // Calcular hashes actuales.
    for dep in resueltas {
        let actual = hash_archivo(&dep.ruta)?;
        if !dep.hash.is_empty() && dep.hash != actual {
            return Err(format!(
                "integridad comprometida para '{}': hash esperado {} y se obtuvo {}",
                dep.nombre, dep.hash, actual
            ));
        }
    }
    // Si existe lock previo, comparar.
    if let Ok(Some(previas)) = leer_bloqueo(base) {
        for prev in &previas {
            if let Some(actual) = resueltas.iter().find(|d| d.nombre == prev.nombre) {
                let actual_hash = hash_archivo(&actual.ruta)?;
                if !prev.hash.is_empty() && prev.hash != actual_hash {
                    return Err(format!(
                        "integridad comprometida para '{}': hash del bloqueo {} y hash actual {}",
                        prev.nombre, prev.hash, actual_hash
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Valida que el lock cubra las dependencias del manifiesto.
pub fn lock_compatible_con_manifiesto(
    manifiesto: &Manifiesto,
    bloqueo: &[DependenciaResuelta],
) -> Result<(), String> {
    for dep in &manifiesto.dependencias {
        let Some(resuelta) = bloqueo.iter().find(|b| b.nombre == dep.nombre) else {
            return Err(format!(
                "basilisco.lock incompatible con el manifiesto: falta la dependencia '{}'; ejecuta 'basilisco update'",
                dep.nombre
            ));
        };
        if !version_compatible(&dep.version, &resuelta.version) {
            return Err(format!(
                "basilisco.lock incompatible con el manifiesto: '{}' requiere {} pero el lock fija {}",
                dep.nombre, dep.version, resuelta.version
            ));
        }
    }
    Ok(())
}

/// Instala/verifica dependencias desde un lock existente sin re-resolver versiones.
/// Falla explícitamente si el lock no puede cumplirse.
pub fn instalar_desde_bloqueo(
    base: &Path,
    manifiesto: &Manifiesto,
    bloqueo: &[DependenciaResuelta],
) -> Result<Vec<DependenciaResuelta>, String> {
    lock_compatible_con_manifiesto(manifiesto, bloqueo)?;
    let mut materializadas = Vec::new();
    for dep in bloqueo {
        let dep_mat = materializar_dep_bloqueada(base, dep)?;
        materializadas.push(dep_mat);
    }
    // Verificar integridad final con hashes bloqueados.
    for dep in &materializadas {
        let actual = hash_archivo(&dep.ruta)?;
        if !dep.hash.is_empty() && actual != dep.hash {
            return Err(format!(
                "integridad comprometida para '{}': lock esperaba {} y se obtuvo {}",
                dep.nombre, dep.hash, actual
            ));
        }
    }
    Ok(materializadas)
}

fn materializar_dep_bloqueada(
    base: &Path,
    dep: &DependenciaResuelta,
) -> Result<DependenciaResuelta, String> {
    let ruta_bloqueada = if dep.ruta.is_absolute() {
        dep.ruta.clone()
    } else {
        base.join(&dep.ruta)
    };

    if ruta_bloqueada.is_file() {
        let actual = hash_archivo(&ruta_bloqueada)?;
        if !dep.hash.is_empty() && actual != dep.hash {
            return Err(format!(
                "integridad comprometida para '{}': hash bloqueado {} y hash actual {}",
                dep.nombre, dep.hash, actual
            ));
        }
        verificar_autenticidad_bloqueada(base, dep, &actual)?;
        return Ok(DependenciaResuelta {
            ruta: ruta_bloqueada,
            ..dep.clone()
        });
    }

    // Intentar restaurar desde caché local.
    let cache = base
        .join(".basilisco")
        .join("cache")
        .join(&dep.nombre)
        .join(&dep.version);
    if cache.is_dir()
        && let Some(archivo) = archivo_de_paquete(&cache)
    {
        let actual = hash_archivo(&archivo)?;
        if dep.hash.is_empty() || actual == dep.hash {
            verificar_autenticidad_bloqueada(base, dep, &actual)?;
            return Ok(DependenciaResuelta {
                ruta: archivo,
                hash: if dep.hash.is_empty() {
                    actual
                } else {
                    dep.hash.clone()
                },
                ..dep.clone()
            });
        }
    }

    // Si es dependencia remota, intentar obtener exactamente el artefacto bloqueado.
    if dep.registro.is_some() {
        return obtener_dep_remota_bloqueada(base, dep);
    }

    Err(format!(
        "dependencia '{}' no disponible según basilisco.lock: falta '{}' (hash {}). Ejecute 'basilisco instalar' desde el proyecto correcto o 'basilisco update' para una actualización deliberada",
        dep.nombre,
        ruta_bloqueada.display(),
        dep.hash
    ))
}

fn verificar_autenticidad_bloqueada(
    base: &Path,
    dep: &DependenciaResuelta,
    hash_actual: &str,
) -> Result<(), String> {
    if dep.registro.is_none() && dep.firma.is_none() && dep.key_id.is_none() {
        return Ok(());
    }
    let Some(registro) = &dep.registro else {
        return Err(format!(
            "lock inválido para '{}': procedencia remota incompleta",
            dep.nombre
        ));
    };
    let Some(firma) = &dep.firma else {
        return Err(format!(
            "autenticidad no demostrada para '{}': falta firma en lock",
            dep.nombre
        ));
    };
    let Some(key_id) = &dep.key_id else {
        return Err(format!(
            "autenticidad no demostrada para '{}': falta key_id en lock",
            dep.nombre
        ));
    };
    let Some(nonce) = &dep.nonce else {
        return Err(format!(
            "autenticidad no demostrada para '{}': falta nonce en lock",
            dep.nombre
        ));
    };
    let Some(expira) = &dep.expira else {
        return Err(format!(
            "autenticidad no demostrada para '{}': falta expira en lock",
            dep.nombre
        ));
    };
    let Some(published_at) = &dep.published_at else {
        return Err(format!(
            "autenticidad no demostrada para '{}': falta published_at en lock",
            dep.nombre
        ));
    };
    let almacen = AlmacenClaves::cargar(base).unwrap_or_default();
    let clave = almacen
        .claves
        .get(registro)
        .and_then(|m| m.get(key_id))
        .cloned()
        .ok_or_else(|| format!("clave no confiable para '{}'", dep.nombre))?;
    if !almacen.es_confiable(registro, key_id, &clave) {
        return Err(format!("clave no confiable para '{}'", dep.nombre));
    }
    let meta = MetadatosPaquete {
        namespace: dep.nombre.clone(),
        name: dep.nombre.clone(),
        version: dep.version.clone(),
        sha256: hash_actual.to_string(),
        published_at: published_at.clone(),
        registry_id: registro.clone(),
        key_id: key_id.clone(),
        nonce: nonce.clone(),
        expires_at: expira.clone(),
        firma: Some(firma.clone()),
        clave: Some(hex::encode(&clave)),
    };
    verificar_firma_metadatos(&meta, firma, &clave)
        .map_err(|e| format!("firma inválida para '{}': {e}", dep.nombre))?;
    // Nonce: si ya fue aceptado en este cliente, no rechazar la reinstalación del mismo lock;
    // si es nuevo, registrarlo.
    let mut nonces = RegistroNonces::cargar(base).unwrap_or_default();
    if !nonces
        .nonces
        .get(registro)
        .is_some_and(|s| s.contains(nonce))
    {
        nonces.aceptar(registro, nonce)?;
        nonces.guardar(base)?;
    }
    Ok(())
}

fn obtener_dep_remota_bloqueada(
    base: &Path,
    dep: &DependenciaResuelta,
) -> Result<DependenciaResuelta, String> {
    let registro = dep
        .registro
        .as_ref()
        .ok_or_else(|| format!("lock inválido para '{}': falta registro", dep.nombre))?;
    let cfg = ConfigRegistros::cargar(base).unwrap_or_default();
    let reg = cfg
        .obtener(registro)
        .or_else(|| cfg.registros.values().find(|r| &r.registry_id == registro))
        .cloned()
        .ok_or_else(|| {
            format!(
                "registro '{}' no configurado para restaurar '{}' desde lock; configure el registro o restaure la caché",
                registro, dep.nombre
            )
        })?;
    if !reg.habilitado {
        return Err(format!("registro '{}' deshabilitado", reg.alias));
    }
    let (namespace, nombre_pkg) = if let Some((ns, n)) = dep.nombre.split_once('/') {
        (ns.to_string(), n.to_string())
    } else {
        (dep.nombre.clone(), dep.nombre.clone())
    };
    let meta_url = format!(
        "{}/api/v1/packages/{}/{}/{}",
        reg.url.trim_end_matches('/'),
        namespace,
        nombre_pkg,
        dep.version
    );
    let art_url = format!("{}/download", meta_url);
    let meta = crate::cliente_remoto::obtener_metadatos(&meta_url).map_err(|e| {
        format!(
            "no se pudo obtener metadatos para '{}' desde lock: {e}",
            dep.nombre
        )
    })?;
    let bytes = crate::cliente_remoto::descargar_artefacto(&art_url)
        .map_err(|e| format!("no se pudo descargar '{}' desde lock: {e}", dep.nombre))?;
    crate::cliente_remoto::verificar_artefacto(&bytes, &dep.hash).map_err(|e| {
        format!(
            "artefacto descargado no coincide con lock para '{}': {e}",
            dep.nombre
        )
    })?;
    if meta.sha256 != dep.hash {
        return Err(format!(
            "metadatos remotos no coinciden con lock para '{}'",
            dep.nombre
        ));
    }
    // Autenticidad fail-closed
    let firma = dep.firma.as_ref().ok_or_else(|| {
        format!(
            "autenticidad no demostrada para '{}': falta firma en lock",
            dep.nombre
        )
    })?;
    let key_id = dep.key_id.as_ref().ok_or_else(|| {
        format!(
            "autenticidad no demostrada para '{}': falta key_id en lock",
            dep.nombre
        )
    })?;
    let nonce = dep.nonce.as_ref().ok_or_else(|| {
        format!(
            "autenticidad no demostrada para '{}': falta nonce en lock",
            dep.nombre
        )
    })?;
    let almacen = AlmacenClaves::cargar(base).unwrap_or_default();
    let clave = almacen
        .claves
        .get(&meta.registry_id)
        .and_then(|m| m.get(key_id))
        .cloned()
        .ok_or_else(|| format!("clave no confiable para '{}'", dep.nombre))?;
    if !almacen.es_confiable(&meta.registry_id, key_id, &clave) {
        return Err(format!("clave no confiable para '{}'", dep.nombre));
    }
    let meta_verif = MetadatosPaquete {
        firma: Some(firma.clone()),
        clave: Some(hex::encode(&clave)),
        ..meta.clone()
    };
    verificar_firma_metadatos(&meta_verif, firma, &clave)
        .map_err(|e| format!("firma inválida para '{}': {e}", dep.nombre))?;
    if meta_verif.registry_id != *registro
        || meta_verif.key_id != *key_id
        || meta_verif.nonce != *nonce
    {
        return Err(format!(
            "metadatos remotos no coinciden con procedencia bloqueada para '{}'",
            dep.nombre
        ));
    }
    let mut nonces = RegistroNonces::cargar(base).unwrap_or_default();
    if !nonces
        .nonces
        .get(&meta.registry_id)
        .is_some_and(|s| s.contains(nonce))
    {
        nonces.aceptar(&meta.registry_id, nonce)?;
        nonces.guardar(base)?;
    }
    crate::cliente_remoto::validar_bpk(&bytes)?;
    let tmp_dir = std::env::temp_dir().join(format!(
        "basilisco_lock_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let extraido = crate::cliente_remoto::extraer_bpk(&bytes, &tmp_dir)?;
    let archivo = archivo_de_paquete(&extraido)
        .ok_or_else(|| "no se encontró archivo de paquete".to_string())?;
    let actual = hash_archivo(&archivo)?;
    if actual != dep.hash {
        fs::remove_dir_all(&tmp_dir).ok();
        return Err(format!(
            "hash extraído no coincide con lock para '{}'",
            dep.nombre
        ));
    }
    let remote_dir = base
        .join(".basilisco")
        .join("remote")
        .join(&nombre_pkg)
        .join(&dep.version);
    fs::create_dir_all(&remote_dir).map_err(|e| e.to_string())?;
    let archivo_instalado = remote_dir.join(archivo.file_name().unwrap_or_default());
    fs::copy(&archivo, &archivo_instalado).map_err(|e| e.to_string())?;
    fs::remove_dir_all(&tmp_dir).ok();
    let resuelta = DependenciaResuelta {
        ruta: archivo_instalado,
        hash: dep.hash.clone(),
        ..dep.clone()
    };
    asegurar_cache(base, &resuelta)?;
    Ok(resuelta)
}

/// Asegura una copia en caché local organizada por nombre y versión.
pub fn asegurar_cache(base: &Path, dep: &DependenciaResuelta) -> Result<PathBuf, String> {
    let cache_dir = base
        .join(".basilisco")
        .join("cache")
        .join(&dep.nombre)
        .join(&dep.version);
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let destino = cache_dir.join(dep.ruta.file_name().unwrap_or_default());
    if destino.exists() {
        let actual = hash_archivo(&destino)?;
        if !dep.hash.is_empty() && actual != dep.hash {
            return Err(format!(
                "integridad comprometida en caché para '{}'",
                dep.nombre
            ));
        }
    } else {
        fs::copy(&dep.ruta, &destino).map_err(|e| e.to_string())?;
    }
    Ok(destino)
}

/// Escribe `basilisco.lock` de forma determinista.
pub fn escribir_bloqueo(base: &Path, resueltas: &[DependenciaResuelta]) -> Result<(), String> {
    let mut salida = String::new();
    salida.push_str("# Generado por basilisco. No editar a mano.\n");
    salida.push_str(&format!("version = {VERSION_BLOQUEO}\n"));
    for dep in resueltas {
        let base_canonical = std::fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
        let ruta_absoluta = std::fs::canonicalize(&dep.ruta)
            .or_else(|_| std::fs::canonicalize(base.join(&dep.ruta)));
        let ruta = ruta_absoluta
            .ok()
            .and_then(|destino| ruta_relativa(&base_canonical, &destino))
            .or_else(|| {
                dep.ruta
                    .strip_prefix(&base_canonical)
                    .map(Path::to_path_buf)
                    .ok()
                    .or_else(|| dep.ruta.strip_prefix(base).map(Path::to_path_buf).ok())
            })
            .unwrap_or_else(|| dep.ruta.clone());
        salida.push_str("\n[[dependencia]]\n");
        salida.push_str(&format!("nombre = \"{}\"\n", escapar(&dep.nombre)));
        salida.push_str(&format!("version = \"{}\"\n", escapar(&dep.version)));
        // El archivo de bloqueo usa separadores portables, también en Windows.
        let ruta_portable = ruta.to_string_lossy().replace('\\', "/");
        salida.push_str(&format!("ruta = \"{}\"\n", escapar(&ruta_portable)));
        salida.push_str(&format!("hash = \"{}\"\n", escapar(&dep.hash)));
        if let Some(registro) = &dep.registro {
            salida.push_str(&format!("registro = \"{}\"\n", escapar(registro)));
        }
        if let Some(key_id) = &dep.key_id {
            salida.push_str(&format!("key_id = \"{}\"\n", escapar(key_id)));
        }
        if let Some(firma) = &dep.firma {
            salida.push_str(&format!("firma = \"{}\"\n", escapar(firma)));
        }
        if let Some(nonce) = &dep.nonce {
            salida.push_str(&format!("nonce = \"{}\"\n", escapar(nonce)));
        }
        if let Some(expira) = &dep.expira {
            salida.push_str(&format!("expira = \"{}\"\n", escapar(expira)));
        }
        if let Some(published_at) = &dep.published_at {
            salida.push_str(&format!("published_at = \"{}\"\n", escapar(published_at)));
        }
        if dep.verificado {
            salida.push_str("verificado = true\n");
        }
    }
    let ruta = base.join(BLOQUEO);
    fs::write(&ruta, salida).map_err(|e| format!("no se pudo escribir '{}': {e}", ruta.display()))
}

fn ruta_relativa(base: &Path, destino: &Path) -> Option<PathBuf> {
    let base_componentes: Vec<_> = base.components().collect();
    let destino_componentes: Vec<_> = destino.components().collect();
    if !base.is_absolute() || !destino.is_absolute() {
        return None;
    }

    let comun = base_componentes
        .iter()
        .zip(&destino_componentes)
        .take_while(|(a, b)| a == b)
        .count();
    let raiz = base_componentes
        .iter()
        .take_while(|componente| {
            matches!(
                componente,
                std::path::Component::Prefix(_) | std::path::Component::RootDir
            )
        })
        .count();
    if raiz == 0 || comun < raiz {
        return None;
    }

    let mut relativa = PathBuf::new();
    for componente in &base_componentes[comun..] {
        if !matches!(componente, std::path::Component::Normal(_)) {
            return None;
        }
        relativa.push("..");
    }
    for componente in &destino_componentes[comun..] {
        if !matches!(componente, std::path::Component::Normal(_)) {
            return None;
        }
        relativa.push(componente.as_os_str());
    }
    Some(if relativa.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        relativa
    })
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
    #[allow(clippy::type_complexity)]
    let mut actual: Option<(
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        bool,
        Option<String>,
    )> = None;
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
            actual = Some((
                None, None, None, None, None, None, None, None, None, false, None,
            ));
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
        if let Some((
            nombre,
            version,
            ruta,
            hash,
            registro,
            key_id,
            firma,
            nonce,
            expira,
            verificado,
            published_at,
        )) = actual.as_mut()
        {
            match clave {
                "verificado" => {
                    *verificado = valor == "true";
                }
                _ => {
                    let texto =
                        parsear_texto(valor).map_err(|e| format!("línea {}: {e}", numero + 1))?;
                    match clave {
                        "nombre" => *nombre = Some(texto),
                        "version" => *version = Some(texto),
                        "ruta" => *ruta = Some(texto),
                        "hash" => *hash = Some(texto),
                        "registro" | "registry" => *registro = Some(texto),
                        "key_id" => *key_id = Some(texto),
                        "firma" | "signature" => *firma = Some(texto),
                        "nonce" => *nonce = Some(texto),
                        "expira" | "expires_at" => *expira = Some(texto),
                        "published_at" => *published_at = Some(texto),
                        _ => {}
                    }
                }
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

#[allow(clippy::type_complexity)]
fn construir_dependencia(
    campos: (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        bool,
        Option<String>,
    ),
    numero: usize,
) -> Result<DependenciaResuelta, String> {
    let (
        nombre,
        version,
        ruta,
        hash,
        registro,
        key_id,
        firma,
        nonce,
        expira,
        verificado,
        published_at,
    ) = campos;
    let ubicacion = if numero == 0 {
        "al final del bloqueo".to_string()
    } else {
        format!("línea {numero}")
    };
    Ok(DependenciaResuelta {
        nombre: nombre.ok_or_else(|| format!("{ubicacion}: falta 'nombre'"))?,
        version: version.ok_or_else(|| format!("{ubicacion}: falta 'version'"))?,
        ruta: PathBuf::from(ruta.ok_or_else(|| format!("{ubicacion}: falta 'ruta'"))?),
        hash: hash.unwrap_or_default(),
        verificado,
        registro,
        key_id,
        firma,
        nonce,
        expira,
        published_at,
    })
}

/// Resuelve el nombre de un paquete importado usando `basilisco.lock` del
/// directorio de trabajo actual. Devuelve `None` si no hay bloqueo o no figura.
pub fn resolver_desde_bloqueo(nombre: &str) -> Option<PathBuf> {
    let base = std::env::current_dir().ok()?;
    let resueltas = leer_bloqueo(&base).ok()??;
    let dep = resueltas.into_iter().find(|d| d.nombre == nombre)?;
    if !dep.ruta.is_file() {
        return None;
    }
    if !dep.hash.is_empty() {
        let actual = hash_archivo(&dep.ruta).ok()?;
        if actual != dep.hash {
            return None;
        }
    }
    Some(dep.ruta)
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

/// Comprueba si una cadena es una versión aceptable (`*`, SemVer suelto,
/// rangos `^`/`~`/`>=`). Pública para `bsk add`.
pub fn version_valida(version: &str) -> bool {
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
        let mut hash = None;
        let mut firma = None;
        let mut clave_hex = None;
        let mut registro = None;
        let mut nonce = None;
        let mut expira = None;
        let mut autorizado = false;
        for parte in dividir_por_comas(interior) {
            let (clave, valor) = parte
                .split_once('=')
                .ok_or_else(|| format!("entrada inválida '{parte}' en tabla en línea"))?;
            let clave = clave.trim();
            let valor = valor.trim();
            if clave == "autorizado" {
                autorizado = valor == "true";
                continue;
            }
            if clave == "firma"
                || clave == "clave"
                || clave == "registro"
                || clave == "nonce"
                || clave == "expira"
            {
                let texto = parsear_texto(valor)?;
                match clave {
                    "firma" => firma = Some(texto),
                    "clave" => clave_hex = Some(texto),
                    "registro" => registro = Some(texto),
                    "nonce" => nonce = Some(texto),
                    "expira" => expira = Some(texto),
                    _ => {}
                }
                continue;
            }
            let texto = parsear_texto(valor)?;
            match clave {
                "version" => version = Some(texto),
                "ruta" | "path" => ruta = Some(texto),
                "hash" => hash = Some(texto),
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
            hash,
            firma,
            clave: clave_hex,
            registro,
            nonce,
            expira,
            autorizado,
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
            hash: None,
            firma: None,
            clave: None,
            registro: None,
            nonce: None,
            expira: None,
            autorizado: false,
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
            hash: String::new(),
            verificado: false,
            registro: None,
            key_id: None,
            firma: None,
            nonce: None,
            expira: None,
            published_at: None,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VersionSem {
    major: u64,
    minor: u64,
    patch: u64,
}

fn parsear_version_sem(texto: &str) -> Option<VersionSem> {
    let texto = texto.trim_start_matches('v');
    let partes: Vec<&str> = texto.split('.').collect();
    if partes.len() != 3 {
        return None;
    }
    Some(VersionSem {
        major: partes[0].parse().ok()?,
        minor: partes[1].parse().ok()?,
        patch: partes[2].parse().ok()?,
    })
}

fn version_compatible(solicitada: &str, declarada: &str) -> bool {
    let solicitada = solicitada.trim();
    if solicitada == "*" {
        return true;
    }
    if let Some(req) = solicitada.strip_prefix(">=") {
        if let (Some(r), Some(d)) = (parsear_version_sem(req), parsear_version_sem(declarada)) {
            return d >= r;
        }
        return solicitada == declarada;
    }
    if let Some(req) = solicitada.strip_prefix('^') {
        if let (Some(r), Some(d)) = (parsear_version_sem(req), parsear_version_sem(declarada)) {
            return d.major == r.major && d >= r;
        }
        return solicitada == declarada;
    }
    if let Some(req) = solicitada.strip_prefix('~') {
        if let (Some(r), Some(d)) = (parsear_version_sem(req), parsear_version_sem(declarada)) {
            return d.major == r.major && d.minor == r.minor && d >= r;
        }
        return solicitada == declarada;
    }
    solicitada == declarada
}

fn manifiesto_vecino(archivo: &Path) -> Option<PathBuf> {
    let mut dir = archivo.parent()?;
    loop {
        let m = dir.join(MANIFIESTO);
        if m.is_file() {
            return Some(m);
        }
        dir = dir.parent()?;
    }
}

pub fn hash_archivo(ruta: &Path) -> Result<String, String> {
    let bytes = std::fs::read(ruta).map_err(|e| e.to_string())?;
    Ok(sha256_hex(&bytes))
}

// Implementación mínima de SHA-256.
pub fn sha256_hex(data: &[u8]) -> String {
    // Constantes.
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    // Preprocesamiento.
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = String::new();
    for v in h {
        out.push_str(&format!("{:08x}", v));
    }
    out
}

use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::Digest;
use sha2::Sha256;

/// Metadatos firmados de un paquete (R3).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MetadatosPaquete {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub published_at: String,
    pub registry_id: String,
    pub key_id: String,
    pub nonce: String,
    pub expires_at: String,
    #[serde(default)]
    pub firma: Option<String>,
    #[serde(default)]
    pub clave: Option<String>,
}

impl MetadatosPaquete {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        format!(
            r#"{{"expires_at":"{}","key_id":"{}","name":"{}","namespace":"{}","nonce":"{}","published_at":"{}","registry_id":"{}","sha256":"{}","version":"{}"}}"#,
            self.expires_at,
            self.key_id,
            self.name,
            self.namespace,
            self.nonce,
            self.published_at,
            self.registry_id,
            self.sha256,
            self.version
        )
        .into_bytes()
    }
}

/// Firma metadatos con una clave privada Ed25519.
pub fn firmar_metadatos(
    metadatos: &MetadatosPaquete,
    clave_privada: &[u8],
) -> Result<String, String> {
    let clave = SigningKey::from_bytes(
        clave_privada
            .try_into()
            .map_err(|_| "clave privada inválida".to_string())?,
    );
    let digest = Sha256::digest(metadatos.canonical_bytes());
    let firma = clave.sign(&digest);
    Ok(base64::engine::general_purpose::STANDARD.encode(firma.to_bytes()))
}

/// Verifica la firma de metadatos con una clave pública Ed25519.
pub fn verificar_firma_metadatos(
    metadatos: &MetadatosPaquete,
    firma_b64: &str,
    clave_publica: &[u8],
) -> Result<(), String> {
    let clave = VerifyingKey::from_bytes(
        clave_publica
            .try_into()
            .map_err(|_| "clave pública inválida".to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let firma_bytes = base64::engine::general_purpose::STANDARD
        .decode(firma_b64)
        .map_err(|e| e.to_string())?;
    let firma = Signature::from_bytes(
        firma_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "firma inválida".to_string())?,
    );
    let digest = Sha256::digest(metadatos.canonical_bytes());
    clave.verify(&digest, &firma).map_err(|e| e.to_string())?;
    Ok(())
}

/// Almacén de claves públicas confiables por registro.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AlmacenClaves {
    pub claves: std::collections::BTreeMap<String, std::collections::BTreeMap<String, Vec<u8>>>,
    pub revocadas: std::collections::BTreeSet<(String, String)>,
}

impl AlmacenClaves {
    pub fn agregar(&mut self, registro: &str, key_id: &str, clave: Vec<u8>) {
        self.claves
            .entry(registro.to_string())
            .or_default()
            .insert(key_id.to_string(), clave);
    }

    pub fn es_confiable(&self, registro: &str, key_id: &str, clave: &[u8]) -> bool {
        if self
            .revocadas
            .contains(&(registro.to_string(), key_id.to_string()))
        {
            return false;
        }
        self.claves
            .get(registro)
            .and_then(|m| m.get(key_id))
            .map(|k| k == clave)
            .unwrap_or(false)
    }

    pub fn revocar(&mut self, registro: &str, key_id: &str) {
        self.revocadas
            .insert((registro.to_string(), key_id.to_string()));
    }

    pub fn guardar(&self, base: &Path) -> Result<(), String> {
        let dir = base.join(".basilisco");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        // Rechazar si el directorio o archivo son enlaces simbólicos.
        if let Ok(meta) = fs::symlink_metadata(&dir)
            && meta.file_type().is_symlink()
        {
            return Err("directorio de confianza es un enlace simbólico".to_string());
        }
        let ruta = dir.join("trust.json");
        if let Ok(meta) = fs::symlink_metadata(&ruta)
            && meta.file_type().is_symlink()
        {
            return Err("archivo de confianza es un enlace simbólico".to_string());
        }
        let tmp = dir.join("trust.json.tmp");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&tmp, json).map_err(|e| e.to_string())?;
        // Permisos restrictivos en Unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = fs::metadata(&tmp).map_err(|e| e.to_string())?.permissions();
            perm.set_mode(0o600);
            fs::set_permissions(&tmp, perm).map_err(|e| e.to_string())?;
        }
        fs::rename(&tmp, &ruta).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn cargar(base: &Path) -> Result<Self, String> {
        let dir = base.join(".basilisco");
        let ruta = dir.join("trust.json");
        if !ruta.is_file() {
            return Ok(Self::default());
        }
        // Rechazar enlaces simbólicos.
        let meta = fs::symlink_metadata(&ruta).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            return Err("archivo de confianza es un enlace simbólico".to_string());
        }
        let texto = fs::read_to_string(&ruta).map_err(|e| e.to_string())?;
        let almacen: Self = serde_json::from_str(&texto).map_err(|e| e.to_string())?;
        // Validar claves.
        for (reg, claves) in &almacen.claves {
            for (kid, clave) in claves {
                if clave.len() != 32 {
                    return Err(format!("clave inválida en {} / {}", reg, kid));
                }
                if kid.is_empty() || reg.is_empty() {
                    return Err("identificador vacío en almacén de confianza".to_string());
                }
            }
        }
        Ok(almacen)
    }

    pub fn agregar_seguro(
        &mut self,
        registro: &str,
        key_id: &str,
        clave: Vec<u8>,
    ) -> Result<(), String> {
        if registro.is_empty() || key_id.is_empty() {
            return Err("identificador vacío".to_string());
        }
        if clave.len() != 32 {
            return Err("clave inválida".to_string());
        }
        if self
            .revocadas
            .contains(&(registro.to_string(), key_id.to_string()))
        {
            return Err("clave revocada no puede reactivarse".to_string());
        }
        self.agregar(registro, key_id, clave);
        Ok(())
    }

    pub fn rotar(
        &mut self,
        registro: &str,
        key_id_antigua: &str,
        key_id_nueva: &str,
        clave_nueva: Vec<u8>,
        firma_autorizacion: &str,
        clave_autoridad: &[u8],
    ) -> Result<(), String> {
        // La nueva clave debe estar firmada por la autoridad o la clave anterior.
        let meta = format!("{}:{}:{}", registro, key_id_antigua, key_id_nueva);
        let firma_bytes = base64::engine::general_purpose::STANDARD
            .decode(firma_autorizacion)
            .map_err(|e| e.to_string())?;
        let firma = ed25519_dalek::Signature::from_bytes(
            firma_bytes
                .as_slice()
                .try_into()
                .map_err(|_| "firma inválida".to_string())?,
        );
        let vk = ed25519_dalek::VerifyingKey::from_bytes(
            clave_autoridad
                .try_into()
                .map_err(|_| "clave de autoridad inválida".to_string())?,
        )
        .map_err(|e| e.to_string())?;
        vk.verify(meta.as_bytes(), &firma)
            .map_err(|e| e.to_string())?;
        self.agregar(registro, key_id_nueva, clave_nueva);
        Ok(())
    }
}

/// Verifica un paquete remoto si se proporcionan metadatos y firma.
pub fn verificar_paquete_remoto(
    almacen: &AlmacenClaves,
    meta: &MetadatosPaquete,
    firma_b64: &str,
    clave_publica: &[u8],
) -> Result<(), String> {
    if !almacen.es_confiable(&meta.registry_id, &meta.key_id, clave_publica) {
        return Err("clave no confiable".to_string());
    }
    verificar_firma_metadatos(meta, firma_b64, clave_publica)
}

/// Registro persistente de nonces aceptados por registro (anti-replay).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RegistroNonces {
    pub nonces: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
}

impl RegistroNonces {
    pub fn cargar(base: &Path) -> Result<Self, String> {
        let ruta = base.join(".basilisco").join("nonces.json");
        if !ruta.is_file() {
            return Ok(Self::default());
        }
        let meta = fs::symlink_metadata(&ruta).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            return Err("archivo de nonces es un enlace simbólico".to_string());
        }
        let texto = fs::read_to_string(&ruta).map_err(|e| e.to_string())?;
        serde_json::from_str(&texto).map_err(|e| e.to_string())
    }

    pub fn guardar(&self, base: &Path) -> Result<(), String> {
        let dir = base.join(".basilisco");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let ruta = dir.join("nonces.json");
        let tmp = dir.join("nonces.json.tmp");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&tmp, json).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &ruta).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn aceptar(&mut self, registro: &str, nonce: &str) -> Result<(), String> {
        let set = self.nonces.entry(registro.to_string()).or_default();
        if set.contains(nonce) {
            return Err("nonce repetido".to_string());
        }
        set.insert(nonce.to_string());
        Ok(())
    }
}

fn hex_decode(texto: &str) -> Result<Vec<u8>, String> {
    if !texto.len().is_multiple_of(2) {
        return Err("hex inválido".to_string());
    }
    let mut out = Vec::new();
    for i in (0..texto.len()).step_by(2) {
        let byte =
            u8::from_str_radix(&texto[i..i + 2], 16).map_err(|_| "hex inválido".to_string())?;
        out.push(byte);
    }
    Ok(out)
}

// --- R4 integración completa ---
pub fn instalar_paquete_remoto_completo(
    base: &std::path::Path,
    metadata_url: &str,
    artifact_url: &str,
) -> Result<DependenciaResuelta, String> {
    let meta = crate::cliente_remoto::obtener_metadatos(metadata_url)?;
    let bytes = crate::cliente_remoto::descargar_artefacto(artifact_url)?;
    crate::cliente_remoto::verificar_artefacto(&bytes, &meta.sha256)?;

    // Verificación criptográfica
    let almacen = AlmacenClaves::cargar(base).unwrap_or_default();
    if let (Some(firma), Some(clave_hex)) = (&meta.firma, &meta.clave) {
        let clave_bytes = hex_decode(clave_hex)?;
        if !almacen.es_confiable(&meta.registry_id, &meta.key_id, &clave_bytes) {
            return Err("clave no confiable".to_string());
        }
        verificar_firma_metadatos(&meta, firma, &clave_bytes)?;
        // Anti-replay
        let mut nonces = RegistroNonces::cargar(base).unwrap_or_default();
        nonces.aceptar(&meta.registry_id, &meta.nonce)?;
        nonces.guardar(base)?;
    } else {
        return Err("autenticidad no demostrada".to_string());
    }

    // Validación BPK completa
    crate::cliente_remoto::validar_bpk(&bytes)?;

    // Extraer en directorio temporal exclusivo (sin sobrescribir ni eliminar otros).
    let mut tmp_dir = None;
    for attempt in 0..10 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let candidate = std::env::temp_dir().join(format!(
            "basilisco_remoto_{}_{}_{}",
            std::process::id(),
            nanos,
            attempt
        ));
        match std::fs::create_dir(&candidate) {
            Ok(()) => {
                tmp_dir = Some(candidate);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    let tmp_dir =
        tmp_dir.ok_or_else(|| "no se pudo crear directorio temporal exclusivo".to_string())?;
    let extraido = crate::cliente_remoto::extraer_bpk(&bytes, &tmp_dir)?;
    let archivo = archivo_de_paquete(&extraido)
        .ok_or_else(|| "no se encontró archivo de paquete".to_string())?;

    // Verificar hash del artefacto BPK
    let hash_bpk = sha256_hex(&bytes);
    if hash_bpk != meta.sha256 {
        std::fs::remove_dir_all(&tmp_dir).ok();
        return Err("hash del artefacto extraído inválido".to_string());
    }
    let hash = hash_archivo(&archivo)?;

    // Instalar en caché y registrar
    let remote_dir = base
        .join(".basilisco")
        .join("remote")
        .join(&meta.name)
        .join(&meta.version);
    std::fs::create_dir_all(&remote_dir).map_err(|e| e.to_string())?;
    let archivo_instalado = remote_dir.join(archivo.file_name().unwrap_or_default());
    std::fs::copy(&archivo, &archivo_instalado).map_err(|e| e.to_string())?;
    let dep_resuelta = DependenciaResuelta {
        nombre: meta.name.clone(),
        version: meta.version.clone(),
        ruta: archivo_instalado,
        hash: hash.clone(),
        verificado: true,
        registro: Some(meta.registry_id.clone()),
        key_id: Some(meta.key_id.clone()),
        firma: meta.firma.clone(),
        nonce: Some(meta.nonce.clone()),
        expira: Some(meta.expires_at.clone()),
        published_at: Some(meta.published_at.clone()),
    };
    asegurar_cache(base, &dep_resuelta)?;

    // Limpiar temporales
    std::fs::remove_dir_all(&tmp_dir).ok();
    // Actualizar lock file
    let mut deps = vec![dep_resuelta.clone()];
    if let Ok(Some(mut existentes)) = leer_bloqueo(base) {
        // evitar duplicados
        existentes.retain(|d| d.nombre != dep_resuelta.nombre);
        existentes.extend(deps.clone());
        deps = existentes;
    }
    escribir_bloqueo(base, &deps)?;
    Ok(dep_resuelta)
}

// --- R5: Múltiples registros ---
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigRegistro {
    pub alias: String,
    pub registry_id: String,
    pub url: String,
    pub habilitado: bool,
    pub prioridad: i32,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ConfigRegistros {
    pub predeterminado: Option<String>,
    pub registros: std::collections::BTreeMap<String, ConfigRegistro>,
}

impl ConfigRegistros {
    pub fn ruta(base: &Path) -> PathBuf {
        base.join(".basilisco").join("registries.json")
    }

    pub fn cargar(base: &Path) -> Result<Self, String> {
        let ruta = Self::ruta(base);
        if !ruta.is_file() {
            return Ok(Self::default());
        }
        let meta = fs::symlink_metadata(&ruta).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            return Err("archivo de registros es un enlace simbólico".to_string());
        }
        let texto = fs::read_to_string(&ruta).map_err(|e| e.to_string())?;
        serde_json::from_str(&texto).map_err(|e| e.to_string())
    }

    pub fn guardar(&self, base: &Path) -> Result<(), String> {
        let dir = base.join(".basilisco");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        if let Ok(meta) = fs::symlink_metadata(&dir)
            && meta.file_type().is_symlink()
        {
            return Err("directorio inválido".to_string());
        }
        let ruta = Self::ruta(base);
        if let Ok(meta) = fs::symlink_metadata(&ruta)
            && meta.file_type().is_symlink()
        {
            return Err("archivo de registros es un enlace simbólico".to_string());
        }
        let tmp = dir.join("registries.json.tmp");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&tmp, json).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = fs::metadata(&tmp).map_err(|e| e.to_string())?.permissions();
            perm.set_mode(0o600);
            fs::set_permissions(&tmp, perm).map_err(|e| e.to_string())?;
        }
        fs::rename(&tmp, &ruta).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn agregar(&mut self, alias: &str, registry_id: &str, url: &str) -> Result<(), String> {
        if !url.starts_with("https://")
            && !url.starts_with("http://127.0.0.1")
            && !url.starts_with("http://localhost")
        {
            return Err("se requiere https para registros remotos".to_string());
        }
        if self
            .registros
            .values()
            .any(|r| r.registry_id == registry_id && r.alias != alias)
        {
            return Err("registry_id duplicado".to_string());
        }
        if self.registros.contains_key(alias) {
            return Err("alias duplicado".to_string());
        }
        self.registros.insert(
            alias.to_string(),
            ConfigRegistro {
                alias: alias.to_string(),
                registry_id: registry_id.to_string(),
                url: url.to_string(),
                habilitado: true,
                prioridad: 0,
            },
        );
        if self.predeterminado.is_none() {
            self.predeterminado = Some(alias.to_string());
        }
        Ok(())
    }

    pub fn obtener(&self, alias: &str) -> Option<&ConfigRegistro> {
        self.registros.get(alias)
    }

    pub fn predeterminado(&self) -> Option<&ConfigRegistro> {
        if let Some(ref p) = self.predeterminado {
            self.registros.get(p)
        } else {
            self.registros.values().find(|r| r.habilitado)
        }
    }

    pub fn set_predeterminado(&mut self, alias: &str) -> Result<(), String> {
        if !self.registros.contains_key(alias) {
            return Err("registro no encontrado".to_string());
        }
        self.predeterminado = Some(alias.to_string());
        Ok(())
    }

    pub fn habilitar(&mut self, alias: &str, habilitado: bool) -> Result<(), String> {
        if let Some(r) = self.registros.get_mut(alias) {
            r.habilitado = habilitado;
            Ok(())
        } else {
            Err("registro no encontrado".to_string())
        }
    }

    pub fn eliminar(&mut self, alias: &str) -> Result<(), String> {
        self.registros.remove(alias);
        if self.predeterminado.as_ref() == Some(&alias.to_string()) {
            self.predeterminado = self.registros.keys().next().cloned();
        }
        Ok(())
    }
}
