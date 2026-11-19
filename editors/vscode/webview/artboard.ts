import type { Bounds, CanvasNode, CanvasSnapshot, Point, Viewport } from '../src/protocol';
import { CanvasViewport } from './canvasViewport';
import { element } from './dom';

export class Artboard {
    readonly element = element('artboard');
    private readonly placeholder = element('placeholder');
    private readonly frame = element<HTMLCanvasElement>('frame');
    // The preview is already rasterized. Keep its backing pixels independent of the compositor.
    private readonly presenter = this.frame.getContext('2d', { willReadFrequently: true })!;
    private readonly selection = element('selection-box');
    private readonly selectionGroup = element('selection-group');
    private readonly hover = element('hover-box');
    private readonly dropBox = element('drop-box');
    private snapshot?: CanvasSnapshot;
    private current = false;
    private selected?: number;
    private selectedNodes: CanvasNode[] = [];
    private lastHovered?: number;

    constructor(
        private readonly viewport: CanvasViewport,
        canSelect: () => boolean,
        select: (id: number, openSource: boolean, extend: boolean) => void,
    ) {
        const hit = (event: MouseEvent) => this.current && canSelect() ? this.hit(this.point(event)) : undefined;
        const pick = (event: MouseEvent, openSource: boolean) => {
            const node = event.altKey && !openSource && this.current && canSelect()
                ? this.overlap(this.point(event)) : hit(event);
            if (node) select(node.id, openSource, !openSource && (event.ctrlKey || event.metaKey || event.shiftKey));
        };
        this.element.addEventListener('click', event => pick(event, false));
        this.element.addEventListener('dblclick', event => pick(event, true));
        this.element.addEventListener('pointermove', event => {
            const node = event.buttons ? undefined : hit(event);
            const id = node?.id === this.selected ? undefined : node?.id;
            if (id !== this.lastHovered) {
                this.lastHovered = id;
                this.draw(this.hover, id === undefined ? undefined : node?.bounds);
            }
        });
        this.element.addEventListener('pointerleave', () => this.clearHover());
        viewport.element.addEventListener('pointerdown', () => this.clearHover());
    }

    async decode(image: NonNullable<CanvasSnapshot['image']>): Promise<ImageBitmap | null> {
        // Image element decoding can wait for a paint in an occluded VS Code window.
        // Decode the PNG independently of document visibility. Null reuses the displayed bitmap.
        if (image.png === this.snapshot?.image?.png) return null;
        const png = atob(image.png);
        const bytes = new Uint8Array(png.length);
        for (let index = 0; index < png.length; index++) bytes[index] = png.charCodeAt(index);
        return createImageBitmap(new Blob([bytes], { type: 'image/png' }));
    }

    present(snapshot: CanvasSnapshot, viewport: Viewport, bitmap: ImageBitmap | null): void {
        if (!snapshot.image) return;
        this.snapshot = snapshot;
        if (bitmap) {
            if (this.frame.width !== bitmap.width) this.frame.width = bitmap.width;
            if (this.frame.height !== bitmap.height) this.frame.height = bitmap.height;
            // Replace transparent pixels as well as opaque ones in the same paint.
            this.presenter.globalCompositeOperation = 'copy';
            this.presenter.drawImage(bitmap, 0, 0);
            bitmap.close();
        }
        this.element.dataset.theme = viewport.dark ? 'dark' : 'light';
        this.element.hidden = false;
        this.placeholder.hidden = true;
        this.viewport.setImage(snapshot.image.width, snapshot.image.height);
        this.current = true;
        this.element.classList.remove('stale');
    }

    pause(): void {
        this.current = false;
        this.element.classList.add('stale');
        this.clearHover();
    }

    select(node: CanvasNode | undefined, additional: CanvasNode[] = []): void {
        if (node?.id !== this.selected) this.clearHover();
        this.selected = node?.id;
        this.selectedNodes = node ? [...additional, node] : [];
        this.draw(this.selection, node?.bounds);
        this.selectionGroup.replaceChildren(...additional.map(item => {
            const box = document.createElement('div');
            box.className = 'control-outline additional-outline';
            this.draw(box, item.bounds);
            return box;
        }));
    }

    movableSelection(anchor: number): number[] {
        return this.selectedNodes.some(node => node.id === anchor) && this.selectedNodes.every(node => node.canMove)
            ? this.selectedNodes.map(node => node.id) : [];
    }

    inspect(bounds?: Bounds | null): void {
        this.selected = undefined;
        this.selectedNodes = [];
        this.clearHover();
        this.draw(this.selection, bounds);
        this.selectionGroup.replaceChildren();
    }

    get hasFrame(): boolean { return !this.element.hidden; }
    get isCurrent(): boolean { return this.current; }
    get surface(): HTMLElement { return this.viewport.element; }

    point(event: MouseEvent, outside = false): Point | undefined {
        const image = this.snapshot?.image;
        if (!this.current || !image) return undefined;
        const rect = this.element.getBoundingClientRect();
        const x = (event.clientX - rect.left) * image.width / rect.width;
        const y = (event.clientY - rect.top) * image.height / rect.height;
        return outside || (x >= 0 && y >= 0 && x < image.width && y < image.height) ? { x, y } : undefined;
    }

    normalizedPoint(event: MouseEvent): Point | undefined {
        const point = this.point(event);
        const image = this.snapshot?.image;
        if (point && image) return { x: point.x / image.width, y: point.y / image.height };
    }

    hit(point?: Point): CanvasNode | undefined {
        if (!point) return undefined;
        return this.snapshot?.nodes.findLast(node => this.contains(node, point));
    }

    stepSelection(direction: number): CanvasNode | undefined {
        if (!this.current) return undefined;
        const available = this.snapshot?.nodes.filter(node => this.isSelectable(node)) ?? [];
        if (!available.length) return undefined;
        const index = available.findIndex(node => node.id === this.selected);
        return available[index === -1 ? (direction > 0 ? 0 : available.length - 1)
            : (index + direction + available.length) % available.length];
    }

    private overlap(point?: Point): CanvasNode | undefined {
        if (!point) return undefined;
        const hits = this.snapshot?.nodes.filter(node => this.contains(node, point)).reverse() ?? [];
        const index = hits.findIndex(node => node.id === this.selected);
        return hits[(index + 1) % hits.length];
    }

    private isSelectable(node: CanvasNode): node is CanvasNode & { bounds: Bounds } {
        return node.selectable !== false && !!node.bounds && node.bounds.width > 0 && node.bounds.height > 0;
    }

    private contains(node: CanvasNode, { x, y }: Point): boolean {
        if (!this.isSelectable(node)) return false;
        const bounds = node.bounds;
        return x >= bounds.x && y >= bounds.y && x < bounds.x + bounds.width && y < bounds.y + bounds.height;
    }

    showDrop(bounds?: Bounds | null): void {
        this.draw(this.dropBox, bounds, true);
    }

    private draw(element: HTMLElement, bounds: Bounds | null | undefined, allowLine = false): void {
        const image = this.snapshot?.image;
        element.hidden = true;
        if (!this.current || !image || !bounds || (!allowLine && (bounds.width <= 0 || bounds.height <= 0))) return;
        element.style.left = `${bounds.x * 100 / image.width}%`;
        element.style.top = `${bounds.y * 100 / image.height}%`;
        element.style.width = `${bounds.width * 100 / image.width}%`;
        element.style.height = `${bounds.height * 100 / image.height}%`;
        element.hidden = false;
    }

    private clearHover(): void { this.lastHovered = undefined; this.hover.hidden = true; }
}
