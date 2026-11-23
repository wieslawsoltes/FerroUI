import type { FromHosted, HostedFrame, ToHosted } from '../src/hostedProtocol';
import { element } from './dom';
import { HostedAnimationView } from './hostedAnimations';
import { HostedPaletteView } from './hostedPalette';
import { HostedStylesView } from './hostedStyles';

declare function acquireVsCodeApi(): {
    postMessage(message: FromHosted): void;
    getState(): { compare?: boolean } | undefined;
    setState(state: { compare: boolean }): void;
};
const vscode = acquireVsCodeApi();
const timelineLayout = vscode.getState()?.compare ? 'clips' : 'tracks';
const animations = new HostedAnimationView(message => vscode.postMessage(message), timelineLayout,
    compare => vscode.setState({ compare }));
const palettes = new HostedPaletteView(message => vscode.postMessage(message));
const styles = new HostedStylesView(message => vscode.postMessage(message));
const canvas = element<HTMLImageElement>('hosted-image');
const status = element('hosted-status');
const input = element<HTMLFormElement>('hosted-input');
const text = element<HTMLInputElement>('hosted-text');
const send = input.querySelector<HTMLButtonElement>('button[type=submit]')!;
const fixtureLabel = element('hosted-fixture-label');
const fixture = element<HTMLSelectElement>('hosted-fixture');
const templates = element<HTMLButtonElement>('hosted-templates-button');
let fixtureRevision = 0;
let availableFixtures: string[] = [];
let current: HostedFrame | undefined;
let interactive = false;
let request = 0;

window.addEventListener('message', async ({ data }: MessageEvent<ToHosted>) => {
    if (data.type === 'animations') { animations.update(data.state, data.version, data.layout); return; }
    if (data.type === 'animationGraphMode') { animations.setGraphMode(data.mode); return; }
    if (data.type === 'timelineLayout') { animations.setTimelineLayout(data.layout); return; }
    if (data.type === 'timelinePresentationRequest') { animations.timelinePresentation(data); return; }
    if (data.type === 'animationReply') { animations.reply(data.request); return; }
    if (data.type === 'curveReply') { animations.curve(data); return; }
    if (data.type === 'openKeyframe') { animations.keyframes.open(data.key); return; }
    if (data.type === 'keyframeReply') { animations.keyframes.receive(data); return; }
    if (data.type === 'keyframeLaneReply') { animations.keyLanes.receive(data); return; }
    if (data.type === 'paletteReply') { palettes.receive(data); return; }
    if (data.type === 'openPalette') { palettes.open(data); return; }
    if (data.type === 'stylesReply') { styles.receive(data); return; }
    if (data.type === 'openStyles') { styles.open(data); return; }
    if (data.type === 'status') {
        if (status.textContent !== data.message) status.textContent = data.message;
        interactive = data.interactive;
        text.disabled = !interactive;
        send.disabled = !interactive;
        const { available, selected, revision, pending } = data.fixtures;
        palettes.setReady(interactive ? revision : undefined);
        styles.setReady(interactive ? revision : undefined);
        fixtureRevision = revision;
        updateTemplates();
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
    animations.rendered(current);
    palettes.rendered(current);
    styles.rendered(current);
    updateTemplates();
});

function updateTemplates(): void {
    templates.hidden = !current?.templatesAvailable;
    templates.disabled = !interactive || current?.revision !== fixtureRevision;
}

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
templates.onclick = () => {
    if (current && !templates.disabled) {
        const { revision, sequence } = current;
        vscode.postMessage({ type: 'templates', frame: { revision, sequence } });
    }
};
fixture.onchange = () => vscode.postMessage({ type: 'fixture', revision: fixtureRevision, name: fixture.value || null });
vscode.postMessage({ type: 'ready', layout: timelineLayout });
