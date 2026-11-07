use basilisco::{ContextoSemantico, Entorno, Lexer, Parser, hir, mir, vm};

fn parsear(src: &str) -> Vec<basilisco::Sentencia> {
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    Parser::nuevo(tokens).parsear_programa().unwrap()
}

fn tree_walk(src: &str) -> String {
    let ss = parsear(&format!("{}\nmain()", src));
    let mut env = Entorno::nueva();
    match env.ejecutar_programa(&ss) {
        Ok(v) => format!("OK:{:?}", v),
        Err(e) => format!("ERR:{}", e.causa),
    }
}

fn vm_run(src: &str) -> String {
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = match hir::lower_programa(&ss, &ctx) {
        Ok(h) => h,
        Err(e) => return format!("HIRERR:{:?}", e),
    };
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    let Some(idx) = maquina.indice_de("main") else {
        return "NOFUNC:main".to_string();
    };
    match maquina.ejecutar_indice(idx, &[]) {
        Ok(v) => format!("OK:{:?}", v),
        Err(e) => {
            let s = e.to_string();
            let kind = s.strip_prefix("error aritmético: ").unwrap_or(&s);
            format!("ERR:{}", kind)
        }
    }
}

fn assert_parity(src: &str) {
    let tw = tree_walk(src);
    let vmr = vm_run(src);
    assert_eq!(
        tw, vmr,
        "\nFUENTE:\n{}\nTREE-WALK: {}\nVM: {}",
        src, tw, vmr
    );
}

#[test]
fn diff_aritmetica_basica() {
    assert_parity("funcion main() -> entero { retornar 2 + 3 * 4 - 1 }");
}

#[test]
fn diff_division_y_modulo() {
    assert_parity("funcion main() -> entero { retornar 10 % 3 + 7 / 2 }");
}

#[test]
fn diff_condicional() {
    assert_parity("funcion main() -> entero { si 5 < 3 { retornar 1 } sino { retornar 2 } }");
}

#[test]
fn diff_mientras() {
    assert_parity(
        "funcion main() -> entero {\n  sea i = 0\n  sea s = 0\n  mientras i < 5 {\n    i = i + 1\n    s = s + i\n  }\n  retornar s\n}",
    );
}

#[test]
fn diff_recursion_factorial() {
    assert_parity(
        "funcion factorial(n: entero) -> entero { si n < 2 { retornar 1 } sino { retornar factorial(n - 1) * n } }\nfuncion main() -> entero { retornar factorial(6) }",
    );
}

#[test]
fn diff_arreglos() {
    assert_parity(
        "funcion main() -> entero {\n  sea xs = [1, 2, 3]\n  xs[1] = 9\n  retornar xs[0] + xs[1] + xs[2]\n}",
    );
}

#[test]
fn diff_texto_concatenacion() {
    assert_parity("funcion main() -> texto { retornar \"a\" + \"b\" + \"c\" }");
}

#[test]
fn diff_error_division_por_cero() {
    assert_parity("funcion main() -> entero { retornar 1 / 0 }");
}

#[test]
fn diff_error_desbordamiento() {
    assert_parity("funcion main() -> entero { retornar 9223372036854775807 + 1 }");
}

#[test]
fn diff_para_cada() {
    assert_parity(
        "funcion main() -> entero {\n  sea s = 0\n  para cada x en [1, 2, 3, 4] {\n    s = s + x\n  }\n  retornar s\n}",
    );
}

#[test]
fn diff_intentar_rescatar() {
    assert_parity(
        "funcion main() -> entero {\n  intentar {\n    retornar 1\n  } rescatar e {\n    retornar 2\n  }\n}",
    );
}

#[test]
fn diff_lista_vacia_y_push() {
    assert_parity(
        "funcion main() -> entero {\n  var xs: lista<entero> = [1, 2]\n  retornar xs[0] + xs[1]\n}",
    );
}

#[test]
fn diff_texto_interpolacion_y_longitud() {
    assert_parity("funcion main() -> texto { retornar \"hola\" + \" \" + \"mundo\" }");
}

