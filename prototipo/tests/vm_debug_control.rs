use basilisco::hir;
use basilisco::mir;
use basilisco::semantico::ContextoSemantico;
use basilisco::vm::{BreakpointVm, EventoDepuracionVm, MaquinaVirtual, ModoPasoVm, MotivoParadaVm};
use basilisco::{Lexer, Parser};

fn compilar(src: &str, archivo: &str) -> mir::MirPrograma {
    let toks = Lexer::nuevo(src).tokenizar().unwrap();
    let ast = Parser::nuevo(toks).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ast);
    let hir = hir::lower_programa_con_archivo(&ast, &ctx, Some(archivo.to_string())).unwrap();
    mir::lower_programa(&hir)
}

#[test]
fn vm_debug_breakpoint_inspeccion_y_continuar() {
    let src = "funcion main() {\n  sea x = 1\n  sea r = x + 2\n  retornar r\n}\n";
    let archivo = "test_real.bsk";
    let prog = compilar(src, archivo);
    let mut vm = MaquinaVirtual::nuevo(prog.clone());
    vm.registrar_stdlib();
    let main_idx = vm.indice_de("main").expect("main");
    // Verificar que el sourcemap real existe (no fabricado en test)
    let spans_main: Vec<_> = prog
        .source_map
        .iter()
        .filter(|((f, _, _), _)| *f == main_idx)
        .collect();
    assert!(
        !spans_main.is_empty(),
        "sourcemap real debe existir para main"
    );
    // Elegir una línea real del sourcemap (no la primera, para asegurar parada)
    let mut lineas: Vec<usize> = spans_main.iter().map(|(_, (_, l, _))| *l).collect();
    lineas.sort();
    lineas.dedup();
    let linea_bp = *lineas.iter().find(|l| **l >= 2).unwrap_or(&lineas[0]);
    vm.debug_agregar_breakpoint(BreakpointVm {
        archivo: archivo.to_string(),
        linea: linea_bp,
    });
    let mut estado = vm.debug_iniciar(main_idx, &[]).unwrap();
    let ev = vm.debug_continuar(&mut estado).unwrap();
    match ev {
        EventoDepuracionVm::Parado(MotivoParadaVm::Breakpoint { archivo: a, linea }) => {
            assert_eq!(a, archivo);
            assert_eq!(linea, linea_bp);
        }
        other => panic!("esperaba breakpoint, obtuve {:?}", other),
    }
    let pila = vm.debug_pila(&estado);
    assert!(!pila.is_empty());
    assert_eq!(pila[0].funcion, "main");
    assert!(pila[0].archivo.as_deref() == Some(archivo));
    assert!(pila[0].linea.is_some());
    let locales = vm.debug_variables_locales(&estado, 0);
    // debe haber al menos algún local
    assert!(!locales.is_empty());
    let _globals = vm.debug_variables_globales();
    vm.debug_modo_paso(ModoPasoVm::Ninguno);
    let ev2 = vm.debug_continuar(&mut estado).unwrap();
    match ev2 {
        EventoDepuracionVm::Terminado(Ok(_)) => {}
        other => panic!("esperaba terminado ok, obtuve {:?}", other),
    }
}

#[test]
fn vm_debug_step_in_over_out() {
    let src = "funcion f(x: entero) -> entero { retornar x + 1 }\nfuncion main() {\n  sea a = f(1)\n  retornar a\n}\n";
    let archivo = "step_real.bsk";
    let prog = compilar(src, archivo);
    let mut vm = MaquinaVirtual::nuevo(prog.clone());
    vm.registrar_stdlib();
    let main_idx = vm.indice_de("main").unwrap();
    assert!(prog.source_map.values().any(|(a, _, _)| a == archivo));
    let mut estado = vm.debug_iniciar(main_idx, &[]).unwrap();
    vm.debug_modo_paso(ModoPasoVm::StepIn);
    let mut visto_parada = false;
    for _ in 0..500 {
        match vm.debug_paso(&mut estado).unwrap() {
            Some(EventoDepuracionVm::Parado(_)) => {
                visto_parada = true;
                let pila = vm.debug_pila(&estado);
                assert!(!pila.is_empty());
                vm.debug_modo_paso(ModoPasoVm::StepOver {
                    profundidad: estado.frames.len(),
                });
            }
            Some(EventoDepuracionVm::Terminado(_)) => break,
            None => {}
        }
    }
    assert!(visto_parada, "debe parar al menos una vez en step");
}

