import type { FrameTarget } from '../src/protocol';
import type { FromTimeline, TimelineControl, TimelineProgram, TimelineState, ToTimeline } from '../src/timelineProtocol';
import { element, labelButton } from './dom';
import { dragKeyframe } from './timelineKeyDrag';

interface ProgramIdentity { id: number; key?: string }
declare function acquireVsCodeApi(): {
    postMessage(message: FromTimeline): void;
    getState(): ProgramIdentity | number | undefined;
    setState(program: ProgramIdentity): void;
};
const vscode = acquireVsCodeApi();
const status = element('timeline-status');
const toolbar = element('timeline-toolbar');
const container = element('timeline-program');
const nativeSource = element('timeline-native-source');
const programs = element<HTMLSelectElement>('timeline-programs');
const play = element<HTMLButtonElement>('timeline-play');
const record = element<HTMLButtonElement>('timeline-record');
const actions = element<HTMLButtonElement>('timeline-actions');
const slider = element<HTMLInputElement>('timeline-seek');
const time = element<HTMLOutputElement>('timeline-time');
const ruler = element('timeline-ruler');
const tracks = element<HTMLOListElement>('timeline-tracks');
const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
let model: TimelineState = { programs: [], names: {}, writable: false };
const saved = vscode.getState();
let remembered = typeof saved === 'number' ? { id: saved } : saved;
let program: number | undefined;
let queued: number | undefined;
let submitted: number | undefined;
let pending = false;
let lastContent = '';
let lastPrograms = '';

programs.addEventListener('change', () => {
    const selected = model.programs.find(item => item.id === Number(programs.value));
    if (selected) remember(selected);
    queued = undefined;
    showTracks();
});
play.addEventListener('click', () => {
    const selected = current();
    if (!model.frame || !selected) return;
    const playing = model.position?.program === selected.id && model.position.playing;
    send(model.frame, { action: playing ? 'pause' : 'play', program: selected.id });
});
record.addEventListener('click', () => {
    const selected = current();
    if (model.frame && model.writable && selected?.timeline && !pending) {
        vscode.postMessage({ type: 'recordKeyframe', frame: model.frame, program: selected.id, time: slider.valueAsNumber });
    }
});
actions.addEventListener('click', () => {
    if (model.frame && !pending) vscode.postMessage({ type: 'programActions', frame: model.frame, program });
});
slider.addEventListener('input', () => {
    if (!model.frame || !current()?.timeline) return;
    queued = slider.valueAsNumber;
    showPosition();
    seek();
});
window.addEventListener('message', event => {
    const message = event.data as ToTimeline | { type: 'state'; state: TimelineState };
    if (message.type === 'state') {
        if (model.frame?.session !== message.state.frame?.session) queued = submitted = undefined;
        model = message.state;
        pending = Boolean(model.position?.busy);
        if (!pending) submitted = undefined;
        showStatus(model.message);
        showTracks();
        seek();
    } else if (message.type === 'position') {
        model.position = message.position;
        model.frame = message.frame;
        pending = Boolean(message.position?.busy);
        if (!pending) submitted = undefined;
        showPosition();
        seek();
    } else if (message.type === 'error') {
        queued = submitted = undefined;
        pending = false;
        showStatus(message.message);
        showPosition();
    }
});
vscode.postMessage({ type: 'ready' });

function current(): TimelineProgram | undefined { return model.programs.find(item => item.id === program); }

