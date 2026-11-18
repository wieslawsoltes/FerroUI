import type { PropertyField, Range } from '../src/protocol';
import { create } from './dom';

interface Actions {
    set(value: string): void;
    reset(): void;
    reveal(range: Range): void;
}

export function propertyEditor(property: PropertyField, actions: Actions): HTMLElement {
    const row = create('form');
    row.className = 'property';
    row.dataset.attribute = property.name;
    const label = create('label');
    label.textContent = property.name === 'HorizontalAlignment' ? 'Align horizontal'
        : property.name === 'VerticalAlignment' ? 'Align vertical' : property.name;
    label.title = property.origin ? `${property.name} · ${property.origin}` : property.name;
    const value = create('div');
    value.className = 'property-value';
    const initial = property.authored ?? '';
    const effective = property.effective ?? '';
    const input = inputFor(property, initial, effective);
    input.id = `property-${property.name}`;
    label.htmlFor = input.id;
    input.dataset.property = property.name;
    input.setAttribute('aria-label', property.name);
    value.append(input);

    if (property.kind === 'color' && property.editable) {
        const pick = create('input');
        pick.type = 'color';
        pick.className = 'property-color';
        pick.setAttribute('aria-label', `Choose ${property.name} color`);
        const hex = asHex(initial) ?? asHex(effective);
        pick.value = hex ?? '#000000';
        pick.toggleAttribute('data-unset', !hex);
        pick.addEventListener('change', () => actions.set(pick.value.toUpperCase()));
        value.prepend(pick);
    }
    if (property.canReset) {
        value.append(actionButton('discard', `Reset ${property.name}`, actions.reset));
    } else if (property.range && !property.editable) {
        const range = property.range;
        value.append(actionButton('go-to-file', `Reveal ${property.name} in source`, () => actions.reveal(range)));
    }
    if (!property.editable) row.classList.add('read-only');
    const commit = () => {
        if (property.editable && input.value !== initial) actions.set(input.value);
    };
    row.addEventListener('submit', event => { event.preventDefault(); commit(); });
    input.addEventListener('change', commit);
    row.append(label, value);
    return row;
}

function inputFor(property: PropertyField, initial: string, effective: string): HTMLInputElement | HTMLSelectElement {
    const options = property.kind === 'boolean' ? ['True', 'False'] : property.options;
    if (property.editable && options.length) {
        const select = create('select');
        const unset = new Option(effective || 'Unset', '');
        unset.disabled = true;
        select.append(unset, ...options.map(value => new Option(value)));
        const canonical = options.find(option => option.toLowerCase() === initial.toLowerCase());
        if (initial && !canonical) select.append(new Option(initial));
        select.value = canonical ?? initial;
        return select;
    }
    const input = create('input');
    input.readOnly = !property.editable;
    input.value = property.editable ? initial : initial || effective || property.origin || '';
    input.placeholder = effective === 'NaN' ? 'Auto' : effective || 'Unset';
    input.title = [property.authored ?? property.effective, property.origin].filter(Boolean).join(' · ');
    input.spellcheck = false;
    if (property.kind === 'number') input.inputMode = 'decimal';
    return input;
}

function asHex(value: string): string | undefined {
    if (/^#[0-9a-f]{6}$/i.test(value)) return value;
    if (/^#[0-9a-f]{8}$/i.test(value)) return '#' + value.slice(3);
    return undefined;
}

export function actionButton(icon: string, label: string, invoke: () => void): HTMLButtonElement {
    const button = create('button');
    button.type = 'button';
    button.className = 'row-action';
    button.title = label;
    button.setAttribute('aria-label', label);
    const glyph = create('span');
    glyph.className = `codicon codicon-${icon}`;
    glyph.setAttribute('aria-hidden', 'true');
    button.append(glyph);
    button.addEventListener('click', invoke);
    return button;
}
