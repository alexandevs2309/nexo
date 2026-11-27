# Basilisco para VS Code

Extensión oficial de VS Code para el lenguaje **Basilisco** (`.bsk`, `.nexo`).

- **Identificador:** `alexandevs.basilisco`
- **Versión publicada verificada:** `0.3.2`
- **Versión local:** `0.4.0` (todavía sin publicar)

## Capacidades verificadas

- **Resaltado de sintaxis** (TextMate/gramática) para `.bsk` y `.nexo`.
- **LSP por stdio**: diagnósticos, completion, hover, definición, referencias y símbolos de documento/workspace provistos por `basilisco lsp`.
- **DAP**: depuración mediante `basilisco dap` (tipo `bas-dap`).
- **Formateo**: proveedor de formateo de documento vía `basilisco formatear`.
- **Comandos**: `BASILISCO: Ejecutar archivo`, `BASILISCO: Analizar archivo actual`, `BASILISCO: Formatear archivo`, y comandos de compatibilidad.
- **Snippets** básicos de Basilisco.
- **Folding** por marcadores/reglas de lenguaje.

## Capacidades locales en la extensión

Además del LSP/DAP, la extensión mantiene proveedores locales para capacidades no expuestas por el LSP actual: rename, signature help, folding, y resaltado semántico básico (según implementación local). Estas capacidades no deben confundirse con soporte LSP contractual completo.

## Configuración

- `basilisco.executable` (principal): ruta al ejecutable de Basilisco. **Predeterminado: `bsk`**.
- `basilisco.path` (alias de transición, **deprecado**): compatibilidad con configuraciones previas.

Resolución del ejecutable: valor explícito de `basilisco.executable` > valor explícito de `basilisco.path` > predeterminado.

## Requisitos

- Ejecutable `basilisco`/`bsk` disponible en PATH o configurado en `basilisco.executable`.
- VS Code `^1.85.0`.

## Desarrollo y pruebas

```bash
cd editors/vscode-basilisco
npm install
npm test
```

Las pruebas de Extension Host usan `@vscode/test-electron` y Mocha.

## Limitaciones comprobadas

- La versión local `0.4.0` **no está publicada**.
- La paridad con 0.3.2 publicada se basa en evidencia de inspección/ejecución disponible; capacidades no verificadas no se declaran como equivalentes.
- Rename/Signature Help/semantic highlighting: implementaciones locales en extensión; no equivalen a soporte LSP contractual completo si el servidor no las expone.
- DAP/VM Debug dependen de las capacidades actuales de `basilisco dap` y del runtime soportado.