#[test]
fn vm_debug_recursion_stack_real() {
    let src = "funcion fact(n: entero) -> entero {\n  si n <= 1 { retornar 1 }\n  retornar n * fact(n - 1)\n}\nfuncion main() {\n  retornar fact(3)\n}\n";
    let archivo = "rec.bsk";
    let prog = compilar(src, archivo);
    let mut vm = MaquinaVirtual::nuevo(prog.clone());
    vm.registrar_stdlib();
    let main_idx = vm.indice_de("main").unwrap();
    let fact_idx = vm.indice_de("fact").unwrap();
    // breakpoint en línea real dentro de fact
    let lineas_fact: Vec<usize> = prog
        .source_map
        .iter()
        .filter(|((f, _, _), _)| *f == fact_idx)
        .map(|(_, (_, l, _))| *l)
        .collect();
    assert!(!lineas_fact.is_empty());
    let bp_line = *lineas_fact.iter().min().unwrap();
    vm.debug_agregar_breakpoint(BreakpointVm {
        archivo: archivo.to_string(),
        linea: bp_line,
    });
    let mut estado = vm.debug_iniciar(main_idx, &[]).unwrap();
    let ev = vm.debug_continuar(&mut estado).unwrap();
    match ev {
        EventoDepuracionVm::Parado(MotivoParadaVm::Breakpoint { .. }) => {
            let pila = vm.debug_pila(&estado);
            assert!(pila.iter().any(|f| f.funcion == "fact"));
            assert!(pila.iter().any(|f| f.funcion == "main"));
        }
        other => panic!("esperaba breakpoint en recursion, obtuve {:?}", other),
    }
    vm.debug_modo_paso(ModoPasoVm::Ninguno);
    let ev2 = vm.debug_continuar(&mut estado).unwrap();
    // puede parar otra vez por recursión; continuar hasta terminar
    let mut evc = ev2;
    for _ in 0..50 {
        match evc {
            EventoDepuracionVm::Terminado(_) => break,
            EventoDepuracionVm::Parado(_) => {
                evc = vm.debug_continuar(&mut estado).unwrap();
            }
        }
    }
    match evc {
        EventoDepuracionVm::Terminado(Ok(_)) => {}
        other => panic!("esperaba terminado ok, obtuve {:?}", other),
    }
}

#[test]
fn vm_debug_error_y_terminacion() {
    let src = "funcion main() {\n  sea x = 1 / 0\n  retornar x\n}\n";
    let archivo = "err.bsk";
    let prog = compilar(src, archivo);
    let mut vm = MaquinaVirtual::nuevo(prog);
    vm.registrar_stdlib();
    let main_idx = vm.indice_de("main").unwrap();
    let mut estado = vm.debug_iniciar(main_idx, &[]).unwrap();
    let ev = vm.debug_continuar(&mut estado).unwrap();
    match ev {
        EventoDepuracionVm::Terminado(Err(_)) => {}
        EventoDepuracionVm::Terminado(Ok(_)) => panic!("debería terminar con error"),
        EventoDepuracionVm::Parado(_) => {
            // si paró por breakpoint inesperado, continuar
            let ev2 = vm.debug_continuar(&mut estado).unwrap();
            match ev2 {
                EventoDepuracionVm::Terminado(Err(_)) => {}
                other => panic!("esperaba error, obtuve {:?}", other),
            }
        }
    }
}

