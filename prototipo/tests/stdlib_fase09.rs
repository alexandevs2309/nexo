use basilisco::{
    CausaError, Comparacion, ContextoSemantico, EfectosObservables, Entorno, Lexer, Parser,
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
    let hir_p = match hir::lower_programa(&ss, &ctx) {
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
    let mir_p = mir::lower_programa(&hir_p);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir_p);
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

#[allow(dead_code)]
fn assert_paridad(src: &str) {
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    match comparar_semantica(&tw, &vmr) {
        Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_) => {}
        other => panic!("divergencia TW↔VM\nSRC:\n{src}\nTW:{tw:?}\nVM:{vmr:?}\nCMP:{other:?}"),
    }
}

fn assert_ok_entero(src: &str, esperado: i64) {
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let exp = ResultadoNormalizado::Ok {
        valor: Some(basilisco::paridad::ValorNormalizado::Entero(esperado)),
        efectos: EfectosObservables::default(),
    };
    assert!(
        matches!(comparar_semantica(&tw, &exp), Comparacion::Equivalente),
        "TW inesperado\nSRC:{src}\nTW:{tw:?}\nESP:{exp:?}"
    );
    assert!(
        matches!(comparar_semantica(&vmr, &exp), Comparacion::Equivalente),
        "VM inesperado\nSRC:{src}\nVM:{vmr:?}\nESP:{exp:?}"
    );
}

fn assert_ok_texto(src: &str, esperado: &str) {
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    let exp = ResultadoNormalizado::Ok {
        valor: Some(basilisco::paridad::ValorNormalizado::Texto(
            esperado.to_string(),
        )),
        efectos: EfectosObservables::default(),
    };
    assert!(
        matches!(comparar_semantica(&tw, &exp), Comparacion::Equivalente),
        "TW inesperado\nSRC:{src}\nTW:{tw:?}\nESP:{exp:?}"
    );
    assert!(
        matches!(comparar_semantica(&vmr, &exp), Comparacion::Equivalente),
        "VM inesperado\nSRC:{src}\nVM:{vmr:?}\nESP:{exp:?}"
    );
}

fn assert_error_paridad(src: &str) {
    let tw = tw_resultado(src);
    let vmr = vm_resultado(src);
    assert!(
        matches!(tw, ResultadoNormalizado::Err(_)),
        "TW debía fallar: {src}\nTW:{tw:?}"
    );
    assert!(
        matches!(vmr, ResultadoNormalizado::Err(_)),
        "VM debía fallar: {src}\nVM:{vmr:?}"
    );
    match comparar_semantica(&tw, &vmr) {
        Comparacion::Equivalente | Comparacion::DiferenciaDiagnostico(_) => {}
        other => {
            panic!("error no equivalente TW↔VM\nSRC:\n{src}\nTW:{tw:?}\nVM:{vmr:?}\nCMP:{other:?}")
        }
    }
}

#[test]
fn stdlib_texto_conversion_valores_y_paridad() {
    let casos = [
        ("funcion main() -> texto { retornar texto(42) }", "42"),
        ("funcion main() -> texto { retornar texto(-7) }", "-7"),
        (
            "funcion main() -> texto { retornar texto(verdadero) }",
            "verdadero",
        ),
        ("funcion main() -> texto { retornar texto(falso) }", "falso"),
        ("funcion main() -> texto { retornar texto(nulo) }", "nulo"),
        (
            "funcion main() -> texto { retornar texto(a_decimal(\"3.5\")) }",
            "3.5",
        ),
        (
            "funcion main() -> texto { retornar texto([1, 2, 3]) }",
            "[1, 2, 3]",
        ),
        (
            "funcion main() -> texto { retornar texto([verdadero, nulo]) }",
            "[verdadero, nulo]",
        ),
    ];
    for (src, esperado) in casos {
        assert_ok_texto(src, esperado);
    }
}

#[test]
fn stdlib_texto_errores_paridad() {
    let casos = [
        "funcion main() -> texto { retornar texto() }",
        "funcion main() -> texto { retornar texto(1, 2) }",
    ];
    for src in casos {
        assert_error_paridad(src);
    }
}

#[test]
fn stdlib_piso_techo_redondear_enteros() {
    for (src, esperado) in [
        ("funcion main() -> entero { retornar piso(5) }", 5i64),
        ("funcion main() -> entero { retornar techo(5) }", 5i64),
        ("funcion main() -> entero { retornar redondear(5) }", 5i64),
        ("funcion main() -> entero { retornar piso(-5) }", -5i64),
        ("funcion main() -> entero { retornar techo(-5) }", -5i64),
        ("funcion main() -> entero { retornar redondear(-5) }", -5i64),
        ("funcion main() -> entero { retornar piso(0) }", 0i64),
    ] {
        assert_ok_entero(src, esperado);
    }
}

