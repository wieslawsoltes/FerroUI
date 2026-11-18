import type { FrameTarget } from '../src/protocol';
import type { FromTimeline, KeyframeGroupEdit, KeyframeLocation, TimelineControl, TimelineProgram, TimelineState, ToTimeline } from '../src/timelineProtocol';
import { element, labelButton } from './dom';
import { dragKeyframe, type RenderedKeyframe } from './timelineKeyDrag';
import { TimelineSelection } from './timelineSelection';

interface ProgramIdentity { id: number; key?: string }
interface SavedTimeline extends ProgramIdentity { frameRate?: number }
declare function acquireVsCodeApi(): {
    postMessage(message: FromTimeline): void;
    getState(): SavedTimeline | number | undefined;
    setState(program: SavedTimeline): void;
};
const vscode = acquireVsCodeApi();
const status = element('timeline-status');
const toolbar = element('timeline-toolbar');
const container = element('timeline-program');
const nativeSource = element('timeline-native-source');
const programs = element<HTMLSelectElement>('timeline-programs');
const snap = element<HTMLSelectElement>('timeline-snap');
const transport = element('timeline-transport');
const play = element<HTMLButtonElement>('timeline-play');
const stop = element<HTMLButtonElement>('timeline-stop');
const record = element<HTMLButtonElement>('timeline-record');
const actions = element<HTMLButtonElement>('timeline-actions');
const keyActions = element<HTMLButtonElement>('timeline-key-actions');
const slider = element<HTMLInputElement>('timeline-seek');
const time = element<HTMLOutputElement>('timeline-time');
const ruler = element('timeline-ruler');
const tracks = element<HTMLOListElement>('timeline-tracks');
const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
let model: TimelineState = { programs: [], names: {}, writable: false };
const saved = vscode.getState();
let remembered = typeof saved === 'number' ? { id: saved } : saved;
let frameRate = [24, 30, 60].find(rate => typeof saved === 'object' && rate === saved.frameRate) ?? 0;
snap.value = String(frameRate);
let program: number | undefined;
let queued: number | undefined;
let submitted: number | undefined;
let pending = false;
let lastContent = '';
let lastPrograms = '';
const selection = new TimelineSelection();
let renderedKeys: RenderedKeyframe[] = [];
let owner: { session: number; program: ProgramIdentity } | undefined;
let keyEdit: { before: FrameTarget; accepted?: KeyframeLocation[] } | undefined;

