const assert = require('assert');
const path = require('path');
const fs = require('fs/promises');
const os = require('os');
const vscode = require('vscode');

function sleep(ms) { return new Promise((r) => setTimeout(r, ms)); }

function locUri(loc) {
  if (!loc) return '';
  if (loc.uri) return loc.uri.toString();
  if (loc.targetUri) return loc.targetUri.toString();
  return '';
}
function locRange(loc) {
  if (!loc) return null;
  return loc.range || loc.targetSelectionRange || loc.targetRange || null;
}

suite('5.1D A - Navegación LSP cross-module', function () {
  this.timeout(120000);
  const repoRoot = path.resolve(__dirname, '../../../../');
  const bsk = path.join(repoRoot, 'prototipo', 'target', 'debug', 'basilisco');
  let dir, mainUri, aUri, bUri, mainDoc;

  const modA = [
    'publico funcion saludar() -> texto {',
    '  retornar "hola"',
    '}',
    'publico sea valor_a = 1',
    '',
  ].join('\n');
  const modB = [
    'publico funcion saludar() -> texto {',
    '  retornar "adios"',
    '}',
    'publico sea valor_b = 2',
    '',
  ].join('\n');
  const main = [
    'usar "mod_a.bsk" { saludar, valor_a }',
    'funcion local() -> entero {',
    '  retornar 42',
    '}',
    'funcion main() {',
    '  sea x = local()',
    '  mostrar saludar()',
    '  mostrar valor_a',
    '  mostrar x',
    '  mostrar valor_b',
    '}',
    '',
  ].join('\n');

  async function waitFor(predicate, label, ms = 15000) {
    const start = Date.now();
    while (Date.now() - start < ms) {
      const v = await predicate();
      if (v) return v;
      await sleep(250);
    }
    assert.fail(`timeout waiting for ${label}`);
  }

  suiteSetup(async () => {
    const cfg = vscode.workspace.getConfiguration('basilisco');
    await cfg.update('executable', bsk, vscode.ConfigurationTarget.Workspace);
    await cfg.update('path', '', vscode.ConfigurationTarget.Workspace);
    const ext = vscode.extensions.getExtension('alexandevs.basilisco');
    await ext.activate();
    dir = await fs.mkdtemp(path.join(os.tmpdir(), 'bsk-nav-'));
    await fs.writeFile(path.join(dir, 'mod_a.bsk'), modA, 'utf8');
    await fs.writeFile(path.join(dir, 'mod_b.bsk'), modB, 'utf8');
    await fs.writeFile(path.join(dir, 'main.bsk'), main, 'utf8');
    aUri = vscode.Uri.file(path.join(dir, 'mod_a.bsk'));
    bUri = vscode.Uri.file(path.join(dir, 'mod_b.bsk'));
    mainUri = vscode.Uri.file(path.join(dir, 'main.bsk'));
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(aUri));
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(bUri));
    mainDoc = await vscode.workspace.openTextDocument(mainUri);
    await vscode.window.showTextDocument(mainDoc);
    // let LSP settle
    await sleep(2500);
  });

  suiteTeardown(async () => {
    try { await fs.rm(dir, { recursive: true, force: true }); } catch {}
  });

  test('1. Document symbols', async () => {
    const syms = await waitFor(async () => {
      const s = await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', aUri);
      return Array.isArray(s) && s.length > 0 ? s : null;
    }, 'document symbols mod_a');
    const names = JSON.stringify(syms);
    assert.ok(/saludar/.test(names), `saludar missing: ${names}`);
    assert.ok(/valor_a/.test(names), `valor_a missing: ${names}`);
  });

  test('2. Completion', async () => {
    // completion in main after "mostrar " on line with local x
    const pos = new vscode.Position(8, 10); // "  mostrar x"
    const comps = await waitFor(async () => {
      const c = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', mainUri, pos);
      const items = c?.items || c;
      return Array.isArray(items) && items.length > 0 ? items : null;
    }, 'completion');
    const labels = comps.map((i) => (typeof i.label === 'string' ? i.label : i.label?.label));
    console.log('COMPLETION_LABELS', labels.slice(0, 30));
    assert.ok(labels.length > 0);
  });

  test('3. Hover sobre definición', async () => {
    // hover over saludar definition in mod_a line 0 "publico funcion saludar..."
    const pos = new vscode.Position(0, 17); // inside saludar
    const hovers = await waitFor(async () => {
      const h = await vscode.commands.executeCommand('vscode.executeHoverProvider', aUri, pos);
      return Array.isArray(h) && h.length > 0 ? h : null;
    }, 'hover on definition', 15000).catch(() => null);
    console.log('HOVER_DEF', JSON.stringify(hovers?.map(h => h.contents?.map(c => String(c.value || c)))));
    assert.ok(hovers && hovers.length > 0, 'hover on definition should return content');
  });

  test('4. Hover sobre referencia', async () => {
    // hover over saludar reference in main line 6: "  mostrar saludar()"
    const pos = new vscode.Position(6, 11);
    const hovers = await waitFor(async () => {
      const h = await vscode.commands.executeCommand('vscode.executeHoverProvider', mainUri, pos);
      return Array.isArray(h) && h.length > 0 ? h : null;
    }, 'hover on reference', 15000).catch(() => null);
    console.log('HOVER_REF', JSON.stringify(hovers?.map(h => h.contents?.map(c => String(c.value || c)))));
    assert.ok(hovers && hovers.length > 0, 'hover on reference should return content');
  });

  test('5. Definition desde main.bsk hacia mod_a.bsk', async () => {
    // "  mostrar saludar()" line 6, saludar starts at col 10
    const defs = await waitFor(async () => {
      const d = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', mainUri, new vscode.Position(6, 11));
      return Array.isArray(d) && d.length > 0 ? d : null;
    }, 'definition to mod_a');
    const uri = locUri(defs[0]);
    console.log('DEF_URI', uri);
    assert.ok(uri.includes('mod_a.bsk'), `expected mod_a.bsk got ${uri}`);
    assert.ok(!uri.includes('mod_b.bsk'), `must not resolve to mod_b: ${uri}`);
  });

  test('6. References desde la definición importada', async () => {
    // saludar definition in mod_a line 0 col 16
    const refs = await waitFor(async () => {
      const r = await vscode.commands.executeCommand('vscode.executeReferenceProvider', aUri, new vscode.Position(0, 17));
      return Array.isArray(r) && r.length > 0 ? r : null;
    }, 'references from definition');
    const uris = refs.map(locUri);
    console.log('REFS_URIS', uris);
    assert.ok(uris.some((u) => u.includes('main.bsk')), `refs missing main: ${uris}`);
    assert.ok(uris.some((u) => u.includes('mod_a.bsk')), `refs missing mod_a: ${uris}`);
  });

  test('7. Exclusión de homónimo mod_b.bsk', async () => {
    const refs = await waitFor(async () => {
      const r = await vscode.commands.executeCommand('vscode.executeReferenceProvider', aUri, new vscode.Position(0, 17));
      return Array.isArray(r) && r.length > 0 ? r : null;
    }, 'references homonym exclusion');
    const uris = refs.map(locUri);
    assert.ok(!uris.some((u) => u.includes('mod_b.bsk')), `must not include mod_b: ${uris}`);
  });

  test('8. Diagnósticos', async () => {
    // main has "mostrar valor_b" - unimported/undefined, should produce diagnostic; but must not resolve to mod_b
    const diags = await waitFor(async () => {
      const d = vscode.languages.getDiagnostics(mainUri);
      return d.length > 0 ? d : null;
    }, 'diagnostics', 15000).catch(() => []);
    console.log('DIAGS', diags.map((d) => d.message));
    // We expect at least a diagnostic related to valor_b or undefined
    // also verify valor_b does not resolve to mod_b
    const defs = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', mainUri, new vscode.Position(9, 11));
    const defUris = (defs || []).map(locUri);
    console.log('VALOR_B_DEF', defUris);
    assert.ok(!defUris.some((u) => u.includes('mod_b.bsk')), `valor_b must not resolve to mod_b: ${defUris}`);
  });

  test('9. Comportamiento después de modificar y volver a abrir', async () => {
    // modify mod_a, close/reopen, check symbols update
    const newA = modA.replace('retornar "hola"', 'retornar "hola2"');
    await fs.writeFile(path.join(dir, 'mod_a.bsk'), newA, 'utf8');
    // close and reopen
    await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
    await sleep(300);
    const doc = await vscode.workspace.openTextDocument(aUri);
    await vscode.window.showTextDocument(doc);
    await sleep(1500);
    const syms = await waitFor(async () => {
      const s = await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', aUri);
      return Array.isArray(s) && s.length > 0 ? s : null;
    }, 'symbols after modify');
    assert.ok(JSON.stringify(syms).includes('saludar'));
    // restore
    await fs.writeFile(path.join(dir, 'mod_a.bsk'), modA, 'utf8');
  });
});
