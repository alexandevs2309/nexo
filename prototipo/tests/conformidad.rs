//! Suite de conformidad del incremento 2, organizada por las reglas
//! aprobadas del ADR-0005 (sistema numérico).
//!
//! Cada bloque cita la sección del ADR que verifica. Casos límite con
//! resultado esperado explícito.

use basilisco::{Lexer, Parser, Valor, evaluar};

fn ejecutar(fuente: &str) -> Result<Valor, String> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| format!("léxico: {e}"))?;
    let expr = Parser::nuevo(tokens)
        .parsear_expresion()
        .map_err(|e| format!("sintaxis: {e}"))?;
    evaluar(&expr).map_err(|e| e.mensaje)
}

fn ok(fuente: &str) -> Valor {
    ejecutar(fuente).unwrap_or_else(|e| panic!("{fuente:?} → error inesperado: {e}"))
}

fn err(fuente: &str) -> String {
    ejecutar(fuente).expect_err(&format!("{fuente:?} → esperaba error, obtuvo valor"))
}

// ================================================================ §1.1 entero

#[test]
fn s11_limites_literales_enteros() {
    assert_eq!(ok("9223372036854775807"), Valor::Entero(i64::MAX));
    assert_eq!(ok("-9223372036854775808"), Valor::Entero(i64::MIN));
    assert_eq!(ok("9223372036854775807 - 1"), Valor::Entero(i64::MAX - 1));
    assert_eq!(ok("-9223372036854775808 + 1"), Valor::Entero(i64::MIN + 1));
}

#[test]
fn s11_desbordamiento_es_error_nunca_envuelve() {
    // PN-INT-OVF: envolver a MIN/MAX sería silenciar el error (prohibido).
    assert_eq!(err("9223372036854775807 + 1"), "desbordamiento entero");
    assert_eq!(err("-9223372036854775808 - 1"), "desbordamiento entero");
    assert_eq!(err("9223372036854775807 * 2"), "desbordamiento entero");
    assert_eq!(err("-9223372036854775808 * 2"), "desbordamiento entero");
    assert_eq!(err("-9223372036854775808 / -1"), "desbordamiento entero");
}

#[test]
fn s11_min_mod_menos_uno_es_cero() {
    // El resto cabe en i64 aunque el cociente no.
    assert_eq!(ok("-9223372036854775808 % -1"), Valor::Entero(0));
}

#[test]
fn s11_division_entera_trunca_hacia_cero() {
    assert_eq!(ok("7 / 2"), Valor::Entero(3));
    assert_eq!(ok("-7 / 2"), Valor::Entero(-3));
    assert_eq!(ok("7 / -2"), Valor::Entero(-3));
    assert_eq!(ok("-7 / -2"), Valor::Entero(3));
}

#[test]
fn s11_division_y_modulo_por_cero_son_error() {
    assert_eq!(err("10 / 0"), "división entre cero");
    assert_eq!(err("-10 / 0"), "división entre cero");
    assert_eq!(err("10 % 0"), "división entre cero");
    assert_eq!(err("0 / 0"), "división entre cero");
    assert_eq!(err("0 % 0"), "división entre cero");
}

#[test]
fn s11_identidad_division_modulo_los_ocho_signos() {
    // a == (a / b) * b + a % b, para todo signo de a y b.
    for a in [7, -7] {
        for b in [3, -3] {
            let r = ok(&format!("({a} / {b}) * {b} + {a} % {b}"));
            assert_eq!(r, Valor::Entero(a), "identidad para a={a}, b={b}");
        }
    }
}

// ================================================================ §1.2 decimal

#[test]
fn s12_suma_exacta_sin_error_flotante() {
    // 0.1 + 0.2 es exactamente 0.3 (nunca 0.30000000000000004).
    assert_eq!(ok("0.1 + 0.2"), Valor::Decimal(300_000_000));
    assert_eq!(ok("0.10 + 0.10 + 0.10"), Valor::Decimal(300_000_000));
    assert_eq!(ok("0.1 + 0.2 + 0.3"), Valor::Decimal(600_000_000));
}

#[test]
fn s12_division_redondea_half_even() {
    // 2/3 = 0.666…667 (residuo > mitad → aleja de cero).
    assert_eq!(ok("2.0 / 3.0").to_string(), "0.666666667");
    // 1/3 = 0.333…333 (residuo < mitad → trunca).
    assert_eq!(ok("1.0 / 3.0").to_string(), "0.333333333");
    // 4/3 = 1.333…333 (residuo < mitad → trunca).
    assert_eq!(ok("4.0 / 3.0").to_string(), "1.333333333");
    // Signo: aleja de cero también en negativo.
    assert_eq!(ok("-2.0 / 3.0").to_string(), "-0.666666667");
    assert_eq!(ok("2.0 / -3.0").to_string(), "-0.666666667");
}

