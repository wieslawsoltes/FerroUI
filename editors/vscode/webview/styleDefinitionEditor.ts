import type { AuthoredSetter, StyleDefinition, StylesAction } from '../src/stylesProtocol';
import { create } from './dom';
import { actionButton } from './propertyEditor';

export function styleDefinition<Base extends string | number>(sheet: StyleDefinition<Base> | undefined,
    send: (action: StylesAction<Base>) => void, previewAvailable = true): HTMLElement {
    const root = create('div');
    if (!sheet) {
        const empty = create('p');
        empty.className = 'style-empty';
        empty.textContent = 'No styles or control themes in this file.';
        root.append(empty);
        return root;
    }
    const bar = create('div');
    bar.className = 'style-subheader';
    const kind = create('span');
    kind.textContent = sheet.kind === 'Style' ? 'Style' : sheet.target ?? 'Control theme';
    kind.title = kind.textContent;
    const tools = create('div');
    tools.className = 'style-tools';
    const add = actionButton('add', 'Add setter', () => send({ type: 'add-setter' }));
    add.disabled = !sheet.available.length;
    if (previewAvailable) tools.append(actionButton('preview', 'Preview definition on canvas', () => send({ type: 'preview-definition' })));
    tools.append(actionButton('go-to-file', 'Reveal definition in source', () => send({ type: 'reveal', range: sheet.range })), add);
    if (sheet.can_remove) tools.append(actionButton('trash', 'Remove style', () => send({ type: 'remove-style' })));
    bar.append(kind, tools);
    root.append(bar, styleIdentity(sheet, send));
    if (sheet.setters.length) {
        const list = create('div');
        list.className = 'style-setters';
        list.setAttribute('aria-label', 'Authored setters');
        for (const setter of sheet.setters) list.append(setterRow(setter, send));
        root.append(list);
    }
    return root;
}

function styleIdentity<Base extends string | number>(sheet: StyleDefinition<Base>, send: (action: StylesAction<Base>) => void): HTMLElement {
    const row = create('label');
    row.className = 'style-identity';
    if (sheet.identity.kind === 'base') {
        row.append('Based on');
        const choices = sheet.base_choices ?? [];
        const select = create('select');
        select.id = `style-base-${sheet.node}`;
        select.setAttribute('aria-label', 'Based on control theme');
        select.disabled = !sheet.identity.editable;
        select.append(new Option('None', ''));
        const selected = choices.findIndex(theme => theme.node === sheet.base);
        if (sheet.identity.value && selected < 0) {
            const source = new Option(sheet.identity.value, 'source');
            source.disabled = true;
            select.append(source);
        }
        for (const [index, theme] of choices.entries()) select.append(new Option(theme.name, String(index)));
        select.value = selected >= 0 ? String(selected) : sheet.identity.value ? 'source' : '';
        select.addEventListener('change', () => send({ type: 'base', base: select.value ? choices[Number(select.value)]?.node ?? null : null }));
        row.append(select);
    } else {
        row.append('Selector');
        const input = create('input');
        input.id = `style-selector-${sheet.node}`;
        input.spellcheck = false;
        input.readOnly = !sheet.identity.editable;
        input.setAttribute('aria-label', 'Style selector');
        input.value = sheet.identity.value ?? '';
        submitValue(input, input.value, value => send({ type: 'selector', value }));
        row.append(input);
    }
    return row;
}

function setterRow<Base extends string | number>(setter: AuthoredSetter, send: (action: StylesAction<Base>) => void): HTMLElement {
    const row = create('div');
    row.className = 'style-setter';
    const name = create('label');
    name.textContent = name.title = setter.property;
    const value = setter.value ?? '';
    const field = setter.editable && setter.field.options.length
        ? setterSelect(setter.field.options, value) : setterInput(setter, value);
    field.id = `setter-${setter.node}`;
    field.setAttribute('aria-label', `${setter.property} setter value`);
    name.htmlFor = field.id;
    if (setter.editable) submitValue(field, value, value => send({ type: 'setter', property: setter.property, value }));
    const buttons = create('div');
    buttons.className = 'style-tools';
    if (setter.field.resources) buttons.append(actionButton('link', `Assign resource to ${setter.property}`,
        () => send({ type: 'setter-resource', property: setter.property })));
    buttons.append(actionButton('go-to-file', `Reveal ${setter.property} setter`, () => send({ type: 'reveal', range: setter.range })),
        actionButton('discard', `Remove ${setter.property} setter`, () => send({ type: 'setter', property: setter.property, value: null })));
    row.append(name, field, buttons);
    return row;
}

function setterSelect(options: string[], value: string): HTMLSelectElement {
    const select = create('select');
    select.append(...options.map(option => new Option(option)));
    const canonical = options.find(option => option.toLowerCase() === value.toLowerCase());
    if (!canonical && value) select.append(new Option(value));
    select.value = canonical ?? value;
    return select;
}

function setterInput(setter: AuthoredSetter, value: string): HTMLInputElement {
    const field = create('input');
    field.readOnly = !setter.editable;
    field.spellcheck = false;
    field.value = value;
    field.title = value || (setter.object ? 'XAML object value' : setter.field.type_name ?? '');
    if (setter.object) field.placeholder = 'XAML object';
    if (setter.field.kind === 'number') field.inputMode = 'decimal';
    return field;
}

export function submitValue(field: HTMLInputElement | HTMLSelectElement, original: string, apply: (value: string) => void): void {
    let last = original;
    const commit = () => {
        if (field.value === last) return;
        last = field.value;
        apply(field.value);
    };
    field.addEventListener('change', commit);
    if (field instanceof HTMLInputElement) field.addEventListener('keydown', event => {
        if (event.key === 'Enter') { event.preventDefault(); commit(); }
        if (event.key === 'Escape') field.value = last;
    });
}
