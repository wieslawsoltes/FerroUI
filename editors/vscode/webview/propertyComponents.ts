import type { PropertyComponents } from '../src/protocol';
import { create, iconButton, labelButton } from './dom';

const decimal = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i;
const names: Record<string, string> = {
    L: 'left', T: 'top', R: 'right', B: 'bottom', W: 'width', H: 'height',
    TL: 'top left', TR: 'top right', BR: 'bottom right', BL: 'bottom left',
};

export function propertyComponents(name: string, model: PropertyComponents, mixed: boolean, commit: (value: string) => void) {
    const panel = create('form');
    panel.className = 'property-components';
    panel.id = `components-${name}`;
    panel.hidden = true;
    panel.setAttribute('role', 'group');
    panel.setAttribute('aria-label', `${name} components`);

    const fields = create('div');
    fields.className = 'component-fields';
    const labels = model.labels.map(part => {
        const label = create('label');
        label.title = names[part] ?? part;
        const caption = create('span', part);
        const input = create('input');
        input.inputMode = 'decimal';
        input.spellcheck = false;
        input.setAttribute('aria-label', `${name} ${label.title}`);
        input.placeholder = mixed ? 'Mixed' : 'Unset';
        input.addEventListener('input', () => input.removeAttribute('aria-invalid'));
        label.append(caption, input);
        fields.append(label);
        return { label, caption, input };
    });
    const inputs = labels.map(field => field.input);

    const actions = create('div');
    actions.className = 'component-actions';
    let linked = false;
    const link = model.linkable ? iconButton('link', `Link ${name} components`, () => setLink(!linked)) : undefined;
    if (link) {
        link.classList.add('component-link');
        actions.append(link);
    }
    const units = model.relative ? create('select') : undefined;
    if (units) {
        units.setAttribute('aria-label', `${name} coordinate units`);
        units.append(new Option('px', 'px'), new Option('%', '%'));
        actions.append(units);
    }
    const apply = iconButton('check', `Apply ${name} components`);
    apply.type = 'submit';
    actions.append(apply);
    const error = create('div');
    error.className = 'component-error';
    error.setAttribute('role', 'alert');
    panel.append(fields, actions, error);

    const disclosure = iconButton('chevron-right', `Edit ${name} components`, () => {
        if (panel.hidden) open();
        else close();
    });
    disclosure.classList.add('row-action');
    disclosure.setAttribute('aria-controls', panel.id);
    disclosure.setAttribute('aria-expanded', 'false');
    disclosure.dataset.property = `${name}:components`;

    function setLink(value: boolean): void {
        if (!link) return;
        linked = value;
        fields.toggleAttribute('data-linked', linked);
        link.setAttribute('aria-pressed', String(linked));
        labelButton(link, `${linked ? 'Unlink' : 'Link'} ${name} components`);
        for (const [index, field] of labels.entries()) field.label.hidden = linked && index > 0;
        const first = labels[0];
        first.caption.textContent = linked ? 'All' : model.labels[0];
        first.input.setAttribute('aria-label', `${name} ${linked ? 'all components' : first.label.title}`);
    }

    function open(): void {
        panel.hidden = false;
        labelButton(disclosure, `Close ${name} components`, 'chevron-down');
        disclosure.setAttribute('aria-expanded', 'true');
        error.textContent = '';
        for (const [index, input] of inputs.entries()) {
            input.value = model.values?.[index]?.toString() ?? '';
            input.removeAttribute('aria-invalid');
        }
        setLink(Boolean(model.values?.length && model.values.every(value => value === model.values?.[0])));
        if (units) units.value = model.percent ? '%' : 'px';
        inputs[0]?.focus();
    }

    function close(): void {
        panel.hidden = true;
        labelButton(disclosure, `Edit ${name} components`, 'chevron-right');
        disclosure.setAttribute('aria-expanded', 'false');
        disclosure.focus();
    }

    panel.addEventListener('keydown', event => {
        if (event.key === 'Escape') {
            event.preventDefault();
            event.stopPropagation();
            close();
        }
    });

    panel.addEventListener('submit', event => {
        event.preventDefault();
        error.textContent = '';
        const values = inputs.map(input => (linked ? inputs[0] : input).value.trim());
        const invalid = values.findIndex(value => !decimal.test(value) || !Number.isFinite(Number(value)));
        if (invalid >= 0) {
            error.textContent = 'Enter a finite number for every component.';
            inputs[invalid].setAttribute('aria-invalid', 'true');
            inputs[invalid].focus();
            return;
        }
        const percent = units?.value === '%';
        const changed = percent !== model.percent || values.some((value, index) => Number(value) !== model.values?.[index]);
        if (changed) commit(values.map(value => value + (percent ? '%' : '')).join(', '));
        else close();
    });
    return { disclosure, panel };
}
