import type { TimelineProgram } from '../src/timelineProtocol';
import { element } from './dom';

export class TimelineNativePreview {
    private readonly section = element('timeline-native-source');
    private readonly note = element('timeline-preview-note');
    private readonly targets = element<HTMLButtonElement>('timeline-preview-targets');
    private readonly source = element<HTMLButtonElement>('timeline-source');
    private readonly slider = element<HTMLInputElement>('timeline-native-seek');

    constructor(seek: (seconds: number) => void, chooseTargets: () => void, reveal: () => void) {
        this.slider.addEventListener('input', () => seek(this.slider.valueAsNumber));
        this.targets.addEventListener('click', chooseTargets);
        this.source.addEventListener('click', reveal);
    }

    show(program: TimelineProgram | undefined, names: Record<number, string>, seconds: number, enabled: boolean, pending: boolean): void {
        const preview = program?.preview;
        const native = Boolean(preview?.native);
        const missingTargets = preview?.available === false;
        const unsupported = program && !program.timeline && !preview;
        const showSource = Boolean(program && (native || unsupported));
        const namesOfTargets = preview?.explicit_targets
            ? program?.targets.map(id => names[id]).filter(Boolean).join(', ') : undefined;
        const message = missingTargets ? 'Choose controls to preview this animation.'
            : unsupported ? 'Open the source to inspect this animation payload.'
                : namesOfTargets || (native ? 'Native animation preview' : '');
        if (this.note.textContent !== message) this.note.textContent = message;
        this.note.hidden = !message;
        this.note.title = program?.diagnostics?.join('\n') ?? '';
        this.section.hidden = !message && !native;
        this.targets.hidden = !program || (program.style != null && !missingTargets && !preview?.explicit_targets);
        this.source.hidden = !showSource;
        this.source.disabled = this.targets.disabled = !enabled || pending;
        this.slider.hidden = !native || missingTargets;
        this.slider.disabled = !enabled;
        if (preview?.native && preview.available) {
            const maximum = preview.total_duration ?? Math.max(preview.duration, seconds);
            this.slider.max = String(maximum);
            this.slider.value = String(seconds);
            this.slider.setAttribute('aria-valuetext', `${seconds.toFixed(3)} seconds elapsed`);
        }
    }
}