#[test]
fn stdlib_piso_techo_redondear_decimales_basicos() {
    for (src, esperado) in [
        (
            "funcion main() -> entero { retornar piso(a_decimal(\"3.7\")) }",
            3i64,
        ),
        (
            "funcion main() -> entero { retornar techo(a_decimal(\"3.2\")) }",
            4i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"3.4\")) }",
            3i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"3.6\")) }",
            4i64,
        ),
        (
            "funcion main() -> entero { retornar piso(a_decimal(\"-1.2\")) }",
            -2i64,
        ),
        (
            "funcion main() -> entero { retornar techo(a_decimal(\"-1.2\")) }",
            -1i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"-1.2\")) }",
            -1i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"-1.8\")) }",
            -2i64,
        ),
        (
            "funcion main() -> entero { retornar piso(a_decimal(\"0.0\")) }",
            0i64,
        ),
        (
            "funcion main() -> entero { retornar techo(a_decimal(\"0.0\")) }",
            0i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"0.0\")) }",
            0i64,
        ),
    ] {
        assert_ok_entero(src, esperado);
    }
}

#[test]
fn stdlib_piso_techo_redondear_half_away_from_zero() {
    for (src, esperado) in [
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"0.5\")) }",
            1i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"-0.5\")) }",
            -1i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"1.5\")) }",
            2i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"-1.5\")) }",
            -2i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"2.5\")) }",
            3i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"-2.5\")) }",
            -3i64,
        ),
    ] {
        assert_ok_entero(src, esperado);
    }
}

