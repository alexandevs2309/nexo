use basilisco::parser::{Expresion, Sentencia, TipoAnotacion};
use basilisco::{Lexer, Parser};

fn parsear_ok(fuente: &str) -> Vec<Sentencia> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico");
    Parser::nuevo(tokens).parsear_programa().expect("sintaxis")
}

fn parsear_err(fuente: &str) -> String {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico");
    match Parser::nuevo(tokens).parsear_programa() {
        Ok(ast) => panic!("se esperaba error de sintaxis para: {fuente}\nAST: {ast:?}"),
        Err(e) => e.mensaje,
    }
}

fn assert_err(fuente: &str, esperado: &str) {
    let msg = parsear_err(fuente);
    assert!(
        msg.contains(esperado),
        "mensaje inesperado.\nfuente: {fuente}\nesperado contiene: {esperado:?}\nreal: {msg:?}"
    );
}

// --- Declaraciones genéricas y uso válido de parámetros de tipo declarados ---

#[test]
fn funcion_generica_un_parametro_ast() {
    let ss = parsear_ok("funcion identidad<T>(valor: T) -> T {\n  retornar valor\n}\n");
    match &ss[0] {
        Sentencia::Funcion {
            nombre,
            parametros_tipo,
            parametros,
            tipo_retorno,
            ..
        } => {
            assert_eq!(nombre, "identidad");
            assert_eq!(parametros_tipo, &vec!["T".to_string()]);
            assert_eq!(parametros.len(), 1);
            assert_eq!(parametros[0].0, "valor");
            assert_eq!(parametros[0].1, Some(TipoAnotacion::Parametro("T".into())));
            assert_eq!(tipo_retorno, &Some(TipoAnotacion::Parametro("T".into())));
        }
        otro => panic!("esperaba Funcion, obtuvo {otro:?}"),
    }
}

#[test]
fn uso_valido_parametros_tipo_en_firma_y_cuerpo() {
    // Parámetros de tipo declarados usables en anotaciones de firma y cuerpo.
    let ss = parsear_ok("funcion id<T>(x: T) -> T {\n  var z: T = x\n  retornar z\n}\n");
    match &ss[0] {
        Sentencia::Funcion {
            parametros_tipo,
            parametros,
            tipo_retorno,
            cuerpo,
            ..
        } => {
            assert_eq!(parametros_tipo, &vec!["T".to_string()]);
            assert_eq!(parametros[0].1, Some(TipoAnotacion::Parametro("T".into())));
            assert_eq!(tipo_retorno, &Some(TipoAnotacion::Parametro("T".into())));
            match &cuerpo[0] {
                Sentencia::Declaracion {
                    nombre,
                    tipo_anotacion,
                    ..
                } => {
                    assert_eq!(nombre, "z");
                    assert_eq!(tipo_anotacion, &Some(TipoAnotacion::Parametro("T".into())));
                }
                otro => panic!("esperaba Declaracion, obtuvo {otro:?}"),
            }
        }
        otro => panic!("esperaba Funcion, obtuvo {otro:?}"),
    }
}

#[test]
fn funcion_generica_multiples_parametros_y_coma_final() {
    let ss = parsear_ok("funcion combinar<T, U,>(a: T, b: U) {\n  retornar a\n}\n");
    match &ss[0] {
        Sentencia::Funcion {
            nombre,
            parametros_tipo,
            parametros,
            ..
        } => {
            assert_eq!(nombre, "combinar");
            assert_eq!(parametros_tipo, &vec!["T".to_string(), "U".to_string()]);
            assert_eq!(parametros.len(), 2);
            assert_eq!(parametros[0].1, Some(TipoAnotacion::Parametro("T".into())));
            assert_eq!(parametros[1].1, Some(TipoAnotacion::Parametro("U".into())));
        }
        otro => panic!("esperaba Funcion, obtuvo {otro:?}"),
    }
}

