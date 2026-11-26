import type { TimelineProgram } from '../src/timelineProtocol';
import { element } from './dom';
import { TimeScrubber } from './timeScrubber';

export class TimelineNativePreview {
    private readonly section = element('timeline-native-source');
    private readonly note = element('timeline-preview-note');
    private readonly targets = element<HTMLButtonElement>('timeline-preview-targets');
    private readonly source = element<HTMLButtonElement>('timeline-source');
    private readonly scrubber: TimeScrubber;

    constructor(seek: (seconds: number) => void, chooseTargets: () => void, reveal: () => void) {
        this.scrubber = new TimeScrubber(element('timeline-native-seek'), seek);
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
        const slider = this.scrubber.input;
        slider.hidden = !native || missingTargets;
        slider.disabled = !enabled;
        if (preview?.native && preview.available) {
            const maximum = preview.total_duration ?? Math.max(preview.duration, seconds);
            this.scrubber.show(seconds, maximum);
            slider.setAttribute('aria-valuetext', `${seconds.toFixed(3)} seconds elapsed`);
        }
    }
}
