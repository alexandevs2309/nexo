//! Formateador básico de BASILISCO basado en el AST.

use crate::parser::{Expresion, Sentencia, TipoAnotacion};
use crate::valor::Valor;

pub fn formatear(fuente: &str) -> Result<String, String> {
    let tokens = crate::Lexer::nuevo(fuente)
        .tokenizar()
        .map_err(|e| e.to_string())?;
    let sentencias = crate::Parser::nuevo(tokens)
        .parsear_programa()
        .map_err(|e| e.to_string())?;
    let mut salida = String::new();
    for s in &sentencias {
        formatear_sentencia(s, 0, &mut salida);
    }
    Ok(salida)
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
            nombre, argumentos, ..
        } => {
            let args: Vec<String> = argumentos.iter().map(expr_str).collect();
            format!("{}({})", nombre, args.join(", "))
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
        Expresion::Rango {
            inicio,
            fin,
            inclusivo,
            ..
        } => {
            let op = if *inclusivo { "..=" } else { ".." };
            format!("{}{}{}", expr_str(inicio), op, expr_str(fin))
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

fn formatear_bloque(sentencias: &[Sentencia], nivel: usize, out: &mut String) {
    for s in sentencias {
        formatear_sentencia(s, nivel, out);
    }
}

fn formatear_sentencia(s: &Sentencia, nivel: usize, out: &mut String) {
    let ind = indent(nivel);
    match s {
        Sentencia::Declaracion {
            nombre,
            tipo_anotacion,
            valor,
            mutable,
            es_const,
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
            formatear_bloque(rama_si, nivel + 1, out);
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
                    formatear_sentencia(&sino[0], 0, &mut tmp);
                    // quitar indentación inicial
                    let trimmed = tmp.trim_start();
                    out.push_str(trimmed);
                    return;
                }
                out.push_str(" sino {\n");
                formatear_bloque(sino, nivel + 1, out);
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
            formatear_bloque(cuerpo, nivel + 1, out);
            out.push_str(&ind);
            out.push_str("}\n");
        }
        Sentencia::Bloque(sentencias) => {
            out.push_str(&ind);
            out.push_str("{\n");
            formatear_bloque(sentencias, nivel + 1, out);
            out.push_str(&ind);
            out.push_str("}\n");
        }
        Sentencia::Funcion {
            nombre,
            parametros,
            tipo_retorno,
            cuerpo,
            ..
        } => {
            out.push_str(&ind);
            out.push_str("funcion ");
            out.push_str(nombre);
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
            formatear_bloque(cuerpo, nivel + 1, out);
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
            formatear_bloque(cuerpo, nivel + 1, out);
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
            formatear_bloque(cuerpo_intentar, nivel + 1, out);
            out.push_str(&ind);
            out.push('}');
            out.push_str(" rescatar");
            if let Some(v) = variable_error {
                out.push(' ');
                out.push_str(v);
            }
            out.push_str(" {\n");
            formatear_bloque(cuerpo_rescatar, nivel + 1, out);
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
            formatear_sentencia(sentencia, 0, &mut tmp);
            out.push_str(tmp.trim_start());
        }
        Sentencia::Expresion(e) => {
            out.push_str(&ind);
            out.push_str(&expr_str(e));
            out.push('\n');
        }
    }
}
