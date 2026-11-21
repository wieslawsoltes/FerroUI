import type { CanvasGuidesState, GuideAxis, GuideEdit } from '../src/canvasGuidesProtocol';
import { anchorPopover, create, element, icon, iconButton } from './dom';

type SendEdit = (state: CanvasGuidesState, edit: GuideEdit) => number | undefined;
interface Row { id?: number; axis: GuideAxis; input: HTMLInputElement }

export class GuideEditor {
    readonly button = element<HTMLButtonElement>('guides-button');
    private readonly menu = element('guides-menu');
    private readonly rulers = element<HTMLInputElement>('guides-rulers-visible');
    private readonly guides = element<HTMLInputElement>('guides-visible');
    private readonly list = element('guide-positions');
    private readonly horizontal = element<HTMLButtonElement>('add-horizontal-guide');
    private readonly vertical = element<HTMLButtonElement>('add-vertical-guide');
    private readonly clear = element<HTMLButtonElement>('clear-guides');
    private readonly apply = element<HTMLButtonElement>('guides-apply');
    private readonly error = element('guides-error');
    private rows: Row[] = [];
    private state?: CanvasGuidesState;
    private draft?: CanvasGuidesState;
    private request?: number;

    constructor(private readonly send: SendEdit) {
        anchorPopover(this.button, this.menu);
        this.menu.addEventListener('beforetoggle', event => {
            if (event.newState === 'open' && this.state) this.open(this.state);
            else if (event.newState === 'closed') this.draft = undefined;
        });
        this.horizontal.onclick = () => this.add('Horizontal', 0).focus();
        this.vertical.onclick = () => this.add('Vertical', 0).focus();
        this.clear.onclick = () => {
            this.rows = [];
            this.list.replaceChildren();
            this.updateButtons();
            this.horizontal.focus();
        };
        element('guides-cancel').onclick = () => this.menu.hidePopover();
        element<HTMLFormElement>('guides-form').onsubmit = event => {
            event.preventDefault();
            if (!this.draft || this.apply.disabled) return;
            const lines = this.rows.map(({ id, axis, input }) => ({ id, axis, position: input.valueAsNumber }));
            this.request = this.send(this.draft, { action: 'apply', rulers: this.rulers.checked, guides: this.guides.checked, lines });
            this.updateButtons();
        };
    }

    update(state: CanvasGuidesState | undefined): void {
        this.state = state;
        this.button.disabled = !state;
        this.button.dataset.active = String(Boolean(state?.settings.rulers_visible));
        this.updateButtons();
    }

    result(request: number, error?: string): boolean {
        if (request !== this.request) return false;
        this.request = undefined;
        if (!error) this.menu.hidePopover();
        else this.report(error);
        this.updateButtons();
        return true;
    }

    private open(state: CanvasGuidesState): void {
        this.draft = state;
        this.request = undefined;
        const { scope, settings } = state;
        element('guides-scope').textContent = { document: 'Document', definition: 'Definition', rsx: 'RSX invocation' }[scope];
        this.rulers.checked = settings.rulers_visible;
        this.guides.checked = settings.guides_visible;
        this.rows = [];
        this.list.replaceChildren();
        for (const guide of settings.guides) this.add(guide.axis, guide.position, guide.id);
        this.report('');
        this.updateButtons();
    }

    private add(axis: GuideAxis, position: number, id?: number): HTMLInputElement {
        const row = create('div');
        row.className = 'guide-position-row';
        const glyph = axis === 'Horizontal' ? 'split-vertical' : 'split-horizontal';
        const coordinate = axis === 'Horizontal' ? 'Y' : 'X';
        const label = create('label');
        label.append(icon(glyph), coordinate);
        const input = create('input');
        input.type = 'number';
        input.value = String(position);
        input.step = 'any';
        input.min = String(-this.draft!.maxPosition);
        input.max = String(this.draft!.maxPosition);
        input.required = true;
        input.setAttribute('aria-label', `${axis} guide ${coordinate} position`);
        label.append(input);
        const entry = { id, axis, input };
        const remove = iconButton('close', `Remove ${axis.toLowerCase()} guide`, () => {
            const index = this.rows.indexOf(entry);
            this.rows.splice(index, 1);
            row.remove();
            this.updateButtons();
            (this.rows[index]?.input ?? this.rows.at(-1)?.input ?? this.horizontal).focus();
        });
        row.append(label, remove);
        this.rows.push(entry);
        this.list.append(row);
        this.updateButtons();
        return input;
    }

    private updateButtons(): void {
        const draft = this.draft;
        if (!draft) return;
        const state = this.state;
        const current = state?.frame.token === draft.frame.token && state?.frame.session === draft.frame.session && state.epoch === draft.epoch;
        const atLimit = this.rows.length >= draft.maxGuides;
        this.horizontal.disabled = this.vertical.disabled = atLimit;
        this.clear.disabled = !this.rows.length;
        this.apply.disabled = !current || this.request !== undefined;
        if (!current && this.request === undefined) this.report('The preview or guides changed. Reopen Canvas Guides to edit the current scope.');
    }

    private report(message: string): void {
        this.error.textContent = message;
        this.error.hidden = !message;
    }
}
