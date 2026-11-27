const vscode = require('vscode');
const cp = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

let client;
let output;
const KEYWORDS = new Set([
  'sea','var','dato','const','funcion','retornar','mostrar','mostrar_sin_salto',
  'si','sino','mientras','para','cada','en','romper','continuar','intentar',
  'capturar','rescatar','importar','exportar','usar','como','publico','privado',
  'verdadero','falso','nulo','y','o','no','clase','estructura','enumeracion','rasgo','implementar'
]);
const BUILTINS = new Set(['mostrar','mostrar_sin_salto','longitud','tipo','texto','entero','decimal','real','logico','rango','agregar','quitar','leer','escribir']);
const tokenTypes = ['keyword','function','variable','parameter','number','string','comment','operator','type'];
const tokenModifiers = ['declaration','readonly','builtin'];
const legend = new vscode.SemanticTokensLegend(tokenTypes, tokenModifiers);

function getBasiliscoCommand() {
  const config = vscode.workspace.getConfiguration('basilisco');
  const exeInspect = config.inspect('executable');
  const explicitExe = [exeInspect?.workspaceFolderValue, exeInspect?.workspaceValue, exeInspect?.globalValue]
    .find((v) => typeof v === 'string' && v.trim().length > 0);
  if (explicitExe) return explicitExe.trim();
  const pathInspect = config.inspect('path');
  const explicitPath = [pathInspect?.workspaceFolderValue, pathInspect?.workspaceValue, pathInspect?.globalValue]
    .find((v) => typeof v === 'string' && v.trim().length > 0);
  if (explicitPath) return explicitPath.trim();
  const exe = (config.get('executable') || '').trim();
  if (exe) return exe;
  const legacy = (config.get('path') || '').trim();
  if (legacy) return legacy;
  return 'bsk';
}

function runBasilisco(args, cwd) {
  const cmd = getBasiliscoCommand();
  const extra = vscode.workspace.getConfiguration('basilisco').get('extraArgs', []);
  return cp.spawnSync(cmd, [...(Array.isArray(extra)?extra:[]), ...args], { cwd, encoding: 'utf8', env: process.env });
}

function isBasilisco(doc) {
  return doc && (doc.languageId === 'basilisco' || ['.bsk','.nexo'].includes(path.extname(doc.fileName).toLowerCase()));
}

function wordAt(document, position) {
  const range = document.getWordRangeAtPosition(position, /[\p{L}_][\p{L}\p{N}_]*/u);
  return range ? { range, word: document.getText(range) } : null;
}

function scanSymbols(document) {
  const text = document.getText();
  const symbols = [];
  const declarations = [];
  const re = /^\s*(?:(?:publico)\s+)?funcion\s+([A-Za-z_]\w*)\s*\(([^)]*)\)|^\s*(sea|var|dato|const)\s+([A-Za-z_]\w*)\s*(?::\s*([A-Za-z_][\w<> ,]*)\s*)?=/gm;
  let m;
  while ((m = re.exec(text))) {
    const isFn = !!m[1];
    const name = isFn ? m[1] : m[3];
    const offset = m.index + m[0].indexOf(name);
    const pos = document.positionAt(offset);
    const end = document.positionAt(offset + name.length);
    const kind = isFn ? vscode.SymbolKind.Function : (m[2] === 'const' || m[2] === 'dato' ? vscode.SymbolKind.Constant : vscode.SymbolKind.Variable);
    const detail = isFn ? `funcion ${name}(${m[2] || ''})` : `${m[2] || 'variable'}${m[4] ? ': ' + m[4] : ''}`;
    const sym = new vscode.DocumentSymbol(name, detail, kind, new vscode.Range(document.positionAt(m.index), end), new vscode.Range(pos, end));
    symbols.push(sym);
    declarations.push({ name, range: new vscode.Range(pos, end), kind, detail });
  }
  return { symbols, declarations };
}

function allOccurrences(document, name) {
  const result = [];
  const re = new RegExp(`\\b${name.replace(/[.*+?^${}()|[\\]\\]/g, '\\$&')}\\b`, 'g');
  let m;
  const text = document.getText();
  while ((m = re.exec(text))) result.push(new vscode.Location(document.uri, new vscode.Range(document.positionAt(m.index), document.positionAt(m.index + name.length))));
  return result;
}

