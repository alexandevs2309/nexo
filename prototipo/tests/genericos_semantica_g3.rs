use std::collections::{HashMap, HashSet};

use basilisco::semantico::{
    TipoParamId, TipoSem, UnificacionError, contiene_param_de, es_tipo_concreto_resoluble,
    inferir_sigma, sustituir_tipo,
};
use basilisco::simbodo::SimbodoId;
use basilisco::{ContextoSemantico, Lexer, Parser};

fn analizar(fuente: &str) -> Vec<basilisco::ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico");
    let ss = Parser::nuevo(tokens).parsear_programa().expect("sintaxis");
    let mut ctx = ContextoSemantico::nuevo();
    ctx.analizar_programa(&ss)
}

fn ok(fuente: &str) {
    let e = analizar(fuente);
    assert!(
        e.is_empty(),
        "se esperaba análisis sin errores.\nfuente: {fuente}\nerrores: {e:?}"
    );
}

fn err_codigos(fuente: &str, esperado: &str) {
    let e = analizar(fuente);
    assert!(
        !e.is_empty(),
        "se esperaba error semántico.\nfuente: {fuente}"
    );
    let codigos: Vec<&str> = e.iter().map(|x| x.causa.codigo()).collect();
    assert!(
        codigos.contains(&esperado),
        "código inesperado.\nfuente: {fuente}\nesperado: {esperado}\nreal: {codigos:?}\nerrores: {e:?}"
    );
}

// ===== V1: errores independientes de sustitución =====

#[test]
fn v1_aridad_incorrecta_argumentos_tipo_explicitos() {
    err_codigos(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion main() {\n  id::<entero, texto>(1)\n}\nmain()",
        "E1043",
    );
}

#[test]
fn v1_argumentos_tipo_en_funcion_no_generica() {
    err_codigos(
        "funcion f(x: entero) -> entero {\n  retornar x\n}\nfuncion main() {\n  f::<entero>(1)\n}\nmain()",
        "E1042",
    );
}

#[test]
fn v1_parametro_tipo_no_declarado_en_anotacion() {
    // S8.1: T no declarado se parsea como Nominal y se rechaza en semántica (BAS-7101).
    let src = "funcion f(x: T) -> T {\n  retornar x\n}\n";
    let tokens = Lexer::nuevo(src).tokenizar().expect("léxico");
    let ss = Parser::nuevo(tokens).parsear_programa().expect("sintaxis");
    let mut ctx = ContextoSemantico::nuevo();
    let e = ctx.analizar_programa(&ss);
    assert!(
        e.iter().any(|x| x.causa.codigo() == "BAS-7101"),
        "T no declarado debe rechazarse en la plantilla: {:?}",
        e.iter().map(|x| x.causa.codigo()).collect::<Vec<_>>()
    );
}

#[test]
fn v1_plantilla_coherente_declara_y_usa_param() {
    ok("funcion id<T>(x: T) -> T {\n  retornar x\n}\n");
}

#[test]
fn v1_turbofish_en_builtin_rechazado() {
    err_codigos(
        "funcion main() {\n  longitud::<entero>([1, 2, 3])\n}\nmain()",
        "E1042",
    );
}

// ===== V2: opacidad estricta de parámetros de tipo =====

#[test]
fn v2_aritmetica_sobre_t_prohibida() {
    err_codigos("funcion f<T>(x: T) -> T {\n  retornar x + x\n}\n", "E1048");
}

#[test]
fn v2_resta_multiplicacion_division_modulo_sobre_t_prohibidas() {
    err_codigos("funcion f<T>(x: T) -> T {\n  retornar x - x\n}\n", "E1048");
    err_codigos("funcion f<T>(x: T) -> T {\n  retornar x * x\n}\n", "E1048");
    err_codigos("funcion f<T>(x: T) -> T {\n  retornar x / x\n}\n", "E1048");
    err_codigos("funcion f<T>(x: T) -> T {\n  retornar x % x\n}\n", "E1048");
}

