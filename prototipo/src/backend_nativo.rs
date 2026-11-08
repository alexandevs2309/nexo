//! Backend nativo (prototipo) de BASILISCO.
//!
//! Genera código Rust a partir de MIR. Soporta funciones cuyos parámetros,
//! locales y retorno son `entero`/`lógico` y (desde DT-06) `texto`:
//! concatenación texto+texto con `format!`, comparaciones y E/S (`mostrar`,
//! `mostrar_sin_salto`). Los tipos `Desconocido` se refinan con una inferencia
//! de tipos (punto fijo): primero los retornos de todas las funciones del
//! programa, luego los locales de cada función (incluidos los destinos de
//! llamada y asignaciones encadenadas).
//!
//! Paridad con la VM: la suma mixta texto/entero NO se convierte
//! silenciosamente (la VM la rechaza con error aritmético), así que las
//! funciones con `Suma` mixta se rechazan. Si `main` no es soportada, el
//! código generado contiene `compile_error!` — nunca una salida silenciosa.

use std::collections::HashMap;

use crate::mir::{
    MirBinOp, MirConst, MirFunction, MirOperand, MirPrograma, MirRvalue, MirStmt, MirTerminator,
    MirUnOp,
};
use crate::semantico::TipoSem;

/// Genera únicamente las funciones `bsk_*` (sin punto de entrada), reutilizable
/// por el backend nativo.
pub fn generar_funciones(programa: &MirPrograma) -> String {
    generar_funciones_con(programa, true)
}

/// Genera solo las funciones del subconjunto puro entero/lógico (sin texto ni
/// E/S), reutilizable por el backend WASM donde `String` no cruza el ABI C.
pub fn generar_funciones_enteras(programa: &MirPrograma) -> String {
    generar_funciones_con(programa, false)
}

fn generar_funciones_con(programa: &MirPrograma, permitir_texto: bool) -> String {
    let retornos = inferir_retornos_programa(programa);
    let mut out = String::new();
    for f in &programa.funciones {
        let ctx = inferir_ctx(f, &retornos);
        let soportada = funcion_soportada(f, &ctx, &retornos)
            && (permitir_texto || funcion_pura_enteros(f, &ctx));
        if !soportada {
            out.push_str(&format!(
                "// función '{}' omitida: tipos no soportados en el prototipo\n\n",
                f.nombre
            ));
            continue;
        }
        generar_funcion(&mut out, f, &ctx);
        out.push('\n');
    }
    out
}

/// Genera código Rust (con función `main` de entrada) a partir de un programa MIR.
/// Funciones no soportadas se omiten con un comentario.
pub fn generar_rust(programa: &MirPrograma) -> String {
    let retornos = inferir_retornos_programa(programa);
    let main_soportada = programa
        .funciones
        .iter()
        .any(|f| f.nombre == "main" && funcion_soportada(f, &inferir_ctx(f, &retornos), &retornos));
    // El retorno efectivo de `main`: si es `Desconocido` la función no retorna
    // valor (como en la VM, que devuelve nulo y el CLI no lo imprime).
    let main_retorno = programa
        .funciones
        .iter()
        .find(|f| f.nombre == "main")
        .map(|f| inferir_ctx(f, &retornos).retorno)
        .unwrap_or(TipoSem::Desconocido);
    let mut out = String::new();
    out.push_str("// Generado por BASILISCO backend nativo (prototipo)\n");
    out.push_str("#![allow(unused)]\n\n");
    // Si `main` no es soportada, fallar la compilación con un diagnóstico
    // explícito en lugar de generar una salida silenciosa incorrecta.
    if !main_soportada {
        out.push_str(
            "compile_error!(\"backend nativo: la función 'main' usa operaciones no soportadas \
             (colecciones, texto mezclado con enteros en +, excepciones, celdas, etc.)\");\n\n",
        );
    }
    out.push_str(&generar_funciones(programa));
    // Punto de entrada Rust.
    out.push_str("fn main() {\n");
    if main_soportada {
        if matches!(main_retorno, TipoSem::Desconocido) {
            out.push_str("    bsk_main();\n");
        } else {
            out.push_str("    println!(\"{}\", bsk_main());\n");
        }
    } else {
        out.push_str("    println!(\"0\");\n");
    }
    out.push_str("}\n");
    out
}

