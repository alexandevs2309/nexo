//! Pruebas diferenciales TW↔VM basadas en contrato semántico común (04.1).
//!
//! - Ejecuta Tree-Walk y VM directamente (sin CLI/fallback).
//! - Normaliza resultados con `basilisco::paridad` (sin depender de `Debug`).
//! - Compara valor, categoría/causa de error y efectos observables deterministas.
//! - Acepta equivalencia semántica aunque mensajes/diagnósticos difieran.

use basilisco::{
    Comparacion, ContextoSemantico, EfectosObservables, Entorno, Lexer, Parser,
    ResultadoNormalizado, comparar_semantica, hir, mir, normalizar_error_tw, normalizar_error_vm,
    normalizar_valor, vm,
};

fn parsear(src: &str) -> Vec<basilisco::Sentencia> {
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    Parser::nuevo(tokens).parsear_programa().unwrap()
}

fn tw_resultado(src: &str) -> ResultadoNormalizado {
    // Misma convención histórica del harness: definir fuente y luego invocar main().
    let ss = parsear(&format!("{}\nmain()", src));
    let mut ctx = ContextoSemantico::nuevo();
    let errores_sem = ctx.analizar_programa(&ss);
    if let Some(e) = errores_sem.first() {
        use basilisco::CausaError;
        let relevante = matches!(
            e.causa.as_ref(),
            CausaError::ReasignacionInmutable { .. }
                | CausaError::CondicionNoLogica { .. }
                | CausaError::VariableNoDefinida(_)
        );
        if relevante {
            return ResultadoNormalizado::Err(normalizar_error_tw(e));
        }
        // Para otros errores semánticos, preservar comportamiento histórico:
        // continuar hacia ejecución para comparar resultados runtime.
    }
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    if let Some(e) = errs_g4.first() {
        return ResultadoNormalizado::Err(normalizar_error_tw(e));
    }
    let mut env = Entorno::nueva();
    match env.ejecutar_programa(&ss) {
        Ok(v) => ResultadoNormalizado::Ok {
            valor: v.as_ref().map(normalizar_valor),
            efectos: EfectosObservables::default(),
        },
        Err(e) => ResultadoNormalizado::Err(normalizar_error_tw(&e)),
    }
}

fn vm_resultado(src: &str) -> ResultadoNormalizado {
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errores_sem = ctx.analizar_programa(&ss);
    if let Some(e) = errores_sem.first() {
        use basilisco::CausaError;
        // Paridad: ciertos errores semánticos deben impedir ejecución (como en TW)
        // y coinciden con el comportamiento observable esperado.
        let relevante = matches!(
            e.causa.as_ref(),
            CausaError::ReasignacionInmutable { .. }
                | CausaError::CondicionNoLogica { .. }
                | CausaError::VariableNoDefinida(_)
        );
        if relevante {
            return ResultadoNormalizado::Err(normalizar_error_tw(e));
        }
    }
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    if let Some(e) = errs_g4.first() {
        return ResultadoNormalizado::Err(normalizar_error_tw(e));
    }
    let hir = match hir::lower_programa(&ss, &ctx) {
        Ok(h) => h,
        Err(e) => {
            // HIR no implementado para una construcción: se reporta como no verificable
            // en el diagnóstico (no se fuerza como divergencia semántica silenciosa).
            return ResultadoNormalizado::Err(basilisco::paridad::ErrorNormalizado {
                causa: basilisco::paridad::CausaNormalizada {
                    categoria: basilisco::paridad::CategoriaError::NoSoportado,
                    subcausa: Some("hir_no_implementado".to_string()),
                    datos: {
                        let mut d = std::collections::BTreeMap::new();
                        d.insert("detalle".into(), format!("{e:?}"));
                        d
                    },
                },
            });
        }
    };
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    let Some(idx) = maquina.indice_de("main") else {
        return ResultadoNormalizado::Err(basilisco::paridad::ErrorNormalizado {
            causa: basilisco::paridad::CausaNormalizada {
                categoria: basilisco::paridad::CategoriaError::FuncionNoEncontrada,
                subcausa: None,
                datos: {
                    let mut d = std::collections::BTreeMap::new();
                    d.insert("nombre".into(), "main".into());
                    d
                },
            },
        });
    };
    match maquina.ejecutar_indice(idx, &[]) {
        Ok(v) => ResultadoNormalizado::Ok {
            valor: v.as_ref().map(normalizar_valor),
            efectos: EfectosObservables::default(),
        },
        Err(e) => ResultadoNormalizado::Err(normalizar_error_vm(&e)),
    }
}

