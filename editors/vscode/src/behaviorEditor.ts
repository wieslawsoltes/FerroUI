import type { Range } from './protocol';
import { actionKinds, actionLabel, chooseAction, chooseTrigger, comparisons, type Behavior, type BehaviorAction } from './studioBehaviors';
import { choose, editText, type StudioContext } from './studioContext';

export const BEHAVIOR_QUERY = 'designer_behaviors';
export const BEHAVIOR_TOOL = 'designer_behavior';

export interface ActionRow {
    id: number;
    range: Range;
    value: BehaviorAction;
}

export interface AttachedBehavior extends Behavior {
    range: Range;
    canEditTrigger: boolean;
    canEditActions: boolean;
    canPreview: boolean;
    actionRows: ActionRow[];
}

export interface BehaviorReport {
    owner: number;
    editable: boolean;
    canAttach: boolean;
    behaviors: AttachedBehavior[];
    activePreview: number | null;
}

interface ActionField { key: string; label: string; optional?: boolean; empty?: boolean }

const target = { key: 'target', label: 'Target', optional: true };
const classField = { key: 'class', label: 'Class' };
const actionFields: Record<string, readonly ActionField[]> = {
    ChangeProperty: [target, { key: 'property', label: 'Property' }, { key: 'value', label: 'Value', empty: true }],
    InvokeCommand: [{ key: 'command', label: 'Command' }, { key: 'parameter', label: 'Parameter', optional: true }],
    CallMethod: [target, { key: 'method', label: 'Method' }],
    AddClass: [target, classField],
    RemoveClass: [target, classField],
};

export class BehaviorEditor {
    constructor(readonly context: StudioContext, readonly report: BehaviorReport) {}

    private get rust(): boolean { return this.context.panel.language === 'rust'; }

    async attach(): Promise<void> {
        if (!this.report.canAttach) return;
        const owner = this.report.owner;
        const trigger = await chooseTrigger(undefined, event => pickBehaviorEvent(this.context, owner, event), this.rust);
        const item = trigger && await chooseAction(undefined, this.rust);
        if (item) await this.context.edit(BEHAVIOR_TOOL, { action: 'upsert', model: { owner, trigger, actions: [item] } });
    }

