import type { FrameTarget } from '../src/protocol';
import type { TimelineProgram } from '../src/timelineProtocol';
import { anchorPopover, element, labelButton } from './dom';

type Clip = TimelineProgram & { clip: NonNullable<TimelineProgram['clip']> };

export class TimelineClipPlacement {
    private readonly button = element<HTMLButtonElement>('clip-place');
    private readonly popup = element('clip-placement');
    private readonly start = element<HTMLInputElement>('clip-start');
    private readonly gap = element<HTMLInputElement>('clip-gap');
    private frame?: FrameTarget;
    private selection: readonly Clip[] = [];
    private draft?: { frame: FrameTarget; ids: number[]; initial: number };

    constructor(apply: (frame: FrameTarget, ids: number[], start: number | undefined, gap: number) => void) {
        anchorPopover(this.button, this.popup);
        this.popup.addEventListener('beforetoggle', event => {
            if (event.newState !== 'open') return;
            const first = this.selection[0];
            if (!this.frame || !first || this.button.disabled) { event.preventDefault(); return; }
            this.draft = { frame: this.frame, ids: this.selection.map(program => program.id), initial: first.clip.start };
            this.start.value = String(first.clip.start);
            this.gap.value = '0';
            element('clip-gap-field').hidden = this.draft.ids.length === 1;
        });
        this.popup.addEventListener('toggle', event => {
            this.button.setAttribute('aria-expanded', String(event.newState === 'open'));
        });
        element<HTMLFormElement>('clip-placement-form').onsubmit = event => {
            event.preventDefault();
            const draft = this.draft;
            if (!draft || this.frame?.token !== draft.frame.token || this.frame.session !== draft.frame.session
                || this.selection.length !== draft.ids.length || this.selection.some((program, index) => program.id !== draft.ids[index])) return;
            const start = this.start.valueAsNumber;
            const gap = this.gap.valueAsNumber;
            if (!Number.isFinite(start) || !Number.isFinite(gap)) return;
            this.popup.hidePopover();
            apply(draft.frame, draft.ids, start === draft.initial ? undefined : start, gap);
        };
    }

    follow(frame: FrameTarget | undefined, selected: readonly Clip[]): void {
        this.frame = frame;
        this.selection = selected;
        this.button.disabled = !frame || !selected.length || selected.some(program => !program.clip.editable);
        labelButton(this.button, selected.length > 1 ? 'Sequence selected clips' : 'Set selected clip start');
        if (this.popup.matches(':popover-open') && (this.button.disabled || !this.draft || !frame
            || this.draft.frame.token !== frame.token || this.draft.frame.session !== frame.session)) this.popup.hidePopover();
    }
}