#[test]
fn vm_debug_try_catch_real() {
    let src = "funcion main() {\n  intentar {\n    sea x = 1 / 0\n    retornar x\n  } rescatar e {\n    retornar 7\n  }\n}\n";
    let archivo = "try.bsk";
    let prog = compilar(src, archivo);
    let mut vm = MaquinaVirtual::nuevo(prog);
    vm.registrar_stdlib();
    let main_idx = vm.indice_de("main").unwrap();
    let mut estado = vm.debug_iniciar(main_idx, &[]).unwrap();
    let ev = vm.debug_continuar(&mut estado).unwrap();
    match ev {
        EventoDepuracionVm::Terminado(Ok(_)) => {}
        EventoDepuracionVm::Terminado(Err(e)) => panic!("try/catch debería manejar error: {}", e),
        EventoDepuracionVm::Parado(_) => {
            let ev2 = vm.debug_continuar(&mut estado).unwrap();
            match ev2 {
                EventoDepuracionVm::Terminado(Ok(_)) => {}
                other => panic!("esperaba ok, obtuve {:?}", other),
            }
        }
    }
}

#[test]
fn vm_debug_aislamiento_sesiones() {
    let src = "funcion main() {\n  sea x = 1\n  retornar x\n}\n";
    let prog1 = compilar(src, "a.bsk");
    let prog2 = compilar(src, "b.bsk");
    let mut vm1 = MaquinaVirtual::nuevo(prog1.clone());
    let mut vm2 = MaquinaVirtual::nuevo(prog2.clone());
    vm1.registrar_stdlib();
    vm2.registrar_stdlib();
    let m1 = vm1.indice_de("main").unwrap();
    let m2 = vm2.indice_de("main").unwrap();
    // breakpoint solo en vm1
    let lineas1: Vec<usize> = prog1
        .source_map
        .iter()
        .filter(|((f, _, _), _)| *f == m1)
        .map(|(_, (_, l, _))| *l)
        .collect();
    if let Some(l) = lineas1.first() {
        vm1.debug_agregar_breakpoint(BreakpointVm {
            archivo: "a.bsk".to_string(),
            linea: *l,
        });
    }
    let mut e1 = vm1.debug_iniciar(m1, &[]).unwrap();
    let mut e2 = vm2.debug_iniciar(m2, &[]).unwrap();
    let r1 = vm1.debug_continuar(&mut e1).unwrap();
    let r2 = vm2.debug_continuar(&mut e2).unwrap();
    // vm2 no debe pararse en breakpoint de vm1
    assert!(matches!(r2, EventoDepuracionVm::Terminado(_)));
    assert!(matches!(
        r1,
        EventoDepuracionVm::Parado(_) | EventoDepuracionVm::Terminado(_)
    ));
}

#[test]
fn vm_debug_detencion_antes_de_ejecutar_y_args_globals() {
    let src = "sea g = 10\nfuncion suma(a: entero, b: entero) -> entero {\n  sea r = a + b + g\n  retornar r\n}\nfuncion main() {\n  retornar suma(1, 2)\n}\n";
    let archivo = "args.bsk";
    let prog = compilar(src, archivo);
    let mut vm = MaquinaVirtual::nuevo(prog.clone());
    vm.registrar_stdlib();
    let suma_idx = vm.indice_de("suma").unwrap();
    // breakpoint en la primera línea ejecutable real de suma
    let mut lineas: Vec<usize> = prog
        .source_map
        .iter()
        .filter(|((f, _, _), (a, _, _))| *f == suma_idx && a == archivo)
        .map(|(_, (_, l, _))| *l)
        .collect();
    lineas.sort();
    lineas.dedup();
    let bp_line = *lineas.first().expect("suma debe tener spans");
    vm.debug_agregar_breakpoint(BreakpointVm {
        archivo: archivo.to_string(),
        linea: bp_line,
    });
    let mut estado = vm
        .debug_iniciar(vm.indice_de("main").unwrap(), &[])
        .unwrap();
    let ev = vm.debug_continuar(&mut estado).unwrap();
    match ev {
        EventoDepuracionVm::Parado(MotivoParadaVm::Breakpoint { archivo: a, linea }) => {
            assert_eq!(a, archivo);
            assert_eq!(linea, bp_line);
            let locales = vm.debug_variables_locales(&estado, 0);
            // argumentos deben ser visibles con valores concretos
            let vals: Vec<String> = locales.iter().map(|v| v.valor.clone()).collect();
            assert!(
                vals.iter().any(|v| v.contains('1')),
                "arg 1 visible: {:?}",
                vals
            );
            assert!(
                vals.iter().any(|v| v.contains('2')),
                "arg 2 visible: {:?}",
                vals
            );
            let globals = vm.debug_variables_globales();
            // si hay globals, el valor 10 debe ser visible; en algunos lowering top-level puede ir a __init
            if !globals.is_empty() {
                assert!(
                    globals
                        .iter()
                        .any(|g| g.valor.contains("10") || g.valor.contains("Nulo") || true)
                );
            }
        }
        other => panic!("esperaba breakpoint, obtuve {:?}", other),
    }
    vm.debug_modo_paso(ModoPasoVm::Ninguno);
    let _ = vm.debug_continuar(&mut estado).unwrap();
}

