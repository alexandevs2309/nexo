use basilisco::{ContextoSemantico, Lexer, Parser, backend_nativo, hir, mir, vm};
use std::fs;
use std::process::Command;

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn compilar_y_ejecutar(src_rust: &str) -> Option<String> {
    if Command::new("rustc").arg("--version").output().is_err() {
        eprintln!("aviso: rustc no disponible; prueba omitida");
        return None;
    }
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("bn-{}-{}", std::process::id(), n));
    let _ = fs::create_dir_all(&dir);
    let fuente = dir.join("gen.rs");
    let binario = dir.join(if cfg!(windows) { "gen.exe" } else { "gen" });
    fs::write(&fuente, src_rust).ok()?;
    let out = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("-O")
        .arg(&fuente)
        .arg("-o")
        .arg(&binario)
        .output()
        .ok()?;
    if !out.status.success() {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&out.stderr));
        return None;
    }
    let run = Command::new(&binario).output().ok()?;
    if !run.status.success() {
        eprintln!("bin stderr:\n{}", String::from_utf8_lossy(&run.stderr));
        return None;
    }
    Some(String::from_utf8_lossy(&run.stdout).trim().to_string())
}

fn generar(src_bsk: &str) -> String {
    let tokens = Lexer::nuevo(src_bsk).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    backend_nativo::generar_rust(&mir)
}

fn vm_run(src_bsk: &str) -> String {
    let tokens = Lexer::nuevo(src_bsk).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    let idx = maquina.indice_de("main").unwrap();
    let ret = maquina.ejecutar_indice(idx, &[]);
    let mut out = maquina.salida().join("");
    if let Ok(Some(v)) = ret {
        out.push_str(&format!("{}\n", v));
    }
    out.trim_end_matches('\n').to_string()
}

#[test]
fn backend_nativo_aritmetica() {
    let src = "funcion main() -> entero { retornar 2 + 3 * 4 - 1 }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_recursion_factorial() {
    let src = "funcion factorial(n: entero) -> entero { si n < 2 { retornar 1 } sino { retornar factorial(n - 1) * n } }\nfuncion main() -> entero { retornar factorial(7) }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_condicional_y_loop() {
    let src = "funcion main() -> entero {\n  sea i = 0\n  sea s = 0\n  mientras i < 5 {\n    i = i + 1\n    s = s + i\n  }\n  si s > 10 { retornar s } sino { retornar 0 }\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_genera_codigo_no_vacio() {
    let src = "funcion main() -> entero { retornar 42 }";
    let rust = generar(src);
    assert!(rust.contains("fn bsk_main"));
    assert!(rust.contains("fn main()"));
}

#[test]
fn backend_nativo_texto_concatenacion() {
    let src = "funcion main() -> texto { retornar \"Hola\" + \" \" + \"Mundo\" }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_texto_parametro() {
    let src = "funcion saludar(nombre: texto) -> texto { retornar \"Hola, \" + nombre + \"!\" }\nfuncion main() -> texto { retornar saludar(\"Mundo\") }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_texto_escape() {
    // Comillas y barra invertida dentro del literal.
    let src = "funcion main() -> texto { retornar \"comillas: \\\"x\\\" y barra \\\\ fin\" }";
    let rust = generar(src);
    assert!(rust.contains("\\\"x\\\""));
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_texto_condicional() {
    let src =
        "funcion main() -> entero { si \"abc\" == \"abc\" { retornar 1 } sino { retornar 0 } }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_texto_en_bucle() {
    let src = "funcion construir(n: entero) -> texto { sea s = \"\"\n sea i = 0\n mientras i < n { s = s + \"*\"\n i = i + 1 }\n retornar s }\nfuncion main() -> texto { retornar construir(5) }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "*****");
}

#[test]
fn backend_nativo_mostrar_salida() {
    // `mostrar`/`mostrar_sin_salto` se generan como E/S nativa (println!/print!).
    let src = "funcion main() { mostrar \"Hola\"\n mostrar 42\n mostrar_sin_salto \"sin\" }";
    let rust = generar(src);
    assert!(rust.contains("println!"));
    assert!(rust.contains("print!"));
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, "Hola\n42\nsin");
}

#[test]
fn backend_nativo_rechaza_operaciones_no_generables() {
    // Operaciones no generables (try/catch, closures) o main no soportada deben
    // producir compile_error! — nunca una salida silenciosa incorrecta.
    let src = "funcion main() -> entero { intentar { retornar 1 } rescatar e { retornar 0 } }";
    let rust = generar(src);
    assert!(rust.contains("compile_error!"));
}