function activate(context) {
  output = vscode.window.createOutputChannel('Basilisco');
  context.subscriptions.push(output);

  const cmd = getBasiliscoCommand();
  const serverOptions = { command: cmd, args: ['lsp'], transport: TransportKind.stdio, options: { env: process.env } };
  const clientOptions = { documentSelector: [{ scheme: 'file', language: 'basilisco' }], outputChannel: output };
  client = new LanguageClient('basilisco', 'Basilisco LSP', serverOptions, clientOptions);
  context.subscriptions.push({ dispose: () => { if (client) { const p = client.stop(); client = undefined; return p; } return undefined; } });
  client.start().catch((err) => output.appendLine(`Error iniciando LSP: ${String(err && err.message ? err.message : err)}`));

  context.subscriptions.push(vscode.debug.registerDebugAdapterDescriptorFactory('bas-dap', {
    createDebugAdapterDescriptor() { return new vscode.DebugAdapterExecutable(getBasiliscoCommand(), ['dap'], { env: process.env }); },
  }));

  const terminal = () => { let t = vscode.window.terminals.find(x => x.name === 'BASILISCO'); if (!t) t = vscode.window.createTerminal('BASILISCO'); t.show(); return t; };
  const send = (cmdName, target) => { const args = [cmdName]; if (target) args.push(target); terminal().sendText([getBasiliscoCommand(), ...args].map(x => `"${String(x).replace(/"/g, '\\"')}"`).join(' ')); };

  const register = (id, fn) => context.subscriptions.push(vscode.commands.registerCommand(id, fn));
  register('basilisco.runFile', async () => { const e = vscode.window.activeTextEditor; if (!e || !isBasilisco(e.document)) return; await e.document.save(); send('run', e.document.fileName); });
  register('basilisco.analyzeFile', async () => { const e = vscode.window.activeTextEditor; if (!e || !isBasilisco(e.document)) return; await e.document.save(); const r = runBasilisco(['analizar', e.document.fileName], path.dirname(e.document.fileName)); output.appendLine(r.stdout || ''); if (r.stderr) output.appendLine(r.stderr); output.show(true); });
  register('basilisco.checkFile', async () => vscode.commands.executeCommand('basilisco.analyzeFile'));
  register('basilisco.formatFile', async () => { const e = vscode.window.activeTextEditor; if (!e || !isBasilisco(e.document)) return; await vscode.commands.executeCommand('editor.action.formatDocument'); });
  register('basilisco.testProject', () => { const f = vscode.workspace.workspaceFolders?.[0]; if (f) send('probar', f.uri.fsPath); });
  register('basilisco.initProject', () => { const f = vscode.workspace.workspaceFolders?.[0]; if (f) send('inicializar', f.uri.fsPath); });
  register('basilisco.listPackages', () => { const f = vscode.workspace.workspaceFolders?.[0]; if (f) send('paquetes', f.uri.fsPath); });
  register('basilisco.checkInstallation', async () => { const r = runBasilisco(['--version'], vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || process.cwd()); if (r.status === 0) vscode.window.showInformationMessage((r.stdout || 'BASILISCO detectado').trim()); else vscode.window.showErrorMessage(`No se pudo ejecutar ${getBasiliscoCommand()}: ${r.stderr || r.error?.message || ''}`); });
  register('basilisco.restartDiagnostics', async () => { if (client) { await client.restart(); } });
  register('basilisco.showOutput', () => output.show(true));

  // Formatter (CLI) - LSP does not provide formatting
  context.subscriptions.push(vscode.languages.registerDocumentFormattingEditProvider('basilisco', {
    provideDocumentFormattingEdits(document) {
      const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'bsk-fmt-'));
      const tmp = path.join(tmpDir, 'tmp.bsk');
      try {
        fs.writeFileSync(tmp, document.getText(), 'utf8');
        const r = runBasilisco(['formatear', tmp], tmpDir);
        if (r.status !== 0) { output.appendLine(`Formato falló: ${r.stderr || r.stdout}`); return []; }
        const formatted = fs.readFileSync(tmp, 'utf8');
        const fullRange = new vscode.Range(document.positionAt(0), document.positionAt(document.getText().length));
        return [vscode.TextEdit.replace(fullRange, formatted)];
      } finally { try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch {} }
    },
  }));

  // Local providers for capabilities NOT provided by LSP: rename, signature help, semantic tokens, folding.
  // Do NOT duplicate LSP-provided: completion, hover, definition, references, documentSymbol, diagnostics.
  context.subscriptions.push(vscode.languages.registerRenameProvider({ language: 'basilisco' }, {
    provideRenameEdits(doc, pos, newName) {
      const w = wordAt(doc, pos);
      if (!w || !/^[_\p{L}][_\p{L}\p{N}]*$/u.test(newName)) throw new Error('Nombre BASILISCO no válido.');
      const edit = new vscode.WorkspaceEdit();
      for (const loc of allOccurrences(doc, w.word)) edit.replace(loc.uri, loc.range, newName);
      return edit;
    },
  }));

  context.subscriptions.push(vscode.languages.registerSignatureHelpProvider({ language: 'basilisco' }, {
    provideSignatureHelp(doc, pos) {
      const before = doc.getText(new vscode.Range(new vscode.Position(0, 0), pos));
      const m = before.match(/([A-Za-z_]\w*)\s*\(([^()]*)$/);
      if (!m) return;
      const decl = scanSymbols(doc).declarations.find(x => x.name === m[1] && x.kind === vscode.SymbolKind.Function);
      if (!decl) return;
      const line = doc.lineAt(decl.range.start.line).text;
      const params = (line.match(new RegExp(`funcion\\s+${m[1]}\\s*\\(([^)]*)\\)`)) || [])[1] || '';
      const sig = new vscode.SignatureInformation(`${m[1]}(${params})`);
      sig.parameters = params.split(',').map(x => x.trim()).filter(Boolean).map(x => new vscode.ParameterInformation(x));
      const help = new vscode.SignatureHelp();
      help.signatures = [sig]; help.activeSignature = 0; help.activeParameter = Math.max(0, (m[2].match(/,/g) || []).length);
      return help;
    },
  }, '(', ','));

  context.subscriptions.push(vscode.languages.registerFoldingRangeProvider({ language: 'basilisco' }, {
    provideFoldingRanges(doc) {
      const stack = []; const ranges = [];
      for (let i = 0; i < doc.lineCount; i++) {
        const line = doc.lineAt(i).text;
        for (const c of line) { if (c === '{') stack.push(i); else if (c === '}' && stack.length) { const start = stack.pop(); if (i > start) ranges.push(new vscode.FoldingRange(start, i, vscode.FoldingRangeKind.Region)); } }
      }
      return ranges;
    },
  }));

  context.subscriptions.push(vscode.languages.registerDocumentSemanticTokensProvider({ language: 'basilisco' }, {
    provideDocumentSemanticTokens(doc) {
      if (!vscode.workspace.getConfiguration('basilisco').get('semanticHighlighting.enable', true)) return new vscode.SemanticTokensBuilder(legend).build();
      const b = new vscode.SemanticTokensBuilder(legend);
      const re = /#.*|"(?:\\.|[^"\\])*"|\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b|\b[\p{L}_][\p{L}\p{N}_]*\b|==|!=|<=|>=|[+\-*/%=<>]/gu;
      const { declarations } = scanSymbols(doc);
      let m;
      const text = doc.getText();
      while ((m = re.exec(text))) {
        const p = doc.positionAt(m.index);
        const word = m[0];
        let type; let mods = [];
        if (word.startsWith('#')) type = 'comment';
        else if (word.startsWith('"')) type = 'string';
        else if (/^\d/.test(word)) type = 'number';
        else if (KEYWORDS.has(word)) type = 'keyword';
        else if (declarations.some(x => x.name === word && x.range.start.isEqual(p))) { type = declarations.find(x => x.name === word).kind === vscode.SymbolKind.Function ? 'function' : 'variable'; mods = ['declaration']; }
        else if (BUILTINS.has(word)) { type = 'function'; mods = ['builtin']; }
        else if (/^[+\-*/%=<>!]+$/.test(word)) type = 'operator';
        else type = 'variable';
        const typeIdx = tokenTypes.indexOf(type);
        let modBits = 0;
        for (const mName of mods) { const mi = tokenModifiers.indexOf(mName); if (mi >= 0) modBits |= (1 << mi); }
        if (typeIdx >= 0) b.push(p.line, p.character, word.length, typeIdx, modBits);
      }
      return b.build();
    },
  }, legend));

  output.appendLine('Basilisco extension activated.');
  return { getBasiliscoCommand };
}

function deactivate() {
  if (client) { const p = client.stop(); client = undefined; return p; }
  return undefined;
}

module.exports = { activate, deactivate, getBasiliscoCommand };
