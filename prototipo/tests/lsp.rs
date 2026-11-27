use std::io::Write;
use std::process::{Command, Stdio};

fn bsk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_basilisco")
}

fn enviar(msg: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("Content-Length: {}\r\n\r\n", msg.len()).as_bytes());
    out.extend_from_slice(msg.as_bytes());
    out
}

fn leer_uno(stdout: &mut std::process::ChildStdout) -> Option<String> {
    use std::io::Read;
    // Leer cabecera
    let mut header = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        if stdout.read(&mut buf).ok()? == 0 {
            return None;
        }
        header.push(buf[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_str = String::from_utf8_lossy(&header);
    let len: usize = header_str
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok())?;
    let mut body = vec![0u8; len];
    stdout.read_exact(&mut body).ok()?;
    Some(String::from_utf8_lossy(&body).to_string())
}

#[test]
fn lsp_initialize_shutdown_exit() {
    let mut child = Command::new(bsk_bin())
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn lsp");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    stdin.write_all(&enviar(init)).unwrap();
    stdin.flush().unwrap();

    let resp = leer_uno(&mut stdout).expect("respuesta initialize");
    assert!(resp.contains("\"capabilities\""), "resp: {resp}");
    assert!(resp.contains("\"textDocumentSync\""), "resp: {resp}");

    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    stdin.write_all(&enviar(initialized)).unwrap();
    stdin.flush().unwrap();

    let shutdown = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}"#;
    stdin.write_all(&enviar(shutdown)).unwrap();
    stdin.flush().unwrap();
    let resp2 = leer_uno(&mut stdout).expect("respuesta shutdown");
    assert!(
        resp2.contains("\"result\":null") || resp2.contains("\"result\": null"),
        "resp2: {resp2}"
    );

    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":{}}"#;
    stdin.write_all(&enviar(exit)).unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn lsp_publica_diagnosticos() {
    let mut child = Command::new(bsk_bin())
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn lsp");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    stdin.write_all(&enviar(init)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);

    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    stdin.write_all(&enviar(initialized)).unwrap();
    stdin.flush().unwrap();

    // didOpen con código inválido (falta `retornar` en función con tipo).
    let src = "mostrar x_no_definida";
    let open_obj = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///t.bsk",
                "languageId": "basilisco",
                "version": 1,
                "text": src
            }
        }
    });
    let open = serde_json::to_string(&open_obj).unwrap();
    stdin.write_all(&enviar(&open)).unwrap();
    stdin.flush().unwrap();

    // Leer la notificación publishDiagnostics
    let notif = leer_uno(&mut stdout).expect("notificacion");
    assert!(notif.contains("publishDiagnostics"), "notif: {notif}");
    assert!(notif.contains("diagnostics"), "notif: {notif}");
    // Debe haber al menos un diagnóstico.
    assert!(notif.contains("\"severity\""), "notif: {notif}");

    let shutdown = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}"#;
    stdin.write_all(&enviar(shutdown)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":{}}"#;
    stdin.write_all(&enviar(exit)).unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    let _ = child.wait();
}

