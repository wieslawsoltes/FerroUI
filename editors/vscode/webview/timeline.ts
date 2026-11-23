import type { AnimationGraphMode } from '../src/animationGraph';
import type { FrameTarget } from '../src/protocol';
import type { FromTimeline, KeyframeLocation, TimelineControl, TimelineKeyAction, TimelineProgram, TimelineState, ToTimeline } from '../src/timelineProtocol';
import type { TimelineTrackCommand, TimelineTrackView } from '../src/timelineTrackProtocol';
import type { TimelineSnapChange, TimelineSnapState } from '../src/timelineTimingProtocol';
import { clipRulerExtent, TimelineClips } from './timelineClips';
import { canPreviewProgram, canPreviewRange, combinedRangeLimit, previewClock } from '../src/timelinePrograms';
import { AnimationGraphMenu } from './animationGraphMenu';
import { anchorPopover, element, labelButton } from './dom';
import { TimelineChannels } from './timelineChannels';
import { curveChannels, TimelineCurves } from './timelineCurves';
import { curveEditValue, dragKeyframe, renderKeyframe, type RenderedKeyframe } from './timelineKeyDrag';
import { TimelineKeyframeEditor } from './timelineKeyframeEditor';
import { bindTimelineMarquee } from './timelineMarquee';
import { TimelineMarkerView } from './timelineMarkerView';
import { TimelineNativePreview } from './timelineNativePreview';
import { TimelineNavigation } from './timelineNavigation';
import { keyframeId, TimelineSelection } from './timelineSelection';
import { TimelineStateTransition } from './timelineStateTransition';
import { TimelineTrackFilter } from './timelineTrackFilter';
import { clamp } from './timelineViewport';
import { answerTimelinePresentation, type WorkAreaView } from './timelinePresentation';
import type { TimelineRulerView } from './timelineViewportRequest';
import { TimelineViewportView } from './timelineViewportView';
import { TimelineWorkArea } from './timelineWorkArea';

