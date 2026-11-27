//! Servidor LSP de BASILISCO (JSON-RPC 2.0 sobre stdio, Content-Length).
//! Reutiliza parser, semántico, HIR/MIR y diagnósticos comunes.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};

use crate::diagnostico::{Diagnostico, Severidad, Span};
use crate::parser::{Expresion, Sentencia};
use crate::{ContextoSemantico, Lexer, Parser, hir, mir};

#[derive(Debug, Clone)]
pub struct Documento {
    pub texto: String,
    pub version: i64,
    pub simbolos: Vec<SimboloIdx>,
    pub referencias: Vec<RefIdx>,
    pub defs_sem: Vec<crate::semantico::DefinicionSemantica>,
    pub refs_sem: Vec<crate::semantico::ReferenciaSemantica>,
}

#[derive(Debug, Clone)]
pub struct SimboloIdx {
    pub nombre: String,
    pub kind: u8, // LSP SymbolKind
    pub detalle: String,
    pub linea: usize,
    pub columna: usize,
    pub linea_fin: usize,
    pub columna_fin: usize,
    pub scope_id: usize,
    pub parent_scope: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct RefIdx {
    pub nombre: String,
    pub linea: usize,
    pub columna: usize,
    pub scope_id: usize,
    pub def: Option<usize>, // index into simbolos
}

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

