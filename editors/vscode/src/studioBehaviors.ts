import { action, choose, editText, type StudioContext, type StudioItem } from './studioContext';

type Trigger = { Event: { event: string; source: string | null } }
    | { Data: { binding: string; comparison: string; value: string; revert_on_false: boolean } }
    | { Custom: { type_name: string } };
type BehaviorAction = Record<string, unknown>;
interface Behavior { id: number; owner: number; trigger: Trigger; actions: BehaviorAction[] }

export async function behaviorItems(context: StudioContext): Promise<StudioItem[]> {
    const owner = context.node;
    if (owner === undefined) return [];
    const { behaviors } = await context.read<{ behaviors: Behavior[] }>('designer_behaviors', { owner });
    const update = (model: Omit<Behavior, 'id'> & { id?: number }) => context.edit('designer_behavior', { action: 'upsert', model });
    const items: StudioItem[] = behaviors.map(behavior => ({
        id: `behavior:${behavior.id}`, label: label(behavior.trigger), description: `${behavior.actions.length} actions`,
        children: async () => {
            const children = behavior.actions.map((item, index) => {
                const kind = Object.keys(item)[0];
                return action(`behavior:${behavior.id}:${index}`, actionLabel(kind), async () => {
                    if (!context.writable) return context.reveal(behavior.id);
                    const options = [
                        { label: 'Reveal behavior in source', run: () => context.reveal(behavior.id) },
                        { label: 'Remove action', run: () => update({ ...behavior, actions: behavior.actions.filter((_, at) => at !== index) }) },
                    ];
                    if (actionKinds.includes(kind)) options.unshift({ label: 'Edit action…', run: async () => {
                        const edited = await chooseAction(item);
                        if (edited) await update({ ...behavior, actions: behavior.actions.map((action, at) => at === index ? edited : action) });
                    } });
                    const selected = await choose(actionLabel(kind), options);
                    await selected?.run();
                }, 'run');
            });
            if (!context.writable) return children;
            children.unshift(action(`behavior:${behavior.id}:add`, 'Add action…', async () => {
                const next = await chooseAction();
                if (next) await update({ ...behavior, actions: [...behavior.actions, next] });
            }, 'add'));
            if (!('Custom' in behavior.trigger)) children.push(action(`behavior:${behavior.id}:trigger`, 'Edit trigger…', async () => {
                const trigger = await chooseTrigger(behavior.trigger);
                if (trigger) await update({ ...behavior, trigger });
            }, 'edit'));
            children.push(action(`behavior:${behavior.id}:remove`, 'Remove behavior',
                () => context.edit('designer_behavior', { action: 'remove', behavior: behavior.id }), 'trash'));
            return children;
        },
    }));
    if (context.writable) items.unshift(action('behavior:create', 'Attach behavior…', async () => {
        const trigger = await chooseTrigger();
        if (!trigger) return;
        const first = await chooseAction();
        if (first) await update({ owner, trigger, actions: [first] });
    }, 'add'));
    return items;
}

function label(trigger: Trigger): string {
    if ('Event' in trigger) return trigger.Event.event;
    if ('Data' in trigger) return `${trigger.Data.binding} · ${trigger.Data.comparison} · ${trigger.Data.value}`;
    return trigger.Custom.type_name;
}

async function chooseTrigger(previous?: Trigger): Promise<Trigger | undefined> {
    const kind = await choose('When should this behavior run?', [{ label: 'Routed event', event: true }, { label: 'Data condition', event: false }]);
    if (!kind) return;
    if (kind.event) {
        const current = previous && 'Event' in previous ? previous.Event : undefined;
        const event = await editText('Routed event name', current?.event ?? 'Click');
        if (!event) return;
        const source = await editText('Source name or binding. Leave empty for the selected control.', current?.source ?? '');
        if (source !== undefined) return { Event: { event, source: source || null } };
        return;
    }
    const current = previous && 'Data' in previous ? previous.Data : undefined;
    const binding = await editText('Data binding', current?.binding ?? '{Binding IsReady}');
    if (!binding) return;
    const comparisons = ['Equal', 'NotEqual', 'LessThan', 'LessThanOrEqual', 'GreaterThan', 'GreaterThanOrEqual'];
    const comparison = await choose('Comparison', comparisons.map(label => ({ label, picked: label === current?.comparison })));
    if (!comparison) return;
    const value = await editText('Compare with', current?.value ?? 'True');
    if (value === undefined) return;
    const revert = await choose('When the condition becomes false', [
        { label: 'Keep the current value', revert: false, picked: !current?.revert_on_false },
        { label: 'Restore the previous value', revert: true, picked: current?.revert_on_false },
    ]);
    if (revert) return { Data: { binding, comparison: comparison.label, value, revert_on_false: revert.revert } };
}

function actionLabel(kind: string): string { return kind.replace(/([a-z])([A-Z])/g, '$1 $2'); }

const actionKinds = ['ChangeProperty', 'InvokeCommand', 'CallMethod', 'AddClass', 'RemoveClass'];

async function chooseAction(previous?: BehaviorAction): Promise<BehaviorAction | undefined> {
    const selection = await choose('What should this behavior do?', actionKinds
        .map(kind => ({ actionKind: kind, label: actionLabel(kind), picked: previous && kind in previous })));
    if (!selection) return;
    const fields = previous?.[selection.actionKind] as Record<string, string | null> | undefined;
    if (selection.actionKind === 'InvokeCommand') {
        const command = await editText('Command binding', fields?.command ?? '{Binding SaveCommand}');
        if (!command) return;
        const parameter = await editText('Optional command parameter', fields?.parameter ?? '');
        if (parameter !== undefined) return { InvokeCommand: { command, parameter: parameter || null } };
        return;
    }
    const destination = await editText('Target name or binding. Leave empty for the selected control.', fields?.target ?? '');
    if (destination === undefined) return;
    const target = destination || null;
    switch (selection.actionKind) {
        case 'ChangeProperty': {
            const property = await editText('Property name', fields?.property ?? 'Opacity');
            if (!property) return;
            const value = await editText('Property value', fields?.value ?? '0.5');
            if (value !== undefined) return { ChangeProperty: { target, property, value } };
            return;
        }
        case 'CallMethod': {
            const method = await editText('Method name', fields?.method ?? undefined);
            if (method) return { CallMethod: { target, method } };
            return;
        }
        default: {
            const className = await editText('Style class name', fields?.class ?? undefined);
            if (className) return { [selection.actionKind]: { target, class: className } };
        }
    }
}