interface ProgramIdentity { id: number; key?: string }
interface SavedTimeline extends ProgramIdentity { frameRate?: number; curves?: boolean; graph?: AnimationGraphMode; clips?: boolean }
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
const snapField = element('timeline-snap-field');
const transport = element('timeline-transport');
const options = element<HTMLButtonElement>('timeline-options');
const optionsPopup = element('timeline-view-options');
const play = element<HTMLButtonElement>('timeline-play');
const stop = element<HTMLButtonElement>('timeline-stop');
const playAll = element<HTMLButtonElement>('timeline-play-all');
const recordMode = element<HTMLButtonElement>('timeline-record-mode');
const record = element<HTMLButtonElement>('timeline-record');
const actions = element<HTMLButtonElement>('timeline-actions');
const keyActions = element<HTMLButtonElement>('timeline-key-actions');
const fitKeys = element<HTMLButtonElement>('timeline-fit-keys');
const slider = element<HTMLInputElement>('timeline-seek');
const time = element<HTMLOutputElement>('timeline-time');
const tracks = element<HTMLOListElement>('timeline-tracks');
const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
let model: TimelineState = { programs: [], names: {}, writable: false };
const saved = vscode.getState();
let remembered = typeof saved === 'number' ? { id: saved } : saved;
const snapRates = [...snap.options].map(option => Number(option.value));
let frameRate = snapRates.find(rate => typeof saved === 'object' && rate === saved.frameRate) ?? 0;
const graph = new AnimationGraphMenu('timeline', typeof saved === 'object'
    ? saved.graph ?? (saved.curves ? 'value' : 'keys') : 'keys', mode => vscode.postMessage({ type: 'animationGraphMode', mode }));
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
let visibleKeys: RenderedKeyframe[] = [];
let editableKeys: RenderedKeyframe[] = [];
const channels = new TimelineChannels(updateTrackVisibility);
const viewport = new TimelineViewportView(slider, () => visibleKeys, () => {
    channels.layout(viewport.range);
    workArea.layout();
    markerView.layout();
    showCurves();
});
const workArea = new TimelineWorkArea('timeline', viewport.grid, () => viewport.range, () => slider.valueAsNumber, showPosition);
const markerView = new TimelineMarkerView('timeline', slider, () => viewport.range, () => slider.valueAsNumber, (edit, time) => {
    const frame = gestureFrame();
    if (!frame || program === undefined) return;
    if (time !== undefined) viewport.reveal(time);
    vscode.postMessage({ type: 'markerEdit', frame, program, edit });
});
const trackFilter = new TimelineTrackFilter(updateTrackVisibility);
const navigation = new TimelineNavigation(viewport.grid, slider, () => frameRate, time => {
    viewport.reveal(time);
    queueSeek(time);
});
const curves = new TimelineCurves(viewport.grid, message => vscode.postMessage(message));
let owner: { session: number; program: ProgramIdentity } | undefined;
let keyEdit: { before: FrameTarget; accepted?: KeyframeLocation[] } | undefined;
let appliedProgramEdit: string | undefined;
let clipEdit: FrameTarget | undefined;
let clipRangeScope: string | undefined;
const clips = new TimelineClips<FrameTarget>(typeof saved === 'object' && saved.clips ? 'clips' : 'tracks', {
    changed: layout => {
        vscode.postMessage({ type: 'timelineLayout', layout });
        showTimelineLayout();
    },
    seek: seconds => {
        if (clips.reference === undefined || !model.frame || clipEdit) return;
        queued = seconds;
        showPosition();
        seek();
    },
    transport: (action, program) => {
        if (!model.frame) return;
        const range = clipRange.range;
        if (action === 'play_all' && range) send(model.frame, { action: 'play_range', program, range, all: true });
        else send(model.frame, { action, program });
    },
    place: (frame, programs, start, gap) => {
        clipEdit = frame;
        vscode.postMessage({ type: 'clipPlacement', frame, programs, start, gap });
        showPosition();
    },
    keys: id => {
        vscode.postMessage({ type: 'timelineLayout', layout: 'tracks' });
        const selected = model.programs.find(program => program.id === id);
        if (!selected) return;
        timedProgramSession = undefined;
        remember(selected);
        saveTimeline();
        showTracks();
        programs.focus({ preventScroll: true });
    },
    layout: () => { sharedMarkers.layout(); clipRange.layout(); },
});
const clipRange = new TimelineWorkArea('clip', element('timeline-clips'), () => clips.viewport, () => clips.elapsed, showPosition);
const sharedMarkerButton = element<HTMLButtonElement>('clip-marker-options');
const sharedMarkers = new TimelineMarkerView('clip', element('clip-seek'), () => clips.viewport, () => clips.elapsed, (edit, time) => {
    if (!model.frame || pending || clipEdit || model.position?.playing) return;
    if (time !== undefined) clips.reveal(time);
    vscode.postMessage({ type: 'markerEdit', frame: model.frame, edit });
}, false);

const gestureFrame = () => pending || keyEdit || keyframeEditor.applying || positionFor(current())?.playing ? undefined : model.frame;
const nativePreview = new TimelineNativePreview(queueSeek, () => programAction('programTargets'), () => programAction('revealProgram'));
const transition = new TimelineStateTransition((frame, control) => vscode.postMessage({ type: 'transitionControl', frame, control }));
const keyframeEditor = new TimelineKeyframeEditor(request => {
    vscode.postMessage(request);
    if (request.apply) showPosition();
},
    (frame, program, time) => send(frame, { action: 'seek', program, time }),
    ({ target, keyframe }, property) => { selection.replace([{ target, property, time: keyframe.time }]); showKeySelection(); },
    () => (visibleKeys.find(({ key, button }) => !button.hidden && selection.has(key))?.button ?? slider).focus({ preventScroll: true }));