    pub fn procesar(&mut self, msg: &serde_json::Value) -> Vec<serde_json::Value> {
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = msg.get("id").cloned();
        match method {
            "initialize" => vec![respuesta(
                id,
                serde_json::json!({
                    "capabilities": {
                        "textDocumentSync": { "openClose": true, "change": 2, "save": { "includeText": false } },
                        "hoverProvider": true,
                        "definitionProvider": true,
                        "referencesProvider": true,
                        "documentSymbolProvider": true,
                        "workspaceSymbolProvider": true,
                        "completionProvider": { "triggerCharacters": [".", ":"] },
                        "diagnosticProvider": { "interFileDependencies": false, "workspaceDiagnostics": false }
                    },
                    "serverInfo": { "name": "basilisco-lsp", "version": env!("CARGO_PKG_VERSION") }
                }),
            )],
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
                let td = msg
                    .pointer("/params/textDocument")
                    .cloned()
                    .unwrap_or_default();
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
                let (simbolos, referencias) = indexar(&texto);
                let (diags, defs_sem, refs_sem) = self.analizar_y_diagnosticar(&uri, &texto);
                self.documentos.insert(
                    uri.clone(),
                    Documento {
                        texto: texto.clone(),
                        version,
                        simbolos,
                        referencias,
                        defs_sem,
                        refs_sem,
                    },
                );
                vec![notificacion(
                    "textDocument/publishDiagnostics",
                    serde_json::json!({"uri": uri, "version": version, "diagnostics": diags}),
                )]
            }
            "textDocument/didChange" => {
                let params = msg.get("params").cloned().unwrap_or_default();
                let uri = params
                    .pointer("/textDocument/uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                let version = params
                    .pointer("/textDocument/version")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let cambios = params
                    .get("contentChanges")
                    .and_then(|c| c.as_array())
                    .cloned()
                    .unwrap_or_default();
                if let Some(doc) = self.documentos.get_mut(&uri) {
                    for c in cambios {
                        if let Some(rango) = c.get("range") {
                            let text = c.get("text").and_then(|t| t.as_str()).unwrap_or("");
                            doc.texto = aplicar_cambio_incremental(&doc.texto, rango, text);
                        } else if let Some(text) = c.get("text").and_then(|t| t.as_str()) {
                            doc.texto = text.to_string();
                        }
                    }
                    doc.version = version;
                    let (simbolos, referencias) = indexar(&doc.texto);
                    doc.simbolos = simbolos;
                    doc.referencias = referencias;
                }
                let texto = self
                    .documentos
                    .get(&uri)
                    .map(|d| d.texto.clone())
                    .unwrap_or_default();
                let (diags, defs_sem, refs_sem) = self.analizar_y_diagnosticar(&uri, &texto);
                if let Some(doc) = self.documentos.get_mut(&uri) {
                    doc.defs_sem = defs_sem;
                    doc.refs_sem = refs_sem;
                }
                vec![notificacion(
                    "textDocument/publishDiagnostics",
                    serde_json::json!({"uri": uri, "version": version, "diagnostics": diags}),
                )]
            }
            "textDocument/didClose" => {
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                self.documentos.remove(&uri);
                vec![notificacion(
                    "textDocument/publishDiagnostics",
                    serde_json::json!({"uri": uri, "diagnostics": []}),
                )]
            }
            "textDocument/hover" => {
                let p = msg.get("params").cloned().unwrap_or_default();
                let uri = p
                    .pointer("/textDocument/uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                let (line, col) = pos_params(&p);
                vec![respuesta(id, self.hover(uri, line, col))]
            }
            "textDocument/definition" => {
                let p = msg.get("params").cloned().unwrap_or_default();
                let uri = p
                    .pointer("/textDocument/uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                let (line, col) = pos_params(&p);
                vec![respuesta(id, self.definicion(uri, line, col))]
            }
            "textDocument/references" => {
                let p = msg.get("params").cloned().unwrap_or_default();
                let uri = p
                    .pointer("/textDocument/uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                let (line, col) = pos_params(&p);
                vec![respuesta(
                    id,
                    serde_json::json!(self.referencias(uri, line, col)),
                )]
            }
            "textDocument/documentSymbol" => {
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                vec![respuesta(
                    id,
                    serde_json::json!(self.simbolos_documento(uri)),
                )]
            }
            "workspace/symbol" => {
                let query = msg
                    .pointer("/params/query")
                    .and_then(|q| q.as_str())
                    .unwrap_or("");
                vec![respuesta(
                    id,
                    serde_json::json!(self.simbolos_workspace(query)),
                )]
            }
            "textDocument/completion" => {
                let p = msg.get("params").cloned().unwrap_or_default();
                let uri = p
                    .pointer("/textDocument/uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                let (line, col) = pos_params(&p);
                vec![respuesta(
                    id,
                    serde_json::json!({"items": self.completar(uri, line, col), "isIncomplete": false}),
                )]
            }
            _ => {
                if id.is_some() {
                    vec![error_respuesta(id, -32601, "método no soportado")]
                } else {
                    vec![]
                }
            }
        }
    }

    fn analizar_y_diagnosticar(
        &self,
        uri: &str,
        texto: &str,
    ) -> (
        Vec<serde_json::Value>,
        Vec<crate::semantico::DefinicionSemantica>,
        Vec<crate::semantico::ReferenciaSemantica>,
    ) {
        let mut diags: Vec<Diagnostico> = Vec::new();
        let tokens = match Lexer::nuevo(texto).tokenizar() {
            Ok(t) => t,
            Err(e) => {
                diags.push(Diagnostico {
                    codigo: None,
                    severidad: Severidad::Error,
                    mensaje: e.to_string(),
                    span: Some(Span::punto(Some(uri.to_string()), e.linea, e.columna)),
                    fuente: "basilisco-lexer".to_string(),
                });
                return (
                    diags.into_iter().map(|d| d.a_lsp()).collect(),
                    vec![],
                    vec![],
                );
            }
        };
        let sentencias = match Parser::nuevo(tokens).parsear_programa() {
            Ok(s) => s,
            Err(e) => {
                diags.push(Diagnostico {
                    codigo: None,
                    severidad: Severidad::Error,
                    mensaje: e.mensaje.clone(),
                    span: Some(Span::punto(Some(uri.to_string()), e.linea, e.columna)),
                    fuente: "basilisco-parser".to_string(),
                });
                return (
                    diags.into_iter().map(|d| d.a_lsp()).collect(),
                    vec![],
                    vec![],
                );
            }
        };
        let mut ctx = if let Some(path) = uri_a_ruta(uri) {
            let root = path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf();
            let cargador = std::sync::Arc::new(std::sync::Mutex::new(
                crate::cargador::CargadorModulos::nuevo(root),
            ));
            ContextoSemantico::con_cargador(cargador, Some(path.to_string_lossy().to_string()))
        } else {
            ContextoSemantico::nuevo()
        };
        ctx.habilitar_registro_semantico_lsp();
        let errs = ctx.analizar_programa(&sentencias);
        for e in errs {
            diags.push(Diagnostico {
                codigo: None,
                severidad: Severidad::Error,
                mensaje: e.mensaje.clone(),
                span: Some(Span::punto(Some(uri.to_string()), e.linea, e.columna)),
                fuente: "basilisco-semantico".to_string(),
            });
        }
        let defs_sem: Vec<crate::semantico::DefinicionSemantica> =
            ctx.definiciones_semanticas().values().cloned().collect();
        let refs_sem: Vec<crate::semantico::ReferenciaSemantica> =
            ctx.referencias_semanticas().to_vec();
        if diags.is_empty() {
            *hir::MODULO_ERROR.lock().unwrap() = None;
            match hir::lower_programa_con_archivo(&sentencias, &ctx, Some(uri.to_string())) {
                Ok(hir_prog) => {
                    if let Some(e) = hir::MODULO_ERROR.lock().unwrap().take() {
                        diags.push(Diagnostico::error(
                            e,
                            Some(Span::punto(Some(uri.to_string()), 1, 1)),
                        ));
                    } else {
                        let rh = hir::VerificadorHir::verificar_detallado(&hir_prog);
                        if rh.es_ok() {
                            let mir_prog = mir::lower_programa(&hir_prog);
                            let rm = mir::VerificadorMir::verificar_detallado(&mir_prog);
                            for e in rm.errores {
                                diags.push(Diagnostico {
                                    codigo: Some(e.codigo.to_string()),
                                    severidad: Severidad::Error,
                                    mensaje: e.to_string(),
                                    span: None,
                                    fuente: "basilisco-mir".to_string(),
                                });
                            }
                        } else {
                            for e in rh.errores {
                                diags.push(Diagnostico {
                                    codigo: Some(e.codigo.to_string()),
                                    severidad: Severidad::Error,
                                    mensaje: e.to_string(),
                                    span: None,
                                    fuente: "basilisco-hir".to_string(),
                                });
                            }
                        }
                    }
                }
                Err(errs_lower) => {
                    if let Some(e) = hir::MODULO_ERROR.lock().unwrap().take() {
                        diags.push(Diagnostico::error(
                            e,
                            Some(Span::punto(Some(uri.to_string()), 1, 1)),
                        ));
                    } else {
                        for e in errs_lower {
                            diags.push(Diagnostico::error(
                                e,
                                Some(Span::punto(Some(uri.to_string()), 1, 1)),
                            ));
                        }
                    }
                }
            }
        }
        let ls = lineas_vec(texto);
        let out = diags.into_iter().map(|d| {
            let (linea, col, fin_linea, fin_col) = d.span.as_ref().map(|s| (
                s.linea.saturating_sub(1), s.columna.saturating_sub(1),
                s.linea_fin.saturating_sub(1), s.columna_fin.saturating_sub(1)
            )).unwrap_or((0,0,0,1));
            let (sl,sc) = pos_fuente_a_lsp(&ls, linea, col);
            let (el,ec) = pos_fuente_a_lsp(&ls, fin_linea, fin_col);
            let sev = match d.severidad { Severidad::Error => 1, Severidad::Warning => 2, Severidad::Info => 3 };
            serde_json::json!({"range":{"start":{"line":sl,"character":sc},"end":{"line":el,"character":ec}},"severity":sev,"source":d.fuente,"message":d.mensaje,"code":d.codigo})
        }).collect();
        (out, defs_sem, refs_sem)
    }

    fn pos_fuente_en(&self, texto: &str, line: usize, col_utf16: usize) -> (usize, usize) {
        let linea = texto.split('\n').nth(line).unwrap_or("");
        let cc = lsp_a_char_col(linea, col_utf16) + 1; // 1-based
        (line + 1, cc)
    }

    fn span_contiene(lc: usize, cc: usize, l1: usize, c1: usize, l2: usize, c2: usize) -> bool {
        if lc < l1 || lc > l2 {
            return false;
        }
        if l1 == l2 {
            return cc >= c1 && cc < c2;
        }
        if lc == l1 {
            return cc >= c1;
        }
        if lc == l2 {
            return cc < c2;
        }
        true
    }

    fn simbodo_en_pos(
        &self,
        doc: &Documento,
        line: usize,
        col_utf16: usize,
    ) -> Option<crate::simbodo::SimbodoId> {
        let (lc, cc) = self.pos_fuente_en(&doc.texto, line, col_utf16);
        // Prefer reference at cursor (most precise identity)
        for r in &doc.refs_sem {
            if Self::span_contiene(lc, cc, r.linea, r.columna, r.linea_fin, r.columna_fin) {
                return Some(r.id);
            }
        }
        // Definition at cursor
        for d in &doc.defs_sem {
            if Self::span_contiene(lc, cc, d.linea, d.columna, d.linea_fin, d.columna_fin) {
                return Some(d.id);
            }
        }
        None
    }

    fn def_sem_por_id<'a>(
        &self,
        doc: &'a Documento,
        id: crate::simbodo::SimbodoId,
    ) -> Option<&'a crate::semantico::DefinicionSemantica> {
        doc.defs_sem.iter().find(|d| d.id == id)
    }

    fn hover(&self, uri: &str, line: usize, col: usize) -> serde_json::Value {
        let Some(doc) = self.documentos.get(uri) else {
            return serde_json::Value::Null;
        };
        let Some(nombre) = palabra_en(&doc.texto, line, col) else {
            return serde_json::Value::Null;
        };
        if let Some(sid) = self.simbodo_en_pos(doc, line, col) {
            if let Some(def) = self.def_sem_por_id(doc, sid) {
                let kind = match def.categoria {
                    crate::simbodo::CategoriaSimbodo::Funcion => "función",
                    crate::simbodo::CategoriaSimbodo::Variable => "variable",
                    crate::simbodo::CategoriaSimbodo::Parametro => "parámetro",
                    crate::simbodo::CategoriaSimbodo::Modulo => "módulo",
                    crate::simbodo::CategoriaSimbodo::Builtin => "builtin",
                };
                let tipo = def.tipo.nombre();
                return serde_json::json!({"contents": {"kind":"markdown","value": format!("**{}** `{}`: `{}`", kind, def.nombre, tipo)}});
            }
            // Resuelto semánticamente pero sin def en este doc: no inventar.
            return serde_json::Value::Null;
        }
        // Fallback AST solo si no hay identidad semántica en posición
        let def_idx = self.def_idx_en_pos(doc, line, col, &nombre);
        if let Some(si) = def_idx.and_then(|i| doc.simbolos.get(i)) {
            let kind = match si.kind {
                12 => "función",
                13 => "variable",
                23 => "estructura",
                10 => "enumeración",
                11 => "rasgo",
                _ => "símbolo",
            };
            return serde_json::json!({"contents": {"kind":"markdown","value": format!("**{}** `{}`\n\n{}", kind, si.nombre, si.detalle)}});
        }
        serde_json::Value::Null
    }

    fn def_idx_en_pos(
        &self,
        doc: &Documento,
        line: usize,
        col_utf16: usize,
        nombre: &str,
    ) -> Option<usize> {
        let lc = line + 1;
        let _cc = lsp_a_char_col(doc.texto.lines().nth(line).unwrap_or(""), col_utf16) + 1;
        // exact def at cursor
        for (i, s) in doc.simbolos.iter().enumerate() {
            if s.nombre == nombre && s.linea == lc {
                return Some(i);
            }
        }
        // ref at cursor
        for r in &doc.referencias {
            if r.nombre == nombre
                && r.linea == lc
                && let Some(d) = r.def
            {
                return Some(d);
            }
        }
        None
    }

    fn identidad_def_sem(
        &self,
        doc: &Documento,
        id: crate::simbodo::SimbodoId,
        uri: &str,
    ) -> Option<(String, usize, usize, String)> {
        let def = self.def_sem_por_id(doc, id)?;
        Some((
            uri_def_sem(def, uri),
            def.linea,
            def.columna,
            def.nombre.clone(),
        ))
    }

    fn definicion(&self, uri: &str, line: usize, col: usize) -> serde_json::Value {
        let Some(doc) = self.documentos.get(uri) else {
            return serde_json::Value::Null;
        };
        if palabra_en(&doc.texto, line, col).is_none() {
            return serde_json::Value::Null;
        };
        if let Some(sid) = self.simbodo_en_pos(doc, line, col) {
            if let Some(def) = self.def_sem_por_id(doc, sid) {
                let def_uri = uri_def_sem(def, uri);
                let texto_def = self
                    .documentos
                    .get(&def_uri)
                    .map(|d| d.texto.as_str())
                    .unwrap_or(&doc.texto);
                return serde_json::json!({"uri": def_uri, "range": rango_lsp(def.linea, def.columna, def.linea_fin, def.columna_fin, texto_def)});
            }
            return serde_json::Value::Null;
        }
        let Some(nombre) = palabra_en(&doc.texto, line, col) else {
            return serde_json::Value::Null;
        };
        if let Some(si) = self
            .def_idx_en_pos(doc, line, col, &nombre)
            .and_then(|i| doc.simbolos.get(i))
        {
            return serde_json::json!({"uri": uri, "range": rango_lsp(si.linea, si.columna, si.linea_fin, si.columna_fin, &doc.texto)});
        }
        serde_json::Value::Null
    }

    fn referencias(&self, uri: &str, line: usize, col: usize) -> Vec<serde_json::Value> {
        let Some(doc) = self.documentos.get(uri) else {
            return vec![];
        };
        let Some(nombre) = palabra_en(&doc.texto, line, col) else {
            return vec![];
        };
        let mut out = Vec::new();
        if let Some(sid) = self.simbodo_en_pos(doc, line, col) {
            let identidad = self.identidad_def_sem(doc, sid, uri);
            for (u, d) in &self.documentos {
                for def in &d.defs_sem {
                    let def_uri = uri_def_sem(def, u);
                    let misma = if let Some((ref_uri, ref_l, ref_c, ref_n)) = &identidad {
                        &def_uri == ref_uri
                            && def.linea == *ref_l
                            && def.columna == *ref_c
                            && &def.nombre == ref_n
                    } else {
                        u == uri && def.id == sid
                    };
                    if misma {
                        out.push(serde_json::json!({"uri": def_uri, "range": rango_lsp(def.linea, def.columna, def.linea_fin, def.columna_fin, &d.texto)}));
                    }
                }
                for r in &d.refs_sem {
                    let misma = if let Some(id_def) = identidad.as_ref() {
                        if let Some(def_r) = self.def_sem_por_id(d, r.id) {
                            let def_uri_r = uri_def_sem(def_r, u);
                            def_uri_r == id_def.0
                                && def_r.linea == id_def.1
                                && def_r.columna == id_def.2
                                && def_r.nombre == id_def.3
                        } else {
                            false
                        }
                    } else {
                        u == uri && r.id == sid
                    };
                    if misma {
                        out.push(serde_json::json!({"uri": u, "range": rango_lsp(r.linea, r.columna, r.linea_fin, r.columna_fin, &d.texto)}));
                    }
                }
            }
            out.sort_by_key(|a| a.to_string());
            out.dedup_by(|a, b| a == b);
            return out;
        }
        // Fallback AST auxiliar, sin adivinar por nombre entre ámbitos/documentos.
        let def_idx = self.def_idx_en_pos(doc, line, col, &nombre);
        let Some(di) = def_idx else {
            return vec![];
        };
        if let Some(s) = doc.simbolos.get(di) {
            out.push(serde_json::json!({"uri": uri, "range": rango_lsp(s.linea, s.columna, s.linea_fin, s.columna_fin, &doc.texto)}));
        }
        for r in doc
            .referencias
            .iter()
            .filter(|r| r.nombre == nombre && r.def == Some(di))
        {
            out.push(serde_json::json!({"uri": uri, "range": rango_lsp(r.linea, r.columna, r.linea, r.columna + nombre.chars().count(), &doc.texto)}));
        }
        out.sort_by_key(|a| a.to_string());
        out.dedup_by(|a, b| a == b);
        out
    }

    fn simbolos_documento(&self, uri: &str) -> Vec<serde_json::Value> {
        let Some(doc) = self.documentos.get(uri) else {
            return vec![];
        };
        doc.simbolos.iter().map(|s| serde_json::json!({
            "name": s.nombre, "kind": s.kind, "detail": s.detalle,
            "location": {"uri": uri, "range": rango_lsp(s.linea, s.columna, s.linea_fin, s.columna_fin, &doc.texto)}
        })).collect()
    }

    fn simbolos_workspace(&self, query: &str) -> Vec<serde_json::Value> {
        let q = query.to_lowercase();
        let mut out = Vec::new();
        for (uri, doc) in &self.documentos {
            for s in &doc.simbolos {
                if q.is_empty() || s.nombre.to_lowercase().contains(&q) {
                    out.push(serde_json::json!({"name": s.nombre, "kind": s.kind, "location": {"uri": uri, "range": rango_lsp(s.linea, s.columna, s.linea_fin, s.columna_fin, &doc.texto)}}));
                }
            }
        }
        out
    }

    fn scope_ancestros(
        &self,
        doc: &Documento,
        scope_id: usize,
    ) -> std::collections::HashSet<usize> {
        let mut set = std::collections::HashSet::new();
        let mut cur = Some(scope_id);
        // Build parent map from symbols: scope_id -> parent_scope
        let mut parent: std::collections::HashMap<usize, Option<usize>> =
            std::collections::HashMap::new();
        parent.insert(0, None);
        for s in &doc.simbolos {
            parent.entry(s.scope_id).or_insert(s.parent_scope);
        }
        let mut guard = 0;
        while let Some(c) = cur {
            if !set.insert(c) {
                break;
            }
            cur = parent.get(&c).copied().unwrap_or(None);
            guard += 1;
            if guard > 256 {
                break;
            }
        }
        set.insert(0);
        set
    }

    fn scope_en_pos(&self, doc: &Documento, line: usize, col_utf16: usize) -> usize {
        let lc = line + 1;
        let _cc = lsp_a_char_col(doc.texto.split('\n').nth(line).unwrap_or(""), col_utf16) + 1;
        // ref on/near cursor
        for r in &doc.referencias {
            if r.linea == lc {
                return r.scope_id;
            }
        }
        // def on line
        for s in &doc.simbolos {
            if s.linea == lc {
                return s.scope_id;
            }
        }
        // nearest previous symbol scope
        let mut best: Option<(usize, usize)> = None;
        for s in &doc.simbolos {
            if s.linea <= lc {
                let d = lc - s.linea;
                if best.map(|b| d <= b.0).unwrap_or(true) {
                    best = Some((d, s.scope_id));
                }
            }
        }
        best.map(|b| b.1).unwrap_or(0)
    }

    fn completar(&self, uri: &str, line: usize, col: usize) -> Vec<serde_json::Value> {
        let mut items: Vec<serde_json::Value> = vec![
            ("mostrar","función"),("sea","keyword"),("var","keyword"),("funcion","keyword"),("retornar","keyword"),("si","keyword"),("sino","keyword"),("mientras","keyword"),("para","keyword"),("cada","keyword"),("en","keyword"),("clase","keyword"),("estructura","keyword"),("enumeracion","keyword"),("rasgo","keyword"),("intentar","keyword"),("rescatar","keyword"),("verdadero","constante"),("falso","constante"),("nulo","constante")
        ].into_iter().map(|(l,k)| serde_json::json!({"label":l, "kind": if k=="keyword"{14}else if k=="constante"{21}else{3}, "detail":k})).collect();
        if let Some(doc) = self.documentos.get(uri) {
            let scope = self.scope_en_pos(doc, line, col);
            let visibles = self.scope_ancestros(doc, scope);
            for s in &doc.simbolos {
                if !visibles.contains(&s.scope_id) {
                    continue;
                }
                let mut detail = s.detalle.clone();
                // Enrich with semantic type when available
                if let Some(def) = doc
                    .defs_sem
                    .iter()
                    .find(|d| d.nombre == s.nombre && d.linea == s.linea && d.columna == s.columna)
                {
                    detail = def.tipo.nombre();
                }
                items
                    .push(serde_json::json!({"label": s.nombre, "kind": s.kind, "detail": detail}));
            }
            // Símbolos semánticos de nivel superior/importados visibles en el documento.
            for def in &doc.defs_sem {
                if def.ambito != crate::simbodo::AmbitoId(0) {
                    continue;
                }
                let mismo_doc = def
                    .archivo
                    .as_deref()
                    .map(ruta_a_uri)
                    .map(|u| u == uri)
                    .unwrap_or(true);
                let importado = def.visibilidad.as_deref() == Some("importado");
                if !mismo_doc && !importado {
                    continue;
                }
                let kind = match def.categoria {
                    crate::simbodo::CategoriaSimbodo::Funcion => 3u8,
                    crate::simbodo::CategoriaSimbodo::Variable => 6u8,
                    crate::simbodo::CategoriaSimbodo::Parametro => 6u8,
                    crate::simbodo::CategoriaSimbodo::Modulo => 9u8,
                    crate::simbodo::CategoriaSimbodo::Builtin => 3u8,
                };
                items.push(serde_json::json!({
                    "label": def.nombre,
                    "kind": kind,
                    "detail": def.tipo.nombre()
                }));
            }
        }
        let mut seen = std::collections::HashSet::new();
        items.retain(|i| {
            seen.insert(
                i.get("label")
                    .and_then(|l| l.as_str())
                    .unwrap_or("")
                    .to_string(),
            )
        });
        items
    }
}

fn pos_params(p: &serde_json::Value) -> (usize, usize) {
    let line = p
        .pointer("/position/line")
        .and_then(|l| l.as_u64())
        .unwrap_or(0) as usize;
    let ch = p
        .pointer("/position/character")
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as usize;
    (line, ch) // 0-based LSP UTF-16
}

fn rango_lsp(
    linea: usize,
    columna: usize,
    linea_fin: usize,
    columna_fin: usize,
    texto: &str,
) -> serde_json::Value {
    let ls = lineas_vec(texto);
    let start = pos_fuente_a_lsp(&ls, linea.saturating_sub(1), columna.saturating_sub(1));
    let end = pos_fuente_a_lsp(
        &ls,
        linea_fin.saturating_sub(1),
        columna_fin.saturating_sub(1),
    );
    serde_json::json!({"start": {"line": start.0, "character": start.1}, "end": {"line": end.0, "character": end.1}})
}

fn lineas_vec(texto: &str) -> Vec<String> {
    texto.split('\n').map(|s| s.to_string()).collect()
}

fn pos_fuente_a_lsp(lineas: &[String], line: usize, col_chars: usize) -> (usize, usize) {
    let line = line.min(lineas.len().saturating_sub(1));
    let utf16 = char_col_to_utf16(&lineas[line], col_chars);
    (line, utf16)
}

fn lsp_a_char_col(linea: &str, utf16_col: usize) -> usize {
    let mut u = 0usize;
    let mut chars_out = 0usize;
    for (chars, c) in linea.chars().enumerate() {
        if u >= utf16_col {
            return chars;
        }
        u += c.len_utf16();
        chars_out = chars + 1;
    }
    chars_out
}

fn char_col_to_utf16(linea: &str, char_col: usize) -> usize {
    let mut u = 0usize;
    for (chars, c) in linea.chars().enumerate() {
        if chars >= char_col {
            break;
        }
        u += c.len_utf16();
    }
    u
}

fn palabra_en(texto: &str, line: usize, col_utf16: usize) -> Option<String> {
    let lineas: Vec<&str> = texto.lines().collect();
    let l = lineas.get(line)?;
    let chars: Vec<char> = l.chars().collect();
    if chars.is_empty() {
        return None;
    }
    let col = lsp_a_char_col(l, col_utf16).min(chars.len().saturating_sub(1));
    let es_id = |c: char| c.is_alphanumeric() || c == '_';
    let mut i = col;
    if !chars.get(i).map(|c| es_id(*c)).unwrap_or(false) {
        if i == 0 {
            return None;
        }
        i -= 1;
        if !es_id(chars[i]) {
            return None;
        }
    }
    let mut s = i;
    while s > 0 && es_id(chars[s - 1]) {
        s -= 1;
    }
    let mut e = i;
    while e < chars.len() && es_id(chars[e]) {
        e += 1;
    }
    let w: String = chars[s..e].iter().collect();
    if w.is_empty() { None } else { Some(w) }
}

fn aplicar_cambio_incremental(texto: &str, rango: &serde_json::Value, nuevo: &str) -> String {
    let start_line = rango
        .pointer("/start/line")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let start_ch = rango
        .pointer("/start/character")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let end_line = rango
        .pointer("/end/line")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let end_ch = rango
        .pointer("/end/character")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let mut lineas: Vec<String> = texto.split('\n').map(|s| s.to_string()).collect();
    if lineas.is_empty() {
        lineas.push(String::new());
    }
    let start_line = start_line.min(lineas.len().saturating_sub(1));
    let end_line = end_line.min(lineas.len().saturating_sub(1));
    let start_col = utf16_to_byte(&lineas[start_line], start_ch);
    let end_col = utf16_to_byte(&lineas[end_line], end_ch);
    let mut prefix = lineas[start_line][..start_col].to_string();
    let suffix = lineas[end_line][end_col..].to_string();
    prefix.push_str(nuevo);
    prefix.push_str(&suffix);
    let mut nuevas: Vec<String> = Vec::new();
    nuevas.extend(lineas[..start_line].iter().cloned());
    nuevas.extend(prefix.split('\n').map(|s| s.to_string()));
    if end_line + 1 < lineas.len() {
        nuevas.extend(lineas[end_line + 1..].iter().cloned());
    }
    nuevas.join("\n")
}

fn utf16_to_byte(linea: &str, utf16_col: usize) -> usize {
    let mut u = 0usize;
    for (i, c) in linea.char_indices() {
        if u >= utf16_col {
            return i;
        }
        u += c.len_utf16();
    }
    linea.len()
}

fn indexar(texto: &str) -> (Vec<SimboloIdx>, Vec<RefIdx>) {
    let toks = match Lexer::nuevo(texto).tokenizar() {
        Ok(t) => t,
        Err(_) => return (vec![], vec![]),
    };
    let ast = match Parser::nuevo(toks).parsear_programa() {
        Ok(a) => a,
        Err(_) => return (vec![], vec![]),
    };
    let mut simbolos = Vec::new();
    let mut refs = Vec::new();
    let mut scopes: Vec<Vec<usize>> = vec![vec![]]; // scope_id -> symbol indices
    let mut scope_stack: Vec<usize> = vec![0];
    let mut next_scope = 1usize;
    for s in &ast {
        recolectar_sentencia(
            s,
            &mut simbolos,
            &mut refs,
            &mut scopes,
            &mut scope_stack,
            &mut next_scope,
        );
    }
    (simbolos, refs)
}

fn scope_actual(stack: &[usize]) -> usize {
    *stack.last().unwrap_or(&0)
}

#[allow(clippy::too_many_arguments)]
fn definir(
    simbolos: &mut Vec<SimboloIdx>,
    scopes: &mut Vec<Vec<usize>>,
    scope_id: usize,
    parent: Option<usize>,
    nombre: &str,
    kind: u8,
    detalle: &str,
    linea: usize,
    columna: usize,
) -> usize {
    let idx = simbolos.len();
    simbolos.push(SimboloIdx {
        nombre: nombre.to_string(),
        kind,
        detalle: detalle.to_string(),
        linea,
        columna,
        linea_fin: linea,
        columna_fin: columna + nombre.chars().count(),
        scope_id,
        parent_scope: parent,
    });
    if scope_id >= scopes.len() {
        scopes.resize(scope_id + 1, vec![]);
    }
    scopes[scope_id].push(idx);
    idx
}

fn resolver(
    nombre: &str,
    simbolos: &[SimboloIdx],
    scopes: &[Vec<usize>],
    stack: &[usize],
) -> Option<usize> {
    for &sid in stack.iter().rev() {
        if let Some(list) = scopes.get(sid) {
            for &si in list.iter().rev() {
                if simbolos[si].nombre == nombre {
                    return Some(si);
                }
            }
        }
    }
    None
}

fn push_ref(
    refs: &mut Vec<RefIdx>,
    simbolos: &[SimboloIdx],
    scopes: &[Vec<usize>],
    stack: &[usize],
    nombre: &str,
    linea: usize,
    columna: usize,
) {
    let def = resolver(nombre, simbolos, scopes, stack);
    refs.push(RefIdx {
        nombre: nombre.to_string(),
        linea,
        columna,
        scope_id: scope_actual(stack),
        def,
    });
}

fn entrar_scope(scopes: &mut Vec<Vec<usize>>, stack: &mut Vec<usize>, next: &mut usize) -> usize {
    let id = *next;
    *next += 1;
    if id >= scopes.len() {
        scopes.resize(id + 1, vec![]);
    }
    stack.push(id);
    id
}
fn salir_scope(stack: &mut Vec<usize>) {
    if stack.len() > 1 {
        stack.pop();
    }
}

fn recolectar_sentencia(
    s: &Sentencia,
    simbolos: &mut Vec<SimboloIdx>,
    refs: &mut Vec<RefIdx>,
    scopes: &mut Vec<Vec<usize>>,
    stack: &mut Vec<usize>,
    next: &mut usize,
) {
    match s {
        Sentencia::Funcion {
            nombre,
            linea,
            columna,
            parametros,
            tipo_retorno,
            cuerpo,
            ..
        } => {
            let det = format!(
                "fn({}){}",
                parametros
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                tipo_retorno.as_ref().map(|_| " -> ...").unwrap_or("")
            );
            let parent = scope_actual(stack);
            definir(
                simbolos,
                scopes,
                parent,
                Some(parent),
                nombre,
                12,
                &det,
                *linea,
                *columna,
            );
            let sid = entrar_scope(scopes, stack, next);
            for (pn, _) in parametros {
                definir(
                    simbolos,
                    scopes,
                    sid,
                    Some(parent),
                    pn,
                    13,
                    "parámetro",
                    *linea,
                    *columna,
                );
            }
            for st in cuerpo {
                recolectar_sentencia(st, simbolos, refs, scopes, stack, next);
            }
            salir_scope(stack);
        }
        Sentencia::Declaracion {
            nombre,
            linea,
            columna,
            valor,
            ..
        } => {
            recolectar_expr(valor, simbolos, refs, scopes, stack, next);
            let parent = scope_actual(stack);
            definir(
                simbolos,
                scopes,
                parent,
                Some(parent),
                nombre,
                13,
                "variable",
                *linea,
                *columna,
            );
        }
        Sentencia::Asignacion {
            nombre,
            linea,
            columna,
            valor,
        } => {
            push_ref(refs, simbolos, scopes, stack, nombre, *linea, *columna);
            recolectar_expr(valor, simbolos, refs, scopes, stack, next);
        }
        Sentencia::Estructura {
            nombre,
            linea,
            columna,
            ..
        } => {
            let p = scope_actual(stack);
            definir(
                simbolos,
                scopes,
                p,
                Some(p),
                nombre,
                23,
                "estructura",
                *linea,
                *columna,
            );
        }
        Sentencia::Enumeracion {
            nombre,
            linea,
            columna,
            ..
        } => {
            let p = scope_actual(stack);
            definir(
                simbolos,
                scopes,
                p,
                Some(p),
                nombre,
                10,
                "enumeración",
                *linea,
                *columna,
            );
        }
        Sentencia::Rasgo {
            nombre,
            linea,
            columna,
            ..
        } => {
            let p = scope_actual(stack);
            definir(
                simbolos,
                scopes,
                p,
                Some(p),
                nombre,
                11,
                "rasgo",
                *linea,
                *columna,
            );
        }
        Sentencia::Si {
            condicion,
            rama_si,
            rama_sino,
            ..
        } => {
            recolectar_expr(condicion, simbolos, refs, scopes, stack, next);
            entrar_scope(scopes, stack, next);
            for st in rama_si {
                recolectar_sentencia(st, simbolos, refs, scopes, stack, next);
            }
            salir_scope(stack);
            if let Some(rs) = rama_sino {
                entrar_scope(scopes, stack, next);
                for st in rs {
                    recolectar_sentencia(st, simbolos, refs, scopes, stack, next);
                }
                salir_scope(stack);
            }
        }
        Sentencia::Mientras {
            condicion, cuerpo, ..
        } => {
            recolectar_expr(condicion, simbolos, refs, scopes, stack, next);
            entrar_scope(scopes, stack, next);
            for st in cuerpo {
                recolectar_sentencia(st, simbolos, refs, scopes, stack, next);
            }
            salir_scope(stack);
        }
        Sentencia::ParaCada {
            variable,
            iterable,
            cuerpo,
            linea,
            columna,
            ..
        } => {
            recolectar_expr(iterable, simbolos, refs, scopes, stack, next);
            let sid = entrar_scope(scopes, stack, next);
            definir(
                simbolos,
                scopes,
                sid,
                Some(scope_actual(stack)),
                variable,
                13,
                "variable",
                *linea,
                *columna,
            );
            for st in cuerpo {
                recolectar_sentencia(st, simbolos, refs, scopes, stack, next);
            }
            salir_scope(stack);
        }
        Sentencia::Retorno { valor: Some(v), .. } => {
            recolectar_expr(v, simbolos, refs, scopes, stack, next)
        }
        Sentencia::Mostrar { valor, .. } | Sentencia::MostrarSinSalto { valor, .. } => {
            recolectar_expr(valor, simbolos, refs, scopes, stack, next)
        }
        Sentencia::Expresion(e) => recolectar_expr(e, simbolos, refs, scopes, stack, next),
        Sentencia::Bloque(b) => {
            entrar_scope(scopes, stack, next);
            for st in b {
                recolectar_sentencia(st, simbolos, refs, scopes, stack, next);
            }
            salir_scope(stack);
        }
        Sentencia::IntentarRescatar {
            cuerpo_intentar,
            variable_error,
            cuerpo_rescatar,
            ..
        } => {
            entrar_scope(scopes, stack, next);
            for st in cuerpo_intentar {
                recolectar_sentencia(st, simbolos, refs, scopes, stack, next);
            }
            salir_scope(stack);
            let sid = entrar_scope(scopes, stack, next);
            if let Some(v) = variable_error {
                definir(simbolos, scopes, sid, Some(sid), v, 13, "error", 1, 1);
            }
            for st in cuerpo_rescatar {
                recolectar_sentencia(st, simbolos, refs, scopes, stack, next);
            }
            salir_scope(stack);
        }
        _ => {}
    }
}

fn recolectar_expr(
    e: &Expresion,
    simbolos: &mut Vec<SimboloIdx>,
    refs: &mut Vec<RefIdx>,
    scopes: &mut Vec<Vec<usize>>,
    stack: &mut Vec<usize>,
    next: &mut usize,
) {
    match e {
        Expresion::Variable {
            nombre,
            linea,
            columna,
        } => push_ref(refs, simbolos, scopes, stack, nombre, *linea, *columna),
        Expresion::Llamada {
            nombre,
            argumentos,
            linea,
            columna,
            ..
        } => {
            push_ref(refs, simbolos, scopes, stack, nombre, *linea, *columna);
            for a in argumentos {
                recolectar_expr(a, simbolos, refs, scopes, stack, next);
            }
        }
        Expresion::LlamadaIndirecta {
            callee, argumentos, ..
        } => {
            recolectar_expr(callee, simbolos, refs, scopes, stack, next);
            for a in argumentos {
                recolectar_expr(a, simbolos, refs, scopes, stack, next);
            }
        }
        Expresion::Suma(a, b)
        | Expresion::Resta(a, b)
        | Expresion::Multiplicacion(a, b)
        | Expresion::Division(a, b)
        | Expresion::Modulo(a, b)
        | Expresion::IgualIgual(a, b)
        | Expresion::Distinto(a, b)
        | Expresion::Menor(a, b)
        | Expresion::MenorIgual(a, b)
        | Expresion::Mayor(a, b)
        | Expresion::MayorIgual(a, b)
        | Expresion::Y(a, b)
        | Expresion::O(a, b) => {
            recolectar_expr(a, simbolos, refs, scopes, stack, next);
            recolectar_expr(b, simbolos, refs, scopes, stack, next);
        }
        Expresion::No(a) => recolectar_expr(a, simbolos, refs, scopes, stack, next),
        Expresion::Indice { objeto, indice, .. } => {
            recolectar_expr(objeto, simbolos, refs, scopes, stack, next);
            recolectar_expr(indice, simbolos, refs, scopes, stack, next);
        }
        Expresion::Rango { inicio, fin, .. } => {
            recolectar_expr(inicio, simbolos, refs, scopes, stack, next);
            recolectar_expr(fin, simbolos, refs, scopes, stack, next);
        }
        Expresion::Coincidir {
            scrutinee, brazos, ..
        } => {
            recolectar_expr(scrutinee, simbolos, refs, scopes, stack, next);
            for b in brazos {
                entrar_scope(scopes, stack, next);
                for st in &b.cuerpo {
                    recolectar_sentencia(st, simbolos, refs, scopes, stack, next);
                }
                salir_scope(stack);
            }
        }
        Expresion::ConstruccionEstructura { campos, .. } => {
            for (_, v) in campos {
                recolectar_expr(v, simbolos, refs, scopes, stack, next);
            }
        }
        _ => {}
    }
}

fn respuesta(id: Option<serde_json::Value>, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"jsonrpc":"2.0","id": id, "result": result})
}
fn error_respuesta(id: Option<serde_json::Value>, code: i64, msg: &str) -> serde_json::Value {
    serde_json::json!({"jsonrpc":"2.0","id": id, "error":{"code":code,"message":msg}})
}
fn notificacion(method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"jsonrpc":"2.0","method": method, "params": params})
}

