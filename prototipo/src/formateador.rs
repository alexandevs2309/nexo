//! Formateador básico de BASILISCO basado en el AST.

use crate::parser::{Expresion, Sentencia, TipoAnotacion};
use crate::valor::Valor;

pub fn formatear(fuente: &str) -> Result<String, String> {
    let mut lx = crate::Lexer::nuevo(fuente);
    let tokens = lx.tokenizar().map_err(|e| e.to_string())?;
    let comentarios = lx.comentarios().to_vec();
    let sentencias = crate::Parser::nuevo(tokens)
        .parsear_programa()
        .map_err(|e| e.to_string())?;
    let mut salida = String::new();
    let mut ci = 0usize;
    let lineas_src: Vec<&str> = fuente.lines().collect();
    for (i, s) in sentencias.iter().enumerate() {
        let next_line = sentencias
            .get(i + 1)
            .map(linea_sentencia)
            .unwrap_or(usize::MAX);
        emitir_comentarios_antes(&comentarios, &mut ci, linea_sentencia(s), 0, &mut salida);
        formatear_sentencia(
            s,
            0,
            &mut salida,
            &comentarios,
            &mut ci,
            next_line,
            &lineas_src,
        );
    }
    emitir_comentarios_restantes(&comentarios, &mut ci, 0, &mut salida);
    Ok(salida)
}

fn linea_sentencia(s: &Sentencia) -> usize {
    match s {
        Sentencia::Declaracion { linea, .. }
        | Sentencia::Asignacion { linea, .. }
        | Sentencia::AsignacionIndice { linea, .. }
        | Sentencia::Si { linea, .. }
        | Sentencia::Mientras { linea, .. }
        | Sentencia::ParaCada { linea, .. }
        | Sentencia::Funcion { linea, .. }
        | Sentencia::Retorno { linea, .. }
        | Sentencia::IntentarRescatar { linea, .. }
        | Sentencia::Usar { linea, .. }
        | Sentencia::Importar { linea, .. }
        | Sentencia::Exportar { linea, .. }
        | Sentencia::Mostrar { linea, .. }
        | Sentencia::MostrarSinSalto { linea, .. }
        | Sentencia::Romper { linea, .. }
        | Sentencia::Continuar { linea, .. } => *linea,
        Sentencia::Expresion(e) => linea_expresion(e),
        Sentencia::Bloque(_) => usize::MAX,
        _ => usize::MAX,
    }
}

fn linea_expresion(e: &Expresion) -> usize {
    match e {
        Expresion::Literal { linea, .. }
        | Expresion::Variable { linea, .. }
        | Expresion::Llamada { linea, .. }
        | Expresion::ArregloLiteral { linea, .. }
        | Expresion::Indice { linea, .. }
        | Expresion::Acceso { linea, .. }
        | Expresion::LlamadaIndirecta { linea, .. }
        | Expresion::Lambda { linea, .. }
        | Expresion::Rango { linea, .. }
        | Expresion::ConstruccionEstructura { linea, .. }
        | Expresion::ConstruccionVariante { linea, .. }
        | Expresion::Coincidir { linea, .. } => *linea,
        _ => usize::MAX,
    }
}

fn emitir_comentarios_antes(
    comentarios: &[(usize, String)],
    ci: &mut usize,
    linea_siguiente: usize,
    nivel: usize,
    salida: &mut String,
) {
    while *ci < comentarios.len() {
        let (linea, texto) = &comentarios[*ci];
        if linea_siguiente != usize::MAX && *linea > linea_siguiente {
            break;
        }
        salida.push_str(&indent(nivel));
        salida.push_str(texto);
        salida.push('\n');
        *ci += 1;
    }
}

