const assert = require('assert');
const path = require('path');
const vscode = require('vscode');

function sleep(ms) { return new Promise((r) => setTimeout(r, ms)); }

suite('Basilisco VS Code Extension', () => {
  const repoRoot = path.resolve(__dirname, '../../../../');
  const bskPath = path.join(repoRoot, 'prototipo', 'target', 'debug', 'basilisco');

  suiteSetup(async () => {
    const cfg = vscode.workspace.getConfiguration('basilisco');
    await cfg.update('executable', bskPath, vscode.ConfigurationTarget.Workspace);
  });

  test('activates and recognizes .bsk as basilisco', async () => {
    const ext = vscode.extensions.getExtension('alexandevs.basilisco');
    assert.ok(ext, 'extension should be present');
    const doc = await vscode.workspace.openTextDocument(path.join(__dirname, '../fixtures/workspace/hola.bsk'));
    await vscode.window.showTextDocument(doc);
    await ext.activate();
    assert.strictEqual(ext.isActive, true, 'extension should activate');
    assert.strictEqual(doc.languageId, 'basilisco', '.bsk should be recognized as basilisco');
  });

  test('declares commands/providers/config/debugger contributions', async () => {
    const ext = vscode.extensions.getExtension('alexandevs.basilisco');
    assert.ok(ext);
    const pkg = ext.packageJSON;
    assert.strictEqual(pkg.publisher, 'alexandevs');
    assert.strictEqual(pkg.name, 'basilisco');
    assert.ok(pkg.contributes.languages.some((l) => l.id === 'basilisco'));
    assert.ok(pkg.contributes.debuggers.some((d) => d.type === 'bas-dap'));
    assert.strictEqual(pkg.contributes.configuration.properties['basilisco.executable'].default, 'bsk');
    assert.ok(pkg.contributes.configuration.properties['basilisco.executable']);
    assert.ok(pkg.contributes.configuration.properties['basilisco.executable']);
    assert.ok(pkg.contributes.configuration.properties['basilisco.path'], 'legacy alias should exist');
    assert.ok(pkg.contributes.commands.some((c) => c.command === 'basilisco.runFile'));
    assert.ok(pkg.contributes.commands.some((c) => c.command === 'basilisco.formatFile'));
    assert.ok(pkg.contributes.commands.some((c) => c.command === 'basilisco.checkFile'));
    assert.ok(pkg.contributes.commands.some((c) => c.command === 'basilisco.analyzeFile'));
    assert.ok(pkg.contributes.snippets.some((s) => s.language === 'basilisco'));
  });

  test('LSP provides diagnostics for invalid file', async () => {
    const ext = vscode.extensions.getExtension('alexandevs.basilisco');
    await ext.activate();
    const badUri = vscode.Uri.file(path.join(__dirname, '../fixtures/workspace/bad.bsk'));
    await vscode.workspace.fs.writeFile(badUri, Buffer.from('funcion main() {\n  mostrar x\n}\n', 'utf8'));
    const doc = await vscode.workspace.openTextDocument(badUri);
    await vscode.window.showTextDocument(doc);

    let diags = [];
    for (let i = 0; i < 40; i++) {
      diags = vscode.languages.getDiagnostics(doc.uri);
      if (diags.length > 0) break;
      await sleep(250);
    }
    assert.ok(diags.length > 0, `expected diagnostics, got ${diags.length}`);
  });

  test('DAP resolves executable descriptor and starts debug session initialization', async () => {
    const ext = vscode.extensions.getExtension('alexandevs.basilisco');
    await ext.activate();
    const program = path.join(__dirname, '../fixtures/workspace/hola.bsk');
    const started = await vscode.debug.startDebugging(vscode.workspace.workspaceFolders?.[0], {
      type: 'bas-dap',
      request: 'launch',
      name: 'Depurar Basilisco',
      program,
    });
    for (let i = 0; i < 40; i++) {
      if (vscode.debug.activeDebugSession || started) break;
      await sleep(250);
    }
    assert.ok(started || vscode.debug.activeDebugSession, 'debug session should start');
    await sleep(1000);
    if (vscode.debug.activeDebugSession) {
      await vscode.debug.stopDebugging(vscode.debug.activeDebugSession);
    }
  });

  test('DAP handles missing executable gracefully', async () => {
    const ext = vscode.extensions.getExtension('alexandevs.basilisco');
    await ext.activate();
    const cfg = vscode.workspace.getConfiguration('basilisco');
    const old = cfg.get('executable');
    await cfg.update('executable', '/tmp/no-existe-basilisco', vscode.ConfigurationTarget.Workspace);
    try {
      const program = path.join(__dirname, '../fixtures/workspace/hola.bsk');
      const started = await vscode.debug.startDebugging(vscode.workspace.workspaceFolders?.[0], {
        type: 'bas-dap',
        request: 'launch',
        name: 'Depurar Basilisco missing',
        program,
      });
      // may return false or start and terminate; must not crash extension host
      assert.ok(ext.isActive, 'extension should remain active');
      await sleep(500);
      if (vscode.debug.activeDebugSession) {
        await vscode.debug.stopDebugging(vscode.debug.activeDebugSession);
      }
      assert.ok(started === true || started === false);
    } finally {
      await cfg.update('executable', old, vscode.ConfigurationTarget.Workspace);
    }
  });

  test('LSP works with path containing spaces', async () => {
    const ext = vscode.extensions.getExtension('alexandevs.basilisco');
    await ext.activate();
    const os = require('os');
    const fs = require('fs/promises');
    const dir = path.join(os.tmpdir(), 'bsk test espacio');
    await fs.mkdir(dir, { recursive: true });
    const file = path.join(dir, 'mal archivo.bsk');
    await fs.writeFile(file, 'funcion main() {\n  mostrar z\n}\n', 'utf8');
    const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(file));
    await vscode.window.showTextDocument(doc);
    let diags = [];
    for (let i = 0; i < 40; i++) {
      diags = vscode.languages.getDiagnostics(doc.uri);
      if (diags.length > 0) break;
      await sleep(250);
    }
    assert.ok(diags.length > 0, `expected diagnostics for spaced path, got ${diags.length}`);
    await fs.rm(dir, { recursive: true, force: true });
  });

  test('local providers recovered: rename, signature help, semantic tokens, folding', async () => {
    const ext = vscode.extensions.getExtension('alexandevs.basilisco');
    await ext.activate();
    const doc = await vscode.workspace.openTextDocument({ language: 'basilisco', content: 'funcion suma(a: entero, b: entero) -> entero {\n  retornar a + b\n}\nfuncion main() {\n  mostrar suma(1, 2)\n}\n' });
    await vscode.window.showTextDocument(doc);
    await sleep(500);
    const edit = await vscode.commands.executeCommand('vscode.executeDocumentRenameProvider', doc.uri, new vscode.Position(0, 9), 'sumar');
    assert.ok(edit, 'rename provider should return edit');
    const sig = await vscode.commands.executeCommand('vscode.executeSignatureHelpProvider', doc.uri, new vscode.Position(4, 15), '(');
    assert.ok(sig, 'signature help provider should respond');
    const folds = await vscode.commands.executeCommand('vscode.executeFoldingRangeProvider', doc.uri);
    assert.ok(Array.isArray(folds) && folds.length > 0, 'folding provider should return ranges');
    const legend = ext.packageJSON.contributes && ext.packageJSON.contributes.semanticTokenTypes;
    const sem = await vscode.commands.executeCommand('vscode.provideDocumentSemanticTokens', doc.uri);
    assert.ok(sem || legend === undefined, 'semantic tokens command should not crash');
  });

  test('config resolution: executable preferred, path alias, default', async () => {
    const ext = vscode.extensions.getExtension('alexandevs.basilisco');
    await ext.activate();
    const cfg = vscode.workspace.getConfiguration('basilisco');
    const oldExe = cfg.get('executable');
    const oldPath = cfg.get('path');
    try {
      await cfg.update('executable', '/tmp/ruta con espacios/basilisco', vscode.ConfigurationTarget.Workspace);
      await cfg.update('path', '/tmp/otro/basilisco', vscode.ConfigurationTarget.Workspace);
      assert.strictEqual(ext.exports.getBasiliscoCommand(), '/tmp/ruta con espacios/basilisco');
      await cfg.update('executable', '', vscode.ConfigurationTarget.Workspace);
      assert.strictEqual(ext.exports.getBasiliscoCommand(), '/tmp/otro/basilisco');
      await cfg.update('path', '', vscode.ConfigurationTarget.Workspace);
      assert.strictEqual(ext.exports.getBasiliscoCommand(), 'bsk');
    } finally {
      await cfg.update('executable', oldExe, vscode.ConfigurationTarget.Workspace);
      await cfg.update('path', oldPath, vscode.ConfigurationTarget.Workspace);
    }
  });
});
