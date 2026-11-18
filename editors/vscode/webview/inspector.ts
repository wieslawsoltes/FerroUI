import type { Operation, OutlineNode, Property } from '../src/protocol';
import { choices, create, element } from './dom';

export class Inspector {
    private readonly target = element('properties');
    private selected?: OutlineNode;
    private inputs = new Map<string, HTMLInputElement>();

    constructor(private readonly edit: (operation: Operation) => void) {}

    show(node: OutlineNode | undefined, enabled: boolean): void {
        if (this.selected === node && this.target.childNodes.length) {
            this.target.toggleAttribute('inert', !enabled);
            return;
        }
        const focus = document.activeElement instanceof HTMLInputElement ? document.activeElement.dataset.property : undefined;
        this.inputs.clear();
        this.selected = node;
        this.target.toggleAttribute('inert', !enabled);
        if (!node) {
            const hint = create('p', 'Select a control on the canvas or in the outline.');
            hint.className = 'hint';
            this.target.replaceChildren(hint);
            return;
        }
        const form = create('div');
        form.className = 'property-list';
        for (const property of node.properties) form.append(this.property(node.id, property));
        if (!node.properties.length) {
            const hint = create('p', 'This control has no authored attributes.');
            hint.className = 'hint';
            form.append(hint);
        }
        if (node.addableProperties.length) form.append(this.addProperty(node));
        if (node.canInsert || node.canRemove) form.append(this.structure(node));
        this.target.replaceChildren(form);
        if (focus && enabled) this.inputs.get(focus)?.focus();
    }

    private property(id: number, property: Property): HTMLElement {
        const row = create('form');
        row.className = 'property';
        const label = create('label', property.name);
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
            const control = choices(['TextBlock', 'Button', 'Border', 'StackPanel'], 'Control to insert');
            const button = create('button', 'Insert child');
            button.className = 'secondary';
            button.addEventListener('click', () => this.edit({ type: 'insertChild', node: node.id, control: control.value }));
            row.append(control, button);
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
