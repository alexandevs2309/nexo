use std::io::{Read, Write};
use std::process::{ChildStdout, Command, Stdio};

fn bsk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_basilisco")
}
fn enviar(msg: &serde_json::Value) -> Vec<u8> {
    let s = msg.to_string();
    format!("Content-Length: {}\r\n\r\n{}", s.len(), s).into_bytes()
}
fn leer_uno(stdout: &mut ChildStdout) -> Option<serde_json::Value> {
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
        if header.len() > 65536 {
            return None;
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
    serde_json::from_slice(&body).ok()
}
fn leer_hasta(
    stdout: &mut ChildStdout,
    pred: impl Fn(&serde_json::Value) -> bool,
) -> Option<serde_json::Value> {
    for _ in 0..60 {
        let msg = leer_uno(stdout)?;
        if pred(&msg) {
            return Some(msg);
        }
    }
    None
}

#[test]
fn dap_flujo_completo_breakpoint_stack_vars_step_continue() {
    let dir = std::env::temp_dir().join(format!("bsk_dap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let prog = dir.join("main.bsk");
    std::fs::write(
        &prog,
        "funcion main() {\n  sea x = 1\n  sea r = x + 2\n  retornar r\n}\n",
    )
    .unwrap();

    let mut child = Command::new(bsk_bin())
        .arg("dap")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    let init = serde_json::json!({"seq":1,"type":"request","command":"initialize","arguments":{"adapterID":"basilisco"}});
    stdin.write_all(&enviar(&init)).unwrap();
    stdin.flush().unwrap();
    let init_resp = leer_hasta(&mut stdout, |m| {
        m.get("command").and_then(|c| c.as_str()) == Some("initialize")
    })
    .unwrap();
    assert_eq!(
        init_resp.get("success").and_then(|v| v.as_bool()),
        Some(true)
    );
    let _initialized = leer_hasta(&mut stdout, |m| {
        m.get("event").and_then(|e| e.as_str()) == Some("initialized")
    });

    let bp = serde_json::json!({"seq":2,"type":"request","command":"setBreakpoints","arguments":{"source":{"path":prog.to_string_lossy()},"breakpoints":[{"line":3}]}});
    stdin.write_all(&enviar(&bp)).unwrap();
    stdin.flush().unwrap();
    let bp_resp = leer_hasta(&mut stdout, |m| {
        m.get("command").and_then(|c| c.as_str()) == Some("setBreakpoints")
    })
    .unwrap();
    let bps = bp_resp
        .get("body")
        .and_then(|b| b.get("breakpoints"))
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!bps.is_empty());
    assert_eq!(bps[0].get("line").and_then(|v| v.as_u64()), Some(3));

    let launch = serde_json::json!({"seq":3,"type":"request","command":"launch","arguments":{"program":prog.to_string_lossy()}});
    stdin.write_all(&enviar(&launch)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_hasta(&mut stdout, |m| {
        m.get("command").and_then(|c| c.as_str()) == Some("launch")
    })
    .unwrap();

    let cfg =
        serde_json::json!({"seq":4,"type":"request","command":"configurationDone","arguments":{}});
    stdin.write_all(&enviar(&cfg)).unwrap();
    stdin.flush().unwrap();
    let _ = leer_hasta(&mut stdout, |m| {
        m.get("command").and_then(|c| c.as_str()) == Some("configurationDone")
    })
    .unwrap();
    let stopped = leer_hasta(&mut stdout, |m| {
        m.get("event").and_then(|e| e.as_str()) == Some("stopped")
            || m.get("event").and_then(|e| e.as_str()) == Some("terminated")
    })
    .unwrap();
    if stopped.get("event").and_then(|e| e.as_str()) == Some("stopped") {
        assert_eq!(
            stopped
                .get("body")
                .and_then(|b| b.get("reason"))
                .and_then(|r| r.as_str()),
            Some("breakpoint")
        );

        let st = serde_json::json!({"seq":5,"type":"request","command":"stackTrace","arguments":{"threadId":1}});
        stdin.write_all(&enviar(&st)).unwrap();
        stdin.flush().unwrap();
        let st_resp = leer_hasta(&mut stdout, |m| {
            m.get("command").and_then(|c| c.as_str()) == Some("stackTrace")
        })
        .unwrap();
        let frames = st_resp
            .get("body")
            .and_then(|b| b.get("stackFrames"))
            .and_then(|f| f.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(!frames.is_empty());
        assert_eq!(frames[0].get("name").and_then(|n| n.as_str()), Some("main"));

        let sc = serde_json::json!({"seq":6,"type":"request","command":"scopes","arguments":{"frameId":1}});
        stdin.write_all(&enviar(&sc)).unwrap();
        stdin.flush().unwrap();
        let sc_resp = leer_hasta(&mut stdout, |m| {
            m.get("command").and_then(|c| c.as_str()) == Some("scopes")
        })
        .unwrap();
        let scopes = sc_resp
            .get("body")
            .and_then(|b| b.get("scopes"))
            .and_then(|s| s.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(
            scopes
                .iter()
                .any(|s| s.get("name").and_then(|n| n.as_str()) == Some("Locales"))
        );

        let vr = scopes
            .iter()
            .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Locales"))
            .and_then(|s| s.get("variablesReference"))
            .and_then(|v| v.as_i64())
            .unwrap_or(11);
        let vars = serde_json::json!({"seq":7,"type":"request","command":"variables","arguments":{"variablesReference":vr}});
        stdin.write_all(&enviar(&vars)).unwrap();
        stdin.flush().unwrap();
        let vars_resp = leer_hasta(&mut stdout, |m| {
            m.get("command").and_then(|c| c.as_str()) == Some("variables")
        })
        .unwrap();
        let vars_arr = vars_resp
            .get("body")
            .and_then(|b| b.get("variables"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(!vars_arr.is_empty());

        let nx = serde_json::json!({"seq":8,"type":"request","command":"next","arguments":{"threadId":1}});
        stdin.write_all(&enviar(&nx)).unwrap();
        stdin.flush().unwrap();
        let _ = leer_hasta(&mut stdout, |m| {
            m.get("command").and_then(|c| c.as_str()) == Some("next")
        });
        let _ = leer_hasta(&mut stdout, |m| {
            m.get("event").and_then(|e| e.as_str()) == Some("stopped")
                || m.get("event").and_then(|e| e.as_str()) == Some("terminated")
        });

        let cont = serde_json::json!({"seq":9,"type":"request","command":"continue","arguments":{"threadId":1}});
        stdin.write_all(&enviar(&cont)).unwrap();
        stdin.flush().unwrap();
        let _ = leer_hasta(&mut stdout, |m| {
            m.get("command").and_then(|c| c.as_str()) == Some("continue")
        });
        let _ = leer_hasta(&mut stdout, |m| {
            m.get("event").and_then(|e| e.as_str()) == Some("terminated")
        });
    }

    let disc = serde_json::json!({"seq":10,"type":"request","command":"disconnect","arguments":{}});
    let _ = stdin.write_all(&enviar(&disc));
    let _ = stdin.flush();
    drop(stdin);
    let _ = child.wait();
    std::fs::remove_dir_all(&dir).ok();
}
