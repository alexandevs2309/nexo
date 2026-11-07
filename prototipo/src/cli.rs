//! Interfaz de línea de comandos de BASILISCO.
//!
//! Contrato: ADR-0010. Extensión canónica: `.bsk`; legado: `.nexo`.
//! `.nx` no se acepta mientras no exista evidencia de archivos externos.

use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use crate::cargador::CargadorModulos;
use crate::formateador;
use crate::paquetes::{self, BLOQUEO};
use crate::{ContextoSemantico, Entorno, Lexer, Parser, Valor, backend_nativo, evaluar, hir, mir};

pub const NOMBRE: &str = "BASILISCO";
pub const COMANDO: &str = "basilisco";
pub const ALIAS: &str = "bsk";
pub const EXT_CANONICA: &str = "bsk";
pub const EXT_LEGADO: &str = "nexo";

/// Motor de ejecución preferido ("arbol" = Tree-Walk, "vm" = VM).
/// Se inicializa desde la variable de entorno BASILISCO_MOTOR o --motor=.
static MOTOR_PREFERIDO: std::sync::LazyLock<std::sync::Mutex<String>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(String::from("vm")));

/// Punto de entrada común de `basilisco` y `bsk`.
pub fn main() -> ExitCode {
    let mut args = env::args_os();
    let _programa = args.next();
    let mut args: Vec<OsString> = args.collect();

    // Extraer --motor=... / --engine=... de los argumentos (no rompe el resto).
    let mut motor = std::env::var("BASILISCO_MOTOR").unwrap_or_else(|_| "vm".to_string());
    args.retain(|a| {
        let s = a.to_string_lossy();
        if let Some(v) = s
            .strip_prefix("--motor=")
            .or_else(|| s.strip_prefix("--engine="))
        {
            motor = v.to_string();
            false
        } else {
            true
        }
    });
    *MOTOR_PREFERIDO.lock().unwrap() = motor;

    if args.is_empty() {
        // Si hay un proyecto válido, ejecutarlo; si no, mostrar ayuda.
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if crate::proyecto::cargar_manifiesto(&cwd).is_ok() {
            return ejecutar_archivo_con_root(
                &crate::proyecto::resolver_entrada(
                    &cwd,
                    &crate::proyecto::cargar_manifiesto(&cwd).unwrap(),
                )
                .unwrap_or_else(|_| cwd.join("src/main.bsk")),
                &cwd,
            );
        }
        // No interactivo: mostrar ayuda.
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            // REPL futuro
            eprintln!("No se encontró un proyecto en el directorio actual.");
            ayuda();
            return ExitCode::from(2);
        } else {
            ayuda();
            return ExitCode::SUCCESS;
        }
    }

    let comando = args[0].to_string_lossy();
    // Ejecución directa: `bsk archivo.bsk` o `bsk archivo` (resuelve extensión).
    if args.len() == 1 {
        let candidato = PathBuf::from(&*comando);
        // Si tiene extensión de archivo, ejecutar directamente.
        if candidato.extension().is_some() && candidato.is_file() {
            return ejecutar_archivo(&candidato);
        }
        // Si no tiene extensión, intentar resolver .bsk
        if candidato.extension().is_none() {
            let con_ext = candidato.with_extension(EXT_CANONICA);
            if con_ext.is_file() {
                return ejecutar_archivo(&con_ext);
            }
        }
    }
    match comando.as_ref() {
        "--version" | "-V" | "version" => {
            println!("{NOMBRE} {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "--help" | "-h" | "ayuda" => {
            ayuda();
            ExitCode::SUCCESS
        }
        "demo" => {
            demo();
            ExitCode::SUCCESS
        }
        "build" | "compilar" => {
            if args.len() > 2 {
                eprintln!("uso: {COMANDO} build [archivo.bsk]");
                return ExitCode::from(2);
            }
            let ruta = if args.len() == 2 {
                PathBuf::from(&args[1])
            } else {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                match crate::proyecto::cargar_manifiesto(&cwd) {
                    Ok(m) => crate::proyecto::resolver_entrada(&cwd, &m)
                        .unwrap_or_else(|_| cwd.join("src/main.bsk")),
                    Err(_) => {
                        eprintln!("No se encontró manifiesto. Usa: {COMANDO} build <archivo.bsk>");
                        return ExitCode::from(1);
                    }
                }
            };
            if !ruta.exists() {
                eprintln!("error: archivo no existe: {}", ruta.display());
                return ExitCode::from(1);
            }
            let fuente = match fs::read_to_string(&ruta) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("error: no se pudo leer '{}': {e}", ruta.display());
                    return ExitCode::from(1);
                }
            };
            let tokens =
                match Lexer::con_archivo(&fuente, Some(ruta.display().to_string())).tokenizar() {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("error léxico: {e}");
                        return ExitCode::from(1);
                    }
                };
            let sentencias = match Parser::con_archivo(tokens, Some(ruta.display().to_string()))
                .parsear_programa()
            {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error sintáctico: {e}");
                    return ExitCode::from(1);
                }
            };
            let mut ctx = ContextoSemantico::nuevo();
            let errs = ctx.analizar_programa(&sentencias);
            if !errs.is_empty() {
                for e in errs {
                    eprintln!("{}", e);
                }
                return ExitCode::from(1);
            }
            let hir = match hir::lower_programa(&sentencias, &ctx) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("error HIR: {:?}", e);
                    return ExitCode::from(1);
                }
            };
            let mir = mir::lower_programa(&hir);
            let rust_code = backend_nativo::generar_rust(&mir);
            let salida = ruta.with_extension("rs");
            match fs::write(&salida, rust_code) {
                Ok(_) => {
                    println!("Código Rust generado: {}", salida.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: no se pudo escribir '{}': {e}", salida.display());
                    ExitCode::from(1)
                }
            }
        }
        "add" | "agregar" => {
            if args.len() < 2 {
                eprintln!("uso: {COMANDO} add <paquete> [versión]");
                return ExitCode::from(2);
            }
            eprintln!(
                "Añadir dependencia: {} (pendiente de implementación completa)",
                args[1].to_string_lossy()
            );
            ExitCode::from(0)
        }
        "update" | "actualizar" => {
            if args.len() > 2 {
                eprintln!("uso: {COMANDO} update [ruta]");
                return ExitCode::from(2);
            }
            eprintln!("Actualización de dependencias (pendiente de implementación completa).");
            ExitCode::from(0)
        }
        "publish" | "publicar" => {
            eprintln!("Publicación de paquetes (pendiente de implementación completa).");
            ExitCode::from(0)
        }
        "repl" => {
            eprintln!("REPL interactivo (pendiente de implementación).");
            ExitCode::SUCCESS
        }
        "nuevo" => {
            if args.len() >= 2 && args[1].to_string_lossy() == "proyecto" {
                // Parsear --plantilla=...
                let mut plantilla = "consola".to_string();
                let mut nombre_idx = 2;
                for (i, arg) in args.iter().enumerate().skip(2) {
                    let arg = arg.to_string_lossy();
                    if let Some(p) = arg.strip_prefix("--plantilla=") {
                        plantilla = p.to_string();
                    } else {
                        nombre_idx = i;
                    }
                }
                if nombre_idx >= args.len() {
                    eprintln!(
                        "uso: {COMANDO} nuevo proyecto <nombre> [--plantilla=consola|inventario|tareas]"
                    );
                    return ExitCode::from(2);
                }
                let nombre = args[nombre_idx].to_string_lossy().to_string();
                let destino = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                match crate::proyecto::crear_proyecto_con_plantilla(&destino, &nombre, &plantilla) {
                    Ok(dir) => {
                        println!("Proyecto creado: {}", dir.display());
                        println!("Ejecuta tu programa con: {COMANDO} run");
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        ExitCode::from(1)
                    }
                }
            } else {
                if args.len() > 2 {
                    eprintln!("uso: {COMANDO} nuevo [archivo.bsk]");
                    return ExitCode::from(2);
                }
                crear_archivo_inicial(args.get(1).map(OsString::as_os_str))
            }
        }
        "run" | "ejecutar" | "r" => {
            if args.len() == 1 {
                // Buscar manifiesto en el directorio actual.
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                match crate::proyecto::cargar_manifiesto(&cwd) {
                    Ok(m) => {
                        let entrada = match crate::proyecto::resolver_entrada(&cwd, &m) {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("error: {e}");
                                return ExitCode::from(1);
                            }
                        };
                        if !entrada.exists() {
                            eprintln!(
                                "error: el archivo de entrada no existe: {}",
                                entrada.display()
                            );
                            return ExitCode::from(1);
                        }
                        ejecutar_archivo_con_root(&entrada, &cwd)
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        eprintln!("uso: {COMANDO} run <archivo.bsk|archivo.nexo>");
                        ExitCode::from(2)
                    }
                }
            } else if args.len() == 2 {
                ejecutar_archivo(Path::new(&args[1]))
            } else {
                eprintln!("uso: {COMANDO} run [<archivo.bsk>]");
                ExitCode::from(2)
            }
        }
        "instalar-remoto" => {
            if args.len() != 4 {
                eprintln!("uso: {COMANDO} instalar-remoto <metadata-url> <artifact-url> <destino>");
                return ExitCode::from(2);
            }
            instalar_remoto(
                args[1].to_str().unwrap_or(""),
                args[2].to_str().unwrap_or(""),
                Path::new(&args[3]),
            )
        }
        "registry" => {
            if args.is_empty() {
                eprintln!(
                    "uso: {COMANDO} registry <add|list|remove|set-default|enable|disable|info>"
                );
                return ExitCode::from(2);
            }
            match args[0].to_str().unwrap_or("") {
                "add" => {
                    if args.len() < 5 {
                        eprintln!("uso: {COMANDO} registry add <alias> <registry-id> <url>");
                        return ExitCode::from(2);
                    }
                    let base = PathBuf::from(".");
                    let mut cfg =
                        crate::paquetes::ConfigRegistros::cargar(&base).unwrap_or_default();
                    if let Err(e) = cfg.agregar(
                        args[1].to_str().unwrap_or(""),
                        args[2].to_str().unwrap_or(""),
                        args[3].to_str().unwrap_or(""),
                    ) {
                        eprintln!("error: {e}");
                        return ExitCode::from(1);
                    }
                    if let Err(e) = cfg.guardar(&base) {
                        eprintln!("error: {e}");
                        return ExitCode::from(1);
                    }
                    println!("registro añadido");
                    ExitCode::SUCCESS
                }
                "list" => {
                    let base = PathBuf::from(".");
                    let cfg = crate::paquetes::ConfigRegistros::cargar(&base).unwrap_or_default();
                    if cfg.registros.is_empty() {
                        println!("sin registros");
                        return ExitCode::SUCCESS;
                    }
                    for (alias, r) in &cfg.registros {
                        let def = if cfg.predeterminado.as_ref() == Some(alias) {
                            " (predeterminado)"
                        } else {
                            ""
                        };
                        println!(
                            "{} [{}] {} {}{}",
                            alias,
                            r.registry_id,
                            r.url,
                            if r.habilitado {
                                "habilitado"
                            } else {
                                "deshabilitado"
                            },
                            def
                        );
                    }
                    ExitCode::SUCCESS
                }
                "remove" => {
                    if args.len() < 2 {
                        eprintln!("uso: {COMANDO} registry remove <alias>");
                        return ExitCode::from(2);
                    }
                    let base = PathBuf::from(".");
                    let mut cfg =
                        crate::paquetes::ConfigRegistros::cargar(&base).unwrap_or_default();
                    if let Err(e) = cfg.eliminar(args[1].to_str().unwrap_or("")) {
                        eprintln!("error: {e}");
                        return ExitCode::from(1);
                    }
                    if let Err(e) = cfg.guardar(&base) {
                        eprintln!("error: {e}");
                        return ExitCode::from(1);
                    }
                    println!("registro eliminado");
                    ExitCode::SUCCESS
                }
                "set-default" => {
                    if args.len() < 2 {
                        eprintln!("uso: {COMANDO} registry set-default <alias>");
                        return ExitCode::from(2);
                    }
                    let base = PathBuf::from(".");
                    let mut cfg =
                        crate::paquetes::ConfigRegistros::cargar(&base).unwrap_or_default();
                    if let Err(e) = cfg.set_predeterminado(args[1].to_str().unwrap_or("")) {
                        eprintln!("error: {e}");
                        return ExitCode::from(1);
                    }
                    if let Err(e) = cfg.guardar(&base) {
                        eprintln!("error: {e}");
                        return ExitCode::from(1);
                    }
                    println!("registro predeterminado actualizado");
                    ExitCode::SUCCESS
                }
                "enable" => {
                    if args.len() < 2 {
                        eprintln!("uso: {COMANDO} registry enable <alias>");
                        return ExitCode::from(2);
                    }
                    let base = PathBuf::from(".");
                    let mut cfg =
                        crate::paquetes::ConfigRegistros::cargar(&base).unwrap_or_default();
                    if let Err(e) = cfg.habilitar(args[1].to_str().unwrap_or(""), true) {
                        eprintln!("error: {e}");
                        return ExitCode::from(1);
                    }
                    if let Err(e) = cfg.guardar(&base) {
                        eprintln!("error: {e}");
                        return ExitCode::from(1);
                    }
                    println!("registro habilitado");
                    ExitCode::SUCCESS
                }
                "disable" => {
                    if args.len() < 2 {
                        eprintln!("uso: {COMANDO} registry disable <alias>");
                        return ExitCode::from(2);
                    }
                    let base = PathBuf::from(".");
                    let mut cfg =
                        crate::paquetes::ConfigRegistros::cargar(&base).unwrap_or_default();
                    if let Err(e) = cfg.habilitar(args[1].to_str().unwrap_or(""), false) {
                        eprintln!("error: {e}");
                        return ExitCode::from(1);
                    }
                    if let Err(e) = cfg.guardar(&base) {
                        eprintln!("error: {e}");
                        return ExitCode::from(1);
                    }
                    println!("registro deshabilitado");
                    ExitCode::SUCCESS
                }
                "info" => {
                    if args.len() < 2 {
                        eprintln!("uso: {COMANDO} registry info <alias>");
                        return ExitCode::from(2);
                    }
                    let base = PathBuf::from(".");
                    let cfg = crate::paquetes::ConfigRegistros::cargar(&base).unwrap_or_default();
                    if let Some(r) = cfg.obtener(args[1].to_str().unwrap_or("")) {
                        println!("alias: {}", r.alias);
                        println!("registry_id: {}", r.registry_id);
                        println!("url: {}", r.url);
                        println!("habilitado: {}", r.habilitado);
                        println!("prioridad: {}", r.prioridad);
                        ExitCode::SUCCESS
                    } else {
                        eprintln!("error: registro no encontrado");
                        ExitCode::from(1)
                    }
                }
                _ => {
                    eprintln!(
                        "uso: {COMANDO} registry <add|list|remove|set-default|enable|disable|info>"
                    );
                    ExitCode::from(2)
                }
            }
        }
        "fmt" | "formatear" => {
            if args.len() != 2 {
                eprintln!("uso: {COMANDO} fmt <archivo.bsk>");
                return ExitCode::from(2);
            }
            formatear_archivo(Path::new(&args[1]))
        }
        "check" | "analizar" => {
            if args.len() != 2 {
                eprintln!("uso: {COMANDO} analizar <archivo.bsk|archivo.nexo>");
                return ExitCode::from(2);
            }
            analizar_archivo(Path::new(&args[1]))
        }
        "inicializar" => {
            if args.len() > 2 {
                eprintln!("uso: {COMANDO} inicializar [ruta]");
                return ExitCode::from(2);
            }
            inicializar_proyecto(args.get(1).map(PathBuf::from))
        }
        "probar" | "test" => {
            if args.len() > 2 {
                eprintln!("uso: {COMANDO} probar [ruta]");
                return ExitCode::from(2);
            }
            probar_ruta(args.get(1).map(PathBuf::from))
        }
        "instalar" => {
            if args.len() > 2 {
                eprintln!("uso: {COMANDO} instalar [ruta-del-proyecto]");
                return ExitCode::from(2);
            }
            instalar_paquetes(args.get(1).map(PathBuf::from))
        }
        "paquetes" => {
            if args.len() > 2 {
                eprintln!("uso: {COMANDO} paquetes [ruta-del-proyecto]");
                return ExitCode::from(2);
            }
            listar_paquetes(args.get(1).map(PathBuf::from))
        }
        otro => {
            eprintln!("comando no reconocido: {otro}");
            ayuda();
            ExitCode::from(2)
        }
    }
}

