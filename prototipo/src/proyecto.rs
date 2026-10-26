//! Proyecto BASILISCO: manifiesto `basilisco.toml` y scaffolding (Fase 5).
//!
//! Subconjunto admitido del manifiesto (parser propio, sin dependencias externas):
//! - Una clave por línea: `clave = "valor"`.
//! - Comentarios con `#` hasta fin de línea.
//! - Líneas vacías se ignoran.
//! - Claves admitidas: `nombre`, `version`, `entrada`.
//! - Todos los valores son cadenas entre comillas dobles.
//! - No se admiten tablas, arrays, números sin comillas ni multilinea.

use std::fs;
use std::path::{Path, PathBuf};

/// Manifiesto de un proyecto BASILISCO.
#[derive(Debug, Clone)]
pub struct Manifiesto {
    pub nombre: String,
    pub version: String,
    pub entrada: String,
}

/// Error de manifiesto.
#[derive(Debug, Clone)]
pub struct ErrorManifiesto {
    pub mensaje: String,
    pub linea: usize,
}

impl std::fmt::Display for ErrorManifiesto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.linea > 0 {
            write!(f, "error de manifiesto [{}]: {}", self.linea, self.mensaje)
        } else {
            write!(f, "error de manifiesto: {}", self.mensaje)
        }
    }
}

/// Parsea un manifiesto `basilisco.toml` desde su contenido.
pub fn parsear_manifiesto(contenido: &str) -> Result<Manifiesto, ErrorManifiesto> {
    let mut nombre: Option<String> = None;
    let mut version: Option<String> = None;
    let mut entrada: Option<String> = None;
    let mut vistos: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (idx, raw_line) in contenido.lines().enumerate() {
        let linea_num = idx + 1;
        // Eliminar comentario.
        let sin_comentario = match raw_line.find('#') {
            Some(pos) => &raw_line[..pos],
            None => raw_line,
        };
        let trimmed = sin_comentario.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parsear clave = "valor"
        let partes: Vec<&str> = trimmed.splitn(2, '=').collect();
        if partes.len() != 2 {
            return Err(ErrorManifiesto {
                mensaje: format!("se esperaba 'clave = \"valor\"', se obtuvo: {trimmed}"),
                linea: linea_num,
            });
        }
        let clave = partes[0].trim();
        let valor_raw = partes[1].trim();

        // Validar clave conocida.
        match clave {
            "nombre" | "version" | "entrada" => {}
            _ => {
                return Err(ErrorManifiesto {
                    mensaje: format!("clave desconocida: '{clave}'"),
                    linea: linea_num,
                });
            }
        }

        // Validar valor entre comillas dobles.
        if !valor_raw.starts_with('"') || !valor_raw.ends_with('"') || valor_raw.len() < 2 {
            return Err(ErrorManifiesto {
                mensaje: format!("el valor de '{clave}' debe ser una cadena entre comillas dobles"),
                linea: linea_num,
            });
        }
        let valor = valor_raw[1..valor_raw.len() - 1].to_string();

        // Validar clave duplicada.
        if !vistos.insert(clave.to_string()) {
            return Err(ErrorManifiesto {
                mensaje: format!("clave duplicada: '{clave}'"),
                linea: linea_num,
            });
        }

        match clave {
            "nombre" => nombre = Some(valor),
            "version" => {
                // Validar formato semántico: X.Y.Z
                if !es_version_semantica(&valor) {
                    return Err(ErrorManifiesto {
                        mensaje: format!("versión inválida: '{valor}' (se espera X.Y.Z)"),
                        linea: linea_num,
                    });
                }
                version = Some(valor);
            }
            "entrada" => entrada = Some(valor),
            _ => {}
        }
    }

    let nombre = nombre.ok_or_else(|| ErrorManifiesto {
        mensaje: "falta el campo obligatorio 'nombre'".to_string(),
        linea: 0,
    })?;
    let version = version.ok_or_else(|| ErrorManifiesto {
        mensaje: "falta el campo obligatorio 'version'".to_string(),
        linea: 0,
    })?;
    let entrada = entrada.ok_or_else(|| ErrorManifiesto {
        mensaje: "falta el campo obligatorio 'entrada'".to_string(),
        linea: 0,
    })?;

    // Validar que entrada no sea absoluta ni escape.
    if Path::new(&entrada).is_absolute() {
        return Err(ErrorManifiesto {
            mensaje: format!("'entrada' no puede ser una ruta absoluta: '{entrada}'"),
            linea: 0,
        });
    }
    for comp in Path::new(&entrada).components() {
        if comp == std::path::Component::ParentDir {
            return Err(ErrorManifiesto {
                mensaje: format!("'entrada' no puede contener '..': '{entrada}'"),
                linea: 0,
            });
        }
    }

    Ok(Manifiesto {
        nombre,
        version,
        entrada,
    })
}

