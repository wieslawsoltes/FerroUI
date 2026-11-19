import { snapEnabled, type CanvasGuidesState, type GuideEdit, type SnapPreferences } from '../src/canvasGuidesProtocol';
import { anchorPopover, element } from './dom';

const fields = {
    snap_to_grid: element<HTMLInputElement>('snap-to-grid'),
    snap_to_guides: element<HTMLInputElement>('snap-to-guides'),
    snap_to_objects: element<HTMLInputElement>('snap-to-objects'),
    grid_visible: element<HTMLInputElement>('show-snap-grid'),
    grid_spacing: element<HTMLInputElement>('snap-grid-spacing'),
    snap_tolerance: element<HTMLInputElement>('snap-tolerance'),
};
const keys = Object.keys(fields) as (keyof SnapPreferences)[];

export class SnappingEditor {
    private readonly button = element<HTMLButtonElement>('snapping-button');
    private readonly menu = element('snapping-menu');
    private readonly apply = element<HTMLButtonElement>('snapping-apply');
    private readonly error = element('snapping-error');
    private state?: CanvasGuidesState;
    private draft?: CanvasGuidesState;
    private request?: number;

    constructor(private readonly send: (state: CanvasGuidesState, edit: GuideEdit) => number) {
        anchorPopover(this.button, this.menu);
        this.menu.addEventListener('beforetoggle', event => {
            if (event.newState === 'open' && this.state) this.open(this.state);
            else if (event.newState === 'closed') this.draft = undefined;
        });
        element('snapping-cancel').onclick = () => this.menu.hidePopover();
        element<HTMLFormElement>('snapping-form').onsubmit = event => {
            event.preventDefault();
            if (!this.draft || this.apply.disabled) return;
            const options = Object.fromEntries(keys.map(key => {
                const input = fields[key];
                return [key, input.type === 'checkbox' ? input.checked : input.valueAsNumber];
            })) as unknown as SnapPreferences;
            this.request = this.send(this.draft, { action: 'configure_snap', options });
            this.updateDraft();
        };
    }

    update(state: CanvasGuidesState | undefined, available: boolean): void {
        this.state = state?.snapping && available ? state : undefined;
        this.button.hidden = !state?.snapping;
        this.button.disabled = !this.state;
        this.button.dataset.active = String(Boolean(state?.snapping && snapEnabled(state.settings)));
        this.updateDraft();
    }

    result(request: number, error?: string): boolean {
        if (request !== this.request) return false;
        this.request = undefined;
        if (error) this.report(error);
        else this.menu.hidePopover();
        this.updateDraft();
        return true;
    }

    private open(state: CanvasGuidesState): void {
        this.draft = state;
        this.request = undefined;
        for (const key of keys) {
            const field = fields[key];
            const value = state.settings[key];
            if (typeof value === 'boolean') field.checked = value;
            else field.value = String(value);
        }
        this.report('');
        this.updateDraft();
    }

    private updateDraft(): void {
        const draft = this.draft;
        if (!draft) return;
        const current = this.state?.frame.token === draft.frame.token && this.state.frame.session === draft.frame.session
            && this.state.epoch === draft.epoch;
        this.apply.disabled = !current || this.request !== undefined;
        if (!current && this.request === undefined) this.report('The preview or snapping settings changed. Reopen Snapping to edit this scope.');
    }

    private report(text: string): void {
        this.error.textContent = text;
        this.error.hidden = !text;
    }
}
