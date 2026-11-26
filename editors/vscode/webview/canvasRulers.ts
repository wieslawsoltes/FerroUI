import type { GuideAxis, RulerSpec } from '../src/canvasGuidesProtocol';
import { element } from './dom';

export interface GuideProjection {
    left: number; top: number; width: number; height: number;
    x: number; y: number; scale: number;
}

export function sameProjection(a: GuideProjection, b: GuideProjection): boolean {
    return a.left === b.left && a.top === b.top && a.width === b.width && a.height === b.height
        && a.x === b.x && a.y === b.y && a.scale === b.scale;
}

export function guidePixel(axis: GuideAxis, value: number, view: GuideProjection): number {
    return value * view.scale + (axis === 'Horizontal' ? view.y : view.x);
}

const coordinates = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3, useGrouping: false });
export function guidePosition(value: number): string { return coordinates.format(value === 0 ? 0 : value); }

export class CanvasRulers {
    private readonly top = element<HTMLCanvasElement>('ruler-top');
    private readonly left = element<HTMLCanvasElement>('ruler-left');
    private readonly corner = element('ruler-origin');

    draw(view: GuideProjection, spec: RulerSpec, visible: boolean): void {
        const { thickness } = spec;
        const show = visible && view.width > thickness && view.height > thickness;
        this.top.hidden = this.left.hidden = this.corner.hidden = !show;
        if (!show) return;
        drawRail(this.top, view.width, view.x, view.scale, spec, true);
        drawRail(this.left, view.height, view.y, view.scale, spec, false);
    }
}

function drawRail(canvas: HTMLCanvasElement, length: number, offset: number, scale: number, spec: RulerSpec, horizontal: boolean): void {
    const { thickness, factors, subdivisions } = spec;
    const context = canvas.getContext('2d');
    if (!context) return;
    const dpr = window.devicePixelRatio || 1;
    const width = Math.ceil(dpr * (horizontal ? length : thickness));
    const height = Math.ceil(dpr * (horizontal ? thickness : length));
    if (canvas.width !== width) canvas.width = width;
    if (canvas.height !== height) canvas.height = height;
    context.setTransform(dpr, 0, 0, dpr, 0, 0);
    context.clearRect(0, 0, width / dpr, height / dpr);
    const style = getComputedStyle(canvas);
    context.fillStyle = style.color;
    context.font = `10px ${style.fontFamily}`;
    context.textBaseline = 'middle';
    if (!horizontal) context.setTransform(0, dpr, -dpr, 0, thickness * dpr, 0);

    const raw = spec.majorPixels / scale;
    const power = 10 ** Math.floor(Math.log10(raw));
    const factor = factors.find(value => raw / power <= value) ?? factors.at(-1)!;
    const minor = power * factor / subdivisions;
    const first = Math.ceil((thickness - offset) / (scale * minor));
    const last = Math.min(first + spec.maxTicks, Math.floor((length - offset) / (scale * minor)));
    for (let tick = first; tick <= last; tick++) {
        const value = tick * minor;
        const pixel = value * scale + offset;
        const major = tick % subdivisions === 0;
        const size = major ? 7 : 4;
        context.globalAlpha = major ? 1 : 0.55;
        context.fillRect(Math.round(pixel), horizontal ? thickness - size : 0, 1, size);
        if (major) context.fillText(guidePosition(value), pixel + 3, horizontal ? 9 : thickness - 9);
    }
    context.globalAlpha = 1;
}