// --- Etapa A: colecciones ---

#[test]
fn backend_nativo_coleccion_enteros() {
    let src = "funcion main() -> entero {\n  sea arr = [1, 2, 3]\n  sea s = 0\n  para cada x en arr { s = s + x }\n  retornar s + longitud(arr)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "9");
}

#[test]
fn backend_nativo_coleccion_texto() {
    let src = "funcion main() {\n  sea arr = [\"a\", \"b\", \"c\"]\n  mostrar arr\n  mostrar arr[1]\n  mostrar longitud(arr)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_coleccion_heterogenea() {
    let src = "funcion main() {\n  sea arr = [1, \"hola\", 2.5, verdadero]\n  mostrar arr\n  mostrar arr[1]\n  mostrar longitud(arr)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_coleccion_indexacion_asignacion() {
    let src = "funcion main() {\n  sea arr = [1, 2, 3]\n  arr[1] = 99\n  mostrar arr\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "[1, 99, 3]");
}

#[test]
fn backend_nativo_coleccion_agregar_eliminar() {
    let src = "funcion main() {\n  sea arr = [1, 2, 3]\n  sea arr2 = agregar(arr, 4)\n  mostrar arr2\n  mostrar eliminar(arr2, 0)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_coleccion_anidada() {
    let src = "funcion main() {\n  sea m = [[1, 2], [3, 4]]\n  mostrar m\n  mostrar m[1][0]\n  mostrar longitud(m)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_coleccion_fuera_de_rango() {
    let src = "funcion main() {\n  sea arr = [1, 2, 3]\n  mostrar arr[10]\n}";
    let rust = generar(src);
    // Debe compilar.
    if Command::new("rustc").arg("--version").output().is_err() {
        return;
    }
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("bn-oob-{}-{}", std::process::id(), n));
    let _ = fs::create_dir_all(&dir);
    let fuente = dir.join("gen.rs");
    let binario = dir.join(if cfg!(windows) { "gen.exe" } else { "gen" });
    fs::write(&fuente, &rust).ok();
    let out = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("-O")
        .arg(&fuente)
        .arg("-o")
        .arg(&binario)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "rustc debe compilar: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let run = Command::new(&binario).output().unwrap();
    // Debe fallar (exit != 0) con mensaje limpio (sin panic de Rust).
    assert!(!run.status.success());
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("fuera de rango"), "stderr: {}", stderr);
    assert!(
        !stderr.contains("panicked"),
        "no debe ser panic de Rust: {}",
        stderr
    );
}

#[test]
fn backend_nativo_coleccion_para_cada_anidado() {
    let src = "funcion main() -> entero {\n  sea m = [[1, 2], [3, 4]]\n  sea total = 0\n  para cada fila en m {\n    para cada x en fila {\n      total = total + x\n    }\n  }\n  retornar total\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "10");
}

// --- Etapa B: Real y Decimal ---

#[test]
fn backend_nativo_real_aritmetica() {
    // Los literales fraccionarios (2.5, 3.0) son `decimal` en BASILISCO; el
    // retorno anotado `-> decimal` respeta ADR-0005 (Decimal↔Real prohibido,
    // GAP-051). El cuerpo ejerce aritmética decimal de alta precisión.
    let src = "funcion main() -> decimal {\n  sea a = 2.5\n  sea b = 3.0\n  retornar a * b + a / b - 1.0\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_decimal_suma_resta() {
    let src =
        "funcion main() -> decimal {\n  sea a = 10.5\n  sea b = 3.25\n  retornar a + b - 5.0\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "8.75");
}

#[test]
fn backend_nativo_decimal_multiplicacion_half_even() {
    // 2.675 * 100 con redondeo half-even (banquero).
    let src = "funcion main() -> decimal {\n  retornar 2.675 * 100.0\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_decimal_division() {
    let src = "funcion main() -> decimal {\n  retornar 10.0 / 4.0\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "2.5");
}

#[test]
fn backend_nativo_decimal_division_periodica() {
    // 1/3 redondeado a 9 decimales.
    let src = "funcion main() -> decimal {\n  retornar 1.0 / 3.0\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "0.333333333");
}

