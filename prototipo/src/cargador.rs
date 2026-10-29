//! Cargador de módulos BASILISCO (ADR-0017, Fase 1-4).
//!
//! Responsable de leer, validar y cachear módulos. No ejecuta sentencias de nivel
//! superior arbitrarias: solo procesa declaraciones, definiciones de función y
//! sentencias `usar` (para importaciones transitivas).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::evaluador::{CausaError, Entorno, ErrorEjecucion};
use crate::lexer::Lexer;
use crate::parser::{Parser, Sentencia};
use crate::valor::Valor;

/// Estado de carga de un módulo (rastreado por el cargador, no por el módulo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoCarga {
    EnCarga,
    Cargado,
}

/// Un símbodo exportado por un módulo.
#[derive(Debug, Clone)]
pub enum SimbodoExportado {
    Variable(Valor),
    Funcion(crate::evaluador::FuncionDef),
}

/// Un módulo cargado y cacheado.
#[derive(Debug)]
pub struct ModuloCargado {
    pub ruta: PathBuf,
    pub ast: Vec<Sentencia>,
    pub entorno: Mutex<Entorno>,
    pub exportaciones: HashMap<String, SimbodoExportado>,
}

/// Cargador de módulos con caché por ruta canónica.
#[derive(Debug)]
pub struct CargadorModulos {
    root: PathBuf,
    root_canonical: Option<PathBuf>,
    root_error: Option<String>,
    cache: HashMap<PathBuf, Arc<ModuloCargado>>,
    en_carga: HashSet<PathBuf>,
}

impl CargadorModulos {
    pub fn nuevo(root: PathBuf) -> Self {
        let (root_canonical, root_error) = match fs::canonicalize(&root) {
            Ok(c) => (Some(c), None),
            Err(e) => (
                None,
                Some(format!(
                    "no se pudo canonicalizar la raíz '{}': {e}",
                    root.display()
                )),
            ),
        };
        Self {
            root,
            root_canonical,
            root_error,
            cache: HashMap::new(),
            en_carga: HashSet::new(),
        }
    }

    pub fn root(&self) -> &Path {
        self.root_canonical.as_ref().unwrap_or(&self.root)
    }

    /// Resuelve una ruta de módulo a su ruta canónica sin cargar ni ejecutar
    /// el módulo. Reutiliza las mismas reglas de normalización y seguridad
    /// que `cargar`.
    pub fn resolver_ruta(
        cargador: &Arc<Mutex<Self>>,
        ruta_relativa: &str,
        archivo_importador: Option<&Path>,
    ) -> Result<PathBuf, ErrorEjecucion> {
        let root = {
            let c = cargador.lock().unwrap();
            match &c.root_canonical {
                Some(r) => r.clone(),
                None => {
                    return Err(ErrorEjecucion {
                        mensaje: c.root_error.clone().unwrap_or_default(),
                        causa: Box::new(CausaError::RutaNoPermitida {
                            ruta: c.root.display().to_string(),
                        }),
                        linea: 1,
                        columna: 1,
                        archivo: None,
                    });
                }
            }
        };

        let ruta_normalizada = normalizar_extension(ruta_relativa)?;

        let base_dir = match archivo_importador {
            Some(p) => {
                if let Ok(canonical_importador) = std::fs::canonicalize(p) {
                    canonical_importador
                        .parent()
                        .map(|pp| pp.to_path_buf())
                        .unwrap_or_else(|| root.clone())
                } else {
                    root.clone()
                }
            }
            None => root.clone(),
        };
        let resolved = base_dir.join(ruta_normalizada);

        let canonical = std::fs::canonicalize(&resolved).map_err(|_| ErrorEjecucion {
            mensaje: format!("módulo no encontrado: {}", resolved.display()),
            causa: Box::new(CausaError::ModuloNoEncontrado {
                ruta: resolved.display().to_string(),
            }),
            linea: 1,
            columna: 1,
            archivo: Some(resolved.display().to_string()),
        })?;

        if !canonical.starts_with(&root) {
            return Err(ErrorEjecucion {
                mensaje: format!("ruta no permitida: {}", canonical.display()),
                causa: Box::new(CausaError::RutaNoPermitida {
                    ruta: canonical.display().to_string(),
                }),
                linea: 1,
                columna: 1,
                archivo: Some(canonical.display().to_string()),
            });
        }

        Ok(canonical)
    }

