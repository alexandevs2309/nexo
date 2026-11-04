//! Backend nativo (prototipo) de BASILISCO.
//!
//! Genera código Rust a partir de MIR (subconjunto: funciones cuyos
//! parámetros, locales y retorno son `entero`/`lógico`). El código generado
//! se compila con `rustc` y se ejecuta; la salida se compara con la VM para
//! verificar paridad observable (Etapa V — investigación/prototipo).

use crate::mir::{
    MirBinOp, MirConst, MirFunction, MirOperand, MirPrograma, MirRvalue, MirStmt, MirTerminator,
    MirUnOp,
};
use crate::semantico::TipoSem;

/// Genera únicamente las funciones `bsk_*` (sin punto de entrada), reutilizable
/// por los backends nativo y WASM.
pub fn generar_funciones(programa: &MirPrograma) -> String {
    let mut out = String::new();
    for f in &programa.funciones {
        if !funcion_soportada(f) {
            out.push_str(&format!(
                "// función '{}' omitida: tipos no soportados en el prototipo\n\n",
                f.nombre
            ));
            continue;
        }
        generar_funcion(&mut out, f);
        out.push('\n');
    }
    out
}

/// Genera código Rust (con función `main` de entrada) a partir de un programa MIR.
/// Funciones no soportadas (tipos no enteros) se omiten con un comentario.
pub fn generar_rust(programa: &MirPrograma) -> String {
    let mut out = String::new();
    out.push_str("// Generado por BASILISCO backend nativo (prototipo)\n");
    out.push_str("#![allow(unused)]\n\n");
    out.push_str(&generar_funciones(programa));
    // Punto de entrada Rust.
    out.push_str("fn main() {\n");
    if programa
        .funciones
        .iter()
        .any(|f| f.nombre == "main" && funcion_soportada(f))
    {
        out.push_str("    println!(\"{}\", bsk_main());\n");
    } else {
        out.push_str("    println!(\"0\");\n");
    }
    out.push_str("}\n");
    out
}

fn funcion_soportada(f: &MirFunction) -> bool {
    f.params.iter().all(|(_, t)| es_entero(t))
        && es_entero(&f.retorno)
        && f.locals.iter().all(es_entero)
}

fn es_entero(t: &TipoSem) -> bool {
    matches!(t, TipoSem::Entero | TipoSem::Logico | TipoSem::Desconocido)
}

fn generar_funcion(out: &mut String, f: &MirFunction) {
    let params: Vec<String> = f
        .params
        .iter()
        .enumerate()
        .map(|(i, (_, t))| format!("p{}: {}", i, tipo_rust(t)))
        .collect();
    out.push_str(&format!(
        "fn bsk_{}({}) -> i64 {{\n",
        f.nombre,
        params.join(", ")
    ));
    for (i, t) in f.locals.iter().enumerate() {
        out.push_str(&format!(
            "    let mut l{}: {} = {};\n",
            i,
            tipo_rust(t),
            valor_por_defecto(t)
        ));
    }
    for (i, (local, _)) in f.params.iter().enumerate() {
        out.push_str(&format!("    l{} = p{};\n", local.0, i));
    }
    out.push_str("    let mut __bloque: usize = 0;\n");
    out.push_str("    let mut __ret: i64 = 0;\n");
    out.push_str("    loop {\n");
    out.push_str("        match __bloque {\n");
    for b in &f.bloques {
        out.push_str(&format!("            {} => {{\n", b.id.0));
        for stmt in &b.stmts {
            generar_stmt(out, stmt, 16);
        }
        generar_terminator(out, &b.terminator);
        out.push_str("            }\n");
    }
    out.push_str("            _ => break,\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("    __ret\n");
    out.push_str("}\n");
}

fn tipo_rust(t: &TipoSem) -> &'static str {
    match t {
        TipoSem::Entero | TipoSem::Logico | TipoSem::Desconocido => "i64",
        _ => "i64",
    }
}

fn valor_por_defecto(t: &TipoSem) -> &'static str {
    match t {
        TipoSem::Logico => "0",
        _ => "0",
    }
}

fn generar_stmt(out: &mut String, stmt: &MirStmt, indent: usize) {
    let pad = " ".repeat(indent);
    match stmt {
        MirStmt::Asignacion { destino, valor } => {
            let rhs = generar_rvalue(valor);
            if destino.projection.is_empty() {
                out.push_str(&format!("{}l{} = {};\n", pad, destino.local.0, rhs));
            } else {
                out.push_str(&format!("{}let _ = {};\n", pad, rhs));
            }
        }
        MirStmt::Llamada {
            destino,
            callee,
            argumentos,
        } => {
            let args: Vec<String> = argumentos.iter().map(generar_operando).collect();
            let call = format!("bsk_{}({})", callee, args.join(", "));
            if let Some(d) = destino {
                if d.projection.is_empty() {
                    out.push_str(&format!("{}l{} = {};\n", pad, d.local.0, call));
                } else {
                    out.push_str(&format!("{}let _ = {};\n", pad, call));
                }
            } else {
                out.push_str(&format!("{}let _ = {};\n", pad, call));
            }
        }
        MirStmt::LlamadaIndirecta { .. } => {
            out.push_str(&format!("{}// llamada indirecta no soportada\n", pad));
        }
        MirStmt::EntrarTry { .. } => {
            out.push_str(&format!("{}// intentar no soportado\n", pad));
        }
        MirStmt::SalirTry => {}
        MirStmt::AsignacionGlobal { .. } => {
            out.push_str(&format!("{}// global no soportado\n", pad));
        }
        MirStmt::Nop => {}
    }
}

