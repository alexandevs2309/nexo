//! Helper para evaluar expresiones a través del motor VM, espejo de `evaluar`
//! del Tree-Walk. Uso: `vm_eval("2 + 3")` devuelve `Ok(Valor::Entero(5))`.

use basilisco::{ContextoSemantico, Lexer, Parser, Valor, hir, mir, vm};

pub fn vm_eval(fuente: &str) -> Result<Valor, String> {
    // Envolver la expresión en un main que la retorna.
    let src = format!("funcion main() {{ retornar {} }}", fuente);
    let tokens = Lexer::nuevo(&src)
        .tokenizar()
        .map_err(|e| format!("léxico: {e}"))?;
    let ss = Parser::nuevo(tokens)
        .parsear_programa()
        .map_err(|e| format!("sintaxis: {e}"))?;
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).map_err(|e| format!("HIR: {:?}", e))?;
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    let idx = maquina
        .indice_de("main")
        .ok_or_else(|| "sin main".to_string())?;
    maquina
        .ejecutar_indice(idx, &[])
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "sin valor de retorno".to_string())
}

pub fn vm_ok(fuente: &str) -> Valor {
    vm_eval(fuente).unwrap_or_else(|e| panic!("{fuente:?} → error inesperado: {e}"))
}

pub fn vm_err(fuente: &str) -> String {
    vm_eval(fuente).expect_err(&format!("{fuente:?} → esperaba error, obtuvo valor"))
}

/// Ejecuta un programa completo vía la VM y devuelve el valor producido.
#[allow(dead_code)]
pub fn vm_program(fuente: &str) -> Result<Option<Valor>, String> {
    let tokens = Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| format!("léxico: {e}"))?;
    let ss = Parser::nuevo(tokens)
        .parsear_programa()
        .map_err(|e| format!("sintaxis: {e}"))?;
    let mut ctx = ContextoSemantico::nuevo();
    let _ = ctx.analizar_programa(&ss);
    let hir = hir::lower_programa(&ss, &ctx).map_err(|e| format!("HIR: {:?}", e))?;
    let mir = mir::lower_programa(&hir);
    let mut maquina = vm::MaquinaVirtual::nuevo(mir);
    maquina.registrar_stdlib();
    // Busca un main de usuario o usa el main sintético.
    let Some(idx) = maquina.indice_de("main") else {
        return Err("el programa no define `funcion main()`".to_string());
    };
    maquina.ejecutar_indice(idx, &[]).map_err(|e| e.to_string())
}

/// Valor de un programa vía VM (panica si falla).
#[allow(dead_code)]
pub fn vm_ok_programa(fuente: &str) -> Option<Valor> {
    vm_program(fuente).unwrap_or_else(|e| panic!("{fuente:?} → error VM inesperado: {e}"))
}

/// Error de un programa vía VM (panica si no hay error).
pub fn assert_valor_parity(fuente: &str) {
    let tw = basilisco::Entorno::nueva()
        .ejecutar_programa(
            &basilisco::Parser::nuevo(basilisco::Lexer::nuevo(fuente).tokenizar().unwrap())
                .parsear_programa()
                .unwrap(),
        )
        .map(|v| v.unwrap_or(Valor::Nulo))
        .map_err(|e| e.mensaje);
    let vm = vm_program(fuente).map(|v| v.unwrap_or(Valor::Nulo));
    match (tw, vm) {
        (Ok(a), Ok(b)) => assert_eq!(a, b, "\nFUENTE:\n{fuente}\nTW: {a:?}\nVM: {b:?}"),
        (Err(_), Err(_)) => { /* ambos errores */ }
        (Err(a), Ok(b)) => panic!("\nFUENTE:\n{fuente}\nTW error: {a}\nVM OK: {b:?}"),
        (Ok(a), Err(b)) => panic!("\nFUENTE:\n{fuente}\nTW OK: {a:?}\nVM error: {b}"),
    }
}