fn ayuda() {
    println!("{NOMBRE} — intérprete del lenguaje (prototipo)");
    println!();
    println!("Uso:");
    println!("  {COMANDO} nuevo [archivo.bsk]   crea un programa inicial");
    println!("  {COMANDO} nuevo proyecto <nombre> crea un proyecto BASILISCO");
    println!("  {COMANDO} run [<archivo.bsk>] ejecuta un archivo o proyecto");
    println!("  {COMANDO} <archivo.bsk>        ejecuta directamente (alias corto).");
    println!("  {COMANDO}                      ejecuta el proyecto actual o abre REPL.");
    println!("  Opciones globales: --motor=arbol|vm (o BASILISCO_MOTOR=...) selecciona el motor.");
    println!("  {COMANDO} run <archivo.nexo>     ejecuta un programa legado");
    println!("  {COMANDO} run                   ejecuta el proyecto actual");
    println!("  {COMANDO} analizar <archivo>     analiza sin ejecutar");
    println!("  {COMANDO} formatear <archivo>    normaliza formato");
    println!("  {COMANDO} inicializar [ruta]     crea un proyecto mínimo");
    println!("  {COMANDO} probar [ruta]          ejecuta archivos .bsk");
    println!("  {COMANDO} instalar [ruta]        resuelve dependencias y escribe {BLOQUEO}");
    println!("  {COMANDO} paquetes [ruta]        lista las dependencias del bloqueo");
    println!("  {COMANDO} demo                   ejecuta una demostración");
    println!("  {COMANDO} --version              muestra la versión");
    println!("  {COMANDO} --help                 muestra esta ayuda");
    println!();
    println!("Alias: {ALIAS} (mismos comandos).");
    println!("Extensiones aceptadas: .{EXT_CANONICA} (canónica), .{EXT_LEGADO} (legado).");
}

