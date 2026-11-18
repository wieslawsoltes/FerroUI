import { randomBytes } from 'node:crypto';
import * as vscode from 'vscode';
import { DEFAULT_VIEWPORT, MAX_VIEWPORT_SIZE, MIN_VIEWPORT_SIZE, type DetailsView, type Language } from './protocol';

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
        <div class="canvas-mode segments" role="group" aria-label="Canvas mode">
            <button type="button" id="canvas-select" aria-pressed="true">Select</button>
            <button type="button" id="canvas-interact" aria-pressed="false" disabled>Interact</button>
        </div>
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
<div class="designer-content">
<div id="status" class="status" role="status" hidden></div>
<main id="preview" aria-label="Design" aria-busy="true">
    <div id="canvas" class="canvas preview-surface" tabindex="0" role="region" aria-label="Preview canvas" aria-keyshortcuts="0 1 + - Space Enter ArrowUp ArrowDown">
        <div id="stage" class="stage">
            <div id="placeholder" class="placeholder">${icon('preview')}<span id="empty-state">Rendering preview…</span></div>
            <div id="artboard" class="artboard" hidden>
                <img id="frame" alt="Rendered FerroUI view" draggable="false">
                <div id="hover-box" class="control-outline hover-outline" hidden></div>
                <div id="selection-box" class="control-outline" hidden><div id="resize-handles" role="group" aria-label="Resize handles" hidden></div></div>
                <div id="selection-group" aria-hidden="true"></div>
                <div id="drop-box" class="control-outline drop-outline" hidden></div>
                <div id="focus-order" class="focus-overlay" role="group" aria-label="Native keyboard navigation" hidden></div>
            </div>
        </div>
    </div>
    <button id="selection-label" class="selection-label" title="Reveal selection in source" hidden>${icon('go-to-file')}<span id="selected-control" aria-live="polite"></span></button>
    <output id="drop-hint" class="drop-hint" aria-live="polite" hidden></output>
    <form id="canvas-preview-input" aria-label="Send input to the focused preview control" hidden>
        <select id="canvas-input-kind" aria-label="Input kind">
            <option value="text">Text</option><option value="key">Key gesture</option>
        </select>
        <input id="canvas-input-value" aria-label="Preview text or key gesture" required maxlength="8192" placeholder="Click a control, then enter text">
        <button type="submit" class="primary" id="canvas-send-input">Send</button>
    </form>
</main>
<main id="counterpart" aria-label="${alternate} projection" hidden>
    <div class="counterpart-toolbar">
        <p id="counterpart-message"></p>
        <button id="reload-draft" class="secondary" hidden>Reload projection</button>
        <button id="apply-draft" class="primary" disabled>Apply to ${authored}</button>
    </div>
    <textarea id="projection" aria-label="${alternate} source" spellcheck="false" wrap="off" readonly></textarea>
</main>
</div>
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
    <button type="button" class="compact-action" id="menu-toolbox">Show Toolbox</button>
    <button type="button" class="compact-action" id="menu-outline">Show Outline</button>
    <button type="button" class="compact-action" id="menu-properties">Show Properties</button>
    <button type="button" id="menu-events">Show Events</button>
    <button type="button" id="menu-resources">Show Resources</button>
    <button type="button" id="menu-assets">Show Assets</button>
    <button type="button" id="menu-studio">Show Studio Tools</button>
    <button type="button" id="menu-timeline">Show Animation Timeline</button>
    <button type="button" id="menu-data">Show Design Data</button>
    <button type="button" id="menu-library">Show Library</button>
    <button type="button" id="menu-focus-order" aria-pressed="false" disabled>Focus Order Overlay</button>
    <button type="button" class="compact-action" id="menu-refresh">Refresh preview</button>
</div>`);
}

export function outlineHtml(webview: vscode.Webview, root: vscode.Uri): string {
    return page(webview, root, 'outline', 'FerroUI Outline', `
