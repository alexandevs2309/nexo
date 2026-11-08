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
            let salida_rs = ruta.with_extension("rs");
            let salida_bin = ruta.with_extension(if cfg!(windows) { "exe" } else { "" });
            match fs::write(&salida_rs, rust_code) {
                Ok(_) => {
                    println!("Código Rust generado: {}", salida_rs.display());
                    // Compilar con rustc.
                    let out = std::process::Command::new("rustc")
                        .arg("-O")
                        .arg(&salida_rs)
                        .arg("-o")
                        .arg(&salida_bin)
                        .output();
                    match out {
                        Ok(o) if o.status.success() => {
                            println!("Binario compilado: {}", salida_bin.display());
                            ExitCode::SUCCESS
                        }
                        Ok(o) => {
                            eprintln!("rustc falló:");
                            eprintln!("{}", String::from_utf8_lossy(&o.stderr));
                            ExitCode::from(1)
                        }
                        Err(e) => {
                            eprintln!("no se pudo ejecutar rustc: {e}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: no se pudo escribir '{}': {e}", salida_rs.display());
                    ExitCode::from(1)
                }
            }
        }
        "add" | "agregar" => {
            if args.len() < 2 {
                eprintln!("uso: {COMANDO} add <paquete> [versión] [--ruta=<ruta>]");
                return ExitCode::from(2);
            }
            agregar_dependencia(args[1].to_str().unwrap_or(""), &args[2..])
        }
        "update" | "actualizar" => {
            if args.len() > 2 {
                eprintln!("uso: {COMANDO} update [ruta]");
                return ExitCode::from(2);
            }
            actualizar_paquetes(args.get(1).map(PathBuf::from))
        }
        "publish" | "publicar" => {
            if args.len() > 4 {
                eprintln!("uso: {COMANDO} publish [ruta] [--clave=<hex32>] [--salida=<archivo>]");
                return ExitCode::from(2);
            }
            publicar_paquete(&args[1..])
        }
        "repl" => repl(),
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
        "depurar" | "debug" => {
            if args.len() < 2 {
                eprintln!("uso: {COMANDO} depurar <archivo.bsk> [--punto-funcion=<nombre>]");
                return ExitCode::from(2);
            }
            depurar_archivo(&args[1..])
        }
        "lsp" => {
            if args.len() > 1 {
                eprintln!("uso: {COMANDO} lsp");
                return ExitCode::from(2);
            }
            ejecutar_lsp()
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

fn repl() -> ExitCode {
    use std::io::{self, BufRead, Write};
    let stdin = io::stdin();
    let mut env = Entorno::nueva();
    let mut buffer = String::new();
    let mut acumulado = String::new();
    println!("{NOMBRE} REPL (escribe 'salir' para terminar)");
    loop {
        print!(">>> ");
        let _ = io::stdout().flush();
        buffer.clear();
        match stdin.lock().read_line(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("error de lectura: {e}");
                break;
            }
        }
        let linea = buffer.trim();
        if linea == "salir" || linea == "exit" || linea == "quit" {
            break;
        }
        if linea.is_empty() {
            continue;
        }
        acumulado.push_str(linea);
        acumulado.push('\n');
        // Intentar parsear y ejecutar.
        let tokens = match Lexer::nuevo(&acumulado).tokenizar() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("léxico: {e}");
                acumulado.clear();
                continue;
            }
        };
        let sentencias = match Parser::nuevo(tokens).parsear_programa() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("sintaxis: {e}");
                acumulado.clear();
                continue;
            }
        };
        match env.ejecutar_programa(&sentencias) {
            Ok(Some(v)) => println!("{v}"),
            Ok(None) => {}
            Err(e) => eprintln!("error: {e}"),
        }
        acumulado.clear();
    }
    ExitCode::SUCCESS
}

fn ejecutar_lsp() -> ExitCode {
    let mut servidor = crate::lsp::ServidorLsp::nuevo();
    servidor.ejecutar();
    ExitCode::SUCCESS
}