#[test]
fn v2_comparacion_orden_sobre_t_prohibida() {
    err_codigos(
        "funcion f<T>(a: T, b: T) -> logico {\n  retornar a < b\n}\n",
        "E1048",
    );
    err_codigos(
        "funcion f<T>(a: T, b: T) -> logico {\n  retornar a >= b\n}\n",
        "E1048",
    );
}

#[test]
fn v2_igualdad_runtime_sobre_t_prohibida() {
    err_codigos(
        "funcion f<T>(a: T, b: T) -> logico {\n  retornar a == b\n}\n",
        "E1048",
    );
    err_codigos(
        "funcion f<T>(a: T, b: T) -> logico {\n  retornar a != b\n}\n",
        "E1048",
    );
}

#[test]
fn v2_indexacion_directa_de_t_prohibida() {
    err_codigos("funcion f<T>(x: T) -> T {\n  retornar x[0]\n}\n", "E1048");
}

#[test]
fn v2_iteracion_directa_de_t_prohibida() {
    err_codigos(
        "funcion f<T>(x: T) {\n  para cada e en x {\n    e\n  }\n}\n",
        "E1048",
    );
}

#[test]
fn v2_acceso_miembro_de_t_prohibido() {
    err_codigos("funcion f<T>(x: T) {\n  x.campo\n}\n", "E1048");
}

#[test]
fn v2_invocacion_de_t_prohibida() {
    err_codigos("funcion f<T>(x: T) {\n  x(1)\n}\n", "E1048");
}

#[test]
fn v2_uso_logico_de_t_prohibido() {
    err_codigos(
        "funcion f<T>(x: T) -> logico {\n  retornar no x\n}\n",
        "E1048",
    );
    err_codigos(
        "funcion f<T>(a: T, b: T) -> logico {\n  retornar a y b\n}\n",
        "E1048",
    );
    err_codigos(
        "funcion f<T>(x: T) {\n  si x {\n    retornar x\n  }\n}\n",
        "E1048",
    );
    err_codigos(
        "funcion f<T>(x: T) {\n  mientras x {\n    retornar x\n  }\n}\n",
        "E1048",
    );
}

#[test]
fn v2_operaciones_universales_u_permitidas() {
    // Pasar, retornar, asignar/enlazar T cuando la identidad es válida.
    ok(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion pasar<T>(x: T) -> T {\n  var v: T = x\n  retornar v\n}\n",
    );
}

#[test]
fn v2_contenedor_conocido_elemento_opaco() {
    // lista<T>/arreglo<T,N> son contenedores conocidos: indexación/iteración del contenedor permitidas.
    // El elemento T sigue siendo opaco (no se puede hacer aritmética sobre él).
    ok("funcion primero<T>(xs: lista<T>) -> T {\n  retornar xs[0]\n}\n");
    ok(
        "funcion recorrer<T>(xs: lista<T>) -> T {\n  var r: T = xs[0]\n  para cada e en xs {\n    r = e\n  }\n  retornar r\n}\n",
    );
    err_codigos("funcion malo<T>(xs: lista<T>) {\n  xs[0] + 1\n}\n", "E1048");
}

// ===== Inferencia local =====

#[test]
fn inf_directa_entero() {
    ok("funcion id<T>(x: T) -> T {\n  retornar x\n}\nid(1)\n");
}

#[test]
fn inf_directa_texto() {
    ok("funcion id<T>(x: T) -> T {\n  retornar x\n}\nid(\"hola\")\n");
}

#[test]
fn inf_multiples_parametros() {
    ok("funcion par<A, B>(a: A, b: B) -> A {\n  retornar a\n}\npar(1, \"x\")\n");
}

#[test]
fn inf_incompleta_sin_argumentos_ordinarios() {
    // Sin información para inferir T: incompleta (no inferencia desde retorno).
    err_codigos(
        "funcion crear<T>() -> lista<T> {\n  retornar [ ]\n}\ncrear()\n",
        "E1045",
    );
}

