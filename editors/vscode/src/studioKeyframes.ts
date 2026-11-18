import { action, choose, editText, editTime, type StudioContext, type StudioItem } from './studioContext';

interface Keyframe { time: number; value_text: string; easing_text: string }
interface Track { target: number; property: string; keyframes: Keyframe[] }
interface Timeline { duration: number; tracks: Track[] }
export interface Program { id: number; name: string; owner: number; targets: number[]; selector: string; timeline: Timeline | null }
interface Property { descriptor: { name: string; category: string; read_only: boolean; animatable: boolean }; value: string | null; expression: boolean }

export function trackItems(context: StudioContext, program: Program, timeline: Timeline): StudioItem[] {
    const items: StudioItem[] = timeline.tracks.map((track, index) => ({
        id: `timeline:${program.id}:track:${index}`, label: track.property,
        description: `${track.keyframes.length} keyframes`,
        children: async () => {
            const frames = track.keyframes.map((frame, key) => action(`timeline:key:${program.id}:${index}:${key}`,
                `${frame.time}s · ${frame.value_text}`, () => keyframeOptions(context, program.id, timeline.duration, track, frame), 'circle-outline'));
            if (context.writable) frames.unshift(action(`timeline:${program.id}:track:${index}:add`, 'Add keyframe…',
                () => authorKeyframe(context, program.id, timeline.duration, track), 'add'));
            return frames;
        },
    }));
    if (context.writable) items.unshift(action(`timeline:${program.id}:add-track`, 'Add property track…', async () => {
        const target = program.targets[0] ?? program.owner;
        const { properties } = await context.read<{ properties: Property[] }>('designer_properties', { node: target });
        const choices = properties.filter(({ descriptor }) => descriptor.animatable && !descriptor.read_only &&
            !timeline.tracks.some(track => track.target === target && track.property === descriptor.name));
        if (!choices.length) throw new Error('This target has no additional animatable properties.');
        const selected = await choose('Animate a property', choices.map(property => ({
            label: property.descriptor.name, description: property.descriptor.category, property,
        })));
        if (selected) await authorKeyframe(context, program.id, timeline.duration,
            { target, property: selected.label, keyframes: [] }, undefined,
            selected.property.expression ? undefined : selected.property.value ?? undefined);
    }, 'add'));
    return items;
}

async function keyframeOptions(context: StudioContext, program: number, duration: number, track: Track, frame: Keyframe): Promise<void> {
    const preview = () => context.present('designer_program', { action: 'preview', programs: [program], time: frame.time });
    if (!context.writable) return preview();
    const item = await choose(`${track.property} · ${frame.time}s`, [
        { label: 'Seek to keyframe', run: preview },
        { label: 'Edit time, value and easing…', run: () => authorKeyframe(context, program, duration, track, frame) },
        { label: 'Remove keyframe', run: () => context.edit('designer_program', {
            action: 'remove_keyframe', program, target: track.target, property: track.property, time: frame.time,
        }) },
    ]);
    await item?.run();
}

async function authorKeyframe(context: StudioContext, program: number, duration: number, track: Track,
    previous?: Keyframe, initialValue?: string): Promise<void> {
    const start = previous?.time ?? (track.keyframes[0]?.time === 0 ? duration : 0);
    const time = await editTime('Keyframe time in seconds', duration, start);
    if (time === undefined) return;
    const value = await editText(`${track.property} value`, previous?.value_text ?? initialValue ?? track.keyframes.at(-1)?.value_text);
    if (value === undefined) return;
    const easing = await editText('Incoming easing. Enter LinearEasing or Bézier x1,y1,x2,y2.', previous?.easing_text ?? 'LinearEasing');
    if (easing === undefined) return;
    await context.edit('designer_program', { action: 'upsert_keyframe', program, target: track.target, property: track.property,
        previous: previous?.time, time, value, easing });
}