fn depurar_archivo(args: &[OsString]) -> ExitCode {
    use crate::ContextoSemantico;
    use crate::mir;
    use crate::vm::MaquinaVirtual;
    let mut archivo = None;
    let mut filtro: Option<String> = None;
    for a in args {
        let s = a.to_string_lossy();
        if let Some(f) = s.strip_prefix("--punto-funcion=") {
            filtro = Some(f.to_string());
        } else if archivo.is_none() {
            archivo = Some(PathBuf::from(s.to_string()));
        } else {
            eprintln!("argumento inesperado: {s}");
            return ExitCode::from(2);
        }
    }
    let Some(ruta) = archivo else {
        eprintln!("falta el archivo");
        return ExitCode::from(2);
    };
    let fuente = match fs::read_to_string(&ruta) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: no se pudo leer '{}': {e}", ruta.display());
            return ExitCode::from(1);
        }
    };
    let ruta_str = ruta.display().to_string();
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
    let tokens = match Lexer::con_archivo(&fuente, Some(ruta_str.clone())).tokenizar() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let sentencias = match Parser::con_archivo(tokens, Some(ruta_str.clone())).parsear_programa() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let mut ctx = ContextoSemantico::con_cargador(cargador, Some(ruta_str.clone()));
    ctx.set_archivo_actual(Some(PathBuf::from(&ruta_str)));
    let _ = ctx.analizar_programa(&sentencias);
    *crate::hir::MODULO_ERROR.lock().unwrap() = None;
    let hir = match crate::hir::lower_programa(&sentencias, &ctx) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("{:?}", e);
            return ExitCode::from(1);
        }
    };
    if let Some(e) = crate::hir::MODULO_ERROR.lock().unwrap().take() {
        eprintln!("{}", e);
        return ExitCode::from(1);
    }
    let programa = mir::lower_programa(&hir);
    let mut vm = MaquinaVirtual::nuevo(programa);
    vm.registrar_stdlib();
    vm.activar_traza();
    let Some(idx) = vm.indice_de("main") else {
        eprintln!("el programa no define `funcion main()`");
        return ExitCode::from(1);
    };
    match vm.ejecutar_indice(idx, &[]) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("error: {}", e);
            for t in vm.traza() {
                eprintln!("{}", t);
            }
            return ExitCode::from(1);
        }
    }
    for t in vm.traza() {
        if let Some(f) = &filtro {
            if t.contains(f) {
                println!("{}", t);
            }
        } else {
            println!("{}", t);
        }
    }
    ExitCode::SUCCESS
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
    println!("  {COMANDO} add <paquete> [ver]    agrega una dependencia y la instala");
    println!("  {COMANDO} update [ruta]          re-resuelve dependencias y reescribe {BLOQUEO}");
    println!("  {COMANDO} publish [ruta]         empaqueta el proyecto en un artefacto .bpk");
    println!("  {COMANDO} lsp                 servidor de idioma (LSP) sobre stdio");
    println!("  {COMANDO} depurar <archivo>    ejecuta con traza de funciones (depurador)");
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
    // Descubrir archivos .bsk recursivamente.
    let mut archivos: Vec<PathBuf> = Vec::new();
    fn recorrer(dir: &Path, archivos: &mut Vec<PathBuf>) {
        if let Ok(entradas) = fs::read_dir(dir) {
            for entrada in entradas.filter_map(|e| e.ok()) {
                let ruta = entrada.path();
                if ruta.is_dir() {
                    recorrer(&ruta, archivos);
                } else if ruta
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case(EXT_CANONICA))
                    .unwrap_or(false)
                {
                    archivos.push(ruta);
                }
            }
        }
    }
    recorrer(&base, &mut archivos);
    archivos.sort();

    if archivos.is_empty() {
        eprintln!("no se encontraron archivos .bsk en '{}'", base.display());
        return ExitCode::from(1);
    }

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

