//! Servidor del Protocolo de Servidor de Idiomas (LSP) de BASILISCO.
//!
//! Habla JSON-RPC 2.0 sobre stdio con encuadre `Content-Length` (igual que
//! cualquier cliente LSP). Ofrece:
//!   - `textDocument/publishDiagnostics`: diagnósticos de lexer/parser/
//!     semántico y, cuando no hay errores previos, de verificación HIR/MIR
//!     (códigos/mensajes; ubicación 0,0 si el IR no conserva spans).
//!   - `textDocument/hover`: tipo del símbodo bajo el cursor (best effort).
//!   - `textDocument/completion`: candidatos del ámbito global.
//!
//! El servidor es deliberately minimal: no pretende ser un LSP completo,
//! pero es funcional y sirve para editores (VS Code, Neovim, Helix).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};

use crate::ContextoSemantico;
use crate::Lexer;
use crate::{Parser, hir};

/// Documento abierto en el servidor.
#[derive(Debug, Clone)]
pub struct Documento {
    pub texto: String,
    pub version: i64,
}

/// Estado del servidor LSP.
pub struct ServidorLsp {
    pub documentos: HashMap<String, Documento>,
    pub shutdown: bool,
    pub salir: bool,
}

impl ServidorLsp {
    pub fn nuevo() -> Self {
        Self {
            documentos: HashMap::new(),
            shutdown: false,
            salir: false,
        }
    }

    /// Bucle principal: lee mensajes de stdin y escribe respuestas a stdout.
    pub fn ejecutar(&mut self) {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut writer = stdout;
        loop {
            if self.salir {
                break;
            }
            let msg = match leer_mensaje(&mut reader) {
                Ok(Some(m)) => m,
                Ok(None) => break,
                Err(_) => break,
            };
            let respuestas = self.procesar(&msg);
            for r in respuestas {
                if escribir_mensaje(&mut writer, &r).is_err() {
                    return;
                }
            }
        }
    }