#[test]
fn s12_division_empate_exacto_half_even() {
    // 1/1024 = 0.0009765625 → mantisa 976562.5 → cociente PAR → no sube.
    assert_eq!(ok("0.000000001 / 0.000001024").to_string(), "0.000976562");
    // 3/1024 = 0.0029296875 → mantisa 2929687.5 → cociente IMPAR → sube.
    assert_eq!(ok("0.000000003 / 0.000001024").to_string(), "0.002929688");
}

#[test]
fn s12_multiplicacion_redondea_half_even() {
    // 0.000000003 * 0.5 = 0.0000000015 → 1.5 → impar → sube a 2.
    assert_eq!(ok("0.000000003 * 0.5").to_string(), "0.000000002");
    // 0.000000005 * 0.5 = 0.0000000025 → 2.5 → par → se queda en 2.
    assert_eq!(ok("0.000000005 * 0.5").to_string(), "0.000000002");
}

#[test]
fn s12_exceso_de_precision_en_literal_redondea_half_even() {
    // 10º dígito 5 exacto, cociente impar → sube.
    assert_eq!(ok("1.0000000015").to_string(), "1.000000002");
    // 10º dígito 5 exacto, cociente par → no sube (el display muestra al menos un decimal).
    assert_eq!(ok("1.0000000005").to_string(), "1.0");
    // Dígitos extra no-cero (sticky) → siempre sube.
    assert_eq!(ok("1.00000000150001").to_string(), "1.000000002");
    // 10º dígito 4 → trunca.
    assert_eq!(ok("1.0000000014999").to_string(), "1.000000001");
    // Negativo: redondea en magnitud.
    assert_eq!(ok("-1.0000000015").to_string(), "-1.000000002");
}

#[test]
fn s12_limites_de_rango_decimal() {
    // Máximo literal válido: 19 enteros + 9 decimales (28 dígitos en total).
    assert_eq!(
        ok("9999999999999999999.999999999").to_string(),
        "9999999999999999999.999999999"
    );
    assert_eq!(
        ok("9999999999999999999.5").to_string(),
        "9999999999999999999.5"
    );
    // 20 dígitos enteros → exceso de rango (se rechaza al clasificar el literal).
    assert!(err("99999999999999999999.0").contains("exceso de rango decimal"));
    // Alcance del límite por suma (no solo en el literal).
    assert_eq!(
        err("9999999999999999999.5 + 0.5"),
        "exceso de rango decimal"
    );
    // Alcance del límite por resta hacia el extremo negativo.
    assert_eq!(
        err("-9999999999999999999.5 - 0.5"),
        "exceso de rango decimal"
    );
}

#[test]
fn s12_desbordamiento_i128_intermedio_en_multiplicacion() {
    // (10^28-1)^2 ≈ 10^56 desborda i128 como intermedio, y el resultado
    // real (≈10^47 en mantisa) excede DECIMAL(28,9) → error, nunca wrap.
    assert_eq!(
        err("9999999999999999999.999999999 * 9999999999999999999.999999999"),
        "exceso de rango decimal"
    );
    // Operando grande con resultado válido: el intermedio i128 (10^37) sí cabe.
    let r = ok("9999999999999999999.9 * 1.0");
    assert_eq!(r.to_string(), "9999999999999999999.9");
    // Redondeo half-even en multiplicación con el máximo posible:
    // mantisa (10^28-1)·0.5 = 5·10^27 - 0.5 → empate con cociente impar → sube.
    let r = ok("9999999999999999999.999999999 * 0.5");
    assert_eq!(r.to_string(), "5000000000000000000.0");
    // El límite exacto por multiplicación: el resultado excede DECIMAL(28,9).
    assert_eq!(
        err("9999999999999999999.999999999 * 1.000000001"),
        "exceso de rango decimal"
    );
}

