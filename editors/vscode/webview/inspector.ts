import type { Bounds, NodeOperation, PropertiesState, PropertyAuthoring, PropertyField, Range } from '../src/protocol';
import { create, element } from './dom';
import { propertyEditor } from './propertyEditor';

const common = [
    ['Position and size', ['Canvas.Left', 'Canvas.Top', 'Grid.Row', 'Grid.Column', 'Width', 'Height']],
    ['Layout', ['HorizontalAlignment', 'VerticalAlignment', 'Orientation', 'Direction', 'Spacing', 'Margin', 'Padding']],
    ['Content', ['Content', 'Header', 'Text', 'Source', 'FontSize', 'Foreground']],
    ['Appearance', ['Background', 'Fill', 'Stroke', 'BorderBrush', 'StrokeThickness', 'BorderThickness', 'CornerRadius', 'Opacity', 'IsVisible']],
] as const;

export class Inspector {
    private readonly target = element('properties');
    private readonly selection = element('element-properties');
    private readonly filter = element<HTMLInputElement>('filter');
    private readonly metrics = element('property-metrics');
    private selectedKey?: string;
    private contentKey?: string;
    private readonly expanded = new Set<string>();

    constructor(
        private readonly edit: (operation: NodeOperation) => void,
        private readonly openToolbox: () => void,
        private readonly reveal: (range: Range) => void,
        private readonly author: (property: string, action: PropertyAuthoring) => void,
    ) {
        this.filter.addEventListener('input', () => this.applyFilter());
    }

    show(state: PropertiesState, enabled: boolean): void {
        const { node, runtime, fields } = state;
        const group = (state.selectionCount ?? 0) > 1;
        const label = group ? `${state.selectionCount} controls` : runtime?.type ?? node?.label;
        this.selection.hidden = !label;
        // A source range can still be revealed on a read-only row.
        this.target.toggleAttribute('inert', !state.frame || Boolean(node && !enabled));
        if (!label) {
            this.selectedKey = this.contentKey = undefined;
            this.target.replaceChildren();
            return;
        }
        const key = `${state.session}:${runtime ? 'runtime' : 'source'}:${node?.id ?? runtime?.inspector_id}`;
        const changedSelection = key !== this.selectedKey;
        this.selectedKey = key;
        if (changedSelection) this.filter.value = '';
        element('control-type').textContent = label;
        const name = element('control-name');
        name.textContent = group ? '' : runtime ? [runtime.name, 'Runtime'].filter(Boolean).join(' · ') : node?.name ?? '';
        name.hidden = !name.textContent;
        this.showMetrics(group ? undefined : runtime?.bounds ?? node?.bounds);
        element('filter-row').hidden = !fields?.length;

        // Keep the existing rows mounted across an edit and its rerender. The host frame gate
        // keeps stale fields inert while the next inspection is being fetched.
        if (!fields && !changedSelection) return;
        const contentKey = JSON.stringify([key, fields, node?.canInsert, node?.canRemove]);
        if (this.contentKey === contentKey) return;
        this.contentKey = contentKey;
        const focus = (document.activeElement as HTMLElement | null)?.dataset.property;
        const byName = new Map(fields?.map(field => [field.name, field]));
        const sections: HTMLElement[] = [];
        for (const [group, names] of common) {
            const visible = names.flatMap(name => {
                const field = byName.get(name);
                byName.delete(name);
                return field ? [field] : [];
            });
            if (visible.length) sections.push(this.commonSection(group, visible, node?.id));
        }
        const groups = new Map<string, PropertyField[]>();
        for (const field of byName.values()) {
            let group = groups.get(field.group);
            if (!group) groups.set(field.group, group = []);
            group.push(field);
        }
        if (groups.size) {
            const more = create('h3', node ? 'More properties' : 'Runtime properties');
            more.className = 'properties-more';
            sections.push(more);
            for (const [group, members] of [...groups].sort(([a], [b]) => a.localeCompare(b))) {
                sections.push(this.detail(group, members, node?.id));
            }
        }
        if (fields?.length) {
            const empty = create('p', 'No matching properties.');
            empty.id = 'filter-empty';
            empty.className = 'hint';
            empty.hidden = true;
            sections.push(empty);
        }
        if (node && !group && (node.canInsert || node.canRemove)) sections.push(this.structure(node.id, node.canInsert, node.canRemove));
        this.target.replaceChildren(...sections);
        this.applyFilter();
        if (focus && enabled && !changedSelection) {
            this.target.querySelector<HTMLElement>(`[data-property="${CSS.escape(focus)}"]`)?.focus({ preventScroll: true });
        }
    }

