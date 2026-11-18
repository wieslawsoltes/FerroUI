import { randomBytes } from 'node:crypto';
import * as vscode from 'vscode';
import { DEFAULT_VIEWPORT, MAX_VIEWPORT_SIZE, MIN_VIEWPORT_SIZE, type Language } from './protocol';

export function webviewOptions(root: vscode.Uri): vscode.WebviewOptions {
    return {
        enableScripts: true,
        localResourceRoots: ['dist', 'media'].map(folder => vscode.Uri.joinPath(root, folder)),
    };
}

export function designerHtml(webview: vscode.Webview, root: vscode.Uri, language: Language): string {
    const alternate = language === 'rust' ? 'XAML' : 'RSX';
    const authored = language === 'rust' ? 'Rust' : 'XAML';
    return page(webview, root, 'designer', 'FerroUI Designer', `
<header class="toolbar" aria-label="Designer toolbar">
    <nav class="segments" aria-label="Designer view">
        <button id="preview-tab" aria-pressed="true" aria-controls="preview">Design</button>
        <button id="source-tab" aria-pressed="false" aria-controls="counterpart">${alternate}</button>
    </nav>
    <select id="blocks" aria-label="RSX invocation" hidden></select>
    <div id="canvas-tools" class="canvas-tools">
        <button id="viewport-button" class="metric" popovertarget="viewport-menu" aria-label="Preview size and theme" title="Preview size and theme">
            ${icon('layout')}<span id="dimensions"></span>${icon('chevron-down')}
        </button>
        <div class="zoom-controls" role="group" aria-label="Canvas zoom">
            ${tool('zoom-out', 'remove', 'Zoom out (−)')}
            <button id="zoom-button" class="metric" popovertarget="zoom-menu" aria-label="Zoom options" title="Zoom options">
                <span id="zoom-value">Fit</span>${icon('chevron-down')}
            </button>
            ${tool('zoom-in', 'add', 'Zoom in (+)')}
        </div>
        ${tool('pan', 'move', 'Pan canvas (Space and drag)', 'aria-pressed="false"')}
    </div>
    <div class="dock-tools" role="group" aria-label="Workbench views">
        ${tool('toolbox-button', 'extensions', 'Show Toolbox')}
        ${tool('outline-button', 'list-tree', 'Show Outline')}
        ${tool('properties-button', 'settings-gear', 'Show Properties')}
        ${tool('refresh', 'refresh', 'Refresh preview')}
        ${tool('actions-button', 'ellipsis', 'More designer actions', 'popovertarget="actions-menu"')}
    </div>
</header>
<div id="status" class="status" role="status" hidden></div>
<main id="preview" aria-label="Design">
    <div id="canvas" class="canvas" tabindex="0" role="region" aria-label="Preview canvas" aria-keyshortcuts="0 1 + - Space Enter">
        <div id="stage" class="stage">
            <div id="placeholder" class="placeholder">${icon('preview')}<span id="empty-state">Rendering preview…</span></div>
            <div id="artboard" class="artboard" hidden>
                <img id="frame" alt="Rendered FerroUI view" draggable="true">
                <div id="hover-box" class="control-outline hover-outline" hidden></div>
                <div id="selection-box" class="control-outline" hidden></div>
                <div id="drop-box" class="control-outline drop-outline" hidden></div>
            </div>
        </div>
    </div>
    <button id="selection-label" class="selection-label" title="Reveal selection in source" hidden>${icon('go-to-file')}<span id="selected-control"></span></button>
    <output id="drop-hint" class="drop-hint" aria-live="polite" hidden></output>
</main>
<main id="counterpart" aria-label="${alternate} projection" hidden>
    <div class="counterpart-toolbar">
        <p id="counterpart-message"></p>
        <button id="reload-draft" class="secondary" hidden>Reload projection</button>
        <button id="apply-draft" class="primary" disabled>Apply to ${authored}</button>
    </div>
    <textarea id="projection" aria-label="${alternate} source" spellcheck="false" wrap="off" readonly></textarea>
</main>
<div id="viewport-menu" class="popover viewport-menu" popover>
    <form id="viewport">
        <label class="field" for="preset">Viewport</label>
        <select id="preset"></select>
        <div class="dimension-fields">
            <label>Width ${dimension('width', DEFAULT_VIEWPORT.width)}</label>
            ${tool('rotate', 'arrow-swap', 'Swap width and height')}
            <label>Height ${dimension('height', DEFAULT_VIEWPORT.height)}</label>
        </div>
        <label class="theme-field">Appearance <select id="theme"><option value="light">Light</option><option value="dark">Dark</option></select></label>
        <button type="submit" class="primary">Apply</button>
    </form>
</div>
<div id="zoom-menu" class="popover zoom-menu" popover>
    <button id="fit" type="button">Fit to view <kbd>0</kbd></button>
    <button id="fit-selection" type="button">Fit selection</button>
    <button id="actual" type="button">Actual size <kbd>1</kbd></button>
    <div class="zoom-presets" role="group" aria-label="Zoom presets">
        <button type="button" data-scale="0.5">50%</button>
        <button type="button" data-scale="2">200%</button>
        <button type="button" data-scale="4">400%</button>
    </div>
</div>
<div id="actions-menu" class="popover zoom-menu" popover>
    <button type="button" id="menu-toolbox">Show Toolbox</button>
    <button type="button" id="menu-outline">Show Outline</button>
    <button type="button" id="menu-properties">Show Properties</button>
    <button type="button" id="menu-refresh">Refresh preview</button>
</div>`);
}

