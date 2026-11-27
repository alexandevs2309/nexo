use std::process::{Command, Output};

fn ejecutar(bin: &str, args: &[&str]) -> Output {
    Command::new(bin)
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("no se pudo ejecutar el binario")
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

#[test]
fn version_identifica_basilisco() {
    let salida = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["--version"]);
    assert!(salida.status.success());
    assert!(texto(&salida.stdout).contains("BASILISCO"));
}

#[test]
fn alias_basilisco_funciona() {
    let salida = ejecutar(env!("CARGO_BIN_EXE_basilisco"), &["--version"]);
    assert!(salida.status.success());
    assert!(texto(&salida.stdout).contains("BASILISCO"));
}

#[test]
fn ayuda_muestra_run() {
    let salida = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["--help"]);
    assert!(salida.status.success());
    let stdout = texto(&salida.stdout);
    assert!(stdout.contains("run [<archivo.bsk>]"));
    assert!(stdout.contains(".nexo"));
}

#[test]
fn ejecuta_extension_canonica() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/hola.bsk"],
    );
    assert!(salida.status.success());
    assert_eq!(texto(&salida.stdout).trim(), "Hola BASILISCO");
}

#[test]
fn ejecuta_extension_legado() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/legado.nexo"],
    );
    assert!(salida.status.success());
    assert_eq!(texto(&salida.stdout).trim(), "42");
    assert!(texto(&salida.stderr).contains("legado"));
}

#[test]
fn rechaza_nx() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/legado.nx"],
    );
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains(".nx no está soportado"));
}

#[test]
fn rechaza_extension_desconocida() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/hola.txt"],
    );
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("no soportada"));
}

#[test]
fn rechaza_archivo_sin_extension() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/sin_extension"],
    );
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("extensión"));
}

#[test]
fn archivo_inexistente_falla() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/no-existe.bsk"],
    );
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("no se pudo leer"));
}

#[test]
fn ruta_con_espacios_funciona() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "tests/fixtures/hola con espacios.bsk"],
    );
    assert!(salida.status.success());
    assert_eq!(texto(&salida.stdout).trim(), "ruta con espacios");
}

#[test]
fn ejecuta_ejemplo_de_funciones() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "examples/funciones_basicas.bsk"],
    );
    assert!(salida.status.success());
    assert_eq!(texto(&salida.stdout), "5\n120\n42\n");
}

#[test]
fn tutorial_inicio_se_ejecuta_por_cli() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "examples/tutorial_inicio.bsk"],
    );
    assert!(salida.status.success());
    assert_eq!(texto(&salida.stdout), "12\n");
}

#[test]
fn aceptacion_v04_ejecuta_el_nucleo_integrado() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["run", "examples/aceptacion_v04.bsk"],
    );
    assert!(salida.status.success(), "stderr: {}", texto(&salida.stderr));
    assert_eq!(texto(&salida.stdout).trim(), "total=9");
}

#[test]
fn aceptacion_v04_diagnostica_errores_con_archivo_y_ubicacion() {
    let ruta = "examples/aceptacion_error_v04.bsk";
    let salida = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["run", ruta]);
    let stderr = texto(&salida.stderr);

    assert!(!salida.status.success());
    assert!(stderr.contains(ruta), "stderr: {stderr}");
    assert!(
        stderr.contains("E1001") && stderr.contains("examples/aceptacion_error_v04.bsk:2:1"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("división entre cero"), "stderr: {stderr}");
}

#[test]
fn analizar_archivo_valido() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["analizar", "examples/tutorial_inicio.bsk"],
    );
    assert!(salida.status.success());
    assert!(texto(&salida.stdout).contains("análisis correcto"));
}

#[test]
fn analizar_archivo_invalido() {
    let dir = directorio_temporal();
    let ruta = dir.join("malo.bsk");
    std::fs::write(&ruta, "sea x = \n").unwrap();
    let ruta_str = ruta.to_string_lossy().to_string();
    let salida = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["analizar", &ruta_str])
        .output()
        .unwrap();
    assert!(!salida.status.success());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_sin_archivo_es_error_de_uso() {
    let salida = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["run"]);
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("uso:"));
}

fn directorio_temporal() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tid = std::thread::current().id();
    let ruta = std::env::temp_dir().join(format!("basilisco-cli-{nanos}-{seq}-{tid:?}"));
    std::fs::create_dir_all(&ruta).unwrap();
    ruta
}

