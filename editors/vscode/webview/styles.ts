import type { CanvasTheme } from '../src/protocol';
import { paletteLayerLabels, type FromStyles, type PaletteLayer, type StylesAction, type StylesState, type ToStyles } from '../src/stylesProtocol';
import { element } from './dom';
import { paletteEditor } from './paletteEditor';
import { styleDefinition } from './styleDefinitionEditor';
import { actionButton } from './propertyEditor';
import type { RustPaletteDisplay } from './rustStylePalette';

declare function acquireVsCodeApi(): {
    getState(): { page: 'styles' | 'palette'; layer: PaletteLayer; compare: boolean } | undefined;
    setState(state: { page: 'styles' | 'palette'; layer: PaletteLayer; compare: boolean }): void;
    postMessage(message: FromStyles): void;
};
const vscode = acquireVsCodeApi();
const tabs = { styles: element<HTMLButtonElement>('styles-tab'), palette: element<HTMLButtonElement>('palette-tab') };
const sheets = { styles: element('styles-sheet'), palette: element('palette-sheet') };
const tools = { styles: element('styles-toolbar'), palette: element('palette-toolbar') };
const fields = element<HTMLFieldSetElement>('styles-fields');
const definitions = element<HTMLSelectElement>('styles-definitions');
const details = element('style-details');
const scope = element<HTMLSelectElement>('palette-scope');
const layers = element<HTMLSelectElement>('palette-layer');
const search = element<HTMLInputElement>('palette-search');
const colors = element('palette-details');
const themeButtons: [CanvasTheme, HTMLButtonElement][] = [
    ['light', element<HTMLButtonElement>('palette-light')],
    ['dark', element<HTMLButtonElement>('palette-dark')],
    ['high_contrast', element<HTMLButtonElement>('palette-high-contrast')],
];
const compare = element<HTMLButtonElement>('palette-compare');
const add = element<HTMLButtonElement>('palette-add');
const addStyle = element<HTMLButtonElement>('style-new');
const back = element<HTMLButtonElement>('style-back');
const includedStyles = element<HTMLButtonElement>('style-includes');
const includedPalettes = element<HTMLButtonElement>('palette-includes');
const createTheme = element<HTMLButtonElement>('theme-create');
const createStyle = element<HTMLButtonElement>('style-create');
const createMenu = element('style-new-menu');
const status = element('styles-status');
const error = element('styles-error');
let state: StylesState = { stamp: 0 };
let settings = vscode.getState() ?? { page: 'styles', layer: 'light', compare: false };
let focusAfterEdit: string | undefined;
const rustPalette: RustPaletteDisplay = { compare: false };

function send(action: StylesAction): void {
    if (state.busy || !state.content || state.message) return;
    error.hidden = true;
    vscode.postMessage({ type: 'action', stamp: state.stamp, action });
}

for (const page of ['styles', 'palette'] as const) {
    tabs[page].addEventListener('click', () => {
        settings.page = page;
        vscode.setState(settings);
        render();
    });
}
definitions.addEventListener('change', () => send({ type: 'definition', node: Number(definitions.value) }));
scope.addEventListener('change', () => send({ type: 'scope', node: Number(scope.value) }));
layers.addEventListener('change', () => {
    settings.layer = layers.value as PaletteLayer;
    vscode.setState(settings);
    renderPalette();
});
search.addEventListener('input', renderPalette);
for (const [theme, button] of themeButtons) {
    button.addEventListener('click', () => send({ type: 'preview-theme', theme }));
}
compare.addEventListener('click', () => {
    settings.compare = !settings.compare;
    vscode.setState(settings);
    renderPalette();
    render();
});
add.addEventListener('click', () => send({ type: 'palette-add', layer: settings.layer }));
back.addEventListener('click', () => send({ type: 'return-to-consumer' }));
includedStyles.addEventListener('click', () => send({ type: 'included-sources', page: 'styles' }));
includedPalettes.addEventListener('click', () => send({ type: 'included-sources', page: 'palette' }));
createStyle.addEventListener('click', () => { createMenu.hidePopover(); send({ type: 'new', kind: 'style' }); });
createTheme.addEventListener('click', () => { createMenu.hidePopover(); send({ type: 'new', kind: 'theme' }); });

