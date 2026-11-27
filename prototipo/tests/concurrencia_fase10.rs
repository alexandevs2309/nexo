//! Fase 10: pruebas de concurrencia robusta (tareas/canales, TW↔VM, determinismo).

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

fn assert_parity(src: &str) {
    let tw = {
        let _g = basilisco::evaluador::instalar_contexto_ejecucion();
        tw_resultado(src)
    };
    let vmr = {
        let _g = basilisco::evaluador::instalar_contexto_ejecucion();
        vm_resultado(src)
    };
    let cmp = comparar_semantica(&tw, &vmr);
    match cmp {
        Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_) => {}
        other => panic!("divergencia TW↔VM\nSRC:\n{src}\nTW: {tw:?}\nVM: {vmr:?}\nCMP: {other:?}"),
    }
}

fn assert_parity_err(src: &str) {
    let tw = {
        let _g = basilisco::evaluador::instalar_contexto_ejecucion();
        tw_resultado(src)
    };
    let vmr = {
        let _g = basilisco::evaluador::instalar_contexto_ejecucion();
        vm_resultado(src)
    };
    let cmp = comparar_semantica(&tw, &vmr);
    match (&tw, &vmr, cmp) {
        (ResultadoNormalizado::Err(_), ResultadoNormalizado::Err(_), Comparacion::Equivalente)
        | (
            ResultadoNormalizado::Err(_),
            ResultadoNormalizado::Err(_),
            Comparacion::DiferenciaDiagnostico(_),
        ) => {}
        (tw, vmr, cmp) => panic!(
            "se esperaba error equivalente TW↔VM\nSRC:\n{src}\nTW: {tw:?}\nVM: {vmr:?}\nCMP: {cmp:?}"
        ),
    }
}

// --- Tareas ---

#[test]
fn f10_tarea_basica() {
    assert_parity(
        "funcion t() -> entero { retornar 42 }\nfuncion main() -> entero { sea id = iniciar_tarea(\"t\")\n retornar esperar_tarea(id) }",
    );
}

#[test]
fn f10_tarea_dos_resultados() {
    assert_parity(
        "funcion a() -> entero { retornar 10 }\nfuncion b() -> entero { retornar 32 }\nfuncion main() -> entero {\n sea ia = iniciar_tarea(\"a\")\n sea ib = iniciar_tarea(\"b\")\n retornar esperar_tarea(ia) + esperar_tarea(ib)\n}",
    );
}

#[test]
fn f10_tarea_inexistente() {
    assert_parity_err("funcion main() -> entero { retornar esperar_tarea(99) }");
}

#[test]
fn f10_tarea_doble_espera() {
    assert_parity_err(
        "funcion t() -> entero { retornar 1 }\nfuncion main() -> entero {\n sea id = iniciar_tarea(\"t\")\n esperar_tarea(id)\n retornar esperar_tarea(id)\n}",
    );
}

#[test]
fn f10_tarea_funcion_inexistente() {
    assert_parity_err("funcion main() -> entero { retornar iniciar_tarea(\"no_existe\") }");
}

// --- Canales ---

#[test]
fn f10_canal_fifo_basico() {
    assert_parity(
        "funcion main() -> entero {\n sea c = crear_canal()\n enviar(c, 1)\n enviar(c, 2)\n enviar(c, 3)\n sea a = recibir(c)\n sea b = recibir(c)\n sea d = recibir(c)\n retornar a * 100 + b * 10 + d\n}",
    );
}

#[test]
fn f10_canal_productor_consumidor() {
    assert_parity(
        "funcion p() { enviar(0, 7)\n retornar 0 }\nfuncion main() -> entero {\n sea c = crear_canal()\n sea t = iniciar_tarea(\"p\")\n sea v = recibir(c)\n esperar_tarea(t)\n retornar v\n}",
    );
}

#[test]
fn f10_canal_cerrar_idempotente() {
    assert_parity(
        "funcion main() -> entero {\n sea c = crear_canal()\n cerrar_canal(c)\n cerrar_canal(c)\n retornar 1\n}",
    );
}