<main class="outline">
    <div class="toolbar outline-toolbar">
        <select id="outline-mode" aria-label="Hierarchy">
            <option value="source">Source</option><option value="combined">Combined</option>
            <option value="visual">Visual</option><option value="logical">Logical</option>
        </select>
        ${tool('outline-layout', 'layout', 'Arrange selection…', 'disabled')}
    </div>
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
        <dl id="property-metrics" class="property-metrics" aria-label="Measured artboard bounds" title="Measured bounds in artboard units" hidden>
            <div><dt>X</dt><dd data-metric="x"></dd></div><div><dt>Y</dt><dd data-metric="y"></dd></div>
            <div><dt>W</dt><dd data-metric="width"></dd></div><div><dt>H</dt><dd data-metric="height"></dd></div>
        </dl>
        <div id="filter-row" class="filter-row" hidden>
            ${icon('search')}<input type="search" id="filter" aria-label="Filter properties" placeholder="Filter properties">
        </div>
        <div id="properties"></div>
    </section>
</main>`);
}

export function detailsHtml(webview: vscode.Webview, root: vscode.Uri, view: DetailsView): string {
    const events = view === 'events';
    const title = events ? 'Events' : 'Resources';
    return page(webview, root, 'details', `FerroUI ${title}`, `
<main class="details" id="${view}-panel" data-view="${view}">
    <header class="details-header">
        <strong id="details-title" hidden></strong>
        <div class="details-filter">${icon('search')}<input type="search" id="details-search" aria-label="Filter ${view}" placeholder="Filter ${view}"></div>
        <label class="details-option"><input type="checkbox" id="details-filtered">${events ? 'Assigned handlers only' : 'Visible to selection'}</label>
    </header>
    <p class="dock-message" id="details-message" role="status"></p>
    <div id="details-list" aria-label="${title}"></div>
</main>`);
}

export function assetsHtml(webview: vscode.Webview, root: vscode.Uri): string {
    return page(webview, root, 'assets', 'FerroUI Assets', `
<main class="project-assets">
    <div class="asset-filter">
        ${icon('search')}<input type="search" id="project-asset-search" aria-label="Filter project assets" placeholder="Filter assets">
        <select id="project-asset-kind" aria-label="Asset type">
            <option value="">All types</option><option>Image</option><option>Font</option><option>Media</option><option>XAML</option>
        </select>
    </div>
    <p class="dock-message" id="project-asset-message" role="status"></p>
    <section id="project-asset-preview" class="asset-preview" aria-label="Selected project asset" hidden>
        <img id="asset-thumbnail" alt="" hidden>
        <strong id="asset-title"></strong><code id="asset-path"></code>
        <div class="asset-actions tool-group" role="group" aria-label="Asset actions">
            ${tool('asset-open', 'go-to-file', 'Open asset')}
            ${tool('asset-copy', 'copy', 'Copy asset path')}
            ${tool('asset-place', 'insert', 'Choose image placement…', 'hidden')}
            <button id="asset-insert" class="primary" type="button" hidden>Insert image</button>
        </div>
    </section>
    <ul id="project-asset-list" aria-label="Project files"></ul>
    <button id="assets-more" type="button" hidden>Show more files</button>
</main>`);
}

export function runtimeHtml(webview: vscode.Webview, root: vscode.Uri): string {
    return page(webview, root, 'runtime', 'FerroUI Live App', `
<header class="toolbar" aria-label="Live App controls">
    <div class="runtime-targets">
        <select id="runtime-target" aria-label="Runtime connection"></select>
        <select id="runtime-window" aria-label="Application window"></select>
    </div>
    <div class="tool-group toolbar-divider" role="group" aria-label="Application actions">
        ${tool('runtime-start', 'debug-start', 'Run with Hot Reload')}
        ${tool('runtime-attach', 'plug', 'Connect to application')}
        ${tool('runtime-refresh', 'refresh', 'Reconnect and refresh')}
    </div>