#[test]
fn lsp_completion() {
    let mut child = Command::new(bsk_bin())
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn lsp");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    stdin.write_all(&enviar(init)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);

    let comp = r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///t.bsk"},"position":{"line":0,"character":0}}}"#;
    stdin.write_all(&enviar(comp)).unwrap();
    stdin.flush().unwrap();
    let resp = leer_uno(&mut stdout).expect("completion");
    assert!(resp.contains("\"items\""), "resp: {resp}");
    assert!(resp.contains("mostrar"), "resp: {resp}");

    let shutdown = r#"{"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}"#;
    stdin.write_all(&enviar(shutdown)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":{}}"#;
    stdin.write_all(&enviar(exit)).unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    let _ = child.wait();
}

#[test]
fn lsp_definition_references_symbols() {
    let mut child = Command::new(bsk_bin())
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    stdin.write_all(&enviar(init)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    let open = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///t.bsk","languageId":"basilisco","version":1,"text":"funcion suma(a: entero) -> entero {\n  retornar a\n}\nfuncion main() {\n  retornar suma(1)\n}\n"}}});
    stdin.write_all(&enviar(&open.to_string())).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    let def = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///t.bsk"},"position":{"line":4,"character":12}}});
    stdin.write_all(&enviar(&def.to_string())).unwrap();
    stdin.flush().unwrap();
    let resp = leer_uno(&mut stdout).unwrap();
    assert!(resp.contains("\"uri\""), "def resp: {resp}");
    let refs = serde_json::json!({"jsonrpc":"2.0","id":3,"method":"textDocument/references","params":{"textDocument":{"uri":"file:///t.bsk"},"position":{"line":0,"character":9}}});
    stdin.write_all(&enviar(&refs.to_string())).unwrap();
    stdin.flush().unwrap();
    let r2 = leer_uno(&mut stdout).unwrap();
    assert!(r2.contains("\"range\""), "refs resp: {r2}");
    let syms = serde_json::json!({"jsonrpc":"2.0","id":4,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":"file:///t.bsk"}}});
    stdin.write_all(&enviar(&syms.to_string())).unwrap();
    stdin.flush().unwrap();
    let r3 = leer_uno(&mut stdout).unwrap();
    assert!(
        r3.contains("suma") && r3.contains("main"),
        "syms resp: {r3}"
    );
    let shutdown = r#"{"jsonrpc":"2.0","id":5,"method":"shutdown","params":null}"#;
    stdin.write_all(&enviar(shutdown)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":{}}"#;
    stdin.write_all(&enviar(exit)).unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    let _ = child.wait();
}

#[test]
fn lsp_scope_homonimos_y_workspace() {
    let mut child = Command::new(bsk_bin())
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    stdin.write_all(&enviar(init)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    let src = "funcion a() {\n  sea x = 1\n  retornar x\n}\nfuncion b() {\n  sea x = 2\n  retornar x\n}\n";
    let open = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///h.bsk","languageId":"basilisco","version":1,"text":src}}});
    stdin.write_all(&enviar(&open.to_string())).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    // references en x de funcion a (linea 2, col 11 aprox)
    let refs = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{"textDocument":{"uri":"file:///h.bsk"},"position":{"line":1,"character":6}}});
    stdin.write_all(&enviar(&refs.to_string())).unwrap();
    stdin.flush().unwrap();
    let r = leer_uno(&mut stdout).unwrap();
    // debe haber referencias pero no debe mezclar con x de b (2 usos en total: def+ref en a)
    let v: serde_json::Value = serde_json::from_str(&r).unwrap();
    let arr = v
        .get("result")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!arr.is_empty(), "refs: {r}");
    // workspace symbol
    let ws = serde_json::json!({"jsonrpc":"2.0","id":3,"method":"workspace/symbol","params":{"query":"a"}});
    stdin.write_all(&enviar(&ws.to_string())).unwrap();
    stdin.flush().unwrap();
    let rw = leer_uno(&mut stdout).unwrap();
    assert!(rw.contains("\"a\""), "ws: {rw}");
    let shutdown = r#"{"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}"#;
    stdin.write_all(&enviar(shutdown)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":{}}"#;
    stdin.write_all(&enviar(exit)).unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    let _ = child.wait();
}

#[test]
fn lsp_incremental_utf16_y_errores() {
    let mut child = Command::new(bsk_bin())
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    stdin.write_all(&enviar(init)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    // texto con acento y emoji
    let src = "funcion main() {\n  sea café = 1\n  retornar café\n}\n";
    let open = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///u.bsk","languageId":"basilisco","version":1,"text":src}}});
    stdin.write_all(&enviar(&open.to_string())).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    // cambio incremental: reemplazar 1 por 2 (línea 1, character 13 en UTF-16 aprox) - usamos rango simple al final de número
    let change = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///u.bsk","version":2},"contentChanges":[{"range":{"start":{"line":1,"character":13},"end":{"line":1,"character":14}},"text":"2"}]}});
    stdin.write_all(&enviar(&change.to_string())).unwrap();
    stdin.flush().unwrap();
    let diag = leer_uno(&mut stdout).unwrap();
    assert!(diag.contains("publishDiagnostics"), "diag: {diag}");
    // método desconocido
    let unk =
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"textDocument/noExiste","params":{}});
    stdin.write_all(&enviar(&unk.to_string())).unwrap();
    stdin.flush().unwrap();
    let ru = leer_uno(&mut stdout).unwrap();
    assert!(
        ru.contains("-32601") || ru.contains("no soportado"),
        "unk: {ru}"
    );
    // didClose limpia diagnósticos
    let close = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"file:///u.bsk"}}});
    stdin.write_all(&enviar(&close.to_string())).unwrap();
    stdin.flush().unwrap();
    let rc = leer_uno(&mut stdout).unwrap();
    assert!(
        rc.contains("publishDiagnostics") && rc.contains("[]"),
        "close: {rc}"
    );
    let shutdown = r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}"#;
    stdin.write_all(&enviar(shutdown)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":{}}"#;
    stdin.write_all(&enviar(exit)).unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    let _ = child.wait();
}

