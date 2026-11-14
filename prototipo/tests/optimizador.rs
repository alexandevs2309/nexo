use basilisco::{ContextoSemantico, Entorno, Lexer, Parser, hir, mir, optimizador, vm};

fn parsear(src: &str) -> Vec<basilisco::Sentencia> {
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    Parser::nuevo(tokens).parsear_programa().unwrap()
}

fn run_vm(src: &str, optimizar: bool) -> String {
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mut mir = mir::lower_programa(&hir);
    if optimizar {
        optimizador::optimizar(&mut mir);
    }
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    let Some(idx) = maquina.indice_de("main") else {
        return "NOFUNC".to_string();
    };
    match maquina.ejecutar_indice(idx, &[]) {
        Ok(v) => format!("OK:{:?}", v),
        Err(e) => format!("ERR:{}", e),
    }
}

fn run_tw(src: &str) -> String {
    let ss = parsear(&format!("{}\nmain()", src));
    let mut env = Entorno::nueva();
    match env.ejecutar_programa(&ss) {
        Ok(v) => format!("OK:{:?}", v),
        Err(e) => format!("ERR:{}", e.causa),
    }
}

#[test]
fn opt_const_folding_aritmetica() {
    let src = "funcion main() -> entero { retornar 2 + 3 * 4 }";
    // El resultado debe ser idéntico con y sin optimización.
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_vm(src, true), "OK:Some(Entero(14))");
}

#[test]
fn opt_const_folding_logicos() {
    let src = "funcion main() -> logico { retornar verdadero y (falso o verdadero) }";
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_vm(src, true), "OK:Some(Logico(true))");
}

#[test]
fn opt_branch_simplification() {
    let src = "funcion main() -> entero { si verdadero { retornar 1 } sino { retornar 2 } }";
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_vm(src, true), "OK:Some(Entero(1))");
}

#[test]
fn opt_loop_preserva_semantica() {
    let src = "funcion main() -> entero {\n  sea i = 0\n  sea s = 0\n  mientras i < 5 {\n    i = i + 1\n    s = s + i\n  }\n  retornar s\n}";
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_tw(src), run_vm(src, true));
}

#[test]
fn opt_recursion_factorial() {
    let src = "funcion factorial(n: entero) -> entero { si n < 2 { retornar 1 } sino { retornar factorial(n - 1) * n } }\nfuncion main() -> entero { retornar factorial(7) }";
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_tw(src), run_vm(src, true));
}

#[test]
fn opt_arreglos() {
    let src = "funcion main() -> entero {\n  sea xs = [1, 2, 3]\n  xs[1] = 9\n  retornar xs[0] + xs[1] + xs[2]\n}";
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_tw(src), run_vm(src, true));
}

#[test]
fn opt_idempotente() {
    let src = "funcion main() -> entero { retornar (1 + 2) * (3 + 4) - 5 }";
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mut mir = mir::lower_programa(&hir);
    optimizador::optimizar(&mut mir);
    let una = format!("{:?}", mir);
    optimizador::optimizar(&mut mir);
    let dos = format!("{:?}", mir);
    assert_eq!(una, dos);
}

#[test]
fn opt_const_propagacion_local() {
    // `x` se asigna una constante y se usa en la misma expresión.
    let src = "funcion main() -> entero { sea x = 5\n retornar x + x * 2 }";
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_vm(src, true), "OK:Some(Entero(15))");
}

#[test]
fn opt_closures_preservan_semantica() {
    let src = "funcion obtener() { retornar funcion (x) { retornar x + 1 } }\nfuncion main() -> entero { retornar obtener()(9) }";
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_tw(src), run_vm(src, true));
}

#[test]
fn opt_condicionales_anidados() {
    let src = "funcion main() -> entero {\n  sea x = 0\n  si x == 0 {\n    si verdadero { retornar 1 } sino { retornar 2 }\n  } sino {\n    retornar 3\n  }\n}";
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_tw(src), run_vm(src, true));
}

#[test]
fn opt_mientras_con_romper() {
    let src = "funcion main() -> entero {\n  sea i = 0\n  sea s = 0\n  mientras i < 10 {\n    i = i + 1\n    si i == 3 { continuar }\n    si i == 7 { romper }\n    s = s + i\n  }\n  retornar s\n}";
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_tw(src), run_vm(src, true));
}

#[test]
fn opt_intentar_rescatar() {
    let src = "funcion main() -> entero {\n  intentar {\n    sea r = 10 / 2\n    retornar r\n  } rescatar e {\n    retornar 0\n  }\n}";
    assert_eq!(run_vm(src, false), run_vm(src, true));
    assert_eq!(run_tw(src), run_vm(src, true));
}