fn crear_archivo_inicial(nombre: Option<&std::ffi::OsStr>) -> ExitCode {
    let mut ruta = nombre.map_or_else(|| PathBuf::from("hola.bsk"), PathBuf::from);
    match ruta.extension().and_then(|extension| extension.to_str()) {
        None | Some("") => {
            ruta.set_extension(EXT_CANONICA);
        }
        Some(extension) if extension.eq_ignore_ascii_case(EXT_CANONICA) => {}
        Some(_) => {
            eprintln!("error: el archivo nuevo debe usar la extensión .{EXT_CANONICA}");
            return ExitCode::from(2);
        }
    }

    let contenido = "mostrar \"Hola, BASILISCO!\"\n";
    let resultado = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&ruta)
        .and_then(|mut archivo| archivo.write_all(contenido.as_bytes()));

    match resultado {
        Ok(()) => {
            println!("Archivo creado: {}", ruta.display());
            println!("Ejecuta tu programa con: {COMANDO} run {}", ruta.display());
            ExitCode::SUCCESS
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            eprintln!("error: '{}' ya existe; no se modificó", ruta.display());
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("error: no se pudo crear '{}': {error}", ruta.display());
            ExitCode::from(1)
        }
    }
}

fn ejecutar_archivo(ruta: &Path) -> ExitCode {
    let root = ruta
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    ejecutar_archivo_con_root(ruta, &root)
}

