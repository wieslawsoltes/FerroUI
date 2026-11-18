import type { NodeOperation, OutlineNode, Property } from '../src/protocol';
import { choices, create, element } from './dom';

export class Inspector {
    private readonly target = element('properties');
    private readonly selection = element('element-properties');
    private readonly filter = element<HTMLInputElement>('filter');
    private selectedKey?: string;
    private contentKey?: string;
    private inputs = new Map<string, HTMLInputElement>();

    constructor(
        private readonly edit: (operation: NodeOperation) => void,
        private readonly openToolbox: () => void,
    ) {
        this.filter.addEventListener('input', () => this.applyFilter());
    }

    show(session: number | undefined, node: OutlineNode | undefined, enabled: boolean): void {
        this.selection.hidden = !node;
        this.target.toggleAttribute('inert', !enabled);
        if (!node) {
            this.selectedKey = undefined;
            this.contentKey = undefined;
            this.target.replaceChildren();
            return;
        }
        const selectedKey = `${session}:${node.id}`;
        const changedSelection = selectedKey !== this.selectedKey;
        this.selectedKey = selectedKey;
        const contentKey = JSON.stringify([selectedKey, node.label, node.name, node.properties,
            node.addableProperties, node.canInsert, node.canRemove]);
        if (this.contentKey === contentKey) return;
        this.contentKey = contentKey;
        if (changedSelection) this.filter.value = '';
        element('control-type').textContent = node.label;
        const name = element('control-name');
        name.textContent = node.name ?? '';
        name.hidden = !node.name;
        element('filter-row').hidden = node.properties.length < 5 && !this.filter.value;

        const focus = document.activeElement instanceof HTMLInputElement ? document.activeElement.dataset.property : undefined;
        this.inputs.clear();
        const form = create('div');
        form.className = 'property-list';
        for (const property of node.properties) form.append(this.property(node.id, property));
        if (!node.properties.length) {
            const hint = create('p', 'No authored properties.');
            hint.className = 'hint';
            form.append(hint);
        } else {
            const empty = create('p', 'No matching properties.');
            empty.className = 'hint';
            empty.id = 'filter-empty';
            form.append(empty);
        }
        if (node.addableProperties.length) form.append(this.addProperty(node));
        if (node.canInsert || node.canRemove) form.append(this.structure(node));
        this.target.replaceChildren(form);
        this.applyFilter();
        if (focus && enabled && !changedSelection) this.inputs.get(focus)?.focus();
    }

    private applyFilter(): void {
        const needle = this.filter.value.trim().toLowerCase();
        const rows = this.target.querySelectorAll<HTMLElement>('[data-attribute]');
        let matching = 0;
        for (const row of rows) {
            row.hidden = !(row.dataset.attribute ?? '').toLowerCase().includes(needle);
            if (!row.hidden) matching++;
        }
        const empty = document.getElementById('filter-empty');
        if (empty) empty.hidden = matching > 0;
    }

    private property(id: number, property: Property): HTMLElement {
        const row = create('form');
        row.className = 'property';
        row.dataset.attribute = property.name;
        const label = create('label', property.name);
        label.title = property.name;
        const input = create('input');
        input.id = `property-${this.inputs.size}`;
        label.htmlFor = input.id;
        input.dataset.property = property.name;
        input.value = property.value;
        input.readOnly = !property.editable;
        const commit = () => {
            if (property.editable && input.value !== property.value) {
                this.edit({ type: 'setProperty', node: id, name: property.name, value: input.value });
            }
        };
        row.addEventListener('submit', event => { event.preventDefault(); commit(); });
        input.addEventListener('change', commit);
        row.append(label, input);
        this.inputs.set(property.name, input);
        return row;
    }

    private addProperty(node: OutlineNode): HTMLElement {
        const form = create('form');
        form.className = 'add-property';
        form.setAttribute('aria-label', 'Add property');
        const property = choices(node.addableProperties, 'New property');
        const value = create('input');
        value.setAttribute('aria-label', 'Property value');
        value.placeholder = 'Value';
        const add = create('button', 'Set');
        add.type = 'submit';
        add.className = 'primary';
        form.append(property, value, add);
        form.addEventListener('submit', event => {
            event.preventDefault();
            this.edit({ type: 'setProperty', node: node.id, name: property.value, value: value.value });
        });
        return form;
    }

    private structure(node: OutlineNode): HTMLElement {
        const row = create('div');
        row.className = 'structure';
        if (node.canInsert) {
            const button = create('button', 'Open Toolbox');
            button.className = 'secondary';
            button.addEventListener('click', this.openToolbox);
            row.append(button);
        }
        if (node.canRemove) {
            const remove = create('button', 'Remove');
            remove.className = 'secondary';
            remove.addEventListener('click', () => this.edit({ type: 'removeNode', node: node.id }));
            row.append(remove);
        }
        return row;
    }
}
