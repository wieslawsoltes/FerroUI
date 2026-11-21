import type { FrameTarget } from '../src/protocol';
import type { AppliedKeyframe, KeyframeDraft, KeyframeLocation, TimelineKeyframe, TimelineKeyframeRequest, TimelineKeyframeResponse } from '../src/timelineProtocol';
import easingPresets from '../../../src/FerroUI.Designer.Animation/keyframe-easing-presets.json';
import { element } from './dom';
import { EasingHandles } from './easingHandles';

interface SelectedKeyframe {
    version: number;
    session: number;
    program: number;
    target: number;
    property: string;
    keyframe: TimelineKeyframe;
}

export class TimelineKeyframeEditor {
    private readonly section = element('timeline-keyframe-editor');
    private readonly name = element('keyframe-name');
    private readonly fields = element<HTMLFieldSetElement>('keyframe-fields');
    private readonly time = element<HTMLInputElement>('keyframe-time');
    private readonly value = element<HTMLInputElement>('keyframe-value');
    private readonly easing = element<HTMLInputElement>('keyframe-easing');
    private readonly preset = element<HTMLSelectElement>('keyframe-easing-preset');
    private readonly curve = new EasingHandles(easing => {
        this.easing.value = easing;
        this.syncPreset();
        this.inspect();
    });
    private readonly apply = element<HTMLButtonElement>('keyframe-apply');
    private readonly seekButton = element<HTMLButtonElement>('keyframe-seek');
    private readonly feedback = element('keyframe-feedback');
    private selected?: SelectedKeyframe;
    private frame?: FrameTarget;
    private pending?: TimelineKeyframeRequest;
    private validated?: KeyframeDraft;
    private applyFocus?: HTMLInputElement;
    private timer?: number;
    private request = 0;

    get applying(): boolean { return Boolean(this.pending?.apply); }

    constructor(private readonly send: (request: TimelineKeyframeRequest) => void,
        private readonly seek: ((frame: FrameTarget, program: number, time: number) => void) | undefined,
        private readonly applied: ((keyframe: AppliedKeyframe, property: string) => void) | undefined,
        private readonly restoreFocus: () => void) {
        this.preset.replaceChildren(...easingPresets.map(({ label, value }) => new Option(label, value)), new Option('Custom', ''));
        this.preset.addEventListener('input', () => {
            if (this.preset.value) this.easing.value = this.preset.value;
            else this.easing.focus();
        });
        this.easing.addEventListener('input', () => this.syncPreset());
        this.fields.addEventListener('input', () => this.inspect());
        element('timeline-keyframe-form').addEventListener('submit', event => {
            event.preventDefault();
            if (this.apply.disabled || !this.frame || !this.validated) return;
            this.applyFocus = [this.time, this.value, this.easing].find(field => field === document.activeElement) ?? this.time;
            this.submit(this.validated, this.frame, true);
            this.enable();
        });
        this.seekButton.onclick = () => {
            const key = this.selected;
            if (key && this.frame) this.seek?.(this.frame, key.program, key.keyframe.time);
        };
        element('keyframe-close').onclick = () => { this.close(); this.restoreFocus(); };
        this.section.addEventListener('keydown', event => {
            if (event.key === 'Escape') { event.stopPropagation(); this.close(); this.restoreFocus(); }
        });
    }

    open(frame: FrameTarget, version: number, program: number, target: number, property: string, keyframe: TimelineKeyframe,
        targetName: string, duration: number, focus: boolean): void {
        if (this.pending?.apply) return;
        const current = this.selected;
        if (current?.session === frame.session && current.version === version && current.program === program
            && current.target === target && current.property === property && current.keyframe.time === keyframe.time) {
            if (focus) this.time.focus();
            return;
        }
        this.clearRequest();
        this.selected = { version, session: frame.session, program, target, property, keyframe };
        this.frame = frame;
        this.name.textContent = this.name.title = `${property} · ${targetName}`;
        this.time.max = String(duration);
        this.fill(keyframe);
        this.section.hidden = false;
        this.inspect();
        if (focus) this.time.focus({ preventScroll: true });
    }

    follow(frame: FrameTarget | undefined, version: number | undefined, program: number | undefined): void {
        const key = this.selected;
        if (!key || this.pending?.apply) return;
        if (frame && (frame.session !== key.session || program !== key.program)) { this.close(); return; }
        const current = version === key.version ? frame : undefined;
        if (current && current.token === this.frame?.token && current.session === this.frame?.session) return;
        this.frame = current;
        if (current) this.inspect();
        else {
            this.clearRequest();
            this.showError(version !== key.version ? 'The source changed. Reopen the keyframe to edit the current version.' : undefined);
            this.enable();
        }
    }