fn ejecutar_archivo_con_root(ruta: &Path, root: &Path) -> ExitCode {
    let Some(extension) = ruta.extension().and_then(|e| e.to_str()) else {
        eprintln!("error: el archivo debe tener extensión .{EXT_CANONICA} o .{EXT_LEGADO}");
        return ExitCode::from(2);
    };
    let extension = extension.to_ascii_lowercase();
    match extension.as_str() {
        EXT_CANONICA => {}
        EXT_LEGADO => eprintln!("aviso: .{EXT_LEGADO} es legado; usa .{EXT_CANONICA}"),
        "nx" => {
            eprintln!("error: .nx no está soportado; usa .{EXT_CANONICA} o .{EXT_LEGADO}");
            return ExitCode::from(2);
        }
        otra => {
            eprintln!("error: extensión .{otra} no soportada; usa .{EXT_CANONICA} o .{EXT_LEGADO}");
            return ExitCode::from(2);
        }
    }

    let fuente = match fs::read_to_string(ruta) {
        Ok(fuente) => fuente,
        Err(e) => {
            eprintln!("error: no se pudo leer '{}': {e}", ruta.display());
            return ExitCode::from(1);
        }
    };

    let ruta_str = ruta.display().to_string();
    let cargador = Arc::new(Mutex::new(CargadorModulos::nuevo(root.to_path_buf())));
    match ejecutar_fuente(&fuente, Some(ruta_str), Some(cargador)) {
        Ok(Some(valor)) => {
            println!("{valor}");
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn analizar_archivo(ruta: &Path) -> ExitCode {
    let Some(extension) = ruta.extension().and_then(|e| e.to_str()) else {
        eprintln!("error: el archivo debe tener extensión .{EXT_CANONICA} o .{EXT_LEGADO}");
        return ExitCode::from(2);
    };
    let extension = extension.to_ascii_lowercase();
    match extension.as_str() {
        EXT_CANONICA => {}
        EXT_LEGADO => eprintln!("aviso: .{EXT_LEGADO} es legado; usa .{EXT_CANONICA}"),
        "nx" => {
            eprintln!("error: .nx no está soportado; usa .{EXT_CANONICA} o .{EXT_LEGADO}");
            return ExitCode::from(2);
        }
        otra => {
            eprintln!("error: extensión .{otra} no soportada; usa .{EXT_CANONICA} o .{EXT_LEGADO}");
            return ExitCode::from(2);
        }
    }

    let fuente = match fs::read_to_string(ruta) {
        Ok(fuente) => fuente,
        Err(e) => {
            eprintln!("error: no se pudo leer '{}': {e}", ruta.display());
            return ExitCode::from(1);
        }
    };

    let tokens = match Lexer::nuevo(&fuente).tokenizar() {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("{}: {e}", ruta.display());
            return ExitCode::from(1);
        }
    };
    let sentencias = match Parser::nuevo(tokens).parsear_programa() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {e}", ruta.display());
            return ExitCode::from(1);
        }
    };

    let root = ruta
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let cargador = Arc::new(Mutex::new(CargadorModulos::nuevo(root)));
    let mut ctx = ContextoSemantico::con_cargador(cargador, Some(ruta.display().to_string()));
    let errs = ctx.analizar_programa(&sentencias);
    if !errs.is_empty() {
        for e in errs {
            eprintln!("{}", e);
        }
        return ExitCode::from(1);
    }
    println!("análisis correcto: {}", ruta.display());
    ExitCode::SUCCESS
}

