import type { Asset, FromToolbox, ToolboxState, ToToolbox } from '../src/protocol';
import { create, element } from './dom';
import { DragSource } from './drag';

declare function acquireVsCodeApi(): {
    postMessage(message: FromToolbox): void;
    getState(): { tiles?: boolean } | undefined;
    setState(state: { tiles: boolean }): void;
};
const vscode = acquireVsCodeApi();
const search = element<HTMLInputElement>('asset-search');
const mode = element('asset-view');
const list = element('asset-list');
const status = element('toolbox-message');
let state: ToolboxState = { assets: [], canInsert: false };
let tiles = Boolean(vscode.getState()?.tiles);
let listKey = '';
let error: string | undefined;

const dragSource = new DragSource(list, event => {
    const control = (event.target as Element).closest<HTMLElement>('[data-asset]')?.dataset.asset;
    if (control && state.frame) return { frame: state.frame, origin: { type: 'asset', control } };
}, vscode.postMessage);

mode.addEventListener('click', () => { tiles = !tiles; vscode.setState({ tiles }); render(); });
search.addEventListener('input', render);
window.addEventListener('message', ({ data }: MessageEvent<ToToolbox>) => {
    if (data.type === 'drag') {
        if (!data.gesture) dragSource.end(false);
        return;
    }
    if (data.type === 'state') {
        if (state.frame?.token !== data.state.frame?.token || state.frame?.session !== data.state.frame?.session) error = undefined;
        state = data.state;
    } else error = data.error;
    render();
});
vscode.postMessage({ type: 'ready' });

function render(): void {
    const query = search.value.toLowerCase().trim();
    const key = JSON.stringify([state.assets, query, tiles, state.frame?.session, !!state.frame, state.canInsert]);
    if (key !== listKey) {
        listKey = key;
        const focused = (document.activeElement as HTMLElement | null)?.dataset.asset;
        const groups = new Map<string, Asset[]>();
        for (const asset of state.assets) {
            if (!`${asset.title} ${asset.category} ${asset.description}`.toLowerCase().includes(query)) continue;
            const group = groups.get(asset.category) ?? [];
            group.push(asset);
            groups.set(asset.category, group);
        }
        mode.setAttribute('aria-pressed', String(tiles));
        list.classList.toggle('tiles', tiles);
        list.replaceChildren(...[...groups].map(([category, assets]) => {
            const section = create('section');
            const heading = create('h2', category);
            const items = create('div');
            items.className = 'assets';
            items.append(...assets.map(control));
            section.append(heading, items);
            return section;
        }));
        if (focused) [...list.querySelectorAll<HTMLButtonElement>('button')].find(button => button.dataset.asset === focused)?.focus();
    }
    status.textContent = error ?? state.message ?? (state.assets.length === 0 ? 'Open a FerroUI designer to insert controls.'
        : list.childElementCount === 0 ? 'No controls match your search.' : 'Drag a control into the design, or press Enter to insert it.');
    status.classList.toggle('error', Boolean(error));
}

function control(asset: Asset): HTMLButtonElement {
    const button = create('button');
    button.type = 'button';
    button.className = 'asset';
    button.dataset.asset = asset.id;
    button.title = asset.description;
    button.setAttribute('aria-disabled', String(!state.frame));
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('viewBox', '0 0 24 24');
    svg.setAttribute('aria-hidden', 'true');
    const path = document.createElementNS(svg.namespaceURI, 'path');
    path.setAttribute('d', asset.icon);
    svg.append(path);
    button.append(svg, create('span', asset.title));
    button.addEventListener('click', () => {
        if (!state.frame) return;
        if (!state.canInsert) {
            error = 'Select an available container, or drag the control to its drop target.';
            render();
            return;
        }
        error = undefined;
        vscode.postMessage({ type: 'insert', frame: state.frame, control: asset.id });
    });
    return button;
}
