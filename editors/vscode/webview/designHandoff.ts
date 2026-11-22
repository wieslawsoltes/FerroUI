import type { HandoffAction } from '../src/designHandoffActions';
import type { HandoffRow, HandoffState } from '../src/designHandoffProtocol';
import { create, element, icon, iconButton } from './dom';

declare function acquireVsCodeApi(): { postMessage(message: object): void };

const vscode = acquireVsCodeApi();
const list = element('handoff-rows');
const filter = element<HTMLInputElement>('handoff-filter');
const refresh = element<HTMLButtonElement>('handoff-refresh');
const source = element<HTMLButtonElement>('handoff-source');
let revision = -1;
let model: HandoffState | undefined;

refresh.onclick = () => vscode.postMessage({ action: 'refresh' });
source.onclick = () => vscode.postMessage({ action: 'manifest' });
filter.oninput = () => render(model?.rows ?? []);

window.addEventListener('message', ({ data }: MessageEvent<HandoffState>) => {
    if (data.type !== 'handoff') return;
    model = data;
    element('handoff-manifest').textContent = data.manifest;
    document.querySelector('main')?.setAttribute('aria-busy', String(data.busy));
    refresh.disabled = data.busy;
    source.disabled = data.busy;
    const status = element('handoff-status');
    status.textContent = data.status || (data.busy ? 'Resolving design handoff…' : '');
    status.classList.toggle('error', data.error);
    status.hidden = !status.textContent;
    if (revision !== data.revision) {
        revision = data.revision;
        render(data.rows ?? []);
    } else updateButtons();
});

function render(rows: readonly HandoffRow[]) {
    const focused = (document.activeElement as HTMLElement | null)?.dataset.focus;
    const query = filter.value.trim().toLowerCase();
    const visible = rows.filter(row => [row.name, row.id, row.kind, row.target].some(value => value.toLowerCase().includes(query)));
    const issues = rows.filter(row => row.issue).length;
    element('handoff-count').textContent = rows.length
        ? `${visible.length} of ${rows.length} mappings${issues ? ` · ${issues} to review` : ''}` : '';
    const fragment = document.createDocumentFragment();
    for (const row of visible) fragment.append(mapping(row));
    list.replaceChildren(fragment);
    updateButtons();
    if (focused) [...list.querySelectorAll<HTMLButtonElement>('button')].find(button => button.dataset.focus === focused)?.focus();
}

function mapping(row: HandoffRow) {
    const item = create('li');
    const label = create('div');
    label.className = 'handoff-label';
    label.append(icon(row.kind === 'Control' ? 'symbol-class' : row.kind === 'Definition' ? 'library' : 'file-media'), create('strong', row.name));
    if (row.id !== row.name) label.append(create('small', row.id));
    const detail = create('div');
    detail.className = 'handoff-detail';
    detail.append(create('span', row.target));
    if (row.reference && row.reference !== row.target) {
        const reference = create('code', row.reference);
        reference.title = row.reference;
        detail.append(reference);
    }
    if (row.copy) {
        const transfer = create('small', `${row.copy.sourceLabel}\n→ ${row.copy.destinationLabel} · ${row.copy.bytes.toLocaleString()} bytes`);
        transfer.className = 'handoff-transfer';
        transfer.title = `${row.copy.source}\n→ ${row.copy.destination}`;
        detail.append(transfer);
    }
    if (row.issue) {
        const issue = create('small', row.issue);
        issue.className = 'handoff-issue';
        detail.append(issue);
    }
    const tools = create('div');
    tools.className = 'tool-group';
    if (row.review) {
        if (row.copy) tools.append(action(row, 'copy', row.copy.direction === 'import' ? 'cloud-download' : 'cloud-upload',
            row.copy.direction === 'import' ? 'Import asset' : 'Export asset'));
        if (row.insertion) tools.append(action(row, 'place', 'add', 'Choose placement'));
        if (row.definition) tools.append(action(row, 'source', 'go-to-file', 'Reveal definition'),
            action(row, 'preview', 'preview', 'Preview definition'));
        if (row.assetUri) tools.append(action(row, 'openAsset', 'go-to-file', 'Open asset'));
    }
    item.append(label, detail, tools);
    return item;
}

function action(row: HandoffRow, action: HandoffAction, glyph: string, name: string) {
    const button = iconButton(glyph, `${name} · ${row.name}`, () => vscode.postMessage({ action, id: row.id }));
    button.dataset.focus = JSON.stringify([row.id, action]);
    return button;
}

function updateButtons() {
    for (const button of list.querySelectorAll<HTMLButtonElement>('button')) button.disabled = !model?.current || model.busy;
}

vscode.postMessage({ action: 'ready' });