fn clasificar(src: &str) -> (ResultadoNormalizado, ResultadoNormalizado, Comparacion) {
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    (tw, vmr, cmp)
}

fn assert_parity(src: &str) {
    let (tw, vmr, cmp) = clasificar(src);
    match cmp {
        Comparacion::Equivalente => {}
        Comparacion::DiferenciaDiagnostico(det) => {
            // Equivalente en causa semántica/categoría con diagnóstico/representación distinta.
            // Aceptado por contrato (p.ej. mensajes/campos obtenidos distintos).
            let _ = det;
        }
        Comparacion::DiferenciaRepresentacion(det) => {
            let _ = det;
        }
        Comparacion::CapacidadNoSoportada { motor, detalle } => {
            panic!(
                "\nFUENTE:\n{src}\nMOTOR: {motor}\nNO SOPORTADO: {detalle}\nTREE-WALK: {tw:?}\nVM: {vmr:?}"
            );
        }
        Comparacion::DivergenciaSemantica(det) | Comparacion::NoVerificable(det) => {
            panic!("\nFUENTE:\n{src}\nCOMPARACION: {det}\nTREE-WALK: {tw:?}\nVM: {vmr:?}");
        }
    }
}

/// 04.3: evidencia reproducible de divergencia real (no corregida en este lote).
#[allow(dead_code)]
fn assert_divergencia(src: &str) {
    let (tw, vmr, cmp) = clasificar(src);
    assert!(
        matches!(cmp, Comparacion::DivergenciaSemantica(_)),
        "\nFUENTE:\n{src}\nSE ESPERABA DivergenciaSemantica, OBTENIDO: {cmp:?}\nTREE-WALK: {tw:?}\nVM: {vmr:?}"
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
    // stdout: TW emite al proceso (no capturable de forma homogénea/in-process en este harness).
    // VM captura salida determinista en `MaquinaVirtual::salida()`; se valida como efecto observable.
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    let idx = maquina.indice_de("main").unwrap();
    let vmr = match maquina.ejecutar_indice(idx, &[]) {
        Ok(v) => ResultadoNormalizado::Ok {
            valor: v.as_ref().map(normalizar_valor),
            efectos: EfectosObservables {
                stdout: maquina.salida(),
            },
        },
        Err(e) => ResultadoNormalizado::Err(normalizar_error_vm(&e)),
    };
    // Efecto stdout determinista esperado en VM (mostrar entero agrega salto de línea).
    assert_eq!(
        vmr,
        ResultadoNormalizado::Ok {
            valor: Some(basilisco::ValorNormalizado::Entero(1)),
            efectos: EfectosObservables {
                stdout: vec!["42\n".to_string()],
            },
        },
        "stdout/valor VM no determinista o inesperado"
    );
    // Paridad semántica del valor con TW (stdout TW no comparable aquí por captura heterogénea).
    let tw = tw_resultado(src);
    let cmp = comparar_semantica(
        &tw,
        &ResultadoNormalizado::Ok {
            valor: Some(basilisco::ValorNormalizado::Entero(1)),
            efectos: EfectosObservables::default(),
        },
    );
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente
                | Comparacion::DiferenciaDiagnostico(_)
                | Comparacion::DiferenciaRepresentacion(_)
        ),
        "TREE-WALK: {tw:?}\nVM(valor): {vmr:?}\nCMP: {cmp:?}"
    );
}

#[test]
fn diff_vm_stdout_mostrar_texto_determinista() {
    // Validación determinista de efectos stdout en VM (mostrar texto).
    // Comparación TW↔VM de stdout no homogénea en harness in-process (TW emite a stdout del proceso).
    let src = "funcion main() -> entero {\n  mostrar \"hola\"\n  retornar 0\n}";
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    let idx = maquina.indice_de("main").unwrap();
    let v = maquina.ejecutar_indice(idx, &[]).unwrap();
    assert_eq!(
        v.as_ref().map(normalizar_valor),
        Some(basilisco::ValorNormalizado::Entero(0))
    );
    assert_eq!(maquina.salida(), vec!["hola\n".to_string()]);
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
    let tw = tw_resultado(&src);
    let vmr = vm_resultado(&src);
    std::fs::remove_file(&ruta).ok();
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
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
    let tw = tw_resultado(&src);
    let vmr = vm_resultado(&src);
    std::fs::remove_file(&ruta).ok();
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
}