/// Tipos resueltos de una función: locales inferidos + retorno.
struct CtxFn {
    tipos: Vec<TipoSem>,
    retorno: TipoSem,
}

/// Mapa nombre → tipo de retorno de todas las funciones del programa
/// (declarado si está anotado, inferido si no).
type Retornos = HashMap<String, TipoSem>;

/// Infiere los retornos del programa con un punto fijo: los locales de cada
/// función dependen de los retornos de las funciones que llama, y los
/// retornos de los tipos de sus locales.
fn inferir_retornos_programa(programa: &MirPrograma) -> Retornos {
    let mut mapa: Retornos = programa
        .funciones
        .iter()
        .map(|f| (f.nombre.clone(), f.retorno.clone()))
        .collect();
    let mut cambio = true;
    while cambio {
        cambio = false;
        for f in &programa.funciones {
            let ctx = inferir_locales(f, &mapa);
            let r = inferir_retorno(f, &ctx, &mapa);
            if let Some(previo) = mapa.get_mut(&f.nombre)
                && matches!(*previo, TipoSem::Desconocido)
                && !matches!(r, TipoSem::Desconocido)
            {
                *previo = r;
                cambio = true;
            }
        }
    }
    mapa
}

/// Infiere los locales `Desconocido` de una función con un punto fijo sobre
/// asignaciones y destinos de llamada.
fn inferir_locales(f: &MirFunction, retornos: &Retornos) -> Vec<TipoSem> {
    let mut tipos = f.locals.clone();
    let mut cambio = true;
    while cambio {
        cambio = false;
        for b in &f.bloques {
            for s in &b.stmts {
                // Tipo inferido para esta sentencia (si aporta información).
                let destino_y_tipo = match s {
                    MirStmt::Asignacion { destino, valor } => {
                        tipo_rvalue(valor, &tipos).map(|t| (destino.local.0, t))
                    }
                    MirStmt::Llamada {
                        destino: Some(d),
                        callee,
                        ..
                    } => {
                        // `mostrar`/`mostrar_sin_salto` devuelven nulo: el
                        // destino queda entero (0), como en la VM.
                        let es_es = callee == "mostrar" || callee == "mostrar_sin_salto";
                        if es_es {
                            None
                        } else {
                            match retornos.get(callee) {
                                Some(t) if !matches!(t, TipoSem::Desconocido) => {
                                    Some((d.local.0, t.clone()))
                                }
                                _ => None,
                            }
                        }
                    }
                    _ => None,
                };
                if let Some((i, t)) = destino_y_tipo
                    && i < tipos.len()
                    && matches!(tipos[i], TipoSem::Desconocido)
                    && tipos[i] != t
                {
                    tipos[i] = t;
                    cambio = true;
                }
            }
        }
    }
    tipos
}

/// Infiere el retorno a partir de los `Return`: solo si todos los retornos con
/// valor coinciden en tipo y ningún camino retorna sin valor (`Return(None)`).
fn inferir_retorno(f: &MirFunction, tipos: &[TipoSem], _retornos: &Retornos) -> TipoSem {
    let mut inferido: Option<TipoSem> = None;
    for b in &f.bloques {
        match &b.terminator {
            MirTerminator::Return(Some(op)) => {
                let Some(t) = tipo_operando(op, tipos) else {
                    return TipoSem::Desconocido;
                };
                if let Some(previo) = &inferido
                    && *previo != t
                {
                    return TipoSem::Desconocido;
                }
                inferido = Some(t);
            }
            MirTerminator::Return(None) => return TipoSem::Desconocido,
            _ => {}
        }
    }
    inferido.unwrap_or(TipoSem::Desconocido)
}

