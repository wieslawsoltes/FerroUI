import type { FrameTarget } from '../src/protocol';
import type { FromTimeline, KeyframeGroupEdit, KeyframeLocation, TimelineControl, TimelineProgram, TimelineState, ToTimeline } from '../src/timelineProtocol';
import { canPreviewProgram, previewClock } from '../src/timelinePrograms';
import { element, labelButton } from './dom';
import { TimelineCurves } from './timelineCurves';
import { dragKeyframe, renderKeyframe, type RenderedKeyframe } from './timelineKeyDrag';
import { TimelineKeyframeEditor } from './timelineKeyframeEditor';
import { bindTimelineMarquee } from './timelineMarquee';
import { TimelineNativePreview } from './timelineNativePreview';
import { TimelineSelection } from './timelineSelection';
import { TimelineStateTransition } from './timelineStateTransition';
import { clamp } from './timelineViewport';
import { TimelineViewportView } from './timelineViewportView';

interface ProgramIdentity { id: number; key?: string }
interface SavedTimeline extends ProgramIdentity { frameRate?: number; curves?: boolean }
declare function acquireVsCodeApi(): {
    postMessage(message: FromTimeline): void;
    getState(): SavedTimeline | number | undefined;
    setState(program: SavedTimeline): void;
};
const vscode = acquireVsCodeApi();
const status = element('timeline-status');
const toolbar = element('timeline-toolbar');
const sourceIssues = element<HTMLButtonElement>('timeline-source-issues');
const container = element('timeline-program');
const programs = element<HTMLSelectElement>('timeline-programs');
const snap = element<HTMLSelectElement>('timeline-snap');
const transport = element('timeline-transport');
const play = element<HTMLButtonElement>('timeline-play');
const stop = element<HTMLButtonElement>('timeline-stop');
const playAll = element<HTMLButtonElement>('timeline-play-all');
const recordMode = element<HTMLButtonElement>('timeline-record-mode');
const record = element<HTMLButtonElement>('timeline-record');
const actions = element<HTMLButtonElement>('timeline-actions');
const keyActions = element<HTMLButtonElement>('timeline-key-actions');
const curveToggle = element<HTMLButtonElement>('timeline-curves');
const fitKeys = element<HTMLButtonElement>('timeline-fit-keys');
const slider = element<HTMLInputElement>('timeline-seek');
const time = element<HTMLOutputElement>('timeline-time');
const tracks = element<HTMLOListElement>('timeline-tracks');
const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
let model: TimelineState = { programs: [], names: {}, writable: false };
const saved = vscode.getState();
let remembered = typeof saved === 'number' ? { id: saved } : saved;
let frameRate = [24, 30, 60].find(rate => typeof saved === 'object' && rate === saved.frameRate) ?? 0;
let curveMode = typeof saved === 'object' && saved.curves === true;
snap.value = String(frameRate);
let program: number | undefined;
let timedProgramSession: number | undefined;
let queued: number | undefined;
let submitted: number | undefined;
let pending = false;
let lastContent = '';
let lastPrograms = '';
const selection = new TimelineSelection();
let renderedKeys: RenderedKeyframe[] = [];
const viewport = new TimelineViewportView(slider, () => renderedKeys, showCurves);
const curves = new TimelineCurves(viewport.grid, message => vscode.postMessage(message));
let owner: { session: number; program: ProgramIdentity } | undefined;
let keyEdit: { before: FrameTarget; accepted?: KeyframeLocation[] } | undefined;

const gestureFrame = () => pending || keyEdit || keyframeEditor.applying || positionFor(current())?.playing ? undefined : model.frame;
const nativePreview = new TimelineNativePreview(queueSeek, () => programAction('programTargets'), () => programAction('revealProgram'));
const transition = new TimelineStateTransition((frame, control) => vscode.postMessage({ type: 'transitionControl', frame, control }));
const keyframeEditor = new TimelineKeyframeEditor(request => {
    vscode.postMessage(request);
    if (request.apply) showPosition();
},
    (frame, program, time) => send(frame, { action: 'seek', program, time }),
    ({ target, keyframe }, property) => { selection.replace([{ target, property, time: keyframe.time }]); showKeySelection(); },
    () => (renderedKeys.find(({ key, button }) => !button.hidden && selection.has(key))?.button ?? slider).focus({ preventScroll: true }));
