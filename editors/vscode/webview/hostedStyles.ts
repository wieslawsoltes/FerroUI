import type { HostedFrame } from '../src/hostedProtocol';
import type { FromHostedStyles, HostedStyleCommand, ToHostedStyles } from '../src/hostedStylesProtocol';
import type { RustStyleEdit, StyleCatalog, StyleDefinition, StylesAction } from '../src/stylesProtocol';
import { anchorPopover, element } from './dom';
import { sourceOptions } from './hostedSourceOptions';
import { styleDefinition } from './styleDefinitionEditor';

type OpenStyles = Extract<ToHostedStyles, { type: 'openStyles' }>;

export class HostedStylesView {
    private readonly toggle = element<HTMLButtonElement>('hosted-styles-button');
    private readonly popup = element('hosted-styles');
    private readonly file = element<HTMLSelectElement>('hosted-styles-file');
    private readonly list = element<HTMLSelectElement>('hosted-styles-definition');
    private readonly create = element<HTMLButtonElement>('hosted-styles-add');
    private readonly issues = element<HTMLButtonElement>('hosted-style-issues');
    private readonly status = element('hosted-styles-status');
    private readonly fields = element<HTMLFieldSetElement>('hosted-styles-fields');
    private frame?: HostedFrame;
    private readyRevision?: number;
    private opening?: OpenStyles;
    private catalog?: { address: string; revision: number; model: StyleCatalog };
    private definition?: StyleDefinition<string>;
    private requested = 0;
    private pending?: number;
    private refresh = true;
    private error?: string;
    private focusAfterEdit?: string;
    private nextName?: string;

    constructor(private readonly post: (message: FromHostedStyles) => void) {
        anchorPopover(this.toggle, this.popup);
        this.popup.addEventListener('toggle', event => {
            const open = event.newState === 'open';
            this.toggle.setAttribute('aria-expanded', String(open));
            if (open) {
                this.refresh = true;
                this.error = undefined;
                this.sync();
            } else {
                this.opening = undefined;
                this.focusAfterEdit = undefined;
                this.nextName = undefined;
            }
        });
        this.file.onchange = () => this.readCatalog();
        this.list.onchange = () => this.readDefinition();
        this.create.onclick = () => this.send({ action: 'create', address: this.file.value });
        this.issues.onclick = () => this.send({ action: 'issues', address: this.file.value });
    }

    setReady(revision: number | undefined): void { this.readyRevision = revision; this.sync(); }

    rendered(frame: HostedFrame): void {
        if (this.frame?.revision !== frame.revision) {
            this.pending = undefined;
            this.error = undefined;
            this.refresh = true;
        }
        if (this.opening && this.opening.revision < frame.revision) this.opening = undefined;
        this.frame = frame;
        this.sync();
    }

    open(message: OpenStyles): void {
        if (this.frame && this.frame.revision > message.revision) return;
        this.opening = message;
        this.refresh = true;
        this.sync();
    }

    receive(message: Extract<ToHostedStyles, { type: 'stylesReply' }>): void {
        if (message.request !== this.pending || message.frame.revision !== this.frame?.revision) return;
        this.pending = undefined;
        if (!this.popup.matches(':popover-open')) return;
        this.error = message.error;
        if (message.error) this.nextName = undefined;
        const result = message.result;
        if (result && 'documents' in result) {
            const address = this.opening?.target?.address ?? this.file.value;
            this.file.replaceChildren(...sourceOptions(result.documents));
            if (result.documents.some(file => file.address === address)) this.file.value = address;
            this.readCatalog();
        } else if (result && 'catalog' in result) {
            this.catalog = { address: this.file.value, revision: message.frame.revision, model: result.catalog };
            const definitions = result.catalog.definitions;
            const previous = this.definition;
            const renamed = definitions.filter(entry => entry.name === this.nextName);
            const matching = previous ? definitions.filter(entry => entry.kind === previous.kind && entry.name === previous.name) : [];
            const selected = definitions.find(entry => entry.node === this.opening?.target?.node)
                ?? (renamed.length === 1 ? renamed[0] : undefined)
                ?? (matching.length === 1 ? matching[0] : definitions[0]);
            this.opening = undefined;
            this.nextName = undefined;
            this.list.replaceChildren(...definitions.map(entry => new Option(
                [entry.name, entry.scope].filter(Boolean).join(' · '), String(entry.node))));
            if (selected) this.list.value = String(selected.node);
            this.readDefinition();
        } else if (result && 'definition' in result) {
            this.definition = result.definition;
            this.render();
        } else if (message.error) {
            this.render();
        }
        this.sync();
    }