fn emitir_comentarios_restantes(
    comentarios: &[(usize, String)],
    ci: &mut usize,
    nivel: usize,
    salida: &mut String,
) {
    while *ci < comentarios.len() {
        let (_, texto) = &comentarios[*ci];
        salida.push_str(&indent(nivel));
        salida.push_str(texto);
        salida.push('\n');
        *ci += 1;
    }
}

fn indent(n: usize) -> String {
    "  ".repeat(n)
}

fn tipo_str(t: &TipoAnotacion) -> String {
    match t {
        TipoAnotacion::Entero => "entero".to_string(),
        TipoAnotacion::Decimal => "decimal".to_string(),
        TipoAnotacion::Real => "real".to_string(),
        TipoAnotacion::Logico => "logico".to_string(),
        TipoAnotacion::Texto => "texto".to_string(),
        TipoAnotacion::Nulo => "nulo".to_string(),
        TipoAnotacion::Arreglo(elemento, tamano) => {
            format!("arreglo<{}, {}>", tipo_str(elemento), tamano)
        }
        TipoAnotacion::Lista(elemento) => {
            format!("lista<{}>", tipo_str(elemento))
        }
        TipoAnotacion::Parametro(n) => n.clone(),
        TipoAnotacion::Nominal(n) => n.clone(),
    }
}

fn valor_str(v: &Valor) -> String {
    match v {
        Valor::Texto(s) => {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        }
        Valor::Logico(true) => "verdadero".to_string(),
        Valor::Logico(false) => "falso".to_string(),
        Valor::Nulo => "nulo".to_string(),
        Valor::Arreglo(xs) | Valor::Lista(xs) => {
            let inner: Vec<String> = xs.iter().map(valor_str).collect();
            format!("[{}]", inner.join(", "))
        }
        other => other.to_string(),
    }
}

fn prec(e: &Expresion) -> u8 {
    match e {
        Expresion::O(_, _) => 1,
        Expresion::Y(_, _) => 2,
        Expresion::No(_) => 3,
        Expresion::IgualIgual(_, _)
        | Expresion::Distinto(_, _)
        | Expresion::Menor(_, _)
        | Expresion::MenorIgual(_, _)
        | Expresion::Mayor(_, _)
        | Expresion::MayorIgual(_, _) => 4,
        Expresion::Suma(_, _) | Expresion::Resta(_, _) => 5,
        Expresion::Multiplicacion(_, _) | Expresion::Division(_, _) | Expresion::Modulo(_, _) => 6,
        _ => 7,
    }
}

