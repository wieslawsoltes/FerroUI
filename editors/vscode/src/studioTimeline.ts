import { action, choose, editText, type StudioContext, type StudioItem } from './studioContext';

interface Timeline { duration: number; tracks: Array<{ property: string; keyframes: Array<{ time: number }> }> }
interface Program { id: number; name: string; selector: string; timeline: Timeline | null }
interface StateGroup { owner: number; name: string; states: Array<{ name: string; setters: Array<{ property: string; value: string; target: string }> }>; transitions: unknown[] }

export async function timelineItems(context: StudioContext): Promise<StudioItem[]> {
    const { programs } = await context.read<{ programs: Program[] }>('designer_programs', { include_source: false });
    const rows: StudioItem[] = programs.map(program => ({
        id: `timeline:${program.id}`, label: program.name,
        description: program.timeline ? `${program.timeline.duration}s · ${program.timeline.tracks.length} tracks` : 'Source animation',
        children: async () => [
            action(`timeline:seek:${program.id}`, 'Seek to time…', async () => {
                const time = await seconds('Time in seconds', program.timeline?.duration ?? 0, 0);
                if (time !== undefined) await context.present('designer_program', { action: 'preview', programs: [program.id], time });
            }, 'debug-step-over'),
            ...(program.timeline?.tracks ?? []).map((track, index) => ({
                id: `timeline:${program.id}:track:${index}`, label: track.property,
                description: `${track.keyframes.length} keyframes`,
                children: async () => track.keyframes.map((frame, key) => action(`timeline:key:${program.id}:${index}:${key}`, `${frame.time}s`,
                    () => context.present('designer_program', { action: 'preview', programs: [program.id], time: frame.time }), 'circle-outline')),
            })),
            ...(context.writable ? [action(`timeline:rename:${program.id}`, 'Rename program…', async () => {
                const name = await editText('Animation name', program.name);
                if (name && name !== program.name) await context.edit('designer_program', { action: 'rename', program: program.id, name });
            }, 'edit')] : []),
        ],
    }));
    const owner = context.node;
    if (context.writable && owner !== undefined) rows.unshift(action('timeline:create', 'New animation program…', async () => {
        const name = await editText('Animation name');
        if (!name) return;
        const node = context.panel.nodes.find(node => node.id === owner);
        const selector = await editText('FerroUI selector', node?.name ? `${node.label}#${node.name}` : node?.label);
        if (!selector) return;
        const duration = await seconds('Duration in seconds', 600, 1);
        if (duration !== undefined && duration > 0) await context.edit('designer_program', { action: 'create', owner, name, selector, duration });
    }, 'add'));
    return rows;
}

export async function stateItems(context: StudioContext): Promise<StudioItem[]> {
    const { groups } = await context.read<{ groups: StateGroup[] }>('designer_states');
    const rows = groups.map(group => ({
        id: `states:${group.owner}:${group.name}`, label: group.name,
        description: `${group.states.length} states`,
        children: async () => group.states.map(state => action(`state:${group.owner}:${group.name}:${state.name}`, state.name, async () => {
            const options = [{ label: 'Preview state', run: () => context.present('designer_state', { action: 'preview', owner: group.owner, group: group.name, state: state.name }) }];
            if (context.writable) options.push({ label: 'Add or edit a state setter', run: async () => {
                const property = await editText('Property name');
                if (!property) return;
                const target = await editText('Named target. Leave empty for the owner.', '');
                if (target === undefined) return;
                const existing = state.setters.find(setter => setter.property === property && setter.target === target);
                const value = await editText('State value', existing?.value);
                if (value === undefined) return;
                const setters = [...state.setters.filter(setter => setter !== existing), { property, value, target }];
                await context.edit('designer_state', { action: 'upsert', model: { ...group, states: group.states.map(item => item === state ? { ...item, setters } : item) } });
            } });
            await (await choose(state.name, options))?.run();
        }, 'symbol-enum-member')),
    })) as StudioItem[];
    rows.unshift(action('states:clear', 'Clear state preview', () => context.present('designer_state', { action: 'clear_preview' }), 'clear-all'));
    return rows;
}

async function seconds(prompt: string, maximum: number, initial: number): Promise<number | undefined> {
    const value = await editText(`${prompt} (0–${maximum})`, String(initial));
    if (value === undefined) return;
    const time = value.trim() ? Number(value) : Number.NaN;
    if (!Number.isFinite(time) || time < 0 || time > maximum) throw new Error(`Enter a time from 0 to ${maximum} seconds.`);
    return time;
}
