import type { TokenReviewState } from '../src/designTokenProtocol';
import { create, element, iconButton } from './dom';

declare function acquireVsCodeApi(): { postMessage(message: object): void };

const vscode = acquireVsCodeApi();
const mappings = element('token-mappings');
const issues = element('token-issue-list');
const button = (name: string) => element<HTMLButtonElement>(`token-${name}`);
for (const action of ['candidate', 'refresh', 'scale', 'save']) {
    button(action).addEventListener('click', () => vscode.postMessage({ type: action }));
}

let revision = -1;
window.addEventListener('message', ({ data }: MessageEvent<TokenReviewState>) => {
    if (data.type !== 'review') return;
    element('token-source').textContent = `${data.direction === 'import' ? 'Import' : 'Export'} · ${data.sourceName}`;
    const problem = element('token-problem');
    problem.hidden = !data.problem;
    problem.textContent = data.problem ?? '';
    const counts = `${data.mappingRows.length} mapped · ${data.issues.length} skipped`;
    element('token-status').textContent = data.ready ? counts : data.busy ? 'Preparing token review…' : '';
    element('token-report').setAttribute('aria-busy', String(data.busy));
    button('candidate').disabled = !data.ready;
    button('refresh').disabled = data.busy;
    button('scale').hidden = data.direction !== 'import';
    button('scale').disabled = data.busy || !data.current;
    button('save').disabled = data.busy || !data.current || data.mappingRows.length === 0;
    const canEdit = !data.busy && data.current;
    if (revision !== data.mappingRevision) { renderRows(data); revision = data.mappingRevision; }
    for (const edit of document.querySelectorAll<HTMLButtonElement>('[data-token-edit]')) edit.disabled = !canEdit;
});

function renderRows(data: TokenReviewState): void {
    const focused = (document.activeElement as HTMLElement | undefined)?.dataset.tokenEdit;
    const mapped = document.createDocumentFragment();
    const paths = new Set<string>();
    for (const row of data.mappingRows) {
        const item = create('li');
        const path = data.direction === 'import' ? row.token : row.key;
        paths.add(path);
        const name = create('strong', path);
        const target = data.direction === 'import' ? row.key : row.token;
        const type = data.direction === 'import' ? row.xamlType : row.kind;
        const detail = create('div');
        detail.className = 'token-detail';
        detail.append(name, create('div', target === path ? type : `${target} · ${type}`), create('code', row.value));
        if (row.note) detail.append(create('small', row.note));
        item.append(detail, editButton(path));
        mapped.append(item);
    }
    const skipped = document.createDocumentFragment();
    for (const issue of data.issues) {
        const item = create('li');
        const detail = create('div');
        detail.className = 'token-detail';
        detail.append(create('strong', issue.path || 'Document'), create('small', issue.message));
        item.append(detail);
        if (issue.path && !paths.has(issue.path)) {
            item.append(editButton(issue.path));
        }
        skipped.append(item);
    }
    mappings.replaceChildren(mapped);
    issues.replaceChildren(skipped);
    element('token-issues').hidden = data.issues.length === 0;
    if (focused !== undefined) [...document.querySelectorAll<HTMLButtonElement>('[data-token-edit]')]
        .find(edit => edit.dataset.tokenEdit === focused)?.focus();
}

function editButton(path: string): HTMLButtonElement {
    const edit = iconButton('edit', `Edit mapping for ${path}`, () => vscode.postMessage({ type: 'edit', path }));
    edit.dataset.tokenEdit = path;
    return edit;
}

vscode.postMessage({ type: 'ready' });