fn formatear_archivo(ruta: &Path) -> ExitCode {
    let Some(extension) = ruta.extension().and_then(|e| e.to_str()) else {
        eprintln!("error: el archivo debe tener extensión .{EXT_CANONICA} o .{EXT_LEGADO}");
        return ExitCode::from(2);
    };
    let extension = extension.to_ascii_lowercase();
    match extension.as_str() {
        EXT_CANONICA | EXT_LEGADO => {}
        "nx" => {
            eprintln!("error: .nx no está soportado; usa .{EXT_CANONICA} o .{EXT_LEGADO}");
            return ExitCode::from(2);
        }
        otra => {
            eprintln!("error: extensión .{otra} no soportada; usa .{EXT_CANONICA} o .{EXT_LEGADO}");
            return ExitCode::from(2);
        }
    }

    let fuente = match fs::read_to_string(ruta) {
        Ok(fuente) => fuente,
        Err(e) => {
            eprintln!("error: no se pudo leer '{}': {e}", ruta.display());
            return ExitCode::from(1);
        }
    };

    let tokens = match Lexer::nuevo(&fuente).tokenizar() {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("{}: {e}", ruta.display());
            return ExitCode::from(1);
        }
    };
    if Parser::nuevo(tokens).parsear_programa().is_err() {
        // Re-parse to get error message.
        let tokens = Lexer::nuevo(&fuente).tokenizar().unwrap();
        let e = Parser::nuevo(tokens).parsear_programa().unwrap_err();
        eprintln!("{}: {e}", ruta.display());
        return ExitCode::from(1);
    }

    let normalizado = match formateador::formatear(&fuente) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {e}", ruta.display());
            return ExitCode::from(1);
        }
    };

    match fs::write(ruta, normalizado) {
        Ok(()) => {
            println!("formateado: {}", ruta.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: no se pudo escribir '{}': {e}", ruta.display());
            ExitCode::from(1)
        }
    }
}

