use basilisco::{Entorno, ErrorEjecucion, Lexer, Parser, Valor};

fn ejecutar(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida");
    Entorno::nueva().ejecutar_programa(&sentencias)
}

fn valor(fuente: &str) -> Valor {
    ejecutar(fuente)
        .expect("ejecución sin error")
        .expect("el programa produce valor")
}

#[test]
fn rango_materializado_supera_limite_controlado() {
    let fuente = "para cada n en 1..=100001 {\n  n\n}\n0";
    let resultado = ejecutar(fuente);
    assert!(resultado.is_err(), "debe rechazar rango excesivo");
    assert!(format!("{:?}", resultado.unwrap_err()).contains("LimiteIteraciones"));
}

#[test]
fn rango_exclusivo_no_incluye_el_extremo_final() {
    let fuente = "sea suma = 0\npara cada n en 1..3 {\n  suma = suma + n\n}\nsuma";
    assert_eq!(valor(fuente), Valor::Entero(3));
}

#[test]
fn rangos_exclusivos_vacios_y_descendentes_no_iteran() {
    for rango in ["1..1", "3..1"] {
        let fuente = format!(
            "sea cantidad = 0\npara cada n en {rango} {{\n  cantidad = cantidad + 1\n}}\ncantidad"
        );
        assert_eq!(valor(&fuente), Valor::Entero(0), "rango {rango}");
    }
}

#[test]
fn rango_inclusivo_incluye_el_extremo_sin_desbordar() {
    let fuente = "sea cantidad = 0\npara cada n en 9223372036854775806..=9223372036854775807 {\n  cantidad = cantidad + 1\n}\ncantidad";
    assert_eq!(valor(fuente), Valor::Entero(2));
}

#[test]
fn rangos_aceptan_extremos_negativos_y_exclusivo_en_limite() {
    let negativos =
        "sea cantidad = 0\npara cada n en -2..=2 {\n  cantidad = cantidad + 1\n}\ncantidad";
    assert_eq!(valor(negativos), Valor::Entero(5));

    let exclusivo = "sea cantidad = 0\npara cada n en 9223372036854775806..9223372036854775807 {\n  cantidad = cantidad + 1\n}\ncantidad";
    assert_eq!(valor(exclusivo), Valor::Entero(1));
}

#[test]
fn intentar_no_captura_romper_ni_continuar_en_nivel_superior() {
    let fuente_romper = "sea rescatados = 0\nmientras verdadero {\n  intentar {\n    romper\n  } rescatar {\n    rescatados = rescatados + 1\n  }\n}\nrescatados";
    assert_eq!(valor(fuente_romper), Valor::Entero(0));

    let fuente_continuar = "sea i = 0\nsea rescatados = 0\nmientras i < 2 {\n  i = i + 1\n  intentar {\n    continuar\n  } rescatar {\n    rescatados = rescatados + 1\n  }\n}\nrescatados";
    assert_eq!(valor(fuente_continuar), Valor::Entero(0));
}

#[test]
fn intentar_no_captura_senales_en_para_cada() {
    let romper = "sea visitados = 0\npara cada n en [1, 2, 3] {\n  intentar {\n    si n == 2 {\n      romper\n    }\n  } rescatar {\n    visitados = visitados + 100\n  }\n  visitados = visitados + 1\n}\nvisitados";
    assert_eq!(valor(romper), Valor::Entero(1));

    let continuar = "sea suma = 0\npara cada n en [1, 2, 3] {\n  intentar {\n    si n == 2 {\n      continuar\n    }\n  } rescatar {\n    suma = suma + 100\n  }\n  suma = suma + n\n}\nsuma";
    assert_eq!(valor(continuar), Valor::Entero(4));
}

#[test]
fn intentar_no_captura_senales_de_ciclo_dentro_de_funciones() {
    let romper = "funcion probar() {\n  sea rescatados = 0\n  mientras verdadero {\n    intentar {\n      romper\n    } rescatar {\n      rescatados = rescatados + 1\n    }\n  }\n  retornar rescatados\n}\nprobar()";
    assert_eq!(valor(romper), Valor::Entero(0));

    let continuar = "funcion probar() {\n  sea i = 0\n  sea rescatados = 0\n  mientras i < 2 {\n    i = i + 1\n    intentar {\n      continuar\n    } rescatar {\n      rescatados = rescatados + 1\n    }\n  }\n  retornar rescatados\n}\nprobar()";
    assert_eq!(valor(continuar), Valor::Entero(0));
}

#[test]
fn intentar_sigue_capturando_errores_de_ejecucion() {
    let fuente = "sea rescatados = 0\nintentar {\n  1 / 0\n} rescatar {\n  rescatados = rescatados + 1\n}\nrescatados";
    assert_eq!(valor(fuente), Valor::Entero(1));
}

#[test]
fn bloque_intentar_normal_no_activa_rescatar_en_funcion() {
    let fuente = "funcion probar() {\n  sea rescatados = 0\n  intentar {\n    sea valor = 1\n  } rescatar {\n    rescatados = rescatados + 1\n  }\n  retornar rescatados\n}\nprobar()";
    assert_eq!(valor(fuente), Valor::Entero(0));
}

#[test]
fn intentar_captura_error_real_y_preserva_retorno_en_funcion() {
    let captura = "funcion probar() {\n  sea rescatados = 0\n  intentar {\n    1 / 0\n  } rescatar {\n    rescatados = 1\n  }\n  retornar rescatados\n}\nprobar()";
    assert_eq!(valor(captura), Valor::Entero(1));

    let retorno = "funcion probar() {\n  intentar {\n    retornar 7\n  } rescatar {\n    retornar 9\n  }\n}\nprobar()";
    assert_eq!(valor(retorno), Valor::Entero(7));
}