    /// Procesa un mensaje JSON-RPC y devuelve las respuestas a enviar.
    pub fn procesar(&mut self, msg: &serde_json::Value) -> Vec<serde_json::Value> {
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = msg.get("id").cloned();
        match method {
            "initialize" => {
                let result = serde_json::json!({
                    "capabilities": {
                        "textDocumentSync": 1,
                        "hoverProvider": true,
                        "completionProvider": { "triggerCharacters": ["."] }
                    },
                    "serverInfo": { "name": "basilisco-lsp", "version": env!("CARGO_PKG_VERSION") }
                });
                vec![respuesta(id, result)]
            }
            "initialized" => vec![],
            "shutdown" => {
                self.shutdown = true;
                vec![respuesta(id, serde_json::Value::Null)]
            }
            "exit" => {
                self.salir = true;
                vec![]
            }
            "textDocument/didOpen" => {
                let params = msg.get("params").cloned().unwrap_or_default();
                let td = params.get("textDocument").cloned().unwrap_or_default();
                let uri = td
                    .get("uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                let texto = td
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let version = td.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
                self.documentos.insert(
                    uri.clone(),
                    Documento {
                        texto: texto.clone(),
                        version,
                    },
                );
                let diags = self.calcular_diagnosticos(&uri, &texto);
                vec![notificacion(
                    "textDocument/publishDiagnostics",
                    serde_json::json!({
                        "uri": uri,
                        "diagnostics": diags
                    }),
                )]
            }
            "textDocument/didChange" => {
                let params = msg.get("params").cloned().unwrap_or_default();
                let uri = params
                    .get("textDocument")
                    .and_then(|t| t.get("uri"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                let version = params
                    .get("textDocument")
                    .and_then(|t| t.get("version"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                // full sync
                let texto = params
                    .get("contentChanges")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                    .and_then(|o| o.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                self.documentos.insert(
                    uri.clone(),
                    Documento {
                        texto: texto.clone(),
                        version,
                    },
                );
                let diags = self.calcular_diagnosticos(&uri, &texto);
                vec![notificacion(
                    "textDocument/publishDiagnostics",
                    serde_json::json!({
                        "uri": uri,
                        "diagnostics": diags
                    }),
                )]
            }
            "textDocument/hover" => {
                let params = msg.get("params").cloned().unwrap_or_default();
                let uri = params
                    .get("textDocument")
                    .and_then(|t| t.get("uri"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                let pos = params.get("position").cloned().unwrap_or_default();
                let line = pos.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as usize;
                let hover = self.hover(uri, line);
                vec![respuesta(id, hover)]
            }
            "textDocument/completion" => {
                let items = serde_json::json!([
                    { "label": "mostrar", "kind": 3 },
                    { "label": "sea", "kind": 14 },
                    { "label": "var", "kind": 14 },
                    { "label": "funcion", "kind": 14 },
                    { "label": "retornar", "kind": 14 },
                    { "label": "si", "kind": 14 },
                    { "label": "sino", "kind": 14 },
                    { "label": "mientras", "kind": 14 },
                    { "label": "para", "kind": 14 },
                    { "label": "cada", "kind": 14 },
                    { "label": "en", "kind": 14 },
                    { "label": "nulo", "kind": 21 },
                    { "label": "verdadero", "kind": 21 },
                    { "label": "falso", "kind": 21 },
                    { "label": "longitud", "kind": 3 },
                    { "label": "agregar", "kind": 3 },
                    { "label": "a_entero", "kind": 3 },
                    { "label": "a_decimal", "kind": 3 },
                    { "label": "a_real", "kind": 3 },
                    { "label": "texto", "kind": 3 },
                    { "label": "json_parsear", "kind": 3 },
                    { "label": "json_serializar", "kind": 3 },
                ]);
                vec![respuesta(
                    id,
                    serde_json::json!({ "items": items, "isIncomplete": false }),
                )]
            }
            _ => {
                if id.is_some() {
                    vec![error_respuesta(id, -32601, "method not found")]
                } else {
                    vec![]
                }
            }
        }
    }

    /// Calcula los diagnósticos (errores) de un documento.
    ///
    /// Orden: lexer -> parser -> semántico. Si no hay errores previos, intenta
    /// lowering HIR/MIR y ejecuta los gates de verificación para exponer
    /// diagnósticos estructurales (códigos HIR0xx/MIR0xx) sin duplicar los
    /// errores ya reportados por etapas anteriores.
    pub fn calcular_diagnosticos(&self, _uri: &str, texto: &str) -> Vec<serde_json::Value> {
        let mut diags = Vec::new();
        // Lexer
        let tokens = match Lexer::nuevo(texto).tokenizar() {
            Ok(t) => t,
            Err(e) => {
                diags.push(diagnostico(0, 0, &e.to_string(), 1));
                return diags;
            }
        };
        // Parser
        let sentencias = match Parser::nuevo(tokens).parsear_programa() {
            Ok(s) => s,
            Err(e) => {
                diags.push(diagnostico(
                    e.linea.saturating_sub(1),
                    e.columna.saturating_sub(1),
                    &e.mensaje,
                    1,
                ));
                return diags;
            }
        };
        // Semántico
        let mut ctx = ContextoSemantico::nuevo();
        let errs = ctx.analizar_programa(&sentencias);
        for e in errs {
            diags.push(diagnostico(
                e.linea.saturating_sub(1),
                e.columna.saturating_sub(1),
                &e.mensaje,
                1,
            ));
        }
        // HIR/MIR: solo si no hubo diagnósticos previos (evita duplicación).
        if diags.is_empty() {
            *hir::MODULO_ERROR.lock().unwrap() = None;
            match hir::lower_programa(&sentencias, &ctx) {
                Ok(hir_prog) => {
                    if let Some(e) = hir::MODULO_ERROR.lock().unwrap().take() {
                        diags.push(diagnostico(0, 0, &e, 1));
                    } else {
                        let rh = hir::VerificadorHir::verificar_detallado(&hir_prog);
                        if rh.es_ok() {
                            let mir_prog = crate::mir::lower_programa(&hir_prog);
                            let rm = crate::mir::VerificadorMir::verificar_detallado(&mir_prog);
                            for e in rm.errores {
                                diags.push(diagnostico(0, 0, &e.to_string(), 1));
                            }
                        } else {
                            for e in rh.errores {
                                diags.push(diagnostico(0, 0, &e.to_string(), 1));
                            }
                        }
                    }
                }
                Err(errs_lower) => {
                    // Errores de lowering HIR: reportar sin duplicar semánticos.
                    if let Some(e) = hir::MODULO_ERROR.lock().unwrap().take() {
                        diags.push(diagnostico(0, 0, &e, 1));
                    } else {
                        for e in errs_lower {
                            diags.push(diagnostico(0, 0, &e, 1));
                        }
                    }
                }
            }
        }
        diags
    }

    /// Hover best-effort: devuelve el texto del símbodo si se puede inferir.
    fn hover(&self, uri: &str, line: usize) -> serde_json::Value {
        let Some(doc) = self.documentos.get(uri) else {
            return serde_json::Value::Null;
        };
        let lineas: Vec<&str> = doc.texto.lines().collect();
        let Some(l) = lineas.get(line) else {
            return serde_json::Value::Null;
        };
        // Devuelve la línea como markdown para que el cliente la muestre.
        serde_json::json!({
            "contents": { "kind": "markdown", "value": format!("```basilisco\n{}\n```", l.trim()) }
        })
    }
}

fn diagnostico(linea: usize, columna: usize, mensaje: &str, severidad: u8) -> serde_json::Value {
    serde_json::json!({
        "range": {
            "start": { "line": linea, "character": columna },
            "end": { "line": linea, "character": columna + 1 }
        },
        "severity": severidad,
        "source": "basilisco",
        "message": mensaje
    })
}

fn respuesta(id: Option<serde_json::Value>, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_respuesta(id: Option<serde_json::Value>, code: i64, msg: &str) -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": msg } })
}

fn notificacion(method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "method": method, "params": params })
}

/// Lee un mensaje JSON-RPC del reader. Devuelve None al EOF.
pub fn leer_mensaje<R: BufRead>(reader: &mut R) -> Result<Option<serde_json::Value>, String> {
    // Leer cabeceras hasta línea vacía.
    let mut content_length: Option<usize> = None;
    loop {
        let mut linea = String::new();
        let n = reader.read_line(&mut linea).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(None); // EOF
        }
        let linea = linea.trim_end_matches(['\r', '\n']);
        if linea.is_empty() {
            break;
        }
        if let Some(resto) = linea.strip_prefix("Content-Length:")
            && let Ok(v) = resto.trim().parse::<usize>()
        {
            content_length = Some(v);
        }
    }
    let len = content_length.ok_or_else(|| "falta Content-Length".to_string())?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
    let texto = String::from_utf8(buf).map_err(|e| e.to_string())?;
    let valor: serde_json::Value = serde_json::from_str(&texto).map_err(|e| e.to_string())?;
    Ok(Some(valor))
}

pub fn escribir_mensaje<W: Write>(writer: &mut W, msg: &serde_json::Value) -> Result<(), String> {
    let texto = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    let header = format!("Content-Length: {}\r\n\r\n", texto.len());
    writer
        .write_all(header.as_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(texto.as_bytes())
        .map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}