#[test]
fn stdlib_piso_techo_redondear_decimales_grandes() {
    // Valores mayores que 2^53 para exponer pérdida de precisión f64; deben ser correctos por aritmética escalada.
    for (src, esperado) in [
        (
            "funcion main() -> entero { retornar piso(a_decimal(\"9007199254740993.1\")) }",
            9007199254740993i64,
        ),
        (
            "funcion main() -> entero { retornar techo(a_decimal(\"9007199254740993.1\")) }",
            9007199254740994i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"9007199254740993.5\")) }",
            9007199254740994i64,
        ),
        (
            "funcion main() -> entero { retornar piso(a_decimal(\"-9007199254740993.1\")) }",
            -9007199254740994i64,
        ),
        (
            "funcion main() -> entero { retornar techo(a_decimal(\"-9007199254740993.1\")) }",
            -9007199254740993i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_decimal(\"-9007199254740993.5\")) }",
            -9007199254740994i64,
        ),
    ] {
        assert_ok_entero(src, esperado);
    }
}

#[test]
fn stdlib_piso_techo_redondear_reales() {
    for (src, esperado) in [
        (
            "funcion main() -> entero { retornar piso(a_real(\"3.7\")) }",
            3i64,
        ),
        (
            "funcion main() -> entero { retornar techo(a_real(\"3.2\")) }",
            4i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_real(\"3.5\")) }",
            4i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_real(\"-3.5\")) }",
            -4i64,
        ),
        (
            "funcion main() -> entero { retornar piso(a_real(\"-1.2\")) }",
            -2i64,
        ),
        (
            "funcion main() -> entero { retornar techo(a_real(\"-1.2\")) }",
            -1i64,
        ),
    ] {
        assert_ok_entero(src, esperado);
    }
}

#[test]
fn stdlib_piso_techo_redondear_errores_paridad() {
    let casos = [
        "funcion main() -> entero { retornar piso() }",
        "funcion main() -> entero { retornar piso(1, 2) }",
        "funcion main() -> entero { retornar techo() }",
        "funcion main() -> entero { retornar redondear(\"x\") }",
        "funcion main() -> entero { retornar piso(verdadero) }",
        "funcion main() -> entero { retornar techo(nulo) }",
    ];
    for src in casos {
        assert_error_paridad(src);
    }
}

#[test]
fn stdlib_posicion_texto_unicode_y_bordes() {
    for (src, esperado) in [
        (
            "funcion main() -> entero { retornar posicion(\"hola mundo\", \"mundo\") }",
            5i64,
        ),
        (
            "funcion main() -> entero { retornar posicion(\"hola\", \"z\") }",
            -1i64,
        ),
        (
            "funcion main() -> entero { retornar posicion(\"\", \"\") }",
            0i64,
        ),
        (
            "funcion main() -> entero { retornar posicion(\"abc\", \"\") }",
            0i64,
        ),
        (
            "funcion main() -> entero { retornar posicion(\"\", \"a\") }",
            -1i64,
        ),
        (
            "funcion main() -> entero { retornar posicion(\"aéíóu\", \"í\") }",
            2i64,
        ),
        (
            "funcion main() -> entero { retornar posicion(\"😀a😀\", \"a\") }",
            1i64,
        ),
        (
            "funcion main() -> entero { retornar posicion(\"😀a😀\", \"😀\") }",
            0i64,
        ),
        (
            "funcion main() -> entero { retornar posicion(\"café\", \"é\") }",
            3i64,
        ),
    ] {
        assert_ok_entero(src, esperado);
    }
}

#[test]
fn stdlib_posicion_colecciones() {
    for (src, esperado) in [
        (
            "funcion main() -> entero { retornar posicion([1, 2, 3], 2) }",
            1i64,
        ),
        (
            "funcion main() -> entero { retornar posicion([1, 2, 3], 9) }",
            -1i64,
        ),
        (
            "funcion main() -> entero { retornar posicion([], 1) }",
            -1i64,
        ),
        (
            "funcion main() -> entero { retornar posicion([verdadero, falso], falso) }",
            1i64,
        ),
        (
            "funcion main() -> entero { retornar posicion([\"a\", \"b\"], \"b\") }",
            1i64,
        ),
    ] {
        assert_ok_entero(src, esperado);
    }
}

#[test]
fn stdlib_posicion_errores_paridad() {
    let casos = [
        "funcion main() -> entero { retornar posicion(\"a\") }",
        "funcion main() -> entero { retornar posicion(\"a\", \"b\", \"c\") }",
        "funcion main() -> entero { retornar posicion(1, 2) }",
        "funcion main() -> entero { retornar posicion(verdadero, falso) }",
    ];
    for src in casos {
        assert_error_paridad(src);
    }
}

#[test]
fn stdlib_piso_techo_redondear_limites_i64_real() {
    // Límite superior exclusivo 2^63: debe producir error (no saturación silenciosa).
    for src in [
        "funcion main() -> entero { retornar piso(a_real(\"9223372036854775808\")) }",
        "funcion main() -> entero { retornar techo(a_real(\"9223372036854775808\")) }",
        "funcion main() -> entero { retornar redondear(a_real(\"9223372036854775808\")) }",
    ] {
        assert_error_paridad(src);
    }

    // f64 inmediatamente inferior a 2^63 (2^63 - 1024, representable): debe aceptarse.
    for (src, esperado) in [
        (
            "funcion main() -> entero { retornar piso(a_real(\"9223372036854774784\")) }",
            9223372036854774784i64,
        ),
        (
            "funcion main() -> entero { retornar techo(a_real(\"9223372036854774784\")) }",
            9223372036854774784i64,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_real(\"9223372036854774784\")) }",
            9223372036854774784i64,
        ),
    ] {
        assert_ok_entero(src, esperado);
    }

    // -2^63 exacto (i64::MIN): debe aceptarse.
    for (src, esperado) in [
        (
            "funcion main() -> entero { retornar piso(a_real(\"-9223372036854775808\")) }",
            i64::MIN,
        ),
        (
            "funcion main() -> entero { retornar techo(a_real(\"-9223372036854775808\")) }",
            i64::MIN,
        ),
        (
            "funcion main() -> entero { retornar redondear(a_real(\"-9223372036854775808\")) }",
            i64::MIN,
        ),
    ] {
        assert_ok_entero(src, esperado);
    }

    // f64 inmediatamente inferior a -2^63 (más negativo, -2^63 - 2048): debe producir error.
    for src in [
        "funcion main() -> entero { retornar piso(a_real(\"-9223372036854777856\")) }",
        "funcion main() -> entero { retornar techo(a_real(\"-9223372036854777856\")) }",
        "funcion main() -> entero { retornar redondear(a_real(\"-9223372036854777856\")) }",
    ] {
        assert_error_paridad(src);
    }

    // Valores que tras redondeo alcanzan 2^63: deben producir error.
    // 9223372036854775808.0 es 2^63; round/floor/ceil lo mantienen en 2^63.
    for src in [
        "funcion main() -> entero { retornar redondear(a_real(\"9223372036854775808\")) }",
        "funcion main() -> entero { retornar piso(a_real(\"9223372036854775808\")) }",
        "funcion main() -> entero { retornar techo(a_real(\"9223372036854775808\")) }",
    ] {
        assert_error_paridad(src);
    }
}