    /// Carga (o reutiliza) un módulo.
    pub fn cargar(
        cargador: &Arc<Mutex<Self>>,
        ruta_relativa: &str,
        archivo_importador: Option<&Path>,
    ) -> Result<Arc<ModuloCargado>, ErrorEjecucion> {
        // 0. Validar raíz.
        let root = {
            let c = cargador.lock().unwrap();
            match &c.root_canonical {
                Some(r) => r.clone(),
                None => {
                    return Err(ErrorEjecucion {
                        mensaje: c.root_error.clone().unwrap_or_default(),
                        causa: Box::new(CausaError::RutaNoPermitida {
                            ruta: c.root.display().to_string(),
                        }),
                        linea: 1,
                        columna: 1,
                        archivo: None,
                    });
                }
            }
        };

        // 1. Normalizar extensión.
        let ruta_normalizada = normalizar_extension(ruta_relativa)?;

        // 2. Resolver ruta relativa al archivo importador (o al root).
        let base_dir = match archivo_importador {
            Some(p) => {
                if let Ok(canonical_importador) = fs::canonicalize(p) {
                    canonical_importador
                        .parent()
                        .map(|pp| pp.to_path_buf())
                        .unwrap_or_else(|| root.clone())
                } else {
                    root.clone()
                }
            }
            None => root.clone(),
        };
        let resolved = base_dir.join(ruta_normalizada);

        // 3. Canonicalizar.
        let canonical = match fs::canonicalize(&resolved) {
            Ok(c) => c,
            Err(_) => {
                return Err(ErrorEjecucion {
                    mensaje: format!("módulo no encontrado: {}", resolved.display()),
                    causa: Box::new(CausaError::ModuloNoEncontrado {
                        ruta: resolved.display().to_string(),
                    }),
                    linea: 1,
                    columna: 1,
                    archivo: Some(resolved.display().to_string()),
                });
            }
        };

        // 4. Validar root.
        if !canonical.starts_with(&root) {
            return Err(ErrorEjecucion {
                mensaje: format!("ruta no permitida: {}", canonical.display()),
                causa: Box::new(CausaError::RutaNoPermitida {
                    ruta: canonical.display().to_string(),
                }),
                linea: 1,
                columna: 1,
                archivo: Some(canonical.display().to_string()),
            });
        }

        // 5. Caché.
        {
            let c = cargador.lock().unwrap();
            if let Some(modulo) = c.cache.get(&canonical) {
                return Ok(modulo.clone());
            }
            // 6. Ciclo.
            if c.en_carga.contains(&canonical) {
                let cadena = vec![canonical.display().to_string()];
                return Err(ErrorEjecucion {
                    mensaje: format!("ciclo de importación: {}", cadena.join(" -> ")),
                    causa: Box::new(CausaError::CicloDeImportacion { cadena }),
                    linea: 1,
                    columna: 1,
                    archivo: Some(canonical.display().to_string()),
                });
            }
        }

        // 7. Marcar como en carga.
        {
            let cache_hit = {
                let c = cargador.lock().unwrap();
                if c.cache.contains_key(&canonical) {
                    Some(c.cache.get(&canonical).unwrap().clone())
                } else {
                    None
                }
            };
            if let Some(m) = cache_hit {
                return Ok(m);
            }
            cargador.lock().unwrap().en_carga_insert(canonical.clone());
        }

        // 8. Cargar internamente.
        let resultado = CargadorModulos::cargar_interno(cargador, &canonical);

        // 9. Limpiar marca.
        cargador.lock().unwrap().en_carga.remove(&canonical);

        match resultado {
            Ok(modulo) => {
                let rc = Arc::new(modulo);
                cargador.lock().unwrap().cache.insert(canonical, rc.clone());
                Ok(rc)
            }
            Err(e) => Err(e),
        }
    }

