import type { Point } from '../src/protocol';
import type { Artboard } from './artboard';

export function startSpacingDrag(artboard: Artboard, button: HTMLButtonElement, press: PointerEvent,
    preview: (delta: Point) => void, commit: (delta: Point) => void, clear: () => void): (() => void) | undefined {
    if (press.button !== 0 || !press.isPrimary) return;
    const origin = artboard.point(press, true);
    if (!origin) return;
    press.preventDefault();
    press.stopPropagation();
    const rect = artboard.element.getBoundingClientRect();
    const listeners = new AbortController();
    const options = { capture: true, signal: listeners.signal };
    const { pointerId } = press;
    const traveled = (event: PointerEvent) => Math.hypot(event.clientX - press.clientX, event.clientY - press.clientY) >= 2;
    const finish = () => {
        listeners.abort();
        if (button.hasPointerCapture(pointerId)) button.releasePointerCapture(pointerId);
        clear();
    };
    const deltaAt = (event: PointerEvent) => {
        const now = artboard.element.getBoundingClientRect();
        if (rect.x !== now.x || rect.y !== now.y || rect.width !== now.width || rect.height !== now.height) return;
        const point = artboard.point(event, true);
        if (point) return { x: point.x - origin.x, y: point.y - origin.y };
    };
    button.focus({ preventScroll: true });
    document.addEventListener('pointermove', event => {
        if (event.pointerId !== pointerId || !(event.buttons & 1)) return;
        const delta = deltaAt(event);
        if (!delta) { finish(); return; }
        if (traveled(event)) preview(delta);
    }, options);
    document.addEventListener('pointerup', event => {
        if (event.pointerId !== pointerId) return;
        const delta = deltaAt(event);
        finish();
        if (delta && traveled(event) && (delta.x || delta.y)) commit(delta);
    }, options);
    button.addEventListener('lostpointercapture', finish, options);
    document.addEventListener('pointercancel', event => { if (event.pointerId === pointerId) finish(); }, options);
    window.addEventListener('blur', finish, options);
    document.addEventListener('keydown', event => {
        if (event.key !== 'Escape') return;
        event.preventDefault();
        event.stopPropagation();
        finish();
    }, options);
    button.setPointerCapture(pointerId);
    return finish;
}