fn generar_terminator(out: &mut String, t: &MirTerminator) {
    let pad = "                ";
    match t {
        MirTerminator::Goto(b) => {
            out.push_str(&format!("{}__bloque = {};\n{}continue;\n", pad, b.0, pad));
        }
        MirTerminator::SwitchInt {
            discriminante,
            casos,
            por_defecto,
        } => {
            let d = generar_operando(discriminante);
            let mut first = true;
            for (v, b) in casos {
                let kw = if first { "if" } else { "else if" };
                out.push_str(&format!(
                    "{}{} {} == {} {{ __bloque = {}; continue; }}\n",
                    pad, kw, d, v, b.0
                ));
                first = false;
            }
            out.push_str(&format!(
                "{}else {{ __bloque = {}; continue; }}\n",
                pad, por_defecto.0
            ));
        }
        MirTerminator::Return(op) => {
            let v = op
                .as_ref()
                .map(generar_operando)
                .unwrap_or_else(|| "0".to_string());
            out.push_str(&format!("{}__ret = {};\n{}break;\n", pad, v, pad));
        }
        MirTerminator::Invoke { .. } => {
            out.push_str(&format!("{}__ret = 0;\n{}break;\n", pad, pad));
        }
        MirTerminator::Unreachable => {
            out.push_str(&format!("{}__ret = 0;\n{}break;\n", pad, pad));
        }
    }
}

fn generar_operando(op: &MirOperand) -> String {
    match op {
        MirOperand::Copy(l) => format!("l{}", l.0),
        MirOperand::Move(l) => format!("l{}", l.0),
        MirOperand::Global(_) => "0".to_string(),
        MirOperand::Const(c) => match c {
            MirConst::Entero(n) => n.to_string(),
            MirConst::Logico(b) => {
                if *b {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            MirConst::Real(n) => format!("{}", *n as i64),
            MirConst::Decimal(n) => format!("{}", *n),
            MirConst::Texto(_) => "0".to_string(),
            MirConst::Nulo => "0".to_string(),
        },
    }
}

fn generar_rvalue(r: &MirRvalue) -> String {
    match r {
        MirRvalue::Use(op) => generar_operando(op),
        MirRvalue::BinOp { op, lhs, rhs } => {
            let l = generar_operando(lhs);
            let r = generar_operando(rhs);
            match op {
                MirBinOp::Suma => format!("({} + {})", l, r),
                MirBinOp::Resta => format!("({} - {})", l, r),
                MirBinOp::Multiplicacion => format!("({} * {})", l, r),
                MirBinOp::Division => format!("({} / {})", l, r),
                MirBinOp::Modulo => format!("({} % {})", l, r),
                MirBinOp::Igual => format!("(if {} == {} {{ 1 }} else {{ 0 }})", l, r),
                MirBinOp::Distinto => format!("(if {} != {} {{ 1 }} else {{ 0 }})", l, r),
                MirBinOp::Menor => format!("(if {} < {} {{ 1 }} else {{ 0 }})", l, r),
                MirBinOp::MenorIgual => format!("(if {} <= {} {{ 1 }} else {{ 0 }})", l, r),
                MirBinOp::Mayor => format!("(if {} > {} {{ 1 }} else {{ 0 }})", l, r),
                MirBinOp::MayorIgual => format!("(if {} >= {} {{ 1 }} else {{ 0 }})", l, r),
                MirBinOp::Y => format!("(if {} != 0 && {} != 0 {{ 1 }} else {{ 0 }})", l, r),
                MirBinOp::O => format!("(if {} != 0 || {} != 0 {{ 1 }} else {{ 0 }})", l, r),
            }
        }
        MirRvalue::UnOp { op, operand } => {
            let v = generar_operando(operand);
            match op {
                MirUnOp::Negacion => format!("(-{})", v),
                MirUnOp::No => format!("(if {} == 0 {{ 1 }} else {{ 0 }})", v),
            }
        }
        MirRvalue::Agregado { .. } => "0".to_string(),
        MirRvalue::Indice { .. } => "0".to_string(),
        MirRvalue::Campo { .. } => "0".to_string(),
        MirRvalue::Longitud { .. } => "0".to_string(),
        MirRvalue::Rango { .. } => "0".to_string(),
        MirRvalue::Clausura { .. } => "0".to_string(),
        MirRvalue::CrearCelda { .. } => "0".to_string(),
        MirRvalue::LeerCelda { .. } => "0".to_string(),
        MirRvalue::ObjetoModulo { .. } => "0".to_string(),
    }
}
