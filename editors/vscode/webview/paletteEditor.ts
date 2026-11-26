import { canvasThemeLabels, type CanvasTheme } from '../src/protocol';
import type { PaletteColumn, PaletteComparisonRow, PaletteSheet, ResolvedColor, StylesAction } from '../src/stylesProtocol';
import { create } from './dom';
import { actionButton } from './propertyEditor';
import { submitValue } from './valueField';
import { colorPicker, swatch } from './colorSwatch';

export function paletteEditor(sheet: PaletteSheet | undefined, layer: PaletteColumn | undefined,
    compare: boolean, filter: string, send: (action: StylesAction) => void): HTMLElement {
    if (compare && sheet) return paletteComparison(sheet.comparison, sheet.contrastPreview ? ['light', 'dark', 'high_contrast'] : ['light', 'dark'],
        filter, (_color, key, theme) => send({ type: 'color-source', key, theme }));
    const root = create('div');
    if (!layer || layer.unavailable || !layer.colors.length) {
        const empty = create('p');
        empty.className = 'style-empty';
        empty.textContent = layer?.unavailable ?? 'No colors in this layer.';
        root.append(empty);
        return root;
    }
    for (const color of layer.colors.filter(color => color.key.toLowerCase().includes(filter))) {
        root.append(paletteColorRow({ key: color.key, id: `palette-value-${color.node}`, value: color.authored,
            color: color.color, description: color.kind }, {
            change: color.editable ? value => send({ type: 'palette-color', node: color.node, color: value }) : undefined,
            references: color.uses ? () => send({ type: 'references', node: color.node }) : undefined,
            reveal: () => send({ type: 'reveal', range: color.range }),
        }));
    }
    return root;
}

export function paletteColorRow(color: { key: string; id: string; value?: string; color?: string; description: string },
    actions: { change?: (value: string) => void; reveal: () => void; references?: () => void; edit?: { label: string; apply: () => void } }): HTMLElement {
    const row = create('div');
    row.className = 'palette-row';
    const title = create('label');
    title.textContent = title.title = color.key;
    const value = create('input');
    value.id = color.id;
    title.htmlFor = value.id;
    value.setAttribute('aria-label', `${color.key} color value`);
    value.readOnly = !actions.change;
    value.spellcheck = false;
    value.value = color.value ?? '';
    value.title = value.value || color.description;
    if (actions.change) submitValue(value, value.value, actions.change);
    const sample = actions.change ? colorPicker(color.key, color.color, actions.change) : swatch(color.color);
    const controls = create('div');
    controls.className = 'style-tools';
    if (actions.edit) controls.append(actionButton('edit', actions.edit.label, actions.edit.apply));
    if (actions.references) controls.append(actionButton('references', `Find ${color.key} references`, actions.references));
    controls.append(actionButton('go-to-file', `Reveal ${color.key} in source`, actions.reveal));
    row.append(sample, title, value, controls);
    return row;
}

type RevealColor = (color: ResolvedColor, key: string, theme: CanvasTheme) => void;

export function paletteComparison(rows: readonly PaletteComparisonRow[], themes: readonly CanvasTheme[], filter: string, reveal: RevealColor): HTMLElement {
    const container = create('div');
    container.className = 'palette-comparison-scroll';
    const table = create('table');
    table.className = 'palette-comparison';
    table.setAttribute('aria-label', 'Scoped resource resolution by theme');
    const header = table.createTHead().insertRow();
    for (const title of ['Color', ...themes.map(theme => canvasThemeLabels[theme])]) {
        const cell = create('th');
        cell.scope = 'col';
        cell.textContent = title;
        header.append(cell);
    }
    const body = table.createTBody();
    for (const color of rows.filter(color => color.key.toLowerCase().includes(filter))) {
        const row = body.insertRow();
        const key = create('th');
        key.scope = 'row';
        key.textContent = key.title = color.key;
        row.append(key);
        for (const theme of themes) {
            appendResolved(row.insertCell(), color[theme], color.key, theme, reveal);
        }
    }
    container.append(table);
    return container;
}

function appendResolved(cell: HTMLTableCellElement, color: ResolvedColor | null | undefined,
    key: string, theme: CanvasTheme, open: RevealColor): void {
    const content = create('div');
    const value = color?.value ?? (color && !color.issue ? 'Expression' : 'Unresolved');
    const path = color?.path?.map(step => step.reference ? `${step.reference} ${step.key}` : step.key).join(' → ');
    const details = [value, path, color?.scope, color?.issue].filter(Boolean).join(' · ');
    content.title = details;
    const sample = create('div');
    sample.className = 'palette-sample';
    sample.append(swatch(color?.value ?? undefined));
    if (color?.source || color?.range) {
        const reveal = actionButton('go-to-file', `Open ${canvasThemeLabels[theme]} declaration for ${key}`,
            () => open(color, key, theme));
        reveal.title += ` · ${details}`;
        reveal.setAttribute('aria-description', details);
        sample.append(reveal);
    }
    const label = create('span');
    label.className = 'palette-comparison-value';
    label.textContent = value;
    content.append(sample, label);
    cell.append(content);
}