    receive(response: TimelineKeyframeResponse): void {
        const request = this.pending;
        const key = this.selected;
        if (!request || !key || request.request !== response.request || request.frame.session !== response.frame.session
            || request.frame.token !== response.frame.token) return;
        this.pending = undefined;
        if (response.applied) {
            const { frame, version, program, target, keyframe } = response.applied;
            this.selected = { ...key, version, program, target, keyframe };
            this.frame = frame;
            this.fill(keyframe);
            this.applied?.(response.applied, key.property);
            this.inspect();
        } else if (response.applied === null) {
            this.close();
            this.restoreFocus();
        } else {
            this.showError(response.error);
            this.curve.show(request.draft.easing, response.preview?.samples);
            this.validated = response.preview ? request.draft : undefined;
            this.enable();
        }
        if (request.apply) {
            if (document.activeElement === document.body) this.applyFocus?.focus({ preventScroll: true });
            this.applyFocus = undefined;
        }
    }

    close(): void {
        this.curve.cancel();
        this.clearRequest();
        this.applyFocus = undefined;
        this.selected = this.frame = undefined;
        this.section.hidden = true;
    }

    keepVisible(visible: (key: KeyframeLocation) => boolean): void {
        const key = this.selected;
        if (key && !this.applying && !visible({ target: key.target, property: key.property, time: key.keyframe.time })) this.close();
    }

    applyCurveDrag(frame: FrameTarget, time: number, value: number | string): void {
        const key = this.selected;
        if (!key || this.applying || frame.session !== this.frame?.session || frame.token !== this.frame.token) return;
        this.clearRequest();
        this.time.value = String(time);
        this.value.value = String(value);
        this.easing.value = key.keyframe.easing_text;
        this.syncPreset();
        this.applyFocus = this.value;
        this.submit(this.draft(key, time), frame, true);
        this.enable();
    }

    private fill(keyframe: TimelineKeyframe): void {
        this.time.value = String(keyframe.time);
        this.value.type = keyframe.value_kind === 'Number' ? 'number' : 'text';
        this.value.step = 'any';
        this.value.value = keyframe.value_text;
        this.easing.value = keyframe.easing_text;
        this.syncPreset();
    }

    private syncPreset(): void {
        const value = this.easing.value.trim().replace(/\s*,\s*/g, ',') || 'LinearEasing';
        this.preset.value = easingPresets.some(preset => preset.value === value) ? value : '';
    }

    private inspect(): void {
        this.clearRequest();
        const { selected: key, frame } = this;
        const time = this.time.valueAsNumber;
        this.curve.show(this.easing.value);
        this.showError(undefined);
        this.enable();
        if (!key || !frame) return;
        if (!Number.isFinite(time) || time < 0 || time > Number(this.time.max)) {
            this.showError(`Enter a time from 0 to ${this.time.max} seconds.`);
            return;
        }
        const draft = this.draft(key, time);
        this.timer = window.setTimeout(() => this.submit(draft, frame, false), 140);
    }

    private draft(key: SelectedKeyframe, time: number): KeyframeDraft {
        return { program: key.program, target: key.target, property: key.property, previous: key.keyframe.time,
            time, value: this.value.value, easing: this.easing.value };
    }

    private submit(draft: KeyframeDraft, frame: FrameTarget, apply: boolean): void {
        this.pending = { type: 'keyframeDraft', request: ++this.request, frame, draft, apply };
        this.send(this.pending);
    }

    private clearRequest(): void {
        window.clearTimeout(this.timer);
        this.pending = this.validated = undefined;
    }

    private showError(message: string | undefined): void {
        this.feedback.hidden = !message;
        this.feedback.textContent = message ?? '';
    }

    private enable(): void {
        const disabled = !this.frame || Boolean(this.pending?.apply);
        this.fields.disabled = this.seekButton.disabled = disabled;
        if (disabled) this.curve.cancel();
        const original = this.selected?.keyframe;
        const edit = this.validated;
        this.apply.disabled = disabled || !edit || !original || (edit.time === original.time && edit.value === original.value_text
            && edit.easing === original.easing_text);
    }
}
