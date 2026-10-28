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

use crate::formateador;
use crate::paquetes::{self, BLOQUEO};
use crate::cargador::CargadorModulos;
use crate::{Entorno, Lexer, Parser, Valor, evaluar};

pub const NOMBRE: &str = "BASILISCO";
pub const COMANDO: &str = "basilisco";
pub const ALIAS: &str = "bsk";
pub const EXT_CANONICA: &str = "bsk";
pub const EXT_LEGADO: &str = "nexo";

/// Punto de entrada común de `basilisco` y `bsk`.
pub fn main() -> ExitCode {
    let mut args = env::args_os();
    let _programa = args.next();
    let args: Vec<OsString> = args.collect();

    if args.is_empty() {
        demo();
        return ExitCode::SUCCESS;
    }

    let comando = args[0].to_string_lossy();
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
        "nuevo" => {
            if args.len() >= 2 && args[1].to_string_lossy() == "proyecto" {
                if args.len() != 3 {
                    eprintln!("uso: {COMANDO} nuevo proyecto <nombre>");
                    return ExitCode::from(2);
                }
                let nombre = args[2].to_string_lossy().to_string();
                let destino = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                match crate::proyecto::crear_proyecto(&destino, &nombre) {
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
        "run" | "ejecutar" => {
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
        "analizar" => {
            if args.len() != 2 {
                eprintln!("uso: {COMANDO} analizar <archivo.bsk|archivo.nexo>");
                return ExitCode::from(2);
            }
            analizar_archivo(Path::new(&args[1]))
        }
        "formatear" => {
            if args.len() != 2 {
                eprintln!("uso: {COMANDO} formatear <archivo.bsk|archivo.nexo>");
                return ExitCode::from(2);
            }
            formatear_archivo(Path::new(&args[1]))
        }
        "inicializar" => {
            if args.len() > 2 {
                eprintln!("uso: {COMANDO} inicializar [ruta]");
                return ExitCode::from(2);
            }
            inicializar_proyecto(args.get(1).map(PathBuf::from))
        }
        "probar" => {
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
    match Parser::nuevo(tokens).parsear_programa() {
        Ok(_) => {
            println!("análisis correcto: {}", ruta.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: {e}", ruta.display());
            ExitCode::from(1)
        }
    }
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
        println!("  {} {} -> {}", dep.nombre, dep.version, dep.ruta.display());
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