/// `bsk add <paquete> [versión] [--ruta=<ruta>]`: agrega la dependencia al
/// `[dependencias]` del manifiesto y ejecuta la instalación (resolución +
/// bloqueo). Si la dependencia no se puede resolver, el manifiesto no se toca.
fn agregar_dependencia(nombre: &str, resto: &[OsString]) -> ExitCode {
    let mut version = "*".to_string();
    let mut ruta: Option<String> = None;
    let mut hubo_posicional = false;
    for arg in resto {
        let s = arg.to_string_lossy().to_string();
        if let Some(r) = s.strip_prefix("--ruta=") {
            ruta = Some(r.to_string());
        } else if s.starts_with("--") {
            eprintln!("error: opción desconocida: {s}");
            return ExitCode::from(2);
        } else if !hubo_posicional {
            version = s;
            hubo_posicional = true;
        } else {
            eprintln!("uso: {COMANDO} add <paquete> [versión] [--ruta=<ruta>]");
            return ExitCode::from(2);
        }
    }
    if !paquetes::nombre_valido(nombre) {
        eprintln!("error: nombre de paquete inválido '{nombre}'");
        return ExitCode::from(3);
    }
    if !paquetes::version_valida(&version) {
        eprintln!("error: versión inválida '{version}'");
        return ExitCode::from(3);
    }
    let base = PathBuf::from(".");
    let manifiesto = match paquetes::cargar_manifiesto(&base) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    if manifiesto.dependencias.iter().any(|d| d.nombre == nombre) {
        eprintln!(
            "error: la dependencia '{nombre}' ya está en {}",
            base.join(paquetes::MANIFIESTO).display()
        );
        return ExitCode::from(3);
    }
    // Si la ruta explícita apunta fuera del proyecto, se autoriza de forma
    // explícita (el resolver sigue limitando al directorio padre).
    let mut autorizado = false;
    if let Some(r) = &ruta {
        let base_canonical = fs::canonicalize(&base).unwrap_or_else(|_| base.clone());
        if let Ok(c) = fs::canonicalize(base_canonical.join(r))
            && !c.starts_with(&base_canonical)
        {
            autorizado = true;
            println!("aviso: la ruta '{r}' está fuera del proyecto; se marca autorizado = true");
        }
    }
    let dep = paquetes::Dependencia {
        nombre: nombre.to_string(),
        version: version.clone(),
        ruta: ruta.clone(),
        hash: None,
        firma: None,
        clave: None,
        registro: None,
        nonce: None,
        expira: None,
        autorizado,
    };
    // Pre-verificar resolubilidad sin tocar el manifiesto.
    let base_canonical = fs::canonicalize(&base).unwrap_or_else(|_| base.clone());
    if let Err(e) = paquetes::resolver_dependencia(&dep, &base_canonical) {
        eprintln!("error: no se pudo resolver '{nombre}': {e}");
        eprintln!("el manifiesto no se modificó");
        return ExitCode::from(3);
    }
    // Editar el manifiesto conservando el resto del archivo.
    let ruta_manifiesto = base.join(paquetes::MANIFIESTO);
    let texto = match fs::read_to_string(&ruta_manifiesto) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "error: no se pudo leer '{}': {e}",
                ruta_manifiesto.display()
            );
            return ExitCode::from(3);
        }
    };
    let ruta_escapada = ruta.as_deref().map(escapar_toml);
    let version_escapada = escapar_toml(&version);
    let entrada = match (&ruta_escapada, autorizado) {
        (Some(r), true) => format!(
            "{nombre} = {{ version = \"{version_escapada}\", ruta = \"{r}\", autorizado = true }}"
        ),
        (Some(r), false) => {
            format!("{nombre} = {{ version = \"{version_escapada}\", ruta = \"{r}\" }}")
        }
        (None, _) => format!("{nombre} = \"{version_escapada}\""),
    };
    let nuevo = match insertar_dependencia_en_toml(&texto, &entrada) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    if let Err(e) = fs::write(&ruta_manifiesto, &nuevo) {
        eprintln!(
            "error: no se pudo escribir '{}': {e}",
            ruta_manifiesto.display()
        );
        return ExitCode::from(3);
    }
    println!("dependencia agregada: {entrada}");
    // Re-instalar para resolver todo y reescribir el bloqueo.
    instalar_paquetes(Some(base))
}

