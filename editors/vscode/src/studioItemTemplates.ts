import type { QuickPickItem } from 'vscode';
import type { Range } from './protocol';
import { choose, type StudioContext } from './studioContext';

type TemplateProperty = keyof typeof templateLabels;
type TemplateValue = { kind: 'unset' | 'expression' }
    | { kind: 'inline'; definition: number }
    | { kind: 'resource'; key: string; dynamic: boolean; definition: number | null };

interface TemplateSlot {
    property: TemplateProperty;
    value: TemplateValue;
    range: Range | null;
    reason: string | null;
    canCopyInline: boolean;
    canReset: boolean;
}

interface TemplateModel {
    model: string;
    slots: TemplateSlot[];
    resources: { node: number; key: string; target_type: string | null }[];
}

interface TemplateAction extends QuickPickItem {
    run: () => Promise<void>;
}

const templateLabels = {
    ItemTemplate: 'Item template',
    SelectionBoxItemTemplate: 'Selection box template',
    MenuItemTemplate: 'Menu item template',
    CellTemplate: 'Cell template',
    CellEditingTemplate: 'Cell editing template',
};
const tool = 'designer_item_template';

export async function editItemTemplates(context: StudioContext, node: number): Promise<void> {
    if (!context.writable) throw new Error('Open the template closure in its Rust source.');
    if (!context.panel.supportsItemTemplates) throw new Error('Use a FerroUI preview helper with item template authoring support.');
    const model = await context.read<TemplateModel>('designer_item_templates', { node });
    if (!model.slots.length) throw new Error('Select a control or column with an item template.');
    const slot = model.slots.length === 1 ? model.slots[0] : (await choose('Choose an Item Template', model.slots.map(slot => ({
        label: `$(symbol-structure) ${templateLabels[slot.property]}`, description: description(slot.value), slot,
    }))))?.slot;
    if (!slot) return;

    const args = { node, property: slot.property, expected_model: model.model };
    const edit = (action: 'create_inline' | 'copy_inline' | 'reset') => context.edit(tool, { ...args, action });
    const available: TemplateAction[] = [];
    if ((slot.value.kind === 'inline' || slot.value.kind === 'resource') && slot.value.definition !== null) {
        const definition = slot.value.definition;
        available.push({
            label: '$(preview) Edit Template on Canvas',
            run: async () => {
                await context.present(tool, { ...args, action: 'preview' });
                if (!context.panel.isDisposed) context.panel.panel.reveal(undefined, true);
            },
        }, { label: '$(go-to-file) Reveal Template Source', run: () => context.reveal(definition) });
    } else if (slot.range) {
        const range = slot.range;
        available.push({ label: '$(go-to-file) Reveal Template Assignment', run: () => context.revealRange(range) });
    }
    if (!slot.reason) {
        if (slot.value.kind === 'unset') {
            available.push({ label: '$(add) Create Inline Template', run: () => edit('create_inline') });
        }
        if (model.resources.length) {
            available.push({
                label: '$(library) Assign Template Resource…',
                run: async () => {
                    const picked = await choose('Assign a Visible Data Template', model.resources.map(resource => ({
                        label: `$(symbol-structure) ${resource.key}`, description: resource.target_type ?? undefined, resource: resource.node,
                    })));
                    if (picked) await context.edit(tool, { ...args, action: 'assign_resource', resource: picked.resource });
                },
            });
        }
        if (slot.canCopyInline) available.push({ label: '$(copy) Make Inline Copy', run: () => edit('copy_inline') });
        if (slot.canReset) available.push({ label: '$(discard) Reset Template', run: () => edit('reset') });
    }
    if (!available.length) throw new Error(slot.reason ?? 'The current template has no local definition.');
    const action = await choose(`${templateLabels[slot.property]} · ${description(slot.value)}`, available);
    await action?.run();
}

function description(value: TemplateValue): string {
    switch (value.kind) {
        case 'unset': return 'Default';
        case 'inline': return 'Inline';
        case 'resource': return value.key;
        case 'expression': return 'Authored expression';
    }
}