/// Contexto completo de una función: locales inferidos + retorno (declarado o
/// inferido).
fn inferir_ctx(f: &MirFunction, retornos: &Retornos) -> CtxFn {
    let tipos = inferir_locales(f, retornos);
    let retorno = if matches!(f.retorno, TipoSem::Desconocido) {
        inferir_retorno(f, &tipos, retornos)
    } else {
        f.retorno.clone()
    };
    CtxFn { tipos, retorno }
}

fn tipo_rvalue(r: &MirRvalue, tipos: &[TipoSem]) -> Option<TipoSem> {
    match r {
        MirRvalue::Use(op) => tipo_operando(op, tipos),
        MirRvalue::BinOp { op, lhs, rhs } => {
            let lt = tipo_operando(lhs, tipos);
            let rt = tipo_operando(rhs, tipos);
            match op {
                MirBinOp::Suma => {
                    if matches!(lt, Some(TipoSem::Texto)) || matches!(rt, Some(TipoSem::Texto)) {
                        Some(TipoSem::Texto)
                    } else {
                        Some(TipoSem::Entero)
                    }
                }
                _ => Some(TipoSem::Entero),
            }
        }
        MirRvalue::UnOp { .. } => Some(TipoSem::Entero),
        _ => None,
    }
}

fn tipo_operando(op: &MirOperand, tipos: &[TipoSem]) -> Option<TipoSem> {
    match op {
        MirOperand::Const(c) => match c {
            MirConst::Entero(_) | MirConst::Logico(_) => Some(TipoSem::Entero),
            MirConst::Texto(_) => Some(TipoSem::Texto),
            _ => None,
        },
        MirOperand::Copy(l) | MirOperand::Move(l) => tipos
            .get(l.0)
            .cloned()
            .filter(|t| !matches!(t, TipoSem::Desconocido)),
        MirOperand::Global(_) => None,
    }
}

fn funcion_soportada(f: &MirFunction, ctx: &CtxFn, retornos: &Retornos) -> bool {
    f.params.iter().all(|(_, t)| es_soportado(t))
        && es_soportado(&ctx.retorno)
        && f.locals.iter().all(es_soportado)
        && ctx.tipos.iter().all(es_soportado)
        && llamadas_resueltas(f, retornos)
        && sumas_homogeneas(f, ctx)
        && f.bloques
            .iter()
            .all(|b| b.stmts.iter().all(stmt_soportado) && terminator_soportado(&b.terminator))
}

/// Toda llamada debe apuntar a una función del programa o a una nativa de E/S;
/// llamadas a nativas de la stdlib (longitud, agregar, …) no se generan aquí.
fn llamadas_resueltas(f: &MirFunction, retornos: &Retornos) -> bool {
    f.bloques.iter().all(|b| {
        b.stmts.iter().all(|s| match s {
            MirStmt::Llamada { callee, .. } => {
                retornos.contains_key(callee)
                    || callee == "mostrar"
                    || callee == "mostrar_sin_salto"
            }
            _ => true,
        })
    })
}

/// La suma mixta texto/entero es un error en la VM: aquí se rechaza la función
/// en el gate para no generar una conversión silenciosa.
fn sumas_homogeneas(f: &MirFunction, ctx: &CtxFn) -> bool {
    f.bloques.iter().all(|b| {
        b.stmts.iter().all(|s| match s {
            MirStmt::Asignacion {
                valor:
                    MirRvalue::BinOp {
                        op: MirBinOp::Suma,
                        lhs,
                        rhs,
                    },
                ..
            } => es_texto_operando(lhs, ctx) == es_texto_operando(rhs, ctx),
            _ => true,
        })
    })
}

fn es_soportado(t: &TipoSem) -> bool {
    matches!(
        t,
        TipoSem::Entero | TipoSem::Logico | TipoSem::Texto | TipoSem::Desconocido
    )
}

