use nexo::{evaluar, Entorno, Lexer, Parser, Valor};

fn principal(fuente: &str) -> Result<Valor, String> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| e.to_string())?;
    let expr = Parser::nuevo(tokens)
        .parsear_expresion()
        .map_err(|e| e.to_string())?;
    evaluar(&expr).map_err(|e| e.to_string())
}

fn principal_programa(fuente: &str) -> Result<Option<Valor>, String> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| e.to_string())?;
    let sentencias = Parser::nuevo(tokens)
        .parsear_programa()
        .map_err(|e| e.to_string())?;
    Entorno::nueva()
        .ejecutar_programa(&sentencias)
        .map_err(|e| e.to_string())
}

fn main() {


    println!("Nexo — prototipo v{}", env!("CARGO_PKG_VERSION"));

    let casos = [
        "10 + 5",
        "0.1 + 0.2",
        "2 + 3.5",
        "1e10",
        "7 / 2",
        "10.0 / 4.0",
        "-7 % 3",
        "10 / 0",
        "9223372036854775807 + 1",
    ];

    println!("\nEvaluación de expresiones:");
    for caso in casos {
        match principal(caso) {
            Ok(v) => println!("  {caso:<24} = {v}"),
            Err(e) => println!("  {caso:<24} → {e}"),
        }
    }

    println!("\nPrograma con variables (ADR-0006):");
    let programa = "sea precio = 10.5\nsea total = precio * 3\ntotal = total - 1.5\ntotal";
    match principal_programa(programa) {
        Ok(Some(v)) => println!("  total después de la modificación = {v}"),
        Ok(None) => println!("  (el programa no produjo valor)"),
        Err(e) => println!("  → {e}"),
    }
    let con_error = "sea x = 5\nx + y";
    match principal_programa(con_error) {
        Ok(v) => println!("  {con_error:?} → {v:?}"),
        Err(e) => println!("  \"sea x = 5 / x + y\" → {e}"),
    }

    println!("\nControl de flujo y lógicos (ADR-0007):");
    let control = "\
sea total = 0
sea contador = 0
mientras contador < 5 {
  contador = contador + 1
  si contador % 2 == 0 {
    total = total + contador
  } sino {
    total = total + 1
  }
}
total";
    match principal_programa(control) {
        Ok(Some(v)) => println!("  total acumulado = {v}"),
        Ok(None) => println!("  (el programa no produjo valor)"),
        Err(e) => println!("  → {e}"),
    }

    let lógicos = ["1 < 2", "2 <= 2", "3 > 4", "verdadero y falso", "no falso", "1 == 1 o 2 == 3"];
    for caso in lógicos {
        match principal(caso) {
            Ok(v) => println!("  {caso:<24} = {v}"),
            Err(e) => println!("  {caso:<24} → {e}"),
        }
    }

    let condicion_invalida = "si 5 {\n  1\n}";
    match principal_programa(condicion_invalida) {
        Ok(v) => println!("  {condicion_invalida:?} → {v:?}"),
        Err(e) => println!("  \"si 5 {{ ... }}\" → {e}"),
    }
}