#[test]
fn inf_conflicto_entre_argumentos() {
    err_codigos(
        "funcion mismo<T>(a: T, b: T) -> T {\n  retornar a\n}\nmismo(1, \"x\")\n",
        "E1047",
    );
}

#[test]
fn inf_desconocido_no_concretable() {
    // Argumento con tipo no concretable impide inferencia completa.
    err_codigos(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion main() {\n  id(no_existe_var)\n}\nmain()",
        "E1045",
    );
}

#[test]
fn inf_ausencia_inferencia_desde_retorno() {
    // Sin argumentos ordinarios ni explícitos, T no es inferible; el contexto de retorno
    // esperado no debe usarse como fuente de inferencia.
    let e = analizar(
        "funcion cont<T>() -> lista<T> {\n  retornar [ ]\n}\nvar xs: lista<entero> = cont()\n",
    );
    assert!(
        e.iter().any(|x| matches!(
            x.causa.codigo(),
            "E1041" | "E1045" | "E1046" | "E1047" | "E1050"
        )),
        "se esperaba rechazo por imposibilidad de inferir sin retorno: {e:?}"
    );
    err_codigos(
        "funcion cont<T>() -> lista<T> {\n  retornar [ ]\n}\ncont()\n",
        "E1045",
    );
}

#[test]
fn inf_estructural_lista() {
    ok("funcion primero<T>(xs: lista<T>) -> T {\n  retornar xs[0]\n}\nprimero([1, 2, 3])\n");
    ok("funcion primero<T>(xs: lista<T>) -> T {\n  retornar xs[0]\n}\nprimero([\"a\", \"b\"])\n");
}

#[test]
fn inf_estructural_arreglo() {
    ok("funcion primero<T>(xs: arreglo<T, 2>) -> T {\n  retornar xs[0]\n}\nprimero([1, 2])\n");
}

// ===== Sustitución estructural (propiedades) =====

fn id(owner: u32, name: &str) -> TipoParamId {
    TipoParamId {
        propietario: SimbodoId(owner),
        nombre: name.to_string(),
    }
}

#[test]
fn sust_param_simple() {
    let t = TipoSem::Param(id(1, "T"));
    let mut s = HashMap::new();
    s.insert(id(1, "T"), TipoSem::Entero);
    assert_eq!(sustituir_tipo(&t, &s), TipoSem::Entero);
}

#[test]
fn sust_recursiva_lista() {
    let t = TipoSem::Lista(Box::new(TipoSem::Param(id(1, "T"))));
    let mut s = HashMap::new();
    s.insert(id(1, "T"), TipoSem::Texto);
    assert_eq!(
        sustituir_tipo(&t, &s),
        TipoSem::Lista(Box::new(TipoSem::Texto))
    );
}

#[test]
fn sust_recursiva_arreglo() {
    let t = TipoSem::Arreglo(Box::new(TipoSem::Param(id(1, "T"))), 3);
    let mut s = HashMap::new();
    s.insert(id(1, "T"), TipoSem::Entero);
    assert_eq!(
        sustituir_tipo(&t, &s),
        TipoSem::Arreglo(Box::new(TipoSem::Entero), 3)
    );
}

#[test]
fn sust_en_funcion_y_compuestos() {
    let t = TipoSem::Funcion {
        params: vec![
            TipoSem::Param(id(1, "T")),
            TipoSem::Lista(Box::new(TipoSem::Param(id(1, "U")))),
        ],
        retorno: Box::new(TipoSem::Arreglo(Box::new(TipoSem::Param(id(1, "T"))), 2)),
        genericos: vec![id(1, "T"), id(1, "U")],
    };
    let mut s = HashMap::new();
    s.insert(id(1, "T"), TipoSem::Entero);
    s.insert(id(1, "U"), TipoSem::Texto);
    let r = sustituir_tipo(&t, &s);
    assert_eq!(
        r,
        TipoSem::Funcion {
            params: vec![TipoSem::Entero, TipoSem::Lista(Box::new(TipoSem::Texto))],
            retorno: Box::new(TipoSem::Arreglo(Box::new(TipoSem::Entero), 2)),
            genericos: vec![id(1, "T"), id(1, "U")],
        }
    );
}

