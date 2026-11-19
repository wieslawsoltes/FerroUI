import type { FrameTarget } from '../src/protocol';
import { clamp, type TimelineViewport } from './timelineViewport';

export function bindClipDrag(bar: HTMLButtonElement, lane: HTMLElement, initial: number,
    viewport: () => TimelineViewport, frame: () => FrameTarget | undefined,
    show: (start: number) => void, apply: (start: number, frame: FrameTarget) => void): void {
    let swallowClick = false;
    bar.onpointerdown = event => {
        const accepted = frame();
        const view = viewport();
        const width = lane.getBoundingClientRect().width;
        if (!accepted || !event.isPrimary || event.button !== 0 || width <= 0) return;
        const pointer = event.pointerId;
        const origin = event.clientX;
        const listeners = new AbortController();
        const options = { signal: listeners.signal };
        const current = () => {
            const next = frame();
            return next?.session === accepted.session && next.token === accepted.token && viewport() === view;
        };
        const startAt = (x: number) => clamp(initial + (x - origin) * view.span / width, 0, view.duration);
        let dragging = false;
        swallowClick = false;
        const finish = () => {
            listeners.abort();
            show(initial);
            bar.removeAttribute('data-dragging');
            if (bar.hasPointerCapture(pointer)) bar.releasePointerCapture(pointer);
        };
        const cancel = () => { swallowClick = dragging; finish(); };
        bar.setPointerCapture(pointer);
        bar.addEventListener('pointermove', move => {
            if (move.pointerId !== pointer) return;
            if (!current()) { cancel(); return; }
            dragging ||= Math.abs(move.clientX - origin) >= 4;
            if (dragging) {
                bar.setAttribute('data-dragging', '');
                show(startAt(move.clientX));
            }
        }, options);
        bar.addEventListener('pointerup', up => {
            if (up.pointerId !== pointer) return;
            swallowClick = dragging || Math.abs(up.clientX - origin) >= 4;
            const valid = current();
            const time = startAt(up.clientX);
            finish();
            if (valid && swallowClick && time !== initial) apply(time, accepted);
        }, options);
        bar.addEventListener('lostpointercapture', cancel, options);
        bar.addEventListener('pointercancel', cancel, options);
        window.addEventListener('blur', cancel, options);
        document.addEventListener('keydown', key => {
            if (key.key === 'Escape') { key.preventDefault(); cancel(); }
        }, options);
    };
    bar.addEventListener('click', event => {
        if (!swallowClick || event.detail === 0) return;
        swallowClick = false;
        event.stopImmediatePropagation();
    }, true);
}
