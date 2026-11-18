import * as vscode from 'vscode';
import { choose, editText, type StudioContext } from './studioContext';

type BindingValue = { kind: 'text' | 'expression'; value: string };
type BindingSource = { kind: 'dataContext' | 'selfValue' | 'templatedParent' }
    | { kind: 'element'; name: string }
    | { kind: 'ancestor'; typeName: string; level: number; logicalTree: boolean }
    | { kind: 'explicit'; value: BindingValue };
type BindingChange = { field: 'path' | 'mode' | 'trigger'; value: string }
    | { field: 'delay'; value: number | null }
    | { field: 'source'; value: BindingSource }
    | { field: 'option'; name: string; value: BindingValue | null };

interface BindingModel {
    authored: string | null;
    extension: string;
    relativeSources: boolean;
    fields: {
        path: string | null;
        mode: string | null;
        trigger: string | null;
        delay: number | null;
        source: BindingSource | null;
    } | null;
    modes: string[];
    triggers: string[];
    options: Record<string, BindingValue | null>;
    elementNames: string[];
}

const optionLabels: Record<string, string> = {
    Converter: 'Converter',
    ConverterParameter: 'Converter parameter',
    ConverterCulture: 'Converter culture',
    StringFormat: 'String format',
    FallbackValue: 'Fallback value',
    TargetNullValue: 'Null value',
};
const sourceLabels = {
    dataContext: 'Data context', element: 'Named element', selfValue: 'Self',
    templatedParent: 'Templated parent', ancestor: 'Find ancestor', explicit: 'Explicit source',
} satisfies Record<BindingSource['kind'], string>;

interface BindingItem extends vscode.QuickPickItem {
    update?: () => Promise<BindingChange | undefined>;
}

// The menu stages only fields the user changed. The native draft retains untouched source.
export async function editBinding(context: StudioContext, node: number, property: string): Promise<void> {
    if (!context.writable) throw new Error('Structured property bindings are available in XAML.');
    const model = await context.read<BindingModel>('designer_binding', { node, property });
    const title = `${model.extension} · ${property}`;
    const replace = model.authored !== null && model.fields === null;
    if (replace && !await choose(title, [{ label: 'Replace value with a binding', description: model.authored ?? undefined }])) return;
    const changes = new Map<string, BindingChange>();
    const fields = { ...model.fields };
    const options = { ...model.options };
    while (true) {
        context.ensure();
        const items: BindingItem[] = [
            textField('Path', fields.path, 'path'),
            choiceField('Mode', fields.mode, 'mode', model.modes),
            choiceField('Update source trigger', fields.trigger, 'trigger', model.triggers),
            {
                label: 'Delay', description: fields.delay == null ? 'Default' : `${fields.delay} ms`,
                update: async () => {
                    const value = await integer('Delay in milliseconds. Leave empty for the default.', fields.delay ?? null, 0);
                    return value === undefined ? undefined : { field: 'delay', value };
                },
            },
            {
                label: 'Source', description: describeSource(fields.source, model.fields !== null),
                update: async () => {
                    const value = await chooseSource(model, fields.source);
                    return value && { field: 'source', value };
                },
            },
            ...Object.entries(optionLabels).map(([name, label]): BindingItem => ({
                label, description: options[name]?.value ?? 'Unset',
                update: async () => {
                    const value = await chooseValue(label, options[name], name !== 'Converter', true);
                    return value === undefined ? undefined : { field: 'option', name, value };
                },
            })),
        ];
        if (changes.size) items.unshift({ label: '$(check) Apply binding', description: `${changes.size} changed fields` });
        const selected = await choose(title, items);
        if (!selected) return;
        if (!selected.update) {
            await context.edit('designer_binding_edit', { node, property, replace, changes: [...changes.values()] });
            return;
        }
        const change = await selected.update();
        if (!change) continue;
        if (change.field === 'option') {
            options[change.name] = change.value;
            changes.set(change.name, change);
        } else {
            switch (change.field) {
                case 'source': fields.source = change.value; break;
                case 'delay': fields.delay = change.value; break;
                default: fields[change.field] = change.value;
            }
            changes.set(change.field, change);
        }
    }
}

