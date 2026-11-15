use basilisco::{ContextoSemantico, Expresion, Lexer, Parser, Sentencia, Valor};

fn parsear(fuente: &str) -> Vec<Sentencia> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico");
    Parser::nuevo(tokens).parsear_programa().expect("sintaxis")
}

fn analizar(fuente: &str) -> Vec<basilisco::ErrorEjecucion> {
    let ss = parsear(fuente);
    let mut ctx = ContextoSemantico::nuevo();
    ctx.analizar_programa(&ss)
}

#[test]
fn sem_func_antes_uso() {
    let e = analizar("funcion f() { retornar 1 }\nf()");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_func_despues_uso() {
    let e = analizar("f()\nfuncion f() { retornar 1 }");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_recursion() {
    let e = analizar(
        "funcion fact(n) {\n  si n <= 1 { retornar 1 }\n  retornar n * fact(n - 1)\n}\nfact(5)",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_parametros_locales() {
    let e = analizar("funcion f(xx) {\n  sea yy = xx + 1\n  retornar yy\n}\nf(3)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_dos_funciones_mismo_nombre_local() {
    let e = analizar(
        "funcion a() {\n  sea xx = 1\n  retornar xx\n}\nfuncion b() {\n  sea xx = 2\n  retornar xx\n}\na()\nb()",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_ambitos_bloques() {
    let e =
        analizar("sea x = 1\nsi x == 1 {\n  sea yy = 2\n  yy\n} sino {\n  sea zz = 3\n  zz\n}\nx");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_identificador_no_definido() {
    let e = analizar("xx");
    assert!(!e.is_empty(), "debe rechazar identificador no definido");
}

#[test]
fn sem_declaracion_duplicada() {
    let e = analizar("sea x = 1\nsea x = 2");
    assert!(!e.is_empty(), "debe rechazar duplicado");
}

#[test]
fn sem_funcion_inexistente() {
    let e = analizar("noexiste()");
    assert!(!e.is_empty(), "debe rechazar función inexistente");
}

#[test]
fn sem_expresiones_llamadas() {
    let e = analizar("funcion suma(a, b) { retornar a + b }\nsuma(2, 3) + 1");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_neg_asignacion_no_definida() {
    let e = analizar("xx = 5");
    assert!(!e.is_empty());
}

#[test]
fn sem_sombreado_valido() {
    let e = analizar("sea x = 1\n{\n  sea x = 2\n  x\n}\nx");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_asignacion_incompatible() {
    let e = analizar("var x: entero = 1\nx = \"a\"");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_operacion_tipos_incompatibles() {
    let e = analizar("1 + \"a\"");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_arity_incorrecta() {
    let e = analizar("funcion f(a, b) { retornar a }\nf(1)");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_retorno_incompatible() {
    let e = analizar("funcion f() -> entero { retornar \"a\" }");
    // calling not needed; body analyzed
    let _ = analizar("funcion f() -> entero { retornar \"a\" }\nf()");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_programa_valido_complejo() {
    let src = "funcion doble(n) { retornar n * 2 }\nfuncion main() {\n  sea i = 0\n  sea acc = 0\n  mientras i < 3 {\n    acc = acc + doble(i)\n    i = i + 1\n  }\n  retornar acc\n}\nmain()";
    let e = analizar(src);
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_diagnostico_ubicacion() {
    let e = analizar("xx");
    assert!(!e.is_empty());
    assert_eq!(e[0].linea, 1);
    assert_eq!(e[0].columna, 1);
}

#[test]
fn sem_indirecta_valida() {
    let e = analizar("funcion doble(x: entero) -> entero { retornar x * 2 }\nsea f = doble\nf(5)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_indirecta_arity_invalida() {
    let e = analizar(
        "funcion doble(x: entero) -> entero { retornar x * 2 }\nfuncion obtener() { retornar doble }\nobtener()(1, 2)",
    );
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_indirecta_tipo_invalido() {
    let e = analizar(
        "funcion doble(x: entero) -> entero { retornar x * 2 }\nfuncion obtener() { retornar doble }\nobtener()(\"a\")",
    );
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_indirecta_retorno_propagado() {
    let e =
        analizar("funcion doble(x: entero) -> entero { retornar x * 2 }\nsea f = doble\nf(5) + 1");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_indirecta_funcion_recibida() {
    let e = analizar(
        "funcion aplicar(f, x) { retornar f(x) }\nfuncion doble(n) { retornar n * 2 }\naplicar(doble, 3)",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_indirecta_funcion_devuelta() {
    let e = analizar(
        "funcion doble(x) { retornar x * 2 }\nfuncion obtener() { retornar doble }\nobtener()(4)",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_indirecta_desconocido_no_rechaza() {
    let e = analizar("funcion f() { retornar 1 }\nsea x = f()\nx()");
    // x no es función; puede fallar en ejecución, pero el análisis no debe panic.
    let _ = e;
}

#[test]
fn sem_tipo_funcion_conserva_firma() {
    let e = analizar(
        "funcion suma(a: entero, b: entero) -> entero { retornar a + b }\nsea f = suma\nf(1, 2)",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_variable_funcion_conserva_tipo() {
    let e = analizar("funcion doble(x: entero) -> entero { retornar x * 2 }\nsea f = doble\nf(5)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_funciones_compatibles() {
    let e = analizar(
        "funcion a(xx: entero) -> entero { retornar xx }\nfuncion b(yy: entero) -> entero { retornar yy }\nsea f = a\nf = b",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_funciones_distinta_arity_incompatibles() {
    let e = analizar(
        "funcion a(xx: entero) -> entero { retornar xx }\nfuncion b(xx: entero, yy: entero) -> entero { retornar xx + yy }\nsea f = a\nf = b",
    );
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_parametro_incompatible_funcion() {
    let e = analizar("funcion f(x: entero) -> entero { retornar x }\nf(\"a\")");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_retorno_incompatible_funcion() {
    let e = analizar("funcion f() -> entero { retornar \"a\" }");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_desconocido_vs_nulo() {
    let e = analizar("funcion f() { retornar 1 }\nsea x = f()\nx");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_funcion_recibe_funcion() {
    let e = analizar(
        "funcion aplicar(f, x) { retornar f(x) }\nfuncion doble(n) { retornar n * 2 }\naplicar(doble, 3)",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_funcion_devuelve_funcion() {
    let e = analizar(
        "funcion doble(x) { retornar x * 2 }\nfuncion obtener() { retornar doble }\nobtener()(4)",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_firma_anidada() {
    let e = analizar(
        "funcion aplicar(f, x) { retornar f(x) }\nfuncion doble(n) { retornar n * 2 }\naplicar(doble, 2)",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_locales_aisladas_entre_funciones() {
    let e = analizar("funcion a() { sea xx = 1\n retornar xx }\nfuncion b() { retornar xx }");
    assert!(!e.is_empty(), "b no debe ver local de a: {:?}", e);
}

#[test]
fn sem_bloque_no_visible_fuera() {
    let e = analizar("{ sea interna = 1 }\ninterna");
    assert!(!e.is_empty(), "no debe ser visible fuera: {:?}", e);
}

#[test]
fn sem_sombreado_anidado_valido() {
    let e = analizar("sea x = 1\n{\n  sea x = 2\n  {\n    sea x = 3\n    x\n  }\n  x\n}\nx");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_variable_usada_antes_de_declaracion() {
    let e = analizar("xx\nsea xx = 1");
    assert!(
        !e.is_empty(),
        "debe rechazar uso antes de declaración: {:?}",
        e
    );
}

#[test]
fn sem_llamada_anticipada_y_recursion() {
    let e = analizar("f()\nfuncion f() { retornar g() }\nfuncion g() { retornar 7 }");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_argumento_incompatible() {
    let e = analizar("funcion f(x: entero) { retornar x }\nf(\"a\")");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_retorno_anotado_incompatible() {
    let e = analizar("funcion f() -> entero { retornar 1.5 }");
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_funcion_sin_anotacion_retorno() {
    let e = analizar("funcion f() { retornar 1 }\nf()");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_conversiones_implicitas() {
    let e = analizar("var xx: decimal = 1\nvar yy: real = 2\nxx = 3\nyy = 4");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_operacion_mixta_valida() {
    let e = analizar("1 + 2.0 + 3.5");
    // 1 entero + 2.0 real => real; + 3.5 real => real
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_tipo_no_determinable_no_error() {
    let e = analizar("funcion f() { retornar 1 }\nsea x = f()\nx + 1");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_retornar_fuera_funcion() {
    let sentencias = vec![Sentencia::Retorno {
        valor: Some(Expresion::Literal {
            valor: Valor::Entero(1),
            linea: 1,
            columna: 1,
        }),
        linea: 1,
        columna: 1,
    }];
    let mut ctx = ContextoSemantico::nuevo();
    let e = ctx.analizar_programa(&sentencias);
    assert!(!e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_condicion_no_logica() {
    let e = analizar("si 1 { 2 }");
    assert!(!e.is_empty(), "errores: {:?}", e);
    let e2 = analizar("mientras 1 { 2 }");
    assert!(!e2.is_empty(), "errores: {:?}", e2);
}

#[test]
fn sem_retorno_en_condicional() {
    let e = analizar("funcion f(x) {\n  si x > 0 {\n    retornar x\n  }\n  retornar 0\n}\nf(1)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_modulo_limitacion_no_engañosa() {
    // El analizador no resuelve `usar`; no debe fallar ni declarar éxito engañoso.
    // Aquí solo se verifica que un programa con `usar` no rompa el análisis.
    let e = analizar("usar \"lib.bsk\" como lib\n1");
    // Puede ser vacío o con diagnósticos, pero no debe panic.
    let _ = e;
}

#[test]
fn sem_dato_const_reasignacion() {
    let e = analizar("dato x: entero = 1\nx = 2");
    assert!(!e.is_empty(), "errores: {:?}", e);
    let e2 = analizar("const X: entero = 1\nX = 2");
    assert!(!e2.is_empty(), "errores: {:?}", e2);
}

#[test]
fn sem_lambda_con_retorno_es_valido() {
    let e = analizar("sea doble = funcion (x) { retornar x * 2 }");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_lambda_retorno_anotado_incorrecto_es_error() {
    let e = analizar("sea f = funcion (x) -> entero { retornar \"t\" }");
    assert!(e.iter().any(|err| err.mensaje.contains("retorno")), "{e:?}");
}

#[test]
fn sem_function_anidada_es_valido() {
    let e = analizar(
        "funcion externa() {\n  funcion interna() { retornar 1 }\n  retornar interna()\n}",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_function_anidada_duplicada_es_error() {
    let e = analizar(
        "funcion externa() {\n  funcion interna() { retornar 1 }\n  funcion interna() { retornar 2 }\n}",
    );
    assert!(
        e.iter().any(|err| err.mensaje.contains("duplicada")),
        "{e:?}"
    );
}

#[test]
fn sem_function_anidada_antes_de_declarar_es_error() {
    let e = analizar(
        "funcion externa() {\n  retornar interna()\n  funcion interna() { retornar 1 }\n}",
    );
    assert!(
        e.iter()
            .any(|err| err.mensaje.contains("no definida: interna")),
        "{e:?}"
    );
}

#[test]
fn sem_function_duplicada_nivel_superior_es_error() {
    let e = analizar("funcion f() { retornar 1 }\nfuncion f() { retornar 2 }");
    assert!(
        e.iter().any(|err| err.mensaje.contains("duplicada")),
        "{e:?}"
    );
}

#[test]
fn sem_lambda_ariodad_incorrecta_es_error() {
    let e = analizar("sea f = funcion (x) { retornar x }\nf()  \nf(1, 2)");
    assert!(!e.is_empty(), "{e:?}");
}

#[test]
fn sem_lambda_como_valor_y_llamada_valida() {
    let e = analizar("sea f = funcion (x) { retornar x }\nf(3)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

// --- B.1: inferencia en para cada e indexación ---

#[test]
fn sem_b1_para_cada_arreglo_tipo_conocido() {
    let e = analizar(
        "funcion main() {\n  var xs: arreglo<entero, 3> = [1, 2, 3]\n  para cada x en xs {\n    x + 1\n  }\n}\nmain()",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b1_para_cada_lista_tipo_conocido() {
    let e = analizar(
        "funcion main() {\n  var xs: lista<entero> = [1, 2]\n  para cada x en xs {\n    x + 1\n  }\n}\nmain()",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b1_para_cada_coleccion_tipo_conocido() {
    let e = analizar("funcion main() {\n  para cada x en [1, 2, 3] {\n    x + 1\n  }\n}\nmain()");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b1_para_cada_elemento_desconocido_permisivo() {
    // Iterable/elemento sin tipo estático determinable: se mantiene Desconocido
    // y no se introduce diagnóstico especulativo (comportamiento conservador).
    let e = analizar(
        "funcion consumir(v) {\n  para cada x en v {\n    x + 1\n  }\n}\nconsumir([1, 2])",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b1_para_cada_incompatibilidad_uso_posterior() {
    let e =
        analizar("funcion main() {\n  para cada x en [1, 2, 3] {\n    x + \"a\"\n  }\n}\nmain()");
    assert!(
        !e.is_empty(),
        "debe detectar incompatibilidad entre elemento inferido y uso posterior: {:?}",
        e
    );
}

#[test]
fn sem_b1_indexacion_arreglo_tipo_conocido() {
    let e = analizar(
        "funcion main() {\n  var xs: arreglo<entero, 2> = [1, 2]\n  var x: entero = xs[0]\n  x\n}\nmain()",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b1_indexacion_lista_tipo_conocido() {
    let e = analizar(
        "funcion main() {\n  var xs: lista<entero> = [1, 2]\n  var x: entero = xs[0]\n  x\n}\nmain()",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b1_indexacion_coleccion_tipo_conocido() {
    let e = analizar("var x: entero = [1, 2][0]\nx");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b1_indexacion_rechaza_indice_no_entero_arreglo() {
    let e =
        analizar("funcion main() {\n  var xs: arreglo<entero, 2> = [1, 2]\n  xs[\"0\"]\n}\nmain()");
    assert!(
        !e.is_empty(),
        "debe rechazar índice no entero en arreglo: {:?}",
        e
    );
}

#[test]
fn sem_b1_indexacion_rechaza_indice_no_entero_lista_y_coleccion() {
    let e_lista =
        analizar("funcion main() {\n  var xs: lista<entero> = [1, 2]\n  xs[verdadero]\n}\nmain()");
    assert!(
        !e_lista.is_empty(),
        "debe rechazar índice no entero en lista: {:?}",
        e_lista
    );
    let e_col = analizar("[1, 2][\"0\"]");
    assert!(
        !e_col.is_empty(),
        "debe rechazar índice no entero en colección: {:?}",
        e_col
    );
}

#[test]
fn sem_b1_indexacion_incompatibilidad_uso_posterior() {
    let e = analizar("var x: texto = [1, 2][0]\nx");
    assert!(
        !e.is_empty(),
        "debe detectar incompatibilidad entre elemento inferido por indexación y uso posterior: {:?}",
        e
    );
}

// --- B.2: inferencia local de variables, expresiones y colecciones ---

#[test]
fn sem_b2_declaracion_inferida_desde_expresion_conocida() {
    // Sin anotación: el tipo local se infiere de la expresión.
    let e = analizar("sea x = 1 + 2\nx + 3");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b2_declaracion_inferida_rechaza_uso_incompatible() {
    let e = analizar("sea x = 1\nx + \"a\"");
    assert!(
        !e.is_empty(),
        "debe rechazar uso incompatible del tipo inferido localmente: {:?}",
        e
    );
}

#[test]
fn sem_b2_asignacion_refina_desconocido_y_detecta_incompatibilidad_posterior() {
    // variable sin tipo conocido se refina con evidencia local demostrable;
    // una reasignación posterior incompatible debe detectarse.
    let e = analizar("funcion f(v) {\n  sea x = v\n  x = 1\n  x = \"a\"\n}\nf(0)");
    assert!(
        !e.is_empty(),
        "debe detectar incompatibilidad tras refinar tipo local por asignación: {:?}",
        e
    );
}

#[test]
fn sem_b2_asignacion_con_tipo_conocido_rechaza_incompatible() {
    let e = analizar("var x: entero = 1\nx = \"a\"");
    assert!(
        !e.is_empty(),
        "debe rechazar asignación incompatible a variable tipada: {:?}",
        e
    );
}

#[test]
fn sem_b2_compatibilidad_tipo_declarado_e_inicializador() {
    let ok = analizar("var x: decimal = 1\nx");
    assert!(
        ok.is_empty(),
        "entero debe ser compatible con decimal: {:?}",
        ok
    );
    let err = analizar("var x: entero = \"a\"");
    assert!(
        !err.is_empty(),
        "inicializador incompatible con tipo declarado debe rechazarse: {:?}",
        err
    );
}

#[test]
fn sem_b2_coleccion_homogenea_entero() {
    let e = analizar("para cada x en [1, 2, 3] {\n  x + 1\n}");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b2_coleccion_homogenea_promocion_numerica() {
    // Evidencia local suficiente para tipo común numérico.
    let e = analizar("para cada x en [1, 1.0] {\n  x + 1.0\n}");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b2_coleccion_heterogenea_sin_evidencia_comun() {
    // Tipos heterogéneos sin tipo común demostrable: elemento desconocido,
    // sin diagnóstico especulativo.
    let e = analizar("para cada x en [1, \"a\"] {\n  x\n}");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b2_coleccion_vacia_sin_inventar_tipo() {
    let e = analizar("para cada x en [] {\n  x + 1\n}");
    assert!(
        e.is_empty(),
        "colección vacía no debe inventar tipo ni provocar falsos positivos: {:?}",
        e
    );
}

#[test]
fn sem_b2_coleccion_con_elemento_desconocido_mantiene_desconocido() {
    let e = analizar("funcion f(v) {\n  para cada x en [1, v] {\n    x + 1\n  }\n}\nf(2)");
    assert!(
        e.is_empty(),
        "elemento desconocido impide afirmar tipo homogéneo: comportamiento conservador: {:?}",
        e
    );
}

#[test]
fn sem_b2_expresiones_conocidas_y_desconocidas() {
    let ok = analizar("funcion f(v) {\n  sea x = v + 1\n  x\n}\nf(1)");
    assert!(
        ok.is_empty(),
        "operar con desconocido debe seguir siendo permisivo: {:?}",
        ok
    );
    let err = analizar("1 + \"a\"");
    assert!(
        !err.is_empty(),
        "incompatibilidad demostrable entre tipos conocidos debe rechazarse: {:?}",
        err
    );
}

#[test]
fn sem_b2_operaciones_numericas_y_conversiones_existentes() {
    let e = analizar(
        "var a: decimal = 1\nvar b: real = 1\nsea c = 1 + 1.0\nsea d = 1.0 + 1\nc\nd\na\nb",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b2_preserva_diagnosticos_existentes() {
    let e = analizar("var x: entero = \"texto\"");
    assert!(
        e.iter()
            .any(|err| err.mensaje.contains("asignación incompatible")),
        "debe preservar diagnóstico de incompatibilidad: {:?}",
        e
    );
}

// --- B.3: consistencia conservadora en contextos locales ---

#[test]
fn sem_b3_asignacion_indice_elemento_conocido_rechaza_incompatible() {
    let e = analizar(
        "funcion main() {\n  var xs: arreglo<entero, 2> = [1, 2]\n  xs[0] = \"a\"\n}\nmain()",
    );
    assert!(
        !e.is_empty(),
        "asignar valor incompatible a elemento conocido debe rechazarse: {:?}",
        e
    );
}

#[test]
fn sem_b3_asignacion_indice_elemento_desconocido_permisivo() {
    let e = analizar("funcion f(v) {\n  para cada x en v {\n  }\n  v[0] = \"a\"\n}\nf([1, 2])");
    assert!(
        e.is_empty(),
        "sin tipo de elemento demostrable, la asignación por índice debe ser permisiva: {:?}",
        e
    );
}

#[test]
fn sem_b3_asignacion_indice_rechaza_indice_no_entero() {
    let e =
        analizar("funcion main() {\n  var xs: lista<entero> = [1, 2]\n  xs[\"0\"] = 3\n}\nmain()");
    assert!(
        !e.is_empty(),
        "índice no entero en asignación por índice debe rechazarse: {:?}",
        e
    );
}

#[test]
fn sem_b3_conocido_produce_rechazo_operaciones_y_comparaciones() {
    let e = analizar("1 < \"a\"");
    assert!(
        !e.is_empty(),
        "comparación/order incompatibilidad demostrable debe rechazarse: {:?}",
        e
    );
}

#[test]
fn sem_b3_desconocido_conserva_permisivo_en_comparacion() {
    let e = analizar("funcion f(v) {\n  sea r = v < 3\n  r\n}\nf(1)");
    assert!(
        e.is_empty(),
        "comparación con operando desconocido debe conservar política permisiva: {:?}",
        e
    );
}

#[test]
fn sem_b3_colecciones_y_expresiones_combinadas_consistencia_local() {
    // [1, 2.0] tiene tipo común numérico local demostrable (decimal/real según reglas).
    let e = analizar("funcion main() {\n  sea xs = [1, 2.0]\n  xs[0] = 3.0\n}\nmain()");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_b3_regresion_b1_b2() {
    // Regresión conjunta: B.1 (iteración/indexación) y B.2 (inferencia local).
    let e = analizar(
        "funcion main() {\n  var xs: arreglo<entero, 2> = [1, 2]\n  para cada x en xs {\n    x + xs[0]\n  }\n  xs[0] = xs[1]\n}\nmain()",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

// --- C.1: unificación semántica conservadora de firmas de valores-función ---

#[test]
fn sem_c1_t01_funcion_nombrada_anotaciones_completas() {
    let e = analizar(
        "funcion suma(a: entero, b: entero) -> entero { retornar a + b }\nvar x: entero = suma(1, 2)\nx",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t02_funcion_sin_retorno_anotado_conservador() {
    // C.1: sin anotación explícita el retorno de la firma es Desconocido
    // (no se infiere desde el cuerpo). Uso conservador, sin falsos positivos.
    let e = analizar("funcion f() { retornar 1 }\nvar x: entero = f()\nx");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t03_referencia_funcion_como_valor() {
    let e = analizar("funcion g() -> entero { retornar 1 }\nsea f = g\nvar x: entero = f()\nx");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t04_lambda_params_y_retorno_anotados() {
    let e = analizar("sea f = funcion (x: entero) -> entero { retornar x + 1 }\nf(1) + 2");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t05_lambda_params_anotados_sin_retorno_anotado() {
    // Sin anotación de retorno, la firma conserva Desconocido (C.1).
    let e = analizar("sea f = funcion (x: entero) { retornar x + 1 }\nf(1)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t06_lambda_parametros_desconocidos() {
    let e = analizar("sea f = funcion (x) { retornar x }\nf(3)\nf(\"a\")");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t07_asignacion_funcion_a_variable_y_llamada() {
    let e = analizar("funcion inc(x: entero) -> entero { retornar x + 1 }\nsea f = inc\nf(2) + 1");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t08_paso_funcion_como_argumento_hof() {
    // Receptor sin firma de función tipada en parámetro: conservador.
    let e = analizar(
        "funcion aplicar(g, v) { retornar g(v) }\nfuncion inc(x: entero) -> entero { retornar x + 1 }\naplicar(inc, 5)",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t09_llamada_directa_aridad_correcta() {
    let e = analizar("funcion suma(a: entero, b: entero) -> entero { retornar a + b }\nsuma(1, 2)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t10_llamada_directa_aridad_incorrecta() {
    let e = analizar("funcion suma(a: entero, b: entero) -> entero { retornar a + b }\nsuma(1)");
    assert!(!e.is_empty(), "debe rechazar aridad incorrecta: {:?}", e);
}

#[test]
fn sem_c1_t11_argumentos_compatibles() {
    // Conversión/compatibilidad existente: entero compatible con decimal.
    let e = analizar("funcion f(x: decimal) -> decimal { retornar x }\nf(1) + 1.0");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t12_argumentos_incompatibles_demostrables() {
    let e = analizar("funcion f(x: entero) -> entero { retornar x }\nf(\"a\")");
    assert!(
        !e.is_empty(),
        "debe rechazar argumento incompatible demostrable: {:?}",
        e
    );
}

#[test]
fn sem_c1_t13_llamada_indirecta_firma_conocida() {
    let e =
        analizar("funcion doble(x: entero) -> entero { retornar x * 2 }\nsea f = doble\nf(3) + 1");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t14_llamada_indirecta_callee_desconocido() {
    let e = analizar("funcion aplicar(v) {\n  v(1)\n}\naplicar(1)");
    assert!(
        e.is_empty(),
        "callee desconocido debe tratarse de forma conservadora: {:?}",
        e
    );
}

#[test]
fn sem_c1_t15_firma_parcial_parametros_desconocidos() {
    // Parámetro sin anotación en firma conocida: validación conservadora.
    let e = analizar("funcion id(x) -> entero { retornar 1 }\nid(\"a\")\nid(1)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t16_closure_como_valor() {
    // Función/lambda usada como valor (captura cubierta por contratos closures.rs).
    let e = analizar("sea f = funcion (x) { retornar x }\nf(2)");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t17_funcion_lambda_retornada_o_pasada_valor() {
    // Lambda con retorno anotado retornada como valor.
    let e = analizar(
        "funcion obtener() {\n  sea f = funcion (x: entero) -> entero { retornar x + 1 }\n  retornar f\n}\nsea g = obtener()\ng(9) + 1",
    );
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t18_retorno_conocido_por_anotacion_propagado() {
    let e = analizar("funcion uno() -> entero { retornar 1 }\nsea x = uno()\nx + 2");
    assert!(e.is_empty(), "errores: {:?}", e);
}

#[test]
fn sem_c1_t19_retorno_no_anotado_en_valor_funcion_conservador() {
    // C.1: la firma como valor-función usa retorno anotado; sin anotación es
    // Desconocido y su uso debe ser conservador (sin falsos positivos).
    let e = analizar("funcion f() { retornar 1 }\nsea g = f\ng()");
    assert!(
        e.is_empty(),
        "retorno no anotado debe ser conservador en valor-función: {:?}",
        e
    );
}
