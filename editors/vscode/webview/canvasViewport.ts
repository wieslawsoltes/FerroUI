import type { Bounds } from '../src/protocol';
import { element } from './dom';

interface Point { x: number; y: number }
const stops = [0.125, 0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4, 6, 8] as const;
const minimum = stops[0];
const maximum = stops[stops.length - 1];

/** Scrolls a stage in CSS pixels while the frame and selection stay in native coordinates. */
export class CanvasViewport {
    readonly element = element('canvas');
    private readonly stage = element('stage');
    private readonly artboard = element('artboard');
    private scale = 1;
    private fitting = true;
    private width = 0;
    private height = 0;

    constructor(private readonly onZoom: (fit: boolean, scale: number, hasImage: boolean) => void) {
        new ResizeObserver(() => { if (this.fitting) this.fit(); }).observe(this.element);
        this.update();
    }

    setImage(width: number, height: number): void {
        if (width === this.width && height === this.height) return;
        this.width = width;
        this.height = height;
        if (this.fitting) this.fit();
        else this.update();
    }

    fit(): void {
        this.fitting = true;
        const scale = this.fitScale(this.width, this.height);
        if (scale === undefined) return;
        this.scale = scale;
        this.update();
        this.element.scrollTo(0, 0);
    }

    fitSelection(bounds: Bounds): void {
        const scale = this.fitScale(bounds.width, bounds.height);
        if (scale === undefined || !this.width || !this.height) return;
        this.fitting = false;
        this.scale = scale;
        this.update();
        this.center({ x: bounds.x + bounds.width / 2, y: bounds.y + bounds.height / 2 });
    }

    step(direction: 1 | -1): void {
        const next = direction === 1 ? stops.find(scale => scale > this.scale + 0.001)
            : stops.findLast(scale => scale < this.scale - 0.001);
        if (next !== undefined) this.zoom(next);
        else if (direction < 0 && this.scale > minimum) this.zoom(minimum);
    }

    zoomBy(factor: number, anchor: Point): void { this.zoom(this.scale * factor, anchor); }

    zoom(scale: number, anchor = this.viewportCenter()): void {
        if (!this.width || !this.height) return;
        const previous = this.artboard.getBoundingClientRect();
        const localX = (anchor.x - previous.left) / previous.width;
        const localY = (anchor.y - previous.top) / previous.height;
        this.fitting = false;
        this.scale = Math.max(minimum, Math.min(maximum, scale));
        this.update();
        const next = this.artboard.getBoundingClientRect();
        this.element.scrollBy(next.left + localX * next.width - anchor.x, next.top + localY * next.height - anchor.y);
    }

    private fitScale(width: number, height: number): number | undefined {
        const padding = parseFloat(getComputedStyle(this.stage).paddingLeft);
        const availableWidth = this.element.offsetWidth - padding * 2 - 1;
        const availableHeight = this.element.offsetHeight - padding * 2 - 1;
        if (width <= 0 || height <= 0 || availableWidth <= 0 || availableHeight <= 0) return undefined;
        return Math.min(maximum, availableWidth / width, availableHeight / height);
    }

    private center(point: Point): void {
        const art = this.artboard.getBoundingClientRect();
        const center = this.viewportCenter();
        this.element.scrollBy(art.left + point.x * this.scale - center.x, art.top + point.y * this.scale - center.y);
    }

    private viewportCenter(): Point {
        const bounds = this.element.getBoundingClientRect();
        return { x: bounds.left + this.element.clientWidth / 2, y: bounds.top + this.element.clientHeight / 2 };
    }

    private update(): void {
        const hasImage = this.width > 0 && this.height > 0;
        this.artboard.style.inlineSize = `${this.width * this.scale}px`;
        this.artboard.style.blockSize = `${this.height * this.scale}px`;
        this.onZoom(this.fitting, this.scale, hasImage);
    }
}