fn es_version_semantica(v: &str) -> bool {
    let partes: Vec<&str> = v.split('.').collect();
    if partes.len() != 3 {
        return false;
    }
    for p in &partes {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    true
}

/// Lee y parsea un manifiesto desde un directorio de proyecto.
pub fn cargar_manifiesto(dir: &Path) -> Result<Manifiesto, ErrorManifiesto> {
    let ruta = dir.join("basilisco.toml");
    let contenido = fs::read_to_string(&ruta).map_err(|e| ErrorManifiesto {
        mensaje: format!("no se pudo leer '{}': {e}", ruta.display()),
        linea: 0,
    })?;
    parsear_manifiesto(&contenido)
}

/// Resuelve la ruta de entrada relativa a la raíz del proyecto.
pub fn resolver_entrada(dir: &Path, manifiesto: &Manifiesto) -> Result<PathBuf, ErrorManifiesto> {
    let entrada = Path::new(&manifiesto.entrada);
    if entrada.is_absolute() {
        return Err(ErrorManifiesto {
            mensaje: format!("'entrada' no puede ser absoluta: '{}'", manifiesto.entrada),
            linea: 0,
        });
    }
    for comp in entrada.components() {
        if comp == std::path::Component::ParentDir {
            return Err(ErrorManifiesto {
                mensaje: format!("'entrada' no puede contener '..': '{}'", manifiesto.entrada),
                linea: 0,
            });
        }
    }
    Ok(dir.join(entrada))
}

/// Valida un nombre de proyecto.
pub fn validar_nombre_proyecto(nombre: &str) -> Result<(), ErrorManifiesto> {
    if nombre.is_empty() {
        return Err(ErrorManifiesto {
            mensaje: "el nombre del proyecto no puede estar vacío".to_string(),
            linea: 0,
        });
    }
    if !nombre
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ErrorManifiesto {
            mensaje: format!(
                "nombre de proyecto inválido: '{nombre}' (solo letras, dígitos, _ y -)"
            ),
            linea: 0,
        });
    }
    if !nombre
        .chars()
        .next()
        .map(|c| c.is_ascii_alphabetic())
        .unwrap_or(false)
    {
        return Err(ErrorManifiesto {
            mensaje: "el nombre del proyecto debe empezar con una letra".to_string(),
            linea: 0,
        });
    }
    Ok(())
}

/// Crea un proyecto nuevo en `destino/<nombre>`.
pub fn crear_proyecto(destino: &Path, nombre: &str) -> Result<PathBuf, ErrorManifiesto> {
    validar_nombre_proyecto(nombre)?;

    let proyecto_dir = destino.join(nombre);
    if proyecto_dir.exists() {
        return Err(ErrorManifiesto {
            mensaje: format!("el directorio '{}' ya existe", proyecto_dir.display()),
            linea: 0,
        });
    }

    // Crear directorio raíz.
    fs::create_dir_all(&proyecto_dir).map_err(|e| ErrorManifiesto {
        mensaje: format!("no se pudo crear '{}': {e}", proyecto_dir.display()),
        linea: 0,
    })?;

    // Crear subdirectorios.
    let src_dir = proyecto_dir.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| ErrorManifiesto {
        mensaje: format!("no se pudo crear '{}': {e}", src_dir.display()),
        linea: 0,
    })?;

    // Archivos a crear.
    let archivos: Vec<(PathBuf, String)> = vec![
        (
            proyecto_dir.join("basilisco.toml"),
            format!(
                "nombre = \"{}\"\nversion = \"0.1.0\"\nentrada = \"src/main.bsk\"\n",
                nombre
            ),
        ),
        (
            src_dir.join("main.bsk"),
            "mostrar \"Hola, BASILISCO!\"\n".to_string(),
        ),
        (
            src_dir.join("lib.bsk"),
            "publico funcion saludar(nombre) {\n  retornar \"Hola, \" + nombre\n}\n".to_string(),
        ),
        (
            proyecto_dir.join("README.md"),
            format!("# {}\n\nProyecto BASILISCO.\n", nombre),
        ),
        (proyecto_dir.join(".gitignore"), "target/\n".to_string()),
    ];

    // Escribir archivos (create_new para no sobrescribir).
    for (ruta, contenido) in &archivos {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(ruta)
            .map_err(|e| ErrorManifiesto {
                mensaje: format!("no se pudo crear '{}': {e}", ruta.display()),
                linea: 0,
            })?;
        f.write_all(contenido.as_bytes())
            .map_err(|e| ErrorManifiesto {
                mensaje: format!("no se pudo escribir '{}': {e}", ruta.display()),
                linea: 0,
            })?;
    }

    Ok(proyecto_dir)
}
