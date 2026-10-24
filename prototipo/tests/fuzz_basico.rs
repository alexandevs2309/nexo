//! Fuzzing determinista y acotado del lexer/parser, sin dependencias externas.
//! Detecta panics reproducibles; no sustituye fuzzing guiado por cobertura.

use basilisco::{Lexer, Parser};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn entradas_aleatorias_acotadas_no_provocan_panics_en_lexer_ni_parser() {
    const ALFABETO: &[char] = &[
        'a', 'Z', '_', 'ñ', '1', '9', '0', '.', 'e', '+', '-', '*', '/', '%', '=', '<', '>', '!',
        '(', ')', '{', '}', '[', ']', ',', ':', '"', '\\', '#', ' ', '\t', '\n', '\r', '¿', '🙂',
    ];
    let mut estado = 0xB451_15C0_u64;

    for _caso in 0..2_000 {
        estado = estado
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let longitud = (estado as usize) % 129;
        let mut fuente = String::with_capacity(longitud);
        for _ in 0..longitud {
            estado = estado
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            fuente.push(ALFABETO[(estado as usize) % ALFABETO.len()]);
        }

        let resultado = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(tokens) = Lexer::nuevo(&fuente).tokenizar() {
                let _ = Parser::nuevo(tokens).parsear_programa();
            }
        }));
        assert!(resultado.is_ok(), "panic al procesar entrada: {fuente:?}");
    }
}
