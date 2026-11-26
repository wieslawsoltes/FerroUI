import * as vscode from 'vscode';
import { choose, editText, type StudioContext } from './studioContext';

interface Field { name: string; kind: string; value: string | null }
interface BrushObject { node: number; kind: string; properties: Field[] }
interface Draft { source: string; brush: BrushObject; stops: BrushObject[]; presets: string[] }
type Change = { edit: 'set'; node: number; property: string; value: string } | { edit: 'preset'; kind: string }
    | { edit: 'insert_stop' } | { edit: 'remove_stop'; node: number } | { edit: 'move_stop'; node: number; direction: -1 | 1 };

type Choice = vscode.QuickPickItem & { run: () => Promise<Change | 'apply' | undefined> };

export async function editBrush(context: StudioContext, node: number, property: string): Promise<void> {
    const draftTool = 'designer_brush_draft';
    let model = await context.read<Draft>(draftTool, { action: 'open', node, property });
    const original = model.source;
    while (true) {
        const items: Choice[] = [];
        if (model.source !== original) items.push({ label: 'Apply Brush', detail: 'Save all draft changes in one undo step', run: async () => 'apply' });
        items.push({ label: 'Replace with brush preset…', description: readable(model.brush.kind), run: async () => {
            const preset = await choose('Brush preset', model.presets.map(kind => ({ label: readable(kind), brushType: kind })));
            if (preset) return { edit: 'preset', kind: preset.brushType };
        } });
        for (const field of model.brush.properties) items.push(fieldChoice(model.brush, field));
        if (model.brush.kind !== 'SolidColorBrush') items.push({ label: 'Add Gradient Stop', run: async () => ({ edit: 'insert_stop' }) });
        const stops = model.stops;
        for (const [index, stop] of stops.entries()) items.push({
            label: `Stop ${index + 1}`, description: stop.properties.find(field => field.name === 'Color')?.value ?? undefined,
            detail: `Offset ${stop.properties.find(field => field.name === 'Offset')?.value ?? '0'}`,
            run: () => chooseStop(stop, index, stops.length),
        });
        const action = await choose(`${property} Brush`, items);
        if (!action) return;
        const change = await action.run();
        if (change === 'apply') {
            await context.edit('designer_component', { action: 'set_object_property', node, property, source: model.source });
            return;
        }
        if (change) model = await context.read(draftTool, { action: 'change', source: model.source, change });
    }
}

function fieldChoice(target: BrushObject, field: Field): Choice {
    return { label: field.name, description: field.value ?? 'Default', detail: field.kind, run: async () => {
        const value = field.kind === 'GradientSpreadMethod'
            ? (await choose(field.name, ['Pad', 'Reflect', 'Repeat'].map(label => ({ label }))))?.label
            : await editText(field.name === 'Offset' ? 'Gradient stop offset from 0 to 1' : `${field.name} · ${field.kind}`, field.value ?? undefined);
        if (value?.trim() && value !== field.value) return { edit: 'set', node: target.node, property: field.name, value };
    } };
}

async function chooseStop(stop: BrushObject, index: number, count: number): Promise<Change | 'apply' | undefined> {
    const items = stop.properties.map(field => fieldChoice(stop, field));
    if (index > 0) items.push({ label: 'Move Earlier', run: async () => ({ edit: 'move_stop', node: stop.node, direction: -1 }) });
    if (index < count - 1) items.push({ label: 'Move Later', run: async () => ({ edit: 'move_stop', node: stop.node, direction: 1 }) });
    items.push({ label: 'Remove Stop', run: async () => ({ edit: 'remove_stop', node: stop.node }) });
    return (await choose(`Gradient Stop ${index + 1}`, items))?.run();
}

function readable(kind: string): string { return kind.replace(/([a-z])([A-Z])/g, '$1 $2'); }