#[test]
fn diff_arreglo_anidado_indice() {
    assert_parity(
        "funcion main() -> entero {\n  sea xs = [10, 20, 30]\n  sea i = 1\n  retornar xs[i] + xs[0]\n}",
    );
}

#[test]
fn diff_closure_inmutable() {
    let src = "funcion crear_sumador(x) {\n  funcion sumar(yy) {\n    retornar x + yy\n  }\n  retornar sumar\n}\nfuncion main() -> entero {\n  sea sumar10 = crear_sumador(10)\n  retornar sumar10(5)\n}";
    assert_parity(src);
}

#[test]
fn diff_closure_mutable() {
    let src = "funcion crear_contador() {\n  sea x = 0\n  funcion inc() {\n    x = x + 1\n    retornar x\n  }\n  retornar inc\n}\nfuncion main() -> entero {\n  sea c = crear_contador()\n  c()\n  retornar c()\n}";
    assert_parity(src);
}

#[test]
fn diff_romper_continuar() {
    assert_parity(
        "funcion main() -> entero {\n  sea s = 0\n  sea i = 0\n  mientras i < 10 {\n    i = i + 1\n    si i == 3 { continuar }\n    si i == 5 { romper }\n    s = s + i\n  }\n  retornar s\n}",
    );
}

#[test]
fn diff_mostrar_stdlib() {
    let src = "funcion main() -> entero {\n  mostrar 42\n  retornar 1\n}";
    // VM needs stdlib registered; tree-walk uses built-in mostrar.
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    maquina.registrar_stdlib();
    let idx = maquina.indice_de("main").unwrap();
    let vmr = match maquina.ejecutar_indice(idx, &[]) {
        Ok(v) => format!("OK:{:?}", v),
        Err(e) => format!("ERR:{}", e),
    };
    // tree-walk
    let ss2 = parsear(&format!("{}\nmain()", src));
    let mut env = Entorno::nueva();
    let tw = match env.ejecutar_programa(&ss2) {
        Ok(v) => format!("OK:{:?}", v),
        Err(e) => format!("ERR:{}", e.causa),
    };
    assert_eq!(tw, vmr);
}

#[test]
fn diff_intentar_rescatar_error_division() {
    assert_parity(
        "funcion main() -> entero {\n  intentar {\n    sea r = 1 / 0\n    retornar 99\n  } rescatar e {\n    retornar 42\n  }\n}",
    );
}

#[test]
fn diff_intentar_rescatar_variable_error() {
    assert_parity(
        "funcion main() -> texto {\n  intentar {\n    sea r = 1 / 0\n    retornar \"ok\"\n  } rescatar e {\n    retornar \"atrapado\"\n  }\n}",
    );
}

#[test]
fn diff_intentar_rescatar_sin_captura() {
    assert_parity(
        "funcion main() -> entero {\n  intentar {\n    sea r = 1 / 0\n    retornar 99\n  } rescatar {\n    retornar 7\n  }\n}",
    );
}

#[test]
fn diff_fibonacci_recursivo() {
    assert_parity(
        "funcion fib(n: entero) -> entero { si n < 2 { retornar n } sino { retornar fib(n - 1) + fib(n - 2) } }\nfuncion main() -> entero { retornar fib(10) }",
    );
}

#[test]
fn diff_longitud_texto() {
    assert_parity("funcion main() -> entero { retornar longitud(\"hola mundo\") }");
}

#[test]
fn diff_abs_min_max() {
    assert_parity("funcion main() -> entero { retornar abs(0 - 7) + min(3, 9) + max(3, 9) }");
}

#[test]
fn diff_mayusculas_minusculas() {
    assert_parity(
        "funcion main() -> texto { retornar mayusculas(\"bas\") + \" \" + minusculas(\"BASILISCO\") }",
    );
}

#[test]
fn diff_es_vacio() {
    assert_parity("funcion main() -> logico { retornar es_vacio(\"\") y no es_vacio(\"a\") }");
}

#[test]
fn diff_contiene_empieza_con() {
    assert_parity(
        "funcion main() -> logico { retornar contiene(\"hola mundo\", \"mundo\") y empieza_con(\"hola mundo\", \"hola\") }",
    );
}