#[test]
fn s12_desbordamiento_i128_intermedio_en_suma_y_resta() {
    // Suma de dos máximos: excede rango (no i128, pero sí DECIMAL(28,9)).
    assert_eq!(
        err("9999999999999999999.999999999 + 9999999999999999999.999999999"),
        "exceso de rango decimal"
    );
    // Resta cruzada de extremos opuestos.
    assert_eq!(
        err("-9999999999999999999.999999999 - 9999999999999999999.999999999"),
        "exceso de rango decimal"
    );
    // Límite exacto alcanzable por suma: mantisa 10^28-1 (el máximo).
    assert_eq!(
        ok("9999999999999999999.999999998 + 0.000000001").to_string(),
        "9999999999999999999.999999999"
    );
    // Límite exacto alcanzable por resta hacia el extremo negativo.
    assert_eq!(
        ok("0 - 9999999999999999999.999999999").to_string(),
        "-9999999999999999999.999999999"
    );
    // Un paso más allá del extremo negativo.
    assert_eq!(
        err("-9999999999999999999.999999999 - 0.000000001"),
        "exceso de rango decimal"
    );
}

#[test]
fn s12_division_por_cero_en_todos_los_combinados() {
    assert_eq!(err("1.0 / 0.0"), "división entre cero");
    assert_eq!(err("1.0 / 0"), "división entre cero");
    assert_eq!(err("1 / 0.0"), "división entre cero");
    assert_eq!(err("0.0 / 0.0"), "división entre cero");
}

#[test]
fn s12_division_decimal_con_entero_como_divisor() {
    // 10.0 / 4 = 2.5 (el entero 4 se convierte a decimal sin pérdida).
    assert_eq!(ok("10.0 / 4").to_string(), "2.5");
    // 10 / 4.0 = 2.5 (el entero 10 se convierte a decimal).
    assert_eq!(ok("10 / 4.0").to_string(), "2.5");
}

#[test]
fn s12_modulo_sobre_decimal_es_error_de_tipos() {
    // % es operador entero; sobre decimal exige conversión explícita (§1.4).
    assert_eq!(err("4.5 % 2"), "conversión ilegal");
    assert_eq!(err("4 % 2.0"), "conversión ilegal");
    assert_eq!(err("4.5 % 2.0"), "conversión ilegal");
}

// ================================================================ §1.3 real

#[test]
fn s13_literales_reales() {
    assert_eq!(ok("1e10"), Valor::Real(1e10));
    assert_eq!(ok("2.5e-3"), Valor::Real(2.5e-3));
    assert_eq!(ok("1E10"), Valor::Real(1e10));
    assert_eq!(ok("-1e10"), Valor::Real(-1e10));
    assert_eq!(ok("3.5e0"), Valor::Real(3.5));
}

#[test]
fn s13_division_por_cero_es_error_no_inf_ni_nan() {
    assert_eq!(err("1.0e0 / 0.0e0"), "división entre cero");
    assert_eq!(err("1.0e0 / 0"), "división entre cero");
    assert_eq!(err("1 / 0.0e0"), "división entre cero");
    assert_eq!(err("0.0e0 / 0.0e0"), "división entre cero");
}

#[test]
fn s13_infinito_por_desbordamiento_de_magnitud_es_admitido() {
    // §1.3: el tipo admite +inf que nace por desbordamiento de magnitud.
    let r = ok("1e308 * 10");
    let Valor::Real(x) = r else {
        panic!("esperaba real")
    };
    assert!(x.is_infinite() && x > 0.0, "esperaba +inf, obtuvo {x}");
    // Operación posterior sobre infinito sigue siendo valor real (no error).
    let r = ok("1e308 * 10 + 1e0");
    let Valor::Real(x) = r else {
        panic!("esperaba real")
    };
    assert!(x.is_infinite());
}

#[test]
fn s13_real_con_entero_usa_conversion_implicita() {
    assert_eq!(ok("2 + 3.5e0"), Valor::Real(5.5));
    assert_eq!(ok("2.5e0 * 4"), Valor::Real(10.0));
    assert_eq!(ok("10 - 2.5e0"), Valor::Real(7.5));
}

// ================================================================ §1.4 conversiones

#[test]
fn s14_solo_conversiones_sin_perdidas_son_implicitas() {
    // entero → decimal: implícita.
    assert_eq!(ok("2 + 3.5"), Valor::Decimal(5_500_000_000));
    // entero → real: implícita.
    assert_eq!(ok("2 + 3.5e0"), Valor::Real(5.5));
    // decimal → real: error en operador (exige conversión explícita).
    assert_eq!(err("1.5 + 2.0e0"), "conversión ilegal");
    assert_eq!(err("1.5 - 2.0e0"), "conversión ilegal");
    assert_eq!(err("1.5 * 2.0e0"), "conversión ilegal");
    assert_eq!(err("1.5 / 2.0e0"), "conversión ilegal");
    // Orden inverso: también error.
    assert_eq!(err("2.0e0 + 1.5"), "conversión ilegal");
    // En resta y multiplicación mixtas.
    assert_eq!(err("5 - 2.5e0 + 1.5"), "conversión ilegal");
    assert_eq!(err("0.5 * 2.0e0"), "conversión ilegal");
}