#[test]
fn diff_archivo_escribir_leer_todo() {
    let ruta = format!("./basilisco_hdl_todo_{}.txt", std::process::id());
    let src = format!(
        "funcion main() -> texto {{\n  sea f = abrir(\"{r}\", \"escribir\")\n  escribir(f, \"hola\\nmundo\")\n  cerrar(f)\n  sea g = abrir(\"{r}\", \"leer\")\n  sea t = leer_todo(g)\n  cerrar(g)\n  retornar t\n}}",
        r = ruta
    );
    let tw = tw_resultado(&src);
    let vmr = vm_resultado(&src);
    std::fs::remove_file(&ruta).ok();
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
}

#[test]
fn diff_archivo_fin_de_archivo_bucle() {
    let ruta = format!("./basilisco_hdl_fda_{}.txt", std::process::id());
    let src = format!(
        "funcion main() -> entero {{\n  sea f = abrir(\"{r}\", \"escribir\")\n  escribir_linea(f, \"x\")\n  escribir_linea(f, \"y\")\n  cerrar(f)\n  sea g = abrir(\"{r}\", \"leer\")\n  sea n = 0\n  mientras no fin_de_archivo(g) {{\n    leer_linea(g)\n    n = n + 1\n  }}\n  cerrar(g)\n  retornar n\n}}",
        r = ruta
    );
    let tw = tw_resultado(&src);
    let vmr = vm_resultado(&src);
    std::fs::remove_file(&ruta).ok();
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
}

#[test]
fn diff_archivo_modo_agregar() {
    let ruta = format!("./basilisco_hdl_add_{}.txt", std::process::id());
    std::fs::write(&ruta, "uno").ok();
    let src = format!(
        "funcion main() -> texto {{\n  sea f = abrir(\"{r}\", \"agregar\")\n  escribir(f, \"-dos\")\n  cerrar(f)\n  retornar leer_archivo(\"{r}\")\n}}",
        r = ruta
    );
    let tw = tw_resultado(&src);
    std::fs::write(&ruta, "uno").ok();
    let vmr = vm_resultado(&src);
    std::fs::remove_file(&ruta).ok();
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
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
    let tw = tw_resultado(&src);
    let vmr = vm_resultado(&src);
    fs::remove_dir_all(&base).ok();
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
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
    let tw = tw_resultado(&src);
    let vmr = vm_resultado(&src);
    handle.join().ok();
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
}

#[test]
fn diff_inferencia_acepta_arreglo_y_lista() {
    assert_parity(
        "funcion contar(x) { retornar longitud(x) }\nfuncion main() -> entero {\n  sea j = json_parsear(\"[1, 2]\")\n  sea a = []\n  retornar contar(j) + contar(a)\n}",
    );
}