#[test]
fn lsp_multidoc_y_utf16_surrogate() {
    let mut child = Command::new(bsk_bin())
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    stdin.write_all(&enviar(init)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    let a = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///a.bsk","languageId":"basilisco","version":1,"text":"funcion foo() -> entero { retornar 1 }\n"}}});
    let b = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///b.bsk","languageId":"basilisco","version":1,"text":"funcion main() {\n  retornar foo()\n}\n"}}});
    stdin.write_all(&enviar(&a.to_string())).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    stdin.write_all(&enviar(&b.to_string())).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    // workspace symbols should see foo
    let ws = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"workspace/symbol","params":{"query":"foo"}});
    stdin.write_all(&enviar(&ws.to_string())).unwrap();
    stdin.flush().unwrap();
    let rw = leer_uno(&mut stdout).unwrap();
    assert!(rw.contains("foo"), "ws: {rw}");
    // UTF-16 surrogate: emoji is 2 UTF-16 code units
    let u = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///e.bsk","languageId":"basilisco","version":1,"text":"funcion main() {\n  sea x = \"😀\"\n  retornar 1\n}\n"}}});
    stdin.write_all(&enviar(&u.to_string())).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    // incremental change after emoji: replace 1 with 2 on line 2, character 11
    let ch = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///e.bsk","version":2},"contentChanges":[{"range":{"start":{"line":2,"character":11},"end":{"line":2,"character":12}},"text":"2"}]}});
    stdin.write_all(&enviar(&ch.to_string())).unwrap();
    stdin.flush().unwrap();
    let diag = leer_uno(&mut stdout).unwrap();
    assert!(diag.contains("publishDiagnostics"));
    let shutdown = r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}"#;
    stdin.write_all(&enviar(shutdown)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_uno(&mut stdout);
    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":{}}"#;
    stdin.write_all(&enviar(exit)).unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    let _ = child.wait();
}

