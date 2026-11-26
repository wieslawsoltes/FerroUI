import type { Snapshot } from '../src/protocol';
import { element } from './dom';

export class Counterpart {
    private readonly input = element<HTMLTextAreaElement>('projection');
    private readonly apply = element<HTMLButtonElement>('apply-draft');
    private readonly reload = element<HTMLButtonElement>('reload-draft');
    private readonly message = element('counterpart-message');
    private value?: Snapshot['counterpart'];
    private sourceKey?: string;
    private baseSource?: string;
    private conflict = false;
    private enabled = false;

    constructor(apply: (source: string) => void) {
        this.input.addEventListener('input', () => this.update());
        this.apply.addEventListener('click', () => apply(this.input.value));
        this.reload.addEventListener('click', () => { this.load(); this.update(); });
    }

    show(value: Snapshot['counterpart'], key: string): void {
        const isDirty = this.dirty;
        this.value = value;
        const changed = this.sourceKey !== key;
        this.sourceKey = key;
        if (!isDirty) this.load();
        else if (changed) this.conflict = true;
        this.update();
    }

    accepted(): void { this.baseSource = this.input.value; }

    setEnabled(enabled: boolean): void {
        this.enabled = enabled;
        this.update();
    }

    private get dirty(): boolean { return this.baseSource !== undefined && this.baseSource !== this.input.value; }

    private load(): void {
        this.baseSource = this.value?.source ?? '';
        this.input.value = this.baseSource;
        this.conflict = false;
    }

    private update(): void {
        const editable = Boolean(this.value?.editable && this.enabled && !this.conflict);
        this.input.readOnly = !editable;
        this.apply.disabled = !editable || !this.dirty;
        this.reload.hidden = !this.conflict;
        this.input.hidden = this.value?.source === null && !this.conflict;
        this.message.textContent = this.conflict
            ? 'The source changed while this draft was open. Reload the projection to apply further edits.'
            : this.value?.reason ?? 'Apply the projected source to update the open editor buffer.';
    }
}
