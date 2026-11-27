//! Adaptador DAP mínimo para BASILISCO (stdio, JSON).

use std::collections::HashMap;
use std::io::BufReader;

use crate::vm::{
    BreakpointVm, EstadoDebugVm, EventoDepuracionVm, MaquinaVirtual, ModoPasoVm, MotivoParadaVm,
};
use crate::{Lexer, Parser, hir, mir, semantico::ContextoSemantico};

pub struct ServidorDap {
    seq: i64,
    vm: Option<MaquinaVirtual>,
    estado: Option<EstadoDebugVm>,
    breakpoints: HashMap<String, Vec<usize>>,
    launch_path: Option<String>,
}

impl ServidorDap {
    pub fn nuevo() -> Self {
        Self {
            seq: 1,
            vm: None,
            estado: None,
            breakpoints: HashMap::new(),
            launch_path: None,
        }
    }

    pub fn ejecutar(&mut self) {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut writer = stdout;
        loop {
            let msg = match crate::lsp::leer_mensaje(&mut reader) {
                Ok(Some(m)) => m,
                Ok(None) => break,
                Err(_) => break,
            };
            let respuestas = self.procesar(&msg);
            for r in respuestas {
                if crate::lsp::escribir_mensaje(&mut writer, &r).is_err() {
                    return;
                }
            }
            if msg.get("command").and_then(|c| c.as_str()) == Some("disconnect") {
                break;
            }
        }
    }