bindTimelineMarquee(viewport.grid, tracks, () => renderedKeys, gestureFrame, () => viewport.range, (keys, additive) => {
    selection.replace([...(additive ? selection.keys : []), ...keys.map(({ key }) => key)]);
    showKeySelection();
    (keys.at(-1)?.button ?? slider).focus({ preventScroll: true });
});

programs.addEventListener('change', () => {
    const selected = model.programs.find(item => item.id === Number(programs.value));
    if (model.recording && model.frame && program !== undefined && selected && selected.id !== program) {
        send(model.frame, { action: 'record', program, enabled: false });
    }
    if (selected) remember(selected);
    timedProgramSession = undefined;
    selection.replace([]);
    queued = undefined;
    showTracks();
});
snap.addEventListener('change', () => {
    frameRate = Number(snap.value);
    saveTimeline();
});
curveToggle.onclick = () => { curveMode = !curveMode; saveTimeline(); showCurves(); };
fitKeys.onclick = () => viewport.fitKeys(selection.keys.map(key => key.time));
play.addEventListener('click', () => {
    const selected = current();
    if (!model.frame || !selected) return;
    const position = positionFor(selected);
    send(model.frame, { action: position?.playing ? 'pause' : position?.all ? 'play_all' : 'play', program: position?.program ?? selected.id });
});
stop.addEventListener('click', () => {
    const selected = current();
    if (model.frame && selected) send(model.frame, { action: 'stop', program: positionFor(selected)?.program ?? selected.id });
});
playAll.addEventListener('click', () => {
    const selected = current();
    if (model.frame && selected) send(model.frame, { action: 'play_all', program: positionFor(selected)?.program ?? selected.id });
});
recordMode.addEventListener('click', () => {
    const selected = current();
    if (model.frame && selected?.timeline && model.canRecord && !pending && !keyEdit) {
        send(model.frame, { action: 'record', program: selected.id, enabled: !model.recording, time: slider.valueAsNumber });
    }
});
record.addEventListener('click', () => {
    recordAt(slider.valueAsNumber);
});
actions.addEventListener('click', () => {
    if (model.frame && !pending && !keyEdit) vscode.postMessage({ type: 'programActions', frame: model.frame, program });
});
sourceIssues.onclick = () => {
    if (model.frame && !pending && !keyEdit) vscode.postMessage({ type: 'sourceIssues', frame: model.frame });
};
keyActions.addEventListener('click', () => editKeys(selection.keys, model.frame));
slider.addEventListener('input', () => queueSeek(slider.valueAsNumber));
function queueSeek(seconds: number): void {
    if (!model.frame || keyframeEditor.applying || keyEdit || !canPreviewProgram(current())) return;
    queued = seconds;
    showPosition();
    seek();
}
function programAction(type: 'programTargets' | 'revealProgram'): void {
    if (!model.frame || program === undefined || pending || keyEdit) return;
    vscode.postMessage({ type, frame: model.frame, program });
}
container.addEventListener('keydown', event => {
    const frame = gestureFrame();
    if (!frame || program === undefined || (event.key !== '[' && event.key !== ']')) return;
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
    } else if (message.type === 'keyframeDraftResult') {
        keyframeEditor.receive(message);
        showPosition();
    } else if (message.type === 'curveResult') {
        curves.receive(message);
    } else if (message.type === 'error') {
        queued = submitted = keyEdit = undefined;
        pending = false;
        showStatus(message.message);
        transition.resetRequest();
        showPosition();
    }
});
vscode.postMessage({ type: 'ready' });

function current(): TimelineProgram | undefined { return model.programs.find(item => item.id === program); }

function positionFor(selected: TimelineProgram | undefined) {
    const position = model.position;
    if (position?.all || position?.program === selected?.id) return position;
}

