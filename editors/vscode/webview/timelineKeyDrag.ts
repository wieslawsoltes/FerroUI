import type { FrameTarget } from '../src/protocol';
import type { KeyframeLocation } from '../src/timelineProtocol';
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

interface CurveDrag {
    value: number;
    scale: () => CurveScale | undefined;
    commit: (time: number, value: number, frame: FrameTarget) => void;
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
        const graph = lane.hasAttribute('data-curve');
        const scale = graph ? curve?.scale() : undefined;
        if (graph && !scale) return;
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
            return viewport() === axis && current?.session === target.session && current.token === target.token
                && lane.hasAttribute('data-curve') === graph && (!graph || curve?.scale() === scale);
        };
        const delta = (pointer: PointerEvent) => {
            const dx = pointer.clientX - startX;
            const dy = pointer.clientY - startY;
            // A few pixels of incidental movement should preserve the other coordinate.
            const offset = Math.abs(dx) < (graph ? 4 : 1) ? 0 :
                clamp(snapTime(initial + dx * axis.span / width) - initial, limits.low, limits.high);
            let value = curve?.value;
            if (value !== undefined && scale && Math.abs(dy) >= 4) {
                const proposed = scale.fromOffset(value, 100 * dy / height);
                if (Number.isFinite(proposed)) value = proposed;
            }
            return { offset, value };
        };
        const show = (offset: number, value: number | undefined, dragging: boolean) => {
            for (const { button, key } of group) {
                button.style.left = `${clamp(timelinePercent(viewport(), key.time + offset), 0, 100)}%`;
                button.classList.toggle('timeline-key-dragging', dragging);
            }
            if (scale && value !== undefined) button.style.top = `${clamp(scale.percent(value), 3, 97)}%`;
        };
        const finish = () => {
            listeners.abort();
            if (viewport() === axis) show(0, curve?.value, false);
            else for (const { button } of group) button.classList.remove('timeline-key-dragging');
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
            if (graph && curve && value !== undefined && (offset !== 0 || value !== curve.value)) {
                curve.commit(initial + offset, value, target);
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
    button.addEventListener('click', event => {
        if (!swallowClick || event.detail === 0) return;
        swallowClick = false;
        event.stopImmediatePropagation();
    }, true);
}