    fn procesar(&mut self, msg: &serde_json::Value) -> Vec<serde_json::Value> {
        let command = msg.get("command").and_then(|c| c.as_str()).unwrap_or("");
        let req_seq = msg.get("seq").and_then(|s| s.as_i64()).unwrap_or(0);
        let args = msg
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        match command {
            "initialize" => vec![
                self.respuesta(
                    req_seq,
                    command,
                    true,
                    serde_json::json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsSetVariable": false,
                        "supportsPauseRequest": true,
                        "supportsTerminateRequest": true,
                        "supportsStepBack": false
                    }),
                ),
                self.evento("initialized", serde_json::json!({})),
            ],
            "setBreakpoints" => {
                let source = args
                    .get("source")
                    .and_then(|s| s.get("path"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut bps_out = Vec::new();
                let mut lineas = Vec::new();
                if let Some(bps) = args.get("breakpoints").and_then(|b| b.as_array()) {
                    for bp in bps {
                        let line = bp.get("line").and_then(|l| l.as_u64()).unwrap_or(1) as usize;
                        lineas.push(line);
                        bps_out.push(serde_json::json!({"verified": true, "line": line}));
                    }
                }
                self.breakpoints.insert(source.clone(), lineas.clone());
                if let Some(vm) = &mut self.vm {
                    let mut todos = Vec::new();
                    for (f, ls) in &self.breakpoints {
                        for l in ls {
                            todos.push(BreakpointVm {
                                archivo: f.clone(),
                                linea: *l,
                            });
                        }
                    }
                    vm.debug_establecer_breakpoints(todos);
                }
                vec![self.respuesta(
                    req_seq,
                    command,
                    true,
                    serde_json::json!({"breakpoints": bps_out}),
                )]
            }
            "launch" => {
                let program = args
                    .get("program")
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string();
                self.launch_path = Some(program.clone());
                match self.preparar_vm(&program) {
                    Ok(()) => {
                        let mut out =
                            vec![self.respuesta(req_seq, command, true, serde_json::json!({}))];
                        let sin_mapa = self
                            .vm
                            .as_ref()
                            .map(|v| v.debug_spans.is_empty())
                            .unwrap_or(true);
                        if sin_mapa {
                            out.push(self.evento("output", serde_json::json!({"category":"console","output":"ADVERTENCIA: sourcemap real ausente; breakpoints por línea no verificados con precisión.\n"})));
                        }
                        out
                    }
                    Err(e) => vec![self.respuesta(
                        req_seq,
                        command,
                        false,
                        serde_json::json!({"message": e}),
                    )],
                }
            }
            "configurationDone" => {
                let mut out = vec![self.respuesta(req_seq, command, true, serde_json::json!({}))];
                let res = if let (Some(vm), Some(estado)) = (&mut self.vm, &mut self.estado) {
                    Some(vm.debug_continuar(estado))
                } else {
                    None
                };
                if let Some(r) = res {
                    out.extend(self.eventos_desde_resultado(r));
                }
                out
            }
            "threads" => vec![self.respuesta(
                req_seq,
                command,
                true,
                serde_json::json!({"threads":[{"id":1,"name":"main"}]}),
            )],
            "stackTrace" => {
                let frames = if let (Some(vm), Some(estado)) = (&self.vm, &self.estado) {
                    vm.debug_pila(estado).into_iter().enumerate().map(|(i,f)| serde_json::json!({
                        "id": i+1, "name": f.funcion,
                        "source": f.archivo.as_ref().map(|a| serde_json::json!({"path": a})),
                        "line": f.linea.unwrap_or(1), "column": f.columna.unwrap_or(1)
                    })).collect::<Vec<_>>()
                } else {
                    vec![]
                };
                vec![self.respuesta(
                    req_seq,
                    command,
                    true,
                    serde_json::json!({"stackFrames": frames, "totalFrames": frames.len()}),
                )]
            }
            "scopes" => {
                let frame_id = args.get("frameId").and_then(|v| v.as_i64()).unwrap_or(1);
                vec![self.respuesta(
                    req_seq,
                    command,
                    true,
                    serde_json::json!({"scopes":[
                        {"name":"Locales","variablesReference": frame_id*10+1,"expensive":false},
                        {"name":"Globales","variablesReference": 9999,"expensive":false}
                    ]}),
                )]
            }
            "variables" => {
                let vr = args
                    .get("variablesReference")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let vars = if let (Some(vm), Some(estado)) = (&self.vm, &self.estado) {
                    if vr == 9999 {
                        vm.debug_variables_globales()
                    } else {
                        let frame_idx = ((vr - 1) / 10).saturating_sub(1) as usize;
                        vm.debug_variables_locales(estado, frame_idx)
                    }
                } else {
                    vec![]
                };
                let vars_json: Vec<_> = vars.into_iter().map(|v| serde_json::json!({"name":v.nombre,"value":v.valor,"type":v.tipo,"variablesReference":0})).collect();
                vec![self.respuesta(
                    req_seq,
                    command,
                    true,
                    serde_json::json!({"variables": vars_json}),
                )]
            }
            "continue" => {
                let mut out = vec![self.respuesta(
                    req_seq,
                    command,
                    true,
                    serde_json::json!({"allThreadsContinued": true}),
                )];
                let res = if let (Some(vm), Some(estado)) = (&mut self.vm, &mut self.estado) {
                    vm.debug_modo_paso(ModoPasoVm::Ninguno);
                    Some(vm.debug_continuar(estado))
                } else {
                    None
                };
                if let Some(r) = res {
                    out.extend(self.eventos_desde_resultado(r));
                }
                out
            }
            "next" => {
                let mut out = vec![self.respuesta(req_seq, command, true, serde_json::json!({}))];
                let res = if let (Some(vm), Some(estado)) = (&mut self.vm, &mut self.estado) {
                    vm.debug_modo_paso(ModoPasoVm::StepOver {
                        profundidad: estado.frames.len(),
                    });
                    Some(vm.debug_continuar(estado))
                } else {
                    None
                };
                if let Some(r) = res {
                    out.extend(self.eventos_desde_resultado(r));
                }
                out
            }
            "stepIn" => {
                let mut out = vec![self.respuesta(req_seq, command, true, serde_json::json!({}))];
                let res = if let (Some(vm), Some(estado)) = (&mut self.vm, &mut self.estado) {
                    vm.debug_modo_paso(ModoPasoVm::StepIn);
                    Some(vm.debug_continuar(estado))
                } else {
                    None
                };
                if let Some(r) = res {
                    out.extend(self.eventos_desde_resultado(r));
                }
                out
            }
            "stepOut" => {
                let mut out = vec![self.respuesta(req_seq, command, true, serde_json::json!({}))];
                let res = if let (Some(vm), Some(estado)) = (&mut self.vm, &mut self.estado) {
                    vm.debug_modo_paso(ModoPasoVm::StepOut {
                        profundidad: estado.frames.len(),
                    });
                    Some(vm.debug_continuar(estado))
                } else {
                    None
                };
                if let Some(r) = res {
                    out.extend(self.eventos_desde_resultado(r));
                }
                out
            }
            "pause" => {
                if let Some(vm) = &mut self.vm {
                    vm.debug_pausar();
                }
                vec![self.respuesta(req_seq, command, true, serde_json::json!({}))]
            }
            "disconnect" | "terminate" => {
                self.vm = None;
                self.estado = None;
                vec![
                    self.respuesta(req_seq, command, true, serde_json::json!({})),
                    self.evento("terminated", serde_json::json!({})),
                ]
            }
            _ => vec![self.respuesta(
                req_seq,
                command,
                false,
                serde_json::json!({"message":"unsupported"}),
            )],
        }
    }

