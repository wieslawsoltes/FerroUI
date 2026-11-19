import { action, editTime, type StudioContext, type StudioItem } from './studioContext';
import { trackItems, type Program } from './studioKeyframes';
import { createAnimationProgram, renameAnimationProgram, showAnimationActions } from './studioPrograms';

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
            ...(context.writable ? [action(`timeline:rename:${program.id}`, 'Rename program…',
                () => renameAnimationProgram(context, program), 'edit')] : []),
            ...(!context.writable && context.panel.supportsRustAnimationPrograms && program.sourceRange
                ? [action(`timeline:actions:${program.id}`, 'Animation actions…',
                    () => showAnimationActions(context, program), 'ellipsis')] : []),
        ],
    }));
    const owner = context.node;
    if ((context.writable && owner !== undefined) || context.panel.rustAnimationOwners.length) rows.unshift(action('timeline:create', 'New animation program…',
        () => createAnimationProgram(context, owner), 'add'));
    return rows;
}
