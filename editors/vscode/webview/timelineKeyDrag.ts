import type { FrameTarget } from '../src/protocol';
import type { KeyframeLocation } from '../src/timelineProtocol';

export interface RenderedKeyframe { key: KeyframeLocation; button: HTMLButtonElement }

export function dragKeyframe(item: RenderedKeyframe, lane: HTMLElement, duration: number,
    selected: () => readonly RenderedKeyframe[], frame: () => FrameTarget | undefined,
    snapTime: (time: number) => number,
    commit: (keys: KeyframeLocation[], offset: number, frame: FrameTarget) => void): void {
    const { button, key: { time: initial } } = item;
    let swallowClick = false;
    button.addEventListener('pointerdown', event => {
        const target = frame();
        const width = lane.getBoundingClientRect().width;
        if (event.button !== 0 || !event.isPrimary || !target || width <= 0) return;
        if (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
        event.preventDefault();
        button.focus({ preventScroll: true });
        swallowClick = false;
        const group = selected();
        const limits = group.reduce(({ low, high }, { key }) => ({
            low: Math.max(low, -key.time),
            high: Math.min(high, duration - key.time),
        }), { low: -initial, high: duration - initial });
        const { pointerId, clientX: start } = event;
        const listeners = new AbortController();
        const options = { capture: true, signal: listeners.signal };
        let moved = false;
        const offsetAt = (x: number) => {
            const time = initial + (x - start) * duration / width;
            const offset = snapTime(time) - initial;
            return Math.min(limits.high, Math.max(limits.low, offset));
        };
        const show = (offset: number, dragging: boolean) => {
            for (const { button, key } of group) {
                button.style.left = `${(key.time + offset) * 100 / duration}%`;
                button.classList.toggle('timeline-key-dragging', dragging);
            }
        };
        const finish = () => {
            listeners.abort();
            show(0, false);
            if (button.hasPointerCapture(pointerId)) button.releasePointerCapture(pointerId);
        };
        const cancel = () => { swallowClick = true; finish(); };
        document.addEventListener('pointermove', pointer => {
            if (pointer.pointerId !== pointerId) return;
            moved ||= Math.abs(pointer.clientX - start) >= 4;
            if (!moved) return;
            show(offsetAt(pointer.clientX), true);
        }, options);
        document.addEventListener('pointerup', pointer => {
            if (pointer.pointerId !== pointerId) return;
            swallowClick = moved || Math.abs(pointer.clientX - start) >= 4;
            const offset = offsetAt(pointer.clientX);
            const current = frame();
            finish();
            if (swallowClick && offset !== 0 && current?.session === target.session && current.token === target.token) {
                commit(group.map(({ key }) => key), offset, target);
            }
        }, options);
        button.addEventListener('lostpointercapture', cancel, options);
        window.addEventListener('blur', cancel, options);
        document.addEventListener('keydown', key => {
            if (key.key === 'Escape') { key.preventDefault(); cancel(); }
        }, options);
        button.setPointerCapture(pointerId);
    });
    button.addEventListener('click', event => {
        if (!swallowClick || event.detail === 0) return;
        swallowClick = false;
        event.stopImmediatePropagation();
    }, true);
}
