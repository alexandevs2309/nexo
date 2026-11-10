//! Backend nativo de BASILISCO (Etapa A: colecciones + runtime embebido).
//!
//! Genera código Rust a partir de MIR. El código generado incluye un módulo
//! runtime (`bsk_runtime`) con un enum `Valor` uniforme que replica la
//! semántica del intérprete (aritmética, colecciones, E/S, errores). Las
//! funciones generadas operan sobre `Valor` y devuelven `Result<Valor, RuntimeError>`.
//!
//! Soporta: entero, lógico, real, decimal, texto, arreglos, listas, objetos,
//! colecciones heterogéneas, indexación, agregar/eliminar, longitud, E/S,
//! control de flujo, llamadas directas/indirectas, globals/`__init`,
//! `ObjetoModulo` (alias de módulos inlinados por HIR) y stdlib nativa
//! `leer_archivo`/`json_parsear` (parser JSON en runtime, std-only).
//!
//! No soporta (rechazado explícitamente): closures/celdas, try/catch/invoke,
//! rangos, y stdlib no cubierta por el runtime embebido.

use std::collections::HashMap;

use crate::mir::{
    MirBinOp, MirConst, MirFunction, MirOperand, MirPrograma, MirProjection, MirRvalue, MirStmt,
    MirTerminator, MirUnOp,
};
use crate::semantico::TipoSem;

/// Runtime embebido (solo std, sin crates externos).
const RUNTIME: &str = include_str!("runtime.rs");

/// Genera únicamente las funciones `bsk_*` (sin punto de entrada).
pub fn generar_funciones(programa: &MirPrograma) -> String {
    generar_funciones_con(programa, true)
}

/// Genera solo las funciones del subconjunto puro entero/lógico (sin texto ni
/// E/S ni colecciones), reutilizable por el backend WASM.
pub fn generar_funciones_enteras(programa: &MirPrograma) -> String {
    crate::backend_entero::generar_funciones_enteras(programa)
}

