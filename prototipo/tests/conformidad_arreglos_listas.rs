// Tests de conformidad para arreglos y listas (Fase C, Parte 2)

use basilisco::{Lexer, Parser, evaluador::Entorno};

fn run_program(fuente: &str) -> Result<Option<basilisco::Valor>, String> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| e.to_string())?;
    eprintln!("Tokens: {:?}", tokens);
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .map_err(|e| e.to_string())?;
    eprintln!("Sentencias parseadas: {:?}", sentencias);
    Entorno::nueva()
        .ejecutar_programa(&sentencias)
        .map_err(|e| e.to_string())
}

#[test]
fn arreglo_literal_tipado() {
    let r = run_program("var a: arreglo<entero, 3> = [1, 2, 3]\na").unwrap();
    match r {
        Some(basilisco::Valor::Arreglo(v)) => {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], basilisco::Valor::Entero(1));
            assert_eq!(v[1], basilisco::Valor::Entero(2));
            assert_eq!(v[2], basilisco::Valor::Entero(3));
        }
        _ => panic!("esperaba Arreglo"),
    }
}