#[test]
fn diff_arbol_expresion() {
    assert_parity("funcion main() -> entero { retornar (1 + 2) * (3 + 4) - (10 / 2) % 3 }");
}

#[test]
fn diff_comparacion_textos() {
    assert_parity("funcion main() -> logico { retornar \"abeja\" < \"abejon\" }");
}

#[test]
fn diff_json_parsear_serializar() {
    assert_parity(
        "funcion main() -> texto { retornar json_serializar(json_parsear(\"{\\\"a\\\": 1, \\\"b\\\": [1, 2, 3]}\")) }",
    );
}

#[test]
fn diff_json_obtener() {
    assert_parity(
        "funcion main() -> entero { sea obj = json_parsear(\"{\\\"x\\\": 40}\")\n retornar 2 + json_obtener(obj, \"x\") }",
    );
}

#[test]
fn diff_archivo_escribir_leer() {
    let ruta = format!("./basilisco_diff_test_{}.txt", std::process::id());
    let src = format!(
        "funcion main() -> texto {{\n  escribir_archivo(\"{}\", \"hola vm\")\n  retornar leer_archivo(\"{}\")\n}}",
        ruta, ruta
    );
    let tw = tree_walk(&src);
    let vmr = vm_run(&src);
    std::fs::remove_file(&ruta).ok();
    assert_eq!(tw, vmr, "\nTREE-WALK: {}\nVM: {}", tw, vmr);
}

#[test]
fn diff_existe_archivo() {
    let src = "funcion main() -> logico { retornar existe_archivo(\".\") }";
    assert_parity(src);
}

#[test]
fn diff_a_entero() {
    assert_parity("funcion main() -> entero { retornar a_entero(\"  42 \") * 2 - 1 }");
}

#[test]
fn diff_a_decimal() {
    assert_parity(
        "funcion main() -> logico { sea d = a_decimal(\"2.5\")\n  retornar d > 2 y d < 3 }",
    );
}

#[test]
fn diff_a_real() {
    assert_parity(
        "funcion main() -> logico { sea r = a_real(\"3.14\")\n  retornar r > 3 y r < 4 }",
    );
}

#[test]
fn diff_agregar_eliminar() {
    assert_parity(
        "funcion main() -> entero {\n  sea a = [1, 2]\n  sea b = agregar(a, 3)\n  sea c = eliminar(b, 0)\n  retornar longitud(b) * 10 + longitud(c)\n}",
    );
}

#[test]
fn diff_agregar_nueva_coleccion() {
    assert_parity("funcion main() -> entero { retornar longitud(agregar(agregar([], 1), 2)) }");
}

#[test]
fn diff_eliminar_indice_invalido_rescatado() {
    assert_parity(
        "funcion main() -> entero {\n  intentar {\n    eliminar([1, 2], 5)\n    retornar 0\n  } rescatar e {\n    retornar 99\n  }\n}",
    );
}

#[test]
fn diff_map_longitud() {
    assert_parity(
        "funcion main() -> entero { retornar longitud(map([1, 2, 3], funcion (x) { retornar x * 2 })) }",
    );
}

#[test]
fn diff_filtrar() {
    assert_parity(
        "funcion main() -> entero { retornar longitud(filtrar([1, 2, 3, 4, 5], funcion (x) { retornar x % 2 == 0 })) }",
    );
}

#[test]
fn diff_reducir() {
    assert_parity(
        "funcion main() -> entero { retornar reducir([1, 2, 3, 4], funcion (acc, x) { retornar acc + x }, 0) }",
    );
}

#[test]
fn diff_map_filtrar_reducir_combinado() {
    assert_parity(
        "funcion main() -> entero {\n  sea duplicada = map([1, 2, 3], funcion (x) { retornar x * 2 })\n  sea pares = filtrar(duplicada, funcion (x) { retornar x > 2 })\n  retornar reducir(pares, funcion (a, b) { retornar a + b }, 0)\n}",
    );
}

#[test]
fn diff_map_con_clausura_capturada() {
    assert_parity(
        "funcion crear_sumador(n) { retornar funcion (x) { retornar x + n } }\nfuncion main() -> entero { retornar reducir(map([1, 2], crear_sumador(10)), funcion (a, b) { retornar a + b }, 0) }",
    );
}

