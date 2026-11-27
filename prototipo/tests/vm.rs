use basilisco::{ContextoSemantico, Lexer, Parser, Valor, hir, mir, vm};

fn bajar(src: &str) -> mir::MirPrograma {
    let tokens = Lexer::nuevo(src).tokenizar().unwrap();
    let ss = Parser::nuevo(tokens).parsear_programa().unwrap();
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).unwrap();
    mir::lower_programa(&hir)
}

#[test]
fn vm_ejecuta_suma() {
    let src = "funcion suma(a: entero, b: entero) -> entero { retornar a + b }";
    let prog = bajar(src);
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    assert_eq!(maquina.numero_funciones(), 1);
    let r = maquina
        .ejecutar_indice(0, &[Valor::Entero(2), Valor::Entero(3)])
        .unwrap();
    assert_eq!(r, Some(Valor::Entero(5)));
}

#[test]
fn vm_mientras_cuenta() {
    let src = "funcion contar(n: entero) -> entero {\n  sea i = 0\n  mientras i < n {\n    i = i + 1\n  }\n  retornar i\n}";
    let prog = bajar(src);
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    let r = maquina.ejecutar_indice(0, &[Valor::Entero(5)]).unwrap();
    assert_eq!(r, Some(Valor::Entero(5)));
}

#[test]
fn vm_si_sino_abs() {
    let src =
        "funcion abs(x: entero) -> entero { si x < 0 { retornar 0 - x } sino { retornar x } }";
    let prog = bajar(src);
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    let negativo = maquina.ejecutar_indice(0, &[Valor::Entero(-3)]).unwrap();
    assert_eq!(negativo, Some(Valor::Entero(3)));
    let positivo = maquina.ejecutar_indice(0, &[Valor::Entero(7)]).unwrap();
    assert_eq!(positivo, Some(Valor::Entero(7)));
}

#[test]
fn vm_recursion_factorial() {
    let src = "funcion factorial(n: entero) -> entero { si n < 2 { retornar 1 } sino { retornar factorial(n - 1) * n } }";
    let prog = bajar(src);
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    let r = maquina.ejecutar_indice(0, &[Valor::Entero(5)]).unwrap();
    assert_eq!(r, Some(Valor::Entero(120)));
}

#[test]
fn vm_limite_pasos_detecta_bucle_infinito() {
    let src = "funcion espiral(inicio: entero) -> entero {\n  mientras verdadero {\n    inicio = inicio + 1\n  }\n  retornar inicio\n}";
    let prog = bajar(src);
    let mut maquina = vm::MaquinaVirtual::con_limites(prog, 100, 100);
    let err = maquina.ejecutar_indice(0, &[Valor::Entero(0)]).unwrap_err();
    assert_eq!(err, vm::ErrorVm::LimitePasosExcedido);
}

#[test]
fn vm_funcion_inexistente_falla() {
    let prog = bajar("funcion suma(a: entero, b: entero) -> entero { retornar a + b }");
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    let err = maquina.ejecutar_indice(9, &[Valor::Entero(1)]).unwrap_err();
    assert_eq!(err, vm::ErrorVm::BloqueInalcanzable);
}

#[test]
fn vm_argumentos_insuficientes_falla() {
    let prog = bajar("funcion suma(a: entero, b: entero) -> entero { retornar a + b }");
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    let err = maquina.ejecutar_indice(0, &[Valor::Entero(1)]).unwrap_err();
    assert_eq!(err, vm::ErrorVm::ArgumentosInsuficientes { faltantes: 1 });
}

#[test]
fn vm_llama_nativo_registrado() {
    let src = "funcion main() -> entero {\n  retornar duplicar(21)\n}";
    let prog = bajar(src);
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    maquina.registrar_nativo(
        "duplicar",
        std::sync::Arc::new(|args: &[Valor]| {
            let Valor::Entero(x) = args.first().cloned().unwrap_or(Valor::Nulo) else {
                return Err(vm::ErrorVm::TiposIncompatibles {
                    izquierdo: "entero",
                    derecho: "otro",
                });
            };
            Ok(Valor::Entero(x * 2))
        }),
    );
    let r = maquina.ejecutar("main", &[]).unwrap();
    assert_eq!(r, Some(Valor::Entero(42)));
}

#[test]
fn vm_nativo_texto_y_error() {
    let src = "funcion main() -> texto {\n  retornar eco(\"hola\")\n}";
    let prog = bajar(src);
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    maquina.registrar_nativo(
        "eco",
        std::sync::Arc::new(|args: &[Valor]| {
            let Valor::Texto(s) = args.first().cloned().unwrap_or(Valor::Nulo) else {
                return Err(vm::ErrorVm::TipoEsperado {
                    esperado: "texto",
                    encontrado: "otro",
                });
            };
            Ok(Valor::Texto(format!("eco: {s}")))
        }),
    );
    let r = maquina.ejecutar("main", &[]).unwrap();
    assert_eq!(r, Some(Valor::Texto("eco: hola".to_string())));
}

