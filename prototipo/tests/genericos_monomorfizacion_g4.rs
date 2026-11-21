use basilisco::{ContextoSemantico, Entorno, Lexer, Parser, Valor, hir, mir, vm};

fn parsear(src: &str) -> Vec<basilisco::Sentencia> {
    let tokens = Lexer::nuevo(src).tokenizar().expect("lex");
    Parser::nuevo(tokens).parsear_programa().expect("parse")
}

fn tw_ejecutar(src: &str) -> Result<Option<Valor>, String> {
    // Convención diferencial_vm: invocar main() como expresión tope para obtener su valor.
    let src_call = format!("{}\nmain()", src);
    let ss = parsear(&src_call);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    if !errs.is_empty() {
        return Err(errs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n"));
    }
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    if !errs_g4.is_empty() {
        return Err(errs_g4
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n"));
    }
    let mut env = Entorno::nueva();
    env.ejecutar_programa(&ss).map_err(|e| e.to_string())
}

fn vm_ejecutar(src: &str) -> Result<Option<Valor>, String> {
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    if !errs.is_empty() {
        return Err(errs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n"));
    }
    let (ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    if !errs_g4.is_empty() {
        return Err(errs_g4
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n"));
    }
    let hir = hir::lower_programa(&ss, &ctx).map_err(|e| format!("{e:?}"))?;
    hir::verificar_sin_residuos_genericos(&hir)?;
    let mir_prog = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir_prog);
    maquina.registrar_stdlib();
    let idx = maquina.indice_de("main").ok_or("sin main")?;
    maquina.ejecutar_indice(idx, &[]).map_err(|e| e.to_string())
}

fn assert_tw_vm(src: &str, esperado: Valor) {
    let tw = tw_ejecutar(src).expect("tw");
    let vmr = vm_ejecutar(src).expect("vm");
    assert_eq!(tw, Some(esperado.clone()), "tw resultado");
    assert_eq!(vmr, Some(esperado), "vm resultado");
}

fn assert_error_codigo(src: &str, codigo: &str) {
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    if !errs.is_empty() {
        assert!(
            errs.iter().any(|e| e.causa.codigo() == codigo),
            "esperado {codigo} en semantico: {errs:?}"
        );
        return;
    }
    let (_ss, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(!errs_g4.is_empty(), "se esperaba error {codigo}");
    assert!(
        errs_g4.iter().any(|e| e.causa.codigo() == codigo),
        "esperado {codigo} en g4: {errs_g4:?}"
    );
}

#[test]
fn g4_identidad_explicita_basica() {
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion main() -> entero { retornar id::<entero>(42) }
"#;
    assert_tw_vm(src, Valor::Entero(42));
}

#[test]
fn g4_inferencia_basica() {
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion main() -> entero { retornar id(7) }
"#;
    assert_tw_vm(src, Valor::Entero(7));
}

#[test]
fn g4_multiples_instancias() {
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion main() -> entero {
    var a: entero = id::<entero>(1)
    var b: texto = id::<texto>("ok")
    si b == "ok" { retornar a }
    retornar 0
}
"#;
    assert_tw_vm(src, Valor::Entero(1));
}

#[test]
fn g4_deduplicacion_misma_instancia() {
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion main() -> entero {
    retornar id::<entero>(1) + id::<entero>(2)
}
"#;
    assert_tw_vm(src, Valor::Entero(3));
}

#[test]
fn g4_forwarding_directo() {
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion envolver<T>(x: T) -> T { retornar id(x) }
funcion main() -> entero { retornar envolver(9) }
"#;
    assert_tw_vm(src, Valor::Entero(9));
}

#[test]
fn g4_forwarding_transitivo() {
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion a<T>(x: T) -> T { retornar id(x) }
funcion b<T>(x: T) -> T { retornar a(x) }
funcion main() -> entero { retornar b(5) }
"#;
    assert_tw_vm(src, Valor::Entero(5));
}

#[test]
fn g4_recursion_monomorfica() {
    let src = r#"
funcion cuenta<T>(n: entero, x: T) -> entero {
    si n <= 0 { retornar 0 }
    retornar 1 + cuenta(n - 1, x)
}
funcion main() -> entero { retornar cuenta::<entero>(4, 1) }
"#;
    assert_tw_vm(src, Valor::Entero(4));
}

#[test]
fn g4_lista_generica() {
    let src = r#"
funcion primero<T>(xs: lista<T>) -> T { retornar xs[0] }
funcion main() -> entero {
    var l: lista<entero> = [10, 20, 30]
    retornar primero(l)
}
"#;
    assert_tw_vm(src, Valor::Entero(10));
}

#[test]
fn g4_multiples_parametros_tipo() {
    let src = r#"
funcion primero_de_dos<A, B>(a: A, b: B) -> A { retornar a }
funcion main() -> entero { retornar primero_de_dos::<entero, texto>(3, "x") }
"#;
    assert_tw_vm(src, Valor::Entero(3));
}

#[test]
fn g4_recursion_polimorfica_creciente_rechazada() {
    // f<T> llama f<lista<T>> -> crecimiento estructural (BAS-7018/E1053)
    let src = r#"
funcion f<T>(x: T) -> entero {
    var xs: lista<T> = [x]
    retornar f(xs)
}
funcion main() -> entero { retornar f::<entero>(1) }
"#;
    assert_error_codigo(src, "E1053");
}

#[test]
fn g4_programa_no_generico_sin_regresion() {
    let src = r#"
funcion suma(a: entero, b: entero) -> entero { retornar a + b }
funcion main() -> entero { retornar suma(2, 3) }
"#;
    assert_tw_vm(src, Valor::Entero(5));
}

#[test]
fn g4_deduplicacion_determinista_reuso() {
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion main() -> entero {
    var a: entero = id::<entero>(1)
    var b: entero = id::<entero>(2)
    var c: entero = id(3)
    retornar a + b + c
}
"#;
    assert_tw_vm(src, Valor::Entero(6));
}

#[test]
fn g4_recursion_monomorfica_en_cadena() {
    // f<T> se llama a sí misma con el mismo T (instancia monomórfica).
    let src = r#"
funcion cuenta<T>(n: entero, x: T) -> entero {
    si n <= 0 { retornar 0 }
    retornar 1 + cuenta(n - 1, x)
}
funcion main() -> entero { retornar cuenta::<entero>(3, 7) }
"#;
    assert_tw_vm(src, Valor::Entero(3));
}

#[test]
fn g4_limite_instancias_excedido_e1056() {
    // 257 instancias únicas de la misma plantilla (M1=256).
    fn tipo_lista_anidado(n: usize) -> String {
        let mut t = "entero".to_string();
        for _ in 0..n {
            t = format!("lista<{t}>");
        }
        t
    }
    let mut src = String::from(
        "funcion id<T>(x: T) -> T { retornar x }\nfuncion main() -> entero {\n    var v0: entero = id::<entero>(0)\n",
    );
    for i in 1..257 {
        let ty = tipo_lista_anidado(i);
        src.push_str(&format!(
            "    var v{i}: {ty} = id::<{ty}>([v{j}])\n",
            j = i - 1
        ));
    }
    src.push_str("    retornar 0\n}\n");
    assert_error_codigo(&src, "E1056");
}

#[test]
fn g4_profundidad_instanciacion_excedida_e1054() {
    // Cadena de dependencias de instanciación profunda: excede M2(32).
    let n = 50usize;
    let mut src = String::new();
    for i in 0..n {
        if i + 1 < n {
            src.push_str(&format!(
                "funcion f{i}<T>(x: T) -> T {{ retornar f{j}::<T>(x) }}\n",
                j = i + 1
            ));
        } else {
            src.push_str(&format!("funcion f{i}<T>(x: T) -> T {{ retornar x }}\n"));
        }
    }
    src.push_str("funcion main() -> entero { retornar f0::<entero>(1) }\n");
    assert_error_codigo(&src, "E1054");
}

#[test]
fn g4_error_transaccional_sin_salida_parcial() {
    // Si G4 falla, no debe devolver programa parcialmente especializado.
    // Cadena profunda: semántica acepta (V3 acotada), G4 detecta E1054.
    let n = 50usize;
    let mut src = String::new();
    for i in 0..n {
        if i + 1 < n {
            src.push_str(&format!(
                "funcion f{i}<T>(x: T) -> T {{ retornar f{j}::<T>(x) }}\n",
                j = i + 1
            ));
        } else {
            src.push_str(&format!("funcion f{i}<T>(x: T) -> T {{ retornar x }}\n"));
        }
    }
    src.push_str("funcion main() -> entero { retornar f0::<entero>(1) }\n");
    let ss = parsear(&src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(errs.is_empty(), "sem: {:?}", errs);
    let (out, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(!errs_g4.is_empty(), "debe fallar en G4");
    // Salida transaccional: programa original sin instancias especializadas G4.
    let tiene_especializada = out.iter().any(
        |s| matches!(s, basilisco::Sentencia::Funcion { nombre, .. } if nombre.contains("__g4__")),
    );
    assert!(
        !tiene_especializada,
        "no debe haber instancias parciales en salida con error"
    );
    assert_eq!(out.len(), ss.len(), "debe preservar programa original");
}

#[test]
fn g4_mir_gate_rechaza_residuos_genericos() {
    use basilisco::mir::{
        self, BlockId, LocalId, MirBasicBlock, MirFunction, MirOperand, MirPrograma, MirTerminator,
    };
    use basilisco::semantico::{TipoParamId, TipoSem};
    use basilisco::simbodo::SimbodoId;

    let tparam = TipoSem::Param(TipoParamId {
        propietario: SimbodoId(1),
        nombre: "T".to_string(),
    });
    let f = MirFunction {
        nombre: "main".to_string(),
        params: vec![(LocalId(0), tparam)],
        retorno: TipoSem::Entero,
        locals: vec![TipoSem::Entero],
        bloques: vec![MirBasicBlock {
            id: BlockId(0),
            stmts: vec![],
            terminator: MirTerminator::Return(Some(MirOperand::Const(mir::MirConst::Entero(0)))),
        }],
        bloque_entrada: BlockId(0),
    };
    let prog = MirPrograma {
        funciones: vec![f],
        globals: vec![],
    };
    let err = basilisco::mir::verificar_sin_residuos_genericos(&prog)
        .expect_err("debe rechazar residuo genérico en MIR");
    assert!(err.contains("E1055") || err.contains("BAS-7020"), "{err}");
}

#[test]
fn g4_hir_gate_rechaza_residuos_genericos() {
    use basilisco::hir::{HirExpresion, HirItem, HirLiteral, HirPrograma, HirSentencia};
    use basilisco::semantico::{TipoParamId, TipoSem};
    use basilisco::simbodo::SimbodoId;

    let tparam = TipoSem::Param(TipoParamId {
        propietario: SimbodoId(1),
        nombre: "T".to_string(),
    });
    let prog = HirPrograma {
        items: vec![HirItem::Funcion {
            nombre: "main".to_string(),
            simbodo: SimbodoId(0),
            params: vec![(SimbodoId(1), tparam)],
            capturadas: vec![],
            capturas_externas: vec![],
            celdas: vec![],
            retorno: TipoSem::Entero,
            cuerpo: vec![HirSentencia::Retornar(Some(HirExpresion::Literal(
                HirLiteral::Entero(0),
                TipoSem::Entero,
            )))],
        }],
    };
    let err = basilisco::hir::verificar_sin_residuos_genericos(&prog)
        .expect_err("debe rechazar residuo genérico en HIR");
    assert!(err.contains("E1055") || err.contains("BAS-7020"), "{err}");
}

#[test]
fn g4_cierre_transitivo_y_dependencias_repetidas() {
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion dup<T>(x: T) -> T {
    var a: T = id(x)
    var b: T = id(a)
    retornar id(b)
}
funcion main() -> entero { retornar dup::<entero>(7) }
"#;
    assert_tw_vm(src, Valor::Entero(7));
}

#[test]
fn g4_sustitucion_total_sin_plantillas_en_salida() {
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion main() -> entero { retornar id::<entero>(5) }
"#;
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(errs.is_empty(), "sem: {:?}", errs);
    let (out, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(errs_g4.is_empty(), "g4: {:?}", errs_g4);
    for s in &out {
        if let basilisco::Sentencia::Funcion {
            nombre,
            parametros_tipo,
            ..
        } = s
        {
            assert!(
                parametros_tipo.is_empty(),
                "función {nombre} no debe tener parámetros de tipo en salida G4"
            );
        }
    }
}

#[test]
fn g4_idempotencia_programa_concreto() {
    // Un programa ya concreto no debe ser alterado por un pase G4 adicional.
    let src = r#"
funcion suma(a: entero, b: entero) -> entero { retornar a + b }
funcion main() -> entero { retornar suma(1, 2) }
"#;
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(errs.is_empty(), "sem: {:?}", errs);
    let (out1, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(errs_g4.is_empty(), "g4: {:?}", errs_g4);
    let mut ctx2 = ContextoSemantico::nuevo();
    let errs2 = ctx2.analizar_programa(&out1);
    assert!(errs2.is_empty(), "sem2: {:?}", errs2);
    let (out2, errs_g4_2) = ctx2.monomorfizar_programa(&out1);
    assert!(errs_g4_2.is_empty(), "g4-2: {:?}", errs_g4_2);
    assert_eq!(out1.len(), out2.len(), "idempotencia estructural de tamaño");
    assert_tw_vm(src, Valor::Entero(3));
}

#[test]
fn g4_limite_aridad_excedido_m3() {
    // M3 provisional: aridad máxima de parámetros de tipo = 8.
    let src = r#"
funcion f<A,B,C,D,E,F,G,H,I>(a: A) -> A { retornar a }
funcion main() -> entero { retornar f::<entero,entero,entero,entero,entero,entero,entero,entero,entero>(1) }
"#;
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    // Si semántica acepta la declaración, G4 debe rechazar por aridad (M3).
    if !errs.is_empty() {
        let all = errs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            all.contains("aridad") || all.contains("E1052") || all.contains("BAS-7017"),
            "rechazo inesperado en semántica: {all}"
        );
        return;
    }
    let (_out, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(!errs_g4.is_empty(), "debe rechazar aridad > M3");
    let all = errs_g4
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(all.contains("aridad"), "debe mencionar aridad: {all}");
}

#[test]
fn g4_v3_post_especializacion_valida_caso_concreto() {
    // V3 post-especialización debe ejecutarse y aceptar instancias concretas válidas.
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion suma_uno(x: entero) -> entero { retornar x + 1 }
funcion aplicar<T>(x: T) -> T { retornar id(x) }
funcion main() -> entero { retornar suma_uno(aplicar::<entero>(41)) }
"#;
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(errs.is_empty(), "semántica debe aceptar: {:?}", errs);
    let (out, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(
        errs_g4.is_empty(),
        "G4/V3 post-especialización debe aceptar caso válido: {:?}",
        errs_g4
    );
    for s in &out {
        if let basilisco::Sentencia::Funcion {
            parametros_tipo,
            nombre,
            ..
        } = s
        {
            assert!(parametros_tipo.is_empty(), "plantilla residual: {nombre}");
        }
    }
    assert_tw_vm(src, Valor::Entero(42));
}

#[test]
fn g4_v3_post_especializacion_detecta_error_en_instancia_concreta() {
    // Caso inválido: la instancia concreta especializada debe ser validada por V3 en G4
    // y el fallo debe impedir publicación de salida especializada.
    // Se construye un caso donde el cuerpo de la plantilla es válido como genérico opaco
    // pero la instancia concreta requiere validación semántica de la aplicación.
    // Estrategia: usar deducción de llamada concreta a través de cadena que el análisis
    // V3 acotado no explora completamente; G4 valida cada instancia post-especialización.
    let src = r#"
funcion idem<T>(x: T) -> T { retornar x }
funcion f0<T>(x: T) -> T { retornar f1::<T>(x) }
funcion f1<T>(x: T) -> T { retornar f2::<T>(x) }
funcion f2<T>(x: T) -> T { retornar f3::<T>(x) }
funcion f3<T>(x: T) -> T { retornar f4::<T>(x) }
funcion f4<T>(x: T) -> T { retornar f5::<T>(x) }
funcion f5<T>(x: T) -> T { retornar f6::<T>(x) }
funcion f6<T>(x: T) -> T { retornar f7::<T>(x) }
funcion f7<T>(x: T) -> T { retornar f8::<T>(x) }
funcion f8<T>(x: T) -> T { retornar f9::<T>(x) }
funcion f9<T>(x: T) -> T { retornar idem::<T>(x) }
funcion main() -> entero { retornar f0::<entero>(7) }
"#;
    // Caso válido profundo: demuestra que V3 post-especialización se ejecuta por instancia
    // en cadena de dependencias (cada instancia validada antes de publicar en G4).
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(errs.is_empty(), "análisis semántico debe pasar: {:?}", errs);
    let (out, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(
        errs_g4.is_empty(),
        "G4/V3 post-especialización debe aceptar cadena válida: {:?}",
        errs_g4
    );
    assert!(!out.is_empty(), "debe producir salida especializada");
    assert_tw_vm(src, Valor::Entero(7));

    // Caso inválido: V3 post-especialización en G4 detecta incompatibilidad concreta
    // en el cuerpo de la instancia especializada e impide su publicación.
    let src_bad = r#"
funcion usar_entero(n: entero) -> entero { retornar n }
funcion f<T>(x: T) -> entero { retornar usar_entero(x) }
funcion main() -> entero { retornar f::<texto>("hola") }
"#;
    let ss_bad = parsear(src_bad);
    let mut ctx_bad = ContextoSemantico::nuevo();
    let _errs_sem = ctx_bad.analizar_programa(&ss_bad);
    let (out_bad, errs_g4_bad) = ctx_bad.monomorfizar_programa(&ss_bad);
    assert!(
        !errs_g4_bad.is_empty(),
        "G4 debe detectar error V3 post-especialización en instancia concreta"
    );
    let tiene_esp = out_bad.iter().any(|s| match s {
        basilisco::Sentencia::Funcion { nombre, .. } => nombre.contains("__g4__"),
        _ => false,
    });
    assert!(
        !tiene_esp,
        "no debe haber salida especializada parcial ante fallo V3 en G4: {out_bad:#?}"
    );
}

#[test]
fn g4_integridad_positiva_dependencias_compartidas_y_deduplicacion() {
    // Programa válido con dependencias compartidas y deduplicación: gate de integridad OK.
    let src = r#"
funcion id<T>(x: T) -> T { retornar x }
funcion wrap<T>(x: T) -> T { retornar id(id(x)) }
funcion combinar<T, U>(a: T, b: U) -> T { retornar id(a) }
funcion main() -> entero {
    retornar combinar::<entero, texto>(wrap::<entero>(id::<entero>(7)), id::<texto>("ok"))
}
"#;
    let ss = parsear(src);
    let mut ctx = ContextoSemantico::nuevo();
    let errs = ctx.analizar_programa(&ss);
    assert!(errs.is_empty(), "sem: {:?}", errs);
    let (out, errs_g4) = ctx.monomorfizar_programa(&ss);
    assert!(
        errs_g4.is_empty(),
        "G4/integridad debe aceptar: {:?}",
        errs_g4
    );
    for s in &out {
        if let basilisco::Sentencia::Funcion {
            nombre,
            parametros_tipo,
            ..
        } = s
        {
            assert!(
                parametros_tipo.is_empty(),
                "plantilla/params tipo residuales en {nombre}"
            );
        }
    }
    let mut nombres: Vec<String> = Vec::new();
    for s in &out {
        if let basilisco::Sentencia::Funcion { nombre, .. } = s {
            nombres.push(nombre.clone());
        }
    }
    let mut unicos = nombres.clone();
    unicos.sort();
    unicos.dedup();
    assert_eq!(
        nombres.len(),
        unicos.len(),
        "no debe haber definiciones duplicadas en salida G4"
    );
    assert_tw_vm(src, Valor::Entero(7));
}

mod integridad_gate_negativos {
    use basilisco::Valor;
    use basilisco::monomorfizacion::{
        EstadoInstancia, GenericFnId, InfoPlantilla, InstanceKey, MotorMonomorfizacion,
        RegistroInstancia, TipoCanon, nombre_especializado_base, verificar_integridad_salida_g4,
    };
    use basilisco::parser::{Expresion, Sentencia, TipoAnotacion};
    use basilisco::simbodo::SimbodoId;
    use std::collections::BTreeMap;

    fn gid(nombre: &str, id: u32, aridad: usize) -> GenericFnId {
        GenericFnId {
            modulo: "m".to_string(),
            decl_id: SimbodoId(id),
            nombre: nombre.to_string(),
            aridad,
        }
    }

    fn key(nombre: &str, id: u32, args: Vec<TipoCanon>) -> InstanceKey {
        InstanceKey {
            funcion: gid(nombre, id, args.len()),
            args,
        }
    }

    fn plantilla(nombre: &str, id: u32) -> InfoPlantilla {
        InfoPlantilla {
            id: gid(nombre, id, 1),
            nombre: nombre.to_string(),
            parametros_tipo: vec![(
                "T".to_string(),
                basilisco::semantico::TipoParamId {
                    propietario: SimbodoId(id),
                    nombre: "T".to_string(),
                },
            )],
            params_formales_sem: vec![],
            retorno_sem: None,
            parametros_ast: vec![],
            tipo_retorno_ast: None,
            cuerpo: vec![],
            linea: 1,
            columna: 1,
            publico: false,
        }
    }

    fn motor_con_instancia_completa(
        nombre: &str,
        id: u32,
        args: Vec<TipoCanon>,
    ) -> (
        MotorMonomorfizacion,
        InstanceKey,
        String,
        BTreeMap<GenericFnId, InfoPlantilla>,
    ) {
        let k = key(nombre, id, args);
        let esp = nombre_especializado_base(&k);
        let mut motor = MotorMonomorfizacion::nuevo(256, 32);
        motor.instancias.insert(
            k.clone(),
            RegistroInstancia {
                key: k.clone(),
                estado: EstadoInstancia::Complete,
                nombre_especializado: Some(esp.clone()),
                funcion: Some(Sentencia::Funcion {
                    nombre: esp.clone(),
                    parametros_tipo: vec![],
                    parametros: vec![],
                    tipo_retorno: Some(TipoAnotacion::Entero),
                    cuerpo: vec![Sentencia::Retorno {
                        valor: Some(Expresion::Literal {
                            valor: Valor::Entero(1),
                            linea: 1,
                            columna: 1,
                        }),
                        linea: 1,
                        columna: 1,
                    }],
                    linea: 1,
                    columna: 1,
                    publico: false,
                }),
                error: None,
                dependencias: vec![],
            },
        );
        let mut plantillas = BTreeMap::new();
        plantillas.insert(k.funcion.clone(), plantilla(nombre, id));
        (motor, k, esp, plantillas)
    }

    fn assert_gate_falla(
        programa: &[Sentencia],
        plantillas: &BTreeMap<GenericFnId, InfoPlantilla>,
        motor: &MotorMonomorfizacion,
        pista: &str,
    ) {
        let errs = verificar_integridad_salida_g4(programa, plantillas, motor, None);
        assert!(!errs.is_empty(), "el gate debe rechazar: {pista}");
    }

    #[test]
    fn gate_neg_plantilla_generica_residual() {
        let (motor, _k, esp, plantillas) =
            motor_con_instancia_completa("id", 1, vec![TipoCanon::Entero]);
        let programa = vec![
            Sentencia::Funcion {
                nombre: "id".to_string(),
                parametros_tipo: vec!["T".to_string()],
                parametros: vec![],
                tipo_retorno: None,
                cuerpo: vec![],
                linea: 1,
                columna: 1,
                publico: false,
            },
            Sentencia::Funcion {
                nombre: esp,
                parametros_tipo: vec![],
                parametros: vec![],
                tipo_retorno: Some(TipoAnotacion::Entero),
                cuerpo: vec![],
                linea: 1,
                columna: 1,
                publico: false,
            },
        ];
        assert_gate_falla(
            &programa,
            &plantillas,
            &motor,
            "plantilla generica residual",
        );
    }

    #[test]
    fn gate_neg_parametros_genericos_en_funciones_emitidas() {
        let (motor, _k, esp, plantillas) =
            motor_con_instancia_completa("id", 1, vec![TipoCanon::Entero]);
        let programa = vec![Sentencia::Funcion {
            nombre: esp,
            parametros_tipo: vec![],
            parametros: vec![(
                "x".to_string(),
                Some(TipoAnotacion::Parametro("T".to_string())),
            )],
            tipo_retorno: Some(TipoAnotacion::Entero),
            cuerpo: vec![],
            linea: 1,
            columna: 1,
            publico: false,
        }];
        assert_gate_falla(
            &programa,
            &plantillas,
            &motor,
            "parametros genericos residuales en emitidas",
        );
    }

    #[test]
    fn gate_neg_llamada_con_argumentos_tipo_residuales() {
        let (motor, _k, esp, plantillas) =
            motor_con_instancia_completa("id", 1, vec![TipoCanon::Entero]);
        let programa = vec![
            Sentencia::Funcion {
                nombre: "main".to_string(),
                parametros_tipo: vec![],
                parametros: vec![],
                tipo_retorno: Some(TipoAnotacion::Entero),
                cuerpo: vec![Sentencia::Expresion(Expresion::Llamada {
                    nombre: "id".to_string(),
                    argumentos: vec![Expresion::Literal {
                        valor: Valor::Entero(1),
                        linea: 1,
                        columna: 1,
                    }],
                    argumentos_tipo: vec![TipoAnotacion::Entero],
                    linea: 1,
                    columna: 1,
                })],
                linea: 1,
                columna: 1,
                publico: false,
            },
            Sentencia::Funcion {
                nombre: esp,
                parametros_tipo: vec![],
                parametros: vec![],
                tipo_retorno: Some(TipoAnotacion::Entero),
                cuerpo: vec![],
                linea: 1,
                columna: 1,
                publico: false,
            },
        ];
        assert_gate_falla(
            &programa,
            &plantillas,
            &motor,
            "argumentos de tipo residuales",
        );
    }

    #[test]
    fn gate_neg_destino_generico_no_resuelto() {
        let mut plantillas = BTreeMap::new();
        plantillas.insert(gid("id", 1, 1), plantilla("id", 1));
        let motor = MotorMonomorfizacion::nuevo(256, 32);
        let programa = vec![Sentencia::Funcion {
            nombre: "main".to_string(),
            parametros_tipo: vec![],
            parametros: vec![],
            tipo_retorno: Some(TipoAnotacion::Entero),
            cuerpo: vec![Sentencia::Expresion(Expresion::Llamada {
                nombre: "id".to_string(),
                argumentos: vec![],
                argumentos_tipo: vec![],
                linea: 1,
                columna: 1,
            })],
            linea: 1,
            columna: 1,
            publico: false,
        }];
        assert_gate_falla(
            &programa,
            &plantillas,
            &motor,
            "destino generico no resuelto",
        );
    }

    #[test]
    fn gate_neg_referencia_a_instancia_inexistente() {
        let (mut motor, k, esp, plantillas) =
            motor_con_instancia_completa("id", 1, vec![TipoCanon::Entero]);
        if let Some(reg) = motor.instancias.get_mut(&k) {
            reg.estado = EstadoInstancia::Failed;
            reg.funcion = None;
        }
        let programa = vec![Sentencia::Funcion {
            nombre: "main".to_string(),
            parametros_tipo: vec![],
            parametros: vec![],
            tipo_retorno: Some(TipoAnotacion::Entero),
            cuerpo: vec![Sentencia::Expresion(Expresion::Llamada {
                nombre: esp,
                argumentos: vec![],
                argumentos_tipo: vec![],
                linea: 1,
                columna: 1,
            })],
            linea: 1,
            columna: 1,
            publico: false,
        }];
        assert_gate_falla(
            &programa,
            &plantillas,
            &motor,
            "referencia a instancia inexistente",
        );
    }

    #[test]
    fn gate_neg_definiciones_especializadas_duplicadas() {
        let (motor, _k, esp, plantillas) =
            motor_con_instancia_completa("id", 1, vec![TipoCanon::Entero]);
        let f = Sentencia::Funcion {
            nombre: esp.clone(),
            parametros_tipo: vec![],
            parametros: vec![],
            tipo_retorno: Some(TipoAnotacion::Entero),
            cuerpo: vec![],
            linea: 1,
            columna: 1,
            publico: false,
        };
        let programa = vec![f.clone(), f];
        assert_gate_falla(
            &programa,
            &plantillas,
            &motor,
            "definiciones especializadas duplicadas",
        );
    }

    #[test]
    fn gate_neg_instancia_sin_definicion_ejecutable_unica() {
        let (motor, _k, _esp, plantillas) =
            motor_con_instancia_completa("id", 1, vec![TipoCanon::Entero]);
        let programa = vec![Sentencia::Funcion {
            nombre: "main".to_string(),
            parametros_tipo: vec![],
            parametros: vec![],
            tipo_retorno: Some(TipoAnotacion::Entero),
            cuerpo: vec![],
            linea: 1,
            columna: 1,
            publico: false,
        }];
        assert_gate_falla(
            &programa,
            &plantillas,
            &motor,
            "instancia sin definicion ejecutable unica",
        );
    }

    #[test]
    fn gate_neg_nombre_especializado_no_canonico() {
        let k = key("id", 1, vec![TipoCanon::Entero]);
        let mut motor = MotorMonomorfizacion::nuevo(256, 32);
        let nombre_malo = "__g4__id__no_canonico".to_string();
        motor.instancias.insert(
            k.clone(),
            RegistroInstancia {
                key: k.clone(),
                estado: EstadoInstancia::Complete,
                nombre_especializado: Some(nombre_malo.clone()),
                funcion: Some(Sentencia::Funcion {
                    nombre: nombre_malo.clone(),
                    parametros_tipo: vec![],
                    parametros: vec![],
                    tipo_retorno: Some(TipoAnotacion::Entero),
                    cuerpo: vec![],
                    linea: 1,
                    columna: 1,
                    publico: false,
                }),
                error: None,
                dependencias: vec![],
            },
        );
        let mut plantillas = BTreeMap::new();
        plantillas.insert(k.funcion.clone(), plantilla("id", 1));
        let programa = vec![Sentencia::Funcion {
            nombre: nombre_malo,
            parametros_tipo: vec![],
            parametros: vec![],
            tipo_retorno: Some(TipoAnotacion::Entero),
            cuerpo: vec![],
            linea: 1,
            columna: 1,
            publico: false,
        }];
        assert_gate_falla(
            &programa,
            &plantillas,
            &motor,
            "nombre especializado no canonico",
        );
    }
}