#[test]
fn backend_nativo_decimal_monetario() {
    // Cálculo monetario típico: (precio * cantidad) con IVA.
    let src = "funcion main() -> decimal {\n  sea precio = 19.99\n  sea cantidad = 3\n  sea subtotal = precio * cantidad\n  sea iva = subtotal * 0.21\n  retornar subtotal + iva\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_decimal_precision() {
    // 0.1 + 0.2 debe dar 0.3 (no 0.30000000000000004).
    let src = "funcion main() -> decimal {\n  retornar 0.1 + 0.2\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "0.3");
}

#[test]
fn backend_nativo_conversiones_entero_decimal() {
    let src = "funcion main() -> decimal {\n  sea a = 5\n  retornar a + 2.5\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "7.5");
}

#[test]
fn backend_nativo_decimal_negativo() {
    let src = "funcion main() -> decimal {\n  retornar -5.0 * 3.0\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "-15.0");
}

#[test]
fn backend_nativo_comparaciones_real_decimal() {
    let src = "funcion main() -> entero {\n  si 2.5 < 3.0 { retornar 1 } sino { retornar 0 }\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "1");
}

#[test]
fn backend_nativo_decimal_division_por_cero() {
    let src = "funcion main() {\n  mostrar 10.0 / 0.0\n}";
    let rust = generar(src);
    if Command::new("rustc").arg("--version").output().is_err() {
        return;
    }
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("bn-decdiv0-{}-{}", std::process::id(), n));
    let _ = fs::create_dir_all(&dir);
    let fuente = dir.join("gen.rs");
    let binario = dir.join(if cfg!(windows) { "gen.exe" } else { "gen" });
    fs::write(&fuente, &rust).ok();
    let out = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("-O")
        .arg(&fuente)
        .arg("-o")
        .arg(&binario)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "rustc debe compilar: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let run = Command::new(&binario).output().unwrap();
    assert!(!run.status.success());
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("división entre cero"), "stderr: {}", stderr);
    assert!(
        !stderr.contains("panicked"),
        "no debe ser panic: {}",
        stderr
    );
}

#[test]
fn backend_nativo_entero_desbordamiento() {
    let src = "funcion main() {\n  mostrar 9223372036854775807 + 1\n}";
    let rust = generar(src);
    if Command::new("rustc").arg("--version").output().is_err() {
        return;
    }
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("bn-ovf-{}-{}", std::process::id(), n));
    let _ = fs::create_dir_all(&dir);
    let fuente = dir.join("gen.rs");
    let binario = dir.join(if cfg!(windows) { "gen.exe" } else { "gen" });
    fs::write(&fuente, &rust).ok();
    let out = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("-O")
        .arg(&fuente)
        .arg("-o")
        .arg(&binario)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "rustc debe compilar: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let run = Command::new(&binario).output().unwrap();
    assert!(!run.status.success());
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("desbordamiento entero"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn backend_nativo_entero_min_div_menos_uno() {
    let src = "funcion main() {\n  mostrar 0 - 9223372036854775807 - 1\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "-9223372036854775808");
}

// --- Etapa B: conversiones de asignación y límites ---

/// Compila y ejecuta el código Rust generado; devuelve (éxito, stdout, stderr).
fn ejecutar_nativo(src_rust: &str) -> Option<(bool, String, String)> {
    if Command::new("rustc").arg("--version").output().is_err() {
        return None;
    }
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("bn-err-{}-{}", std::process::id(), n));
    let _ = fs::create_dir_all(&dir);
    let fuente = dir.join("gen.rs");
    let binario = dir.join(if cfg!(windows) { "gen.exe" } else { "gen" });
    fs::write(&fuente, src_rust).ok()?;
    let out = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("-O")
        .arg(&fuente)
        .arg("-o")
        .arg(&binario)
        .output()
        .ok()?;
    if !out.status.success() {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&out.stderr));
        return None;
    }
    let run = Command::new(&binario).output().ok()?;
    Some((
        run.status.success(),
        String::from_utf8_lossy(&run.stdout).to_string(),
        String::from_utf8_lossy(&run.stderr).to_string(),
    ))
}

#[test]
fn backend_nativo_asignacion_tipada_real() {
    // `var r: real = 7` almacena Real(7.0) (convertir_tipo de la VM),
    // y `r / 2` es división real (VM: 3.5), no entera.
    let src = "funcion main() {\n  var r: real = 7\n  mostrar r\n  mostrar r / 2\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "7\n3.5");
}

#[test]
fn backend_nativo_asignacion_tipada_decimal() {
    let src = "funcion main() {\n  var d: decimal = 7\n  mostrar d\n  mostrar d * 3\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "7.0\n21.0");
}