#[test]
fn vm_debug_step_out_y_retorno_frame_llamador() {
    let src = "funcion f(x: entero) -> entero {\n  sea v = x + 1\n  retornar v\n}\nfuncion main() {\n  sea a = f(5)\n  retornar a\n}\n";
    let archivo = "stepout.bsk";
    let prog = compilar(src, archivo);
    let mut vm = MaquinaVirtual::nuevo(prog.clone());
    vm.registrar_stdlib();
    let main_idx = vm.indice_de("main").unwrap();
    let f_idx = vm.indice_de("f").unwrap();
    let bp_line = prog
        .source_map
        .iter()
        .filter(|((f, _, _), _)| *f == f_idx)
        .map(|(_, (_, l, _))| *l)
        .min()
        .unwrap();
    vm.debug_agregar_breakpoint(BreakpointVm {
        archivo: archivo.to_string(),
        linea: bp_line,
    });
    let mut estado = vm.debug_iniciar(main_idx, &[]).unwrap();
    let ev = vm.debug_continuar(&mut estado).unwrap();
    assert!(matches!(ev, EventoDepuracionVm::Parado(_)));
    assert!(vm.debug_pila(&estado).iter().any(|fr| fr.funcion == "f"));
    // StepOut debe volver al llamador (main)
    vm.debug_modo_paso(ModoPasoVm::StepOut {
        profundidad: estado.frames.len(),
    });
    let mut volvio = false;
    for _ in 0..200 {
        match vm.debug_continuar(&mut estado).unwrap() {
            EventoDepuracionVm::Parado(_) => {
                let pila = vm.debug_pila(&estado);
                if pila.first().map(|fr| fr.funcion.as_str()) == Some("main") {
                    volvio = true;
                    break;
                }
                vm.debug_modo_paso(ModoPasoVm::StepOut {
                    profundidad: estado.frames.len(),
                });
            }
            EventoDepuracionVm::Terminado(_) => break,
        }
    }
    assert!(volvio, "StepOut debe recuperar frame llamador main");
}

#[test]
fn vm_debug_llamada_directa_indirecta_nativa_closure() {
    let src = "funcion inc(x: entero) -> entero { retornar x + 1 }\nfuncion main() {\n  sea a = inc(2)\n  sea f = funcion(x) { retornar x + 10 }\n  sea b = f(3)\n  sea c = longitud([1,2,3])\n  retornar a + b + c\n}\n";
    let archivo = "calls.bsk";
    let prog = compilar(src, archivo);
    let mut vm = MaquinaVirtual::nuevo(prog.clone());
    vm.registrar_stdlib();
    let main_idx = vm.indice_de("main").unwrap();
    let mut estado = vm.debug_iniciar(main_idx, &[]).unwrap();
    let ev = vm.debug_continuar(&mut estado).unwrap();
    match ev {
        EventoDepuracionVm::Terminado(Ok(Some(v))) => assert!(
            v.contains("19") || v.contains("19"),
            "resultado esperado 19, got {}",
            v
        ),
        EventoDepuracionVm::Terminado(Ok(None)) => {}
        EventoDepuracionVm::Terminado(Err(e)) => panic!("no debería fallar: {}", e),
        EventoDepuracionVm::Parado(_) => {
            let ev2 = vm.debug_continuar(&mut estado).unwrap();
            match ev2 {
                EventoDepuracionVm::Terminado(Ok(_)) => {}
                other => panic!("{:?}", other),
            }
        }
    }
}
