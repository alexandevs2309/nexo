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
