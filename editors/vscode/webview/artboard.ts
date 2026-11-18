import type { Bounds, CanvasNode, CanvasSnapshot, Viewport } from '../src/protocol';
import { CanvasViewport } from './canvasViewport';
import { element } from './dom';

export class Artboard {
    private readonly board = element('artboard');
    private readonly placeholder = element('placeholder');
    private readonly frame = element<HTMLImageElement>('frame');
    private readonly selection = element('selection-box');
    private readonly hover = element('hover-box');
    private snapshot?: CanvasSnapshot;
    private current = false;
    private selected?: number;
    private lastHovered?: number;

    constructor(
        private readonly viewport: CanvasViewport,
        canSelect: () => boolean,
        select: (id: number, openSource: boolean) => void,
    ) {
        const hit = (event: MouseEvent) => this.current && canSelect() ? this.hit(event) : undefined;
        const pick = (event: MouseEvent, openSource: boolean) => {
            const node = hit(event);
            if (node) select(node.id, openSource);
        };
        this.frame.addEventListener('click', event => pick(event, false));
        this.frame.addEventListener('dblclick', event => pick(event, true));
        this.frame.addEventListener('pointermove', event => {
            const node = hit(event);
            const id = node?.id === this.selected ? undefined : node?.id;
            if (id !== this.lastHovered) {
                this.lastHovered = id;
                this.draw(this.hover, id === undefined ? undefined : node?.bounds);
            }
        });
        this.frame.addEventListener('pointerleave', () => this.clearHover());
        viewport.element.addEventListener('pointerdown', () => this.clearHover());
    }

    show(snapshot: CanvasSnapshot, viewport: Viewport): void {
        this.snapshot = snapshot;
        if (snapshot.image) {
            const image = snapshot.image;
            this.frame.src = `data:image/png;base64,${image.png}`;
            this.frame.width = image.width;
            this.frame.height = image.height;
            this.board.dataset.theme = viewport.dark ? 'dark' : 'light';
            this.board.hidden = false;
            this.placeholder.hidden = true;
            this.viewport.setImage(image.width, image.height);
        }
        this.fresh(Boolean(snapshot.image));
    }

    fresh(current: boolean): void {
        this.current = current;
        this.board.classList.toggle('stale', !current);
        this.clearHover();
        if (!current) this.selection.hidden = true;
    }

    select(node: CanvasNode | undefined): void {
        if (node?.id !== this.selected) this.clearHover();
        this.selected = node?.id;
        this.draw(this.selection, node?.bounds);
    }

    get hasFrame(): boolean { return !this.board.hidden; }

    private hit(event: MouseEvent): CanvasNode | undefined {
        const image = this.snapshot?.image;
        if (!this.snapshot || !image) return undefined;
        const rect = this.frame.getBoundingClientRect();
        const x = (event.clientX - rect.left) * image.width / rect.width;
        const y = (event.clientY - rect.top) * image.height / rect.height;
        return this.snapshot.nodes.findLast(({ bounds: b }) => b && b.width > 0 && b.height > 0
            && x >= b.x && y >= b.y && x < b.x + b.width && y < b.y + b.height);
    }

    private draw(element: HTMLElement, bounds: Bounds | null | undefined): void {
        const image = this.snapshot?.image;
        element.hidden = true;
        if (!this.current || !image || !bounds || bounds.width <= 0 || bounds.height <= 0) return;
        element.style.left = `${bounds.x * 100 / image.width}%`;
        element.style.top = `${bounds.y * 100 / image.height}%`;
        element.style.width = `${bounds.width * 100 / image.width}%`;
        element.style.height = `${bounds.height * 100 / image.height}%`;
        element.hidden = false;
    }

    private clearHover(): void { this.lastHovered = undefined; this.hover.hidden = true; }
}