</header>
<p id="runtime-status" class="dock-message" role="status">Connecting…</p>
<p id="runtime-problem" class="dock-message error" role="alert" hidden></p>
<main class="runtime-main" hidden>
    <section class="runtime-stage" aria-label="Running application">
        <div class="toolbar runtime-controls">
            <div class="segments" role="group" aria-label="Live preview mode">
                <button id="runtime-inspect" aria-pressed="true">${icon('inspect')}Inspect</button>
                <button id="runtime-interact" aria-pressed="false">${icon('run')}Interact</button>
            </div>
        </div>
        <div class="runtime-canvas preview-surface"><img id="runtime-image" alt="Running FerroUI window" draggable="false"></div>
        <form id="runtime-input" aria-label="Send input to the selected control" hidden>
            <select id="runtime-input-kind" aria-label="Input kind"><option value="text">Text</option><option value="key">Key gesture</option></select>
            <input id="runtime-input-value" aria-label="Text or key gesture" required maxlength="8192">
            <button type="submit" class="tool" aria-label="Send input" title="Send input">${icon('send')}</button>
        </form>
    </section>
    <aside aria-label="Runtime inspection">
        <section class="runtime-inspect-section" aria-label="Application hierarchy">
            <div class="runtime-inspect-toolbar">
                <select id="runtime-hierarchy" aria-label="Hierarchy type"><option value="visual">Visual tree</option><option value="logical">Logical tree</option><option value="automation">Accessibility tree</option></select>
                <input id="runtime-filter" type="search" placeholder="Filter elements" aria-label="Filter elements">
            </div>
            <ul id="runtime-tree" aria-label="Application elements"></ul>
        </section>
        <section class="runtime-inspect-section" aria-label="Selected element">
            <div class="runtime-inspect-toolbar">
                <select id="runtime-detail" aria-label="Inspection category"><option value="properties">Properties</option><option value="styles">Styles</option><option value="resources">Resources</option><option value="diagnostics">Diagnostics</option></select>
                ${tool('runtime-reveal', 'go-to-file', 'Reveal Source', 'hidden')}
            </div>
            <input id="runtime-property-filter" type="search" placeholder="Filter properties" aria-label="Filter runtime properties" hidden>
            <div id="runtime-details" aria-label="Runtime details"></div>
        </section>
    </aside>
</main>`);
}

export function hostedHtml(webview: vscode.Webview, root: vscode.Uri): string {
    return page(webview, root, 'hosted', 'FerroUI App Preview', `
<header class="toolbar hosted-toolbar" aria-label="Application preview controls">
    <label id="hosted-fixture-label" class="metric" hidden>${icon('database')}<select id="hosted-fixture" aria-label="Preview fixture"></select></label>
    ${tool('hosted-refresh', 'refresh', 'Refresh Source')}
</header>
<p id="hosted-status" class="dock-message" role="status">Starting application preview…</p>
<main class="hosted-canvas preview-surface" aria-label="Application artboard"><img id="hosted-image" alt="App-hosted FerroUI design" draggable="false" hidden></main>
<form id="hosted-input" class="toolbar" aria-label="Preview input" hidden>
    <input id="hosted-text" aria-label="Text for the focused preview control" required maxlength="8192">
    <button type="submit" class="tool" title="Send Text" aria-label="Send Text">${icon('send')}</button>
</form>`);
}

export function stagesHtml(webview: vscode.Webview, root: vscode.Uri): string {
    return page(webview, root, 'stages', 'FerroUI Stages', `
<header class="toolbar stage-toolbar" aria-label="Stages controls">
    <nav id="stages-tabs" class="segments" aria-label="Stage workbench">
        <button id="stages-show" aria-controls="stage-cards" aria-pressed="true">${icon('layout')}Stages</button>
        <button id="stages-edit-prototype" aria-controls="prototype-graph" aria-pressed="false">${icon('git-branch')}Prototype</button>
    </nav>
    <div id="stages-commands" class="tool-group" role="group" aria-label="Stage set commands">
        ${tool('stages-add', 'add', 'Add Stage')}
        ${tool('stages-load', 'folder-opened', 'Open Stage Set')}
        ${tool('stages-save', 'save', 'Save Stage Set', 'disabled')}
        <span class="toolbar-divider" aria-hidden="true"></span>
        ${tool('stages-prototype', 'play', 'Play Prototype', 'disabled')}
    </div>
    <nav id="prototype-actions" class="prototype-controls" aria-label="Prototype playback" hidden></nav>
