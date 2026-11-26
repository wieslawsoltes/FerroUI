import type { BaselineComparison, ImageSize, VisualReview } from '../src/scenarioRunner';
import { create, element } from './dom';

declare function acquireVsCodeApi(): { postMessage(message: object): void };

type Mode = 'split' | 'overlay' | 'difference';
type Capture = Omit<VisualReview, 'actual'> & { actual: string | null };
type Message = {
    type: 'review';
    captures: Capture[];
    reference?: { name: string; commit: string };
    approved?: string;
    error?: string;
} | { type: 'status'; busy: boolean; text: string; error?: boolean };

const vscode = acquireVsCodeApi();
const selector = element<HTMLSelectElement>('capture');
const canvases = element('image-review');
const message = element('review-message');
const approve = element<HTMLButtonElement>('approve');
const refresh = element<HTMLButtonElement>('rerun-scenario');
const wipe = element<HTMLInputElement>('wipe');
const modes: Mode[] = ['split', 'overlay', 'difference'];
const frames = ['expected', 'actual', 'overlay-frame', 'diff-frame'].map(id => element(id));
const scroll = frames.map(frame => frame.querySelector<HTMLElement>('.review-scroll')!);
let captures: Capture[] = [];
let selected: Capture | undefined;
let reference: { name: string; commit: string } | undefined;
let mode: Mode = 'split';
let zoom: number | undefined;
let rendering = 0;
let busy = false;

function status(text: string, error = false): void {
    message.hidden = !text;
    message.textContent = text;
    message.classList.toggle('error', error);
}

function setBusy(value: boolean): void {
    busy = value;
    refresh.disabled = value;
    approve.disabled = value;
    canvases.setAttribute('aria-busy', String(value));
}

function options(): void {
    selector.replaceChildren(...captures.map(item => new Option(
        `Step ${item.step} · ${item.baseline.split(/[\\/]/).at(-1)}`, String(item.step))));
    selector.hidden = captures.length < 2;
}

function setMode(next: Mode): void {
    const sameSize = selected?.comparison.expected?.width === selected?.comparison.actual.width
        && selected?.comparison.expected?.height === selected?.comparison.actual.height;
    const overlay = !!selected?.expected && sameSize;
    const difference = !!selected?.difference;
    if ((next === 'overlay' && !overlay) || (next === 'difference' && !difference)) next = 'split';
    mode = next;
    canvases.dataset.mode = mode;
    for (const option of modes) {
        const button = element<HTMLButtonElement>(option);
        button.disabled = (option === 'overlay' && !overlay) || (option === 'difference' && !difference);
        button.setAttribute('aria-pressed', String(option === mode));
    }
    frames.forEach((frame, index) => {
        frame.hidden = index < 2 ? mode !== 'split' : mode !== (index === 2 ? 'overlay' : 'difference');
    });
    resize();
}

async function decoded(url: string | null, alt: string): Promise<HTMLImageElement | undefined> {
    if (!url) return undefined;
    const picture = create('img');
    picture.alt = alt;
    picture.draggable = false;
    picture.src = url;
    await picture.decode();
    return picture;
}

function imageTile(image: HTMLImageElement | undefined, label: string): HTMLElement {
    const tile = create('div');
    tile.className = 'review-tile';
    tile.append(image ?? create('p', label));
    return tile;
}

function describe(comparison: BaselineComparison): string {
    if (comparison.status === 'missing') return 'New screenshot';
    if (comparison.status === 'resized') return `${comparison.expected?.width} × ${comparison.expected?.height} → ${comparison.actual.width} × ${comparison.actual.height}`;
    const count = comparison.changedPixels ?? 0;
    const pixels = `${count.toLocaleString()} changed ${count === 1 ? 'pixel' : 'pixels'}`;
    return comparison.maxChangedPixels ? `${pixels} · ${comparison.maxChangedPixels} allowed` : pixels;
}