#[test]
fn lsp_identidad_semantica_simbodoid() {
    use basilisco::lsp::ServidorLsp;
    use basilisco::{ContextoSemantico, Lexer, Parser};

    let src = r#"funcion suma(a: entero, b: entero) -> entero {
  sea x = a
  retornar x + b
}
funcion main() {
  sea x = suma(1, 2)
  mostrar(x)
}
"#;
    // Compilador: identidad real
    let toks = Lexer::nuevo(src).tokenizar().expect("lex");
    let ast = Parser::nuevo(toks).parsear_programa().expect("parse");
    let mut ctx = ContextoSemantico::nuevo();
    ctx.habilitar_registro_semantico_lsp();
    let errs = ctx.analizar_programa(&ast);
    assert!(errs.is_empty(), "errs: {:?}", errs);
    let defs = ctx.definiciones_semanticas();
    let refs = ctx.referencias_semanticas();
    assert!(!defs.is_empty());
    assert!(!refs.is_empty());

    // LSP
    let mut lsp = ServidorLsp::nuevo();
    let open = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///id.bsk","languageId":"basilisco","version":1,"text":src}}});
    let _ = lsp.procesar(&open);
    {
        let doc = lsp.documentos.get("file:///id.bsk").expect("doc");
        assert_eq!(doc.defs_sem.len(), defs.len());
        assert_eq!(doc.refs_sem.len(), refs.len());
        for r in refs {
            let lsp_r = doc
                .refs_sem
                .iter()
                .find(|x| x.linea == r.linea && x.columna == r.columna && x.id == r.id);
            assert!(lsp_r.is_some(), "ref faltante en LSP: {:?}", r);
        }
    }

    // Cada referencia del compilador debe resolverse en LSP al mismo SimbodoId/def
    for r in refs {
        let def = defs.get(&r.id).expect("def por id");
        let line0 = r.linea.saturating_sub(1);
        let col0_utf16 = {
            let l = src.split('\n').nth(line0).unwrap_or("");
            l.chars()
                .take(r.columna.saturating_sub(1))
                .map(|c| c.len_utf16())
                .sum::<usize>()
        };
        let msg = serde_json::json!({"jsonrpc":"2.0","id":10,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///id.bsk"},"position":{"line":line0,"character":col0_utf16}}});
        let resp = lsp.procesar(&msg);
        let res = resp[0]
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        assert!(!res.is_null(), "def null para ref {:?}", r);
        let rl = res
            .pointer("/range/start/line")
            .and_then(|v| v.as_u64())
            .unwrap() as usize
            + 1;
        let rc = {
            let l = src.split('\n').nth(rl - 1).unwrap_or("");
            let u16 = res
                .pointer("/range/start/character")
                .and_then(|v| v.as_u64())
                .unwrap() as usize;
            let mut u = 0usize;
            let mut chars = 0usize;
            for c in l.chars() {
                if u >= u16 {
                    break;
                }
                u += c.len_utf16();
                chars += 1;
            }
            chars + 1
        };
        assert_eq!(rl, def.linea, "linea def para {:?}", r);
        assert_eq!(rc, def.columna, "col def para {:?}", r);
    }

    // Homónimos: x en suma y x en main deben tener SimbodoId distintos
    let xs: Vec<_> = defs.values().filter(|d| d.nombre == "x").collect();
    assert!(xs.len() >= 2, "xs: {:?}", xs);
    let ids: std::collections::HashSet<_> = xs.iter().map(|d| d.id).collect();
    assert_eq!(ids.len(), xs.len(), "homónimos colisionan");
}

#[test]
fn lsp_semantico_hover_completion_no_resueltos() {
    use basilisco::lsp::ServidorLsp;
    let src = "funcion suma(a: entero, b: entero) -> entero {\n  retornar a + b\n}\nfuncion main() {\n  sea x = suma(1, 2)\n  mostrar(x)\n}\n";
    let mut lsp = ServidorLsp::nuevo();
    let open = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///h.bsk","languageId":"basilisco","version":1,"text":src}}});
    let _ = lsp.procesar(&open);
    // Hover en x (linea 4, 'x' after 'sea ') debe mostrar tipo entero real (inferido)
    let hover = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///h.bsk"},"position":{"line":4,"character":6}}});
    let rh = lsp.procesar(&hover);
    let hv = rh[0]
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let hs = hv.to_string();
    assert!(hs.contains("entero"), "hover sin tipo real: {hs}");
    assert!(hs.contains("x"), "hover sin nombre: {hs}");

    // Completion en main debe incluir x y suma, no inventar tipos
    let comp = serde_json::json!({"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///h.bsk"},"position":{"line":5,"character":2}}});
    let rc = lsp.procesar(&comp);
    let items = rc[0]
        .pointer("/result/items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let labels: Vec<String> = items
        .iter()
        .filter_map(|i| {
            i.get("label")
                .and_then(|l| l.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert!(
        labels.iter().any(|l| l == "x"),
        "completion sin x: {:?}",
        labels
    );
    assert!(
        labels.iter().any(|l| l == "suma"),
        "completion sin suma: {:?}",
        labels
    );

    // Símbolo no resuelto: no debe devolver definición arbitraria
    let src2 = "funcion main() {\n  mostrar(no_existe)\n}\n";
    let open2 = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///u.bsk","languageId":"basilisco","version":1,"text":src2}}});
    let _ = lsp.procesar(&open2);
    let def = serde_json::json!({"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///u.bsk"},"position":{"line":1,"character":10}}});
    let rd = lsp.procesar(&def);
    let dv = rd[0]
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    assert!(dv.is_null(), "no resuelto no debe resolverse: {dv}");
}

#[test]
fn lsp_utf16_protocol_matrix() {
    use basilisco::lsp::ServidorLsp;
    let mut lsp = ServidorLsp::nuevo();
    // Emoji/surrogate pair + incremental
    let src = "funcion main() {\n  sea s = \"😀a\"\n  retornar 1\n}\n";
    let open = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///e2.bsk","languageId":"basilisco","version":1,"text":src}}});
    let diags = lsp.procesar(&open);
    assert!(!diags.is_empty());
    // Cambio incremental: replace '1' (line 2, char 11) with '2'
    let ch = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///e2.bsk","version":2},"contentChanges":[{"range":{"start":{"line":2,"character":11},"end":{"line":2,"character":12}},"text":"2"}]}});
    let d2 = lsp.procesar(&ch);
    assert!(d2.iter().any(
        |m| m.get("method").and_then(|v| v.as_str()) == Some("textDocument/publishDiagnostics")
    ));

    // Acento en contenido (UTF-16: 'é' = 1 code unit)
    let src2 = "funcion main() {\n  sea s = \"café\"\n  mostrar(s)\n}\n";
    let open2 = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///acc.bsk","languageId":"basilisco","version":1,"text":src2}}});
    let dacc = lsp.procesar(&open2);
    let doc = lsp.documentos.get("file:///acc.bsk").unwrap();
    assert!(
        doc.simbolos.iter().any(|s| s.nombre == "s"),
        "syms={:?} diags={dacc:?}",
        doc.simbolos
            .iter()
            .map(|s| s.nombre.clone())
            .collect::<Vec<_>>()
    );

    // Rango inválido/fuera de límites no debe panic
    let bad = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///acc.bsk","version":3},"contentChanges":[{"range":{"start":{"line":99,"character":99},"end":{"line":100,"character":0}},"text":"x"}]}});
    let _ = lsp.procesar(&bad);

    // Cambio rápido/versiones
    for v in 4..8 {
        let chv = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///acc.bsk","version":v},"contentChanges":[{"text":"funcion main() { retornar 1 }\n"}]}});
        let _ = lsp.procesar(&chv);
    }

    // Multi-doc isolation
    let a = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///iso1.bsk","languageId":"basilisco","version":1,"text":"funcion alpha() -> entero { retornar 1 }\n"}}});
    let b = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///iso2.bsk","languageId":"basilisco","version":1,"text":"funcion beta() -> entero { retornar 2 }\n"}}});
    let _ = lsp.procesar(&a);
    let _ = lsp.procesar(&b);
    let doc1 = lsp.documentos.get("file:///iso1.bsk").unwrap();
    let doc2 = lsp.documentos.get("file:///iso2.bsk").unwrap();
    assert!(doc1.simbolos.iter().any(|s| s.nombre == "alpha"));
    assert!(!doc1.simbolos.iter().any(|s| s.nombre == "beta"));
    assert!(doc2.simbolos.iter().any(|s| s.nombre == "beta"));

    // Método desconocido con id -> error -32601
    let unk =
        serde_json::json!({"jsonrpc":"2.0","id":99,"method":"textDocument/noExiste","params":{}});
    let ru = lsp.procesar(&unk);
    assert_eq!(
        ru[0].pointer("/error/code").and_then(|v| v.as_i64()),
        Some(-32601)
    );

    // Parámetros inválidos no deben panic
    let badp = serde_json::json!({"jsonrpc":"2.0","id":100,"method":"textDocument/definition","params":{"textDocument":{}}});
    let _ = lsp.procesar(&badp);

    // didClose
    let close = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"file:///iso1.bsk"}}});
    let _ = lsp.procesar(&close);
    assert!(!lsp.documentos.contains_key("file:///iso1.bsk"));
}