#[test]
fn vm_nativo_propaga_error() {
    let src = "funcion main() -> entero {\n  retornar explota(0)\n}";
    let prog = bajar(src);
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    maquina.registrar_nativo(
        "explota",
        std::sync::Arc::new(|_| Err(vm::ErrorVm::LimitePasosExcedido)),
    );
    let err = maquina.ejecutar("main", &[]).unwrap_err();
    assert_eq!(err, vm::ErrorVm::LimitePasosExcedido);
}

#[test]
fn vm_funcion_mir_tiene_prioridad_sobre_nativo() {
    let src = "funcion doble(n: entero) -> entero { retornar n + n }";
    let prog = bajar(src);
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    maquina.registrar_nativo(
        "doble",
        std::sync::Arc::new(|args: &[Valor]| {
            let Valor::Entero(x) = args.first().cloned().unwrap_or(Valor::Nulo) else {
                return Err(vm::ErrorVm::TiposIncompatibles {
                    izquierdo: "entero",
                    derecho: "otro",
                });
            };
            Ok(Valor::Entero(x * 1000))
        }),
    );
    let r = maquina.ejecutar("doble", &[Valor::Entero(3)]).unwrap();
    assert_eq!(r, Some(Valor::Entero(6)));
}

#[test]
fn vm_rechaza_escape_celda_en_retorno() {
    use basilisco::mir::{
        BlockId, LocalId, MirBasicBlock, MirFunction, MirOperand, MirPrograma, MirRvalue, MirStmt,
        MirTerminator,
    };
    let prog2 = MirPrograma {
        funciones: vec![MirFunction {
            nombre: "main".into(),
            params: vec![],
            retorno: basilisco::semantico::TipoSem::Desconocido,
            locals: vec![
                basilisco::semantico::TipoSem::Desconocido,
                basilisco::semantico::TipoSem::Desconocido,
            ],
            bloques: vec![MirBasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Asignacion {
                    destino: basilisco::mir::MirPlace {
                        local: LocalId(0),
                        projection: vec![],
                    },
                    valor: MirRvalue::CrearCelda {
                        valor: MirOperand::Const(basilisco::mir::MirConst::Entero(7)),
                    },
                }],
                terminator: MirTerminator::Return(Some(MirOperand::Copy(LocalId(0)))),

                stmt_spans: Vec::new(),
                terminator_span: None,
            }],
            bloque_entrada: BlockId(0),
        }],
        globals: vec![],
        contiene_coincidir: false,
        contiene_nominales: false,

        source_map: std::collections::HashMap::new(),
    };
    let mut maquina = vm::MaquinaVirtual::nuevo(prog2);
    let err = maquina.ejecutar("main", &[]).unwrap_err();
    match err {
        vm::ErrorVm::NoSoportado(m) => assert!(m.contains("celda interna"), "{m}"),
        otro => panic!("esperaba NoSoportado celda interna, got {otro:?}"),
    }
}

#[test]
fn vm_rechaza_escape_celda_en_global() {
    use basilisco::mir::{
        BlockId, GlobalId, LocalId, MirBasicBlock, MirFunction, MirOperand, MirPrograma, MirRvalue,
        MirStmt, MirTerminator,
    };
    let prog = MirPrograma {
        funciones: vec![MirFunction {
            nombre: "main".into(),
            params: vec![],
            retorno: basilisco::semantico::TipoSem::Desconocido,
            locals: vec![basilisco::semantico::TipoSem::Desconocido],
            bloques: vec![MirBasicBlock {
                id: BlockId(0),
                stmts: vec![
                    MirStmt::Asignacion {
                        destino: basilisco::mir::MirPlace {
                            local: LocalId(0),
                            projection: vec![],
                        },
                        valor: MirRvalue::CrearCelda {
                            valor: MirOperand::Const(basilisco::mir::MirConst::Entero(1)),
                        },
                    },
                    MirStmt::AsignacionGlobal {
                        global: GlobalId(0),
                        valor: MirRvalue::Use(MirOperand::Copy(LocalId(0))),
                    },
                ],
                terminator: MirTerminator::Return(Some(MirOperand::Const(
                    basilisco::mir::MirConst::Entero(0),
                ))),

                stmt_spans: Vec::new(),
                terminator_span: None,
            }],
            bloque_entrada: BlockId(0),
        }],
        globals: vec![basilisco::semantico::TipoSem::Desconocido],
        contiene_coincidir: false,
        contiene_nominales: false,

        source_map: std::collections::HashMap::new(),
    };
    let mut maquina = vm::MaquinaVirtual::nuevo(prog);
    let err = maquina.ejecutar("main", &[]).unwrap_err();
    match err {
        vm::ErrorVm::NoSoportado(m) => assert!(m.contains("celda interna"), "{m}"),
        otro => panic!("esperaba NoSoportado celda interna, got {otro:?}"),
    }
}