function textField(label: string, current: string | null | undefined, field: 'path'): BindingItem {
    return {
        label, description: current || 'Whole source',
        update: async () => {
            const value = await editText('Binding path. Leave empty to bind to the whole source.', current ?? '');
            return value === undefined ? undefined : { field, value };
        },
    };
}

function choiceField(label: string, current: string | null | undefined, field: 'mode' | 'trigger', options: string[]): BindingItem {
    return {
        label, description: current ?? 'Default',
        update: async () => {
            const item = await choose(label, options.map(value => ({ label: value, value })));
            return item && { field, value: item.value };
        },
    };
}

function describeSource(source: BindingSource | null | undefined, existing: boolean): string {
    if (!source) return existing ? 'Authored source' : sourceLabels.dataContext;
    if (source.kind === 'element') return source.name;
    if (source.kind === 'ancestor') return `${source.typeName} · level ${source.level}`;
    return sourceLabels[source.kind];
}

async function chooseSource(model: BindingModel, current?: BindingSource | null): Promise<BindingSource | undefined> {
    const relative = new Set(['selfValue', 'templatedParent', 'ancestor']);
    const kinds = Object.keys(sourceLabels) as BindingSource['kind'][];
    const kind = (await choose('Binding source', kinds
        .filter(kind => model.relativeSources || !relative.has(kind))
        .map(kind => ({ label: sourceLabels[kind], sourceKind: kind }))))?.sourceKind;
    switch (kind) {
        case 'element': {
            const item = await choose('Source element', [
                ...model.elementNames.map(name => ({ label: name, name })),
                { label: 'Enter an element name…', name: undefined },
            ]);
            if (!item) return;
            const name = item.name ?? await editText('Element name in this namescope', current?.kind === 'element' ? current.name : '');
            return name?.trim() ? { kind, name: name.trim() } : undefined;
        }
        case 'ancestor': {
            const previous = current?.kind === kind ? current : undefined;
            const typeName = await editText('Ancestor XAML type', previous?.typeName);
            if (!typeName?.trim()) return;
            const level = await integer('Ancestor level', previous?.level ?? 1, 1);
            if (level == null) return;
            const tree = await choose('Ancestor tree', [{ label: 'Visual', logical: false }, { label: 'Logical', logical: true }]);
            return tree && { kind, typeName: typeName.trim(), level, logicalTree: tree.logical };
        }
        case 'explicit': {
            const value = await chooseValue('Explicit source', current?.kind === kind ? current.value : undefined, true, false);
            return value ? { kind, value } : undefined;
        }
        case 'dataContext':
        case 'selfValue':
        case 'templatedParent': return { kind };
        default: return undefined;
    }
}

async function chooseValue(label: string, current: BindingValue | null | undefined,
    textAllowed: boolean, clearAllowed: boolean): Promise<BindingValue | null | undefined> {
    const choices: { label: string; valueKind: BindingValue['kind'] | 'clear' }[] =
        [{ label: 'XAML expression', valueKind: 'expression' }];
    if (textAllowed) choices.push({ label: 'Text', valueKind: 'text' });
    if (clearAllowed) choices.push({ label: 'Clear', valueKind: 'clear' });
    const selected = await choose(label, choices);
    if (!selected) return;
    if (selected.valueKind === 'clear') return null;
    const value = await editText(label, current?.kind === selected.valueKind ? current.value : undefined);
    return value === undefined ? undefined : { kind: selected.valueKind, value };
}

async function integer(prompt: string, initial: number | null, minimum: number): Promise<number | null | undefined> {
    const value = await vscode.window.showInputBox({
        title: 'FerroUI Binding', prompt, value: initial?.toString() ?? '', ignoreFocusOut: true,
        validateInput: value => {
            if (!value.trim() && minimum === 0) return undefined;
            const number = Number(value);
            return /^\d+$/.test(value.trim()) && number >= minimum && number <= 2_147_483_647
                ? undefined : `Enter a whole number of at least ${minimum}.`;
        },
    });
    return value === undefined ? undefined : value.trim() ? Number(value) : null;
}
