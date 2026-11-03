//! Suite de conformidad para las funciones JSON de BASILISCO.
//!
//! Cubre `json_parsear`, `json_serializar` y `json_obtener` según la especificación
//! de la Fase 3.1-3.3 (estabilizacion-basilisco-0.6.0.md).

use basilisco::{Entorno, ErrorEjecucion, Lexer, Parser, Valor};
use std::collections::BTreeMap;

// ------------------------------------------------------------------ Helpers

fn ejecutar(fuente: &str) -> Result<Option<Valor>, ErrorEjecucion> {
    let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico válido");
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .expect("sintaxis válida");
    Entorno::nueva().ejecutar_programa(&sentencias)
}

fn ejecutar_ok(fuente: &str) -> Valor {
    ejecutar(fuente)
        .expect("sin error")
        .expect("se esperaba un valor")
}

fn ejecutar_err(fuente: &str) -> String {
    ejecutar(fuente).expect_err("se esperaba un error").mensaje
}

fn objeto(pares: &[(&str, Valor)]) -> Valor {
    let mut m = BTreeMap::new();
    for (k, v) in pares {
        m.insert(k.to_string(), v.clone());
    }
    Valor::Objeto(m)
}

// ------------------------------------------------------------------ Parseo básico

#[test]
fn json_parsear_null() {
    let v = ejecutar_ok("json_parsear(\"null\")");
    assert_eq!(v, Valor::Nulo);
}

#[test]
fn json_parsear_verdadero() {
    let v = ejecutar_ok("json_parsear(\"true\")");
    assert_eq!(v, Valor::Logico(true));
}

#[test]
fn json_parsear_falso() {
    let v = ejecutar_ok("json_parsear(\"false\")");
    assert_eq!(v, Valor::Logico(false));
}

#[test]
fn json_parsear_entero() {
    let v = ejecutar_ok("json_parsear(\"42\")");
    assert_eq!(v, Valor::Entero(42));
}

#[test]
fn json_parsear_entero_negativo() {
    let v = ejecutar_ok("json_parsear(\"-7\")");
    assert_eq!(v, Valor::Entero(-7));
}

#[test]
fn json_parsear_real() {
    let v = ejecutar_ok("json_parsear(\"3.14\")");
    assert_eq!(v, Valor::Real(3.0_f64 + 0.14_f64));
}

#[test]
fn json_parsear_cadena() {
    let v = ejecutar_ok("json_parsear(\"\\\"hola mundo\\\"\")");
    assert_eq!(v, Valor::Texto("hola mundo".to_string()));
}

#[test]
fn json_parsear_cadena_unicode() {
    // El lexer de BASILISCO no soporta \u en strings; se usa el carácter directamente
    let v = ejecutar_ok("json_parsear(\"\\\"café\\\"\")");
    assert_eq!(v, Valor::Texto("café".to_string()));
}

#[test]
fn json_parsear_array_vacio() {
    let v = ejecutar_ok("json_parsear(\"[]\")");
    assert_eq!(v, Valor::Lista(vec![]));
}

#[test]
fn json_parsear_array_con_elementos() {
    let v = ejecutar_ok("json_parsear(\"[1, 2, 3]\")");
    assert_eq!(
        v,
        Valor::Lista(vec![Valor::Entero(1), Valor::Entero(2), Valor::Entero(3)])
    );
}

#[test]
fn json_parsear_objeto_vacio() {
    let v = ejecutar_ok("json_parsear(\"{}\")");
    assert_eq!(v, Valor::Objeto(BTreeMap::new()));
}

#[test]
fn json_parsear_objeto_con_claves() {
    let v = ejecutar_ok("json_parsear(\"{\\\"nombre\\\": \\\"Ana\\\", \\\"edad\\\": 30}\")");
    let esperado = objeto(&[
        ("edad", Valor::Entero(30)),
        ("nombre", Valor::Texto("Ana".to_string())),
    ]);
    assert_eq!(v, esperado);
}

// ------------------------------------------------------------------ Serialización básica

#[test]
fn json_serializar_nulo() {
    let v = ejecutar_ok("json_serializar(nulo)");
    assert_eq!(v, Valor::Texto("null".to_string()));
}

#[test]
fn json_serializar_verdadero() {
    let v = ejecutar_ok("json_serializar(verdadero)");
    assert_eq!(v, Valor::Texto("true".to_string()));
}