fn uri_a_ruta(uri: &str) -> Option<std::path::PathBuf> {
    let ruta = uri.strip_prefix("file://")?;
    let mut bytes = Vec::with_capacity(ruta.len());
    let b = ruta.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(hex) = std::str::from_utf8(&b[i + 1..i + 3])
            && let Ok(val) = u8::from_str_radix(hex, 16)
        {
            bytes.push(val);
            i += 3;
        } else {
            bytes.push(b[i]);
            i += 1;
        }
    }
    let decoded = String::from_utf8(bytes).ok()?;
    Some(std::path::PathBuf::from(decoded))
}

fn ruta_a_uri(ruta: &str) -> String {
    if ruta.starts_with("file://") {
        return ruta.to_string();
    }
    let p = std::path::Path::new(ruta);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    };
    format!("file://{}", abs.to_string_lossy())
}

fn uri_def_sem(def: &crate::semantico::DefinicionSemantica, uri_actual: &str) -> String {
    def.archivo
        .as_deref()
        .map(ruta_a_uri)
        .unwrap_or_else(|| uri_actual.to_string())
}

pub fn leer_mensaje<R: BufRead>(reader: &mut R) -> Result<Option<serde_json::Value>, String> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut linea = String::new();
        let n = reader.read_line(&mut linea).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(None);
        }
        let linea = linea.trim_end_matches(['\r', '\n']);
        if linea.is_empty() {
            break;
        }
        if let Some(resto) = linea.strip_prefix("Content-Length:") {
            if let Ok(v) = resto.trim().parse::<usize>() {
                content_length = Some(v);
            }
        } else if let Some(resto) = linea.strip_prefix("content-length:")
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