    fn preparar_vm(&mut self, path: &str) -> Result<(), String> {
        let fuente = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let toks = Lexer::nuevo(&fuente)
            .tokenizar()
            .map_err(|e| e.to_string())?;
        let ast = Parser::nuevo(toks)
            .parsear_programa()
            .map_err(|e| e.to_string())?;
        let mut ctx = ContextoSemantico::nuevo();
        let _ = ctx.analizar_programa(&ast);
        let h = hir::lower_programa_con_archivo(&ast, &ctx, Some(path.to_string()))
            .map_err(|e| format!("{:?}", e))?;
        let prog = mir::lower_programa(&h);
        let mut vm = MaquinaVirtual::nuevo(prog.clone());
        vm.registrar_stdlib();
        if !prog.source_map.is_empty() {
            vm.debug_spans = prog.source_map.clone();
        } else {
            // Sin sourcemap real: no fabricar ubicaciones sintéticas.
            // Los breakpoints por línea fuente no se pueden verificar con precisión.
        }
        let main_idx = vm
            .indice_de("main")
            .ok_or_else(|| "no hay main".to_string())?;
        let mut todos = Vec::new();
        for (f, ls) in &self.breakpoints {
            for l in ls {
                todos.push(BreakpointVm {
                    archivo: f.clone(),
                    linea: *l,
                });
            }
        }
        vm.debug_establecer_breakpoints(todos);
        let estado = vm.debug_iniciar(main_idx, &[]).map_err(|e| e.to_string())?;
        self.vm = Some(vm);
        self.estado = Some(estado);
        Ok(())
    }

    fn eventos_desde_resultado(
        &mut self,
        res: Result<EventoDepuracionVm, crate::vm::ErrorVm>,
    ) -> Vec<serde_json::Value> {
        match res {
            Ok(EventoDepuracionVm::Parado(m)) => {
                let reason = match &m {
                    MotivoParadaVm::Breakpoint { .. } => "breakpoint",
                    MotivoParadaVm::Step => "step",
                    MotivoParadaVm::Pause => "pause",
                    MotivoParadaVm::Entry => "entry",
                    _ => "pause",
                };
                vec![self.evento(
                    "stopped",
                    serde_json::json!({"reason": reason, "threadId": 1, "allThreadsStopped": true}),
                )]
            }
            Ok(EventoDepuracionVm::Terminado(r)) => {
                let mut out = Vec::new();
                if let Err(e) = r {
                    out.push(self.evento(
                        "output",
                        serde_json::json!({"category":"stderr","output": format!("{e}\n")}),
                    ));
                }
                out.push(self.evento("terminated", serde_json::json!({})));
                out
            }
            Err(e) => vec![
                self.evento(
                    "output",
                    serde_json::json!({"category":"stderr","output": format!("{e}\n")}),
                ),
                self.evento("terminated", serde_json::json!({})),
            ],
        }
    }

    fn respuesta(
        &mut self,
        req_seq: i64,
        command: &str,
        success: bool,
        body: serde_json::Value,
    ) -> serde_json::Value {
        let seq = self.seq;
        self.seq += 1;
        serde_json::json!({"seq": seq, "type":"response", "request_seq": req_seq, "success": success, "command": command, "body": body})
    }
    fn evento(&mut self, event: &str, body: serde_json::Value) -> serde_json::Value {
        let seq = self.seq;
        self.seq += 1;
        serde_json::json!({"seq": seq, "type":"event", "event": event, "body": body})
    }
}