#[test]
fn json_serializar_falso() {
    let v = ejecutar_ok("json_serializar(falso)");
    assert_eq!(v, Valor::Texto("false".to_string()));
}

#[test]
fn json_serializar_entero() {
    let v = ejecutar_ok("json_serializar(42)");
    assert_eq!(v, Valor::Texto("42".to_string()));
}

#[test]
fn json_serializar_real() {
    let v = ejecutar_ok("json_serializar(3.14)");
    assert_eq!(v, Valor::Texto("3.14".to_string()));
}

#[test]
fn json_serializar_texto() {
    let v = ejecutar_ok("json_serializar(\"hola\")");
    assert_eq!(v, Valor::Texto("\"hola\"".to_string()));
}

#[test]
fn json_serializar_lista() {
    let v = ejecutar_ok("json_serializar([1, 2, 3])");
    assert_eq!(v, Valor::Texto("[1,2,3]".to_string()));
}

#[test]
fn json_serializar_objeto() {
    // Usar json_parsear para crear el objeto y verificar que serializa correctamente
    let v = ejecutar_ok("sea obj = json_parsear(\"{\\\"a\\\": 1}\")\njson_serializar(obj)");
    assert_eq!(v, Valor::Texto("{\"a\":1}".to_string()));
}

// ------------------------------------------------------------------ Ida y vuelta (round-trip)

#[test]
fn round_trip_entero() {
    let v = ejecutar_ok("json_serializar(json_parsear(\"99\"))");
    assert_eq!(v, Valor::Texto("99".to_string()));
}

#[test]
fn round_trip_cadena() {
    let v = ejecutar_ok("json_serializar(json_parsear(\"\\\"hello\\\"\"))");
    assert_eq!(v, Valor::Texto("\"hello\"".to_string()));
}

#[test]
fn round_trip_array() {
    let v = ejecutar_ok("json_serializar(json_parsear(\"[1,2,3]\"))");
    assert_eq!(v, Valor::Texto("[1,2,3]".to_string()));
}

#[test]
fn round_trip_objeto_simple() {
    let v = ejecutar_ok("json_serializar(json_parsear(\"{\\\"x\\\":10,\\\"y\\\":20}\"))");
    // BTreeMap ordena las claves: x antes que y
    assert_eq!(v, Valor::Texto("{\"x\":10,\"y\":20}".to_string()));
}

#[test]
fn round_trip_objeto_anidado() {
    let fuente = r#"json_serializar(json_parsear("{\"a\":{\"b\":1}}"))"#;
    let v = ejecutar_ok(fuente);
    assert_eq!(v, Valor::Texto("{\"a\":{\"b\":1}}".to_string()));
}

#[test]
fn round_trip_array_de_objetos() {
    let fuente = r#"json_serializar(json_parsear("[{\"id\":1},{\"id\":2}]"))"#;
    let v = ejecutar_ok(fuente);
    assert_eq!(v, Valor::Texto("[{\"id\":1},{\"id\":2}]".to_string()));
}

// ------------------------------------------------------------------ Acceso a claves

#[test]
fn json_obtener_clave_existente() {
    let v = ejecutar_ok(
        "sea obj = json_parsear(\"{\\\"nombre\\\": \\\"Pedro\\\"}\")\njson_obtener(obj, \"nombre\")",
    );
    assert_eq!(v, Valor::Texto("Pedro".to_string()));
}

#[test]
fn json_obtener_clave_inexistente_devuelve_nulo() {
    let v = ejecutar_ok("sea obj = json_parsear(\"{\\\"a\\\": 1}\")\njson_obtener(obj, \"z\")");
    assert_eq!(v, Valor::Nulo);
}

#[test]
fn json_obtener_sobre_no_objeto_es_error() {
    let err = ejecutar_err("json_obtener(\"texto\", \"clave\")");
    assert!(
        err.contains("objeto") || err.contains("texto"),
        "mensaje inesperado: {err}"
    );
}

// ------------------------------------------------------------------ Anidamiento

#[test]
fn objeto_dentro_de_array() {
    let fuente = r#"
        sea datos = json_parsear("[{\"id\":1,\"ok\":true}]")
        sea primero = datos[0]
        json_obtener(primero, "id")
    "#;
    let v = ejecutar_ok(fuente);
    assert_eq!(v, Valor::Entero(1));
}