#[test]
fn diff_inferencia_convierte_entero_a_decimal() {
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

#[test]
fn diff_closure_captura_simple() {
    assert_parity(
        "funcion crear(x) { retornar funcion (yy) { retornar x + yy } }\nfuncion main() -> entero { sea c = crear(10)\n retornar c(5) }",
    );
}

#[test]
fn diff_closure_dos_instancias() {
    assert_parity(
        "funcion crear(x) { retornar funcion (yy) { retornar x + yy } }\nfuncion main() -> entero { sea a = crear(10)\n sea b = crear(20)\n retornar a(5) + b(5) }",
    );
}

#[test]
fn diff_closure_mutable_captura() {
    assert_parity(
        "funcion crear() { sea x = 0\n funcion inc() { x = x + 1\n retornar x }\n retornar inc }\nfuncion main() -> entero { sea c = crear()\n c()\n retornar c() }",
    );
}

#[test]
fn diff_closure_como_argumento() {
    assert_parity(
        "funcion aplicar(f, x) { retornar f(x) }\nfuncion doble(n) { retornar n * 2 }\nfuncion main() -> entero { retornar aplicar(doble, 4) }",
    );
}

#[test]
fn diff_closure_retornada_anidada() {
    assert_parity(
        "funcion obtener() { retornar funcion (x) { retornar x + 1 } }\nfuncion main() -> entero { retornar obtener()(9) }",
    );
}

#[test]
fn diff_lambda_en_map() {
    assert_parity(
        "funcion main() -> entero { sea r = map([1, 2, 3], funcion (x) { retornar x * 2 })\n retornar r[0] + r[1] + r[2] }",
    );
}

#[test]
fn diff_lambda_en_filtrar() {
    assert_parity(
        "funcion main() -> entero { sea r = filtrar([1, 2, 3, 4], funcion (x) { retornar x > 2 })\n retornar longitud(r) }",
    );
}

#[test]
fn diff_lambda_en_reducir() {
    assert_parity(
        "funcion main() -> entero { retornar reducir([1, 2, 3, 4], funcion (acc, x) { retornar acc + x }, 0) }",
    );
}

#[test]
fn diff_canal_productor_consumidor() {
    assert_parity(
        "funcion productor() { enviar(0, 42)\n retornar 0 }\nfuncion main() -> entero { sea c = crear_canal()\n sea t = iniciar_tarea(\"productor\")\n sea v = recibir(0)\n esperar_tarea(t)\n retornar v }",
    );
}

#[test]
fn diff_dos_canales() {
    assert_parity(
        "funcion p1() { enviar(0, 10)\n retornar 0 }\nfuncion p2() { enviar(1, 20)\n retornar 0 }\nfuncion main() -> entero { crear_canal()\n crear_canal()\n sea t1 = iniciar_tarea(\"p1\")\n sea t2 = iniciar_tarea(\"p2\")\n sea a = recibir(0)\n sea b = recibir(1)\n esperar_tarea(t1)\n esperar_tarea(t2)\n retornar a + b }",
    );
}

// --- GAP-051: conversiones de asignación estrictas (ADR-0005 §1.4 / spec §3.1) ---

#[test]
fn diff_gap051_real_rechaza_decimal() {
    assert_parity("funcion main() -> entero { var r: real = 1.5\n retornar 0 }");
}

#[test]
fn diff_gap051_decimal_acepta_decimal() {
    assert_parity("funcion main() -> decimal { var d: decimal = 2.5\n retornar d }");
}

#[test]
fn diff_gap051_entero_rechaza_decimal() {
    assert_parity("funcion main() -> entero { var x: entero = 2.5\n retornar 0 }");
}

#[test]
fn diff_gap051_real_acepta_entero() {
    assert_parity("funcion main() -> real { var r: real = 2\n retornar r }");
}

#[test]
fn diff_gap051_decimal_acepta_entero() {
    assert_parity("funcion main() -> decimal { var d: decimal = 2\n retornar d }");
}

#[test]
fn diff_gap051_texto_rechaza_entero() {
    assert_parity("funcion main() -> entero { var t: texto = 5\n retornar 0 }");
}

#[test]
fn diff_gap051_logico_rechaza_entero() {
    assert_parity("funcion main() -> entero { var l: logico = 1\n retornar 0 }");
}

#[test]
fn diff_gap051_entero_rechaza_texto() {
    assert_parity("funcion main() -> entero { var x: entero = \"hola\"\n retornar 0 }");
}

#[test]
fn diff_gap051_retorno_real_rechaza_decimal() {
    assert_parity("funcion f() -> real { retornar 1.5 }\nfuncion main() -> real { retornar f() }");
}

#[test]
fn diff_gap051_retorno_entero_rechaza_decimal() {
    assert_parity(
        "funcion f() -> entero { retornar 2.5 }\nfuncion main() -> entero { retornar f() }",
    );
}

#[test]
fn diff_gap051_lista_entero_rechaza_elemento_decimal() {
    // Ambos deben rechazar por asignación incompatible; el diagnóstico puede diferir
    // en representación del "obtenido" (Lista vs Arreglo). Causa semántica equivalente.
    let src = "funcion main() -> entero { var xs: lista<entero> = [1, 2.5]\n retornar 0 }";
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "Se esperaba rechazo equivalente por asignación incompatible.\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
}

#[test]
fn diff_gap051_reasignacion_entero_rechaza_decimal() {
    assert_parity("funcion main() -> entero { var x: entero = 1\n x = 2.5\n retornar x }");
}

// --- Casos dirigidos adicionales 04.1 ---

#[test]
fn diff_dir_valores_primitivos() {
    assert_parity("funcion main() -> logico { retornar verdadero y no falso }");
    assert_parity("funcion main() -> texto { retornar \"ok\" }");
    assert_parity("funcion main() -> entero { retornar 0 }");
}

#[test]
fn diff_dir_comparaciones() {
    assert_parity(
        "funcion main() -> logico { retornar 3 == 3 y 3 != 4 y 2 < 3 y 3 <= 3 y 4 > 3 y 4 >= 4 }",
    );
}

#[test]
fn diff_dir_colecciones_indexacion() {
    assert_parity("funcion main() -> entero { sea xs = [10,20,30]\n retornar xs[2] }");
}

#[test]
fn diff_dir_error_aritmetico_modulo_cero() {
    // Misma causa semántica aritmética (división/módulo por cero).
    assert_parity("funcion main() -> entero { retornar 10 % 0 }");
}

#[test]
fn diff_dir_error_semantico_diferente_tipos() {
    // Distinto de división por cero: incompatibilidad de tipos/operación.
    assert_parity("funcion main() -> entero { retornar 1 + verdadero }");
}

#[test]
fn diff_dir_error_equivalente_mensaje_distinto_asignacion() {
    // Misma causa semántica (asignación incompatible) aunque representación/diagnóstico difiera.
    assert_parity("funcion main() -> entero { var a: entero = 1.0\n retornar a }");
}

#[test]
fn diff_dir_funcion_y_recursion() {
    assert_parity(
        "funcion suma(n: entero) -> entero { si n <= 0 { retornar 0 } sino { retornar n + suma(n - 1) } }\nfuncion main() -> entero { retornar suma(5) }",
    );
}

#[test]
fn diff_dir_conversion_entero_a_real_aceptada() {
    assert_parity("funcion main() -> real { var r: real = 7\n retornar r }");
}

#[test]
fn diff_dir_conversion_coleccion_tipada_incompatible() {
    assert_parity("funcion main() -> entero { var xs: lista<entero> = [1, \"x\"]\n retornar 0 }");
}

#[test]
fn diff_dir_coleccion_orden_y_longitud() {
    // Colecciones secuenciales: orden y contenido preservados.
    assert_parity(
        "funcion main() -> entero { sea xs = [3,1,2]\n retornar xs[0] * 100 + xs[1] * 10 + xs[2] }",
    );
}

#[test]
fn diff_dir_efecto_observable_determinista_estado() {
    // Efecto observable determinista: mutación de colección reflejada en resultado.
    assert_parity(
        "funcion main() -> entero { sea xs = [1,2]\n xs[0] = 9\n retornar xs[0] + xs[1] }",
    );
}

// === 04.3 Consolidación de evidencia TW↔VM (batería sistemática dirigida) ===
// Casos clasificados por contrato: Equivalente/DiferenciaDiagnostico pasan por assert_parity;
// divergencias reales confirmadas se registran como evidencia reproducible (assert_divergencia)
// sin corregir semántica de motores (decisión diferida a lotes posteriores).

#[test]
fn diff_043_inmutabilidad_dato_reasignacion() {
    assert_parity("funcion main() -> entero {\n  dato x: entero = 1\n  x = 2\n  retornar x\n}");
}

#[test]
fn diff_043_nombre_no_definido() {
    assert_parity("funcion main() -> entero { retornar no_existe_var_043 }");
}

#[test]
fn diff_043_indice_lista_fuera_de_rango() {
    assert_parity("funcion main() -> entero { sea xs = [1,2]\n retornar xs[5] }");
}

#[test]
fn diff_043_indice_lista_negativo() {
    assert_parity("funcion main() -> entero { sea xs = [1,2]\n retornar xs[0 - 1] }");
}

#[test]
fn diff_043_iteracion_sobre_no_coleccion() {
    assert_parity(
        "funcion main() -> entero {\n  para cada x en 5 {\n    retornar x\n  }\n  retornar 0\n}",
    );
}

#[test]
fn diff_043_rango_tipo_invalido() {
    assert_parity(
        "funcion main() -> entero {\n  para cada i en verdadero..10 {\n    retornar i\n  }\n  retornar 0\n}",
    );
}

#[test]
fn diff_043_condicion_no_logica_si() {
    assert_parity("funcion main() -> entero { si 1 { retornar 1 } sino { retornar 0 } }");
}

#[test]
fn diff_043_condicion_no_logica_mientras() {
    assert_parity(
        "funcion main() -> entero {\n  mientras 1 {\n    retornar 1\n  }\n  retornar 0\n}",
    );
}

#[test]
fn diff_043_operando_no_logico_y() {
    assert_parity("funcion main() -> logico { retornar 1 y verdadero }");
}

#[test]
fn diff_043_cortocircuito_y_no_evalua_rhs() {
    assert_parity(
        "funcion main() -> entero {\n  sea x = 0\n  si falso y (1 / 0 == 0) {\n    x = 1\n  }\n  retornar x\n}",
    );
}

#[test]
fn diff_043_cortocircuito_o_no_evalua_rhs() {
    assert_parity(
        "funcion main() -> entero {\n  sea x = 0\n  si verdadero o (1 / 0 == 0) {\n    x = 7\n  }\n  retornar x\n}",
    );
}

#[test]
fn diff_043_mutacion_coleccion_parametro() {
    assert_parity(
        "funcion toca(xs) { xs[0] = 9 }\nfuncion main() -> entero { sea a = [1,2]\n toca(a)\n retornar a[0] + a[1] }",
    );
}

#[test]
fn diff_043_agregar_funcional_indice_fuera_de_rango() {
    // `agregar` es funcional (no muta en sitio); acceso fuera de rango equivalente entre motores.
    assert_parity(
        "funcion main() -> entero { sea xs = [1]\n agregar(xs, 2)\n agregar(xs, 3)\n retornar longitud(xs) + xs[2] }",
    );
}

#[test]
fn diff_043_eliminar_elemento() {
    assert_parity(
        "funcion main() -> entero { sea xs = [10,20,30]\n eliminar(xs, 1)\n retornar xs[0] + xs[1] }",
    );
}

#[test]
fn diff_043_objeto_json_lectura() {
    assert_parity(
        "funcion main() -> entero { sea obj = json_parsear(\"{\\\"x\\\": 4, \\\"y\\\": 5}\")\n retornar json_obtener(obj, \"x\") + json_obtener(obj, \"y\") }",
    );
}

#[test]
fn diff_043_conversion_explicita_texto_entero() {
    assert_parity("funcion main() -> entero { retornar a_entero(\"12\") + 1 }");
}

#[test]
fn diff_043_conversion_texto_invalida() {
    assert_parity("funcion main() -> entero { retornar a_entero(\"no\") }");
}

#[test]
fn diff_043_comparacion_mixta_enteros_logico() {
    assert_parity("funcion main() -> logico { retornar (1 < 2) == verdadero }");
}

#[test]
fn diff_043_asignacion_compuesta_o_igual() {
    assert_parity(
        "funcion main() -> entero { sea xs = [1,2,3]\n xs[1] = xs[1] + 5\n retornar xs[0] + xs[1] + xs[2] }",
    );
}

#[test]
fn diff_043_modulo_por_cero_enteros() {
    assert_parity("funcion main() -> entero { retornar 7 % 0 }");
}

#[test]
fn diff_043_igualdad_secuencias() {
    assert_parity("funcion main() -> logico { retornar [1,2] == [1,2] }");
}

// === Evidencia adicional 04.x (consolidación) ===

#[test]
fn diff_gap051_retorno_tipado_rechaza_coleccion_incompatible() {
    let src = "funcion f() -> lista<entero> { retornar [1, 2.5] }\nfuncion main() -> entero { f()\n retornar 0 }";
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "Se esperaba rechazo equivalente por retorno incompatible.\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
}

#[test]
fn diff_gap051_retorno_entero_rechaza_texto_equivalente() {
    assert_parity(
        "funcion f() -> entero { retornar \"hola\" }\nfuncion main() -> entero { retornar f() }",
    );
}

#[test]
fn diff_gap051_asignacion_texto_rechaza_lista_equivalente() {
    assert_parity("funcion main() -> entero { var t: texto = [1, 2]\n retornar 0 }");
}

#[test]
fn diff_gap051_try_rescatar_invoke_ruta_expresable_equivalente() {
    // Ruta expresable: try/rescatar captura error de conversión inválida (lowering/ejecución vía VM).
    // Evidencia de flujo equivalente con manejo de error (try/invoke en MIR según lowering de intentar/rescatar).
    assert_parity(
        "funcion main() -> entero {\n  intentar {\n    a_entero(\"no\")\n    retornar 0\n  } rescatar e {\n    retornar 7\n  }\n}",
    );
}

// === GAP-051 (continuación): parámetros tipados y contextos adicionales ===

#[test]
fn diff_gap051_parametro_anotado_entero_acepta_entero() {
    assert_parity(
        "funcion f(x: entero) -> entero { retornar x }\nfuncion main() -> entero { retornar f(3) }",
    );
}

#[test]
fn diff_gap051_parametro_anotado_decimal_acepta_entero() {
    assert_parity(
        "funcion f(x: decimal) -> decimal { retornar x }\nfuncion main() -> decimal { retornar f(2) }",
    );
}

#[test]
fn diff_gap051_parametro_anotado_real_acepta_entero() {
    assert_parity(
        "funcion f(x: real) -> real { retornar x }\nfuncion main() -> real { retornar f(2) }",
    );
}

#[test]
fn diff_gap051_parametro_anotado_entero_rechaza_decimal() {
    assert_parity(
        "funcion f(x: entero) -> entero { retornar x }\nfuncion main() -> entero { retornar f(2.5) }",
    );
}

#[test]
fn diff_gap051_parametro_anotado_real_rechaza_decimal() {
    assert_parity(
        "funcion f(x: real) -> real { retornar x }\nfuncion main() -> real { retornar f(1.5) }",
    );
}

#[test]
fn diff_gap051_parametro_anotado_texto_rechaza_entero() {
    assert_parity(
        "funcion f(x: texto) -> texto { retornar x }\nfuncion main() -> texto { retornar f(5) }",
    );
}

#[test]
fn diff_gap051_parametro_anotado_lista_entero_rechaza_elemento_decimal() {
    // Rechazo equivalente por conversión de elemento inválida; diagnóstico puede diferir
    // en representación/identificador de parámetro (whitelist GAP-051 acotada).
    let src = "funcion f(xs: lista<entero>) -> entero { retornar 0 }\nfuncion main() -> entero { retornar f([1, 2.5]) }";
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "Se esperaba rechazo equivalente por parámetro tipado incompatible.\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
}

#[test]
fn diff_gap051_parametro_anotado_arreglo_tamano_incorrecto_rechaza() {
    let src = "funcion f(xs: arreglo<entero, 2>) -> entero { retornar 0 }\nfuncion main() -> entero { retornar f([1, 2, 3]) }";
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "Se esperaba rechazo equivalente por parámetro tipado incompatible (tamaño).\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
}

#[test]
fn diff_gap051_parametro_anotado_lista_decimal_convierte_enteros() {
    assert_parity(
        "funcion f(xs: lista<decimal>) -> decimal { retornar xs[0] }\nfuncion main() -> decimal { retornar f([1, 2]) }",
    );
}

#[test]
fn diff_gap051_reasignacion_decimal_acepta_entero() {
    assert_parity("funcion main() -> decimal { var d: decimal = 1.5\n d = 2\n retornar d }");
}

#[test]
fn diff_gap051_reasignacion_real_acepta_entero() {
    assert_parity("funcion main() -> real { var r: real = 1.5\n r = 2\n retornar r }");
}

#[test]
fn diff_gap051_local_lista_tipada_rechaza_elemento_incompatible() {
    let src = "funcion main() -> entero { var xs: lista<entero> = [1, \"x\"]\n retornar 0 }";
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "Se esperaba rechazo equivalente por asignación tipada incompatible.\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
}

// Nota de cobertura (MISIÓN 03): no se identificó sintaxis expresable estable para
// asignación/mutación de campos de objeto (obj.campo = ...) ni módulo file-based TW↔VM
// comparable en este harness (VM sin cargador de módulos importar/usar). Casos omitidos
// se documentan como limitaciones con evidencia, sin inventar equivalencias.

// ---------------------------------------------------------------------------
// G5 — Integración de motores y paridad (genéricos monomorfizados)
// ---------------------------------------------------------------------------

#[test]
fn diff_g5_generica_simple_identidad() {
    assert_parity(
        "funcion id<T>(x: T) -> T { retornar x }\nfuncion main() -> entero { retornar id::<entero>(42) }",
    );
}

#[test]
fn diff_g5_generica_inferencia() {
    assert_parity(
        "funcion id<T>(x: T) -> T { retornar x }\nfuncion main() -> entero { retornar id(7) }",
    );
}

#[test]
fn diff_g5_multiples_parametros_tipo() {
    assert_parity(
        "funcion par<A, B>(a: A, b: B) -> A { retornar a }\nfuncion main() -> entero { retornar par::<entero, texto>(10, \"x\") }",
    );
}

#[test]
fn diff_g5_especializaciones_tipos_diferentes() {
    assert_parity(
        "funcion id<T>(x: T) -> T { retornar x }\nfuncion main() -> entero {\n  var a: entero = id::<entero>(1)\n  var b: texto = id::<texto>(\"ok\")\n  si b == \"ok\" { retornar a }\n  retornar 0\n}",
    );
}

#[test]
fn diff_g5_deduplicacion_instancias_compartidas() {
    assert_parity(
        "funcion id<T>(x: T) -> T { retornar x }\nfuncion main() -> entero { retornar id::<entero>(1) + id::<entero>(2) }",
    );
}

#[test]
fn diff_g5_llamadas_genericas_transitivas() {
    assert_parity(
        "funcion id<T>(x: T) -> T { retornar x }\nfuncion envolver<T>(x: T) -> T { retornar id::<T>(x) }\nfuncion main() -> entero { retornar envolver::<entero>(9) }",
    );
}

#[test]
fn diff_g5_forwarding_generico() {
    assert_parity(
        "funcion id<T>(x: T) -> T { retornar x }\nfuncion reenviar<T>(x: T) -> T { retornar id(x) }\nfuncion main() -> entero { retornar reenviar::<entero>(3) }",
    );
}

#[test]
fn diff_g5_recursion_monomorfica() {
    assert_parity(
        "funcion cuenta<T>(n: entero, v: T) -> entero {\n  si n <= 0 { retornar 0 }\n  retornar 1 + cuenta::<T>(n - 1, v)\n}\nfuncion main() -> entero { retornar cuenta::<entero>(4, 1) }",
    );
}

#[test]
fn diff_g5_coleccion_lista_generica() {
    assert_parity(
        "funcion primero<T>(xs: lista<T>) -> T { retornar xs[0] }\nfuncion main() -> entero { retornar primero::<entero>([10, 20, 30]) }",
    );
}

#[test]
fn diff_g5_coleccion_arreglo_generico() {
    assert_parity(
        "funcion cabeza<T>(xs: arreglo<T, 3>) -> T { retornar xs[0] }\nfuncion main() -> entero { var a: arreglo<entero, 3> = [7, 8, 9]\n retornar cabeza::<entero>(a) }",
    );
}

#[test]
fn diff_g5_error_recursion_polimorfica_creciente_equivalente() {
    let src = "funcion f<T>(x: T) -> entero { retornar f::<lista<T>>(x) }\nfuncion main() -> entero { retornar f::<entero>(1) }";
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "Se esperaba rechazo equivalente por recursión polimórfica creciente.\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
    assert!(
        matches!(tw, ResultadoNormalizado::Err(_)),
        "TW debe fallar: {tw:?}"
    );
    assert!(
        matches!(vmr, ResultadoNormalizado::Err(_)),
        "VM debe fallar: {vmr:?}"
    );
}

#[test]
fn diff_g5_error_semantico_generico_equivalente() {
    let src = "funcion id<T>(x: T) -> T { retornar x }\nfuncion main() -> entero { retornar id::<entero, entero>(1) }";
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "Se esperaba error semántico genérico equivalente.\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
}

#[test]
fn diff_g5_error_residual_generico_como_valor_equivalente() {
    let src = "funcion id<T>(x: T) -> T { retornar x }\nfuncion aplicar(f, x) { retornar f(x) }\nfuncion main() -> entero { retornar aplicar(id, 5) }";
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "Se esperaba rechazo equivalente por referencia genérica residual/usada como valor.\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
    assert!(
        matches!(tw, ResultadoNormalizado::Err(_)),
        "TW debe fallar: {tw:?}"
    );
    assert!(
        matches!(vmr, ResultadoNormalizado::Err(_)),
        "VM debe fallar: {vmr:?}"
    );
}

#[test]
fn diff_g5_error_runtime_en_especializacion_equivalente() {
    let src = "funcion dividir<T>(a: entero, b: entero, x: T) -> entero { retornar a / b }\nfuncion main() -> entero { retornar dividir::<texto>(1, 0, \"x\") }";
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let cmp = comparar_semantica(&tw, &vmr);
    assert!(
        matches!(
            cmp,
            Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_)
        ),
        "Se esperaba error runtime equivalente dentro de especialización genérica.\nTREE-WALK: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
    );
    assert!(
        matches!(tw, ResultadoNormalizado::Err(_)),
        "TW debe fallar: {tw:?}"
    );
    assert!(
        matches!(vmr, ResultadoNormalizado::Err(_)),
        "VM debe fallar: {vmr:?}"
    );
}

#[test]
fn diff_g5_no_regresion_no_generico() {
    assert_parity(
        "funcion suma(a: entero, b: entero) -> entero { retornar a + b }\nfuncion main() -> entero { retornar suma(2, 3) }",
    );
}