fn inicializar_proyecto(ruta: Option<PathBuf>) -> ExitCode {
    let base = ruta.unwrap_or_else(|| PathBuf::from("."));
    if let Err(e) = fs::create_dir_all(&base) {
        eprintln!("error: no se pudo crear '{}': {e}", base.display());
        return ExitCode::from(1);
    }
    let archivo = base.join("hola.bsk");
    let manifiesto = base.join("basilisco.toml");
    if archivo.exists() || manifiesto.exists() {
        eprintln!("error: el proyecto ya existe; no se modificó");
        return ExitCode::from(1);
    }
    let contenido = "mostrar \"Hola, BASILISCO!\"\n";
    let toml = "[paquete]\nnombre = \"mi-proyecto\"\nversion = \"0.1.0\"\n\n[dependencias]\n";
    if let Err(e) = fs::create_dir_all(base.join("paquetes")) {
        eprintln!("error: no se pudo crear 'paquetes/': {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = fs::write(&manifiesto, toml) {
        eprintln!("error: no se pudo crear '{}': {e}", manifiesto.display());
        return ExitCode::from(1);
    }
    match fs::write(&archivo, contenido) {
        Ok(()) => {
            println!("Proyecto inicializado en {}", base.display());
            println!("Archivo creado: {}", archivo.display());
            println!("Manifiesto creado: {}", manifiesto.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: no se pudo crear '{}': {e}", archivo.display());
            ExitCode::from(1)
        }
    }
}

fn probar_ruta(ruta: Option<PathBuf>) -> ExitCode {
    let base = ruta.unwrap_or_else(|| PathBuf::from("."));
    let entradas = match fs::read_dir(&base) {
        Ok(entradas) => entradas,
        Err(e) => {
            eprintln!("error: no se pudo leer '{}': {e}", base.display());
            return ExitCode::from(1);
        }
    };

    let mut archivos: Vec<PathBuf> = entradas
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case(EXT_CANONICA))
                .unwrap_or(false)
        })
        .collect();
    archivos.sort();

    if archivos.is_empty() {
        eprintln!("no se encontraron archivos .bsk en '{}'", base.display());
        return ExitCode::from(1);
    }

    let mut fallos = 0usize;
    for archivo in &archivos {
        let fuente = match fs::read_to_string(archivo) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{}: no se pudo leer: {e}", archivo.display());
                fallos += 1;
                continue;
            }
        };
        match ejecutar_fuente(&fuente, Some(archivo.display().to_string()), None) {
            Ok(_) => println!("OK  {}", archivo.display()),
            Err(e) => {
                eprintln!("FAIL {} -> {e}", archivo.display());
                fallos += 1;
            }
        }
    }

    if fallos == 0 {
        println!("{} prueba(s) correcta(s)", archivos.len());
        ExitCode::SUCCESS
    } else {
        eprintln!("{} fallo(s) de {}", fallos, archivos.len());
        ExitCode::from(1)
    }
}

