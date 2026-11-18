import type { FrameTarget } from '../src/protocol';

export function dragKeyframe(button: HTMLButtonElement, lane: HTMLElement, initial: number, duration: number,
    frame: () => FrameTarget | undefined, commit: (time: number, frame: FrameTarget) => void): void {
    let swallowClick = false;
    button.addEventListener('pointerdown', event => {
        const target = frame();
        const width = lane.getBoundingClientRect().width;
        if (event.button !== 0 || !event.isPrimary || !target || width <= 0) return;
        event.preventDefault();
        button.focus({ preventScroll: true });
        swallowClick = false;
        const { pointerId, clientX: start } = event;
        const listeners = new AbortController();
        const options = { capture: true, signal: listeners.signal };
        let moved = false;
        const timeAt = (x: number) => {
            const time = initial + (x - start) * duration / width;
            return Math.min(duration, Math.max(0, Math.round(time * 1000) / 1000));
        };
        const finish = () => {
            listeners.abort();
            button.classList.remove('timeline-key-dragging');
            button.style.left = `${initial * 100 / duration}%`;
            if (button.hasPointerCapture(pointerId)) button.releasePointerCapture(pointerId);
        };
        const cancel = () => { swallowClick = true; finish(); };
        document.addEventListener('pointermove', pointer => {
            if (pointer.pointerId !== pointerId) return;
            moved ||= Math.abs(pointer.clientX - start) >= 4;
            if (!moved) return;
            button.style.left = `${timeAt(pointer.clientX) * 100 / duration}%`;
            button.classList.add('timeline-key-dragging');
        }, options);
        document.addEventListener('pointerup', pointer => {
            if (pointer.pointerId !== pointerId) return;
            swallowClick = moved || Math.abs(pointer.clientX - start) >= 4;
            const time = timeAt(pointer.clientX);
            const current = frame();
            finish();
            if (swallowClick && time !== initial && current?.session === target.session && current.token === target.token) {
                commit(time, target);
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