#[test]
fn lsp_utf16_surrogate_pair_posicion_correcta() {
    use basilisco::lsp::ServidorLsp;
    let mut lsp = ServidorLsp::nuevo();
    // 😀 es 1 codepoint, 2 unidades UTF-16. La variable 'x' después del emoji
    // debe resolverse con columna UTF-16 correcta.
    let src = "funcion main() {\n  sea s = \"😀\"\n  sea x = 1\n  mostrar(x)\n}\n";
    let open = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///u16.bsk","languageId":"basilisco","version":1,"text":src}}});
    let _ = lsp.procesar(&open);
    // "  mostrar(x)" - line 3, 'x' is at char index 10 (0-based in line: "  mostrar(x)")
    // "  mostrar(x)" chars: space,space,m,o,s,t,r,a,r,(,x,) => x at char 10, UTF-16 10 (all ASCII)
    let def = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///u16.bsk"},"position":{"line":3,"character":10}}});
    let rd = lsp.procesar(&def);
    let dv = rd[0]
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    assert!(!dv.is_null(), "def x: {dv}");
    assert_eq!(
        dv.pointer("/range/start/line").and_then(|v| v.as_u64()),
        Some(2)
    );

    // Incremental change on line with emoji: replace '1' with '2' in "  sea x = 1"
    // line 2: "  sea x = 1" - '1' at char 10, UTF-16 10
    let ch = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///u16.bsk","version":2},"contentChanges":[{"range":{"start":{"line":2,"character":10},"end":{"line":2,"character":11}},"text":"2"}]}});
    let d2 = lsp.procesar(&ch);
    assert!(d2.iter().any(
        |m| m.get("method").and_then(|v| v.as_str()) == Some("textDocument/publishDiagnostics")
    ));
    let doc = lsp.documentos.get("file:///u16.bsk").unwrap();
    assert!(doc.texto.contains("sea x = 2"), "texto: {}", doc.texto);
}