</header>
<main class="stage-workspace preview-surface">
    <p id="stages-status" class="dock-message stage-board-message" role="status">Rendering stages…</p>
    <section id="stage-cards" aria-label="Native preview stages"></section>
    <section id="prototype-graph" aria-label="Prototype graph" hidden>
        <header class="prototype-heading">
            <ul id="prototype-flows" class="prototype-flows" aria-label="Prototype flows"></ul>
            <div class="tool-group" role="group" aria-label="Prototype authoring">
                ${tool('prototype-add-flow', 'add', 'Add flow')}
                ${tool('prototype-add-link', 'link', 'Add interaction')}
            </div>
        </header>
        <div id="prototype-stages" class="prototype-stages" aria-label="Stage connections"></div>
    </section>
</main>
<div id="stage-menu" class="popover command-menu" popover aria-label="Stage actions">
    <button type="button" data-stage-action="environment" autofocus>${icon('paintcan')}Environment…</button>
    <button type="button" data-stage-action="layout">${icon('layout')}Sizing…</button>
    <button type="button" data-stage-action="designData">${icon('database')}Design data…</button>
    <button type="button" data-stage-action="state">${icon('symbol-enum')}Visual states…</button>
    <hr>
    <button type="button" data-stage-action="refresh">${icon('refresh')}Refresh stage</button>
    <button type="button" data-stage-action="export">${icon('file-media')}Save PNG…</button>
    <button type="button" data-stage-action="name">${icon('edit')}Rename…</button>
    <hr>
    <button type="button" data-stage-action="remove" class="danger">${icon('trash')}Remove stage</button>
</div>`);
}

export function timelineHtml(webview: vscode.Webview, root: vscode.Uri): string {
    return page(webview, root, 'timeline', 'FerroUI Timeline', `
<main class="timeline" aria-label="Animation timeline">
    <p id="timeline-status" role="status" hidden></p>
    <section id="timeline-program" aria-label="Selected animation" hidden>
        <div class="timeline-toolbar">
            ${tool('timeline-play', 'play', 'Play animation')}
            <select id="timeline-programs" aria-label="Animation program"></select>
            <output id="timeline-time" for="timeline-seek" aria-live="off"></output>
        </div>
        <div class="timeline-grid">
            <input id="timeline-seek" type="range" min="0" value="0" step="any" aria-label="Animation playhead">
            <div id="timeline-ruler" class="timeline-ruler" aria-hidden="true"></div>
            <ol id="timeline-tracks" aria-label="Property tracks"></ol>
        </div>
    </section>
</main>`);
}

export function designDataHtml(webview: vscode.Webview, root: vscode.Uri): string {
    return page(webview, root, 'designData', 'FerroUI Design Data', `
<main class="data-view" aria-label="Design data">
    <p id="data-status" role="status">Reading design data…</p>
    <p id="data-error" role="alert" hidden></p>
    <section id="data-editor" class="data-editor" hidden>
        <div class="data-preview-field">
            ${icon('database')}<select id="data-preview" aria-label="Preview data"></select>
        </div>
        <div class="data-toolbar">
            <label for="data-source">Fixtures JSON</label>
            <div class="tool-group" role="group" aria-label="Draft actions">
                ${tool('data-examples', 'sparkle', 'Create example fixtures')}
                ${tool('data-discard', 'discard', 'Discard draft')}
                <button type="button" id="data-apply" class="primary" aria-label="Apply fixtures">Apply</button>
            </div>
        </div>
        <textarea id="data-source" spellcheck="false" autocapitalize="off" autocorrect="off"
            aria-describedby="data-fixture-hint"></textarea>
        <p id="data-fixture-hint">Bind to fields under <code>data</code>. Clear the JSON to remove the fixtures.</p>
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
