import type { FromStyles, PaletteLayer, StylesAction, StylesState, ToStyles } from '../src/stylesProtocol';
import { element } from './dom';
import { paletteEditor } from './paletteEditor';
import { styleDefinition } from './styleDefinitionEditor';

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
const light = element<HTMLButtonElement>('palette-light');
const dark = element<HTMLButtonElement>('palette-dark');
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
light.addEventListener('click', () => send({ type: 'preview-theme', dark: false }));
dark.addEventListener('click', () => send({ type: 'preview-theme', dark: true }));
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
    if (state.page) {
        settings.page = state.page;
        if (state.paletteFocus) {
            settings.layer = state.paletteFocus.layer;
            settings.compare = false;
            search.value = '';
        }
        vscode.setState(settings);
    }
    if (previous.content !== state.content) renderData();
    render();
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
    details.replaceChildren(styleDefinition(content.definition, send));
    scope.replaceChildren(...content.catalog.palette_scopes.map(entry => new Option(entry.label, String(entry.node))));
    if (content.palette) scope.value = String(content.palette.owner);
    layers.replaceChildren(...(content.palette?.layers ?? []).map(layer =>
        new Option(layer.layer[0].toUpperCase() + layer.layer.slice(1), layer.layer)));
    layers.value = settings.layer;
    renderPalette();
}

function render(): void {
    const unavailable = !state.content || state.busy || !!state.message;
    for (const page of ['styles', 'palette'] as const) {
        const selected = page === settings.page;
        tabs[page].setAttribute('aria-pressed', String(selected));
        sheets[page].hidden = tools[page].hidden = !selected;
    }
    fields.hidden = !state.content;
    fields.disabled = unavailable;
    fields.setAttribute('aria-busy', String(!!state.busy));
    status.textContent = state.message ?? (!state.content && state.busy ? 'Reading styles and themes…' : '');
    status.hidden = !status.textContent;
    createStyle.disabled = unavailable || !state.content?.catalog.style_hosts.length;
    createTheme.disabled = unavailable || !state.content?.catalog.palette_scopes.length;
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
    light.disabled = dark.disabled = unavailable || !state.preview;
    light.setAttribute('aria-pressed', String(state.preview && state.dark === false));
    dark.setAttribute('aria-pressed', String(state.preview && state.dark === true));
    compare.disabled = unavailable || !state.content?.palette;
    compare.setAttribute('aria-pressed', String(settings.compare));
    layers.hidden = settings.compare;
    setAdd();
}

function activeLayer() { return state.content?.palette?.layers.find(layer => layer.layer === settings.layer); }
function setAdd(): void {
    const layer = activeLayer();
    add.disabled = !layer || !!layer.unavailable || settings.compare || !!state.busy || !!state.message;
}
function renderPalette(): void {
    colors.replaceChildren(paletteEditor(state.content?.palette, activeLayer(), settings.compare, search.value.trim().toLowerCase(), send));
    setAdd();
}