function showTracks(): void {
    if (model.transition) {
        toolbar.hidden = container.hidden = true;
        keyframeEditor.close();
        nativePreview.show(undefined, {}, 0, false, false);
        transition.show(model.transition, model.frame);
        showCurves();
        return;
    }
    transition.show();
    const available = model.programs;
    // A rebuilt source can assign different IDs to legacy programs. The native
    // pose points to the resolved clip when the selected program was playing.
    const remapped = model.frame && model.frame.session === timedProgramSession && model.position?.elapsed !== undefined
        ? available.find(item => item.id === model.position?.program) : undefined;
    const selected = remapped ?? available.find(item => item.source_key && item.source_key === remembered?.key)
        ?? available.find(item => matchesProgram(remembered, item)) ?? available[0];
    program = selected?.id;
    if (selected && model.frame) remember(selected);
    const timeline = selected?.timeline;
    toolbar.hidden = !model.frame && !selected;
    sourceIssues.hidden = !model.sourceIssues?.length;
    sourceIssues.disabled = !model.frame;
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
    if (!selected || !timeline) {
        viewport.follow(undefined, false);
        keyframeEditor.close();
        showKeySelection();
        showPosition();
        return;
    }
    const frame = model.frame;
    let newProgram = false;
    if (frame) {
        const sameProgram = owner?.session === frame.session && matchesProgram(owner.program, selected);
        newProgram = !sameProgram;
        if (!sameProgram) selection.replace([]);
        if (keyEdit?.before.session !== frame.session) keyEdit = undefined;
        if (keyEdit?.accepted && keyEdit.before.token !== frame.token) {
            selection.replace(keyEdit.accepted);
            keyEdit = undefined;
        }
        owner = { session: frame.session, program: { id: selected.id, key: selected.source_key ?? undefined } };
        selection.reconcile(timeline.tracks);
    }
    const changedViewport = viewport.follow(timeline.duration, newProgram);
    const content = JSON.stringify([model.frame?.session ?? owner?.session, selected, model.names, model.writable, model.canEditKeyframes]);
    if (lastContent !== content) {
        lastContent = content;
        const restoreFocus = renderedKeys.some(item => item.button === document.activeElement);
        renderedKeys = [];
        curves.clear();
        slider.max = String(timeline.duration);
        tracks.replaceChildren(...timeline.tracks.map((track, index) => {
            const row = document.createElement('li');
            row.className = 'timeline-track';
            const name = document.createElement('span');
            name.className = 'timeline-track-name';
            const label = document.createElement('span');
            label.className = 'timeline-track-label';
            label.textContent = track.property;
            name.append(label);
            const target = model.names[track.target];
            name.title = target ? `${target} · ${track.property}` : track.property;
            const lane = document.createElement('div');
            lane.className = 'timeline-lane';
            const plot = track.keyframes[0]?.value_kind === 'Number' ? curves.add(lane, name, track) : undefined;
            lane.addEventListener('dblclick', event => {
                if (event.target !== lane) return;
                const bounds = lane.getBoundingClientRect();
                const { start, span, duration } = viewport.range;
                if (!bounds.width || !span) return;
                const raw = start + span * clamp((event.clientX - bounds.left) / bounds.width, 0, 1);
                const time = clamp(Math.round(raw * (frameRate || 1000)) / (frameRate || 1000), 0, duration);
                recordAt(time, index);
            });
            for (const key of track.keyframes) {
                const location = { target: track.target, property: track.property, time: key.time };
                const rendered = renderKeyframe(location, `${name.title}, ${number.format(key.time)}s · ${key.value_text} · ${key.easing_text}`);
                const { button } = rendered;
                renderedKeys.push(rendered);
                const value = Number(key.value_text);
                if (plot && Number.isFinite(value)) plot.keys.push({ rendered, value });
                const open = (frame: FrameTarget, focus: boolean) => {
                    if (!model.canEditKeyframes || !(model.writable || selected.sourceRange) || model.documentVersion === undefined) return false;
                    keyframeEditor.open(frame, model.documentVersion, selected.id, track.target, track.property, key,
                        target ?? track.property, timeline.duration, focus);
                    return true;
                };
                const editableCurve = plot && Number.isFinite(value) && model.canEditKeyframes && (model.writable || selected.sourceRange);
                if ((model.writable || editableCurve) && timeline.duration > 0) dragKeyframe(rendered, lane, () => viewport.range,
                    () => selection.has(location) ? renderedKeys.filter(item => selection.has(item.key)) : [rendered],
                    gestureFrame,
                    time => Math.round(time * (frameRate || 1000)) / (frameRate || 1000),
                    (keys, offset, frame) => editKeys(keys, frame, { action: 'move_keyframes', offset }),
                    editableCurve ? { value, scale: () => curves.canEdit(plot) ? plot.scale : undefined, commit: (time, value, frame) => {
                        if (!open(frame, false)) return;
                        selection.replace([location]);
                        showKeySelection();
                        keyframeEditor.applyCurveDrag(frame, time, value);
                    } } : undefined);
                button.addEventListener('click', event => {
                    const frame = gestureFrame();
                    if (!frame) return;
                    const additive = event.metaKey || event.ctrlKey;
                    selection.pick(location, event.shiftKey ? renderedKeys.map(item => item.key) : [], event.shiftKey, additive);
                    showKeySelection();
                    if (event.shiftKey || additive) return;
                    if (open(frame, true)) return;
                    if (model.writable) vscode.postMessage({ type: 'keyframe', frame, program: selected.id, track: index, time: key.time });
                    else send(frame, { action: 'seek', program: selected.id, time: key.time });
                });
                button.addEventListener('keydown', event => keyShortcuts(event, location, timeline.duration));
                lane.append(button);
            }
            row.append(name, lane);
            return row;
        }));
        viewport.layout();
        if (restoreFocus) {
            const last = renderedKeys.findLast(item => !item.button.hidden && selection.has(item.key));
            (last?.button ?? slider).focus({ preventScroll: true });
        }
    } else if (changedViewport) viewport.layout();
    showKeySelection();
    showPosition();
}