export function outlineHtml(webview: vscode.Webview, root: vscode.Uri): string {
    return page(webview, root, 'outline', 'FerroUI Outline', `
<main class="outline">
    <p class="dock-message" id="outline-message" role="status"></p>
    <ul id="outline-tree" role="tree" aria-label="Control hierarchy"></ul>
</main>`);
}

export function toolboxHtml(webview: vscode.Webview, root: vscode.Uri): string {
    return page(webview, root, 'toolbox', 'FerroUI Toolbox', `
<main class="toolbox">
    <div class="toolbox-search">
        ${icon('search')}<input type="search" id="asset-search" aria-label="Filter controls" placeholder="Filter controls">
        ${tool('asset-view', 'list-flat', 'Toggle list or tile view', 'aria-pressed="false"')}
    </div>
    <p class="dock-message" id="toolbox-message" role="status">Open a FerroUI designer to insert controls.</p>
    <div id="asset-list" aria-label="Toolbox controls"></div>
</main>`);
}

export function propertiesHtml(webview: vscode.Webview, root: vscode.Uri): string {
    return page(webview, root, 'properties', 'FerroUI Properties', `
<main class="inspector">
    <p class="view-message" id="view-message" role="status">Select a control in a FerroUI designer.</p>
    <section id="element-properties" aria-label="Selected control properties" hidden>
        <header class="element-heading">
            ${icon('symbol-field')}
            <div><strong id="control-type"></strong><code id="control-name"></code></div>
        </header>
        <div id="filter-row" class="filter-row" hidden>
            ${icon('search')}<input type="search" id="filter" aria-label="Filter properties" placeholder="Filter properties">
        </div>
        <div id="properties"></div>
    </section>
</main>`);
}

function page(webview: vscode.Webview, root: vscode.Uri, entry: string, title: string, body: string): string {
    const resource = (...parts: string[]) => webview.asWebviewUri(vscode.Uri.joinPath(root, ...parts));
    const nonce = randomBytes(16).toString('base64');
    return `<!doctype html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; font-src ${webview.cspSource}; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';">
    <link rel="stylesheet" href="${resource('dist', 'icons', 'codicon.css')}">
    <link rel="stylesheet" href="${resource('media', 'workbench.css')}">
    <link rel="stylesheet" href="${resource('media', `${entry}.css`)}">
    <title>${title}</title>
</head>
<body>${body}
    <script nonce="${nonce}" src="${resource('dist', `${entry}.js`)}"></script>
</body>
</html>`;
}

function dimension(id: string, value: number): string {
    return `<input id="${id}" type="number" min="${MIN_VIEWPORT_SIZE}" max="${MAX_VIEWPORT_SIZE}" step="1" value="${value}" required>`;
}

function icon(name: string): string {
    return `<span class="codicon codicon-${name}" aria-hidden="true"></span>`;
}

function tool(id: string, glyph: string, label: string, attributes = ''): string {
    return `<button id="${id}" type="button" class="tool" aria-label="${label}" title="${label}" ${attributes}>${icon(glyph)}</button>`;
}