programs.addEventListener('change', () => {
    const selected = model.programs.find(item => item.id === Number(programs.value));
    if (selected) remember(selected);
    selection.replace([]);
    queued = undefined;
    showTracks();
});
snap.addEventListener('change', () => {
    frameRate = Number(snap.value);
    if (remembered) vscode.setState({ ...remembered, frameRate });
});
play.addEventListener('click', () => {
    const selected = current();
    if (!model.frame || !selected) return;
    const playing = model.position?.program === selected.id && model.position.playing;
    send(model.frame, { action: playing ? 'pause' : 'play', program: selected.id });
});
stop.addEventListener('click', () => {
    const selected = current();
    if (model.frame && selected) send(model.frame, { action: 'stop', program: selected.id });
});
record.addEventListener('click', () => {
    const selected = current();
    if (model.frame && model.writable && selected?.timeline && !pending && !keyEdit) {
        vscode.postMessage({ type: 'recordKeyframe', frame: model.frame, program: selected.id, time: slider.valueAsNumber });
    }
});
actions.addEventListener('click', () => {
    if (model.frame && !pending && !keyEdit) vscode.postMessage({ type: 'programActions', frame: model.frame, program });
});
keyActions.addEventListener('click', () => editKeys(selection.keys, model.frame));
slider.addEventListener('input', () => {
    if (!model.frame || !current()?.timeline) return;
    queued = slider.valueAsNumber;
    showPosition();
    seek();
});
container.addEventListener('keydown', event => {
    const frame = model.frame;
    if (!frame || program === undefined || pending || keyEdit || (event.key !== '[' && event.key !== ']')) return;
    const previous = event.key === '[';
    const from = slider.valueAsNumber;
    let destination = previous ? -Infinity : Infinity;
    for (const { key: { time } } of renderedKeys) {
        if (previous && time < from) destination = Math.max(destination, time);
        else if (!previous && time > from) destination = Math.min(destination, time);
    }
    event.preventDefault();
    event.stopPropagation();
    if (Number.isFinite(destination)) send(frame, { action: 'seek', program, time: destination });
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
    } else if (message.type === 'keyframesEdited') {
        if (keyEdit?.before.session !== message.before.session || keyEdit.before.token !== message.before.token) return;
        if (message.keys) keyEdit.accepted = message.keys;
        else keyEdit = undefined;
        showTracks();
    } else if (message.type === 'error') {
        queued = submitted = keyEdit = undefined;
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
    programs.title = selected?.name ?? 'Animation program';
    if (!selected || !timeline) { showKeySelection(); showPosition(); return; }
    const frame = model.frame;
    if (frame) {
        const sameProgram = owner?.session === frame.session
            && (owner.program.key ? owner.program.key === selected.source_key : owner.program.id === selected.id);
        if (!sameProgram) selection.replace([]);
        if (keyEdit?.before.session !== frame.session) keyEdit = undefined;
        if (keyEdit?.accepted && keyEdit.before.token !== frame.token) {
            selection.replace(keyEdit.accepted);
            keyEdit = undefined;
        }
        owner = { session: frame.session, program: { id: selected.id, key: selected.source_key ?? undefined } };
        selection.reconcile(timeline.tracks);
    }
    const content = JSON.stringify([selected, model.names, model.writable]);
    if (lastContent !== content) {
        lastContent = content;
        const restoreFocus = renderedKeys.some(item => item.button === document.activeElement);
        renderedKeys = [];
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
                const location = { target: track.target, property: track.property, time: key.time };
                const rendered = { button, key: location };
                renderedKeys.push(rendered);
                if (model.writable && timeline.duration > 0) dragKeyframe(rendered, lane, timeline.duration,
                    () => selection.has(location) ? renderedKeys.filter(item => selection.has(item.key)) : [rendered],
                    () => pending || keyEdit ? undefined : model.frame,
                    time => Math.round(time * (frameRate || 1000)) / (frameRate || 1000),
                    (keys, offset, frame) => editKeys(keys, frame, { action: 'move_keyframes', offset }));
                button.addEventListener('click', event => {
                    if (!model.frame || pending || keyEdit) return;
                    const additive = event.metaKey || event.ctrlKey;
                    selection.pick(location, event.shiftKey ? renderedKeys.map(item => item.key) : [], event.shiftKey, additive);
                    showKeySelection();
                    if (event.shiftKey || additive) return;
                    if (model.writable) vscode.postMessage({ type: 'keyframe', frame: model.frame, program: selected.id, track: index, time: key.time });
                    else send(model.frame, { action: 'seek', program: selected.id, time: key.time });
                });
                button.addEventListener('keydown', event => keyShortcuts(event, location, timeline.duration));
                lane.append(button);
            }
            row.append(name, lane);
            return row;
        }));
        if (restoreFocus) {
            const last = renderedKeys.findLast(item => selection.has(item.key));
            (last?.button ?? slider).focus({ preventScroll: true });
        }
    }
    showKeySelection();
    showPosition();
}

function remember(program: TimelineProgram): void {
    const key = program.source_key ?? undefined;
    if (remembered?.id === program.id && remembered.key === key) return;
    remembered = { id: program.id, key };
    vscode.setState({ ...remembered, frameRate });
}