    private readCatalog(): void {
        if (this.file.value) this.send({ action: 'catalog', address: this.file.value });
    }

    private readDefinition(): void {
        const address = this.file.value;
        const node = Number(this.list.value);
        this.definition = undefined;
        if (address && node) this.send({ action: 'read', target: { address, node } });
        else this.render();
    }

    private get ready(): boolean { return Boolean(this.frame?.stylesAvailable && this.frame.revision === this.readyRevision); }

    private send(command: HostedStyleCommand): void {
        if (!this.frame || !this.ready || this.pending !== undefined) return;
        const { revision, sequence } = this.frame;
        const request = ++this.requested;
        this.pending = request;
        this.error = undefined;
        this.post({ type: 'styles', request, frame: { revision, sequence }, command });
        this.sync();
    }

    private sync(): void {
        this.toggle.hidden = !this.frame?.stylesAvailable;
        this.toggle.disabled = !this.ready;
        const open = this.popup.matches(':popover-open');
        if (this.ready && this.opening?.revision === this.frame?.revision && !open) { this.popup.showPopover(); return; }
        if (!open) return;
        if (this.ready && this.refresh && this.pending === undefined) {
            this.refresh = false;
            this.send({ action: 'files' });
            return;
        }
        const idle = this.ready && this.pending === undefined;
        const catalog = this.catalog?.address === this.file.value && this.catalog.revision === this.frame?.revision
            ? this.catalog.model : undefined;
        this.file.disabled = !idle || this.file.options.length < 2;
        this.list.disabled = !idle || !catalog || this.list.options.length < 2;
        this.create.disabled = !idle || !catalog || !(catalog.style_hosts.length || catalog.palette_scopes.length);
        this.issues.hidden = !catalog?.issues?.length;
        this.issues.disabled = !idle;
        this.fields.disabled = !idle || !catalog || !this.definition;
        const message = this.error ?? (idle ? '' : 'Waiting for the current styles…');
        if (this.status.textContent !== message) this.status.textContent = message;
        this.status.hidden = !message;
        this.status.classList.toggle('error', Boolean(this.error));
        if (!this.fields.disabled && this.focusAfterEdit) {
            if (document.activeElement === document.body || this.fields.contains(document.activeElement)) {
                document.getElementById(this.focusAfterEdit)?.focus({ preventScroll: true });
            }
            this.focusAfterEdit = undefined;
        }
    }

    private render(): void {
        this.fields.replaceChildren(styleDefinition(this.definition, action => this.action(action), false));
        this.sync();
    }

    private action(action: StylesAction<string>): void {
        if (!this.definition || this.fields.disabled) return;
        const target = { address: this.file.value, node: this.definition.node };
        if (this.fields.contains(document.activeElement)) this.focusAfterEdit = document.activeElement?.id;
        const rust = this.catalog?.model.source_kind === 'rust';
        const rustEdit = (edit: RustStyleEdit) => this.send({ action: 'edit', target, edit: { action: 'rust', edit } });
        switch (action.type) {
            case 'selector':
                this.nextName = action.value.trim();
                if (rust) rustEdit({ action: 'selector', style: target.node, selector: action.value });
                else this.send({ action: 'edit', target, edit: { action: 'selector', value: action.value } });
                break;
            case 'setter':
                if (rust) rustEdit({ action: 'setter', style: target.node, setter: action.node, value: action.value });
                else this.send({ action: 'edit', target, edit: { action: 'setter', property: action.property, value: action.value } });
                break;
            case 'base': this.send({ action: 'edit', target, edit: { action: 'base', key: action.base } }); break;
            case 'remove-style':
                if (rust) rustEdit({ action: 'remove', style: target.node });
                else this.send({ action: 'edit', target, edit: { action: 'remove' } });
                break;
            case 'add-setter': this.send({ action: 'addSetter', target }); break;
            case 'add-nested-style': this.send({ action: 'addNested', target }); break;
            case 'setter-resource': this.send({ action: 'chooseResource', target, property: action.property }); break;
            case 'reveal': this.send({ action: 'locate', address: target.address, range: action.range }); break;
        }
    }
}
