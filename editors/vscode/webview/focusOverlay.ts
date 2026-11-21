import type { CanvasFocusOrder, CanvasNode, Point } from '../src/protocol';
import { element } from './dom';

interface OverlayFrame { token: number; width: number; height: number; nodes: readonly CanvasNode[] }

export class FocusOverlay {
    private readonly layer = element('focus-order');
    readonly toggle = element<HTMLButtonElement>('menu-focus-order');
    private frame?: OverlayFrame;
    private requested?: number;
    private displayed?: number;
    enabled = false;

    constructor(private readonly request: (token: number) => void, private readonly reveal: (node: number) => void) {}

    setEnabled(enabled: boolean): void {
        this.enabled = enabled;
        this.toggle.setAttribute('aria-pressed', String(enabled));
    }

    update(available: boolean, selecting: boolean, frame?: OverlayFrame): void {
        this.frame = available && selecting ? frame : undefined;
        this.toggle.disabled = !available;
        this.layer.hidden = !this.enabled || !selecting || !available || this.displayed === undefined
            || (this.frame !== undefined && this.displayed !== this.frame.token);
        this.layer.inert = this.frame === undefined;
        if (this.enabled && this.frame && this.requested !== this.frame.token) {
            this.requested = this.frame.token;
            this.request(this.frame.token);
        }
    }

    show(token: number, overlay: CanvasFocusOrder): void {
        const frame = this.frame;
        if (!frame || frame.token !== token) {
            if (this.requested === token) this.requested = undefined;
            return;
        }
        const nodes = new Map(frame.nodes.map(node => [node.id, node]));
        const { width, height } = frame;
        const elements: HTMLElement[] = [];
        const addLine = (from: Point, to: Point, kind: 'tab' | 'label') => {
            const line = document.createElement('div');
            line.className = `focus-link focus-link-${kind}`;
            line.setAttribute('aria-hidden', 'true');
            place(line, from, frame);
            line.style.width = `${Math.hypot(to.x - from.x, to.y - from.y) / width * 100}%`;
            line.style.transform = `rotate(${Math.atan2(to.y - from.y, to.x - from.x)}rad)`;
            elements.push(line);
        };
        const marker = (node: number, point: Point, title: string, text: string, kind: 'tab' | 'group' | 'label') => {
            const button = document.createElement('button');
            button.type = 'button';
            button.className = `focus-marker focus-marker-${kind}`;
            button.textContent = text;
            button.title = title;
            button.setAttribute('aria-label', title);
            button.disabled = !nodes.has(node) || nodes.get(node)?.selectable === false;
            place(button, point, frame);
            button.addEventListener('pointerdown', event => event.stopPropagation());
            button.addEventListener('dblclick', event => event.stopPropagation());
            button.addEventListener('click', event => {
                event.stopPropagation();
                if (this.frame?.token === token && this.enabled) this.reveal(node);
            });
            elements.push(button);
        };
        const positions = overlay.badges.map(({ bounds, groupEntry }) => {
            if (bounds.x + bounds.width <= 0 || bounds.y + bounds.height <= 0 || bounds.x >= width || bounds.y >= height) return undefined;
            const x = groupEntry ? Math.max(0, Math.min(width, bounds.x + bounds.width) - 3)
                : Math.min(width - 1, Math.max(0, bounds.x) + 3);
            return { x, y: Math.min(height - 1, Math.max(0, bounds.y) + 3) };
        });
        for (const [a, b] of overlay.links) {
            if (positions[a] && positions[b]) addLine(positions[a], positions[b], 'tab');
        }
        for (const link of overlay.labels) addLine(link.from, link.to, 'label');
        for (const [i, badge] of overlay.badges.entries()) {
            if (positions[i]) marker(badge.node, positions[i], `Focus group ${badge.group}, step ${badge.rank}: ${badge.label}`,
                `${badge.group}.${badge.rank}`, badge.groupEntry ? 'group' : 'tab');
        }
        for (const link of overlay.labels) {
            if (link.from.x < 0 || link.from.y < 0 || link.from.x >= width || link.from.y >= height) continue;
            const target = nodes.get(link.target);
            marker(link.node, link.from, `${link.label} labels ${target?.name ?? target?.label ?? 'the linked control'}`, '↗', 'label');
        }
        this.layer.replaceChildren(...elements);
        this.displayed = token;
        this.layer.hidden = !this.enabled || elements.length === 0;
    }
}

function place(element: HTMLElement, point: Point, frame: OverlayFrame): void {
    element.style.left = `${point.x / frame.width * 100}%`;
    element.style.top = `${point.y / frame.height * 100}%`;
}
