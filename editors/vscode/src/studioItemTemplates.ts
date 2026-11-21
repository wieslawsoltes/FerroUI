import type { QuickPickItem } from 'vscode';
import type { Range } from './protocol';
import type { IncludedTemplateSource } from './includedTemplate';
import { templateLabels, type TemplateProperty } from './itemTemplateLabels';
import { choose, editText, type StudioContext } from './studioContext';

type TemplateValue = { kind: 'unset' | 'expression' }
    | { kind: 'inline'; definition: number }
    | { kind: 'resource'; key: string; dynamic: boolean; definition: number | null };

interface TemplateSlot {
    property: TemplateProperty;
    value: TemplateValue;
    range: Range | null;
    reason: string | null;
    canCopyInline: boolean;
    copyReason?: string | null;
    canReset: boolean;
    canCreate?: boolean;
    rustBody?: Range | null;
    includedDefinition?: IncludedTemplateSource | null;
}

interface TemplateModel {
    model: string;
    slots: TemplateSlot[];
    resources: { node: number; key: string; target_type: string | null }[];
    includedResources?: Pick<IncludedTemplateSource, 'uri' | 'key'>[];
}

interface TemplateAction extends QuickPickItem {
    run: () => Promise<void>;
}

const tool = 'designer_item_template';

export async function editItemTemplates(context: StudioContext, node: number, member?: 'Template' | 'CellTemplate'): Promise<void> {
    if (member === 'Template' ? !context.panel.supportsControlTemplate : !context.panel.supportsItemTemplates) {
        throw new Error('Update the FerroUI preview helper to edit this template.');
    }
    const model = await context.read<TemplateModel>('designer_item_templates', member === 'Template' ? { node, property: member } : { node });
    const slots = model.slots.filter(slot => !member || slot.property === member);
    if (!slots.length) throw new Error('Select a control or column with a supported template property.');
    const slot = slots.length === 1 ? slots[0] : (await choose('Choose an Item Template', slots.map(slot => ({
        label: `$(symbol-structure) ${templateLabels[slot.property]}`, description: description(slot), slot,
    }))))?.slot;
    if (!slot) return;

    const args = { node, property: slot.property, expected_model: model.model };
    const edit = (action: 'create_inline' | 'copy_inline' | 'reset') => context.edit(tool, { ...args, action });
    const available: TemplateAction[] = [];
    const definition = slot.value.kind === 'inline' || slot.value.kind === 'resource' ? slot.value.definition : null;
    const included = slot.includedDefinition;
    if (definition !== null || included || slot.rustBody) {
        available.push({
            label: '$(preview) Edit Template on Canvas',
            run: async () => {
                await context.present(tool, { ...args, action: 'preview' });
                if (!included && !context.panel.isDisposed) context.panel.panel.reveal(undefined, true);
            },
        });
    }
    if (definition !== null || included) {
        available.push({ label: '$(go-to-file) Reveal Template Source', detail: slot.copyReason ?? undefined,
            run: () => included ? context.revealIncludedTemplate(included) : context.reveal(definition!) });
    } else if (slot.range) {
        const range = slot.range;
        available.push({ label: '$(go-to-file) Reveal Template Assignment', detail: slot.copyReason ?? slot.reason ?? undefined,
            run: () => context.revealRange(range) });
    }
    if (!slot.reason) {
        if (slot.value.kind === 'unset' && context.writable) {
            available.push({ label: '$(add) Create Inline Template', run: () => edit('create_inline') });
        }
        if (slot.canCreate && !context.writable) {
            available.push({ label: '$(add) Create Inline Rust Template…', run: async () => {
                const rowType = await editText('Rust item type, such as crate::Customer or String', undefined,
                    input => input.trim() ? undefined : 'Enter the type of one item.', templateLabels[slot.property]);
                if (rowType !== undefined) await context.edit(tool, { ...args, action: 'create_inline', rust_type: rowType });
            } });
        }
        const resources = [
            ...model.resources.map(resource => ({
                label: `$(symbol-structure) ${resource.key}`, description: resource.target_type ?? undefined,
                arguments: { action: 'assign_resource', resource: resource.node },
            })),
            ...(model.includedResources ?? []).map(resource => ({
                label: `$(symbol-structure) ${resource.key}`,
                description: decodeURIComponent(resource.uri.split('/').at(-1) ?? ''),
                arguments: { action: 'assign_included_resource', key: resource.key },
            })),
        ];
        if (resources.length) {
            available.push({
                label: '$(library) Assign Template Resource…',
                run: async () => {
                    const picked = await choose(member ? 'Assign a Visible Control Template' : 'Assign a Visible Data Template', resources);
                    if (picked) await context.edit(tool, { ...args, ...picked.arguments });
                },
            });
        }
        if (slot.canCopyInline) available.push({ label: '$(copy) Make Inline Copy', run: () => edit('copy_inline') });
        if (slot.canReset) available.push({ label: '$(discard) Reset Template', run: () => edit('reset') });
    }
    if (!available.length) throw new Error(slot.reason ?? 'The current template has no local definition.');
    const action = await choose(`${templateLabels[slot.property]} · ${description(slot)}`, available);
    await action?.run();
}

function description({ value, rustBody }: TemplateSlot): string {
    if (rustBody) return 'Inline RSX';
    switch (value.kind) {
        case 'unset': return 'Default';
        case 'inline': return 'Inline';
        case 'resource': return value.key;
        case 'expression': return 'Authored expression';
    }
}
