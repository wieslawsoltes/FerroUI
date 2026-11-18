import type { AssetsState, DragGesture, FromAssets, ProjectAsset, ToAssets } from '../src/protocol';
import { create, element } from './dom';
import { DragSource } from './drag';

declare function acquireVsCodeApi(): { postMessage(message: FromAssets): void };
const vscode = acquireVsCodeApi();
const list = element<HTMLUListElement>('project-asset-list');
const search = element<HTMLInputElement>('project-asset-search');
const kind = element<HTMLSelectElement>('project-asset-kind');
const status = element('project-asset-message');
const details = element('project-asset-preview');
const thumbnail = element<HTMLImageElement>('asset-thumbnail');
const insert = element<HTMLButtonElement>('asset-insert');
const place = element<HTMLButtonElement>('asset-place');
const more = element<HTMLButtonElement>('assets-more');
const pageSize = 120;
let limit = pageSize;
let state: AssetsState = { assets: [], canInsert: false };
let key = '';
let selected: ProjectAsset | undefined;
let byId = new Map<string, ProjectAsset>();
let drag: DragGesture | undefined;

const dragSource = new DragSource(list, event => {
    const asset = rowAsset(event);
    if (asset?.source && state.frame) return {
        frame: state.frame, origin: { type: 'image', source: asset.source, asset: asset.id },
    };
}, vscode.postMessage, gesture => { drag = gesture; });

window.addEventListener('message', ({ data }: MessageEvent<ToAssets>) => {
    switch (data.type) {
        case 'drag':
            drag = data.gesture;
            if (!drag) dragSource.end(false);
            return;
        case 'thumbnail':
            if (data.id === selected?.id && data.uri?.startsWith('data:image/')) {
                thumbnail.src = data.uri;
                thumbnail.hidden = false;
            }
            return;
        case 'assetStatus': return showStatus(data.message, data.error);
        case 'state': {
            const previousFolder = state.folder;
            state = data.state;
            if (state.folder !== previousFolder) limit = pageSize;
            const next = JSON.stringify([state.folder, state.assets, Boolean(state.frame)]);
            if (next !== key) {
                key = next;
                byId = new Map(state.assets.map(asset => [asset.id, asset]));
                const keep = selected && state.folder === previousFolder ? byId.get(selected.id) : undefined;
                if (!keep) choose(undefined);
                else if (keep !== selected) choose(keep);
                render();
            }
            showPreviewButtons();
            showStatus(state.message ?? (state.assets.length ? undefined : 'No project assets found.'));
        }
    }
});

list.addEventListener('click', event => {
    const asset = rowAsset(event);
    if (asset) choose(asset);
});
list.addEventListener('dblclick', event => {
    const asset = rowAsset(event);
    if (asset) vscode.postMessage({ type: 'open', id: asset.id });
});
const refilter = () => { limit = pageSize; render(); };
search.addEventListener('input', refilter);
kind.addEventListener('change', refilter);
more.addEventListener('click', () => {
    const first = limit;
    limit += pageSize;
    render();
    list.querySelectorAll('button')[first]?.focus();
});
element('asset-open').addEventListener('click', () => { if (selected) vscode.postMessage({ type: 'open', id: selected.id }); });
element('asset-copy').addEventListener('click', () => { if (selected) vscode.postMessage({ type: 'copy', id: selected.id }); });
insert.addEventListener('click', () => {
    if (selected?.source && state.frame) vscode.postMessage({ type: 'insert', id: selected.id, frame: state.frame });
});
place.addEventListener('click', () => {
    if (selected?.source && state.frame) vscode.postMessage({ type: 'insert', id: selected.id, frame: state.frame, choosePlacement: true });
});
vscode.postMessage({ type: 'ready' });

function render(): void {
    const needle = search.value.trim().toLowerCase();
    const results = state.assets.filter(item => (!kind.value || item.kind === kind.value) && item.path.toLowerCase().includes(needle));
    list.replaceChildren(...results.slice(0, limit).map(asset => {
        const row = create('li');
        const button = create('button');
        button.type = 'button';
        button.title = asset.source && state.frame ? `${asset.path} · Drag to the canvas` : asset.path;
        button.dataset.asset = asset.id;
        button.toggleAttribute('data-draggable', Boolean(asset.source && state.frame));
        if (asset.source) {
            button.setAttribute('aria-keyshortcuts', 'Shift+Enter');
            button.addEventListener('keydown', event => {
                if (event.key !== 'Enter' || !event.shiftKey || !state.frame) return;
                event.preventDefault();
                vscode.postMessage({ type: 'insert', id: asset.id, frame: state.frame, choosePlacement: true });
            });
        }
        button.setAttribute('aria-pressed', String(selected?.id === asset.id));
        const icon = create('span');
        const glyph = asset.kind === 'Image' ? 'file-media' : asset.kind === 'XAML' ? 'file-code' : 'file';
        icon.className = `codicon codicon-${glyph}`;
        icon.setAttribute('aria-hidden', 'true');
        const text = create('div');
        text.append(create('strong', asset.label), create('small', asset.path));
        button.append(icon, text);
        row.append(button);
        return row;
    }));
    more.hidden = results.length <= limit;
    showStatus(state.message ?? (results.length ? undefined : 'No matching assets.'));
}

function choose(asset?: ProjectAsset): void {
    selected = asset;
    details.hidden = !asset;
    thumbnail.hidden = true;
    thumbnail.removeAttribute('src');
    for (const row of list.querySelectorAll<HTMLButtonElement>('[data-asset]'))
        row.setAttribute('aria-pressed', String(row.dataset.asset === asset?.id));
    if (!asset) return;
    element('asset-title').textContent = asset.label;
    element('asset-path').textContent = asset.path;
    showPreviewButtons();
    if (asset.thumbnail) vscode.postMessage({ type: 'preview', id: asset.id });
}

function showPreviewButtons(): void {
    insert.hidden = !selected?.source;
    insert.disabled = !state.frame || !state.canInsert;
    place.hidden = !selected?.source;
    place.disabled = !state.frame;
    insert.title = insert.disabled ? 'Select a panel that accepts children or drag onto the artboard' : '';
}

function rowAsset(event: Event): ProjectAsset | undefined {
    const button = (event.target as Element).closest<HTMLButtonElement>('[data-asset]');
    return button ? byId.get(button.dataset.asset ?? '') : undefined;
}

function showStatus(message?: string, error = false): void {
    status.textContent = message ?? '';
    status.hidden = !message;
    status.classList.toggle('error', error);
}