fn generar_funciones_con(programa: &MirPrograma, _permitir_texto: bool) -> String {
    let retornos = inferir_retornos_programa(programa);
    let mut out = String::new();
    out.push_str("mod bsk_runtime {\n");
    // indent the runtime? No — just include as-is inside mod. The runtime uses
    // its own `use` statements and pub items; wrapping in mod is fine.
    out.push_str(RUNTIME);
    out.push_str("}\n\n");
    out.push_str("use bsk_runtime::Valor;\n");
    out.push_str("use bsk_runtime::RuntimeError;\n");
    out.push_str("use bsk_runtime::TipoConv;\n\n");

    // Wrappers de stdlib nativa (E/S, JSON) para despacho directo `bsk_*`.
    out.push_str(
        "fn bsk_leer_archivo(ruta: Valor) -> Result<Valor, RuntimeError> { bsk_runtime::bsk_leer_archivo(ruta) }\n\
         fn bsk_json_parsear(texto: Valor) -> Result<Valor, RuntimeError> { bsk_runtime::bsk_json_parsear(texto) }\n\n",
    );

    // Almacén de variables globales (módulos/__init, Etapa C).
    let n_globals = programa.globals.len();
    out.push_str(&format!(
        "static BSK_GLOBALS: std::sync::OnceLock<std::sync::Mutex<Vec<Valor>>> = std::sync::OnceLock::new();\n\
         fn bsk_g() -> &'static std::sync::Mutex<Vec<Valor>> {{\n\
         \x20   BSK_GLOBALS.get_or_init(|| std::sync::Mutex::new(vec![Valor::Nulo; {}]))\n\
         }}\n\n",
        n_globals
    ));

    // Despacho de llamadas indirectas (alias de módulo / valores función).
    let mut soportadas: Vec<&MirFunction> = Vec::new();
    for f in &programa.funciones {
        let ctx = inferir_ctx(f, &retornos);
        if funcion_soportada(f, &ctx, &retornos) {
            soportadas.push(f);
        }
    }
    out.push_str(
        "fn bsk_llamar_indirecto(callee: Valor, args: Vec<Valor>) -> Result<Valor, RuntimeError> {\n\
         \x20   match callee {\n\
         \x20       Valor::Funcion(nombre) => match nombre.as_str() {\n",
    );
    for f in &soportadas {
        let params: Vec<String> = (0..f.params.len())
            .map(|i| format!("args.get({i}).cloned().unwrap_or(Valor::Nulo)"))
            .collect();
        out.push_str(&format!(
            "            \"{}\" => bsk_{}({}),\n",
            f.nombre,
            f.nombre,
            params.join(", ")
        ));
    }
    out.push_str(
        "            otro => Err(RuntimeError(format!(\"función no encontrada: {otro}\"))),\n\
         \x20       },\n\
         \x20       otro => Err(RuntimeError(format!(\"no es una función: {otro}\"))),\n\
         \x20   }\n\
         }\n\n",
    );

    for f in &programa.funciones {
        let ctx = inferir_ctx(f, &retornos);
        let soportada = funcion_soportada(f, &ctx, &retornos);
        if !soportada {
            out.push_str(&format!(
                "// función '{}' omitida: tipos no soportados en el backend nativo\n\n",
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
pub fn generar_rust(programa: &MirPrograma) -> String {
    let retornos = inferir_retornos_programa(programa);
    let main_fn = programa.funciones.iter().find(|f| f.nombre == "main");
    let main_soportada = main_fn
        .map(|f| funcion_soportada(f, &inferir_ctx(f, &retornos), &retornos))
        .unwrap_or(false);
    let main_retorno = main_fn
        .map(|f| inferir_ctx(f, &retornos).retorno)
        .unwrap_or(TipoSem::Desconocido);
    let init_soportada = programa.funciones.iter().any(|f| {
        f.nombre == "__init" && funcion_soportada(f, &inferir_ctx(f, &retornos), &retornos)
    });

    let mut out = String::new();
    out.push_str("// Generado por BASILISCO backend nativo (Etapa A: runtime embebido)\n");
    out.push_str("#![allow(unused)]\n\n");
    if !main_soportada {
        out.push_str(
            "compile_error!(\"backend nativo: la función 'main' usa operaciones no soportadas \
             (closures, try/catch/invoke u otras construcciones aún no soportadas)\");\n\n",
        );
    }
    out.push_str(&generar_funciones(programa));
    out.push_str("fn main() {\n");
    if init_soportada {
        out.push_str("    if let Err(e) = bsk___init() {\n");
        out.push_str("        eprintln!(\"error: {}\", e);\n");
        out.push_str("        std::process::exit(1);\n");
        out.push_str("    }\n");
    }
    if main_soportada {
        if matches!(main_retorno, TipoSem::Desconocido) {
            out.push_str("    match bsk_main() {\n");
            out.push_str("        Ok(_) => {}\n");
            out.push_str(
                "        Err(e) => { eprintln!(\"error: {}\", e); std::process::exit(1); }\n",
            );
            out.push_str("    }\n");
        } else {
            out.push_str("    match bsk_main() {\n");
            out.push_str("        Ok(v) => println!(\"{}\", v),\n");
            out.push_str(
                "        Err(e) => { eprintln!(\"error: {}\", e); std::process::exit(1); }\n",
            );
            out.push_str("    }\n");
        }
    } else {
        out.push_str("    println!(\"0\");\n");
    }
    out.push_str("}\n");
    out
}

// ---------- Inferencia (reutilizada) ----------

struct CtxFn {
    tipos: Vec<TipoSem>,
    retorno: TipoSem,
}

type Retornos = HashMap<String, TipoSem>;

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

fn inferir_locales(f: &MirFunction, retornos: &Retornos) -> Vec<TipoSem> {
    let mut tipos = f.locals.clone();
    let mut cambio = true;
    while cambio {
        cambio = false;
        for b in &f.bloques {
            for s in &b.stmts {
                let destino_y_tipo = match s {
                    MirStmt::Asignacion { destino, valor } => {
                        tipo_rvalue(valor, &tipos).map(|t| (destino.local.0, t))
                    }
                    MirStmt::Llamada {
                        destino: Some(d),
                        callee,
                        ..
                    } => {
                        if callee == "mostrar" || callee == "mostrar_sin_salto" {
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
                    && !matches!(t, TipoSem::Desconocido)
                {
                    tipos[i] = t;
                    cambio = true;
                }
            }
        }
    }
    tipos
}

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
        MirRvalue::Agregado { .. } => Some(TipoSem::Arreglo(Box::new(TipoSem::Desconocido), 0)),
        MirRvalue::Indice { .. } => Some(TipoSem::Desconocido),
        MirRvalue::Longitud { .. } => Some(TipoSem::Entero),
        MirRvalue::Campo { .. } => Some(TipoSem::Desconocido),
        _ => None,
    }
}

fn tipo_operando(op: &MirOperand, tipos: &[TipoSem]) -> Option<TipoSem> {
    match op {
        MirOperand::Const(c) => match c {
            MirConst::Entero(_) | MirConst::Logico(_) => Some(TipoSem::Entero),
            MirConst::Texto(_) => Some(TipoSem::Texto),
            MirConst::Funcion(_) => Some(TipoSem::Desconocido),
            _ => None,
        },
        MirOperand::Copy(l) | MirOperand::Move(l) => tipos
            .get(l.0)
            .cloned()
            .filter(|t| !matches!(t, TipoSem::Desconocido)),
        MirOperand::Global(_) => None,
    }
}

fn es_soportado(t: &TipoSem) -> bool {
    matches!(
        t,
        TipoSem::Entero
            | TipoSem::Logico
            | TipoSem::Texto
            | TipoSem::Real
            | TipoSem::Decimal
            | TipoSem::Desconocido
            | TipoSem::Arreglo(_, _)
            | TipoSem::Lista(_)
            | TipoSem::Coleccion(_)
    )
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

fn llamadas_resueltas(f: &MirFunction, retornos: &Retornos) -> bool {
    f.bloques.iter().all(|b| {
        b.stmts.iter().all(|s| match s {
            MirStmt::Llamada { callee, .. } => {
                retornos.contains_key(callee)
                    || callee == "mostrar"
                    || callee == "mostrar_sin_salto"
                    || callee == "agregar"
                    || callee == "eliminar"
                    || callee == "longitud"
                    || callee == "contiene"
                    || callee == "es_vacio"
                    || callee == "invertir"
                    || callee == "ordenar"
                    || callee == "leer_archivo"
                    || callee == "json_parsear"
            }
            // Llamadas indirectas se despachan en tiempo de ejecución contra
            // las funciones conocidas del programa (Etapa C, módulos/alias).
            MirStmt::LlamadaIndirecta { .. } => true,
            _ => true,
        })
    })
}

fn sumas_homogeneas(_f: &MirFunction, _ctx: &CtxFn) -> bool {
    // La `suma` del runtime es dinámica (numérica, texto, colecciones según
    // soporte); con tipos `Desconocido` (JSON/índices/heterogéneos) no puede
    // decidirse estáticamente. Se delega al runtime (Etapa C, programas reales).
    true
}

fn stmt_soportado(stmt: &MirStmt) -> bool {
    match stmt {
        MirStmt::Asignacion { valor, .. } => rvalue_soportado(valor),
        MirStmt::Llamada { argumentos, .. } => argumentos.iter().all(operando_soportado),
        MirStmt::LlamadaIndirecta {
            callee, argumentos, ..
        } => operando_soportado(callee) && argumentos.iter().all(operando_soportado),
        MirStmt::EntrarTry { .. } | MirStmt::SalirTry => false,
        MirStmt::AsignacionGlobal { valor, .. } => rvalue_soportado(valor),
        MirStmt::Nop => true,
    }
}

fn terminator_soportado(t: &MirTerminator) -> bool {
    match t {
        MirTerminator::Goto(_) | MirTerminator::Unreachable => true,
        MirTerminator::SwitchInt { discriminante, .. } => operando_soportado(discriminante),
        MirTerminator::Return(op) => op.as_ref().map(operando_soportado).unwrap_or(true),
        MirTerminator::Invoke { .. } => false,
    }
}

fn operando_soportado(op: &MirOperand) -> bool {
    match op {
        MirOperand::Copy(_)
        | MirOperand::Move(_)
        | MirOperand::Const(_)
        | MirOperand::Global(_) => true,
    }
}

fn rvalue_soportado(r: &MirRvalue) -> bool {
    match r {
        MirRvalue::Use(op) => operando_soportado(op),
        MirRvalue::BinOp { lhs, rhs, .. } => operando_soportado(lhs) && operando_soportado(rhs),
        MirRvalue::UnOp { operand, .. } => operando_soportado(operand),
        MirRvalue::Agregado { elementos } => elementos.iter().all(operando_soportado),
        MirRvalue::Indice { base, indice } => {
            operando_soportado(base) && operando_soportado(indice)
        }
        MirRvalue::Longitud { base } => operando_soportado(base),
        MirRvalue::Campo { base, .. } => operando_soportado(base),
        // ObjetoModulo se materializa como Objeto(BTreeMap) de funciones; no
        // usa operandos dinámicos. Soportado para módulos/alias (Etapa C).
        MirRvalue::ObjetoModulo { .. } => true,
        MirRvalue::CrearCelda { .. }
        | MirRvalue::LeerCelda { .. }
        | MirRvalue::Clausura { .. }
        | MirRvalue::Rango { .. } => false,
    }
}

// ---------- Generación ----------

fn generar_funcion(out: &mut String, f: &MirFunction, ctx: &CtxFn) {
    out.push_str(&format!(
        "fn bsk_{}({}) -> Result<Valor, RuntimeError> {{\n",
        f.nombre,
        f.params
            .iter()
            .enumerate()
            .map(|(i, (_, _t))| format!("p{}: Valor", i))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    for (i, t) in ctx.tipos.iter().enumerate() {
        out.push_str(&format!(
            "    let mut l{}: Valor = {};\n",
            i,
            valor_por_defecto(t)
        ));
    }
    for (i, (local, _)) in f.params.iter().enumerate() {
        out.push_str(&format!("    l{} = p{};\n", local.0, i));
    }
    out.push_str("    let mut __bloque: usize = 0;\n");
    out.push_str("    let mut __ret: Result<Valor, RuntimeError> = Ok(Valor::Nulo);\n");
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

fn valor_por_defecto(_t: &TipoSem) -> &'static str {
    "Valor::Nulo"
}

/// Genera la expresión `TipoConv` de una local según su tipo declarado.
/// Devuelve `None` si el tipo no aplica conversiones en escritura (igual que
/// `convertir_tipo` de vm.rs: Entero/Texto/Lógico/Nulo/Desconocido/Función dejan
/// el valor tal cual).
fn tipo_conv_expr(t: Option<&TipoSem>) -> Option<String> {
    fn conv_de(t: &TipoSem) -> Option<String> {
        match t {
            TipoSem::Decimal => Some("&TipoConv::Decimal".to_string()),
            TipoSem::Real => Some("&TipoConv::Real".to_string()),
            TipoSem::Lista(elem) => Some(format!(
                "&TipoConv::Lista(Box::new({}))",
                conv_de(elem).unwrap_or_else(|| "TipoConv::Nada".to_string())
            )),
            TipoSem::Arreglo(elem, n) => Some(format!(
                "&TipoConv::Arreglo(Box::new({}), {})",
                conv_de(elem).unwrap_or_else(|| "TipoConv::Nada".to_string()),
                n
            )),
            TipoSem::Coleccion(elem) => Some(format!(
                "&TipoConv::Coleccion(Box::new({}))",
                conv_de(elem).unwrap_or_else(|| "TipoConv::Nada".to_string())
            )),
            _ => None,
        }
    }
    t.and_then(conv_de)
}

fn generar_stmt(out: &mut String, stmt: &MirStmt, indent: usize, ctx: &CtxFn) {
    let pad = " ".repeat(indent);
    match stmt {
        MirStmt::Asignacion { destino, valor } => {
            let rhs = generar_rvalue(valor, ctx);
            if destino.projection.is_empty() {
                // La VM convierte el valor al tipo declarado de la local en
                // cada escritura (`escribir_place` → `convertir_tipo`); el
                // backend nativo replica eso para preservar la paridad
                // (ej. `var r: real = 7` debe almacenar Real(7.0)).
                let tipo = ctx.tipos.get(destino.local.0);
                if let Some(conv) = tipo_conv_expr(tipo) {
                    out.push_str(&format!(
                        "{}l{} = {}.convertir_a({});\n",
                        pad, destino.local.0, rhs, conv
                    ));
                } else {
                    out.push_str(&format!("{}l{} = {};\n", pad, destino.local.0, rhs));
                }
            } else {
                // Proyección: l{base}.indice_set(idx, rhs)?  o  campo
                let base = destino.local.0;
                for proj in &destino.projection {
                    match proj {
                        MirProjection::Indice(idx_op) => {
                            let idx = generar_operando(idx_op);
                            out.push_str(&format!(
                                "{}l{}.indice_set(match {} {{ Valor::Entero(n) => n, _ => 0 }}, {})?;\n",
                                pad, base, idx, rhs
                            ));
                        }
                        MirProjection::Campo(campo) => {
                            out.push_str(&format!(
                                "{}let _ = (l{}, \"{}\", {});\n",
                                pad, base, campo, rhs
                            ));
                        }
                        MirProjection::Celda => {
                            out.push_str(&format!("{}let _ = (l{}, {});\n", pad, base, rhs));
                        }
                    }
                }
            }
        }
        MirStmt::Llamada {
            destino,
            callee,
            argumentos,
        } => {
            let args: Vec<String> = argumentos.iter().map(generar_operando).collect();
            match callee.as_str() {
                "mostrar" => {
                    let cuerpo = if args.len() == 1 {
                        format!("println!(\"{{}}\", {})", args[0])
                    } else {
                        "println!(\"\")".to_string()
                    };
                    out.push_str(&format!("{}{};\n", pad, cuerpo));
                    if let Some(d) = destino
                        && d.projection.is_empty()
                    {
                        out.push_str(&format!("{}l{} = Valor::Nulo;\n", pad, d.local.0));
                    }
                }
                "mostrar_sin_salto" => {
                    let cuerpo = if args.len() == 1 {
                        format!("print!(\"{{}}\", {})", args[0])
                    } else {
                        "print!(\"\")".to_string()
                    };
                    out.push_str(&format!("{}{};\n", pad, cuerpo));
                    if let Some(d) = destino
                        && d.projection.is_empty()
                    {
                        out.push_str(&format!("{}l{} = Valor::Nulo;\n", pad, d.local.0));
                    }
                }
                "agregar" => {
                    let call = if args.len() >= 2 {
                        format!("{}.agregar({})", args[0], args[1])
                    } else if args.len() == 1 {
                        format!("{}.agregar(Valor::Nulo)", args[0])
                    } else {
                        "Valor::Nulo".to_string()
                    };
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
                "eliminar" => {
                    let call = if args.len() >= 2 {
                        format!(
                            "{}.eliminar(match {} {{ Valor::Entero(n) => n, _ => 0 }})?",
                            args[0], args[1]
                        )
                    } else {
                        "Err(RuntimeError(\"argumentos insuficientes\".to_string()))?".to_string()
                    };
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
                "longitud" => {
                    let arg = if !args.is_empty() {
                        args[0].clone()
                    } else {
                        "Valor::Nulo".to_string()
                    };
                    let call = format!("Valor::Entero({}.longitud())", arg);
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
                _ => {
                    let call = format!("bsk_{}({})", callee, args.join(", "));
                    if let Some(d) = destino {
                        if d.projection.is_empty() {
                            out.push_str(&format!("{}l{} = {}?;\n", pad, d.local.0, call));
                        } else {
                            out.push_str(&format!("{}let _ = {}?;\n", pad, call));
                        }
                    } else {
                        out.push_str(&format!("{}let _ = {}?;\n", pad, call));
                    }
                }
            }
        }
        MirStmt::LlamadaIndirecta {
            destino,
            callee,
            argumentos,
        } => {
            let args: Vec<String> = argumentos.iter().map(generar_operando).collect();
            let cv = generar_operando(callee);
            let call = format!("bsk_llamar_indirecto({}, vec![{}])", cv, args.join(", "));
            if let Some(d) = destino {
                if d.projection.is_empty() {
                    out.push_str(&format!("{}l{} = {}?;\n", pad, d.local.0, call));
                } else {
                    out.push_str(&format!("{}let _ = {}?;\n", pad, call));
                }
            } else {
                out.push_str(&format!("{}let _ = {}?;\n", pad, call));
            }
        }
        MirStmt::EntrarTry { .. } => {
            out.push_str(&format!("{}// intentar no soportado\n", pad));
        }
        MirStmt::SalirTry => {}
        MirStmt::AsignacionGlobal { global, valor } => {
            let rhs = generar_rvalue(valor, ctx);
            out.push_str(&format!(
                "{}bsk_g().lock().unwrap()[{}] = {};\n",
                pad, global.0, rhs
            ));
        }
        MirStmt::Nop => {}
    }
}

fn generar_terminator(out: &mut String, t: &MirTerminator, _ctx: &CtxFn) {
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
                    "{}{} {}.discriminante() == {} {{ __bloque = {}; continue; }}\n",
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
                .unwrap_or_else(|| "Valor::Nulo".to_string());
            out.push_str(&format!("{}__ret = Ok({});\n{}break;\n", pad, v, pad));
        }
        MirTerminator::Invoke { .. } => {
            out.push_str(&format!(
                "{}__ret = Err(RuntimeError(\"invoke no soportado\".to_string()));\n{}break;\n",
                pad, pad
            ));
        }
        MirTerminator::Unreachable => {
            out.push_str(&format!("{}__ret = Ok(Valor::Nulo);\n{}break;\n", pad, pad));
        }
    }
}

fn const_to_valor(c: &MirConst) -> String {
    match c {
        MirConst::Entero(n) => format!("Valor::Entero({})", n),
        MirConst::Decimal(n) => format!("Valor::Decimal({})", n),
        MirConst::Real(n) => format!("Valor::Real({})", n),
        MirConst::Logico(b) => format!("Valor::Logico({})", b),
        MirConst::Texto(s) => {
            let esc = escapar_texto(s);
            format!("Valor::Texto(\"{}\".to_string())", esc)
        }
        MirConst::Nulo => "Valor::Nulo".to_string(),
        MirConst::Funcion(s) => format!("Valor::Funcion(\"{}\".to_string())", s),
    }
}

fn generar_operando(op: &MirOperand) -> String {
    match op {
        MirOperand::Copy(l) | MirOperand::Move(l) => format!("l{}.clone()", l.0),
        MirOperand::Const(c) => const_to_valor(c),
        MirOperand::Global(g) => format!("bsk_g().lock().unwrap()[{}].clone()", g.0),
    }
}

fn generar_rvalue(r: &MirRvalue, _ctx: &CtxFn) -> String {
    match r {
        MirRvalue::Use(op) => generar_operando(op),
        MirRvalue::BinOp { op, lhs, rhs } => {
            let l = generar_operando(lhs);
            let r = generar_operando(rhs);
            let method = match op {
                MirBinOp::Suma => "suma",
                MirBinOp::Resta => "resta",
                MirBinOp::Multiplicacion => "mul",
                MirBinOp::Division => "div",
                MirBinOp::Modulo => "modulo",
                MirBinOp::Igual => {
                    return format!(
                        "if {}.igual(&{}) {{ Valor::Logico(true) }} else {{ Valor::Logico(false) }}",
                        l, r
                    );
                }
                MirBinOp::Distinto => {
                    return format!(
                        "if {}.igual(&{}) {{ Valor::Logico(false) }} else {{ Valor::Logico(true) }}",
                        l, r
                    );
                }
                MirBinOp::Menor => {
                    return format!(
                        "match {}.lt(&{}) {{ Ok(b) => Valor::Logico(b), Err(e) => return Err(e) }}",
                        l, r
                    );
                }
                MirBinOp::MenorIgual => {
                    return format!(
                        "match {}.lt(&{}) {{ Ok(b) => Valor::Logico(!b && {}.igual(&{})), Err(e) => return Err(e) }}",
                        l, r, l, r
                    );
                }
                MirBinOp::Mayor => {
                    return format!(
                        "match {}.lt(&{}) {{ Ok(b) => Valor::Logico(!b && !{}.igual(&{})), Err(e) => return Err(e) }}",
                        l, r, l, r
                    );
                }
                MirBinOp::MayorIgual => {
                    return format!(
                        "match {}.lt(&{}) {{ Ok(b) => Valor::Logico(!b), Err(e) => return Err(e) }}",
                        l, r
                    );
                }
                MirBinOp::Y => {
                    return format!(
                        "if {}.igual(&Valor::Logico(true)) && {}.igual(&Valor::Logico(true)) {{ Valor::Logico(true) }} else {{ Valor::Logico(false) }}",
                        l, r
                    );
                }
                MirBinOp::O => {
                    return format!(
                        "if {}.igual(&Valor::Logico(true)) || {}.igual(&Valor::Logico(true)) {{ Valor::Logico(true) }} else {{ Valor::Logico(false) }}",
                        l, r
                    );
                }
            };
            format!("{}.{}(&{})?", l, method, r)
        }
        MirRvalue::UnOp { op, operand } => {
            let o = generar_operando(operand);
            match op {
                MirUnOp::Negacion => format!("{}.neg()?", o),
                MirUnOp::No => format!("{}.not()?", o),
            }
        }
        MirRvalue::Agregado { elementos } => {
            let elems: Vec<String> = elementos.iter().map(generar_operando).collect();
            // MIR Agregado produce Arreglo (como en VM).
            format!("Valor::Arreglo(vec![{}])", elems.join(", "))
        }
        MirRvalue::Indice { base, indice } => {
            let b = generar_operando(base);
            let i = generar_operando(indice);
            // indice espera i64; el operando puede ser Valor. Convertir.
            format!(
                "{}.indice(match {} {{ Valor::Entero(n) => n, _ => 0 }})?",
                b, i
            )
        }
        MirRvalue::Longitud { base } => {
            let b = generar_operando(base);
            format!("Valor::Entero({}.longitud())", b)
        }
        MirRvalue::Campo { base, campo } => {
            let b = generar_operando(base);
            format!("{}.campo(\"{}\")?", b, escapar_texto(campo))
        }
        MirRvalue::ObjetoModulo { prefijo, nombres } => {
            let mut s = String::from("{ let mut __m = std::collections::BTreeMap::new(); ");
            for n in nombres {
                let fname = format!("{}_{}", prefijo, n);
                s.push_str(&format!(
                    "__m.insert(\"{}\".to_string(), Valor::Funcion(\"{}\".to_string())); ",
                    escapar_texto(n),
                    escapar_texto(&fname)
                ));
            }
            s.push_str("Valor::Objeto(__m) }");
            s
        }
        _ => "Valor::Nulo".to_string(),
    }
}

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
