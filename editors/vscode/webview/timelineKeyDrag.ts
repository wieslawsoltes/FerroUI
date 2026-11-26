import type { FrameTarget } from '../src/protocol';
import type { CurveChannel, CurveStep, KeyframeComponent, KeyframeLocation } from '../src/timelineProtocol';
import { create, labelButton } from './dom';
import type { CurveScale } from './timelineCurveScale';
import { clamp, timelinePercent, type TimelineViewport } from './timelineViewport';

export interface RenderedKeyframe { key: KeyframeLocation; button: HTMLButtonElement }

export function renderKeyframe(key: KeyframeLocation, label: string): RenderedKeyframe {
    const button = create('button');
    button.type = 'button';
    button.className = 'timeline-key';
    labelButton(button, label);
    return { key, button };
}

export interface CurveHandle {
    value: number;
    scale: CurveScale;
    channel: CurveChannel;
    normalized: boolean;
    step?: CurveStep;
    keyRow: number;
}

interface CurveDrag {
    handle: () => CurveHandle | undefined;
    commit: (time: number, value: number, channel: CurveChannel, frame: FrameTarget) => void;
}

export function curveKeyPosition(percent: number, keyRow: number): string {
    return keyRow ? `calc(${percent}% - ${percent / 100 * keyRow}px)` : `${percent}%`;
}

export function curveEditValue(channel: CurveChannel, value: number): number | KeyframeComponent {
    return channel === 'value' ? value : { channel, value };
}

function boundCurveValue(handle: CurveHandle, value: number): number {
    if (!Number.isFinite(value)) return handle.value;
    if (handle.step === 'boolean') return Math.round(clamp(value, 0, 1));
    if (handle.step === 'integer') return snapCurveInteger(value);
    return handle.normalized ? clamp(value, 0, 1) : value;
}

export function snapCurveInteger(value: number): number {
    return clamp(Math.sign(value) * Math.round(Math.abs(value)), -2147483648, 2147483647);
}

/** Retiming in a lane moves the selection. A curve marker edits one time and value. */
export function dragKeyframe(item: RenderedKeyframe, lane: HTMLElement, viewport: () => TimelineViewport,
    selected: () => readonly RenderedKeyframe[], frame: () => FrameTarget | undefined,
    snapTime: (time: number) => number,
    commitGroup: (keys: KeyframeLocation[], offset: number, frame: FrameTarget) => void,
    curve?: CurveDrag): void {
    const { button, key: { time: initial } } = item;
    let swallowClick = false;
    button.addEventListener('pointerdown', event => {
        const target = frame();
        const { width, height } = lane.getBoundingClientRect();
        if (event.button !== 0 || !event.isPrimary || !target || width <= 0 || height <= 0) return;
        if (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
        const display = lane.dataset.curve;
        const handle = curve?.handle();
        if (button.hasAttribute('data-curve-key') && !handle) return;
        const graph = Boolean(handle);
        const axis = viewport();
        event.preventDefault();
        button.focus({ preventScroll: true });
        swallowClick = false;
        const group = graph ? [item] : selected();
        const limits = group.reduce(({ low, high }, { key }) => ({
            low: Math.max(low, -key.time),
            high: Math.min(high, axis.duration - key.time),
        }), { low: -initial, high: axis.duration - initial });
        const { pointerId, clientX: startX, clientY: startY } = event;
        const listeners = new AbortController();
        const options = { capture: true, signal: listeners.signal };
        const unchanged = () => {
            const current = frame();
            const active = curve?.handle();
            return viewport() === axis && current?.session === target.session && current.token === target.token
                && lane.dataset.curve === display && (!handle || (active?.scale === handle.scale && active.channel === handle.channel));
        };
        const delta = (pointer: PointerEvent) => {
            const dx = pointer.clientX - startX;
            const dy = pointer.clientY - startY;
            // A few pixels of incidental movement should preserve the other coordinate.
            const offset = Math.abs(dx) < (graph ? 4 : 1) ? 0 :
                clamp(snapTime(initial + dx * axis.span / width) - initial, limits.low, limits.high);
            let value = handle?.value;
            if (handle && Math.abs(dy) >= 4) {
                value = boundCurveValue(handle, handle.scale.fromOffset(handle.value, 100 * dy / Math.max(1, height - handle.keyRow)));
            }
            return { offset, value };
        };
        const show = (offset: number, value: number | undefined, dragging: boolean) => {
            for (const { button, key } of group) {
                button.style.left = `${clamp(timelinePercent(viewport(), key.time + offset), 0, 100)}%`;
                button.classList.toggle('timeline-key-dragging', dragging);
            }
            if (handle && value !== undefined) {
                button.style.top = curveKeyPosition(clamp(handle.scale.percent(value), 3, 97), handle.keyRow);
            }
        };
        const finish = () => {
            listeners.abort();
            for (const { button, key } of group) {
                button.style.left = `${clamp(timelinePercent(viewport(), key.time), 0, 100)}%`;
                button.classList.remove('timeline-key-dragging');
            }
            const current = curve?.handle();
            if (handle && current) button.style.top = curveKeyPosition(current.scale.percent(current.value), current.keyRow);
            if (button.hasPointerCapture(pointerId)) button.releasePointerCapture(pointerId);
        };
        const cancel = () => { swallowClick = true; finish(); };
        const moved = (pointer: PointerEvent) => (graph ? Math.hypot(pointer.clientX - startX, pointer.clientY - startY)
            : Math.abs(pointer.clientX - startX)) >= 4;
        let crossedThreshold = false;
        document.addEventListener('pointermove', pointer => {
            if (pointer.pointerId !== pointerId) return;
            if (!unchanged()) { cancel(); return; }
            crossedThreshold ||= moved(pointer);
            if (crossedThreshold) {
                const { offset, value } = delta(pointer);
                show(offset, value, true);
            }
        }, options);
        document.addEventListener('pointerup', pointer => {
            if (pointer.pointerId !== pointerId) return;
            swallowClick = crossedThreshold || moved(pointer);
            const { offset, value } = delta(pointer);
            const valid = unchanged();
            finish();
            if (!swallowClick || !valid) return;
            if (handle && curve && value !== undefined && (offset !== 0 || value !== handle.value)) {
                curve.commit(initial + offset, value, handle.channel, target);
            } else if (!graph && offset !== 0) commitGroup(group.map(({ key }) => key), offset, target);
        }, options);
        button.addEventListener('lostpointercapture', cancel, options);
        document.addEventListener('pointercancel', pointer => { if (pointer.pointerId === pointerId) cancel(); }, options);
        window.addEventListener('blur', cancel, options);
        document.addEventListener('keydown', key => {
            if (key.key === 'Escape') { key.preventDefault(); key.stopPropagation(); cancel(); }
        }, options);
        button.setPointerCapture(pointerId);
    });
    button.addEventListener('keydown', event => {
        if (!curve || !event.altKey || event.ctrlKey || event.metaKey || (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')) return;
        const target = frame();
        const handle = curve.handle();
        if (!target || !handle) return;
        event.preventDefault();
        event.stopPropagation();
        const up = event.key === 'ArrowUp';
        const value = boundCurveValue(handle, handle.step ? handle.value + (up ? 1 : -1) : handle.scale.nudge(handle.value, up));
        if (value !== handle.value) curve.commit(initial, value, handle.channel, target);
    });
    button.addEventListener('click', event => {
        if (!swallowClick || event.detail === 0) return;
        swallowClick = false;
        event.stopImmediatePropagation();
    }, true);
}