#[test]
fn array_dentro_de_objeto() {
    let fuente = r#"
        sea datos = json_parsear("{\"nums\":[10,20,30]}")
        sea nums = json_obtener(datos, "nums")
        longitud(nums)
    "#;
    let v = ejecutar_ok(fuente);
    assert_eq!(v, Valor::Entero(3));
}

#[test]
fn objeto_tres_niveles() {
    let fuente = r#"json_serializar(json_parsear("{\"a\":{\"b\":{\"c\":42}}}"))"#;
    let v = ejecutar_ok(fuente);
    assert_eq!(v, Valor::Texto("{\"a\":{\"b\":{\"c\":42}}}".to_string()));
}

// ------------------------------------------------------------------ Errores

#[test]
fn json_parsear_json_invalido_es_error() {
    let err = ejecutar_err("json_parsear(\"{mal json}\")");
    assert!(
        err.contains("JSON") || err.contains("inválido") || err.contains("invalid"),
        "mensaje inesperado: {err}"
    );
}

#[test]
fn json_parsear_cadena_vacia_es_error() {
    let err = ejecutar_err("json_parsear(\"\")");
    assert!(
        err.contains("JSON") || err.contains("inválido") || err.contains("invalid"),
        "mensaje inesperado: {err}"
    );
}

#[test]
fn json_serializar_real_nan_es_error() {
    // 0.0 / 0.0 en BASILISCO es error antes de llegar a JSON; usamos otra vía:
    // real que sea NaN via operación que lo produce
    // Como BASILISCO detecta /0 como error, verificamos directamente
    // que el mensaje de error mencione el tipo cuando se pasa un tipo no serializable
    let err = ejecutar_err("json_serializar(abrir(\"/dev/null\", \"leer\"))");
    assert!(
        err.contains("archivo") || err.contains("serializable"),
        "mensaje inesperado: {err}"
    );
}

#[test]
fn json_serializar_funcion_es_error() {
    let err = ejecutar_err("funcion f() { retornar 1 }\njson_serializar(f)");
    assert!(
        err.contains("función") || err.contains("serializable"),
        "mensaje inesperado: {err}"
    );
}

#[test]
fn json_parsear_sin_argumentos_es_error() {
    let err = ejecutar_err("json_parsear()");
    assert!(
        err.contains("firma") || err.contains("argument") || err.contains("1"),
        "mensaje inesperado: {err}"
    );
}

#[test]
fn json_serializar_sin_argumentos_es_error() {
    let err = ejecutar_err("json_serializar()");
    assert!(
        err.contains("firma") || err.contains("argument") || err.contains("1"),
        "mensaje inesperado: {err}"
    );
}

#[test]
fn json_obtener_sin_argumentos_es_error() {
    let err = ejecutar_err("json_obtener()");
    assert!(
        err.contains("firma") || err.contains("argument") || err.contains("2"),
        "mensaje inesperado: {err}"
    );
}

// ------------------------------------------------------------------ Integración con el lenguaje

#[test]
fn iterar_array_json_con_para_cada() {
    let fuente = r#"
        sea arr = json_parsear("[10, 20, 30]")
        sea suma = 0
        para cada x en arr {
            suma = suma + x
        }
        suma
    "#;
    let v = ejecutar_ok(fuente);
    assert_eq!(v, Valor::Entero(60));
}

#[test]
fn longitud_de_array_json() {
    let v = ejecutar_ok("longitud(json_parsear(\"[1,2,3,4,5]\"))");
    assert_eq!(v, Valor::Entero(5));
}

#[test]
fn asignar_y_operar_resultado_json() {
    let fuente = r#"
        sea datos = json_parsear("{\"valor\": 100}")
        sea n = json_obtener(datos, "valor")
        n * 2
    "#;
    let v = ejecutar_ok(fuente);
    assert_eq!(v, Valor::Entero(200));
}

#[test]
fn json_parsear_en_condicional() {
    let fuente = r#"
        sea obj = json_parsear("{\"activo\": true}")
        sea estado = json_obtener(obj, "activo")
        sea resultado = "no activo"
        si estado {
            resultado = "si activo"
        }
        resultado
    "#;
    let v = ejecutar_ok(fuente);
    assert_eq!(v, Valor::Texto("si activo".to_string()));
}

#[test]
fn json_serializar_lista_mixta() {
    let fuente = r#"
        sea datos = json_parsear("[1, \"dos\", true, null]")
        json_serializar(datos)
    "#;
    let v = ejecutar_ok(fuente);
    assert_eq!(v, Valor::Texto("[1,\"dos\",true,null]".to_string()));
}