/// Subconjunto WASM: solo enteros/lógicos (inferidos incluidos) y sin E/S.
fn funcion_pura_enteros(f: &MirFunction, ctx: &CtxFn) -> bool {
    if !tipo_puro_entero(&ctx.retorno) || !ctx.tipos.iter().all(tipo_puro_entero) {
        return false;
    }
    if !f.params.iter().all(|(_, t)| tipo_puro_entero(t)) {
        return false;
    }
    !f.bloques.iter().any(|b| {
        b.stmts.iter().any(|s| {
            matches!(
                s,
                MirStmt::Llamada { callee, .. }
                    if callee == "mostrar" || callee == "mostrar_sin_salto"
            )
        })
    })
}

fn tipo_puro_entero(t: &TipoSem) -> bool {
    matches!(t, TipoSem::Entero | TipoSem::Logico | TipoSem::Desconocido)
}

fn stmt_soportado(stmt: &MirStmt) -> bool {
    match stmt {
        MirStmt::Asignacion { valor, .. } => rvalue_soportado(valor),
        MirStmt::Llamada { argumentos, .. } => argumentos.iter().all(operando_soportado),
        MirStmt::LlamadaIndirecta { .. } => false,
        MirStmt::EntrarTry { .. } | MirStmt::SalirTry => false,
        MirStmt::AsignacionGlobal { .. } => false,
        MirStmt::Nop => true,
    }
}

fn terminator_soportado(t: &MirTerminator) -> bool {
    match t {
        MirTerminator::Goto(_) | MirTerminator::Unreachable => true,
        MirTerminator::SwitchInt {
            discriminante,
            casos,
            por_defecto: _,
        } => operando_soportado(discriminante) && casos.iter().all(|_| true),
        MirTerminator::Return(op) => op.as_ref().map(operando_soportado).unwrap_or(true),
        MirTerminator::Invoke { .. } => false,
    }
}

fn operando_soportado(op: &MirOperand) -> bool {
    match op {
        MirOperand::Copy(_) | MirOperand::Move(_) | MirOperand::Const(_) => true,
        MirOperand::Global(_) => false,
    }
}

fn rvalue_soportado(r: &MirRvalue) -> bool {
    match r {
        MirRvalue::Use(op) => operando_soportado(op),
        MirRvalue::BinOp { lhs, rhs, .. } => operando_soportado(lhs) && operando_soportado(rhs),
        MirRvalue::UnOp { operand, .. } => operando_soportado(operand),
        MirRvalue::CrearCelda { valor } => operando_soportado(valor),
        MirRvalue::LeerCelda { base } => operando_soportado(base),
        MirRvalue::Agregado { .. }
        | MirRvalue::Indice { .. }
        | MirRvalue::Campo { .. }
        | MirRvalue::Longitud { .. }
        | MirRvalue::Rango { .. }
        | MirRvalue::Clausura { .. }
        | MirRvalue::ObjetoModulo { .. } => false,
    }
}

fn generar_funcion(out: &mut String, f: &MirFunction, ctx: &CtxFn) {
    let params: Vec<String> = f
        .params
        .iter()
        .enumerate()
        .map(|(i, (_, t))| format!("p{}: {}", i, tipo_rust(t)))
        .collect();
    out.push_str(&format!(
        "fn bsk_{}({}) -> {} {{\n",
        f.nombre,
        params.join(", "),
        tipo_rust(&ctx.retorno)
    ));
    for (i, t) in ctx.tipos.iter().enumerate() {
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
    out.push_str(&format!(
        "    let mut __ret: {} = {};\n",
        tipo_rust(&ctx.retorno),
        valor_por_defecto(&ctx.retorno)
    ));
    out.push_str("    loop {\n");
    out.push_str("        match __bloque {\n");
    for b in &f.bloques {
        out.push_str(&format!("            {} => {{\n", b.id.0));
        for stmt in &b.stmts {
            generar_stmt(out, stmt, 16, ctx);
        }
        generar_terminator(out, &b.terminator, ctx);
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
        TipoSem::Texto => "String",
        _ => "i64",
    }
}

fn valor_por_defecto(t: &TipoSem) -> &'static str {
    match t {
        TipoSem::Texto => "String::new()",
        _ => "0",
    }
}