function matchesProgram(identity: ProgramIdentity | undefined, program: TimelineProgram): boolean {
    // Authoring can assign a key to a legacy program and Undo can remove it again.
    return Boolean(identity && (identity.key && program.source_key ? identity.key === program.source_key : identity.id === program.id));
}

function remember(program: TimelineProgram): void {
    const key = program.source_key ?? undefined;
    if (remembered?.id === program.id && remembered.key === key) return;
    remembered = { id: program.id, key };
    saveTimeline();
}

function saveTimeline(): void {
    if (remembered) vscode.setState({ ...remembered, frameRate, curves: curveMode });
}

function showCurves(): void {
    const selected = current();
    const hasScalarTrack = selected?.timeline?.tracks.some(track => track.keyframes[0]?.value_kind === 'Number');
    curveToggle.hidden = !model.canShowCurves || !hasScalarTrack || Boolean(model.transition);
    const enabled = !curveToggle.hidden && curveMode;
    curveToggle.setAttribute('aria-pressed', String(enabled));
    fitKeys.hidden = !enabled || selection.size === 0;
    fitKeys.disabled = selection.size === 0;
    const frame = gestureFrame();
    curves.follow(enabled, viewport.range, frame && model.curveCatalog && selected ?
        { frame, catalog: model.curveCatalog, program: selected.id } : undefined);
}