async function select(item: Capture): Promise<void> {
    const sequence = ++rendering;
    approve.disabled = true;
    try {
        const [expected, actual, difference] = await Promise.all([
            decoded(item.expected, 'Expected screenshot'),
            decoded(item.actual, 'Current native screenshot'),
            decoded(item.difference, 'Changed pixels highlighted in magenta'),
        ]);
        if (sequence !== rendering) return;
        selected = item;
        selector.value = String(item.step);
        element('reference').textContent = reference ? `${reference.name} · ${reference.commit.slice(0, 8)}` : 'Workspace baseline';
        element('pixels').textContent = describe(item.comparison);
        const workspace = element('workspace-status');
        workspace.textContent = item.workspace.status === 'match' ? 'Baseline verified'
            : item.workspace.status === 'missing' ? 'Baseline needed' : 'Awaiting approval';
        workspace.dataset.pending = String(item.workspace.status !== 'match');
        scroll[0].replaceChildren(imageTile(expected, reference ? 'No image at the merge base' : 'No approved baseline'));
        scroll[1].replaceChildren(imageTile(actual, 'Capture unavailable'));
        scroll[3].replaceChildren(imageTile(difference, 'No highlighted differences'));
        const layers = imageTile(expected?.cloneNode() as HTMLImageElement | undefined, '');
        if (actual && expected) {
            const foreground = actual.cloneNode() as HTMLImageElement;
            foreground.className = 'review-overlay';
            layers.append(foreground);
        }
        scroll[2].replaceChildren(layers);
        approve.hidden = !item.approval;
        approve.disabled = busy;
        for (const id of ['comparison-tools', 'review-summary', 'review-zoom']) element(id).hidden = false;
        canvases.hidden = false;
        setMode(mode);
    } catch {
        if (sequence === rendering) status('A screenshot could not be displayed. Run the review again.', true);
    }
}

function resize(): void {
    if (!selected || canvases.hidden) return;
    canvases.dataset.fit = String(zoom === undefined);
    const visible = scroll.filter((_, index) => !frames[index].hidden);
    const sizes = [selected.comparison.actual, selected.comparison.expected].filter((size): size is ImageSize => !!size);
    const width = Math.max(...sizes.map(size => size.width));
    const height = Math.max(...sizes.map(size => size.height));
    const fit = Math.max(0.05, Math.min(1, ...visible.map(view =>
        Math.min((view.clientWidth - 42) / width, (view.clientHeight - 42) / height))));
    const scale = zoom ?? fit;
    const imageSizes = [selected.comparison.expected, selected.comparison.actual, selected.comparison.actual, selected.comparison.actual];
    scroll.forEach((view, index) => {
        const size = imageSizes[index];
        const tile = view.firstElementChild as HTMLElement | null;
        if (tile && size) {
            tile.style.inlineSize = `${size.width * scale}px`;
            tile.style.blockSize = `${size.height * scale}px`;
        }
    });
    element('fit').textContent = zoom === undefined ? 'Fit' : `${Math.round(scale * 100)}%`;
    canvases.style.setProperty('--wipe', `${wipe.value}%`);
}

function clear(): void {
    ++rendering;
    selected = undefined;
    canvases.hidden = approve.hidden = true;
    for (const id of ['comparison-tools', 'review-summary', 'review-zoom']) element(id).hidden = true;
}

function changeZoom(factor: number): void {
    const tile = scroll.find((_, index) => !frames[index].hidden)?.querySelector('img');
    const current = zoom ?? (tile && tile.naturalWidth ? tile.clientWidth / tile.naturalWidth : 1);
    zoom = Math.max(0.05, Math.min(8, current * factor));
    resize();
}

let synced: HTMLElement | undefined;
for (const viewport of scroll.slice(0, 2)) {
    viewport.addEventListener('scroll', () => {
        if (mode !== 'split' || (synced && synced !== viewport)) return;
        synced = viewport;
        const other = scroll[viewport === scroll[0] ? 1 : 0];
        other.scrollTo(viewport.scrollLeft, viewport.scrollTop);
        requestAnimationFrame(() => { synced = undefined; });
    });
}
new ResizeObserver(resize).observe(canvases);
wipe.oninput = resize;
selector.onchange = () => {
    const item = captures.find(item => String(item.step) === selector.value);
    if (item) void select(item);
};
for (const option of modes) element<HTMLButtonElement>(option).onclick = () => setMode(option);
element('zoom-out').onclick = () => changeZoom(0.8);
element('zoom-in').onclick = () => changeZoom(1.25);
element('fit').onclick = () => { zoom = undefined; resize(); };
element('open-scenario').onclick = () => vscode.postMessage({ type: 'source' });
refresh.onclick = () => vscode.postMessage({ type: 'refresh' });
approve.onclick = () => {
    if (!busy && selected?.approval) {
        vscode.postMessage({ type: 'approve', step: selected.step, approval: selected.approval });
    }
};

window.addEventListener('message', ({ data }: MessageEvent<Message>) => {
    if (data.type === 'status') {
        setBusy(data.busy);
        if (!data.busy) approve.disabled = !selected?.approval;
        status(data.text, data.error);
        return;
    }
    if (data.type !== 'review') return;
    reference = data.reference;
    captures = data.captures;
    options();
    setBusy(false);
    status(data.error ?? (data.approved ? `Approved ${data.approved}` : captures.length ? '' : 'The scenario has no screenshot assertions.'), !!data.error);
    const item = captures.find(item => item.step === selected?.step) ?? captures[0];
    if (item) void select(item);
    else clear();
});
vscode.postMessage({ type: 'ready' });