fn generar_stmt(out: &mut String, stmt: &MirStmt, indent: usize, ctx: &CtxFn) {
    let pad = " ".repeat(indent);
    match stmt {
        MirStmt::Asignacion { destino, valor } => {
            let rhs = generar_rvalue(valor, ctx);
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
            match callee.as_str() {
                // Nativas de E/S: imprimen y dejan el destino en su valor nulo.
                "mostrar" | "mostrar_sin_salto" => {
                    let macroname = if callee == "mostrar" {
                        "println"
                    } else {
                        "print"
                    };
                    let cuerpo = if args.len() == 1 {
                        format!("{macroname}!(\"{{}}\", {})", args[0])
                    } else {
                        format!("{macroname}!(\"\")")
                    };
                    out.push_str(&format!("{}{};\n", pad, cuerpo));
                    if let Some(d) = destino
                        && d.projection.is_empty()
                    {
                        let t = ctx
                            .tipos
                            .get(d.local.0)
                            .cloned()
                            .unwrap_or(TipoSem::Desconocido);
                        out.push_str(&format!(
                            "{}l{} = {};\n",
                            pad,
                            d.local.0,
                            valor_por_defecto(&t)
                        ));
                    }
                }
                _ => {
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

fn generar_terminator(out: &mut String, t: &MirTerminator, ctx: &CtxFn) {
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
                .unwrap_or_else(|| valor_por_defecto(&ctx.retorno).to_string());
            out.push_str(&format!("{}__ret = {};\n{}break;\n", pad, v, pad));
        }
        MirTerminator::Invoke { .. } => {
            let v = valor_por_defecto(&ctx.retorno);
            out.push_str(&format!("{}__ret = {};\n{}break;\n", pad, v, pad));
        }
        MirTerminator::Unreachable => {
            let v = valor_por_defecto(&ctx.retorno);
            out.push_str(&format!("{}__ret = {};\n{}break;\n", pad, v, pad));
        }
    }
}

/// Escape de un literal de texto para generar un `&str` de Rust válido.
fn escapar_texto(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            otro => out.push(otro),
        }
    }
    out
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
            MirConst::Texto(s) => format!("\"{}\".to_string()", escapar_texto(s)),
            MirConst::Nulo => "0".to_string(),
            MirConst::Funcion(_) => "0".to_string(), // first-class funcs not supported in native backend
        },
    }
}

fn es_texto_operando(op: &MirOperand, ctx: &CtxFn) -> bool {
    match op {
        MirOperand::Const(MirConst::Texto(_)) => true,
        MirOperand::Copy(l) | MirOperand::Move(l) => ctx
            .tipos
            .get(l.0)
            .is_some_and(|t| matches!(t, TipoSem::Texto)),
        _ => false,
    }
}

fn generar_rvalue(r: &MirRvalue, ctx: &CtxFn) -> String {
    match r {
        MirRvalue::Use(op) => generar_operando(op),
        MirRvalue::BinOp { op, lhs, rhs } => {
            let l = generar_operando(lhs);
            let r = generar_operando(rhs);
            match op {
                // La concatenación texto+texto usa `format!` porque
                // `String + String` no compila en Rust. La mezcla texto/entero
                // queda rechazada por `sumas_homogeneas` (el VM también la
                // rechaza), así que el caso `else` es numérico puro.
                MirBinOp::Suma => {
                    if es_texto_operando(lhs, ctx) && es_texto_operando(rhs, ctx) {
                        format!("format!(\"{{}}{{}}\", {}, {})", l, r)
                    } else {
                        format!("({} + {})", l, r)
                    }
                }
                MirBinOp::Resta => format!("({} - {})", l, r),
                MirBinOp::Multiplicacion => format!("({} * {})", l, r),
                MirBinOp::Division => format!("({} / {})", l, r),
                MirBinOp::Modulo => format!("({} % {})", l, r),
                // Las comparaciones de texto compilan igual (String: PartialEq/Ord).
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