/// Inserta `entrada` al final de la sección `[dependencias]` del TOML,
/// conservando el resto del archivo (comentarios incluidos). Si la sección no
/// existe, se agrega al final.
fn insertar_dependencia_en_toml(texto: &str, entrada: &str) -> Result<String, String> {
    if entrada.contains('\n') {
        return Err("entrada de dependencia inválida".to_string());
    }
    let mut lineas: Vec<String> = texto.lines().map(str::to_string).collect();
    match lineas.iter().position(|l| l.trim() == "[dependencias]") {
        Some(i) => {
            let mut fin = lineas.len();
            for (j, l) in lineas.iter().enumerate().skip(i + 1) {
                if l.trim_start().starts_with('[') {
                    fin = j;
                    break;
                }
            }
            lineas.insert(fin, entrada.to_string());
        }
        None => {
            if lineas.last().is_some_and(|l| !l.trim().is_empty()) {
                lineas.push(String::new());
            }
            lineas.push("[dependencias]".to_string());
            lineas.push(entrada.to_string());
        }
    }
    let mut salida = lineas.join("\n");
    if !salida.ends_with('\n') {
        salida.push('\n');
    }
    Ok(salida)
}

fn escapar_toml(texto: &str) -> String {
    texto.replace('\\', "\\\\").replace('"', "\\\"")
}

/// `bsk update [ruta]`: re-resuelve todas las dependencias ignorando el
/// bloqueo previo (acepta los cambios locales) y reescribe `basilisco.lock`.
fn actualizar_paquetes(ruta: Option<PathBuf>) -> ExitCode {
    let base = ruta.unwrap_or_else(|| PathBuf::from("."));
    let manifiesto = match paquetes::cargar_manifiesto(&base) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    // La fuente manda: la caché se reconstruye con los hashes nuevos.
    let cache = base.join(".basilisco").join("cache");
    if cache.exists()
        && let Err(e) = fs::remove_dir_all(&cache)
    {
        eprintln!(
            "error: no se pudo limpiar la caché '{}': {e}",
            cache.display()
        );
        return ExitCode::from(3);
    }
    let resueltas = match paquetes::resolver_dependencias(&manifiesto, &base) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    if let Err(e) = paquetes::escribir_bloqueo(&base, &resueltas) {
        eprintln!("error: {e}");
        return ExitCode::from(3);
    }
    println!(
        "{} {}: {} dependencia(s) actualizada(s)",
        manifiesto.paquete.nombre,
        manifiesto.paquete.version,
        resueltas.len()
    );
    for dep in &resueltas {
        println!("  {} {} -> {}", dep.nombre, dep.version, dep.ruta.display());
    }
    println!("bloqueo reescrito: {}", base.join(BLOQUEO).display());
    ExitCode::SUCCESS
}

#[derive(serde::Serialize)]
struct BpkFormato {
    name: &'static str,
    version: &'static str,
}

#[derive(serde::Serialize)]
struct BpkPaquete {
    namespace: String,
    name: String,
    version: String,
}

#[derive(serde::Serialize)]
struct BpkArchivo {
    path: String,
    size: i64,
    sha256: String,
}

#[derive(serde::Serialize)]
struct BpkManifiesto {
    format: BpkFormato,
    package: BpkPaquete,
    files: Vec<BpkArchivo>,
}