fn expr_str(e: &Expresion) -> String {
    match e {
        Expresion::Literal { valor, .. } => valor_str(valor),
        Expresion::Variable { nombre, .. } => nombre.clone(),
        Expresion::Suma(a, b) => bin_str(a, b, "+", 5),
        Expresion::Resta(a, b) => bin_str(a, b, "-", 5),
        Expresion::Multiplicacion(a, b) => bin_str(a, b, "*", 6),
        Expresion::Division(a, b) => bin_str(a, b, "/", 6),
        Expresion::Modulo(a, b) => bin_str(a, b, "%", 6),
        Expresion::IgualIgual(a, b) => bin_str(a, b, "==", 4),
        Expresion::Distinto(a, b) => bin_str(a, b, "!=", 4),
        Expresion::Menor(a, b) => bin_str(a, b, "<", 4),
        Expresion::MenorIgual(a, b) => bin_str(a, b, "<=", 4),
        Expresion::Mayor(a, b) => bin_str(a, b, ">", 4),
        Expresion::MayorIgual(a, b) => bin_str(a, b, ">=", 4),
        Expresion::Y(a, b) => bin_str(a, b, "y", 2),
        Expresion::O(a, b) => bin_str(a, b, "o", 1),
        Expresion::No(x) => {
            if prec(x) < 3 {
                format!("no ({})", expr_str(x))
            } else {
                format!("no {}", expr_str(x))
            }
        }
        Expresion::Llamada {
            nombre,
            argumentos,
            argumentos_tipo,
            ..
        } => {
            let args: Vec<String> = argumentos.iter().map(expr_str).collect();
            if argumentos_tipo.is_empty() {
                format!("{}({})", nombre, args.join(", "))
            } else {
                let targs: Vec<String> = argumentos_tipo.iter().map(tipo_str).collect();
                format!("{}::<{}>({})", nombre, targs.join(", "), args.join(", "))
            }
        }
        Expresion::ArregloLiteral { elementos, .. } => {
            let inner: Vec<String> = elementos.iter().map(expr_str).collect();
            format!("[{}]", inner.join(", "))
        }
        Expresion::Indice { objeto, indice, .. } => {
            format!("{}[{}]", expr_str(objeto), expr_str(indice))
        }
        Expresion::Acceso {
            objeto, miembro, ..
        } => {
            format!("{}.{}", expr_str(objeto), miembro)
        }
        Expresion::LlamadaIndirecta {
            callee, argumentos, ..
        } => {
            let args: Vec<String> = argumentos.iter().map(expr_str).collect();
            format!("{}({})", expr_str(callee), args.join(", "))
        }
        Expresion::Lambda { parametros, .. } => {
            let params: Vec<String> = parametros.iter().map(|(n, _)| n.clone()).collect();
            format!("funcion({}) {{ ... }}", params.join(", "))
        }
        Expresion::Rango {
            inicio,
            fin,
            inclusivo,
            ..
        } => {
            let op = if *inclusivo { "..=" } else { ".." };
            format!("{}{}{}", expr_str(inicio), op, expr_str(fin))
        }
        Expresion::ConstruccionEstructura { nombre, campos, .. } => {
            if campos.is_empty() {
                format!("{nombre} {{}}")
            } else {
                let inner: Vec<String> = campos
                    .iter()
                    .map(|(n, v)| format!("{n}: {}", expr_str(v)))
                    .collect();
                format!("{nombre} {{ {} }}", inner.join(", "))
            }
        }
        Expresion::ConstruccionVariante {
            tipo,
            variante,
            argumentos,
            ..
        } => {
            if argumentos.is_empty() {
                format!("{tipo}.{variante}")
            } else {
                let args: Vec<String> = argumentos.iter().map(expr_str).collect();
                format!("{tipo}.{variante}({})", args.join(", "))
            }
        }
        Expresion::Coincidir { scrutinee, .. } => {
            format!("coincidir {} {{ ... }}", expr_str(scrutinee))
        }
    }
}

fn bin_str(a: &Expresion, b: &Expresion, op: &str, p: u8) -> String {
    let left = if prec(a) < p {
        format!("({})", expr_str(a))
    } else {
        expr_str(a)
    };
    let right = if prec(b) < p {
        format!("({})", expr_str(b))
    } else {
        expr_str(b)
    };
    format!("{} {} {}", left, op, right)
}

fn comentario_pertenece_bloque(linea_com: usize, nivel: usize, lineas_src: &[&str]) -> bool {
    // Heurística: si la línea original del comentario tiene indentación >= 2*nivel espacios,
    // lo consideramos dentro del bloque. Comentarios en col 0 son top-level.
    if nivel == 0 {
        return true;
    }
    let Some(src_line) = lineas_src.get(linea_com.saturating_sub(1)) else {
        return false;
    };
    let indent_chars = src_line
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .count();
    indent_chars >= nivel * 2
        || src_line.trim_start().starts_with('#') && indent_chars >= nivel.saturating_sub(1) * 2
}