#[test]
fn lsp_completion_respeta_scope() {
    use basilisco::lsp::ServidorLsp;
    let mut lsp = ServidorLsp::nuevo();
    let src = "funcion main() {\n  sea local = 1\n}\nfuncion otra() {\n  \n}\n";
    let open = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///scope.bsk","languageId":"basilisco","version":1,"text":src}}});
    let _ = lsp.procesar(&open);
    // Completion inside otra() - should not include 'local' from main
    let comp = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///scope.bsk"},"position":{"line":4,"character":2}}});
    let rc = lsp.procesar(&comp);
    let items = rc[0]
        .pointer("/result/items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let labels: Vec<String> = items
        .iter()
        .filter_map(|i| {
            i.get("label")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert!(
        !labels.iter().any(|l| l == "local"),
        "local no debe ser visible en otra(): {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l == "main"),
        "main debe ser visible: {labels:?}"
    );
}

#[test]
fn lsp_incremental_multi_doc_aislamiento() {
    use basilisco::lsp::ServidorLsp;
    let mut lsp = ServidorLsp::nuevo();
    let a = "funcion alfa() -> entero { retornar 1 }\n";
    let b = "funcion beta() -> entero { retornar 2 }\n";
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///a.bsk","languageId":"basilisco","version":1,"text":a}}}));
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///b.bsk","languageId":"basilisco","version":1,"text":b}}}));
    // Change A incrementally: '1' at char 36 in "funcion alfa() -> entero { retornar 1 }"
    let ch = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///a.bsk","version":2},"contentChanges":[{"range":{"start":{"line":0,"character":36},"end":{"line":0,"character":37}},"text":"9"}]}});
    let _ = lsp.procesar(&ch);
    let da = lsp.documentos.get("file:///a.bsk").unwrap();
    let db = lsp.documentos.get("file:///b.bsk").unwrap();
    assert!(da.texto.contains("retornar 9"), "A: {}", da.texto);
    assert!(db.texto.contains("retornar 2"), "B: {}", db.texto);
    assert!(db.simbolos.iter().any(|s| s.nombre == "beta"));
    assert!(!db.simbolos.iter().any(|s| s.nombre == "alfa"));
    // Close A, B still works
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"file:///a.bsk"}}}));
    assert!(!lsp.documentos.contains_key("file:///a.bsk"));
    assert!(lsp.documentos.contains_key("file:///b.bsk"));
    let syms = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":"file:///b.bsk"}}});
    let rs = lsp.procesar(&syms);
    let body = serde_json::to_string(&rs[0]).unwrap();
    assert!(body.contains("beta"), "syms: {body}");
}

#[test]
fn lsp_referencias_incluyen_definicion_y_uso_mismo_doc() {
    use basilisco::lsp::ServidorLsp;
    let mut lsp = ServidorLsp::nuevo();
    let src = "funcion suma(a: entero) -> entero {\n  retornar a\n}\nfuncion main() {\n  retornar suma(1)\n}\n";
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///r.bsk","languageId":"basilisco","version":1,"text":src}}}));
    // refs on suma definition (line 0, col 9)
    let refs = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"textDocument/references","params":{"textDocument":{"uri":"file:///r.bsk"},"position":{"line":0,"character":9}}});
    let rr = lsp.procesar(&refs);
    let arr = rr[0]
        .get("result")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(arr.len() >= 2, "refs suma: {arr:?}");
    // all in same uri
    for r in &arr {
        assert_eq!(r.get("uri").and_then(|v| v.as_str()), Some("file:///r.bsk"));
    }
}