/// `bsk publish [ruta] [--clave=<hex32>] [--salida=<archivo>]`: empaqueta los
/// `.bsk` del proyecto en un artefacto BPK (R2), lo auto-valida y, si se
/// proporciona clave privada Ed25519, firma los metadatos y escribe el
/// sidecar `.json` junto al artefacto.
fn publicar_paquete(args: &[OsString]) -> ExitCode {
    let mut base = PathBuf::from(".");
    let mut clave_hex: Option<String> = None;
    let mut salida: Option<PathBuf> = None;
    for arg in args {
        let s = arg.to_string_lossy().to_string();
        if let Some(k) = s.strip_prefix("--clave=") {
            clave_hex = Some(k.to_string());
        } else if let Some(o) = s.strip_prefix("--salida=") {
            salida = Some(PathBuf::from(o));
        } else if s.starts_with("--") {
            eprintln!("error: opción desconocida: {s}");
            return ExitCode::from(2);
        } else {
            base = PathBuf::from(&s);
        }
    }
    let manifiesto = match paquetes::cargar_manifiesto(&base) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    // El validador BPK exige SemVer estricto (la versión `*` no es publicable).
    if semver::Version::parse(&manifiesto.paquete.version).is_err() {
        eprintln!(
            "error: la versión '{}' no es SemVer estricto (se requiere X.Y.Z para publicar)",
            manifiesto.paquete.version
        );
        return ExitCode::from(3);
    }
    let fuentes = match recoger_fuentes_bsk(&base) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    let mut payload: Vec<(String, Vec<u8>)> = Vec::new();
    let mut archivos: Vec<BpkArchivo> = Vec::new();
    for (relativa, ruta) in &fuentes {
        let contenido = match fs::read(ruta) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: no se pudo leer '{}': {e}", ruta.display());
                return ExitCode::from(3);
            }
        };
        archivos.push(BpkArchivo {
            path: relativa.clone(),
            size: contenido.len() as i64,
            sha256: paquetes::sha256_hex(&contenido),
        });
        payload.push((relativa.clone(), contenido));
    }
    let manifiesto_bpk = BpkManifiesto {
        format: BpkFormato {
            name: "BASILISCO_PACKAGE",
            version: "1.0",
        },
        package: BpkPaquete {
            namespace: manifiesto.paquete.nombre.clone(),
            name: manifiesto.paquete.nombre.clone(),
            version: manifiesto.paquete.version.clone(),
        },
        files: archivos,
    };
    let manifest_toml = match toml::to_string_pretty(&manifiesto_bpk) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: no se pudo generar el manifiesto BPK: {e}");
            return ExitCode::from(3);
        }
    };
    let bytes = match crate::cliente_remoto::crear_bpk(&manifest_toml, &payload) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    // Autocomprobación: nunca publicar un artefacto que no pase el validador.
    if let Err(e) = crate::cliente_remoto::validar_bpk(&bytes) {
        eprintln!("error interno: el paquete generado no pasa la validación: {e}");
        return ExitCode::from(3);
    }
    let destino = salida.unwrap_or_else(|| {
        base.join("dist").join(format!(
            "{}-{}.bpk",
            manifiesto.paquete.nombre, manifiesto.paquete.version
        ))
    });
    if let Some(padre) = destino.parent()
        && let Err(e) = fs::create_dir_all(padre)
    {
        eprintln!("error: no se pudo crear '{}': {e}", padre.display());
        return ExitCode::from(3);
    }
    if let Err(e) = fs::write(&destino, &bytes) {
        eprintln!("error: no se pudo escribir '{}': {e}", destino.display());
        return ExitCode::from(3);
    }
    println!(
        "Paquete: {} {}",
        manifiesto.paquete.nombre, manifiesto.paquete.version
    );
    println!(
        "  artefacto: {} ({} bytes, {} archivo(s))",
        destino.display(),
        bytes.len(),
        payload.len()
    );
    println!("  sha256: {}", paquetes::sha256_hex(&bytes));
    if let Some(clave) = &clave_hex {
        match firmar_paquete(clave, &manifiesto, &bytes) {
            Ok(meta) => {
                let json_ruta = destino.with_extension("json");
                let escritura = serde_json::to_string_pretty(&meta)
                    .map_err(|e| e.to_string())
                    .and_then(|j| fs::write(&json_ruta, j).map_err(|e| e.to_string()));
                match escritura {
                    Ok(()) => {
                        println!("  metadatos firmados: {}", json_ruta.display());
                        if let Some(clave_publica) = &meta.clave {
                            println!("  clave pública (hex): {clave_publica}");
                        }
                        println!(
                            "  registry_id: {} key_id: {} nonce: {}",
                            meta.registry_id, meta.key_id, meta.nonce
                        );
                    }
                    Err(e) => {
                        eprintln!("error: no se pudieron escribir los metadatos firmados: {e}");
                        return ExitCode::from(3);
                    }
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(3);
            }
        }
    }
    ExitCode::SUCCESS
}