// ================================================================ §1.5 literales

#[test]
fn s15_clasificacion_de_literales_sin_sufijo() {
    let casos = [
        ("42", "entero"),
        ("42.5", "decimal"),
        ("1e10", "real"),
        ("-42", "entero"),
        ("-42.5", "decimal"),
        ("-1e10", "real"),
        ("0", "entero"),
        ("0.0", "decimal"),
        ("0e0", "real"),
    ];
    for (fuente, tipo) in casos {
        let v = ok(fuente);
        assert_eq!(v.nombre_tipo(), tipo, "literal {fuente:?}");
    }
}

#[test]
fn s15_literal_entero_en_el_limite_de_i64() {
    assert_eq!(ok("9223372036854775807").nombre_tipo(), "entero");
    assert_eq!(ok("-9223372036854775808").nombre_tipo(), "entero");
}

// ================================================================ §1.6 errores

#[test]
fn s16_los_cuatro_motivos_tienen_mensaje_exacto() {
    assert_eq!(err("9223372036854775807 + 1"), "desbordamiento entero");
    assert_eq!(err("10 / 0"), "división entre cero");
    // El literal fuera de rango se rechaza en la fase sintáctica; el motivo es el mismo.
    assert!(err("99999999999999999999.0").contains("exceso de rango decimal"));
    assert_eq!(
        err("9999999999999999999.5 + 0.5"),
        "exceso de rango decimal"
    );
    assert_eq!(err("1.5 + 2.0e0"), "conversión ilegal");
}

#[test]
fn s16_ningun_error_se_sustituye_por_valor_por_defecto() {
    let fallidos = [
        "9223372036854775807 + 1",
        "10 / 0",
        "1.0 / 0.0",
        "1.5 + 2.0e0",
        "9999999999999999999.5 + 0.5",
        "9999999999999999999.999999999 * 9999999999999999999.999999999",
    ];
    for fuente in fallidos {
        assert!(
            ejecutar(fuente).is_err(),
            "{fuente:?} devolvió un valor en lugar de error"
        );
    }
}

#[test]
fn s16_todo_error_de_ejecucion_informa_ubicacion() {
    let fallidos = [
        "9223372036854775807 + 1",
        "10 / 0",
        "1.5 + 2.0e0",
        "9999999999999999999.5 + 0.5",
    ];
    for fuente in fallidos {
        let tokens = Lexer::nuevo(fuente).tokenizar().expect("léxico");
        let expr = Parser::nuevo(tokens).parsear_expresion().expect("sintaxis");
        let e = basilisco::evaluar(&expr).expect_err("esperaba error");
        assert!(e.linea >= 1 && e.columna >= 1, "ubicación para {fuente:?}");
        let display = e.to_string();
        assert!(
            display.contains(&format!("[{}:{}]", e.linea, e.columna)),
            "display sin ubicación: {display}"
        );
    }
}

// ================================================================ Coherencia general

#[test]
fn coherencia_entero_decimal_en_misma_cadena() {
    // El mismo número escrito como entero y como decimal difiere en tipo,
    // pero su suma mixta usa la conversión implícita sin pérdida.
    let v = ok("2 + 3.5");
    assert_eq!(v, Valor::Decimal(5_500_000_000));
    assert_eq!(v.to_string(), "5.5");
    // Y el entero solo no se degrada a decimal.
    assert_eq!(ok("2 + 3"), Valor::Entero(5));
}

#[test]
fn precedencia_completa_del_nucleo() {
    // * / % por encima de + -, asociatividad izquierda, paréntesis anulan.
    assert_eq!(ok("2 + 3 * 4"), Valor::Entero(14));
    assert_eq!(ok("2 * 3 + 4"), Valor::Entero(10));
    assert_eq!(ok("10 - 3 - 2"), Valor::Entero(5));
    assert_eq!(ok("100 / 5 / 2"), Valor::Entero(10));
    assert_eq!(ok("7 + 6 % 4"), Valor::Entero(9));
    assert_eq!(ok("(2 + 3) * 4"), Valor::Entero(20));
    assert_eq!(ok("2 + 3 * 4 % 5"), Valor::Entero(4)); // 2 + ((3*4)%5) = 2+2
}