    private showMetrics(bounds?: Bounds | null): void {
        this.metrics.hidden = !bounds;
        if (!bounds) return;
        for (const output of this.metrics.querySelectorAll<HTMLElement>('[data-metric]')) {
            const value = bounds[output.dataset.metric as keyof Bounds];
            output.textContent = Number(value.toFixed(1)).toString();
        }
    }

    private commonSection(name: string, fields: PropertyField[], node?: number): HTMLElement {
        const box = create('section');
        box.className = 'property-section';
        box.dataset.section = name;
        const heading = create('h3', name);
        const list = create('div');
        list.className = name === 'Position and size' ? 'property-list size-grid' : 'property-list';
        list.append(...fields.map(field => this.field(field, node)));
        box.append(heading, list);
        return box;
    }

    private detail(name: string, fields: PropertyField[], node?: number): HTMLElement {
        const detail = create('details');
        detail.className = 'property-section extra-properties';
        detail.dataset.section = name;
        detail.open = this.expanded.has(name) || node === undefined;
        const summary = create('summary', name);
        summary.addEventListener('click', () => {
            if (detail.open) this.expanded.delete(name);
            else this.expanded.add(name);
        });
        const rows = create('div');
        rows.className = 'property-list';
        rows.append(...fields.map(field => this.field(field, node)));
        detail.append(summary, rows);
        return detail;
    }

    private field(field: PropertyField, node?: number): HTMLElement {
        return propertyEditor(field, {
            set: value => { if (node !== undefined) this.edit({ type: 'setProperty', node, name: field.name, value }); },
            reset: () => { if (node !== undefined) this.edit({ type: 'resetProperty', node, name: field.name }); },
            reveal: this.reveal,
            binding: node !== undefined && field.canBind ? () => this.author(field.name, 'binding') : undefined,
            promote: node !== undefined && field.canPromote ? () => this.author(field.name, 'promoteResource') : undefined,
            brush: node !== undefined && field.canEditBrush ? () => this.author(field.name, 'brush') : undefined,
        });
    }

    private applyFilter(): void {
        const needle = this.filter.value.trim().toLowerCase();
        let matches = 0;
        for (const section of this.target.querySelectorAll<HTMLElement>('[data-section]')) {
            let visible = 0;
            for (const row of section.querySelectorAll<HTMLElement>('[data-attribute]')) {
                const haystack = `${section.dataset.section} ${row.dataset.attribute}`.toLowerCase();
                row.hidden = !haystack.includes(needle);
                if (!row.hidden) visible++;
            }
            section.hidden = visible === 0;
            matches += visible;
            if (section instanceof HTMLDetailsElement) section.open = Boolean(needle) || this.expanded.has(section.dataset.section ?? '') || section.dataset.section === 'Runtime';
        }
        const empty = document.getElementById('filter-empty');
        if (empty) empty.hidden = matches > 0 || !needle;
        const more = this.target.querySelector<HTMLElement>('.properties-more');
        if (more) more.hidden = !this.target.querySelector('details:not([hidden])');
    }

    private structure(node: number, canInsert: boolean, canRemove: boolean): HTMLElement {
        const row = create('div');
        row.className = 'structure';
        if (canInsert) {
            const button = create('button', 'Open Toolbox');
            button.className = 'secondary';
            button.addEventListener('click', this.openToolbox);
            row.append(button);
        }
        if (canRemove) {
            const remove = create('button', 'Remove control');
            remove.className = 'secondary';
            remove.addEventListener('click', () => this.edit({ type: 'removeNode', node }));
            row.append(remove);
        }
        return row;
    }
}
