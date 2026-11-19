import { action, choose, editText, editTime, type StudioContext, type StudioItem } from './studioContext';
import type { KeyframeGroupEdit, TimelineKeyframe, TimelineProgram, TimelineTrack } from './timelineProtocol';

export type Program = TimelineProgram;
type Timeline = NonNullable<TimelineProgram['timeline']>;
interface Property { descriptor: { name: string; category: string; read_only: boolean; animatable: boolean }; value: string | null; expression: boolean }

export function trackItems(context: StudioContext, program: Program, timeline: Timeline): StudioItem[] {
    const items: StudioItem[] = timeline.tracks.map((track, index) => ({
        id: `timeline:${program.id}:track:${index}`, label: track.property,
        description: `${track.keyframes.length} keyframes`,
        children: async () => {
            const frames = track.keyframes.map((frame, key) => action(`timeline:key:${program.id}:${index}:${key}`,
                `${frame.time}s · ${frame.value_text}`, () => keyframeOptions(context, program.id, timeline.duration, track, frame), 'circle-outline'));
            if (context.writable || (program.sourceRange && context.panel.supportsRustKeyframeStructure)) {
                frames.unshift(action(`timeline:${program.id}:track:${index}:add`, 'Add keyframe…',
                    () => authorKeyframe(context, program.id, timeline.duration, track), 'add'));
            }
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

export async function keyframeOptions(context: StudioContext, program: number, duration: number, track: TimelineTrack, frame: TimelineKeyframe,
    seek?: () => Promise<void>): Promise<void> {
    const preview = seek ?? (() => context.present('designer_program', { action: 'scrub', program, time: frame.time }));
    if (!(context.writable || context.panel.supportsRustKeyframeStructure)) return preview();
    const item = await choose(`${track.property} · ${frame.time}s`, [
        { label: 'Seek to keyframe', run: preview },
        { label: 'Edit time, value and easing…', run: () => authorKeyframe(context, program, duration, track, frame) },
        { label: 'Remove keyframe', run: () => context.edit('designer_program', {
            action: 'remove_keyframe', program, target: track.target, property: track.property, time: frame.time,
        }) },
    ]);
    await item?.run();
}

export async function chooseKeyframeGroupEdit(count: number, structural: boolean, pivot?: number): Promise<KeyframeGroupEdit | undefined> {
    const actions = [
        { action: 'move_keyframes', label: '$(arrow-both) Move by offset…' },
        { action: 'scale_keyframes', label: '$(arrow-swap) Scale from playhead…' },
        { action: 'duplicate_keyframes', label: '$(copy) Duplicate at offset…' },
        { action: 'remove_keyframes', label: '$(trash) Delete' },
    ] as const;
    const available = actions.filter(({ action }) => action === 'scale_keyframes' ? count > 1 && pivot !== undefined
        : structural || action === 'move_keyframes');
    const picked = available.length === 1 ? available[0] :
        await choose(count === 1 ? 'Selected keyframe' : `${count} selected keyframes`, available);
    if (!picked) return;
    if (picked.action === 'remove_keyframes') return { action: picked.action };
    if (picked.action === 'scale_keyframes' && pivot !== undefined) {
        const factor = await editText(`Scale key times around ${Number(pivot.toFixed(6))} s. Use 0.5 to compress or 2 to stretch.`, '2', input =>
            Number.isFinite(Number(input)) && Number(input) > 0 && Number(input) !== 1 ? undefined : 'Enter a positive factor other than 1.');
        if (factor !== undefined) return { action: picked.action, factor: Number(factor), pivot };
        return;
    }
    if (picked.action === 'scale_keyframes') return;
    const value = await editText('Offset in seconds. Use a negative value to move earlier.', '0.1', input =>
        input.trim() && Number.isFinite(Number(input)) && Number(input) !== 0 ? undefined : 'Enter a nonzero number.');
    if (value !== undefined) return { action: picked.action, offset: Number(value) };
}

async function authorKeyframe(context: StudioContext, program: number, duration: number, track: TimelineTrack,
    previous?: TimelineKeyframe, initialValue?: string): Promise<void> {
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
