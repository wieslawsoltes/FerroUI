import { canvasThemeLabels, type CanvasTheme } from '../src/protocol';
import type { PaletteColumn, PaletteColor, PaletteSheet, ResolvedColor, StylesAction } from '../src/stylesProtocol';
import { create } from './dom';
import { actionButton } from './propertyEditor';
import { submitValue } from './styleDefinitionEditor';
import { colorPicker, swatch } from './colorSwatch';

export function paletteEditor(sheet: PaletteSheet | undefined, layer: PaletteColumn | undefined,
    compare: boolean, filter: string, send: (action: StylesAction) => void): HTMLElement {
    if (compare && sheet) return comparison(sheet, filter, send);
    const root = create('div');
    if (!layer || layer.unavailable || !layer.colors.length) {
        const empty = create('p');
        empty.className = 'style-empty';
        empty.textContent = layer?.unavailable ?? 'No colors in this layer.';
        root.append(empty);
        return root;
    }
    for (const color of layer.colors.filter(color => color.key.toLowerCase().includes(filter))) {
        root.append(colorRow(color, send));
    }
    return root;
}

function colorRow(color: PaletteColor, send: (action: StylesAction) => void): HTMLElement {
    const row = create('div');
    row.className = 'palette-row';
    const title = create('label');
    title.textContent = title.title = color.key;
    const value = create('input');
    value.id = `palette-value-${color.node}`;
    title.htmlFor = value.id;
    value.setAttribute('aria-label', `${color.key} color value`);
    value.readOnly = !color.editable;
    value.spellcheck = false;
    value.value = color.authored ?? '';
    value.title = value.value || color.kind;
    const set = (value: string) => send({ type: 'palette-color', node: color.node, color: value });
    if (color.editable) submitValue(value, value.value, set);
    const sample = color.editable ? colorPicker(color.key, color.color, set) : swatch(color.color);
    const controls = create('div');
    controls.className = 'style-tools';
    if (color.uses) controls.append(actionButton('references', `Find ${color.key} references`,
        () => send({ type: 'references', node: color.node })));
    controls.append(actionButton('go-to-file', `Reveal ${color.key} in source`, () => send({ type: 'reveal', range: color.range })));
    row.append(sample, title, value, controls);
    return row;
}

function comparison(sheet: PaletteSheet, filter: string, send: (action: StylesAction) => void): HTMLElement {
    const themes: CanvasTheme[] = sheet.contrastPreview ? ['light', 'dark', 'high_contrast'] : ['light', 'dark'];
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
    for (const color of sheet.comparison.filter(color => color.key.toLowerCase().includes(filter))) {
        const row = body.insertRow();
        const key = create('th');
        key.scope = 'row';
        key.textContent = key.title = color.key;
        row.append(key);
        for (const theme of themes) {
            appendResolved(row.insertCell(), color[theme], color.key, theme, send);
        }
    }
    container.append(table);
    return container;
}

function appendResolved(cell: HTMLTableCellElement, color: ResolvedColor | null | undefined,
    key: string, theme: CanvasTheme, send: (action: StylesAction) => void): void {
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
            () => send({ type: 'color-source', key, theme }));
        reveal.title += ` · ${details}`;
        reveal.setAttribute('aria-description', details);
        sample.append(reveal);
    }
    const label = create('span');
    label.textContent = value;
    content.append(sample, label);
    cell.append(content);
}
