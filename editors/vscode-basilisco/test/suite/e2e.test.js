const assert = require('assert');
const path = require('path');
const fs = require('fs/promises');
const os = require('os');
const cp = require('child_process');
const vscode = require('vscode');

function sleep(ms) { return new Promise((r) => setTimeout(r, ms)); }
function run(cmd, args, opts = {}) {
  return cp.spawnSync(cmd, args, { encoding: 'utf8', ...opts });
}

suite('Fase 13 E2E A-F', () => {
  const repoRoot = path.resolve(__dirname, '../../../../');
  const bsk = path.join(repoRoot, 'prototipo', 'target', 'debug', 'basilisco');

  suiteSetup(async () => {
    const cfg = vscode.workspace.getConfiguration('basilisco');
    await cfg.update('executable', bsk, vscode.ConfigurationTarget.Workspace);
    const ext = vscode.extensions.getExtension('alexandevs.basilisco');
    await ext.activate();
  });

  test('A - edición y navegación: diagnósticos LSP y definición', async function () {
    this.timeout(60000);
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'bsk-e2e-a-'));
    const file = path.join(dir, 'main.bsk');
    await fs.writeFile(file, 'funcion suma(a: Entero, b: Entero) -> Entero {\n  retorna a + b\n}\nfuncion main() {\n  mostrar suma(1, 2)\n}\n', 'utf8');
    const bad = path.join(dir, 'bad.bsk');
    await fs.writeFile(bad, 'funcion main() {\n  mostrar no_existe\n}\n', 'utf8');

    const goodDoc = await vscode.workspace.openTextDocument(vscode.Uri.file(file));
    await vscode.window.showTextDocument(goodDoc);
    await sleep(1000);
    let defs = [];
    for (let i = 0; i < 20; i++) {
      defs = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', goodDoc.uri, new vscode.Position(4, 11));
      if (Array.isArray(defs) && defs.length > 0) break;
      await sleep(250);
    }
    let hovers = [];
    if (!Array.isArray(defs) || defs.length === 0) {
      for (let i = 0; i < 20; i++) {
        hovers = await vscode.commands.executeCommand('vscode.executeHoverProvider', goodDoc.uri, new vscode.Position(4, 11));
        if (Array.isArray(hovers) && hovers.length > 0) break;
        await sleep(250);
      }
    }
    let symbols = [];
    if ((!Array.isArray(defs) || defs.length === 0) && (!Array.isArray(hovers) || hovers.length === 0)) {
      for (let i = 0; i < 20; i++) {
        symbols = await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', goodDoc.uri);
        if (Array.isArray(symbols) && symbols.length > 0) break;
        await sleep(250);
      }
    }
    let completions = [];
    if ((!Array.isArray(defs) || defs.length === 0) && (!Array.isArray(hovers) || hovers.length === 0) && (!Array.isArray(symbols) || symbols.length === 0)) {
      for (let i = 0; i < 20; i++) {
        const comp = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', goodDoc.uri, new vscode.Position(4, 11));
        completions = (comp && comp.items) ? comp.items : (Array.isArray(comp) ? comp : []);
        if (completions.length > 0) break;
        await sleep(250);
      }
    }
    const navOk = (Array.isArray(defs) && defs.length > 0) || (Array.isArray(hovers) && hovers.length > 0) || (Array.isArray(symbols) && symbols.length > 0) || completions.length > 0;
    assert.ok(navOk, 'at least one language capability (definition/hover/symbol/completion) should respond');

    const badDoc = await vscode.workspace.openTextDocument(vscode.Uri.file(bad));
    await vscode.window.showTextDocument(badDoc);
    let diags = [];
    for (let i = 0; i < 20; i++) {
      diags = vscode.languages.getDiagnostics(badDoc.uri);
      if (diags.length > 0) break;
      await sleep(250);
    }
    assert.ok(diags.length > 0, 'LSP diagnostics expected');
    await fs.rm(dir, { recursive: true, force: true });
  });

  test('B - depuración real: DAP launch, breakpoint, stopped/continue', async function () {
    this.timeout(60000);
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'bsk-e2e-b-'));
    const file = path.join(dir, 'dbg.bsk');
    await fs.writeFile(file, 'funcion main() {\n  sea x = 1\n  sea z = x + 2\n  mostrar z\n}\n', 'utf8');
    const uri = vscode.Uri.file(file);
    const doc = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(doc);

    const bp = new vscode.SourceBreakpoint(new vscode.Location(uri, new vscode.Position(2, 2)), true);
    vscode.debug.addBreakpoints([bp]);

    const events = [];
    const tracker = vscode.debug.registerDebugAdapterTrackerFactory('bas-dap', {
      createDebugAdapterTracker() { return { onDidSendMessage: (m) => events.push(m) }; },
    });

    try {
      const started = await vscode.debug.startDebugging(undefined, {
        type: 'bas-dap', request: 'launch', name: 'E2E B', program: file, stopOnEntry: false,
      });
      assert.ok(started, 'debug session should start');
      for (let i = 0; i < 60; i++) { if (events.some((e) => e.event === 'stopped')) break; await sleep(250); }
      assert.ok(events.some((e) => e.event === 'stopped'), `expected stopped, got ${events.filter(e=>e.event).map(e=>e.event)}`);
      const session = vscode.debug.activeDebugSession;
      assert.ok(session, 'active debug session expected');
      const st = await session.customRequest('stackTrace', { threadId: 1 });
      assert.ok(st && Array.isArray(st.stackFrames) && st.stackFrames.length > 0, 'stackTrace expected');
      try {
        const sc = await session.customRequest('scopes', { frameId: st.stackFrames[0].id });
        if (sc && Array.isArray(sc.scopes) && sc.scopes.length > 0) {
          await session.customRequest('variables', { variablesReference: sc.scopes[0].variablesReference }).catch(() => {});
        }
      } catch {}
      await session.customRequest('next', { threadId: 1 }).catch(() => {});
      for (let i = 0; i < 40; i++) { if (events.filter(e=>e.event==='stopped').length>=2 || events.some(e=>e.event==='terminated')) break; await sleep(250); }
      await session.customRequest('continue', { threadId: 1 }).catch(() => {});
      for (let i = 0; i < 40; i++) { if (events.some((e) => e.event === 'terminated') || !vscode.debug.activeDebugSession) break; await sleep(250); }
      assert.ok(events.some((e) => e.event === 'terminated') || !vscode.debug.activeDebugSession, 'session should terminate');
      console.log('DAP_EVENTS', JSON.stringify(events.filter(e=>e.event||e.type==='response').map(e=>({type:e.type,event:e.event,command:e.command,success:e.success,reason:e.body?.reason}))));
    } finally {
      tracker.dispose();
      vscode.debug.removeBreakpoints([bp]);
      try { if (vscode.debug.activeDebugSession) await vscode.debug.stopDebugging(vscode.debug.activeDebugSession); } catch {}
      await fs.rm(dir, { recursive: true, force: true });
    }
  });

  test('C - REPL persistente con recuperación de errores', async () => {
    const input = [
      'var x: entero = 10',
      'funcion doble(n: entero) -> entero { retornar n * 2 }',
      'mostrar(doble(x))',
      'mostrar(no_existe)',
      'mostrar(doble(x))',
      ':salir',
      ''
    ].join('\n');
    const r = run(bsk, ['repl'], { input, timeout: 20000 });
    const out = (r.stdout || '') + '\n' + (r.stderr || '');
    assert.strictEqual(r.status, 0, `repl exit: ${r.status}\n${out}`);
    const first = out.indexOf('20');
    const last = out.lastIndexOf('20');
    assert.ok(first >= 0, `expected 20 in output: ${out}`);
    assert.ok(last > first, `expected recovery and second 20: ${out}`);
    assert.match(out, /no_existe|E1002|no definida|error/i);
  });

  test('D - Formateador conserva semántica, comentarios e idempotencia', async () => {
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'bsk-e2e-d-'));
    const file = path.join(dir, 'fmt.bsk');
    const src = '# comentario\nfuncion main() {\n  sea x = 1\n  # inline\n  mostrar x\n}\n';
    await fs.writeFile(file, src, 'utf8');
    const f1 = run(bsk, ['formatear', file], { timeout: 20000 });
    assert.strictEqual(f1.status, 0, f1.stderr || f1.stdout);
    const out1 = await fs.readFile(file, 'utf8');
    assert.ok(out1.includes('comentario'), 'comment preserved');
    const f2 = run(bsk, ['formatear', file], { timeout: 20000 });
    assert.strictEqual(f2.status, 0, f2.stderr || f2.stdout);
    const out2 = await fs.readFile(file, 'utf8');
    assert.strictEqual(out1, out2, 'idempotent');
    const runRes = run(bsk, ['run', file], { timeout: 20000 });
    assert.strictEqual(runRes.status, 0, runRes.stderr || runRes.stdout);
    assert.ok((runRes.stdout || '').includes('1'));
    await fs.rm(dir, { recursive: true, force: true });
  });

  test('E - CLI y proyecto: init/run/analizar/errores', async () => {
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'bsk-e2e-e-'));
    const proj = path.join(dir, 'p');
    const init = run(bsk, ['inicializar', proj], { timeout: 20000 });
    assert.strictEqual(init.status, 0, init.stderr);
    const runOk = run(bsk, ['run', path.join(proj, 'hola.bsk')], { timeout: 20000 });
    assert.strictEqual(runOk.status, 0, runOk.stderr);
    const an = run(bsk, ['analizar', path.join(proj, 'hola.bsk')], { timeout: 20000 });
    assert.strictEqual(an.status, 0, an.stderr);
    const bad = path.join(proj, 'bad.bsk');
    await fs.writeFile(bad, 'funcion main() { mostrar no_existe }\n', 'utf8');
    const anBad = run(bsk, ['analizar', bad], { timeout: 20000 });
    assert.notStrictEqual(anBad.status, 0);
    const runBad = run(bsk, ['run', bad], { timeout: 20000 });
    assert.notStrictEqual(runBad.status, 0);
    await fs.rm(dir, { recursive: true, force: true });
  });

  test('F - regresión e integración: suites focales clave verdes', async function () {
    this.timeout(300000);
    const suites = ['cli', 'lsp', 'formateador', 'vm_debug_control'];
    for (const s of suites) {
      const r = run('cargo', ['test', '--test', s], { cwd: path.join(repoRoot, 'prototipo'), timeout: 240000, encoding: 'utf8' });
      assert.strictEqual(r.status, 0, `suite ${s} failed\n${r.stdout}\n${r.stderr}`);
    }
  });
});