fn formatear_bloque(
    sentencias: &[Sentencia],
    nivel: usize,
    out: &mut String,
    comentarios: &[(usize, String)],
    ci: &mut usize,
    limite_linea: usize,
    lineas_src: &[&str],
) {
    for (i, s) in sentencias.iter().enumerate() {
        let next_line = sentencias
            .get(i + 1)
            .map(linea_sentencia)
            .unwrap_or(limite_linea);
        emitir_comentarios_antes(comentarios, ci, linea_sentencia(s), nivel, out);
        formatear_sentencia(s, nivel, out, comentarios, ci, next_line, lineas_src);
    }
    // Comentarios finales dentro del bloque (antes de '}') - no robar comentarios del padre.
    while *ci < comentarios.len() {
        let (linea, _) = &comentarios[*ci];
        if *linea >= limite_linea {
            break;
        }
        if !comentario_pertenece_bloque(*linea, nivel, lineas_src) {
            break;
        }
        let (_, texto) = &comentarios[*ci];
        out.push_str(&indent(nivel));
        out.push_str(texto);
        out.push('\n');
        *ci += 1;
    }
}

fn formatear_sentencia(
    s: &Sentencia,
    nivel: usize,
    out: &mut String,
    comentarios: &[(usize, String)],
    ci: &mut usize,
    limite_linea: usize,
    lineas_src: &[&str],
) {
    let ind = indent(nivel);
    match s {
        Sentencia::Declaracion {
            nombre,
            tipo_anotacion,
            valor,
            mutable,
            es_const,
            publico,
            ..
        } => {
            let kw = if *es_const {
                "const"
            } else if *mutable {
                if tipo_anotacion.is_some() {
                    "var"
                } else {
                    "sea"
                }
            } else {
                "dato"
            };
            out.push_str(&ind);
            if *publico {
                out.push_str("publico ");
            }
            out.push_str(kw);
            out.push(' ');
            out.push_str(nombre);
            if let Some(t) = tipo_anotacion {
                out.push_str(": ");
                out.push_str(&tipo_str(t));
            }
            out.push_str(" = ");
            out.push_str(&expr_str(valor));
            out.push('\n');
        }
        Sentencia::Asignacion { nombre, valor, .. } => {
            out.push_str(&ind);
            out.push_str(nombre);
            out.push_str(" = ");
            out.push_str(&expr_str(valor));
            out.push('\n');
        }
        Sentencia::AsignacionIndice {
            objeto,
            indice,
            valor,
            ..
        } => {
            out.push_str(&ind);
            out.push_str(&format!(
                "{}[{}] = {}",
                expr_str(objeto),
                expr_str(indice),
                expr_str(valor)
            ));
            out.push('\n');
        }
        Sentencia::AsignacionCampo {
            objeto,
            campo,
            valor,
            ..
        } => {
            out.push_str(&ind);
            out.push_str(&format!(
                "{}.{} = {}",
                expr_str(objeto),
                campo,
                expr_str(valor)
            ));
            out.push('\n');
        }
        Sentencia::Estructura {
            nombre,
            campos,
            publico,
            ..
        } => {
            out.push_str(&ind);
            if *publico {
                out.push_str("publico ");
            }
            out.push_str("estructura ");
            out.push_str(nombre);
            if campos.is_empty() {
                out.push_str(" {}\n");
            } else {
                out.push_str(" {\n");
                for c in campos {
                    out.push_str(&indent(nivel + 1));
                    if c.publico {
                        out.push_str("publico ");
                    }
                    out.push_str(&c.nombre);
                    out.push_str(": ");
                    out.push_str(&tipo_str(&c.tipo));
                    out.push('\n');
                }
                out.push_str(&ind);
                out.push_str("}\n");
            }
        }
        Sentencia::Enumeracion {
            nombre,
            variantes,
            publico,
            ..
        } => {
            out.push_str(&ind);
            if *publico {
                out.push_str("publico ");
            }
            out.push_str("enumeracion ");
            out.push_str(nombre);
            if variantes.is_empty() {
                out.push_str(" {}\n");
            } else {
                out.push_str(" {\n");
                for v in variantes {
                    out.push_str(&indent(nivel + 1));
                    out.push_str(&v.nombre);
                    if !v.payload.is_empty() {
                        out.push('(');
                        let tipos: Vec<String> = v.payload.iter().map(tipo_str).collect();
                        out.push_str(&tipos.join(", "));
                        out.push(')');
                    }
                    out.push('\n');
                }
                out.push_str(&ind);
                out.push_str("}\n");
            }
        }
        Sentencia::Rasgo {
            nombre,
            publico,
            metodos,
            ..
        } => {
            let vis = if *publico { "publico " } else { "" };
            out.push_str(&format!("{ind}{vis}rasgo {nombre} {{\n"));
            let ind2 = indent(nivel + 1);
            for m in metodos {
                let rec = match m.receptor {
                    crate::parser::ReceptorMetodo::Ninguno => "".to_string(),
                    crate::parser::ReceptorMetodo::Yo { mutable } => {
                        if mutable {
                            "yo var".to_string()
                        } else {
                            "yo".to_string()
                        }
                    }
                };
                let mut params: Vec<String> = Vec::new();
                if !rec.is_empty() {
                    params.push(rec);
                }
                for (n, t) in &m.parametros {
                    if let Some(t) = t {
                        params.push(format!("{n}: {}", tipo_str(t)));
                    } else {
                        params.push(n.clone());
                    }
                }
                let ret = m
                    .tipo_retorno
                    .as_ref()
                    .map(|t| format!(" -> {}", tipo_str(t)))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "{ind2}funcion {}({}){ret}\n",
                    m.nombre,
                    params.join(", ")
                ));
            }
            out.push_str(&format!("{ind}}}\n"));
        }
        Sentencia::ImplementacionInherente { tipo, metodos, .. } => {
            out.push_str(&format!("{ind}implementar {tipo} {{\n"));
            let ind2 = indent(nivel + 1);
            for m in metodos {
                let rec = match m.receptor {
                    crate::parser::ReceptorMetodo::Ninguno => "".to_string(),
                    crate::parser::ReceptorMetodo::Yo { mutable } => {
                        if mutable {
                            "yo var".to_string()
                        } else {
                            "yo".to_string()
                        }
                    }
                };
                let mut params: Vec<String> = Vec::new();
                if !rec.is_empty() {
                    params.push(rec);
                }
                for (n, t) in &m.parametros {
                    if let Some(t) = t {
                        params.push(format!("{n}: {}", tipo_str(t)));
                    } else {
                        params.push(n.clone());
                    }
                }
                let ret = m
                    .tipo_retorno
                    .as_ref()
                    .map(|t| format!(" -> {}", tipo_str(t)))
                    .unwrap_or_default();
                let vis_m = if m.publico { "publico " } else { "" };
                out.push_str(&format!(
                    "{ind2}{vis_m}funcion {}({}){ret} {{\n",
                    m.nombre,
                    params.join(", ")
                ));
                formatear_bloque(
                    &m.cuerpo,
                    nivel + 2,
                    out,
                    comentarios,
                    ci,
                    limite_linea,
                    lineas_src,
                );
                out.push_str(&format!("{ind2}}}\n"));
            }
            out.push_str(&format!("{ind}}}\n"));
        }
        Sentencia::ImplementacionRasgo {
            rasgo,
            tipo,
            metodos,
            ..
        } => {
            out.push_str(&format!("{ind}implementar {rasgo} para {tipo} {{\n"));
            let ind2 = indent(nivel + 1);
            for m in metodos {
                let rec = match m.receptor {
                    crate::parser::ReceptorMetodo::Ninguno => "".to_string(),
                    crate::parser::ReceptorMetodo::Yo { mutable } => {
                        if mutable {
                            "yo var".to_string()
                        } else {
                            "yo".to_string()
                        }
                    }
                };
                let mut params: Vec<String> = Vec::new();
                if !rec.is_empty() {
                    params.push(rec);
                }
                for (n, t) in &m.parametros {
                    if let Some(t) = t {
                        params.push(format!("{n}: {}", tipo_str(t)));
                    } else {
                        params.push(n.clone());
                    }
                }
                let ret = m
                    .tipo_retorno
                    .as_ref()
                    .map(|t| format!(" -> {}", tipo_str(t)))
                    .unwrap_or_default();
                let vis_m = if m.publico { "publico " } else { "" };
                out.push_str(&format!(
                    "{ind2}{vis_m}funcion {}({}){ret} {{\n",
                    m.nombre,
                    params.join(", ")
                ));
                formatear_bloque(
                    &m.cuerpo,
                    nivel + 2,
                    out,
                    comentarios,
                    ci,
                    limite_linea,
                    lineas_src,
                );
                out.push_str(&format!("{ind2}}}\n"));
            }
            out.push_str(&format!("{ind}}}\n"));
        }
        Sentencia::Si {
            condicion,
            rama_si,
            rama_sino,
            ..
        } => {
            out.push_str(&ind);
            out.push_str("si ");
            out.push_str(&expr_str(condicion));
            out.push_str(" {\n");
            formatear_bloque(
                rama_si,
                nivel + 1,
                out,
                comentarios,
                ci,
                limite_linea,
                lineas_src,
            );
            out.push_str(&ind);
            out.push('}');
            if let Some(sino) = rama_sino {
                // Detectar sino si
                if sino.len() == 1
                    && let Sentencia::Si { .. } = &sino[0]
                {
                    out.push_str(" sino ");
                    // formatear el si anidado sin indentación inicial extra
                    let mut tmp = String::new();
                    formatear_sentencia(
                        &sino[0],
                        0,
                        &mut tmp,
                        comentarios,
                        ci,
                        limite_linea,
                        lineas_src,
                    );
                    // quitar indentación inicial
                    let trimmed = tmp.trim_start();
                    out.push_str(trimmed);
                    return;
                }
                out.push_str(" sino {\n");
                formatear_bloque(
                    sino,
                    nivel + 1,
                    out,
                    comentarios,
                    ci,
                    limite_linea,
                    lineas_src,
                );
                out.push_str(&ind);
                out.push('}');
            }
            out.push('\n');
        }
        Sentencia::Mientras {
            condicion, cuerpo, ..
        } => {
            out.push_str(&ind);
            out.push_str("mientras ");
            out.push_str(&expr_str(condicion));
            out.push_str(" {\n");
            formatear_bloque(
                cuerpo,
                nivel + 1,
                out,
                comentarios,
                ci,
                limite_linea,
                lineas_src,
            );
            out.push_str(&ind);
            out.push_str("}\n");
        }
        Sentencia::Bloque(sentencias) => {
            out.push_str(&ind);
            out.push_str("{\n");
            formatear_bloque(
                sentencias,
                nivel + 1,
                out,
                comentarios,
                ci,
                limite_linea,
                lineas_src,
            );
            out.push_str(&ind);
            out.push_str("}\n");
        }
        Sentencia::Funcion {
            nombre,
            parametros_tipo,
            parametros,
            tipo_retorno,
            cuerpo,
            publico,
            ..
        } => {
            out.push_str(&ind);
            if *publico {
                out.push_str("publico ");
            }
            out.push_str("funcion ");
            out.push_str(nombre);
            if !parametros_tipo.is_empty() {
                out.push('<');
                out.push_str(&parametros_tipo.join(", "));
                out.push('>');
            }
            out.push('(');
            let params: Vec<String> = parametros
                .iter()
                .map(|(n, t)| {
                    if let Some(tipo) = t {
                        format!("{}: {}", n, tipo_str(tipo))
                    } else {
                        n.clone()
                    }
                })
                .collect();
            out.push_str(&params.join(", "));
            out.push(')');
            if let Some(t) = tipo_retorno {
                out.push_str(" -> ");
                out.push_str(&tipo_str(t));
            }
            out.push_str(" {\n");
            formatear_bloque(
                cuerpo,
                nivel + 1,
                out,
                comentarios,
                ci,
                limite_linea,
                lineas_src,
            );
            out.push_str(&ind);
            out.push_str("}\n");
        }
        Sentencia::Retorno { valor, .. } => {
            out.push_str(&ind);
            out.push_str("retornar");
            if let Some(v) = valor {
                out.push(' ');
                out.push_str(&expr_str(v));
            }
            out.push('\n');
        }
        Sentencia::Mostrar { valor, .. } => {
            out.push_str(&ind);
            out.push_str("mostrar ");
            out.push_str(&expr_str(valor));
            out.push('\n');
        }
        Sentencia::MostrarSinSalto { valor, .. } => {
            out.push_str(&ind);
            out.push_str("mostrar_sin_salto ");
            out.push_str(&expr_str(valor));
            out.push('\n');
        }
        Sentencia::ParaCada {
            variable,
            iterable,
            cuerpo,
            ..
        } => {
            out.push_str(&ind);
            out.push_str("para cada ");
            out.push_str(variable);
            out.push_str(" en ");
            out.push_str(&expr_str(iterable));
            out.push_str(" {\n");
            formatear_bloque(
                cuerpo,
                nivel + 1,
                out,
                comentarios,
                ci,
                limite_linea,
                lineas_src,
            );
            out.push_str(&ind);
            out.push_str("}\n");
        }
        Sentencia::Romper { .. } => {
            out.push_str(&ind);
            out.push_str("romper\n");
        }
        Sentencia::Continuar { .. } => {
            out.push_str(&ind);
            out.push_str("continuar\n");
        }
        Sentencia::IntentarRescatar {
            cuerpo_intentar,
            variable_error,
            cuerpo_rescatar,
            ..
        } => {
            out.push_str(&ind);
            out.push_str("intentar {\n");
            formatear_bloque(
                cuerpo_intentar,
                nivel + 1,
                out,
                comentarios,
                ci,
                limite_linea,
                lineas_src,
            );
            out.push_str(&ind);
            out.push('}');
            out.push_str(" rescatar");
            if let Some(v) = variable_error {
                out.push(' ');
                out.push_str(v);
            }
            out.push_str(" {\n");
            formatear_bloque(
                cuerpo_rescatar,
                nivel + 1,
                out,
                comentarios,
                ci,
                limite_linea,
                lineas_src,
            );
            out.push_str(&ind);
            out.push_str("}\n");
        }
        Sentencia::Importar { ruta, alias, .. } => {
            out.push_str(&ind);
            out.push_str("importar \"");
            out.push_str(ruta);
            out.push('"');
            if let Some(a) = alias {
                out.push_str(" como ");
                out.push_str(a);
            }
            out.push('\n');
        }
        Sentencia::Usar {
            ruta,
            alias,
            nombres,
            ..
        } => {
            out.push_str(&ind);
            out.push_str("usar \"");
            out.push_str(ruta);
            out.push('"');
            if let Some(alias) = alias {
                out.push_str(" como ");
                out.push_str(alias);
            } else if let Some(nombres) = nombres {
                out.push_str(" { ");
                out.push_str(&nombres.join(", "));
                out.push_str(" }");
            }
            out.push('\n');
        }
        Sentencia::Exportar { sentencia, .. } => {
            out.push_str(&ind);
            out.push_str("exportar ");
            // formatear sentencia interna sin indentación inicial
            let mut tmp = String::new();
            formatear_sentencia(
                sentencia,
                0,
                &mut tmp,
                comentarios,
                ci,
                limite_linea,
                lineas_src,
            );
            out.push_str(tmp.trim_start());
        }
        Sentencia::Expresion(e) => {
            out.push_str(&ind);
            out.push_str(&expr_str(e));
            out.push('\n');
        }
    }
}