    async trigger(behavior: AttachedBehavior): Promise<void> {
        if (!behavior.canEditTrigger) return this.context.revealRange(behavior.range);
        const previous = behavior.trigger;
        if ('Event' in previous) {
            const current = previous.Event;
            const sourceName = current.source?.replace(/^\{(?:Reflection)?Binding #([^}]+)\}$/, '$1');
            const sourceText = this.rust ? sourceName : current.source;
            const field = await choose('Edit event trigger', [
                { label: 'Event', key: 'event', description: current.event },
                { label: 'Source', key: 'source', description: sourceText ?? 'Selected control' },
            ]);
            if (!field) return;
            const eventOwner = sourceName && this.context.panel.nodes.find(node => node.name === sourceName);
            const next = field.key === 'event'
                ? await pickBehaviorEvent(this.context, eventOwner ? eventOwner.id : behavior.owner, current.event)
                : await editText(this.rust ? 'Control name for the event source' : 'Name or binding for the event source',
                    sourceText ?? '', undefined, 'Event source');
            if (next !== undefined) await this.edit(behavior, { action: 'set_trigger',
                trigger: { Event: { ...current, [field.key]: field.key === 'source' && !next.trim() ? null : next } },
            });
            return;
        }
        if (!('Data' in previous)) return;
        const current = previous.Data;
        const field = await choose('Edit data condition', [
            { label: 'Binding', key: 'binding', description: current.binding },
            { label: 'Comparison', key: 'comparison', description: current.comparison },
            { label: 'Value', key: 'value', description: current.value },
            { label: 'Revert on false', key: 'revert_on_false', description: current.revert_on_false ? 'Yes' : 'No' },
        ]);
        if (!field) return;
        let next: string | boolean | undefined;
        if (field.key === 'comparison') {
            next = (await choose('Compare using', comparisons.map(label => ({ label, picked: label === current.comparison }))))?.label;
        } else if (field.key === 'revert_on_false') {
            next = (await choose('When the condition becomes false', [
                { label: 'Restore the previous value', value: true, picked: current.revert_on_false },
                { label: 'Keep the current value', value: false, picked: !current.revert_on_false },
            ]))?.value;
        } else {
            next = await editText(field.label, field.description,
                field.key === 'binding' ? required : undefined, 'Data condition');
        }
        if (next !== undefined) await this.edit(behavior, { action: 'set_trigger', trigger: { Data: { ...current, [field.key]: next } } });
    }

    async append(behavior: AttachedBehavior): Promise<void> {
        const item = behavior.canEditActions && await chooseAction(undefined, this.rust);
        if (item) await this.edit(behavior, { action: 'append_action', item });
    }

    async action(behavior: AttachedBehavior, row: ActionRow): Promise<void> {
        const { kind, values } = actionInfo(row);
        if (!behavior.canEditActions || !actionKinds.includes(kind)) return this.context.revealRange(row.range);
        const field = await choose(actionLabel(kind), actionFields[kind].map(field => ({
            ...field, description: values[field.key] ?? (field.key === 'target' ? 'Selected control' : undefined),
        })));
        if (!field) return;
        const next = await editText(field.label, values[field.key] ?? '', field.optional || field.empty ? undefined : required, actionLabel(kind));
        if (next !== undefined) await this.edit(behavior, {
            action: 'set_action', action_node: row.id, item: { [kind]: { ...values, [field.key]: field.optional && !next.trim() ? null : next } },
        });
    }

    move(behavior: AttachedBehavior, row: ActionRow, index: number): Promise<void> | undefined {
        if (behavior.canEditActions && index >= 0 && index < behavior.actionRows.length) {
            return this.edit(behavior, { action: 'move_action', action_node: row.id, index });
        }
    }

    edit(behavior: AttachedBehavior, args: object): Promise<void> {
        return this.context.edit(BEHAVIOR_TOOL, { behavior: behavior.id, ...args });
    }

    preview(behavior: AttachedBehavior): Promise<void> {
        return this.context.present(BEHAVIOR_TOOL, { action: 'preview', behavior: behavior.id });
    }

    clear(): Promise<void> {
        return this.context.present(BEHAVIOR_TOOL, { action: 'clear_preview' });
    }
}

function required(value: string): string | undefined {
    if (!value.trim()) return 'Enter a value.';
}

export async function pickBehaviorEvent(context: StudioContext, owner: number, current: string): Promise<string | undefined> {
    const { events } = await context.read<{ events: { name: string }[] }>('designer_events', { node: owner });
    const names = [...new Set([current, ...events.map(event => event.name)])].filter(Boolean);
    const selected = await choose('Routed event', [
        ...names.map(label => ({ label, other: false, picked: label === current })),
        { label: 'Enter event name…', other: true, picked: false },
    ]);
    if (selected?.other) return editText('Routed event name', current, required, 'Routed event');
    return selected?.label;
}

export function actionInfo(row: ActionRow) {
    const kind = Object.keys(row.value)[0];
    const values = row.value[kind] as Record<string, string | null>;
    return { kind, values };
}

export function describeAction(row: ActionRow): { label: string; summary?: string; icon: string } {
    const { kind, values } = actionInfo(row);
    const label = kind === 'Custom' ? values.type_name ?? 'Custom action' : actionLabel(kind);
    const summary = kind === 'ChangeProperty' ? `${values.property} → ${values.value}`
        : values.command ?? values.method ?? values.class ?? undefined;
    const icon = kind === 'Custom' ? 'symbol-structure' : kind === 'ChangeProperty' ? 'symbol-property' : 'symbol-method';
    return { label, summary, icon };
}
