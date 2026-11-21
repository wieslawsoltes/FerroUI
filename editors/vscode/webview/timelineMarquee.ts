import type { FrameTarget } from '../src/protocol';
import { create } from './dom';
import type { RenderedKeyframe } from './timelineKeyDrag';
import { clamp, type TimelineViewport } from './timelineViewport';

export function bindTimelineMarquee(grid: HTMLElement, tracks: HTMLElement, keys: () => readonly RenderedKeyframe[],
    frame: () => FrameTarget | undefined, viewport: () => TimelineViewport,
    select: (keys: readonly RenderedKeyframe[], additive: boolean) => void): void {
    const box = create('div');
    box.className = 'timeline-marquee';
    box.setAttribute('aria-hidden', 'true');
    box.hidden = true;
    grid.append(box);

    tracks.addEventListener('pointerdown', event => {
        const current = frame();
        const lane = event.target instanceof HTMLElement ? event.target.closest('.timeline-lane') : undefined;
        if (!current || event.button !== 0 || !event.isPrimary || !lane || event.target !== lane) return;
        const axis = viewport();
        const bounds = grid.getBoundingClientRect();
        const { left: minimumX, right: maximumX } = lane.getBoundingClientRect();
        const trackBounds = tracks.getBoundingClientRect();
        const ruler = grid.querySelector('.timeline-axis')?.getBoundingClientRect();
        const pan = grid.querySelector('.timeline-pan:not([hidden])')?.getBoundingClientRect();
        const minimumY = Math.max(trackBounds.top, ruler?.bottom ?? bounds.top);
        const maximumY = Math.min(trackBounds.bottom, pan?.top ?? bounds.bottom);
        const { clientX: startX, clientY: startY, pointerId } = event;
        const additive = event.metaKey || event.ctrlKey || event.shiftKey;
        const listeners = new AbortController();
        const options = { capture: true, signal: listeners.signal };
        const finish = () => {
            listeners.abort();
            box.hidden = true;
            if (grid.hasPointerCapture(pointerId)) grid.releasePointerCapture(pointerId);
        };
        const rectangle = (pointer: PointerEvent) => {
            const x = clamp(pointer.clientX, minimumX, maximumX);
            const y = clamp(pointer.clientY, minimumY, maximumY);
            return { left: Math.min(startX, x), right: Math.max(startX, x), top: Math.min(startY, y), bottom: Math.max(startY, y) };
        };
        const valid = (pointer: PointerEvent) => pointer.pointerId === pointerId;
        const moved = (pointer: PointerEvent) => Math.hypot(pointer.clientX - startX, pointer.clientY - startY) >= 4;
        const unchanged = () => {
            const next = frame();
            return next?.token === current.token && next.session === current.session && viewport() === axis;
        };
        document.addEventListener('pointermove', pointer => {
            if (!valid(pointer)) return;
            if (!unchanged()) { finish(); return; }
            if (!moved(pointer)) { box.hidden = true; return; }
            if (!grid.hasPointerCapture(pointerId)) grid.setPointerCapture(pointerId);
            const { left, right, top, bottom } = rectangle(pointer);
            box.style.left = `${left - bounds.left + grid.scrollLeft}px`;
            box.style.top = `${top - bounds.top + grid.scrollTop}px`;
            box.style.width = `${right - left}px`;
            box.style.height = `${bottom - top}px`;
            box.hidden = false;
        }, options);
        document.addEventListener('pointerup', pointer => {
            if (!valid(pointer)) return;
            finish();
            if (!unchanged() || !moved(pointer)) return;
            const { left, right, top, bottom } = rectangle(pointer);
            select(keys().filter(({ button }) => {
                if (button.hidden) return false;
                const bounds = button.getBoundingClientRect();
                const x = bounds.left + bounds.width / 2;
                const y = bounds.top + bounds.height / 2;
                return x >= left && x <= right && y >= top && y <= bottom;
            }), additive);
        }, options);
        grid.addEventListener('lostpointercapture', finish, options);
        grid.addEventListener('scroll', finish, options);
        document.addEventListener('pointercancel', pointer => { if (valid(pointer)) finish(); }, options);
        window.addEventListener('blur', finish, options);
        document.addEventListener('keydown', key => {
            if (key.key === 'Escape') { key.preventDefault(); finish(); }
        }, options);
    });
}