fn ejecutar_fuente(
    fuente: &str,
    archivo: Option<String>,
    cargador: Option<Arc<Mutex<CargadorModulos>>>,
) -> Result<Option<Valor>, String> {
    let motor = MOTOR_PREFERIDO.lock().unwrap().clone();
    if motor == "vm" {
        match ejecutar_fuente_vm(fuente, archivo.clone(), cargador.clone()) {
            Ok(v) => return Ok(v),
            Err(e) => {
                eprintln!("aviso: motor VM falló ({e}); usando motor árbol (Tree-Walk).");
            }
        }
    }

    let tokens = Lexer::con_archivo(fuente, archivo.clone())
        .tokenizar()
        .map_err(|e| e.to_string())?;
    let sentencias = Parser::con_archivo(tokens, archivo.clone())
        .parsear_programa()
        .map_err(|e| e.to_string())?;
    let mut entorno = match cargador {
        Some(c) => Entorno::con_cargador(c, archivo.clone()),
        None => {
            let mut e = Entorno::nueva();
            e.archivo = archivo.clone();
            e
        }
    };
    let resultado = entorno.ejecutar_programa(&sentencias);
    match resultado {
        Ok(v) => Ok(v),
        Err(mut e) => {
            if e.archivo.is_none() {
                e.archivo = archivo;
            }
            Err(e.to_string())
        }
    }
}

/// Ejecuta un programa con el motor VM. Devuelve error si no hay `main` o si
/// el pipeline MIR/VM falla (el llamador cae al Tree-Walk).
fn ejecutar_fuente_vm(
    fuente: &str,
    archivo: Option<String>,
    cargador: Option<Arc<Mutex<CargadorModulos>>>,
) -> Result<Option<Valor>, String> {
    use crate::ContextoSemantico;
    use crate::mir;
    use crate::vm::MaquinaVirtual;
    use std::path::PathBuf;

    let tokens = Lexer::con_archivo(fuente, archivo.clone())
        .tokenizar()
        .map_err(|e| e.to_string())?;
    let sentencias = Parser::con_archivo(tokens, archivo.clone())
        .parsear_programa()
        .map_err(|e| e.to_string())?;

    let mut ctx = match cargador.clone() {
        Some(c) => ContextoSemantico::con_cargador(c, archivo.clone()),
        None => {
            let mut c = ContextoSemantico::nuevo();
            c.archivo = archivo.clone();
            c
        }
    };
    if let Some(a) = &archivo {
        ctx.set_archivo_actual(Some(PathBuf::from(a)));
    }
    let _ = ctx.analizar_programa(&sentencias);
    // Limpiar error previo de módulo.
    *crate::hir::MODULO_ERROR.lock().unwrap() = None;
    let hir = crate::hir::lower_programa(&sentencias, &ctx).map_err(|e| format!("{:?}", e))?;
    if let Some(e) = crate::hir::MODULO_ERROR.lock().unwrap().take() {
        return Err(e);
    }
    let programa = mir::lower_programa(&hir);

    let mut vm = MaquinaVirtual::nuevo(programa);
    vm.registrar_stdlib();

    let Some(idx) = vm.indice_de("main") else {
        return Err("el programa no define `funcion main()`".to_string());
    };
    vm.ejecutar_indice(idx, &[]).map_err(|e| e.to_string())
}

/// Solo para la demo; evalúa una única expresión.
fn principal(fuente: &str) -> Result<Valor, String> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| e.to_string())?;
    let expr = Parser::nuevo(tokens)
        .parsear_expresion()
        .map_err(|e| e.to_string())?;
    evaluar(&expr).map_err(|e| e.to_string())
}