window.addEventListener('message', event => {
    const incoming = event.data as ToStyles | { type: 'state'; state: StylesState };
    if (incoming.type === 'error') {
        if (incoming.stamp === state.stamp) {
            error.textContent = incoming.message;
            error.hidden = false;
        }
        return;
    }
    const previous = state;
    state = incoming.state;
    if (!previous.busy && state.busy && fields.contains(document.activeElement)) focusAfterEdit = document.activeElement?.id;
    if (state.stamp !== previous.stamp) error.hidden = true;
    const rust = state.content?.catalog.source_kind === 'rust';
    if (state.page) {
        if (rust) {
            if (state.compare !== undefined) rustPalette.compare = state.compare;
        } else {
            settings.page = state.page;
            if (state.compare !== undefined) settings.compare = state.compare;
            if (state.paletteFocus) {
                settings.layer = state.paletteFocus.layer;
                settings.compare = false;
                search.value = '';
            }
            vscode.setState(settings);
        }
    }
    if (previous.content !== state.content || rust && state.compare !== undefined) renderData();
    else if (state.compare !== undefined) renderPalette();
    render();
    if (rust && state.page === 'palette') document.getElementById('rust-palette-layer')?.scrollIntoView({ block: 'nearest' });
    if (state.paletteFocus?.color != null) {
        const value = document.getElementById(`palette-value-${state.paletteFocus.color}`);
        value?.scrollIntoView({ block: 'nearest' });
        value?.focus({ preventScroll: true });
    }
    if (focusAfterEdit && !state.busy) {
        if (document.activeElement === document.body || fields.contains(document.activeElement)) {
            document.getElementById(focusAfterEdit)?.focus({ preventScroll: true });
        }
        focusAfterEdit = undefined;
    }
});
vscode.postMessage({ type: 'ready' });

function renderData(): void {
    const content = state.content;
    if (!content) return;
    const entries = content.catalog.definitions;
    definitions.hidden = entries.length === 0;
    definitions.replaceChildren(...entries.map(entry => new Option(
        [entry.name, entry.scope].filter(Boolean).join(' · '), String(entry.node))));
    definitions.value = content.definition ? String(content.definition.node) : '';
    const rust = content.catalog.source_kind === 'rust';
    details.replaceChildren(styleDefinition(content.definition, send, !rust, rustPalette));
    if (content.catalog.issues?.length) {
        details.append(actionButton('warning', 'Reveal Rust styles and values without a preview mapping',
            () => send({ type: 'style-issues' })));
    }
    scope.replaceChildren(...content.catalog.palette_scopes.map(entry => new Option(entry.label, String(entry.node))));
    if (content.palette) scope.value = String(content.palette.owner);
    const available = content.palette?.layers ?? [];
    if (available.length && !available.some(layer => layer.layer === settings.layer)) {
        settings.layer = available.find(layer => layer.layer === state.theme)?.layer ?? available[0].layer;
        vscode.setState(settings);
    }
    layers.replaceChildren(...available.map(layer => new Option(paletteLayerLabels[layer.layer], layer.layer)));
    layers.value = settings.layer;
    renderPalette();
}

function render(): void {
    const unavailable = !state.content || state.busy || !!state.message;
    const rust = state.content?.catalog.source_kind === 'rust';
    for (const page of ['styles', 'palette'] as const) {
        const selected = page === (rust ? 'styles' : settings.page);
        tabs[page].setAttribute('aria-pressed', String(selected));
        tabs[page].hidden = rust;
        sheets[page].hidden = tools[page].hidden = !selected;
    }
    fields.hidden = !state.content;
    fields.disabled = unavailable;
    fields.setAttribute('aria-busy', String(!!state.busy));
    status.textContent = state.message ?? (!state.content && state.busy ? 'Reading styles and themes…' : '');
    status.hidden = !status.textContent;
    createStyle.disabled = unavailable || !state.content?.catalog.style_hosts.length;
    createTheme.disabled = unavailable || !state.content?.catalog.palette_scopes.length;
    createTheme.hidden = rust;
    addStyle.disabled = createStyle.disabled && createTheme.disabled;
    const sources = state.content?.catalog.included_sources ?? [];
    includedStyles.hidden = !sources.some(choice => choice.source.kind !== 'palette');
    includedPalettes.hidden = !sources.some(choice => choice.source.kind === 'palette');
    includedStyles.disabled = includedPalettes.disabled = unavailable;
    back.hidden = !state.consumer;
    back.disabled = unavailable;
    if (state.consumer) {
        back.title = `Return to ${state.consumer}`;
        back.setAttribute('aria-label', back.title);
    }
    for (const [theme, button] of themeButtons) {
        button.hidden = theme === 'high_contrast' && !state.content?.palette?.contrastPreview;
        button.disabled = unavailable || !state.preview;
        button.setAttribute('aria-pressed', String(state.theme === theme));
    }
    compare.disabled = unavailable || !state.content?.palette;
    compare.setAttribute('aria-pressed', String(settings.compare));
    layers.hidden = settings.compare;
    setAdd();
}

function activeLayer() { return state.content?.palette?.layers.find(layer => layer.layer === settings.layer); }
function setAdd(): void {
    const layer = activeLayer();
    const providerMissing = layer?.layer === 'high_contrast' && layer.declaration == null
        && !state.content?.palette?.contrastProviders?.length;
    add.disabled = !layer || !!layer.unavailable || providerMissing || settings.compare || !!state.busy || !!state.message;
}
function renderPalette(): void {
    colors.replaceChildren(paletteEditor(state.content?.palette, activeLayer(), settings.compare, search.value.trim().toLowerCase(), send));
    setAdd();
}