#[test]
fn sust_idempotente() {
    let t = TipoSem::Funcion {
        params: vec![TipoSem::Lista(Box::new(TipoSem::Param(id(1, "T"))))],
        retorno: Box::new(TipoSem::Param(id(1, "T"))),
        genericos: vec![id(1, "T")],
    };
    let mut s = HashMap::new();
    s.insert(id(1, "T"), TipoSem::Entero);
    let once = sustituir_tipo(&t, &s);
    let twice = sustituir_tipo(&once, &s);
    assert_eq!(once, twice);
}

#[test]
fn sust_completa_sin_param_residual_objetivo() {
    let t = TipoSem::Lista(Box::new(TipoSem::Param(id(1, "T"))));
    let mut s = HashMap::new();
    s.insert(id(1, "T"), TipoSem::Decimal);
    let r = sustituir_tipo(&t, &s);
    let objetivo: HashSet<TipoParamId> = [id(1, "T")].into_iter().collect();
    assert!(!contiene_param_de(&r, &objetivo));
}

#[test]
fn sust_primitivos_nulo_inalterados() {
    let s = HashMap::new();
    for t in [
        TipoSem::Entero,
        TipoSem::Decimal,
        TipoSem::Real,
        TipoSem::Logico,
        TipoSem::Texto,
        TipoSem::Nulo,
    ] {
        assert_eq!(sustituir_tipo(&t, &s), t);
    }
}

// ===== V3: validación dependiente del tipo concreto =====

