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
    assert_parity("funcion fib(n: entero) -> entero { si n < 2 { retornar n } sino { retornar fib(n - 1) + fib(n - 2) } }\nfuncion main() -> entero { retornar fib(10) }");
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
    assert_parity("funcion main() -> texto { retornar mayusculas(\"bas\") + \" \" + minusculas(\"BASILISCO\") }");
}

#[test]
fn diff_es_vacio() {
    assert_parity("funcion main() -> logico { retornar es_vacio(\"\") y no es_vacio(\"a\") }");
}

#[test]
fn diff_contiene_empieza_con() {
    assert_parity("funcion main() -> logico { retornar contiene(\"hola mundo\", \"mundo\") y empieza_con(\"hola mundo\", \"hola\") }");
}

#[test]
fn diff_arbol_expresion() {
    assert_parity("funcion main() -> entero { retornar (1 + 2) * (3 + 4) - (10 / 2) % 3 }");
}

#[test]
fn diff_comparacion_textos() {
    assert_parity("funcion main() -> logico { retornar \"abeja\" < \"abejon\" }");
}