function showTracks(): void {
    const available = model.programs;
    const selected = available.find(item => remembered?.key ? item.source_key === remembered.key : item.id === remembered?.id) ?? available[0];
    program = selected?.id;
    if (selected && model.frame) remember(selected);
    const timeline = selected?.timeline;
    toolbar.hidden = !model.frame && !selected;
    nativeSource.hidden = !selected || Boolean(timeline);
    container.hidden = !timeline;
    const choices = JSON.stringify(available.map(({ id, name }) => [id, name]));
    if (lastPrograms !== choices) {
        lastPrograms = choices;
        programs.replaceChildren(...(available.length
            ? available.map(item => new Option(item.name, String(item.id)))
            : [new Option('No animations', '')]));
    }
    programs.value = program === undefined ? '' : String(program);
    if (!selected || !timeline) { showPosition(); return; }
    const content = JSON.stringify([selected, model.names, model.writable]);
    if (lastContent !== content) {
        lastContent = content;
        slider.max = String(timeline.duration);
        ruler.replaceChildren(...[0, .25, .5, .75, 1].map(proportion => {
            const mark = document.createElement('span');
            mark.textContent = number.format(timeline.duration * proportion);
            mark.style.left = `${proportion * 100}%`;
            return mark;
        }));
        tracks.replaceChildren(...timeline.tracks.map((track, index) => {
            const row = document.createElement('li');
            row.className = 'timeline-track';
            const name = document.createElement('span');
            name.className = 'timeline-track-name';
            name.textContent = track.property;
            const target = model.names[track.target];
            name.title = target ? `${target} · ${track.property}` : track.property;
            const lane = document.createElement('div');
            lane.className = 'timeline-lane';
            for (const key of track.keyframes) {
                const button = document.createElement('button');
                button.type = 'button';
                button.className = 'timeline-key';
                button.style.left = `${percent(key.time, timeline.duration)}%`;
                button.title = `${name.title}, ${number.format(key.time)}s · ${key.value_text} · ${key.easing_text}`;
                button.setAttribute('aria-label', button.title);
                if (model.writable && timeline.duration > 0) dragKeyframe(button, lane, key.time, timeline.duration,
                    () => pending ? undefined : model.frame, (to, frame) => {
                        vscode.postMessage({ type: 'moveKeyframe', frame, program: selected.id, track: index, time: key.time, to });
                    });
                button.addEventListener('click', () => {
                    if (!model.frame || pending) return;
                    if (model.writable) vscode.postMessage({ type: 'keyframe', frame: model.frame, program: selected.id, track: index, time: key.time });
                    else send(model.frame, { action: 'seek', program: selected.id, time: key.time });
                });
                lane.append(button);
            }
            row.append(name, lane);
            return row;
        }));
    }
    showPosition();
}

function remember(program: TimelineProgram): void {
    const key = program.source_key ?? undefined;
    if (remembered?.id === program.id && remembered.key === key) return;
    remembered = { id: program.id, key };
    vscode.setState(remembered);
}

function showPosition(): void {
    const selected = current();
    const timeline = selected?.timeline;
    const position = model.position?.program === selected?.id ? model.position : undefined;
    const playing = Boolean(position?.playing);
    play.hidden = time.hidden = !timeline;
    record.hidden = !timeline || !model.writable;
    record.disabled = !model.frame || pending || playing;
    actions.disabled = !model.frame || pending || playing || (!selected && !model.writable);
    programs.disabled = pending || !model.frame || !selected;
    if (!timeline) return;
    const seconds = queued ?? submitted ?? position?.time ?? 0;
    slider.value = String(seconds);
    slider.setAttribute('aria-valuetext', `${number.format(seconds)} of ${number.format(timeline.duration)} seconds`);
    tracks.style.setProperty('--timeline-position', `${percent(seconds, timeline.duration)}%`);
    time.value = `${number.format(seconds)} / ${number.format(timeline.duration)} s`;
    labelButton(play, playing ? 'Pause animation' : 'Play animation', playing ? 'debug-pause' : 'play');
    play.disabled = !model.frame || (!playing && (pending || timeline.duration <= 0));
    slider.disabled = !model.frame || playing || timeline.duration <= 0;
    for (const button of tracks.querySelectorAll('button')) button.disabled = pending || !model.frame;
}

function seek(): void {
    if (!model.frame || program === undefined || pending || queued === undefined) return;
    const time = queued;
    queued = undefined;
    send(model.frame, { action: 'seek', program, time });
}

function send(frame: FrameTarget, control: TimelineControl): void {
    pending = true;
    if (control.action === 'seek') submitted = control.time;
    vscode.postMessage({ type: 'control', frame, control });
    showPosition();
}

function showStatus(message: string | undefined): void {
    status.textContent = message ?? '';
    status.hidden = !message;
}

function percent(time: number, duration: number): number { return duration > 0 ? time * 100 / duration : 0; }
