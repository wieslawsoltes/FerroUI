import { randomBytes } from 'node:crypto';
import * as vscode from 'vscode';
import type { Language } from './protocol';

export function designerHtml(webview: vscode.Webview, root: vscode.Uri, language: Language): string {
    const script = webview.asWebviewUri(vscode.Uri.joinPath(root, 'dist', 'designer.js'));
    const style = webview.asWebviewUri(vscode.Uri.joinPath(root, 'media', 'designer.css'));
    const nonce = randomBytes(16).toString('base64');
    const alternate = language === 'rust' ? 'XAML' : 'RSX';
    const authored = language === 'rust' ? 'Rust' : 'XAML';
    return `<!doctype html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';">
    <link rel="stylesheet" href="${style}">
    <title>FerroUI Designer</title>
</head>
<body>
    <header class="topbar">
        <nav aria-label="Designer view" class="tabs">
            <button id="preview-tab" aria-pressed="true" aria-controls="preview">Design</button>
            <button id="source-tab" aria-pressed="false" aria-controls="counterpart">${alternate}</button>
        </nav>
        <select id="blocks" aria-label="RSX invocation" hidden></select>
        <button class="secondary" id="refresh" aria-label="Refresh preview">Refresh</button>
    </header>
    <div id="status" class="status" role="status">Starting preview…</div>
    <section id="preview" aria-label="Design">
        <form class="canvas-toolbar" id="viewport">
            <label>Width <input aria-label="Preview width" id="width" type="number" min="64" max="1536" step="1" value="800" required></label>
            <label>Height <input aria-label="Preview height" id="height" type="number" min="64" max="1536" step="1" value="520" required></label>
            <label>Theme <select id="theme"><option value="light">Light</option><option value="dark">Dark</option></select></label>
            <button type="submit" class="secondary">Update</button>
        </form>
        <div class="canvas" id="canvas">
            <div id="placeholder">Waiting for FerroUI to render the view</div>
            <div id="artboard" class="artboard" hidden>
                <img id="frame" alt="Rendered FerroUI view" draggable="false">
                <div id="selection-box" class="selection-box" hidden></div>
            </div>
        </div>
        <div class="details">
            <section class="outline-pane" aria-labelledby="outline-heading">
                <h2 id="outline-heading">Outline</h2>
                <div id="outline"></div>
            </section>
            <section class="properties-pane" aria-labelledby="properties-heading">
                <h2 id="properties-heading">Properties</h2>
                <div id="properties"></div>
            </section>
        </div>
    </section>
    <section id="counterpart" aria-label="${alternate} projection" hidden>
        <div class="counterpart-toolbar">
            <p id="counterpart-message"></p>
            <button id="reload-draft" class="secondary" hidden>Reload projection</button>
            <button id="apply-draft" disabled>Apply to ${authored}</button>
        </div>
        <textarea id="projection" aria-label="${alternate} source" spellcheck="false" wrap="off" readonly></textarea>
    </section>
    <script nonce="${nonce}" src="${script}"></script>
</body>
</html>`;
}