function showPosition(): void {
    const selected = current();
    const timeline = selected?.timeline;
    const position = model.position?.program === selected?.id ? model.position : undefined;
    const playing = Boolean(position?.playing);
    transport.hidden = time.hidden = !timeline;
    snap.hidden = !timeline || !model.writable;
    snap.disabled = !model.frame || pending || playing || Boolean(keyEdit);
    record.hidden = !timeline || !model.writable;
    record.disabled = !model.frame || pending || playing || Boolean(keyEdit);
    actions.disabled = !model.frame || pending || playing || Boolean(keyEdit) || (!selected && !model.writable);
    programs.disabled = pending || Boolean(keyEdit) || !model.frame || !selected;
    keyActions.disabled = !model.frame || pending || playing || Boolean(keyEdit);
    if (!timeline) return;
    const seconds = queued ?? submitted ?? position?.time ?? 0;
    slider.value = String(seconds);
    slider.setAttribute('aria-valuetext', `${number.format(seconds)} of ${number.format(timeline.duration)} seconds`);
    tracks.style.setProperty('--timeline-position', `${percent(seconds, timeline.duration)}%`);
    time.value = `${number.format(seconds)} / ${number.format(timeline.duration)} s`;
    if (position?.elapsed !== undefined) {
        const clock = timeline.total_duration === null ? `${number.format(position.elapsed)} s elapsed`
            : `${number.format(position.elapsed)} / ${number.format(timeline.total_duration)} s elapsed`;
        time.title = clock;
    } else {
        time.removeAttribute('title');
    }
    labelButton(play, playing ? 'Pause animation' : 'Play animation', playing ? 'debug-pause' : 'play');
    play.disabled = !model.frame || (!playing && (pending || Boolean(keyEdit) || timeline.duration <= 0));
    stop.disabled = !model.frame || !position || Boolean(keyEdit) || (!playing && pending);
    slider.disabled = !model.frame || playing || Boolean(keyEdit) || timeline.duration <= 0;
    for (const { button } of renderedKeys) button.disabled = pending || Boolean(keyEdit) || !model.frame;
}

function showKeySelection(): void {
    keyActions.hidden = !current()?.timeline || !model.writable || selection.size === 0;
    for (const { button, key } of renderedKeys) button.setAttribute('aria-pressed', String(selection.has(key)));
}

function editKeys(keys: KeyframeLocation[], frame?: FrameTarget, edit?: KeyframeGroupEdit): void {
    if (!frame || program === undefined || pending || keyEdit || !model.writable || !keys.length) return;
    keyEdit = { before: frame };
    vscode.postMessage({ type: 'keyframeGroup', frame, program, keys, edit });
    showPosition();
}

function keyShortcuts(event: KeyboardEvent, location: KeyframeLocation, duration: number): void {
    const found = model.frame;
    if (!found || pending || keyEdit) return;
    const modifier = event.metaKey || event.ctrlKey;
    const command = event.key.toLowerCase();
    if (command === ' ' || command === 'escape' || (modifier && command === 'a')) {
        event.preventDefault();
        event.stopPropagation();
        if (command === 'escape') selection.replace([]);
        else if (command === 'a') selection.replace(renderedKeys.map(item => item.key));
        else selection.pick(location, renderedKeys.map(item => item.key), event.shiftKey, true);
        showKeySelection();
        return;
    }
    if (!model.writable) return;
    const keys = selection.size ? selection.keys : [location];
    const step = frameRate ? 1 / frameRate : Math.min(0.1, duration / 20);
    let edit: KeyframeGroupEdit | undefined;
    if (command === 'delete' || command === 'backspace') edit = { action: 'remove_keyframes' };
    else if (modifier && command === 'd') edit = { action: 'duplicate_keyframes', offset: step };
    else if (!modifier && (command === 'arrowleft' || command === 'arrowright')) {
        edit = { action: 'move_keyframes', offset: command === 'arrowleft' ? -step : step };
    }
    if (!edit) return;
    event.preventDefault();
    event.stopPropagation();
    editKeys(keys, found, edit);
}

function seek(): void {
    if (!model.frame || program === undefined || pending || keyEdit || queued === undefined) return;
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