/// Firma los metadatos del paquete con la clave privada Ed25519 (`--clave`,
/// 32 bytes en hex) y auto-verifica la firma antes de devolverlos. El
/// `key_id` "key1" coincide con el que usa el resolver local para confiar.
fn firmar_paquete(
    clave_hex: &str,
    manifiesto: &paquetes::Manifiesto,
    artefacto: &[u8],
) -> Result<paquetes::MetadatosPaquete, String> {
    let clave_bytes = hex::decode(clave_hex)
        .map_err(|_| "clave privada inválida: se esperaba hex".to_string())?;
    if clave_bytes.len() != 32 {
        return Err("clave privada inválida: se esperaban 32 bytes (64 hex)".to_string());
    }
    let clave = ed25519_dalek::SigningKey::from_bytes(
        clave_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "clave privada inválida".to_string())?,
    );
    let publica = clave.verifying_key();
    let publico_hex = hex::encode(publica.as_bytes());
    let ahora = chrono::Utc::now();
    let nonce = format!("{:x}", ahora.timestamp_nanos_opt().unwrap_or_default());
    let meta = paquetes::MetadatosPaquete {
        namespace: manifiesto.paquete.nombre.clone(),
        name: manifiesto.paquete.nombre.clone(),
        version: manifiesto.paquete.version.clone(),
        sha256: paquetes::sha256_hex(artefacto),
        published_at: ahora.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        registry_id: "local".to_string(),
        key_id: "key1".to_string(),
        nonce,
        expires_at: (ahora + chrono::Duration::days(365))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        firma: None,
        clave: None,
    };
    let firma = paquetes::firmar_metadatos(&meta, clave.as_bytes())?;
    // Autocomprobación: la firma debe verificar con la clave pública derivada.
    paquetes::verificar_firma_metadatos(&meta, &firma, publica.as_bytes())?;
    let mut meta = meta;
    meta.firma = Some(firma);
    meta.clave = Some(publico_hex);
    Ok(meta)
}

/// Recopila todos los `.bsk` del proyecto para el payload del BPK: raíz y
/// subdirectorios recursivos, excluyendo cachés (`.basilisco`), dependencias
/// locales (`paquetes/`, `vendor/`), salidas (`dist/`) y directorios ocultos.
fn recoger_fuentes_bsk(base: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    use unicode_normalization::UnicodeNormalization;
    const MAX_ARCHIVOS: usize = 1000;
    const MAX_TOTAL: u64 = 100 * 1024 * 1024;
    const MAX_ARCHIVO: u64 = 10 * 1024 * 1024;
    let mut archivos: Vec<(String, PathBuf)> = Vec::new();
    let mut total = 0u64;
    fn recorrer(
        dir: &Path,
        base: &Path,
        archivos: &mut Vec<(String, PathBuf)>,
        total: &mut u64,
    ) -> Result<(), String> {
        if archivos.len() >= MAX_ARCHIVOS {
            return Err("demasiados archivos en el paquete".to_string());
        }
        let entradas = fs::read_dir(dir).map_err(|e| e.to_string())?;
        for entrada in entradas.filter_map(|e| e.ok()) {
            let ruta = entrada.path();
            let nombre = entrada.file_name().to_string_lossy().to_string();
            if ruta.is_dir() {
                if nombre.starts_with('.')
                    || matches!(nombre.as_str(), "paquetes" | "vendor" | "dist")
                {
                    continue;
                }
                recorrer(&ruta, base, archivos, total)?;
            } else if ruta.extension().and_then(|e| e.to_str()) == Some("bsk") {
                let relativa = ruta
                    .strip_prefix(base)
                    .map_err(|_| "ruta interna inválida".to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                let normalizada: String = relativa.nfc().collect();
                if normalizada != relativa {
                    return Err(format!(
                        "nombre de archivo no normalizado en NFC: {relativa}"
                    ));
                }
                if relativa.is_empty() || relativa.contains("..") || relativa.starts_with('/') {
                    return Err(format!("ruta de payload inválida: {relativa}"));
                }
                let meta = fs::metadata(&ruta).map_err(|e| e.to_string())?;
                if meta.len() > MAX_ARCHIVO {
                    return Err(format!("archivo demasiado grande: {relativa}"));
                }
                *total += meta.len();
                if *total > MAX_TOTAL {
                    return Err("paquete demasiado grande".to_string());
                }
                archivos.push((relativa, ruta));
            }
        }
        Ok(())
    }
    recorrer(base, base, &mut archivos, &mut total)?;
    if archivos.is_empty() {
        return Err("no hay archivos .bsk para publicar".to_string());
    }
    archivos.sort();
    Ok(archivos)
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
