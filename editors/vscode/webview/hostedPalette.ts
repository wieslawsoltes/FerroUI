import type { FromHostedPalette, HostedPaletteAction, HostedPaletteDocument, HostedPaletteTarget, ToHostedPalette } from '../src/hostedPaletteProtocol';
import type { HostedFrame } from '../src/hostedProtocol';
import { paletteLayerLabels, type PaletteLayer, type PaletteSheet, type StylesAction } from '../src/stylesProtocol';
import { anchorPopover, element } from './dom';
import { paletteEditor } from './paletteEditor';
import { sourceOptions } from './hostedSourceOptions';

type OpenPalette = Extract<ToHostedPalette, { type: 'openPalette' }>;

export class HostedPaletteView {
    private readonly toggle = element<HTMLButtonElement>('hosted-palette-button');
    private readonly popup = element('hosted-palette');
    private readonly file = element<HTMLSelectElement>('hosted-palette-file');
    private readonly scope = element<HTMLSelectElement>('hosted-palette-scope');
    private readonly layer = element<HTMLSelectElement>('hosted-palette-layer');
    private readonly search = element<HTMLInputElement>('hosted-palette-search');
    private readonly compare = element<HTMLButtonElement>('hosted-palette-compare');
    private readonly add = element<HTMLButtonElement>('hosted-palette-add');
    private readonly status = element('hosted-palette-status');
    private readonly colors = element<HTMLFieldSetElement>('hosted-palette-colors');
    private frame?: HostedFrame;
    private readyRevision?: number;
    private documents: HostedPaletteDocument[] = [];
    private sheet?: PaletteSheet;
    private sequence = 0;
    private pending?: number;
    private refresh = true;
    private opening?: OpenPalette;
    private comparing?: boolean;
    private preferredLayer?: PaletteLayer;
    private focusAfterEdit?: string;
    private error?: string;

    constructor(private readonly post: (message: FromHostedPalette) => void) {
        anchorPopover(this.toggle, this.popup);
        this.popup.addEventListener('toggle', event => {
            const open = event.newState === 'open';
            this.toggle.setAttribute('aria-expanded', String(open));
            if (open) {
                this.error = undefined;
                this.refresh = true;
                this.sync();
            } else {
                this.opening = undefined;
                this.focusAfterEdit = undefined;
            }
        });
        this.file.onchange = () => { this.preferredLayer = undefined; this.chooseFile(); };
        this.scope.onchange = () => { this.preferredLayer = undefined; this.read(); };
        this.layer.onchange = () => { this.preferredLayer = this.layer.value as PaletteLayer; this.render(); };
        this.search.oninput = () => this.render();
        this.compare.onclick = () => {
            this.comparing = !this.isComparing;
            this.render();
        };
        this.add.onclick = () => {
            const target = this.target();
            if (target) this.send({ action: 'add', target, layer: this.layer.value as PaletteLayer });
        };
    }

    setReady(revision: number | undefined): void {
        this.readyRevision = revision;
        this.sync();
    }

    rendered(frame: HostedFrame): void {
        if (this.frame?.revision !== frame.revision) {
            this.refresh = true;
            this.pending = undefined;
            this.error = undefined;
        }
        if (this.opening && this.opening.revision < frame.revision) this.opening = undefined;
        this.frame = frame;
        this.sync();
    }

    open(message: OpenPalette): void {
        if (this.frame && this.frame.revision > message.revision) return;
        this.opening = message;
        this.refresh = true;
        this.sync();
    }

    receive(message: Extract<ToHostedPalette, { type: 'paletteReply' }>): void {
        if (message.request !== this.pending || message.frame.revision !== this.frame?.revision) return;
        this.pending = undefined;
        if (!this.popup.matches(':popover-open')) return;
        this.error = message.error;
        if (message.result && 'documents' in message.result) {
            const presentation = this.opening?.presentation;
            const address = presentation?.target?.address ?? this.file.value;
            const previous = this.selectedFile()?.scopes.find(scope => String(scope.node) === this.scope.value);
            this.documents = message.result.documents;
            this.file.replaceChildren(...sourceOptions(this.documents));
            if (this.documents.some(file => file.address === address)) this.file.value = address;
            if (presentation?.compare !== undefined) this.comparing = presentation.compare;
            if (presentation?.layer) this.preferredLayer = presentation.layer;
            this.opening = undefined;
            this.chooseFile(this.file.value === address ? previous?.label : undefined, presentation?.target?.owner);
        } else if (message.result && 'palette' in message.result) {
            this.sheet = message.result.palette;
            const layers = this.sheet.layers;
            this.layer.replaceChildren(...layers.map(layer => new Option(paletteLayerLabels[layer.layer], layer.layer)));
            const selected = layers.find(layer => layer.layer === this.preferredLayer) ?? layers.find(layer => layer.colors.length);
            if (selected) this.layer.value = selected.layer;
            this.render();
        } else if (message.error) {
            this.render();
        }
        this.sync();
    }