#[test]
fn f10_canal_enviar_cerrado() {
    assert_parity_err(
        "funcion main() -> entero {\n sea c = crear_canal()\n cerrar_canal(c)\n enviar(c, 1)\n retornar 0\n}",
    );
}

#[test]
fn f10_canal_recibir_cerrado_vacio() {
    assert_parity_err(
        "funcion main() -> entero {\n sea c = crear_canal()\n cerrar_canal(c)\n retornar recibir(c)\n}",
    );
}

#[test]
fn f10_canal_recibir_drena_antes_de_cerrar() {
    assert_parity(
        "funcion main() -> entero {\n sea c = crear_canal()\n enviar(c, 10)\n enviar(c, 20)\n cerrar_canal(c)\n sea a = recibir(c)\n sea b = recibir(c)\n retornar a + b\n}",
    );
}

#[test]
fn f10_canal_inexistente_enviar() {
    assert_parity_err("funcion main() -> entero { enviar(99, 1)\n retornar 0 }");
}

#[test]
fn f10_canal_inexistente_recibir() {
    assert_parity_err("funcion main() -> entero { retornar recibir(99) }");
}

#[test]
fn f10_canal_inexistente_cerrar() {
    assert_parity_err("funcion main() -> entero { cerrar_canal(99)\n retornar 0 }");
}

#[test]
fn f10_tarea_anidada() {
    assert_parity(
        "funcion hija() -> entero { retornar 5 }\nfuncion padre() -> entero {\n sea t = iniciar_tarea(\"hija\")\n retornar esperar_tarea(t) + 10\n}\nfuncion main() -> entero {\n sea t = iniciar_tarea(\"padre\")\n retornar esperar_tarea(t)\n}",
    );
}

#[test]
fn f10_determinismo_repetido() {
    let src2 = "funcion p() { enviar(0, 1)\n enviar(0, 2)\n enviar(0, 3)\n retornar 0 }\nfuncion main() -> entero {\n crear_canal()\n sea t = iniciar_tarea(\"p\")\n sea a = recibir(0)\n sea b = recibir(0)\n sea d = recibir(0)\n esperar_tarea(t)\n retornar a*100+b*10+d\n}";
    for _ in 0..12 {
        assert_parity(src2);
    }
}

#[test]
fn f10_tarea_error_propagado() {
    assert_parity_err(
        "funcion boom() -> entero { retornar 1 / 0 }\nfuncion main() -> entero {\n sea t = iniciar_tarea(\"boom\")\n retornar esperar_tarea(t)\n}",
    );
}

#[test]
fn f10_multi_productor_un_consumidor_suma() {
    // Orden de llegada no determinista, suma sí.
    let src = "funcion p1() { enviar(0, 10)\n retornar 0 }\nfuncion p2() { enviar(0, 20)\n retornar 0 }\nfuncion p3() { enviar(0, 12)\n retornar 0 }\nfuncion main() -> entero {\n crear_canal()\n sea t1 = iniciar_tarea(\"p1\")\n sea t2 = iniciar_tarea(\"p2\")\n sea t3 = iniciar_tarea(\"p3\")\n sea a = recibir(0)\n sea b = recibir(0)\n sea c = recibir(0)\n esperar_tarea(t1)\n esperar_tarea(t2)\n esperar_tarea(t3)\n retornar a + b + c\n}";
    for _ in 0..8 {
        assert_parity(src);
    }
}

#[test]
fn f10_aislamiento_ejecuciones() {
    // Dos ejecuciones independientes no deben compartir IDs/canales.
    let src = "funcion main() -> entero {\n sea c = crear_canal()\n enviar(c, 5)\n retornar recibir(c)\n}";
    for _ in 0..6 {
        assert_parity(src);
    }
}

#[test]
fn f10_cerrar_desbloquea_receptor() {
    // El receptor bloqueado debe despertarse con canal cerrado y vacío.
    // Resultado observable: la tarea falla por recibir en canal cerrado/vacío.
    let src = "funcion worker() -> entero { retornar recibir(0) }\nfuncion main() -> entero {\n crear_canal()\n sea t = iniciar_tarea(\"worker\")\n cerrar_canal(0)\n retornar esperar_tarea(t)\n}";
    for _ in 0..8 {
        assert_parity_err(src);
    }
}