    fn cargar_interno(
        cargador: &Arc<Mutex<Self>>,
        canonical: &Path,
    ) -> Result<ModuloCargado, ErrorEjecucion> {
        let ruta_str = canonical.display().to_string();

        // Leer archivo.
        let fuente = fs::read_to_string(canonical).map_err(|e| {
            let causa = if e.kind() == std::io::ErrorKind::PermissionDenied {
                CausaError::PermisosDenegados {
                    ruta: ruta_str.clone(),
                }
            } else {
                CausaError::ArchivoNoEncontrado {
                    ruta: ruta_str.clone(),
                }
            };
            ErrorEjecucion {
                mensaje: format!("no se pudo leer '{}': {e}", ruta_str),
                causa: Box::new(causa),
                linea: 1,
                columna: 1,
                archivo: Some(ruta_str.clone()),
            }
        })?;

        // Lexar.
        let mut lexer = Lexer::con_archivo(&fuente, Some(ruta_str.clone()));
        let tokens = lexer.tokenizar().map_err(|e| ErrorEjecucion {
            mensaje: e.mensaje.clone(),
            causa: Box::new(CausaError::ErrorDeArchivo {
                ruta: ruta_str.clone(),
                detalle: e.mensaje.clone(),
            }),
            linea: e.linea,
            columna: e.columna,
            archivo: e.archivo.clone().or(Some(ruta_str.clone())),
        })?;

        // Parsear.
        let mut parser = Parser::con_archivo(tokens, Some(ruta_str.clone()));
        let ast = parser.parsear_programa().map_err(|e| ErrorEjecucion {
            mensaje: e.mensaje.clone(),
            causa: Box::new(CausaError::ErrorDeArchivo {
                ruta: ruta_str.clone(),
                detalle: e.mensaje.clone(),
            }),
            linea: e.linea,
            columna: e.columna,
            archivo: e.archivo.clone().or(Some(ruta_str.clone())),
        })?;

        // Validar: solo Declaracion, Funcion y Usar en nivel superior (ADR-0017 D4).
        for s in &ast {
            match s {
                Sentencia::Declaracion { .. }
                | Sentencia::Funcion { .. }
                | Sentencia::Usar { .. } => {}
                _ => {
                    let (linea, columna) = ubicacion_sentencia(s);
                    return Err(ErrorEjecucion {
                        mensaje: "sentencia no permitida en el nivel superior de un módulo"
                            .to_string(),
                        causa: Box::new(CausaError::SentenciaNoPermitidaEnModulo {
                            sentencia: format!("{:?}", s),
                        }),
                        linea,
                        columna,
                        archivo: Some(ruta_str.clone()),
                    });
                }
            }
        }

        // Procesar: ejecutar programa (procesa declaraciones, funciones y usar transitivos).
        let mut entorno = Entorno::con_cargador(cargador.clone(), Some(ruta_str.clone()));
        entorno.ejecutar_programa(&ast).map_err(|mut e| {
            if e.archivo.is_none() {
                e.archivo = Some(ruta_str.clone());
            }
            e
        })?;

        // Construir exportaciones (Fase 3: solo `publico`).
        let mut exportaciones = HashMap::new();
        for s in &ast {
            match s {
                Sentencia::Declaracion {
                    nombre, publico, ..
                } => {
                    if *publico && let Some(valor) = entorno.variable(nombre) {
                        exportaciones.insert(nombre.clone(), SimbodoExportado::Variable(valor));
                    }
                }
                Sentencia::Funcion {
                    nombre, publico, ..
                } => {
                    if *publico && let Some(fdef) = entorno.funcion(nombre) {
                        exportaciones.insert(nombre.clone(), SimbodoExportado::Funcion(fdef));
                    }
                }
                _ => {}
            }
        }

        Ok(ModuloCargado {
            ruta: canonical.to_path_buf(),
            ast,
            entorno: Mutex::new(entorno),
            exportaciones,
        })
    }

    pub fn en_carga_contains(&self, path: &Path) -> bool {
        self.en_carga.contains(path)
    }

    pub fn en_carga_insert(&mut self, path: PathBuf) {
        self.en_carga.insert(path);
    }

    pub fn en_carga_remove(&mut self, path: &Path) {
        self.en_carga.remove(path);
    }
}

fn normalizar_extension(ruta: &str) -> Result<String, ErrorEjecucion> {
    let path = Path::new(ruta);
    match path.extension().and_then(|e| e.to_str()) {
        None | Some("") => {
            if ruta.ends_with('.') {
                Ok(format!("{}bsk", ruta))
            } else {
                Ok(format!("{}.bsk", ruta))
            }
        }
        Some(ext) => {
            let ext_lower = ext.to_ascii_lowercase();
            match ext_lower.as_str() {
                "bsk" | "nexo" => Ok(ruta.to_string()),
                _ => Err(ErrorEjecucion {
                    mensaje: format!("extensión no soportada: .{ext}"),
                    causa: Box::new(CausaError::ExtensionNoSoportada {
                        extension: format!(".{ext}"),
                    }),
                    linea: 1,
                    columna: 1,
                    archivo: Some(ruta.to_string()),
                }),
            }
        }
    }
}

fn ubicacion_sentencia(s: &Sentencia) -> (usize, usize) {
    match s {
        Sentencia::Declaracion { linea, columna, .. }
        | Sentencia::Asignacion { linea, columna, .. }
        | Sentencia::AsignacionIndice { linea, columna, .. }
        | Sentencia::Si { linea, columna, .. }
        | Sentencia::Mientras { linea, columna, .. }
        | Sentencia::Funcion { linea, columna, .. }
        | Sentencia::Retorno { linea, columna, .. }
        | Sentencia::Mostrar { linea, columna, .. }
        | Sentencia::MostrarSinSalto { linea, columna, .. }
        | Sentencia::ParaCada { linea, columna, .. }
        | Sentencia::Romper { linea, columna }
        | Sentencia::Continuar { linea, columna }
        | Sentencia::IntentarRescatar { linea, columna, .. }
        | Sentencia::Importar { linea, columna, .. }
        | Sentencia::Exportar { linea, columna, .. }
        | Sentencia::Usar { linea, columna, .. } => (*linea, *columna),
        Sentencia::Bloque(_) | Sentencia::Expresion(_) => (1, 1),
    }
}