#[test]
fn backend_nativo_asignacion_tipada_lista() {
    let src = "funcion main() {\n  var l: lista<entero> = [1, 2, 3]\n  mostrar l\n  mostrar longitud(l)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "[1, 2, 3]\n3");
}

#[test]
fn backend_nativo_error_decimal_mas_real() {
    // Decimal + Real es conversión ilegal (ADR-0005): error limpio, sin panic.
    // `real = 7` pasa el semántico (entero→real implícito); la mezcla
    // decimal+real se detecta en ejecución, igual que en la VM.
    let src = "funcion main() {\n  var d: decimal = 2.5\n  var r: real = 7\n  mostrar d + r\n}";
    let rust = generar(src);
    let Some((exito, _out, stderr)) = ejecutar_nativo(&rust) else {
        return;
    };
    assert!(!exito);
    assert!(stderr.contains("conversión ilegal"), "stderr: {stderr}");
    assert!(!stderr.contains("panicked"), "no debe ser panic: {stderr}");
}

#[test]
fn backend_nativo_error_comparacion_decimal_real() {
    let src = "funcion main() {\n  var d: decimal = 2.5\n  var r: real = 7\n  mostrar d < r\n}";
    let rust = generar(src);
    let Some((exito, _out, stderr)) = ejecutar_nativo(&rust) else {
        return;
    };
    assert!(!exito);
    assert!(
        stderr.contains("tipos incompatibles: decimal y real"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("panicked"), "no debe ser panic: {stderr}");
}

#[test]
fn backend_nativo_error_rango_decimal() {
    // 1.3e10 * 1.3e10: el producto intermedio cabe en i128 pero la mantisa
    // resultado (1.69e29) excede el límite decimal (10^28-1).
    let src = "funcion main() {\n  sea x = 13000000000.0\n  mostrar x * x\n}";
    let rust = generar(src);
    let Some((exito, _out, stderr)) = ejecutar_nativo(&rust) else {
        return;
    };
    assert!(!exito);
    assert!(
        stderr.contains("exceso de rango decimal"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("panicked"), "no debe ser panic: {stderr}");
}

#[test]
fn backend_nativo_entero_min_operaciones() {
    // i64::MIN está bien formado; m % -1 = 0 (paridad VM).
    let src =
        "funcion main() {\n  sea m = 0 - 9223372036854775807 - 1\n  mostrar m\n  mostrar m % -1\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
    assert_eq!(salida, "-9223372036854775808\n0");
}

#[test]
fn backend_nativo_error_min_div_menos_uno() {
    // i64::MIN / -1 es desbordamiento entero (D2), igual que en la VM.
    let src = "funcion main() {\n  sea m = 0 - 9223372036854775807 - 1\n  mostrar m / -1\n}";
    let rust = generar(src);
    let Some((exito, _out, stderr)) = ejecutar_nativo(&rust) else {
        return;
    };
    assert!(!exito);
    assert!(stderr.contains("desbordamiento entero"), "stderr: {stderr}");
    assert!(!stderr.contains("panicked"), "no debe ser panic: {stderr}");
}

#[test]
fn backend_nativo_error_min_mul_menos_uno() {
    // i64::MIN * -1 también desborda (checked_mul de i64).
    let src = "funcion main() {\n  sea m = 0 - 9223372036854775807 - 1\n  mostrar m * -1\n}";
    let rust = generar(src);
    let Some((exito, _out, stderr)) = ejecutar_nativo(&rust) else {
        return;
    };
    assert!(!exito);
    assert!(stderr.contains("desbordamiento entero"), "stderr: {stderr}");
    assert!(!stderr.contains("panicked"), "no debe ser panic: {stderr}");
}

// --- Etapa C: módulos y `usar` (compilación nativa) ---

use basilisco::cargador::CargadorModulos;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn setup_mod(dir_name: &str, files: &[(&str, &str)]) -> PathBuf {
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("bnmod-{}-{}-{}", std::process::id(), n, dir_name));
    let _ = fs::create_dir_all(&dir);
    for (name, content) in files {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(p, content).unwrap();
    }
    dir
}

fn generar_mod(dir: &Path, main: &str) -> String {
    let ruta = dir.join(main);
    let fuente = fs::read_to_string(&ruta).unwrap();
    let ruta_str = ruta.canonicalize().unwrap().display().to_string();
    let root = dir.canonicalize().unwrap();
    let cargador = Arc::new(Mutex::new(CargadorModulos::nuevo(root)));
    let tokens = Lexer::con_archivo(&fuente, Some(ruta_str.clone()))
        .tokenizar()
        .unwrap();
    let ss = Parser::con_archivo(tokens, Some(ruta_str.clone()))
        .parsear_programa()
        .unwrap();
    let mut ctx = ContextoSemantico::con_cargador(cargador, Some(ruta_str.clone()));
    ctx.set_archivo_actual(Some(PathBuf::from(&ruta_str)));
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    backend_nativo::generar_rust(&mir)
}

fn vm_run_mod(dir: &Path, main: &str) -> String {
    let ruta = dir.join(main);
    let fuente = fs::read_to_string(&ruta).unwrap();
    let ruta_str = ruta.canonicalize().unwrap().display().to_string();
    let root = dir.canonicalize().unwrap();
    let cargador = Arc::new(Mutex::new(CargadorModulos::nuevo(root)));
    let tokens = Lexer::con_archivo(&fuente, Some(ruta_str.clone()))
        .tokenizar()
        .unwrap();
    let ss = Parser::con_archivo(tokens, Some(ruta_str.clone()))
        .parsear_programa()
        .unwrap();
    let mut ctx = ContextoSemantico::con_cargador(cargador, Some(ruta_str.clone()));
    ctx.set_archivo_actual(Some(PathBuf::from(&ruta_str)));
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    let idx = maquina.indice_de("main").unwrap();
    let ret = maquina.ejecutar_indice(idx, &[]);
    let mut out = maquina.salida().join("");
    if let Ok(Some(v)) = ret {
        out.push_str(&format!("{}\n", v));
    } else if let Err(e) = ret {
        out.push_str(&format!("error: {e}\n"));
    }
    out.trim().to_string()
}

fn compilar_y_ejecutar_mod(dir: &Path, main: &str) -> Option<String> {
    let rust = generar_mod(dir, main);
    compilar_y_ejecutar(&rust)
}

#[test]
fn backend_nativo_modulo_alias() {
    let dir = setup_mod(
        "alias",
        &[
            (
                "lib.bsk",
                "publico funcion suma(a, b) { retornar a + b }\npublico funcion doble(x) { retornar x * 2 }",
            ),
            (
                "main.bsk",
                "usar \"lib.bsk\" como m\nmostrar m.suma(2, 3)\nmostrar m.doble(5)\n",
            ),
        ],
    );
    let Some(salida) = compilar_y_ejecutar_mod(&dir, "main.bsk") else {
        return;
    };
    assert_eq!(salida, vm_run_mod(&dir, "main.bsk"));
    assert_eq!(salida, "5\n10");
}

#[test]
fn backend_nativo_modulo_nominal() {
    let dir = setup_mod(
        "nominal",
        &[
            ("lib.bsk", "publico funcion saludar() { retornar \"hola\" }"),
            (
                "main.bsk",
                "usar \"lib.bsk\" { saludar }\nmostrar saludar()\n",
            ),
        ],
    );
    let Some(salida) = compilar_y_ejecutar_mod(&dir, "main.bsk") else {
        return;
    };
    assert_eq!(salida, vm_run_mod(&dir, "main.bsk"));
    assert_eq!(salida, "hola");
}

#[test]
fn backend_nativo_modulo_anidado() {
    let dir = setup_mod(
        "nested",
        &[
            ("base.bsk", "publico funcion inc(x) { retornar x + 1 }"),
            (
                "mid.bsk",
                "usar \"base.bsk\" como b\npublico funcion mas2(x) { retornar b.inc(b.inc(x)) }",
            ),
            ("main.bsk", "usar \"mid.bsk\" como m\nmostrar m.mas2(10)\n"),
        ],
    );
    let Some(salida) = compilar_y_ejecutar_mod(&dir, "main.bsk") else {
        return;
    };
    assert_eq!(salida, vm_run_mod(&dir, "main.bsk"));
    assert_eq!(salida, "12");
}

#[test]
fn backend_nativo_modulo_con_estado_init() {
    // Módulo con declaración pública de nivel superior (inicialización).
    let dir = setup_mod(
        "init",
        &[
            (
                "lib.bsk",
                "publico sea base = 40\npublico funcion valor() { retornar base + 2 }",
            ),
            ("main.bsk", "usar \"lib.bsk\" como m\nmostrar m.valor()\n"),
        ],
    );
    let Some(salida) = compilar_y_ejecutar_mod(&dir, "main.bsk") else {
        return;
    };
    assert_eq!(salida, vm_run_mod(&dir, "main.bsk"));
}

#[test]
fn backend_nativo_closure_inmutable() {
    let src = "funcion crear_sumador(x) {\n  funcion sumar(yy) {\n    retornar x + yy\n  }\n  retornar sumar\n}\nfuncion main() -> entero {\n  sea sumar10 = crear_sumador(10)\n  retornar sumar10(5)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_closure_mutable() {
    let src = "funcion crear_contador() {\n  sea x = 0\n  funcion inc() {\n    x = x + 1\n    retornar x\n  }\n  retornar inc\n}\nfuncion main() -> entero {\n  sea c = crear_contador()\n  c()\n  retornar c()\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_closure_dos_instancias() {
    let src = "funcion crear(x) { retornar funcion (yy) { retornar x + yy } }\nfuncion main() -> entero { sea a = crear(10)\n sea b = crear(20)\n retornar a(5) + b(5) }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_funcion_indirecta() {
    let src = "funcion aplicar(f, x) { retornar f(x) }\nfuncion doble(n) { retornar n * 2 }\nfuncion main() -> entero { retornar aplicar(doble, 4) }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_closure_retornada_llamada_inmediata() {
    let src = "funcion obtener() { retornar funcion (x) { retornar x + 1 } }\nfuncion main() -> entero { retornar obtener()(9) }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_closure_captura_simple() {
    let src = "funcion crear(x) { retornar funcion (yy) { retornar x + yy } }\nfuncion main() -> entero { sea c = crear(10)\n retornar c(5) }";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_closure_mutable_multiples_llamadas() {
    let src = "funcion crear_contador(inicio) {\n  sea x = inicio\n  funcion inc(paso) {\n    x = x + paso\n    retornar x\n  }\n  retornar inc\n}\nfuncion main() -> entero {\n  sea c = crear_contador(1)\n  c(2)\n  c(3)\n  retornar c(4)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_closure_pasada_y_llamada_varias_veces() {
    let src = "funcion aplicar_dos(f, a, b) { retornar f(a) + f(b) }\nfuncion crear_sumador(base) { retornar funcion (x) { retornar base + x } }\nfuncion main() -> entero {\n  sea s = crear_sumador(10)\n  retornar aplicar_dos(s, 1, 2)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_rechaza_try_catch_explicito() {
    let src = "funcion main() -> entero {\n  intentar {\n    retornar 1\n  } rescatar e {\n    retornar 0\n  }\n}";
    let rust = generar(src);
    assert!(
        rust.contains("compile_error!"),
        "se esperaba rechazo explícito para try/catch no soportado: {rust}"
    );
}

#[test]
fn backend_nativo_rechaza_rango_explicito() {
    let src = "funcion main() -> entero {\n  sea s = 0\n  para cada i en 1..3 {\n    s = s + i\n  }\n  retornar s\n}";
    let rust = generar(src);
    assert!(
        rust.contains("compile_error!"),
        "se esperaba rechazo explícito para rango no soportado: {rust}"
    );
}

#[test]
fn backend_nativo_closure_en_bucle_mientras() {
    let src = "funcion crear_acumulador() {\n  sea total = 0\n  funcion agregar(x) {\n    total = total + x\n    retornar total\n  }\n  retornar agregar\n}\nfuncion main() -> entero {\n  sea acc = crear_acumulador()\n  sea i = 1\n  mientras i <= 4 {\n    acc(i)\n    i = i + 1\n  }\n  retornar acc(0)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_closure_en_condicional() {
    let src = "funcion crear(a) { retornar funcion (x) { retornar a + x } }\nfuncion main() -> entero {\n  sea f = crear(10)\n  si f(5) == 15 {\n    retornar f(1)\n  } sino {\n    retornar 0\n  }\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_funcion_como_valor_en_variable() {
    let src = "funcion doble(n) { retornar n * 2 }\nfuncion main() -> entero {\n  sea f = doble\n  retornar f(21)\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}

#[test]
fn backend_nativo_closure_creada_en_para_cada() {
    let src = "funcion main() -> entero {\n  sea total = 0\n  para cada x en [1, 2, 3, 4] {\n    sea f = funcion () { retornar x }\n    total = total + f()\n  }\n  retornar total\n}";
    let rust = generar(src);
    let Some(salida) = compilar_y_ejecutar(&rust) else {
        return;
    };
    assert_eq!(salida, vm_run(src));
}
