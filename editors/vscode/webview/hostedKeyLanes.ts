import type { HostedAnimationFrame, HostedAnimationProgram, HostedAnimationTarget } from '../src/hostedAnimationProtocol';
import type { FromHosted, ToHosted } from '../src/hostedProtocol';
import type { FrameTarget } from '../src/protocol';
import type { KeyframeLocation, TimelineKeyAction, TimelineKeyframe, TimelineTrack } from '../src/timelineProtocol';
import { create, element, labelButton } from './dom';
import { dragKeyframe, renderKeyframe, type RenderedKeyframe } from './timelineKeyDrag';
import { bindTimelineMarquee } from './timelineMarquee';
import { keyframeId, TimelineSelection } from './timelineSelection';
import { clamp, fitTimeline, timelinePercent } from './timelineViewport';

interface GroupSelection {
    program: HostedAnimationProgram;
    scope?: string;
    keys: KeyframeLocation[];
    version: number;
}

export class HostedKeyLanes {
    private readonly toggle = element<HTMLButtonElement>('hosted-key-lanes-toggle');
    private readonly section = element('hosted-key-lanes');
    private readonly grid = element('hosted-key-grid');
    private readonly tracks = element('hosted-key-tracks');
    private readonly axis = element('hosted-key-axis');
    private readonly time = element<HTMLInputElement>('hosted-key-time');
    private readonly count = element('hosted-key-selection');
    private readonly actions = element<HTMLButtonElement>('hosted-key-actions');
    private readonly error = element('hosted-key-error');
    private readonly selection = new TimelineSelection();
    private readonly number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 6 });
    private keys: RenderedKeyframe[] = [];
    private viewport = fitTimeline(0);
    private program?: HostedAnimationProgram;
    private programKey?: string;
    private scope?: string;
    private content = '';
    private shown = false;
    private accepted?: FrameTarget;
    private request = 0;
    private pending?: { request: number; program: HostedAnimationProgram; scope?: string };
    private replacement?: GroupSelection;
    private pivot = 0;
    private restoreFocus = false;

    constructor(private readonly send: (message: FromHosted) => void, private readonly changed: () => void,
        private readonly openKey: (track: TimelineTrack, key: TimelineKeyframe) => void) {
        this.toggle.onclick = () => { this.shown = !this.shown; this.changed(); };
        this.actions.onclick = () => this.edit();
        this.time.oninput = () => {
            const value = this.time.valueAsNumber;
            if (Number.isFinite(value) && value >= 0 && value <= this.viewport.duration) this.setPivot(value);
        };
        this.time.onchange = () => { this.time.value = String(this.pivot); };
        this.axis.onclick = event => {
            const bounds = this.axis.getBoundingClientRect();
            if (!this.frame || bounds.width <= 0) return;
            const elapsed = clamp((event.clientX - bounds.left) / bounds.width, 0, 1) * this.viewport.span;
            this.setPivot(Math.min(this.viewport.duration, Math.round(elapsed * 1000) / 1000), true);
        };
        bindTimelineMarquee(this.grid, this.tracks, () => this.keys, () => this.frame, () => this.viewport,
            (items, additive) => {
                this.selection.replace([...(additive ? this.selection.keys : []), ...items.map(item => item.key)]);
                this.paintSelection();
            });
        for (const control of [this.actions, this.grid]) {
            control.addEventListener('keydown', event => this.shortcut(event));
        }
    }

    get editing(): boolean { return this.pending !== undefined; }
    private get frame() { return this.editing ? undefined : this.accepted; }

    follow(program: HostedAnimationProgram | undefined, targets: readonly HostedAnimationTarget[], scope: string | undefined,
        frame: HostedAnimationFrame | undefined, available: boolean, version: number): void {
        const tracks = program?.authoring?.tracks ?? [];
        const visible = available && tracks.length > 0 && this.shown;
        this.toggle.hidden = !available || !tracks.length;
        this.toggle.setAttribute('aria-pressed', String(visible));
        this.toggle.setAttribute('aria-expanded', String(visible));
        this.section.hidden = !visible;
        this.program = program;
        this.scope = scope;
        const identity = program && JSON.stringify([scope, program.sourceKey ?? program.markerKey ?? program.id]);
        const changedProgram = identity !== this.programKey;
        if (changedProgram) {
            this.programKey = identity;
            this.selection.replace([]);
            this.setPivot(0, true);
            this.showError();
        }
        this.accepted = visible && frame ? { session: frame.revision, token: frame.sequence } : undefined;
        if (visible) {
            const content = JSON.stringify([identity, program?.duration, tracks]);
            if (this.content !== content) {
                this.content = content;
                this.render(tracks, targets, program?.duration ?? 0);
            }
        }
        if (frame && this.replacement && version >= this.replacement.version) {
            const { keys, program: before, scope: originalScope, version: edited } = this.replacement;
            this.replacement = undefined;
            if (version === edited && scope === originalScope && program
                && (before.sourceKey ? before.sourceKey === program.sourceKey : before.selector === program.selector
                    && (before.name === program.name || before.id === program.id))) {
                this.selection.replace(keys);
                this.selection.reconcile(tracks);
                if (this.restoreFocus) this.focusKey();
            }
            this.restoreFocus = false;
        }
        this.actions.disabled = this.time.disabled = !this.frame;
        this.paintSelection();
    }

    receive(reply: Extract<ToHosted, { type: 'keyframeGroupReply' }>): void {
        const pending = this.pending;
        if (reply.request !== pending?.request) return;
        this.pending = undefined;
        if (reply.keys && reply.version !== undefined) {
            this.replacement = { program: pending.program, scope: pending.scope, keys: reply.keys, version: reply.version };
        } else this.restoreFocus = false;
        this.showError(reply.error);
        this.changed();
    }

    private render(tracks: TimelineTrack[], targets: readonly HostedAnimationTarget[], duration: number): void {
        const hadFocus = this.keys.some(key => key.button === document.activeElement);
        this.keys = [];
        this.viewport = fitTimeline(duration);
        this.time.max = String(duration);
        this.setPivot(Math.min(this.pivot, duration), true);
        this.axis.replaceChildren(...[0, .25, .5, .75, 1].map(fraction => {
            const label = create('span', this.number.format(duration * fraction));
            label.style.left = `${fraction * 100}%`;
            return label;
        }));
        this.selection.reconcile(tracks);
        this.tracks.replaceChildren(...tracks.map(track => {
            const target = targets.find(target => target.id === track.target)?.label;
            const description = target ? `${track.property} · ${target}` : track.property;
            const row = create('li');
            row.className = 'hosted-key-row';
            const name = create('span', track.property);
            name.className = 'hosted-key-name';
            name.title = description;
            const lane = create('div');
            lane.className = 'timeline-lane';
            for (const key of track.keyframes) {
                const location = { target: track.target, property: track.property, time: key.time };
                const rendered = renderKeyframe(location, `${description}, ${this.number.format(key.time)} s · ${key.value_text} · ${key.easing_text}`);
                const { button } = rendered;
                button.style.left = `${timelinePercent(this.viewport, key.time)}%`;
                button.toggleAttribute('data-eased', !['', 'linear', 'lineareasing', '0,0,1,1'].includes(key.easing_text.toLowerCase().replace(/\s/g, '')));
                this.keys.push(rendered);
                dragKeyframe(rendered, lane, () => this.viewport,
                    () => this.selection.has(location) ? this.keys.filter(item => this.selection.has(item.key)) : [rendered],
                    () => this.frame, time => Math.round(time * 1000) / 1000,
                    (keys, offset) => this.edit({ action: 'move_keyframes', offset }, keys));
                button.onclick = event => {
                    if (!this.frame) return;
                    this.selection.pick(location, this.keys.map(item => item.key), event.shiftKey, event.metaKey || event.ctrlKey);
                    this.paintSelection();
                    if (event.detail === 0 && !event.shiftKey && !event.metaKey && !event.ctrlKey) this.openKey(track, key);
                };
                button.ondblclick = () => { if (this.frame) this.openKey(track, key); };
                button.onkeydown = event => {
                    if (!this.frame || event.key !== ' ') return;
                    event.preventDefault();
                    event.stopPropagation();
                    this.selection.pick(location, this.keys.map(item => item.key), event.shiftKey, true);
                    this.paintSelection();
                };
                lane.append(button);
            }
            row.append(name, lane);
            return row;
        }));
        if (hadFocus && !this.editing && !this.replacement) this.focusKey();
    }

    private focusKey(): void {
        this.restoreFocus = false;
        const first = this.selection.keys[0];
        const selected = first && this.keys.find(item => keyframeId(item.key) === keyframeId(first));
        (selected?.button ?? this.toggle).focus({ preventScroll: true });
    }

    private setPivot(time: number, updateInput = false): void {
        this.pivot = time;
        if (updateInput) this.time.value = String(time);
        this.grid.style.setProperty('--hosted-key-pivot', `${timelinePercent(this.viewport, time)}%`);
    }

    private edit(edit?: TimelineKeyAction, keys = this.selection.keys): void {
        const frame = this.frame;
        if (!frame || !this.program) return;
        this.restoreFocus = this.keys.some(key => key.button === document.activeElement);
        this.showError();
        const request = ++this.request;
        this.pending = { request, program: this.program, scope: this.scope };
        this.send({
            type: 'keyframeGroup', frame: { revision: frame.session, sequence: frame.token },
            request, action: { program: this.program.id, keys, pivot: this.pivot, edit },
        });
        this.changed();
    }

    private shortcut(event: KeyboardEvent): void {
        if (!this.frame) return;
        const key = event.key.toLowerCase();
        const modifier = event.metaKey || event.ctrlKey;
        const focused = this.keys.find(item => item.button === event.target)?.key;
        const keys = this.selection.size || !focused ? this.selection.keys : [focused];
        const step = Math.min(.1, this.viewport.duration / 20);
        let edit: TimelineKeyAction | undefined;
        if (modifier && key === 'a') this.selection.replace(this.keys.map(item => item.key));
        else if (key === 'escape') this.selection.replace([]);
        else if (modifier && key === 'v') edit = { action: 'paste_keyframes', reverse: event.shiftKey };
        else if (keys.length && modifier && key === 'c') edit = { action: 'copy_keyframes' };
        else if (keys.length && modifier && key === 'd') edit = { action: 'duplicate_keyframes', offset: step };
        else if (keys.length && (key === 'delete' || key === 'backspace')) edit = { action: 'remove_keyframes' };
        else if (keys.length && !modifier && (key === 'arrowleft' || key === 'arrowright')) {
            edit = { action: 'move_keyframes', offset: key === 'arrowleft' ? -step : step };
        } else return;
        event.preventDefault();
        event.stopPropagation();
        if (edit) this.edit(edit, keys);
        else this.paintSelection();
    }

    private paintSelection(): void {
        for (const { key, button } of this.keys) {
            button.setAttribute('aria-pressed', String(this.selection.has(key)));
            button.setAttribute('aria-disabled', String(!this.frame));
        }
        const count = this.selection.size ? `${this.selection.size} selected` : '';
        if (this.count.textContent !== count) this.count.textContent = count;
        labelButton(this.actions, count ? `Keyframe actions (${count})` : 'Keyframe actions');
    }

    private showError(error?: string): void {
        this.error.textContent = error ?? '';
        this.error.hidden = !error;
    }
}