bindTimelineMarquee(viewport.grid, tracks, () => editableKeys, gestureFrame, () => viewport.range, (keys, additive) => {
    selection.replace([...(additive ? selection.keys : []), ...keys.map(({ key }) => key)]);
    showKeySelection();
    (keys.at(-1)?.button ?? slider).focus({ preventScroll: true });
});
anchorPopover(options, optionsPopup);
optionsPopup.addEventListener('toggle', event => options.setAttribute('aria-expanded', String(event.newState === 'open')));

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
snap.addEventListener('change', () => setFrameRate(Number(snap.value)));
fitKeys.onclick = () => viewport.fitKeys(selection.keys.map(key => key.time));
play.addEventListener('click', () => {
    const selected = current();
    if (!model.frame || !selected) return;
    const position = positionFor(selected);
    const range = workArea.range;
    if (position?.playing) send(model.frame, { action: 'pause', program: position.program });
    else if (range) send(model.frame, { action: 'play_range', program: selected.id, range });
    else send(model.frame, { action: position?.all ? 'play_all' : 'play', program: position?.program ?? selected.id });
});
stop.addEventListener('click', () => {
    const selected = current();
    if (model.frame && selected) send(model.frame, { action: 'stop', program: positionFor(selected)?.program ?? selected.id });
});
playAll.addEventListener('click', () => {
    const selected = current();
    if (!model.frame || !selected) return;
    closeViewOptions();
    send(model.frame, { action: 'play_all', program: positionFor(selected)?.program ?? selected.id });
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
const pasteControls: HTMLElement[] = [slider, keyActions];
for (const control of pasteControls) control.addEventListener('keydown', event => { clipboardShortcut(event, selection.keys); });
function queueSeek(seconds: number): void {
    if (!model.frame || keyframeEditor.applying || keyEdit || !canPreviewProgram(current())) return;
    queued = seconds;
    showPosition();
    seek();
}
function programAction(type: 'programTargets' | 'revealProgram'): void {
    if (!model.frame || program === undefined || pending || keyEdit) return;
    closeViewOptions();
    vscode.postMessage({ type, frame: model.frame, program });
}

function closeViewOptions(): void {
    if (!optionsPopup.matches(':popover-open')) return;
    optionsPopup.hidePopover();
    if (!options.hidden && !toolbar.hidden) options.focus({ preventScroll: true });
}
window.addEventListener('message', event => {
    const message = event.data as ToTimeline | { type: 'state'; state: TimelineState };
    if (message.type === 'state') {
        if (model.frame?.session !== message.state.frame?.session) queued = submitted = undefined;
        if (message.state.frame && clipEdit?.session !== message.state.frame.session) clipEdit = undefined;
        model = message.state;
        pending = Boolean(model.position?.busy);
        if (!pending) submitted = undefined;
        showStatus(model.message);
        showTracks();
        seek();
    } else if (message.type === 'animationGraphMode') {
        if (graph.apply(message.mode)) { saveTimeline(); showCurves(); }
    } else if (message.type === 'timelineLayout') {
        if (clips.applyLayout(message.layout)) showTimelineLayout();
    } else if (message.type === 'timelinePresentationRequest') {
        const frame = model.frame;
        const matchingFrame = frame?.session === message.frame.session && frame.token === message.frame.token;
        vscode.postMessage(answerTimelinePresentation(message, matchingFrame ? {
            ruler: timelineRuler, tracks: timelineTrackView, changeTracks: changeTimelineTracks,
            workArea: timelineWorkArea, snap: timelineSnap, changeSnap: changeFrameSnap,
        } : undefined));
    } else if (message.type === 'position') {
        model.position = message.position;
        model.frame = message.frame;
        pending = Boolean(message.position?.busy);
        if (!pending) submitted = undefined;
        showPosition();
        seek();
    } else if (message.type === 'clipsPlaced') {
        if (clipEdit?.session === message.before.session && clipEdit.token === message.before.token) clipEdit = undefined;
        showPosition();
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
        clipEdit = undefined;
        pending = false;
        showStatus(message.message);
        transition.resetRequest();
        showPosition();
    }
});
vscode.postMessage({ type: 'ready' });
vscode.postMessage({ type: 'timelineDisplayReady', graph: graph.preference, layout: clips.layoutPreference });

function current(): TimelineProgram | undefined { return model.programs.find(item => item.id === program); }

function positionFor(selected: TimelineProgram | undefined) {
    const position = model.position;
    if (position?.all || position?.program === selected?.id) return position;
}

function showTracks(): void {
    if (model.transition) {
        toolbar.hidden = container.hidden = true;
        clips.follow({ programs: [] }, false, true, 0);
        closeViewOptions();
        trackFilter.follow(false, []);
        workArea.follow(0, false);
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
    const programEdit = model.frame && model.preferredProgram
        ? `${model.frame.session}:${model.documentVersion}:${model.preferredProgram}` : undefined;
    const preferred = programEdit && programEdit !== appliedProgramEdit
        ? available.find(item => item.source_key === model.preferredProgram) : undefined;
    if (preferred) {
        appliedProgramEdit = programEdit;
        timedProgramSession = undefined;
    }
    const selected = preferred ?? remapped ?? available.find(item => item.source_key && item.source_key === remembered?.key)
        ?? available.find(item => matchesProgram(remembered, item)) ?? available[0];
    program = selected?.id;
    if (selected && model.frame) remember(selected);
    const timeline = selected?.timeline;
    const filterChanged = trackFilter.follow(Boolean(timeline?.tracks.length), model.selectedTargets ?? []);
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
        workArea.follow(0, false);
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
    workArea.follow(canPreviewRange(selected) ? timeline.duration : 0, newProgram);
    const changedViewport = viewport.follow(timeline.duration, newProgram);
    const content = JSON.stringify([model.frame?.session ?? owner?.session, selected, model.names,
        model.writable, model.canEditKeyframes, model.canEditComponentBatch, model.canAuthorRustKeys]);
    if (lastContent !== content) {
        lastContent = content;
        const restoreFocus = renderedKeys.some(item => item.button === document.activeElement);
        renderedKeys = [];
        curves.clear();
        slider.max = String(timeline.duration);
        const curveTracks = new Map<string, number>();
        channels.render(timeline.tracks, model.names, newProgram, (track, index, target) => {
            const row = document.createElement('li');
            row.className = 'timeline-track';
            const name = document.createElement('span');
            name.className = 'timeline-track-name';
            const label = document.createElement('span');
            label.className = 'timeline-track-label';
            label.textContent = track.property;
            name.append(label);
            name.title = `${target} · ${track.property}`;
            const lane = document.createElement('div');
            lane.className = 'timeline-lane';
            const curveTrack = JSON.stringify([model.frame?.session ?? owner?.session, model.names[track.target], track.property]);
            const occurrence = curveTracks.get(curveTrack) ?? 0;
            curveTracks.set(curveTrack, occurrence + 1);
            const plot = curveChannels(track).length ? curves.add(lane, name, track, `${curveTrack}:${occurrence}`) : undefined;
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
                const easing = key.easing_text.replace(/\s/g, '').toLowerCase();
                button.toggleAttribute('data-eased', Boolean(easing && easing !== 'lineareasing' && easing !== '0,0,1,1'));
                renderedKeys.push(rendered);
                if (plot) plot.keys.push({ rendered, keyframe: key });
                const open = (frame: FrameTarget, focus: boolean) => {
                    if (!model.canEditKeyframes || !(model.writable || selected.sourceRange) || model.documentVersion === undefined) return false;
                    keyframeEditor.open(frame, model.documentVersion, selected.id, track.target, track.property, key,
                        target, timeline.duration, focus, model.canEditComponentBatch);
                    return true;
                };
                const editableCurve = plot && model.canEditKeyframes && (model.writable || selected.sourceRange);
                if (canRetimeKeys() && timeline.duration > 0) dragKeyframe(rendered, lane, () => viewport.range,
                    () => selection.has(location) ? editableKeys.filter(item => selection.has(item.key)) : [rendered],
                    () => channels.isLocked(track.target) ? undefined : gestureFrame(),
                    time => Math.round(time * (frameRate || 1000)) / (frameRate || 1000),
                    (keys, offset, frame) => editKeys(keys, frame, { action: 'move_keyframes', offset }),
                    editableCurve ? { handle: () => curves.handle(plot, key), commit: (time, value, channel, frame) => {
                        if (!open(frame, false)) return;
                        selection.replace([location]);
                        showKeySelection();
                        keyframeEditor.applyCurveDrag(frame, time, curveEditValue(channel, value));
                    } } : undefined);
                button.addEventListener('click', event => {
                    const frame = gestureFrame();
                    if (!frame) return;
                    if (channels.isLocked(track.target)) {
                        if (canPreviewProgram(selected)) send(frame, { action: 'seek', program: selected.id, time: key.time });
                        return;
                    }
                    const additive = event.metaKey || event.ctrlKey;
                    selection.pick(location, event.shiftKey ? editableKeys.map(item => item.key) : [], event.shiftKey, additive);
                    showKeySelection();
                    if (event.shiftKey || additive) return;
                    if (open(frame, true)) return;
                    if (model.writable) vscode.postMessage({ type: 'keyframe', frame, program: selected.id, track: index, time: key.time });
                    else send(frame, { action: 'seek', program: selected.id, time: key.time });
                });
                button.addEventListener('keydown', event => keyShortcuts(event, location, timeline.duration, index));
                lane.append(button);
            }
            row.append(name, lane);
            return row;
        });
        layoutTracks(newProgram);
        if (restoreFocus) {
            const last = visibleKeys.findLast(item => !item.button.hidden && selection.has(item.key));
            (last?.button ?? slider).focus({ preventScroll: true });
        }
    } else if (filterChanged) layoutTracks(false);
    else if (changedViewport) viewport.layout();
    showKeySelection();
    showPosition();
}

function updateTrackVisibility(): void {
    layoutTracks(false);
    showKeySelection();
    showPosition();
}

function layoutTracks(newProgram: boolean): void {
    channels.filter(trackFilter.active, (target, label) => trackFilter.matches(target, label));
    visibleKeys = renderedKeys.filter(({ button }) => !button.parentElement?.closest('[hidden]'));
    editableKeys = visibleKeys.filter(({ key }) => !channels.isLocked(key.target));
    if (editableKeys.length !== renderedKeys.length) {
        const editable = new Set(editableKeys.map(({ key }) => keyframeId(key)));
        keyframeEditor.keepVisible(key => editable.has(keyframeId(key)));
        const selected = selection.keys;
        const retained = selected.filter(key => editable.has(keyframeId(key)));
        if (retained.length !== selected.length) selection.replace(retained);
    }
    navigation.setKeys(visibleKeys.map(({ key }) => key.time), newProgram);
    viewport.layout();
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
    if (remembered) vscode.setState({ ...remembered, frameRate, graph: graph.preference, clips: clips.enabled });
}

function showTimelineLayout(): void {
    queued = undefined;
    keyframeEditor.close();
    closeViewOptions();
    saveTimeline();
    showTracks();
}

function timelineRuler(): TimelineRulerView | undefined {
    if (model.transition) return;
    if (clips.visible) return clips.rulerView();
    const selected = current();
    if (!selected?.timeline) return;
    return { ruler: 'tracks', program: selected.id, range: viewport.range, apply: range => viewport.apply(range),
        selectedKeys: fitKeys.hidden ? undefined : selection.keys.map(key => key.time) };
}

function displayedTrackProgram(): number | undefined {
    const selected = current();
    if (!model.transition && !clips.visible && selected?.timeline?.tracks.length) return selected.id;
}

function timelineTrackView(): TimelineTrackView | undefined {
    const program = displayedTrackProgram();
    if (program === undefined) return;
    const { groups, tracks } = channels.view();
    return { program, filter: trackFilter.state, groups, tracks: curves.trackPresentation(tracks) };
}

function changeTimelineTracks(command: TimelineTrackCommand): void {
    if (command.program !== displayedTrackProgram()) throw new Error('Read the visible Timeline tracks and current program again.');
    switch (command.action) {
        case 'filter': trackFilter.setFilter(command); return;
        case 'collapse': channels.setCollapsed(command.target, command.collapsed); return;
        case 'lock': channels.setLocked(command.target, command.locked); return;
        case 'channel': curves.chooseChannel(command, command.channel); return;
        default: throw new Error('Choose a track filter, target group, or curve component.');
    }
}

function timelineWorkArea(): WorkAreaView | undefined {
    if (model.transition) return;
    if (clips.visible) {
        if (clipRange.state) return { area: clipRange, scope: 'clips', clock: 'elapsed' };
    } else if (program !== undefined && workArea.state) {
        return { area: workArea, scope: 'program', program, clock: 'authored' };
    }
}

function timelineSnap(): TimelineSnapState | undefined {
    if (!model.transition && !clips.visible && !snapField.hidden && program !== undefined) {
        return { program, frame_rate: frameRate, rates: snapRates, editable: !snap.disabled };
    }
}

function changeFrameSnap(change: TimelineSnapChange): void {
    const current = timelineSnap();
    if (!current?.editable || current.program !== change.program) throw new Error('Read the current editable keyframe snap settings.');
    if (!snapRates.includes(change.frame_rate)) throw new Error('Choose a frame rate listed in the Timeline view.');
    setFrameRate(change.frame_rate);
}

function setFrameRate(rate: number): void {
    frameRate = rate;
    snap.value = String(rate);
    saveTimeline();
}

function showCurves(): void {
    const selected = current();
    const hasCurveTrack = selected?.timeline?.tracks.some(track => curveChannels(track).length);
    const available = Boolean(model.canShowCurves && hasCurveTrack && !model.transition && !clips.visible);
    graph.show(available, Boolean(model.canShowVelocity));
    const metric = available && graph.mode !== 'keys' ? graph.mode : undefined;
    fitKeys.hidden = !metric || selection.size === 0;
    fitKeys.disabled = selection.size === 0;
    const frame = gestureFrame();
    curves.follow(metric, viewport.range, frame && model.curveCatalog && selected ?
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
    const combined = model.position?.all ? model.position.elapsed : undefined;
    const reference = model.position?.all && model.programs.some(program => program.id === model.position?.program)
        ? model.position.program : model.programs.find(canPreviewProgram)?.id;
    const clipRows = model.programs.flatMap(program => program.clip ? [{
        id: program.id, name: program.name, clip: program.clip, sourceKey: program.source_key,
        duration: program.preview?.duration ?? program.timeline?.duration ?? 1, keys: Boolean(program.timeline),
    }] : []);
    const markerEnd = Math.max(0, ...(model.sharedMarkers ?? []).map(marker => marker.time));
    const combinedPosition = model.position?.all ? model.position : undefined;
    clipRange.follow(model.canPlayAll && model.canPreviewCombinedRanges && model.clipScope ? combinedRangeLimit(model.programs) : 0,
        clipRangeScope !== model.clipScope, clipRulerExtent(clipRows));
    if (model.clipScope) clipRangeScope = model.clipScope;
    clipRange.adopt(combinedPosition?.previewRange, Boolean(combinedPosition?.playing));
    clips.follow({ frame: model.frame, scope: model.clipScope, programs: clipRows, reference, position: model.position,
        rangeEnabled: Boolean(clipRange.range), minimumDuration: Math.max(markerEnd, clipRange.range?.end ?? 0) },
        pending, editing || Boolean(clipEdit), queued ?? submitted ?? combined ?? 0);
    clipRange.show(!model.frame || pending || editing || Boolean(clipEdit || model.position?.playing),
        combinedPosition?.playing ? combinedPosition.previewRange : undefined);
    sharedMarkerButton.hidden = model.sharedMarkers === undefined;
    sharedMarkerButton.disabled = !model.frame || pending || Boolean(clipEdit || model.position?.playing);
    sharedMarkers.show(model.sharedMarkers ?? [], sharedMarkerButton.disabled);
    container.hidden = !timeline || clips.visible;
    if (clips.visible) {
        snapField.hidden = true;
        nativePreview.show(undefined, {}, 0, false, false);
        showCurves();
        return;
    }
    channels.showLocks(canRetimeKeys(), !model.frame || pending || playing || editing || Boolean(model.recording));
    workArea.adopt(position?.all ? undefined : position?.previewRange, playing && !position?.all);
    workArea.show(!model.frame || pending || playing || editing, playing ? position?.previewRange : undefined);
    markerView.show(selected && canPreviewRange(selected) ? model.markers?.[selected.id] ?? [] : [],
        !model.frame || pending || playing || editing);
    const available = canPreviewProgram(selected);
    const showTransport = Boolean(clock || position?.all);
    transport.hidden = !showTransport;
    play.hidden = stop.hidden = !showTransport;
    const preciseClock = Boolean(timeline && available && !clock?.native && !position?.all);
    time.hidden = !showTransport || preciseClock;
    playAll.hidden = !model.canPlayAll;
    playAll.disabled = !model.frame || pending || playing || editing;
    playAll.setAttribute('aria-pressed', String(Boolean(position?.all)));
    snapField.hidden = !timeline || !(model.writable || canRetimeKeys());
    snap.disabled = !model.frame || pending || playing || editing;
    record.hidden = !timeline || !canChangeKeyStructure();
    record.disabled = !model.frame || pending || playing || editing
        || Boolean(timeline?.tracks.length && timeline.tracks.every(track => channels.isLocked(track.target)));
    recordMode.hidden = !timeline || !model.canRecord;
    recordMode.disabled = !model.frame || !available || pending || playing || editing || (channels.hasLocks && !model.recording);
    labelButton(recordMode, channels.hasLocks && !model.recording ? 'Unlock tracks to record animation edits'
        : model.canRecordGeometry ? 'Record animation edits' : 'Record property changes');
    recordMode.setAttribute('aria-pressed', String(Boolean(model.recording)));
    actions.disabled = !model.frame || pending || playing || editing || (!selected && !model.writable && !model.canCreateRustProgram);
    options.hidden = !selected;
    if (options.hidden) closeViewOptions();
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
    const playLabel = workArea.range ? 'Play preview range' : position?.all
        ? (position.completed ? 'Play all animations' : 'Resume all animations') : 'Play animation';
    const pauseLabel = position?.previewRange ? 'Pause preview range' : position?.all ? 'Pause all animations' : 'Pause animation';
    labelButton(play, playing ? pauseLabel : playLabel, playing ? 'debug-pause' : 'play');
    play.disabled = !model.frame || !(available || position?.all) || (!playing && (pending || editing));
    stop.disabled = !model.frame || !position || editing || (!playing && pending);
    slider.disabled = !model.frame || !timeline || !available || Boolean(clock?.native) || playing || editing;
    navigation.show(seconds, timeline?.duration, preciseClock, time.title);
    for (const { button } of renderedKeys) button.disabled = pending || editing || !model.frame;
    keyframeEditor.follow(pending || playing || editing ? undefined : model.frame, model.documentVersion, program);
    showCurves();
}

function showKeySelection(): void {
    keyActions.hidden = !current()?.timeline || !canRetimeKeys()
        || (!selection.size && !(model.canTransferKeys && channels.editableTracks().length));
    for (const { button, key } of renderedKeys) button.setAttribute('aria-pressed', String(selection.has(key)));
    showCurves();
}

function canRetimeKeys(): boolean {
    return Boolean(model.writable || (model.canEditKeyframes && current()?.sourceRange));
}

function canChangeKeyStructure(): boolean {
    return Boolean(model.writable || (model.canAuthorRustKeys && current()?.sourceRange));
}

function editKeys(keys: KeyframeLocation[], frame?: FrameTarget, edit?: TimelineKeyAction): void {
    const destinations = model.canTransferKeys ? channels.editableTracks() : [];
    const mayPaste = destinations.length > 0 && (!edit || edit.action === 'paste_keyframes');
    const currentFrame = gestureFrame();
    if (!frame || !currentFrame || frame.session !== currentFrame.session || frame.token !== currentFrame.token
        || program === undefined || !canRetimeKeys() || (!keys.length && !mayPaste)
        || (edit?.action === 'paste_keyframes' && !mayPaste)
        || (edit && ['duplicate_keyframes', 'remove_keyframes'].includes(edit.action) && !canChangeKeyStructure())
        || keys.some(key => channels.isLocked(key.target))) return;
    keyEdit = { before: frame };
    keyframeEditor.close();
    vscode.postMessage({ type: 'keyframeGroup', frame, program, keys, edit, destinations });
    showPosition();
}

function recordAt(time: number, track?: number): void {
    const frame = gestureFrame();
    const selected = current();
    if (!frame || !selected?.timeline || !canChangeKeyStructure()) return;
    const tracks = selected.timeline.tracks;
    if (track !== undefined && (!tracks[track] || channels.isLocked(tracks[track].target))) return;
    const candidates = track === undefined && channels.hasLocks
        ? tracks.flatMap((item, index) => channels.isLocked(item.target) ? [] : [index]) : undefined;
    if (candidates && !candidates.length) return;
    keyframeEditor.close();
    keyEdit = { before: frame };
    vscode.postMessage({ type: 'recordKeyframe', frame, program: selected.id, time, track, candidates });
    showPosition();
}

function clipboardShortcut(event: KeyboardEvent, keys: KeyframeLocation[], track?: number): boolean {
    if (!model.canTransferKeys || !(event.metaKey || event.ctrlKey) || !gestureFrame() || !canRetimeKeys()) return false;
    const command = event.key.toLowerCase();
    if (command === 'c' && keys.length) editKeys(keys, model.frame, { action: 'copy_keyframes' });
    else if (command === 'v' && channels.editableTracks().length) {
        editKeys([], model.frame, { action: 'paste_keyframes', reverse: event.shiftKey, track });
    } else return false;
    event.preventDefault();
    event.stopPropagation();
    return true;
}

function keyShortcuts(event: KeyboardEvent, location: KeyframeLocation, duration: number, track: number): void {
    const found = model.frame;
    if (!found || pending || keyEdit || channels.isLocked(location.target)) return;
    const modifier = event.metaKey || event.ctrlKey;
    const command = event.key.toLowerCase();
    const keys = selection.size ? selection.keys : [location];
    if (clipboardShortcut(event, keys, track)) return;
    if (command === ' ' || command === 'escape' || (modifier && command === 'a')) {
        event.preventDefault();
        event.stopPropagation();
        if (command === 'escape') selection.replace([]);
        else if (command === 'a') selection.replace(editableKeys.map(item => item.key));
        else selection.pick(location, editableKeys.map(item => item.key), event.shiftKey, true);
        showKeySelection();
        return;
    }
    if (!canRetimeKeys()) return;
    const step = frameRate ? 1 / frameRate : Math.min(0.1, duration / 20);
    let edit: TimelineKeyAction | undefined;
    if (canChangeKeyStructure() && (command === 'delete' || command === 'backspace')) edit = { action: 'remove_keyframes' };
    else if (canChangeKeyStructure() && modifier && command === 'd') edit = { action: 'duplicate_keyframes', offset: step };
    else if (!modifier && (command === 'arrowleft' || command === 'arrowright')) {
        edit = { action: 'move_keyframes', offset: command === 'arrowleft' ? -step : step };
    }
    if (!edit) return;
    event.preventDefault();
    event.stopPropagation();
    editKeys(keys, found, edit);
}

function seek(): void {
    const reference = clips.visible ? clips.reference : program;
    if (!model.frame || reference === undefined || pending || keyEdit || keyframeEditor.applying || clipEdit || queued === undefined) return;
    const time = queued;
    queued = undefined;
    send(model.frame, { action: clips.visible ? 'scrub_all' : 'seek', program: reference, time });
}

function send(frame: FrameTarget, control: TimelineControl): void {
    pending = true;
    if (control.action === 'seek' || control.action === 'scrub_all') submitted = control.time;
    vscode.postMessage({ type: 'control', frame, control });
    showPosition();
}

function showStatus(message: string | undefined): void {
    status.textContent = message ?? '';
    status.hidden = !message;
}
