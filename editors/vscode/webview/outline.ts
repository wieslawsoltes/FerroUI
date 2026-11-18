import type { OutlineNode } from '../src/protocol';
import { create, element } from './dom';

export class Outline {
    private readonly target = element('outline');
    private readonly rows = new Map<number, HTMLButtonElement>();

    constructor(private readonly selectNode: (id: number) => void) {}

    show(nodes: OutlineNode[]): void {
        this.rows.clear();
        const depth = new Map<number, number>();
        const list = document.createDocumentFragment();
        for (const node of nodes) {
            const button = create('button');
            const level = node.parent === null ? 0 : 1 + (depth.get(node.parent) ?? 0);
            depth.set(node.id, level);
            button.type = 'button';
            button.className = 'outline-row';
            button.style.setProperty('--indent', String(Math.min(level, 12)));
            button.append(create('span', node.label));
            if (node.name) {
                const name = create('span', node.name);
                name.className = 'node-name';
                button.append(name);
            }
            button.addEventListener('click', () => this.selectNode(node.id));
            list.append(button);
            this.rows.set(node.id, button);
        }
        this.target.replaceChildren(list);
    }

    select(id: number | undefined, enabled: boolean): void {
        for (const [key, row] of this.rows) {
            row.setAttribute('aria-pressed', String(key === id));
            row.disabled = !enabled;
        }
    }
}
