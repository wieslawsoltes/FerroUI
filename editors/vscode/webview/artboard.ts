import type { OutlineNode, Snapshot } from '../src/protocol';
import { element } from './dom';

export class Artboard {
    private readonly holder = element('artboard');
    private readonly placeholder = element('placeholder');
    private readonly frame = element<HTMLImageElement>('frame');
    private readonly selection = element('selection-box');
    private snapshot?: Snapshot;
    private current = false;

    constructor(select: (id: number) => void) {
        this.frame.addEventListener('click', event => {
            if (!this.current || !this.snapshot?.image) return;
            const rect = this.frame.getBoundingClientRect();
            const x = (event.clientX - rect.left) * this.snapshot.image.width / rect.width;
            const y = (event.clientY - rect.top) * this.snapshot.image.height / rect.height;
            const hit = this.snapshot.nodes.findLast(({ bounds: b }) => b && b.width > 0 && b.height > 0
                && x >= b.x && y >= b.y && x < b.x + b.width && y < b.y + b.height);
            if (hit) select(hit.id);
        });
    }

    show(snapshot: Snapshot, dark: boolean): void {
        this.snapshot = snapshot;
        if (snapshot.image) {
            const image = snapshot.image;
            this.frame.src = `data:image/png;base64,${image.png}`;
            this.frame.width = image.width;
            this.frame.height = image.height;
            this.holder.style.setProperty('--frame-width', `${image.width}px`);
            this.holder.style.setProperty('--frame-aspect', String(image.width / image.height));
            this.holder.dataset.theme = dark ? 'dark' : 'light';
            this.holder.hidden = false;
            this.placeholder.hidden = true;
        }
        this.fresh(Boolean(snapshot.image));
    }

    fresh(current: boolean): void {
        this.current = current;
        this.holder.classList.toggle('stale', !current);
        if (!current) this.selection.hidden = true;
    }

    select(node: OutlineNode | undefined): void {
        const bounds = node?.bounds;
        const image = this.snapshot?.image;
        this.selection.hidden = true;
        if (!this.current || !image || !bounds || bounds.width <= 0 || bounds.height <= 0) return;
        this.selection.style.left = `${bounds.x * 100 / image.width}%`;
        this.selection.style.top = `${bounds.y * 100 / image.height}%`;
        this.selection.style.width = `${bounds.width * 100 / image.width}%`;
        this.selection.style.height = `${bounds.height * 100 / image.height}%`;
        this.selection.hidden = false;
    }

    get hasFrame(): boolean { return !this.holder.hidden; }
}