    private selectedFile(): HostedPaletteDocument | undefined {
        return this.documents.find(file => file.address === this.file.value);
    }

    private chooseFile(previousLabel?: string, owner?: number): void {
        const scopes = this.selectedFile()?.scopes ?? [];
        const matching = previousLabel ? scopes.filter(scope => scope.label === previousLabel) : [];
        this.scope.replaceChildren(...scopes.map(scope => new Option(scope.label, String(scope.node))));
        if (scopes.some(scope => scope.node === owner)) this.scope.value = String(owner);
        else if (matching.length === 1) this.scope.value = String(matching[0].node);
        this.read();
    }

    private target(): HostedPaletteTarget | undefined {
        return this.file.value && this.scope.value ? { address: this.file.value, owner: Number(this.scope.value) } : undefined;
    }

    private read(): void {
        const target = this.target();
        this.sheet = undefined;
        this.colors.replaceChildren();
        if (target) this.send({ action: 'read', target });
    }

    private get ready(): boolean {
        return Boolean(this.frame?.paletteAvailable && this.readyRevision === this.frame.revision);
    }

    private get isComparing(): boolean {
        return this.comparing ?? Boolean(this.sheet?.comparison.length && !this.sheet.layers.some(layer => layer.colors.length));
    }

    private send(command: HostedPaletteAction): void {
        if (!this.frame || !this.ready || this.pending !== undefined) return;
        const { revision, sequence } = this.frame;
        const request = ++this.sequence;
        this.pending = request;
        this.error = undefined;
        this.post({ type: 'palette', request, frame: { revision, sequence }, command });
        this.sync();
    }

    private sync(): void {
        this.toggle.hidden = !this.frame?.paletteAvailable;
        this.toggle.disabled = !this.ready;
        if (this.ready && this.opening?.revision === this.frame?.revision && !this.popup.matches(':popover-open')) {
            this.popup.showPopover();
            return;
        }
        if (!this.popup.matches(':popover-open')) return;
        if (this.ready && this.refresh && this.pending === undefined) {
            this.refresh = false;
            this.send({ action: 'catalog' });
            return;
        }
        const idle = this.ready && this.pending === undefined;
        const enabled = idle && Boolean(this.sheet);
        this.file.disabled = !idle || this.documents.length < 2;
        this.scope.disabled = !idle || this.scope.options.length < 2;
        this.colors.disabled = this.compare.disabled = this.search.disabled = !enabled;
        const layer = this.sheet?.layers.find(layer => layer.layer === this.layer.value);
        this.compare.setAttribute('aria-pressed', String(this.isComparing));
        this.layer.disabled = !enabled || this.isComparing;
        this.add.disabled = !enabled || this.isComparing || !layer || Boolean(layer.unavailable);
        const message = this.error ?? (idle ? '' : 'Waiting for the current palette…');
        if (this.status.textContent !== message) this.status.textContent = message;
        this.status.hidden = !message;
        this.status.classList.toggle('error', Boolean(this.error));
        if (enabled && this.focusAfterEdit) {
            if (document.activeElement === document.body || this.colors.contains(document.activeElement)) {
                document.getElementById(this.focusAfterEdit)?.focus({ preventScroll: true });
            }
            this.focusAfterEdit = undefined;
        }
    }

    private render(): void {
        const layer = this.sheet?.layers.find(layer => layer.layer === this.layer.value);
        const view = paletteEditor(this.sheet, layer, this.isComparing, this.search.value.trim().toLowerCase(), action => this.action(action));
        this.colors.replaceChildren(view);
        this.sync();
    }

    private action(action: StylesAction): void {
        const target = this.target();
        if (!target) return;
        switch (action.type) {
            case 'palette-color':
                if (this.colors.contains(document.activeElement)) this.focusAfterEdit = `palette-value-${action.node}`;
                this.send({ action: 'edit', target, edit: { action: 'set', node: action.node, color: action.color } });
                break;
            case 'references': this.send({ action: 'references', target, node: action.node }); break;
            case 'reveal': this.send({ action: 'locate', address: target.address, range: action.range }); break;
            case 'included-color': {
                const source = this.sheet?.comparison.find(row => row.key === action.key)?.[action.theme]?.source;
                const file = this.documents.find(file => file.uri === source?.uri);
                if (file && source) this.send({ action: 'locate', address: file.address, range: source.range });
                break;
            }
        }
    }
}