#[test]
fn parametros_tipo_en_colecciones_existentes_y_anidadas() {
    let ss = parsear_ok(
        "funcion f<T>(xs: lista<T>) -> arreglo<T, 3> {\n  var ys: lista<lista<T>> = xs\n  retornar xs\n}\n",
    );
    match &ss[0] {
        Sentencia::Funcion {
            parametros,
            tipo_retorno,
            cuerpo,
            ..
        } => {
            assert_eq!(
                parametros[0].1,
                Some(TipoAnotacion::Lista(Box::new(TipoAnotacion::Parametro(
                    "T".into()
                ))))
            );
            assert_eq!(
                tipo_retorno,
                &Some(TipoAnotacion::Arreglo(
                    Box::new(TipoAnotacion::Parametro("T".into())),
                    3
                ))
            );
            match &cuerpo[0] {
                Sentencia::Declaracion { tipo_anotacion, .. } => {
                    assert_eq!(
                        tipo_anotacion,
                        &Some(TipoAnotacion::Lista(Box::new(TipoAnotacion::Lista(
                            Box::new(TipoAnotacion::Parametro("T".into()))
                        ))))
                    );
                }
                otro => panic!("esperaba Declaracion, obtuvo {otro:?}"),
            }
        }
        otro => panic!("esperaba Funcion, obtuvo {otro:?}"),
    }
}

// --- Rechazo de tipos desconocidos fuera de contexto genérico válido ---

#[test]
fn rechazo_tipo_desconocido_fuera_de_funcion_generica() {
    assert_err(
        "var x: Foo = 1\n",
        "tipo desconocido; se esperaba: entero, decimal, real, logico, texto, nulo, arreglo<T, N>, lista<T>",
    );
    assert_err(
        "funcion f(x: Foo) -> entero {\n  retornar 1\n}\n",
        "tipo desconocido; se esperaba: entero, decimal, real, logico, texto, nulo, arreglo<T, N>, lista<T>",
    );
    assert_err(
        "funcion f(x: entero) -> Bar {\n  retornar x\n}\n",
        "tipo desconocido; se esperaba: entero, decimal, real, logico, texto, nulo, arreglo<T, N>, lista<T>",
    );
}

#[test]
fn rechazo_tipo_desconocido_no_declarado_en_funcion_generica() {
    // U no está declarado como parámetro de tipo de f<T>.
    assert_err(
        "funcion f<T>(x: U) -> T {\n  retornar x\n}\n",
        "tipo desconocido; se esperaba: entero, decimal, real, logico, texto, nulo, arreglo<T, N>, lista<T>",
    );
}

// --- Turbofish (llamada directa por identificador) ---

#[test]
fn turbofish_llamada_directa_simple() {
    let ss = parsear_ok("identidad::<entero>(42)\n");
    match &ss[0] {
        Sentencia::Expresion(Expresion::Llamada {
            nombre,
            argumentos,
            argumentos_tipo,
            ..
        }) => {
            assert_eq!(nombre, "identidad");
            assert_eq!(argumentos.len(), 1);
            assert_eq!(argumentos_tipo, &vec![TipoAnotacion::Entero]);
        }
        otro => panic!("esperaba llamada, obtuvo {otro:?}"),
    }
}

#[test]
fn turbofish_multiples_y_anidados_con_coma_final() {
    let ss = parsear_ok("combinar::<entero, lista<texto>,>(1, x)\n");
    match &ss[0] {
        Sentencia::Expresion(Expresion::Llamada {
            argumentos_tipo, ..
        }) => {
            assert_eq!(argumentos_tipo.len(), 2);
            assert_eq!(argumentos_tipo[0], TipoAnotacion::Entero);
            assert_eq!(
                argumentos_tipo[1],
                TipoAnotacion::Lista(Box::new(TipoAnotacion::Texto))
            );
        }
        otro => panic!("esperaba llamada, obtuvo {otro:?}"),
    }
}