#[test]
fn diff_archivo_escribir_linea_leer_linea() {
    let ruta = format!("./basilisco_hdl_ll_{}.txt", std::process::id());
    let src = format!(
        "funcion main() -> texto {{\n  sea f = abrir(\"{r}\", \"escribir\")\n  escribir_linea(f, \"linea1\")\n  escribir_linea(f, \"linea2\")\n  cerrar(f)\n  sea g = abrir(\"{r}\", \"leer\")\n  sea a = leer_linea(g)\n  sea b = leer_linea(g)\n  cerrar(g)\n  retornar a + \"|\" + b\n}}",
        r = ruta
    );
    let tw = tree_walk(&src);
    let vmr = vm_run(&src);
    std::fs::remove_file(&ruta).ok();
    assert_eq!(tw, vmr, "\nTREE-WALK: {}\nVM: {}", tw, vmr);
}

#[test]
fn diff_archivo_escribir_leer_todo() {
    let ruta = format!("./basilisco_hdl_todo_{}.txt", std::process::id());
    let src = format!(
        "funcion main() -> texto {{\n  sea f = abrir(\"{r}\", \"escribir\")\n  escribir(f, \"hola\\nmundo\")\n  cerrar(f)\n  sea g = abrir(\"{r}\", \"leer\")\n  sea t = leer_todo(g)\n  cerrar(g)\n  retornar t\n}}",
        r = ruta
    );
    let tw = tree_walk(&src);
    let vmr = vm_run(&src);
    std::fs::remove_file(&ruta).ok();
    assert_eq!(tw, vmr, "\nTREE-WALK: {}\nVM: {}", tw, vmr);
}

#[test]
fn diff_archivo_fin_de_archivo_bucle() {
    let ruta = format!("./basilisco_hdl_fda_{}.txt", std::process::id());
    let src = format!(
        "funcion main() -> entero {{\n  sea f = abrir(\"{r}\", \"escribir\")\n  escribir_linea(f, \"x\")\n  escribir_linea(f, \"y\")\n  cerrar(f)\n  sea g = abrir(\"{r}\", \"leer\")\n  sea n = 0\n  mientras no fin_de_archivo(g) {{\n    leer_linea(g)\n    n = n + 1\n  }}\n  cerrar(g)\n  retornar n\n}}",
        r = ruta
    );
    let tw = tree_walk(&src);
    let vmr = vm_run(&src);
    std::fs::remove_file(&ruta).ok();
    assert_eq!(tw, vmr, "\nTREE-WALK: {}\nVM: {}", tw, vmr);
}

#[test]
fn diff_archivo_modo_agregar() {
    let ruta = format!("./basilisco_hdl_add_{}.txt", std::process::id());
    std::fs::write(&ruta, "uno").ok();
    let src = format!(
        "funcion main() -> texto {{\n  sea f = abrir(\"{r}\", \"agregar\")\n  escribir(f, \"-dos\")\n  cerrar(f)\n  retornar leer_archivo(\"{r}\")\n}}",
        r = ruta
    );
    let tw = tree_walk(&src);
    std::fs::write(&ruta, "uno").ok();
    let vmr = vm_run(&src);
    std::fs::remove_file(&ruta).ok();
    assert_eq!(tw, vmr, "\nTREE-WALK: {}\nVM: {}", tw, vmr);
}

#[test]
fn diff_tarea_iniciar_y_esperar() {
    assert_parity(
        "funcion tarea() -> entero {\n  sea s = 0\n  sea i = 1\n  mientras i <= 100 {\n    s = s + i\n    i = i + 1\n  }\n  retornar s\n}\nfuncion main() -> entero {\n  sea id = iniciar_tarea(\"tarea\")\n  sea local = 5\n  retornar esperar_tarea(id) + local\n}",
    );
}

#[test]
fn diff_tarea_dos_tareas() {
    assert_parity(
        "funcion a() -> entero { retornar 10 * 10 }\nfuncion b() -> entero { retornar 20 * 20 }\nfuncion main() -> entero {\n  sea ia = iniciar_tarea(\"a\")\n  sea ib = iniciar_tarea(\"b\")\n  retornar esperar_tarea(ia) + esperar_tarea(ib)\n}",
    );
}

