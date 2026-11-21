import { canvasThemeLabels, type CanvasTheme } from '../src/protocol';
import type { PaletteColumn, PaletteColor, PaletteSheet, ResolvedColor, StylesAction } from '../src/stylesProtocol';
import { create } from './dom';
import { actionButton } from './propertyEditor';
import { submitValue } from './styleDefinitionEditor';

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
    const sample = color.editable ? colorPicker(color, set) : swatch(color.color);
    const controls = create('div');
    controls.className = 'style-tools';
    if (color.uses) controls.append(actionButton('references', `Find ${color.key} references`,
        () => send({ type: 'references', node: color.node })));
    controls.append(actionButton('go-to-file', `Reveal ${color.key} in source`, () => send({ type: 'reveal', range: color.range })));
    row.append(sample, title, value, controls);
    return row;
}

function colorPicker(color: PaletteColor, set: (value: string) => void): HTMLInputElement {
    const input = create('input');
    input.type = 'color';
    input.className = 'palette-picker';
    input.value = color.color ? `#${color.color.slice(3)}` : '#000000';
    input.title = `Choose ${color.key} color`;
    input.setAttribute('aria-label', input.title);
    input.addEventListener('change', () => set(`#${color.color?.slice(1, 3) ?? 'FF'}${input.value.slice(1)}`));
    return input;
}

function swatch(argb: string | undefined): HTMLElement {
    const result = create('span');
    result.className = 'palette-swatch';
    result.setAttribute('aria-hidden', 'true');
    if (argb && /^#[0-9A-F]{8}$/i.test(argb)) result.style.backgroundColor = `#${argb.slice(3)}${argb.slice(1, 3)}`;
    return result;
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
    const value = color?.value ?? (color ? 'Expression' : 'Unresolved');
    content.title = [value, color?.scope].filter(Boolean).join(' · ');
    const sample = create('div');
    sample.className = 'palette-sample';
    sample.append(swatch(color?.value ?? undefined));
    if (color?.source) {
        sample.append(actionButton('go-to-file', `Open ${canvasThemeLabels[theme]} declaration for ${key}`,
            () => send({ type: 'included-color', key, theme })));
    }
    const label = create('span');
    label.textContent = value;
    content.append(sample, label);
    cell.append(content);
}