#[test]
fn lsp_cross_doc_imports_definicion_y_referencias() {
    use basilisco::lsp::ServidorLsp;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("bsk_lsp_xdoc_{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    let mod_a = dir.join("mod_a.bsk");
    let mod_b = dir.join("mod_b.bsk");
    let mainf = dir.join("main.bsk");
    fs::write(
        &mod_a,
        "publico funcion sumar(a: entero, b: entero) -> entero {\n  retornar a + b\n}\n",
    )
    .unwrap();
    fs::write(
        &mod_b,
        "publico funcion sumar(a: entero, b: entero) -> entero {\n  retornar a * b\n}\n",
    )
    .unwrap();
    fs::write(
        &mainf,
        "usar \"mod_a.bsk\" { sumar }\nfuncion main() {\n  retornar sumar(1, 2)\n}\n",
    )
    .unwrap();
    let uri_a = format!("file://{}", mod_a.to_string_lossy());
    let uri_b = format!("file://{}", mod_b.to_string_lossy());
    let uri_m = format!("file://{}", mainf.to_string_lossy());

    let mut lsp = ServidorLsp::nuevo();
    let open_a = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri_a,"languageId":"basilisco","version":1,"text":fs::read_to_string(&mod_a).unwrap()}}});
    let open_b = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri_b,"languageId":"basilisco","version":1,"text":fs::read_to_string(&mod_b).unwrap()}}});
    let open_m = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri_m,"languageId":"basilisco","version":1,"text":fs::read_to_string(&mainf).unwrap()}}});
    let _ = lsp.procesar(&open_a);
    let _ = lsp.procesar(&open_b);
    let _ = lsp.procesar(&open_m);

    // Definición desde uso de sumar en main (line 2, "  retornar sumar(1, 2)")
    // "  retornar sumar(1, 2)" -> sumar starts at char 11
    let def = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"textDocument/definition","params":{"textDocument":{"uri":uri_m},"position":{"line":2,"character":11}}});
    let rd = lsp.procesar(&def);
    let dv = rd[0]
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let doc_m = lsp.documentos.get(&uri_m).unwrap();
    let defs: Vec<String> = doc_m
        .defs_sem
        .iter()
        .map(|d| {
            format!(
                "{}@{}:{} id={:?} file={:?}",
                d.nombre, d.linea, d.columna, d.id, d.archivo
            )
        })
        .collect();
    let refs: Vec<String> = doc_m
        .refs_sem
        .iter()
        .map(|r| {
            format!(
                "id={:?}@{}:{} file={:?}",
                r.id, r.linea, r.columna, r.archivo
            )
        })
        .collect();
    let syms: Vec<String> = doc_m
        .simbolos
        .iter()
        .map(|s| format!("{}@{}", s.nombre, s.linea))
        .collect();
    assert!(
        !dv.is_null(),
        "def sumar desde main: {dv}\ndefs={defs:?}\nrefs={refs:?}\nsyms={syms:?}\ntexto={:?}",
        doc_m.texto
    );
    let def_uri = dv.get("uri").and_then(|v| v.as_str()).unwrap_or("");
    assert!(def_uri.ends_with("mod_a.bsk"), "def uri={def_uri} dv={dv}");
    assert_ne!(def_uri, uri_b.as_str(), "no debe resolver a mod_b");

    // Referencias: deben incluir uso en main y no contaminación con mod_b
    let refs = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{"textDocument":{"uri":uri_m},"position":{"line":2,"character":11}}});
    let rr = lsp.procesar(&refs);
    let arr = rr[0]
        .get("result")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!arr.is_empty(), "refs: {arr:?}");
    let uris: Vec<String> = arr
        .iter()
        .filter_map(|r| r.get("uri").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();
    assert!(
        uris.iter().any(|u| u.ends_with("main.bsk")),
        "uris={uris:?}"
    );
    assert!(
        uris.iter().any(|u| u.ends_with("mod_a.bsk")),
        "uris={uris:?}"
    );
    assert!(
        !uris.iter().any(|u| u.ends_with("mod_b.bsk")),
        "no debe contaminar mod_b: {uris:?}"
    );

    // Cerrar mod_a, main sigue abierto; B no contaminado
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri_a}}}));
    assert!(!lsp.documentos.contains_key(&uri_a));
    assert!(lsp.documentos.contains_key(&uri_b));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn lsp_utf16_consultas_reales() {
    use basilisco::lsp::ServidorLsp;
    let mut lsp = ServidorLsp::nuevo();
    // Línea con emoji (2 UTF-16) antes de identificadores ASCII.
    let src = "funcion main() {\n  sea s = \"😀\"\n  sea x = s\n  mostrar(x)\n}\n";
    let uri = "file:///u16q.bsk";
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"languageId":"basilisco","version":1,"text":src}}}));
    // hover en 'x' de "  mostrar(x)" línea 3, char 10 (ASCII, sin desplazamiento por emoji en esa línea)
    let hover = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":3,"character":10}}}));
    let hv = hover[0]
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    assert!(!hv.is_null(), "hover x: {hv}");
    // references en 'x' definición línea 2, char 6
    let refs = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{"textDocument":{"uri":uri},"position":{"line":2,"character":6}}}));
    let arr = refs[0]
        .get("result")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(arr.len() >= 2, "refs x: {arr:?}");
    // completion en línea 3 col 2 debe incluir x (visible) y no incluir nombres inexistentes
    let comp = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":3,"character":2}}}));
    let items = comp[0]
        .pointer("/result/items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let labels: Vec<String> = items
        .iter()
        .filter_map(|i| {
            i.get("label")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert!(
        labels.iter().any(|l| l == "x"),
        "completion debe incluir x: {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l == "s"),
        "completion debe incluir s: {labels:?}"
    );
}

#[test]
fn lsp_visibilidad_completion_imports_publico_privado() {
    use basilisco::lsp::ServidorLsp;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("bsk_lsp_vis_{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    let modp = dir.join("modp.bsk");
    let mainf = dir.join("main.bsk");
    fs::write(&modp, "publico funcion pubf() -> entero { retornar 1 }\nfuncion privf() -> entero { retornar 2 }\n").unwrap();
    fs::write(
        &mainf,
        "usar \"modp.bsk\" { pubf }\nfuncion main() {\n  \n}\n",
    )
    .unwrap();
    let uri_m = format!("file://{}", mainf.to_string_lossy());
    let mut lsp = ServidorLsp::nuevo();
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri_m,"languageId":"basilisco","version":1,"text":fs::read_to_string(&mainf).unwrap()}}}));
    let comp = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"textDocument/completion","params":{"textDocument":{"uri":uri_m},"position":{"line":2,"character":2}}}));
    let items = comp[0]
        .pointer("/result/items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let labels: Vec<String> = items
        .iter()
        .filter_map(|i| {
            i.get("label")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert!(
        labels.iter().any(|l| l == "pubf"),
        "pubf visible: {labels:?}"
    );
    assert!(
        !labels.iter().any(|l| l == "privf"),
        "privf no debe ser visible: {labels:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn lsp_incremental_secuencia_multidoc() {
    use basilisco::lsp::ServidorLsp;
    let mut lsp = ServidorLsp::nuevo();
    let uri_a = "file:///seq_a.bsk";
    let uri_b = "file:///seq_b.bsk";
    let a1 = "funcion alfa() -> entero { retornar 1 }\n";
    let b1 = "funcion beta() -> entero { retornar 2 }\n";
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri_a,"languageId":"basilisco","version":1,"text":a1}}}));
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri_b,"languageId":"basilisco","version":1,"text":b1}}}));
    // Estado inicial
    let syma = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri_a}}}));
    assert!(serde_json::to_string(&syma[0]).unwrap().contains("alfa"));
    // Cambiar A incrementalmente
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri_a,"version":2},"contentChanges":[{"range":{"start":{"line":0,"character":36},"end":{"line":0,"character":37}},"text":"9"}]}}));
    // Consultas post-cambio en A
    let syma2 = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","id":2,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri_a}}}));
    assert!(serde_json::to_string(&syma2[0]).unwrap().contains("alfa"));
    let compa = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":uri_a},"position":{"line":0,"character":0}}}));
    assert!(compa[0].pointer("/result/items").is_some());
    // B conserva estado
    let symb = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","id":4,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri_b}}}));
    let sb = serde_json::to_string(&symb[0]).unwrap();
    assert!(sb.contains("beta") && !sb.contains("alfa"));
    // Otro cambio incremental en A
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri_a,"version":3},"contentChanges":[{"range":{"start":{"line":0,"character":36},"end":{"line":0,"character":37}},"text":"7"}]}}));
    assert!(
        lsp.documentos
            .get(uri_a)
            .unwrap()
            .texto
            .contains("retornar 7")
    );
    // Cerrar A, reabrir A
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri_a}}}));
    assert!(!lsp.documentos.contains_key(uri_a));
    let _ = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri_a,"languageId":"basilisco","version":1,"text":"funcion alfa2() -> entero { retornar 5 }\n"}}}));
    let syma3 = lsp.procesar(&serde_json::json!({"jsonrpc":"2.0","id":5,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri_a}}}));
    let sa3 = serde_json::to_string(&syma3[0]).unwrap();
    assert!(sa3.contains("alfa2") && !sa3.contains("retornar 7"));
}