#[test]
fn v3_argumentos_compatibles_explicitos() {
    ok(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion main() {\n  id::<entero>(1)\n  id::<texto>(\"a\")\n}\nmain()",
    );
}

#[test]
fn v3_argumentos_incompatibles_explicitos_e1049() {
    // G2.2.1 observación MEDIO: incompatibilidad ordinaria diferida ahora detectada en G3.
    err_codigos(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nid::<entero>(\"hola\")\n",
        "E1049",
    );
}

#[test]
fn v3_retorno_concreto_valido() {
    ok(
        "funcion suma<T>(x: T) -> T {\n  retornar x\n}\nfuncion main() {\n  var r: entero = suma::<entero>(1)\n  r\n}\nmain()",
    );
}

#[test]
fn v3_operaciones_concretas_validas_en_cuerpo_sustituido() {
    // V3: vista sustituida valida compatibilidad/contenedores concretos (plantilla V2-válida).
    ok(
        "funcion cabeza<T>(xs: lista<T>) -> T {\n  retornar xs[0]\n}\nfuncion main() {\n  cabeza::<entero>([1, 2, 3])\n  cabeza::<texto>([\"a\", \"b\"])\n}\nmain()",
    );
}

#[test]
fn v3_operaciones_concretas_invalidas_en_cuerpo() {
    // V3: con T=texto, suma aritmética texto+1 no es válida como operación numérica concreta.
    let e = analizar("funcion inc<T>(x: T) -> T {\n  retornar x + 1\n}\ninc::<texto>(\"a\")\n");
    assert!(
        !e.is_empty(),
        "aplicación concreta inválida debe diagnosticarse: {e:?}"
    );
}

#[test]
fn v3_inferencia_y_validacion_concreta_compatibles() {
    // Inferencia G3 válida (mismo tipo) y V3 compatible.
    ok("funcion mismo<T>(a: T, b: T) -> T {\n  retornar a\n}\nmismo(1, 2)\n");
}

// ===== Forwarding genérico y dependencias =====

#[test]
fn forwarding_directo_explicito() {
    ok(
        "funcion primero<T>(valor: T) -> T {\n  retornar valor\n}\nfuncion intermediaria<T>(valor: T) -> T {\n  retornar primero::<T>(valor)\n}\nfuncion main() {\n  intermediaria::<entero>(7)\n}\nmain()",
    );
}

#[test]
fn forwarding_multiples_parametros() {
    ok(
        "funcion segundo<A, B>(a: A, b: B) -> B {\n  retornar b\n}\nfuncion envuelve<A, B>(a: A, b: B) -> B {\n  retornar segundo::<A, B>(a, b)\n}\nfuncion main() {\n  envuelve::<entero, texto>(1, \"z\")\n}\nmain()",
    );
}

#[test]
fn forwarding_inferido_en_cuerpo() {
    // Inferencia local en llamada interna con propagación paramétrica.
    ok(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion wrap<T>(x: T) -> T {\n  retornar id(x)\n}\nwrap::<entero>(5)\n",
    );
}

#[test]
fn forwarding_dependencias_transitivas_sin_instancias() {
    ok(
        "funcion a<T>(x: T) -> T {\n  retornar x\n}\nfuncion b<T>(x: T) -> T {\n  retornar a(x)\n}\nfuncion c<T>(x: T) -> T {\n  retornar b(x)\n}\nc::<entero>(1)\n",
    );
}

#[test]
fn forwarding_argumento_tipo_invalido_fuera_contexto_e1044() {
    // G2.2.1 observación MEDIO / corrección post-auditoría G3 (M3):
    // BAS-7009/E1044 debe alcanzarse por la ruta semántica real (no solo rechazo frontend).
    // Construimos AST con un argumento de tipo explícito `T` usado en un contexto donde `T`
    // no pertenece al ámbito de parámetros de tipo de la función contenedora (`g<U>`).
    // El parser rechaza este caso en fuente (consistente con V1), pero la condición semántica
    // ("no resoluble o con identidad de parámetro fuera de contexto válido") es diagnosticable
    // por el analizador y se ejerce aquí de forma determinista.
    use basilisco::Valor;
    use basilisco::parser::{Expresion, Sentencia, TipoAnotacion};

    let prog = vec![
        Sentencia::Funcion {
            nombre: "id".to_string(),
            parametros_tipo: vec!["T".to_string()],
            parametros: vec![(
                "x".to_string(),
                Some(TipoAnotacion::Parametro("T".to_string())),
            )],
            tipo_retorno: Some(TipoAnotacion::Parametro("T".to_string())),
            cuerpo: vec![Sentencia::Retorno {
                valor: Some(Expresion::Variable {
                    nombre: "x".to_string(),
                    linea: 2,
                    columna: 12,
                }),
                linea: 2,
                columna: 3,
            }],
            linea: 1,
            columna: 1,
            publico: false,
        },
        Sentencia::Funcion {
            nombre: "g".to_string(),
            parametros_tipo: vec!["U".to_string()],
            parametros: vec![(
                "v".to_string(),
                Some(TipoAnotacion::Parametro("U".to_string())),
            )],
            tipo_retorno: Some(TipoAnotacion::Parametro("U".to_string())),
            cuerpo: vec![Sentencia::Retorno {
                valor: Some(Expresion::Llamada {
                    nombre: "id".to_string(),
                    argumentos: vec![Expresion::Variable {
                        nombre: "v".to_string(),
                        linea: 5,
                        columna: 22,
                    }],
                    argumentos_tipo: vec![TipoAnotacion::Parametro("T".to_string())],
                    linea: 5,
                    columna: 12,
                }),
                linea: 5,
                columna: 3,
            }],
            linea: 4,
            columna: 1,
            publico: false,
        },
        // Uso no genérico para forzar análisis del cuerpo de g en contexto no-V3 de g.
        Sentencia::Expresion(Expresion::Llamada {
            nombre: "g".to_string(),
            argumentos: vec![Expresion::Literal {
                valor: Valor::Entero(1),
                linea: 7,
                columna: 3,
            }],
            argumentos_tipo: vec![],
            linea: 7,
            columna: 1,
        }),
    ];

    let mut ctx = ContextoSemantico::nuevo();
    let e = ctx.analizar_programa(&prog);
    assert!(
        !e.is_empty(),
        "debe rechazarse argumento de tipo fuera de contexto (ruta semántica)"
    );
    let codigos: Vec<&str> = e.iter().map(|x| x.causa.codigo()).collect();
    assert!(
        codigos.contains(&"E1044"),
        "se esperaba E1044 por ruta semántica, real: {codigos:?}, errores: {e:?}"
    );
}

#[test]
fn forwarding_argumento_tipo_fuera_contexto_rechazo_frontend_trazable() {
    // S8.1: U fuera de contexto se parsea como nominal/tipo y se rechaza en semántica (BAS-7101 o E1046).
    let src = "funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion g(x: entero) -> entero {\n  retornar id::<U>(x)\n}\n";
    let tokens = Lexer::nuevo(src).tokenizar().expect("léxico");
    let ss = Parser::nuevo(tokens).parsear_programa().expect("sintaxis");
    let mut ctx = ContextoSemantico::nuevo();
    let e = ctx.analizar_programa(&ss);
    assert!(
        e.iter()
            .any(|x| matches!(x.causa.codigo(), "BAS-7101" | "E1046" | "E1049")),
        "el frontend debe rechazar parámetros de tipo fuera de contexto: {:?}",
        e.iter().map(|x| x.causa.codigo()).collect::<Vec<_>>()
    );
}

#[test]
fn forwarding_aridad_incorrecta_en_llamada_interna() {
    err_codigos(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion g<U>(x: U) -> U {\n  retornar id::<U, U>(x)\n}\n",
        "E1043",
    );
}

// ===== Propiedades de inferencia/unificación (API) =====

#[test]
fn api_inferir_sigma_directa_y_completa() {
    let tp = vec![id(1, "T")];
    let formales = vec![TipoSem::Param(id(1, "T"))];
    let obs = vec![TipoSem::Entero];
    let sigma = inferir_sigma(&formales, &obs, &tp).expect("sigma");
    assert_eq!(sigma.get(&id(1, "T")), Some(&TipoSem::Entero));
}

#[test]
fn api_inferir_sigma_conflicto() {
    let tp = vec![id(1, "T")];
    let formales = vec![TipoSem::Param(id(1, "T")), TipoSem::Param(id(1, "T"))];
    let obs = vec![TipoSem::Entero, TipoSem::Texto];
    let r = inferir_sigma(&formales, &obs, &tp);
    assert_eq!(r, Err(UnificacionError::Conflicto));
}

#[test]
fn api_inferir_sigma_incompleta() {
    let tp = vec![id(1, "T")];
    let formales = vec![TipoSem::Entero];
    let obs = vec![TipoSem::Entero];
    let r = inferir_sigma(&formales, &obs, &tp);
    assert_eq!(r, Err(UnificacionError::Incompleta));
}

#[test]
fn api_inferir_sigma_no_ambigua_bajo_unificacion_igualdad_local() {
    // A1 (post-auditoría G3): BAS-7011/E1046 requiere "inferencia ambigua (múltiples soluciones)".
    // Bajo inferencia local por unificación estructural de igualdad determinista, las restricciones
    // consistentes y completas producen σ única; restricciones incompatibles producen conflicto;
    // restricciones insuficientes producen incompletitud. No se identifica caso legítimo de múltiples
    // soluciones válidas distintas en este modelo, por lo que Ambigua no es emitible en G3.
    // Propuesta de reclasificación: mantener BAS-7011/E1046 reservado (catálogo/cableado) como no
    // activable en G3 mientras no exista un mecanismo normativo que admita múltiples σ válidas.

    // Mismo parámetro, mismas observaciones -> unificación consistente (no ambigua).
    let tp = vec![id(1, "T")];
    let formales = vec![TipoSem::Param(id(1, "T")), TipoSem::Param(id(1, "T"))];
    let obs = vec![TipoSem::Entero, TipoSem::Entero];
    assert_eq!(
        inferir_sigma(&formales, &obs, &tp),
        Ok(HashMap::from([(id(1, "T"), TipoSem::Entero)]))
    );

    // Mismo parámetro, observaciones distintas -> contradicción (conflicto), no múltiples soluciones.
    let formales = vec![TipoSem::Param(id(1, "T")), TipoSem::Param(id(1, "T"))];
    let obs = vec![TipoSem::Entero, TipoSem::Texto];
    assert_eq!(
        inferir_sigma(&formales, &obs, &tp),
        Err(UnificacionError::Conflicto)
    );

    // Parámetro sin restricción efectiva -> incompleta.
    let tp = vec![id(1, "T"), id(1, "U")];
    let formales = vec![TipoSem::Param(id(1, "T")), TipoSem::Entero];
    let obs = vec![TipoSem::Entero, TipoSem::Entero];
    assert_eq!(
        inferir_sigma(&formales, &obs, &tp),
        Err(UnificacionError::Incompleta)
    );
}

#[test]
fn api_es_tipo_concreto_resoluble() {
    assert!(es_tipo_concreto_resoluble(&TipoSem::Entero));
    assert!(!es_tipo_concreto_resoluble(&TipoSem::Desconocido));
}

// ===== Preservación: sin G4 / sin instancias ejecutables =====
// (Las pruebas anteriores ejercitan solo resolución semántica: sin SpecializedName,
// InstanceKey ejecutable, worklist, monomorfización, clonado especializado, HIR/MIR/VM/gate/codegen).

#[test]
fn inf_incompleta_parametro_no_inferido_en_multiples() {
    // Múltiples parámetros de tipo: U no aparece en posiciones inferibles desde args ordinarios.
    err_codigos(
        "funcion f<T, U>(x: T) -> T {\n  retornar x\n}\nf(1)\n",
        "E1045",
    );
}

#[test]
fn v3_conversiones_permitidas_concreto() {
    // Conversiones/compatibilidades numéricas permitidas existentes se preservan en vista concreta.
    ok(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nfuncion main() {\n  var r: decimal = id::<entero>(1)\n  r\n}\nmain()",
    );
}

#[test]
fn v3_no_contamina_c2_resultados_analisis_principal() {
    // M1 (post-auditoría G3): la vista semántica sustituida V3 es auxiliar y no debe
    // contaminar el estado de inferencia C.2 (`c2_resultados`) del análisis principal.
    // Se ejercitan aplicaciones concretas con cuerpo que incluye declaración anidada
    // (sin anotación de retorno, susceptible de análisis C.2 en su contexto propio).
    ok(
        "funcion id<T>(x: T) -> T {\n  funcion ayuda(z: entero) {\n    retornar z\n  }\n  retornar x\n}\nid::<entero>(1)\nid::<texto>(\"ok\")\n",
    );
    // Reentrada/reuso: múltiples aplicaciones V3 en el mismo análisis sin efectos residuales visibles.
    ok(
        "funcion par<A, B>(a: A, b: B) -> A {\n  funcion interna(z: entero) {\n    retornar z\n  }\n  retornar a\n}\npar::<entero, texto>(1, \"x\")\npar::<texto, entero>(\"z\", 2)\n",
    );
}

#[test]
fn v3_contenedor_concreto_lista_valido() {
    ok(
        "funcion suma_lista(xs: lista<entero>) -> entero {\n  retornar xs[0]\n}\nfuncion env<T>(xs: lista<T>) -> T {\n  retornar xs[0]\n}\nfuncion main() {\n  env::<entero>([1, 2, 3])\n}\nmain()",
    );
}

#[test]
fn g2_2_1_observacion_incompatibilidad_ordinaria_explicita_medio() {
    // G2.2.1 observación MEDIO: deferral de incompatibilidad ordinaria en llamada genérica
    // explícita válida (aridad/ligadura) ahora se diagnostica en G3 (V3 concreta).
    err_codigos(
        "funcion id<T>(x: T) -> T {\n  retornar x\n}\nid::<entero>(\"texto\")\n",
        "E1049",
    );
}