#[test]
fn turbofish_con_tipos_anidados_profundos() {
    let ss = parsear_ok("f::<lista<lista<entero>>>(x)\n");
    match &ss[0] {
        Sentencia::Expresion(Expresion::Llamada {
            argumentos_tipo, ..
        }) => {
            assert_eq!(argumentos_tipo.len(), 1);
            assert_eq!(
                argumentos_tipo[0],
                TipoAnotacion::Lista(Box::new(TipoAnotacion::Lista(Box::new(
                    TipoAnotacion::Entero
                ))))
            );
        }
        otro => panic!("esperaba llamada, obtuvo {otro:?}"),
    }
    let ss = parsear_ok("g::<arreglo<entero, 3>>(x)\n");
    match &ss[0] {
        Sentencia::Expresion(Expresion::Llamada {
            argumentos_tipo, ..
        }) => {
            assert_eq!(
                argumentos_tipo[0],
                TipoAnotacion::Arreglo(Box::new(TipoAnotacion::Entero), 3)
            );
        }
        otro => panic!("esperaba llamada, obtuvo {otro:?}"),
    }
}

#[test]
fn turbofish_con_parametro_tipo_en_alcance() {
    let ss = parsear_ok("funcion f<T>(x: T) -> T {\n  retornar identidad::<T>(x)\n}\n");
    match &ss[0] {
        Sentencia::Funcion { cuerpo, .. } => match &cuerpo[0] {
            Sentencia::Retorno {
                valor:
                    Some(Expresion::Llamada {
                        argumentos_tipo, ..
                    }),
                ..
            } => {
                assert_eq!(argumentos_tipo, &vec![TipoAnotacion::Parametro("T".into())]);
            }
            otro => panic!("esperaba retorno con llamada, obtuvo {otro:?}"),
        },
        otro => panic!("esperaba Funcion, obtuvo {otro:?}"),
    }
}

#[test]
fn llamada_sin_turbofish_sigue_funcionando() {
    let ss = parsear_ok("suma(1, 2)\n");
    match &ss[0] {
        Sentencia::Expresion(Expresion::Llamada {
            nombre,
            argumentos_tipo,
            argumentos,
            ..
        }) => {
            assert_eq!(nombre, "suma");
            assert!(argumentos_tipo.is_empty());
            assert_eq!(argumentos.len(), 2);
        }
        otro => panic!("esperaba llamada, obtuvo {otro:?}"),
    }
}

// --- Validaciones sintácticas negativas: declaraciones genéricas ---

#[test]
fn rechazo_parametro_tipo_duplicado() {
    assert_err(
        "funcion f<T, T>(x: T) {\n  retornar x\n}\n",
        "el parámetro de tipo 'T' está duplicado",
    );
}

#[test]
fn rechazo_lista_parametros_tipo_vacia() {
    assert_err(
        "funcion f<>(x: entero) {\n  retornar x\n}\n",
        "se esperaba al menos un parámetro de tipo",
    );
}

#[test]
fn rechazo_separador_y_cierres_malformados_en_declaraciones() {
    assert_err(
        "funcion f<T U>(x: T) {\n  retornar x\n}\n",
        "se esperaba ',' o '>' en la lista de parámetros de tipo",
    );
    assert_err(
        "funcion f<T(x: T) {\n  retornar x\n}\n",
        "se esperaba ',' o '>' en la lista de parámetros de tipo",
    );
    assert_err(
        "funcion f<T,>(x: T {\n  retornar x\n}\n",
        "se esperaba ',' o ')'",
    );
}

#[test]
fn rechazo_nombres_builtin_como_parametro_tipo() {
    for nombre in ["entero", "decimal", "real", "logico", "texto"] {
        let src = format!("funcion f<{nombre}>(x: {nombre}) {{\n  retornar x\n}}\n");
        assert_err(
            &src,
            &format!("el nombre '{nombre}' no puede usarse como parámetro de tipo"),
        );
    }
}

