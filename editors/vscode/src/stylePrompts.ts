import { choose, editText } from './studioContext';
import type { StyleCatalog, StyleCreation, StyleSetter } from './stylesProtocol';

export async function chooseNewStyle(catalog: StyleCatalog, kind: 'style' | 'theme', suggestedType?: string): Promise<{ node: number; edit: StyleCreation } | undefined> {
    const scopes = kind === 'theme' ? catalog.palette_scopes : catalog.style_hosts;
    const scope = await choose(kind === 'style' ? 'Choose a Style Host' : 'Choose a Resource Scope',
        scopes.map(scope => ({ label: scope.label, description: 'type_name' in scope ? String(scope.type_name) : undefined, scope })));
    if (!scope) return;
    const node = scope.scope.node;
    if (kind === 'style') {
        const selector = await editText('Selector', 'nested' in scope.scope && scope.scope.nested ? '^:pointerover' : suggestedType,
            value => value.trim() ? undefined : 'Enter a style selector.', 'New FerroUI Style');
        if (selector !== undefined) return { node, edit: { action: 'create', selector } };
        return;
    }
    const key = await editText('Control theme resource key', undefined, value => value.trim() ? undefined : 'Enter a resource key.', 'New FerroUI Control Theme');
    if (key === undefined) return;
    const target = await editText('Control target type', suggestedType ?? 'Button', value => value.trim() ? undefined : 'Enter a XAML type.', 'New FerroUI Control Theme');
    if (target !== undefined) return { node, edit: { action: 'create_theme', key: key.trim(), target_type: target.trim() } };
}

type NewSetter = { property: string; kind: 'literal'; value: string; getter?: string } | { property: string; kind: 'resource' };

export async function chooseStyleIssue(issues: NonNullable<StyleCatalog['issues']>) {
    const selected = await choose('Reveal a Rust Style Expression', issues.map(issue => ({
        label: issue.reason, description: `Line ${issue.range.start.line + 1}`, range: issue.range,
    })));
    return selected?.range;
}

export async function chooseNewSetter(available: readonly StyleSetter[]): Promise<NewSetter | undefined> {
    const item = await choose('Choose a Setter Property', available.map(setter => ({
        label: setter.property, description: [setter.field.type_name, setter.field.owner].filter(Boolean).join(' · '), setter,
    })));
    if (!item) return;
    const { property, field } = item.setter;
    const mode = field.resources ? await choose(property, [{ label: 'Literal value', resource: false }, { label: 'Resource reference', resource: true }])
        : { resource: false };
    if (!mode) return;
    if (mode.resource) return { property, kind: 'resource' };
    const value = field.options.length ? (await choose(property, field.options.map(label => ({ label }))))?.label
        : await editText(`${property} value`, field.kind === 'color' ? '#FF4C82D0' : undefined, undefined, 'New FerroUI Setter');
    if (value !== undefined) return { property, kind: 'literal', value, getter: item.setter.getter };
}

export async function chooseStyleResource<T extends { key: string; type_name: string }>(property: string, resources: readonly T[]) {
    if (!resources.length) throw new Error('This style scope has no compatible resource for that property.');
    const item = await choose(`Choose a Resource for ${property}`, resources.map(resource => ({
        label: resource.key, description: resource.type_name.replace(/^\{.*?\}/, ''), resource,
    })));
    if (!item) return;
    const mode = await choose('Reference Mode', [
        { label: 'Dynamic Resource', description: 'Resolve against the live theme and resource scope', dynamic: true },
        { label: 'Static Resource', description: 'Resolve when the definition loads', dynamic: false },
    ]);
    if (mode) return { resource: item.resource, dynamic: mode.dynamic };
}