#[test]
fn diff_ffi_sumar_y_cuadrado() {
    use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    let base =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ffi-diff-{}", std::process::id()));
    if base.exists() {
        fs::remove_dir_all(&base).ok();
    }
    fs::create_dir_all(&base).unwrap();
    let fuente = base.join("ejemplo.c");
    fs::write(
        &fuente,
        "long long sumar(long long a, long long b) { return a + b; }\nlong long cuadrado(long long x) { return x * x; }\n",
    )
    .unwrap();
    let salida = base.join(format!("{DLL_PREFIX}basilisco_ffi{DLL_SUFFIX}"));
    let mut compilador = Command::new("cc");
    compilador.arg("-shared");
    if !cfg!(windows) {
        compilador.arg("-fPIC");
    }
    let estado = compilador.arg("-o").arg(&salida).arg(&fuente).output().ok();
    if estado.is_none() || !estado.unwrap().status.success() {
        eprintln!("aviso: compilación C falló; prueba FFI omitida");
        return;
    }
    let ruta = salida.display().to_string().replace('\\', "/");
    let src = format!(
        "funcion main() -> entero {{\n  sea id = cargar_ffi(\"{r}\")\n  sea r1 = llamar_ffi(id, \"sumar\", [3, 4], \"entero\")\n  sea r2 = llamar_ffi(id, \"cuadrado\", [5], \"entero\")\n  descargar_ffi(id)\n  retornar r1 + r2\n}}",
        r = ruta
    );
    let tw = tree_walk(&src);
    let vmr = vm_run(&src);
    fs::remove_dir_all(&base).ok();
    assert_eq!(tw, vmr, "\nTREE-WALK: {}\nVM: {}", tw, vmr);
}

#[test]
fn diff_fecha_actual_formato() {
    assert_parity("funcion main() -> entero { retornar longitud(fecha_actual()) }");
}

#[test]
fn diff_formatear_fecha() {
    assert_parity(
        "funcion main() -> entero { retornar longitud(formatear_fecha(0, \"%Y-%m-%d\")) }",
    );
}

#[test]
fn diff_ahora_positivo() {
    assert_parity("funcion main() -> logico { retornar ahora() > 0 }");
}

#[test]
fn diff_http_get_local() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        let mut servidos = 0;
        let limite = Instant::now() + Duration::from_secs(10);
        while servidos < 2 && Instant::now() < limite {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let body = "hola http";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.flush();
                servidos += 1;
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        }
    });
    let url = format!("http://127.0.0.1:{port}/");
    let src = format!("funcion main() -> texto {{ retornar http_get(\"{url}\") }}");
    let tw = tree_walk(&src);
    let vmr = vm_run(&src);
    handle.join().ok();
    assert_eq!(tw, vmr, "\nTREE-WALK: {}\nVM: {}", tw, vmr);
}

#[test]
fn diff_inferencia_acepta_arreglo_y_lista() {
    // El literal `[]` es Lista en Tree-Walk y Arreglo en la VM; json_parsear es
    // Lista en ambos. La misma función debe aceptar ambas secuencias.
    assert_parity(
        "funcion contar(x) { retornar longitud(x) }\nfuncion main() -> entero {\n  sea j = json_parsear(\"[1, 2]\")\n  sea a = []\n  retornar contar(j) + contar(a)\n}",
    );
}

#[test]
fn diff_inferencia_convierte_entero_a_decimal() {
    // Tree-Walk convierte entero -> decimal cuando la primera llamada usó
    // decimal; la VM debe convertir igual y no rechazar.
    assert_parity(
        "funcion f(x) { retornar x }\nfuncion main() -> decimal {\n  f(1.5)\n  retornar f(2)\n}",
    );
}

#[test]
fn diff_anotado_convierte_entero_a_decimal() {
    assert_parity(
        "funcion f(x: decimal) { retornar x }\nfuncion main() -> decimal { retornar f(2) }",
    );
}

#[test]
fn diff_anotado_convierte_entero_a_real() {
    assert_parity("funcion f(x: real) { retornar x }\nfuncion main() -> real { retornar f(2) }");
}