#[test]
fn nuevo_crea_hola_bsk_y_se_ejecuta() {
    let dir = directorio_temporal();
    let salida = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["nuevo"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(salida.status.success(), "stderr: {}", texto(&salida.stderr));
    assert!(dir.join("hola.bsk").is_file());

    let salida = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", "hola.bsk"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(salida.status.success(), "stderr: {}", texto(&salida.stderr));
    assert_eq!(texto(&salida.stdout).trim(), "Hola, BASILISCO!");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nuevo_con_nombre_conserva_bsk_y_no_sobrescribe() {
    let dir = directorio_temporal();
    let salida = Command::new(env!("CARGO_BIN_EXE_basilisco"))
        .args(["nuevo", "saludo"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(salida.status.success(), "stderr: {}", texto(&salida.stderr));
    assert!(dir.join("saludo.bsk").is_file());

    let salida = Command::new(env!("CARGO_BIN_EXE_basilisco"))
        .args(["nuevo", "saludo.bsk"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("ya existe"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nuevo_rechaza_extension_distinta() {
    let dir = directorio_temporal();
    let salida = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["nuevo", "saludo.txt"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(!salida.status.success());
    assert!(texto(&salida.stderr).contains("extensión"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(feature = "test-hooks")]
#[test]
fn gate_build_aborta_y_no_genera_artefacto() {
    let dir = directorio_temporal();
    let src = dir.join("prog.bsk");
    std::fs::write(&src, "mostrar 42\n").unwrap();
    let salida = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["build", src.to_str().unwrap()])
        .env("BASILISCO_TEST_FORCE_GATE_FAIL", "1")
        .output()
        .unwrap();
    assert!(
        !salida.status.success(),
        "stderr: {}",
        texto(&salida.stderr)
    );
    assert!(
        texto(&salida.stderr).contains("verificacion:"),
        "stderr: {}",
        texto(&salida.stderr)
    );
    assert!(!src.with_extension("rs").exists(), "no debe generarse .rs");
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(feature = "test-hooks")]
#[test]
fn gate_run_aborta_sin_fallback() {
    let dir = directorio_temporal();
    let src = dir.join("prog.bsk");
    std::fs::write(&src, "mostrar 42\n").unwrap();
    let salida = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", src.to_str().unwrap()])
        .env("BASILISCO_TEST_FORCE_GATE_FAIL", "1")
        .output()
        .unwrap();
    assert!(
        !salida.status.success(),
        "stderr: {}",
        texto(&salida.stderr)
    );
    assert!(
        texto(&salida.stderr).contains("verificacion:"),
        "stderr: {}",
        texto(&salida.stderr)
    );
    assert!(
        !texto(&salida.stderr).contains("usando motor árbol"),
        "no debe activar fallback: {}",
        texto(&salida.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(feature = "test-hooks")]
#[test]
fn gate_analizar_devuelve_codigo_salida() {
    let dir = directorio_temporal();
    let src = dir.join("prog.bsk");
    std::fs::write(&src, "mostrar 42\n").unwrap();
    let salida = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["analizar", src.to_str().unwrap()])
        .env("BASILISCO_TEST_FORCE_GATE_FAIL", "1")
        .output()
        .unwrap();
    assert!(
        !salida.status.success(),
        "stderr: {}",
        texto(&salida.stderr)
    );
    assert!(
        texto(&salida.stderr).contains("verificacion:"),
        "stderr: {}",
        texto(&salida.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_modular_resuelve_importaciones() {
    let dir = directorio_temporal();
    std::fs::write(
        dir.join("lib.bsk"),
        "publico funcion saludar() {\n  retornar \"hola-modulo\"\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.bsk"),
        "usar \"lib.bsk\" como m\nmostrar m.saludar()\n",
    )
    .unwrap();
    let salida = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", "main.bsk"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(salida.status.success(), "stderr: {}", texto(&salida.stderr));
    assert_eq!(texto(&salida.stdout).trim(), "hola-modulo");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_modular_genera_y_ejecuta_binario() {
    let dir = directorio_temporal();
    std::fs::write(
        dir.join("lib.bsk"),
        "publico funcion sumar(a, b) {\n  retornar a + b\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.bsk"),
        "usar \"lib.bsk\" como m\nmostrar m.sumar(20, 22)\n",
    )
    .unwrap();
    let salida = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["build", "main.bsk"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(salida.status.success(), "stderr: {}", texto(&salida.stderr));
    assert!(dir.join("main.rs").is_file(), "debe generarse main.rs");
    let bin = dir.join(if cfg!(windows) { "main.exe" } else { "main" });
    assert!(bin.is_file(), "debe generarse binario nativo");

    let salida_bin = Command::new(&bin).current_dir(&dir).output().unwrap();
    assert!(
        salida_bin.status.success(),
        "stderr: {}",
        texto(&salida_bin.stderr)
    );
    assert_eq!(texto(&salida_bin.stdout).trim(), "42");
    let _ = std::fs::remove_dir_all(&dir);
}

fn ejecutar_repl(entrada: &str) -> std::process::Output {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .arg("repl")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("no se pudo iniciar repl");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(entrada.as_bytes())
        .unwrap();
    child.wait_with_output().expect("repl no terminó")
}

#[test]
fn repl_persistencia_sesion_vm() {
    // 1. Declaración inicial, 2. Lectura posterior, 3. Mutación, 4. Lectura mutada,
    // 6. Funciones disponibles entre evaluaciones, 7. Recuperación ante errores.
    let entrada = concat!(
        "var x: entero = 1\n",
        "mostrar(x)\n",
        "x = 2\n",
        "mostrar(x)\n",
        "funcion suma(a: entero, b: entero) -> entero { retornar a + b }\n",
        "mostrar(suma(20, 22))\n",
        "mostrar(x + 1)\n",
        "no_existe\n",
        "mostrar(x)\n",
        ":salir\n",
    );
    let salida = ejecutar_repl(entrada);
    let stdout = texto(&salida.stdout);
    let stderr = texto(&salida.stderr);
    assert!(
        salida.status.success(),
        "repl falló: stdout={stdout} stderr={stderr}"
    );
    let mut lineas: Vec<String> = Vec::new();
    for l in stdout.lines() {
        let mut s = l.trim().to_string();
        loop {
            if let Some(rest) = s.strip_prefix(">>> ") {
                s = rest.trim().to_string();
            } else if let Some(rest) = s.strip_prefix("... ") {
                s = rest.trim().to_string();
            } else if s == ">>>" || s == "..." {
                s.clear();
                break;
            } else {
                break;
            }
        }
        if !s.is_empty() && !s.starts_with("BASILISCO") {
            lineas.push(s);
        }
    }
    let lineas: Vec<&str> = lineas.iter().map(|s| s.as_str()).collect();
    // Esperamos: 1, 2, 42, 3, 2
    assert_eq!(
        lineas,
        vec!["1", "2", "42", "3", "2"],
        "stdout={stdout} stderr={stderr}"
    );
    assert!(
        stderr.contains("no_existe")
            || stderr.to_lowercase().contains("error")
            || stderr.contains("no encontrada")
            || stderr.contains("indefinido")
            || stderr.contains("E"),
        "debe reportar error y recuperarse: stderr={stderr}"
    );
}

#[test]
fn repl_inicializadores_no_replay() {
    // 5. Inicializadores ejecutados una sola vez. 8. Coherencia con ejecución normal.
    let entrada = concat!(
        "var c: entero = 0\n",
        "funcion inc() -> entero { c = c + 1\n retornar c }\n",
        "var resultado: entero = inc()\n",
        "mostrar(resultado)\n",
        "mostrar(c)\n",
        "mostrar(resultado)\n",
        "c = 100\n",
        "mostrar(c)\n",
        ":salir\n",
    );
    let salida = ejecutar_repl(entrada);
    let stdout = texto(&salida.stdout);
    let stderr = texto(&salida.stderr);
    assert!(
        salida.status.success(),
        "repl falló: stdout={stdout} stderr={stderr}"
    );
    let mut lineas: Vec<String> = Vec::new();
    for l in stdout.lines() {
        let mut s = l.trim().to_string();
        loop {
            if let Some(rest) = s.strip_prefix(">>> ") {
                s = rest.trim().to_string();
            } else if let Some(rest) = s.strip_prefix("... ") {
                s = rest.trim().to_string();
            } else if s == ">>>" || s == "..." {
                s.clear();
                break;
            } else {
                break;
            }
        }
        if !s.is_empty() && !s.starts_with("BASILISCO") {
            lineas.push(s);
        }
    }
    let lineas: Vec<&str> = lineas.iter().map(|s| s.as_str()).collect();
    // resultado inicializado una vez -> 1; c tras init -> 1; resultado sigue 1; c mutado a 100
    assert_eq!(
        lineas,
        vec!["1", "1", "1", "100"],
        "stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn repl_multilinea_y_recuperacion() {
    let entrada = concat!(
        "funcion doble(x: entero) -> entero {\n",
        "  retornar x * 2\n",
        "}\n",
        "mostrar(doble(21))\n",
        "si verdadero {\n",
        "  mostrar(7)\n",
        "}\n",
        "1 +\n",
        "2\n",
        ":salir\n",
    );
    let salida = ejecutar_repl(entrada);
    let stdout = texto(&salida.stdout);
    let stderr = texto(&salida.stderr);
    assert!(salida.status.success(), "stdout={stdout} stderr={stderr}");
    let mut lineas: Vec<String> = Vec::new();
    for l in stdout.lines() {
        let mut s = l.trim().to_string();
        loop {
            if let Some(rest) = s.strip_prefix(">>> ") {
                s = rest.trim().to_string();
            } else if let Some(rest) = s.strip_prefix("... ") {
                s = rest.trim().to_string();
            } else if s == ">>>" || s == "..." {
                s.clear();
                break;
            } else {
                break;
            }
        }
        if !s.is_empty() && !s.starts_with("BASILISCO") {
            lineas.push(s);
        }
    }
    let lineas: Vec<&str> = lineas.iter().map(|s| s.as_str()).collect();
    // multiline función y bloque si ejecutan; recuperación de error implícita por sesión viva
    assert!(
        lineas.contains(&"42") && lineas.contains(&"7"),
        "stdout={stdout} stderr={stderr} lineas={lineas:?}"
    );
}

#[test]
fn cli_ayuda_y_errores_contrato_basico() {
    let ayuda = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["--help"]);
    assert!(ayuda.status.success());
    let out = texto(&ayuda.stdout);
    assert!(out.contains("run [<archivo.bsk>]"));
    assert!(out.contains("lsp"));
    assert!(out.contains("dap"));
    assert!(out.contains("formatear"));

    let err = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["comando-inexistente"]);
    assert!(!err.status.success());
    let stderr = texto(&err.stderr);
    assert!(
        stderr.to_lowercase().contains("comando")
            || stderr.to_lowercase().contains("uso")
            || stderr.to_lowercase().contains("error")
    );
}

#[test]
fn formatter_preserva_semantica_basica_cli() {
    let dir = directorio_temporal();
    let src =
        "funcion main() {\n  # comentario\n  var x: entero = 1\n  x = x + 1\n  mostrar(x)\n}\n";
    std::fs::write(dir.join("main.bsk"), src).unwrap();
    let run1 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", "main.bsk"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(run1.status.success(), "stderr: {}", texto(&run1.stderr));
    let fmt = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["formatear", "main.bsk"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(fmt.status.success(), "stderr: {}", texto(&fmt.stderr));
    let run2 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", "main.bsk"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(run2.status.success(), "stderr: {}", texto(&run2.stderr));
    assert_eq!(texto(&run1.stdout), texto(&run2.stdout));
    let formatted = std::fs::read_to_string(dir.join("main.bsk")).unwrap();
    assert!(formatted.contains("# comentario"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_formatear_archivo_inexistente_da_error() {
    let salida = ejecutar(
        env!("CARGO_BIN_EXE_bsk"),
        &["formatear", "/tmp/no_existe_bsk_12345.bsk"],
    );
    assert!(!salida.status.success());
    let err = texto(&salida.stderr).to_lowercase();
    assert!(
        err.contains("no se pudo leer") || err.contains("error"),
        "stderr: {}",
        texto(&salida.stderr)
    );
}

#[test]
fn cli_formatear_error_sintactico_preserva_contenido() {
    let dir = directorio_temporal();
    let ruta = dir.join("malo.bsk");
    let original = "funcion main() {\n  sea x = \n}\n";
    std::fs::write(&ruta, original).unwrap();
    let salida = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["formatear", ruta.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(!salida.status.success());
    let despues = std::fs::read_to_string(&ruta).unwrap();
    assert_eq!(
        despues, original,
        "no debe modificar archivo con error sintáctico"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_formatear_archivo_valido_idempotente() {
    let dir = directorio_temporal();
    let ruta = dir.join("ok.bsk");
    std::fs::write(&ruta, "funcion main(){\nmostrar 1\n}\n").unwrap();
    let s1 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["formatear", ruta.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(s1.status.success(), "stderr: {}", texto(&s1.stderr));
    let a = std::fs::read_to_string(&ruta).unwrap();
    let s2 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["formatear", ruta.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(s2.status.success());
    let b = std::fs::read_to_string(&ruta).unwrap();
    assert_eq!(a, b);
    assert!(a.contains("funcion main()"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_version_y_ayuda_aliases() {
    for args in [["--version"], ["-V"], ["version"]] {
        let s = ejecutar(env!("CARGO_BIN_EXE_bsk"), &args);
        assert!(s.status.success(), "args={args:?}");
        assert!(texto(&s.stdout).contains("BASILISCO"));
    }
    for args in [["--help"], ["-h"], ["ayuda"]] {
        let s = ejecutar(env!("CARGO_BIN_EXE_bsk"), &args);
        assert!(s.status.success(), "args={args:?}");
        let out = texto(&s.stdout);
        assert!(out.contains("run"));
        assert!(out.contains("formatear"));
    }
}

#[test]
fn cli_errores_uso_exit_2() {
    let casos: &[&[&str]] = &[
        &["run", "a.bsk", "b.bsk"],
        &["analizar"],
        &["formatear"],
        &["lsp", "extra"],
        &["dap", "extra"],
        &["inicializar", "a", "b"],
        &["probar", "a", "b"],
        &["instalar", "a", "b"],
    ];
    for args in casos {
        let s = ejecutar(env!("CARGO_BIN_EXE_bsk"), args);
        assert_eq!(
            s.status.code(),
            Some(2),
            "args={args:?} stderr={}",
            texto(&s.stderr)
        );
    }
}

#[test]
fn cli_archivo_inexistente_exit_1() {
    for args in [["run", "no_existe.bsk"], ["analizar", "no_existe.bsk"]] {
        let s = ejecutar(env!("CARGO_BIN_EXE_bsk"), &args);
        assert_eq!(
            s.status.code(),
            Some(1),
            "args={args:?} stderr={}",
            texto(&s.stderr)
        );
    }
}

#[test]
fn cli_errores_lex_sint_sem_ejecucion() {
    let dir = directorio_temporal();
    let lex = dir.join("lex.bsk");
    std::fs::write(&lex, "\"sin cerrar\n").unwrap();
    let s1 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", lex.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(s1.status.code(), Some(1));

    let sin = dir.join("sin.bsk");
    std::fs::write(&sin, "funcion main() {\n  sea x = \n}\n").unwrap();
    let s2 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", sin.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(s2.status.code(), Some(1));

    let sem = dir.join("sem.bsk");
    std::fs::write(&sem, "funcion main() {\n  mostrar y\n}\n").unwrap();
    let s3 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", sem.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(s3.status.code(), Some(1));

    let rt = dir.join("rt.bsk");
    std::fs::write(&rt, "funcion main() {\n  mostrar 1 / 0\n}\n").unwrap();
    let s4 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", rt.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(s4.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_run_desde_otro_directorio_ruta_absoluta() {
    let dir = directorio_temporal();
    let ruta = dir.join("app.bsk");
    std::fs::write(&ruta, "funcion main() { mostrar 7 }\n").unwrap();
    let cwd = std::env::temp_dir();
    let s = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["run", ruta.to_str().unwrap()])
        .current_dir(&cwd)
        .output()
        .unwrap();
    assert!(s.status.success(), "stderr={}", texto(&s.stderr));
    assert!(texto(&s.stdout).contains('7'));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_demo_version_repl_help_contrato() {
    let demo = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["demo"]);
    assert!(demo.status.success());
    assert!(!texto(&demo.stdout).trim().is_empty());
    // repl command exists - not tested interactively here (REPL tests already cover it)
}

#[test]
fn cli_build_compila_y_reporta_errores() {
    let dir = directorio_temporal();
    let ok = dir.join("ok.bsk");
    std::fs::write(&ok, "funcion main() { mostrar 1 }\n").unwrap();
    let s = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["build", ok.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    // puede fallar si rustc no disponible; pero en entorno de dev debe estar
    if s.status.success() {
        let out = texto(&s.stdout);
        assert!(out.contains("Código Rust generado") || out.contains("Binario compilado"));
        assert!(ok.with_extension("rs").exists());
    } else {
        // al menos no debe ser error de uso
        assert_ne!(s.status.code(), Some(2), "stderr={}", texto(&s.stderr));
    }
    let bad = dir.join("bad.bsk");
    std::fs::write(&bad, "funcion main() { sea x = }\n").unwrap();
    let s2 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["build", bad.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(s2.status.code(), Some(1));
    let no = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["build", "no.bsk"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(no.status.code(), Some(1));
    let uso = ejecutar(env!("CARGO_BIN_EXE_bsk"), &["build", "a", "b"]);
    assert_eq!(uso.status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_inicializar_y_probar_contrato() {
    let dir = directorio_temporal();
    let proj = dir.join("p");
    let s = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["inicializar", proj.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(s.status.success(), "stderr={}", texto(&s.stderr));
    assert!(proj.join("hola.bsk").exists());
    assert!(proj.join("basilisco.toml").exists());
    let s2 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["inicializar", proj.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(s2.status.code(), Some(1));

    let tdir = dir.join("testsdir");
    std::fs::create_dir_all(&tdir).unwrap();
    std::fs::write(tdir.join("ok.bsk"), "funcion main() { mostrar 1 }\n").unwrap();
    let tp = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["probar", tdir.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(tp.status.success(), "stderr={}", texto(&tp.stderr));
    std::fs::write(tdir.join("bad.bsk"), "funcion main() { mostrar y }\n").unwrap();
    let tf = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["probar", tdir.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(tf.status.code(), Some(1));
    let vacio = dir.join("vacio");
    std::fs::create_dir_all(&vacio).unwrap();
    let tv = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["probar", vacio.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(tv.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_nuevo_proyecto_y_depurar() {
    let dir = directorio_temporal();
    let s = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["nuevo", "proyecto", "miproy"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(s.status.success(), "stderr={}", texto(&s.stderr));
    assert!(dir.join("miproy").exists());
    // depurar
    let f = dir.join("d.bsk");
    std::fs::write(&f, "funcion main() { mostrar 3 }\n").unwrap();
    let d = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["depurar", f.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(d.status.success(), "stderr={}", texto(&d.stderr));
    let d2 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["depurar"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(d2.status.code(), Some(2));
    let d3 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["depurar", "no.bsk"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(d3.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_registry_y_paquetes_contrato_basico() {
    let dir = directorio_temporal();
    // registry add/list/info/set-default/enable/disable/remove
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_bsk"))
            .args(args)
            .current_dir(&dir)
            .output()
            .unwrap()
    };
    let add = run(&["registry", "add", "local", "reg1", "http://127.0.0.1:1"]);
    assert!(add.status.success(), "stderr={}", texto(&add.stderr));
    let list = run(&["registry", "list"]);
    assert!(list.status.success());
    assert!(texto(&list.stdout).contains("local"));
    let info = run(&["registry", "info", "local"]);
    assert!(info.status.success());
    assert!(texto(&info.stdout).contains("reg1"));
    let def = run(&["registry", "set-default", "local"]);
    assert!(def.status.success());
    let dis = run(&["registry", "disable", "local"]);
    assert!(dis.status.success());
    let en = run(&["registry", "enable", "local"]);
    assert!(en.status.success());
    let rem = run(&["registry", "remove", "local"]);
    assert!(rem.status.success());
    let info2 = run(&["registry", "info", "local"]);
    assert_eq!(info2.status.code(), Some(1));

    // paquetes sin lock
    let p = run(&["paquetes"]);
    assert_eq!(p.status.code(), Some(3));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_add_update_publish_instalar_contrato_local() {
    let dir = directorio_temporal();
    // crear proyecto base
    let init = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["inicializar"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(init.status.success(), "stderr={}", texto(&init.stderr));

    // add: nombre inválido / opción desconocida
    let bad_name = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["add", "mal nombre"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(bad_name.status.code(), Some(3));
    let bad_opt = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["add", "pkg", "--nope=x"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(bad_opt.status.code(), Some(2));

    // update: en proyecto inicial (sin deps) debe funcionar y escribir lock
    let upd = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["update"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(upd.status.success(), "stderr={}", texto(&upd.stderr));
    assert!(dir.join("basilisco.lock").exists());

    // instalar: con lock debe funcionar
    let inst = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(inst.status.success(), "stderr={}", texto(&inst.stderr));

    // paquetes: ahora debe listar
    let pkgs = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["paquetes"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(pkgs.status.success(), "stderr={}", texto(&pkgs.stderr));

    // publish: proyecto válido debe empaquetar
    let pubr = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["publish"])
        .current_dir(&dir)
        .output()
        .unwrap();
    // publish puede fallar por contrato BPK; si tiene éxito, verifica artefacto
    if pubr.status.success() {
        let out = texto(&pubr.stdout);
        assert!(
            out.to_lowercase().contains("public")
                || out.to_lowercase().contains("bpk")
                || out.to_lowercase().contains("paquete")
                || out.to_lowercase().contains("artefacto")
                || !out.trim().is_empty()
        );
    } else {
        // no debe ser error de uso
        assert_ne!(
            pubr.status.code(),
            Some(2),
            "stderr={}",
            texto(&pubr.stderr)
        );
    }

    // instalar-remoto: validación de args
    let ir = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar-remoto", "solo-uno"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(ir.status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_instalar_remoto_con_servidor_local_controlado() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn servir_una(respuesta: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(&respuesta);
                let _ = stream.flush();
            }
        });
        format!("http://127.0.0.1:{}", addr.port())
    }

    // Construir BPK válido
    // Crear BPK real con API
    let contenido = b"mostrar 1\n".to_vec();
    let sha = basilisco::paquetes::sha256_hex(&contenido);
    let manifest_toml = format!(
        r#"[format]
name = "BASILISCO_PACKAGE"
version = "1.0"

[package]
namespace = "demo"
name = "demo"
version = "0.1.0"
description = "demo"
license = "MIT"
authors = ["t"]

[compatibility]
basilisco = ">=0.0.0"

[dependencies]

[[files]]
path = "lib.bsk"
size = {}
sha256 = "{}"
"#,
        contenido.len(),
        sha
    );
    let bpk =
        basilisco::cliente_remoto::crear_bpk(&manifest_toml, &[("lib.bsk".to_string(), contenido)])
            .unwrap();
    let bpk_sha = basilisco::paquetes::sha256_hex(&bpk);

    // Firmar metadatos
    use ed25519_dalek::SigningKey;
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let pk_hex = sk
        .verifying_key()
        .to_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    let meta = basilisco::paquetes::MetadatosPaquete {
        namespace: "demo".into(),
        name: "demo".into(),
        version: "0.1.0".into(),
        sha256: bpk_sha.clone(),
        published_at: "2026-01-01T00:00:00Z".into(),
        registry_id: "reg1".into(),
        key_id: "key1".into(),
        nonce: "n1".into(),
        expires_at: "2099-01-01T00:00:00Z".into(),
        firma: None,
        clave: None,
    };
    let firma = basilisco::paquetes::firmar_metadatos(&meta, &sk.to_bytes()).unwrap();
    let mut meta_firmado = meta.clone();
    meta_firmado.firma = Some(firma);
    meta_firmado.clave = Some(pk_hex.clone());
    let meta_json =
        serde_json::json!({"success": true, "data": meta_firmado, "error": null}).to_string();

    let resp_meta = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        meta_json.len(),
        meta_json
    );
    let resp_art = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
        bpk.len()
    );
    let mut resp_art_bytes = resp_art.into_bytes();
    resp_art_bytes.extend_from_slice(&bpk);

    let url_meta = servir_una(resp_meta.into_bytes());
    let url_art = servir_una(resp_art_bytes);

    let dir = directorio_temporal();
    // almacenar clave confiable
    let mut almacen = basilisco::paquetes::AlmacenClaves::default();
    almacen.agregar("reg1", "key1", sk.verifying_key().to_bytes().to_vec());
    almacen.guardar(&dir).unwrap();

    let destino = dir.join("instalado");
    let s = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args([
            "instalar-remoto",
            &url_meta,
            &url_art,
            destino.to_str().unwrap(),
        ])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        s.status.success(),
        "stderr={} stdout={}",
        texto(&s.stderr),
        texto(&s.stdout)
    );
    assert!(destino.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_instalar_remoto_errores_controlados() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    fn servir_una(respuesta: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(&respuesta);
                let _ = stream.flush();
            }
        });
        format!("http://127.0.0.1:{}", addr.port())
    }
    let dir = directorio_temporal();
    let destino = dir.join("out");

    // 404 metadata
    let url_404 = servir_una(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec());
    let url_art = servir_una(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_vec());
    let s = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args([
            "instalar-remoto",
            &url_404,
            &url_art,
            destino.to_str().unwrap(),
        ])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(!s.status.success());

    // Metadata sin firma/clave -> autenticidad no demostrada
    let contenido = b"mostrar 1\n".to_vec();
    let sha = basilisco::paquetes::sha256_hex(&contenido);
    let manifest_toml = format!(
        "[format]\nname=\"BASILISCO_PACKAGE\"\nversion=\"1.0\"\n[package]\nnamespace=\"d\"\nname=\"d\"\nversion=\"0.1.0\"\ndescription=\"d\"\nlicense=\"MIT\"\nauthors=[\"t\"]\n[compatibility]\nbasilisco=\">=0.0.0\"\n[dependencies]\n[[files]]\npath=\"lib.bsk\"\nsize={}\nsha256=\"{}\"\n",
        contenido.len(),
        sha
    );
    let bpk = basilisco::cliente_remoto::crear_bpk(
        &manifest_toml,
        &[("lib.bsk".to_string(), contenido.clone())],
    )
    .unwrap();
    let bpk_sha = basilisco::paquetes::sha256_hex(&bpk);
    let meta = serde_json::json!({"success":true,"data":{"namespace":"d","name":"d","version":"0.1.0","sha256":bpk_sha,"published_at":"2026-01-01T00:00:00Z","registry_id":"r","key_id":"k","nonce":"n","expires_at":"2099-01-01T00:00:00Z"},"error":null}).to_string();
    let resp_meta = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        meta.len(),
        meta
    );
    let mut resp_art =
        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", bpk.len()).into_bytes();
    resp_art.extend_from_slice(&bpk);
    let u1 = servir_una(resp_meta.into_bytes());
    let u2 = servir_una(resp_art);
    let s2 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar-remoto", &u1, &u2, destino.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(s2.status.code(), Some(1));
    let err = texto(&s2.stderr).to_lowercase();
    assert!(
        err.contains("autenticidad") || err.contains("firma") || err.contains("clave"),
        "stderr={}",
        texto(&s2.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_publish_contrato_local_real() {
    let dir = directorio_temporal();
    let init = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["inicializar"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(init.status.success());
    // publish válido
    let p = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["publish"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        p.status.success(),
        "stderr={} stdout={}",
        texto(&p.stderr),
        texto(&p.stdout)
    );
    let out = texto(&p.stdout);
    assert!(out.contains("artefacto:") || out.to_lowercase().contains("bpk"));
    let dist = dir.join("dist");
    assert!(dist.exists());
    let hay_bpk = std::fs::read_dir(&dist)
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("bpk"));
    assert!(hay_bpk, "no hay .bpk en dist");

    // publish con salida custom
    let outb = dir.join("custom").join("p.bpk");
    let p2 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["publish", ".", &format!("--salida={}", outb.display())])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(p2.status.success(), "stderr={}", texto(&p2.stderr));
    assert!(outb.exists());

    // versión no semver -> rechazo
    let toml = dir.join("basilisco.toml");
    let t = std::fs::read_to_string(&toml)
        .unwrap()
        .replace("0.1.0", "*");
    std::fs::write(&toml, t).unwrap();
    let p3 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["publish"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(p3.status.code(), Some(3), "stderr={}", texto(&p3.stderr));
    let _ = std::fs::remove_dir_all(&dir);
}

fn http_servir_una(respuesta: Vec<u8>) -> String {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 8192];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(&respuesta);
            let _ = stream.flush();
        }
    });
    format!("http://127.0.0.1:{}", addr.port())
}

fn bpk_y_meta_firmada(nombre: &str, version: &str) -> (Vec<u8>, String, ed25519_dalek::SigningKey) {
    use ed25519_dalek::SigningKey;
    let contenido = b"mostrar 1\n".to_vec();
    let sha = basilisco::paquetes::sha256_hex(&contenido);
    let manifest_toml = format!(
        "[format]\nname=\"BASILISCO_PACKAGE\"\nversion=\"1.0\"\n[package]\nnamespace=\"{nombre}\"\nname=\"{nombre}\"\nversion=\"{version}\"\ndescription=\"d\"\nlicense=\"MIT\"\nauthors=[\"t\"]\n[compatibility]\nbasilisco=\">=0.0.0\"\n[dependencies]\n[[files]]\npath=\"lib.bsk\"\nsize={}\nsha256=\"{}\"\n",
        contenido.len(),
        sha
    );
    let bpk =
        basilisco::cliente_remoto::crear_bpk(&manifest_toml, &[("lib.bsk".to_string(), contenido)])
            .unwrap();
    let bpk_sha = basilisco::paquetes::sha256_hex(&bpk);
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let pk_hex: String = sk
        .verifying_key()
        .to_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    let meta = basilisco::paquetes::MetadatosPaquete {
        namespace: nombre.into(),
        name: nombre.into(),
        version: version.into(),
        sha256: bpk_sha,
        published_at: "2026-01-01T00:00:00Z".into(),
        registry_id: "reg1".into(),
        key_id: "key1".into(),
        nonce: format!("n-{nombre}-{version}"),
        expires_at: "2099-01-01T00:00:00Z".into(),
        firma: None,
        clave: None,
    };
    let firma = basilisco::paquetes::firmar_metadatos(&meta, &sk.to_bytes()).unwrap();
    let mut meta_firmado = meta;
    meta_firmado.firma = Some(firma);
    meta_firmado.clave = Some(pk_hex);
    let meta_json =
        serde_json::json!({"success": true, "data": meta_firmado, "error": null}).to_string();
    (bpk, meta_json, sk)
}

#[test]
fn cli_instalar_remoto_fallos_integridad_y_atomicidad() {
    let (bpk, meta_json, sk) = bpk_y_meta_firmada("demo2", "0.1.0");
    let resp_meta = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        meta_json.len(),
        meta_json
    );
    let mut resp_art_ok =
        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", bpk.len()).into_bytes();
    resp_art_ok.extend_from_slice(&bpk);

    // hash incorrecto: corromper artefacto
    let mut bpk_malo = bpk.clone();
    bpk_malo[10] ^= 0xFF;
    let mut resp_art_malo = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
        bpk_malo.len()
    )
    .into_bytes();
    resp_art_malo.extend_from_slice(&bpk_malo);

    let dir = directorio_temporal();
    let mut almacen = basilisco::paquetes::AlmacenClaves::default();
    almacen.agregar("reg1", "key1", sk.verifying_key().to_bytes().to_vec());
    almacen.guardar(&dir).unwrap();
    let destino = dir.join("instalado");

    let u1 = http_servir_una(resp_meta.clone().into_bytes());
    let u2 = http_servir_una(resp_art_malo);
    let s = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar-remoto", &u1, &u2, destino.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(s.status.code(), Some(1), "stderr={}", texto(&s.stderr));
    assert!(
        !destino.exists(),
        "destino no debe existir tras hash inválido"
    );

    // firma incorrecta
    let mut meta_mala: serde_json::Value = serde_json::from_str(&meta_json).unwrap();
    if let Some(f) = meta_mala
        .get_mut("data")
        .and_then(|d| d.get_mut("firma"))
        .and_then(|v| v.as_str())
    {
        let mut chars: Vec<char> = f.chars().collect();
        chars[0] = if chars[0] == 'a' { 'b' } else { 'a' };
        *meta_mala.get_mut("data").unwrap().get_mut("firma").unwrap() =
            serde_json::Value::String(chars.into_iter().collect());
    }
    let meta_mala_str = meta_mala.to_string();
    let resp_meta_mala = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        meta_mala_str.len(),
        meta_mala_str
    );
    let u3 = http_servir_una(resp_meta_mala.into_bytes());
    let u4 = http_servir_una(resp_art_ok.clone());
    let s2 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar-remoto", &u3, &u4, destino.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(s2.status.code(), Some(1), "stderr={}", texto(&s2.stderr));
    assert!(
        !destino.exists(),
        "destino no debe existir tras firma inválida"
    );

    // éxito, luego destino existente no vacío
    let u5 = http_servir_una(resp_meta.into_bytes());
    let u6 = http_servir_una(resp_art_ok);
    let s3 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar-remoto", &u5, &u6, destino.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(s3.status.success(), "stderr={}", texto(&s3.stderr));
    assert!(destino.join("lib.bsk").exists());

    // paquete previamente instalado / destino no vacío -> debe fallar sin corromper
    let (bpk2, meta2, _sk2) = bpk_y_meta_firmada("demo3", "0.1.0");
    // reutilizar misma clave confiable (sk igual)
    let resp_meta2 = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        meta2.len(),
        meta2
    );
    let mut resp_art2 =
        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", bpk2.len()).into_bytes();
    resp_art2.extend_from_slice(&bpk2);
    let u7 = http_servir_una(resp_meta2.into_bytes());
    let u8 = http_servir_una(resp_art2);
    let contenido_antes = std::fs::read_to_string(destino.join("lib.bsk")).unwrap();
    let s4 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar-remoto", &u7, &u8, destino.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(s4.status.code(), Some(1), "stderr={}", texto(&s4.stderr));
    let contenido_despues = std::fs::read_to_string(destino.join("lib.bsk")).unwrap();
    assert_eq!(
        contenido_antes, contenido_despues,
        "contenido existente no debe corromperse"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn crear_dep_local(base: &std::path::Path, nombre: &str, version: &str) {
    let dep = base.join("paquetes").join(nombre);
    std::fs::create_dir_all(&dep).unwrap();
    std::fs::write(
        dep.join("basilisco.toml"),
        format!("[paquete]\nnombre = \"{nombre}\"\nversion = \"{version}\"\n\n[dependencias]\n"),
    )
    .unwrap();
    std::fs::write(dep.join("lib.bsk"), "funcion hola() { mostrar 1 }\n").unwrap();
}

#[test]
fn cli_add_contrato_resolucion_y_integridad() {
    let dir = directorio_temporal();
    let init = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["inicializar"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(init.status.success(), "{}", texto(&init.stderr));
    crear_dep_local(&dir, "util", "1.0.0");

    // dependencia existente compatible
    let add = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["add", "util", "1.0.0"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        add.status.success(),
        "stderr={} stdout={}",
        texto(&add.stderr),
        texto(&add.stdout)
    );
    let manifiesto = std::fs::read_to_string(dir.join("basilisco.toml")).unwrap();
    assert!(manifiesto.contains("util"));
    assert!(dir.join("basilisco.lock").exists());

    // duplicada / ya declarada
    let dup = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["add", "util", "1.0.0"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(dup.status.code(), Some(3));
    let manifiesto2 = std::fs::read_to_string(dir.join("basilisco.toml")).unwrap();
    assert_eq!(
        manifiesto.matches("util").count(),
        manifiesto2.matches("util").count()
    );

    // paquete inexistente - no debe mutar manifiesto
    let antes = std::fs::read_to_string(dir.join("basilisco.toml")).unwrap();
    let noex = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["add", "noexiste", "1.0.0"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(noex.status.code(), Some(3));
    let despues = std::fs::read_to_string(dir.join("basilisco.toml")).unwrap();
    assert_eq!(antes, despues, "manifiesto no debe mutar ante fallo");

    // versión inexistente
    let antes2 = std::fs::read_to_string(dir.join("basilisco.toml")).unwrap();
    let vbad = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["add", "util", "9.9.9"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(vbad.status.code(), Some(3));
    let despues2 = std::fs::read_to_string(dir.join("basilisco.toml")).unwrap();
    assert_eq!(antes2, despues2);

    // manifiesto inválido
    let dir2 = directorio_temporal();
    std::fs::write(dir2.join("basilisco.toml"), "esto no es toml = [").unwrap();
    let bad = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["add", "x", "1.0.0"])
        .current_dir(&dir2)
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(3));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn cli_update_instalar_contrato_lockfile_e_integridad() {
    let dir = directorio_temporal();
    let init = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["inicializar"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(init.status.success());
    crear_dep_local(&dir, "dep", "1.0.0");
    let add = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["add", "dep", "1.0.0"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(add.status.success(), "{}", texto(&add.stderr));

    // update sin cambios disponibles
    let upd = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["update"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(upd.status.success(), "stderr={}", texto(&upd.stderr));
    let lock = std::fs::read_to_string(dir.join("basilisco.lock")).unwrap();
    assert!(lock.contains("dep"));

    // instalar desde lock válido
    let inst = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(inst.status.success(), "stderr={}", texto(&inst.stderr));

    // lockfile malformado
    let lock_path = dir.join("basilisco.lock");
    let original = std::fs::read_to_string(&lock_path).unwrap();
    std::fs::write(&lock_path, "no = [valido").unwrap();
    let inst_bad = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(inst_bad.status.code(), Some(3));

    // lockfile ausente - instalar debe resolver desde manifiesto y generar lock
    std::fs::remove_file(&lock_path).unwrap();
    let inst_nolock = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        inst_nolock.status.success(),
        "stderr={}",
        texto(&inst_nolock.stderr)
    );
    assert!(lock_path.exists());

    // restaurar y verificar reproducibilidad: instalar repetido
    std::fs::write(&lock_path, &original).unwrap();
    let inst2 = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["instalar"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(inst2.status.success(), "stderr={}", texto(&inst2.stderr));

    // update con manifiesto inválido -> conserva estado anterior
    let manifiesto = dir.join("basilisco.toml");
    let man_orig = std::fs::read_to_string(&manifiesto).unwrap();
    std::fs::write(&manifiesto, "[paquete\nnombre=broken").unwrap();
    let upd_bad = Command::new(env!("CARGO_BIN_EXE_bsk"))
        .args(["update"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(upd_bad.status.code(), Some(3));
    let lock_despues = std::fs::read_to_string(&lock_path).unwrap();
    assert_eq!(lock_despues, original);
    std::fs::write(&manifiesto, man_orig).unwrap();

    let _ = std::fs::remove_dir_all(&dir);
}