fn demo() {
    println!("{NOMBRE} — prototipo v{}", env!("CARGO_PKG_VERSION"));
    println!();

    let casos = [
        "10 + 5",
        "0.1 + 0.2",
        "2 + 3.5",
        "1e10",
        "7 / 2",
        "10.0 / 4.0",
        "-7 % 3",
        "10 / 0",
        "9223372036854775807 + 1",
    ];

    println!("Evaluación de expresiones:");
    for caso in casos {
        match principal(caso) {
            Ok(v) => println!("  {caso:<24} = {v}"),
            Err(e) => println!("  {caso:<24} → {e}"),
        }
    }

    println!();
    println!("Programa con variables:");
    let programa = "sea precio = 10.5\nsea total = precio * 3\ntotal = total - 1.5\ntotal";
    match ejecutar_fuente(programa, None, None) {
        Ok(Some(v)) => println!("  total después de la modificación = {v}"),
        Ok(None) => println!("  (el programa no produjo valor)"),
        Err(e) => println!("  → {e}"),
    }

    println!();
    println!("Funciones y texto:");
    let programa = "\
funcion suma(a, b) {
  retornar a + b
}
mostrar suma(2, 3)
mostrar \"Hola \" + \"BASILISCO\"";
    match ejecutar_fuente(programa, None, None) {
        Ok(Some(v)) => println!("  resultado = {v}"),
        Ok(None) => {}
        Err(e) => println!("  → {e}"),
    }

    println!();
    println!("Control de flujo y lógicos:");
    let control = "\
sea total = 0
sea contador = 0
mientras contador < 5 {
  contador = contador + 1
  si contador % 2 == 0 {
    total = total + contador
  } sino {
    total = total + 1
  }
}
total";
    match ejecutar_fuente(control, None, None) {
        Ok(Some(v)) => println!("  total acumulado = {v}"),
        Ok(None) => println!("  (el programa no produjo valor)"),
        Err(e) => println!("  → {e}"),
    }
}

fn instalar_paquetes(ruta: Option<PathBuf>) -> ExitCode {
    let base = ruta.unwrap_or_else(|| PathBuf::from("."));
    let manifiesto = match paquetes::cargar_manifiesto(&base) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };

    let resueltas = match paquetes::resolver_dependencias(&manifiesto, &base) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };

    if let Err(e) = paquetes::verificar_integridad(&base, &resueltas) {
        eprintln!("error: {e}");
        return ExitCode::from(3);
    }
    if let Err(e) = paquetes::escribir_bloqueo(&base, &resueltas) {
        eprintln!("error: {e}");
        return ExitCode::from(3);
    }

    println!(
        "{} {}: {} dependencia(s) resuelta(s)",
        manifiesto.paquete.nombre,
        manifiesto.paquete.version,
        resueltas.len()
    );
    for dep in &resueltas {
        if dep.verificado {
            println!(
                "  {} {} -> {} (hash verificado)",
                dep.nombre,
                dep.version,
                dep.ruta.display()
            );
        } else {
            println!(
                "  {} {} -> {} (hash no verificado)",
                dep.nombre,
                dep.version,
                dep.ruta.display()
            );
        }
    }
    println!("bloqueo escrito: {}", base.join(BLOQUEO).display());
    ExitCode::SUCCESS
}

fn listar_paquetes(ruta: Option<PathBuf>) -> ExitCode {
    let base = ruta.unwrap_or_else(|| PathBuf::from("."));
    match paquetes::leer_bloqueo(&base) {
        Ok(Some(deps)) => {
            if deps.is_empty() {
                println!("sin dependencias");
            } else {
                for dep in deps {
                    println!("{} {} -> {}", dep.nombre, dep.version, dep.ruta.display());
                }
            }
            ExitCode::SUCCESS
        }
        Ok(None) => {
            eprintln!(
                "error: no existe '{}'; ejecuta '{COMANDO} instalar'",
                base.join(BLOQUEO).display()
            );
            ExitCode::from(3)
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(3)
        }
    }
}

fn instalar_remoto(metadata_url: &str, artifact_url: &str, destino: &Path) -> ExitCode {
    let meta = match crate::cliente_remoto::obtener_metadatos(metadata_url) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes = match crate::cliente_remoto::descargar_artefacto(artifact_url) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = crate::cliente_remoto::verificar_artefacto(&bytes, &meta.sha256) {
        eprintln!("error: {e}");
        return ExitCode::from(1);
    }
    // Verificación criptográfica si hay firma y clave.
    let base = destino.parent().unwrap_or(destino);
    let almacen = crate::paquetes::AlmacenClaves::cargar(base).unwrap_or_default();
    if let (Some(firma), Some(clave_hex)) = (&meta.firma, &meta.clave) {
        let clave_bytes = match hex_decode(clave_hex) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(1);
            }
        };
        if !almacen.es_confiable(&meta.registry_id, &meta.key_id, &clave_bytes) {
            eprintln!("error: clave no confiable");
            return ExitCode::from(1);
        }
        if let Err(e) = crate::paquetes::verificar_firma_metadatos(&meta, firma, &clave_bytes) {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    }
    match crate::cliente_remoto::extraer_bpk(&bytes, destino) {
        Ok(_) => {
            println!("instalado: {}", destino.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
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
