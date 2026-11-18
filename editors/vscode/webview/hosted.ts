import type { FromHosted, HostedFrame, ToHosted } from '../src/hostedProtocol';
import { element } from './dom';

declare function acquireVsCodeApi(): { postMessage(message: FromHosted): void };
const vscode = acquireVsCodeApi();
const canvas = element<HTMLImageElement>('hosted-image');
const status = element('hosted-status');
const input = element<HTMLFormElement>('hosted-input');
const text = element<HTMLInputElement>('hosted-text');
const send = input.querySelector<HTMLButtonElement>('button[type=submit]')!;
const fixtureLabel = element('hosted-fixture-label');
const fixture = element<HTMLSelectElement>('hosted-fixture');
let fixtureRevision = 0;
let availableFixtures: string[] = [];
let current: HostedFrame | undefined;
let interactive = false;
let request = 0;

window.addEventListener('message', async ({ data }: MessageEvent<ToHosted>) => {
    if (data.type === 'status') {
        status.textContent = data.message;
        interactive = data.interactive;
        text.disabled = !interactive;
        send.disabled = !interactive;
        const { available, selected, revision, pending } = data.fixtures;
        fixtureRevision = revision;
        fixtureLabel.hidden = !available.length;
        if (available.length !== availableFixtures.length || available.some((name, index) => name !== availableFixtures[index])) {
            availableFixtures = available;
            fixture.replaceChildren(new Option('Source preview', ''), ...available.map(name => new Option(name, name)));
        }
        fixture.value = selected ?? '';
        fixture.disabled = pending;
        return;
    }
    const decoded = new Image();
    const sequence = ++request;
    decoded.src = `data:image/png;base64,${data.frame.image.png}`;
    try { await decoded.decode(); } catch { return; }
    if (sequence !== request) return;
    current = data.frame;
    canvas.src = decoded.src;
    canvas.hidden = false;
    input.hidden = false;
});

function position(event?: MouseEvent) {
    if (!current || !interactive) return undefined;
    const rect = canvas.getBoundingClientRect();
    return { revision: current.revision, sequence: current.sequence,
        x: event ? (event.clientX - rect.left) / rect.width : 0.5,
        y: event ? (event.clientY - rect.top) / rect.height : 0.5 };
}

canvas.onclick = event => {
    const point = position(event);
    if (point) vscode.postMessage({ type: 'click', ...point });
};
canvas.addEventListener('wheel', event => {
    const point = position(event);
    if (point) { event.preventDefault(); vscode.postMessage({ type: 'scroll', ...point, delta: -event.deltaY / 80 }); }
}, { passive: false });
input.onsubmit = event => {
    event.preventDefault();
    const point = position();
    if (point && text.value) {
        vscode.postMessage({ type: 'text', ...point, text: text.value });
        text.value = '';
    }
};
element('hosted-refresh').onclick = () => vscode.postMessage({ type: 'refresh' });
fixture.onchange = () => vscode.postMessage({ type: 'fixture', revision: fixtureRevision, name: fixture.value || null });
vscode.postMessage({ type: 'ready' });