function showPosition(): void {
    if (model.transition) return;
    const selected = current();
    if (model.frame && model.position && !model.position.busy) {
        timedProgramSession = model.position.elapsed !== undefined && model.position.program === selected?.id ? model.frame.session : undefined;
    }
    const timeline = selected?.timeline;
    const clock = previewClock(selected);
    const position = positionFor(selected);
    const playing = Boolean(position?.playing);
    const editing = Boolean(keyEdit || keyframeEditor.applying);
    const available = canPreviewProgram(selected);
    const showTransport = Boolean(clock || position?.all);
    transport.hidden = !showTransport && !model.canPlayAll;
    play.hidden = stop.hidden = !showTransport;
    time.hidden = !showTransport;
    playAll.hidden = !model.canPlayAll;
    playAll.disabled = !model.frame || pending || playing || editing;
    playAll.setAttribute('aria-pressed', String(Boolean(position?.all)));
    snap.hidden = !timeline || !model.writable;
    snap.disabled = !model.frame || pending || playing || editing;
    record.hidden = !timeline || !model.writable;
    record.disabled = !model.frame || pending || playing || editing;
    recordMode.hidden = !timeline || !model.canRecord;
    recordMode.disabled = !model.frame || !available || pending || playing || editing;
    labelButton(recordMode, model.canRecordGeometry ? 'Record animation edits' : 'Record property changes');
    recordMode.setAttribute('aria-pressed', String(Boolean(model.recording)));
    actions.disabled = !model.frame || pending || playing || editing || (!selected && !model.writable);
    programs.disabled = pending || playing || editing || !model.frame || !selected;
    keyActions.disabled = !model.frame || pending || playing || editing;
    const seconds = queued ?? submitted ?? (position?.program === selected?.id ? position?.time : undefined) ?? 0;
    nativePreview.show(selected, model.names, seconds, Boolean(model.frame && !playing && !editing), pending);
    if (timeline) {
        slider.value = String(seconds);
        slider.setAttribute('aria-valuetext', `${number.format(seconds)} of ${number.format(timeline.duration)} seconds`);
        viewport.position(seconds);
        time.value = `${number.format(seconds)} / ${number.format(timeline.duration)} s`;
    }
    if (clock?.native) {
        time.value = clock.total_duration === null ? `${number.format(seconds)} s`
            : `${number.format(Math.min(seconds, clock.total_duration))} / ${number.format(clock.total_duration)} s`;
    }
    if (position?.elapsed !== undefined) {
        const displayClock = position.all || clock?.total_duration == null ? `${number.format(position.elapsed)} s elapsed`
            : `${number.format(position.elapsed)} / ${number.format(clock.total_duration)} s elapsed`;
        if (position.all) time.value = `${number.format(position.elapsed)} s`;
        time.title = position.all ? `All animations · ${displayClock}` : displayClock;
        time.setAttribute('aria-label', time.title);
    } else {
        time.removeAttribute('title');
        time.removeAttribute('aria-label');
    }
    const playLabel = position?.all ? (position.completed ? 'Play all animations' : 'Resume all animations') : 'Play animation';
    labelButton(play, playing ? (position?.all ? 'Pause all animations' : 'Pause animation') : playLabel, playing ? 'debug-pause' : 'play');
    play.disabled = !model.frame || !(available || position?.all) || (!playing && (pending || editing));
    stop.disabled = !model.frame || !position || editing || (!playing && pending);
    slider.disabled = !model.frame || !timeline || !available || Boolean(clock?.native) || playing || editing;
    for (const { button } of renderedKeys) button.disabled = pending || editing || !model.frame;
    keyframeEditor.follow(pending || playing || editing ? undefined : model.frame, model.documentVersion, program);
    showCurves();
}

function showKeySelection(): void {
    keyActions.hidden = !current()?.timeline || !model.writable || selection.size === 0;
    for (const { button, key } of renderedKeys) button.setAttribute('aria-pressed', String(selection.has(key)));
    showCurves();
}

function editKeys(keys: KeyframeLocation[], frame?: FrameTarget, edit?: KeyframeGroupEdit): void {
    if (!frame || program === undefined || pending || keyEdit || !model.writable || !keys.length) return;
    keyEdit = { before: frame };
    keyframeEditor.close();
    vscode.postMessage({ type: 'keyframeGroup', frame, program, keys, edit });
    showPosition();
}

function recordAt(time: number, track?: number): void {
    const frame = gestureFrame();
    const selected = current();
    if (!frame || !selected?.timeline || !model.writable) return;
    keyframeEditor.close();
    keyEdit = { before: frame };
    vscode.postMessage({ type: 'recordKeyframe', frame, program: selected.id, time, track });
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
    if (!model.frame || program === undefined || pending || keyEdit || keyframeEditor.applying || queued === undefined) return;
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
