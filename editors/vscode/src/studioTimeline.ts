import { action, editText, editTime, type StudioContext, type StudioItem } from './studioContext';
import { trackItems, type Program } from './studioKeyframes';

export async function timelineItems(context: StudioContext): Promise<StudioItem[]> {
    const { programs } = await context.read<{ programs: Program[] }>('designer_programs', { include_source: false, include_keyframe_text: true });
    const rows: StudioItem[] = programs.map(program => ({
        id: `timeline:${program.id}`, label: program.name,
        description: program.timeline ? `${program.timeline.duration}s · ${program.timeline.tracks.length} tracks` : 'Source animation',
        children: async () => [
            action(`timeline:seek:${program.id}`, 'Seek to time…', async () => {
                const time = await editTime('Time in seconds', program.timeline?.duration ?? 0, 0);
                if (time !== undefined) await context.present('designer_program', { action: 'preview', programs: [program.id], time });
            }, 'debug-step-over'),
            ...(program.timeline ? trackItems(context, program, program.timeline) : []),
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
        const duration = await editTime('Duration in seconds', 600, 1);
        if (duration !== undefined && duration > 0) await context.edit('designer_program', { action: 'create', owner, name, selector, duration });
    }, 'add'));
    return rows;
}
