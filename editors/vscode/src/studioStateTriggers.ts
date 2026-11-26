import type { Range } from './protocol';
import { action, choose, type StudioContext, type StudioItem } from './studioContext';

interface TriggerGroup { group: string; states: string[] }
interface Trigger { event: string; group: string; state: string; range: Range }

export interface StateTriggerReport {
    events: string[];
    groups: TriggerGroup[];
    triggers: Trigger[];
}

export function stateTriggerItems(context: StudioContext, source: number, report: StateTriggerReport): StudioItem[] {
    const groups = report.groups.filter(group => group.states.length);
    if (!report.triggers.length && (!report.events.length || !groups.length)) return [];
    const edit = (event: string, group: string, state: string | null) =>
        context.edit('designer_state', { action: 'set_event_trigger', event_source: source, event, group, state });
    const rows: StudioItem[] = report.triggers.map((trigger, index) => ({
        ...action(`states:trigger:${source}:${index}`, `${trigger.event} → ${trigger.state}`, async () => {
            const group = groups.find(group => group.group === trigger.group);
            const editable = report.events.includes(trigger.event)
                && report.triggers.filter(item => item.event === trigger.event && item.group === trigger.group).length === 1;
            const actions = [{ label: 'Reveal Rust declaration', run: () => context.revealRustRange(trigger.range) }];
            if (editable && group) actions.unshift({ label: 'Change destination…', run: async () => {
                const state = await pickState(group, trigger.state);
                if (state !== undefined) await edit(trigger.event, trigger.group, state);
            } });
            if (editable) actions.push({ label: 'Remove trigger', run: () => edit(trigger.event, trigger.group, null) });
            await (await choose('Visual state trigger', actions))?.run();
        }, 'zap'),
        description: trigger.group,
    }));
    if (report.events.length && groups.length) rows.unshift(action(`states:trigger:add:${source}`, 'Link event to state…', async () => {
        const event = await choose('Routed event', report.events.map(label => ({ label })));
        if (!event) return;
        const group = groups.length === 1 ? groups[0] : (await choose('Enclosing state group',
            groups.map(group => ({ label: group.group, group }))))?.group;
        if (!group) return;
        const current = report.triggers.find(trigger => trigger.event === event.label && trigger.group === group.group);
        const state = await pickState(group, current?.state);
        if (state !== undefined) await edit(event.label, group.group, state);
    }, 'add'));
    return [{ id: `states:triggers:${source}`, label: 'Event triggers', icon: 'zap', children: async () => rows }];
}

async function pickState(group: TriggerGroup, current?: string): Promise<string | undefined> {
    return (await choose(group.group, group.states.map(label => ({ label, picked: label === current }))))?.label;
}
