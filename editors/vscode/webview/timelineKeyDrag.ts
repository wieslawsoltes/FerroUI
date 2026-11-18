import type { FrameTarget } from '../src/protocol';

interface Drag {
    pointer: number;
    frame: FrameTarget;
    x: number;
    width: number;
    time: number;
    moved: boolean;
}

export function dragKeyframe(button: HTMLButtonElement, lane: HTMLElement, initial: number, duration: number,
    frame: () => FrameTarget | undefined, commit: (time: number, frame: FrameTarget) => void): void {
    let drag: Drag | undefined;
    let swallowClick = false;
    const reset = () => {
        drag = undefined;
        button.classList.remove('timeline-key-dragging');
        button.style.left = `${initial * 100 / duration}%`;
    };
    button.addEventListener('pointerdown', event => {
        const target = frame();
        const width = lane.getBoundingClientRect().width;
        if (event.button !== 0 || !target || width <= 0) return;
        swallowClick = false;
        drag = { pointer: event.pointerId, frame: target, x: event.clientX, width, time: initial, moved: false };
        button.setPointerCapture(event.pointerId);
    });
    button.addEventListener('pointermove', event => {
        if (!drag || drag.pointer !== event.pointerId) return;
        const distance = event.clientX - drag.x;
        if (!drag.moved && Math.abs(distance) < 4) return;
        drag.moved = true;
        const time = initial + distance * duration / drag.width;
        drag.time = Math.min(duration, Math.max(0, Math.round(time * 1000) / 1000));
        button.style.left = `${drag.time * 100 / duration}%`;
        button.classList.add('timeline-key-dragging');
    });
    button.addEventListener('pointerup', event => {
        if (!drag || drag.pointer !== event.pointerId) return;
        const { moved, time, frame } = drag;
        swallowClick = moved;
        reset();
        if (moved && time !== initial) commit(time, frame);
    });
    button.addEventListener('lostpointercapture', reset);
    button.addEventListener('click', event => {
        if (!swallowClick || event.detail === 0) return;
        swallowClick = false;
        event.stopImmediatePropagation();
    }, true);
}