#[test]
fn rechazo_nombres_tipo_reservados_token_como_parametro_tipo() {
    for nombre in ["lista", "arreglo", "nulo"] {
        let src = format!("funcion f<{nombre}>(x: entero) {{\n  retornar x\n}}\n");
        assert_err(
            &src,
            &format!("el nombre '{nombre}' no puede usarse como parámetro de tipo"),
        );
    }
}

#[test]
fn rechazo_keywords_como_parametro_tipo() {
    // Keywords representativas (tokens dedicados, no identificadores).
    let keywords = [
        "si",
        "sino",
        "mientras",
        "funcion",
        "retornar",
        "mostrar",
        "verdadero",
        "falso",
        "sea",
        "var",
        "dato",
        "const",
        "para",
        "cada",
        "en",
        "romper",
        "continuar",
        "intentar",
        "rescatar",
        "importar",
        "exportar",
        "usar",
        "como",
        "publico",
    ];
    for kw in keywords {
        let src = format!("funcion f<{kw}>(x: entero) {{\n  retornar x\n}}\n");
        let msg = parsear_err(&src);
        assert!(
            msg.contains(&format!(
                "el nombre '{kw}' no puede usarse como parámetro de tipo"
            )),
            "keyword={kw} msg={msg:?}"
        );
    }
}

// --- Validaciones sintácticas negativas: turbofish / argumentos de tipo ---

#[test]
fn rechazo_turbofish_mal_formado() {
    assert_err("f::<>(1)\n", "se esperaba al menos un argumento de tipo");
    // Separador/cierre malformado en lista de argumentos de tipo.
    let msg = parsear_err("f::<entero(1)\n");
    assert!(
        msg.contains("se esperaba ',' o '>' en la lista de argumentos de tipo"),
        "msg: {msg:?}"
    );
    // Llamada con turbofish válido en forma pero paréntesis de llamada sin cerrar.
    assert_err("f::<entero,>(x\n", "se esperaba ',' o ')'");
}

#[test]
fn rechazo_argumentos_tipo_fuera_de_contexto() {
    // Sin llamada directa.
    assert_err(
        "f::<entero>\n",
        "los argumentos de tipo solo se permiten en llamadas directas por identificador",
    );
    // Acceso a miembro con turbofish (no admitido en G2).
    let _msg_miembro = parsear_err("obj.metodo::<entero>()\n");
    // Llamada indirecta con turbofish (no admitido en G2).
    let _msg_indirecta = parsear_err("(f)::<entero>(1)\n");
    // Ambos deben rechazarse (no parsear como programa válido con turbofish directo admitido).
    // Verificamos rechazo explícito mediante parsear_err arriba.
}

#[test]
fn rechazo_turbofish_acceso_miembro_y_llamada_indirecta() {
    let msg_miembro = parsear_err("obj.metodo::<entero>()\n");
    assert!(
        !msg_miembro.is_empty(),
        "acceso a miembro con turbofish debe rechazarse"
    );
    let msg_indirecta = parsear_err("(f)::<entero>(1)\n");
    assert!(
        !msg_indirecta.is_empty(),
        "llamada indirecta con turbofish debe rechazarse"
    );
}

// --- Regresiones: tipos/colecciones existentes y relacionales ---

#[test]
fn regresion_tipos_y_colecciones_existentes() {
    let _ = parsear_ok(
        "var x: entero = 1\nvar ys: lista<entero> = [1,2]\nvar z: arreglo<texto, 2> = [\"a\",\"b\"]\n",
    );
}

#[test]
fn regresion_colecciones_anidadas_existentes() {
    // Sintaxis de anidación de tipos en anotaciones (preservada).
    let _ = parsear_ok("var z: lista<lista<entero>> = []\n");
}

#[test]
fn regresion_operadores_relacionales() {
    let ss = parsear_ok("sea r = 1 < 2\nsea s = b > c\nsea t = x <= w\nsea u = v >= m\n");
    assert_eq!(ss.len(), 4);
}
