import { choose, editText, type StudioContext } from './studioContext';
import type { AuthoredTimeline } from './timelineProtocol';

const playback = { Auto: 'Automatic', Always: 'Always', OnlyIfVisible: 'Only when visible' };
const directions = { Normal: 'Normal', Reverse: 'Reverse', Alternate: 'Alternate', AlternateReverse: 'Alternate in reverse' };
const fills = { None: 'None', Forward: 'Forward', Backward: 'Backward', Both: 'Both' };

export async function editAnimationTiming(context: StudioContext, program: number, timeline: AuthoredTimeline): Promise<void> {
    const update = (property: string, value: number | string) =>
        context.edit('designer_program', { action: 'update_timing', program, timing: { [property]: value } });

    const number = (property: 'duration' | 'delay' | 'delay_between_iterations' | 'speed_ratio', label: string,
        unit: string, strictlyPositive: boolean, prompt?: string) => ({
        label,
        description: `${timeline[property]}${unit}`,
        run: async () => {
            const value = await editText(prompt ?? label, String(timeline[property]), input => {
                const numeric = input.trim() ? Number(input) : NaN;
                if (Number.isFinite(numeric) && (strictlyPositive ? numeric > 0 : numeric >= 0)) return;
                return strictlyPositive ? 'Enter a positive number.' : 'Enter a nonnegative number.';
            });
            if (value !== undefined && Number(value) !== timeline[property]) await update(property, Number(value));
        },
    });
    const choice = (property: 'playback_behavior' | 'playback_direction' | 'fill', label: string, options: Record<string, string>) => ({
        label,
        description: options[timeline[property]],
        run: async () => {
            const picked = await choose(label, Object.entries(options).map(([value, text]) => ({
                label: text, description: value === timeline[property] ? 'Current' : undefined, value,
            })));
            if (picked && picked.value !== timeline[property]) await update(property, picked.value);
        },
    });
    const picked = await choose('Animation Timing and Playback', [
        number('duration', 'Duration', ' s', true, 'Duration in seconds. Keyframe positions scale with the duration.'),
        number('delay', 'Start delay', ' s', false, 'Start delay in seconds'),
        number('delay_between_iterations', 'Delay between iterations', ' s', false, 'Delay between iterations in seconds'),
        number('speed_ratio', 'Playback speed', '×', true),
        {
            label: 'Repeat',
            description: timeline.repeat_text,
            run: async () => {
                const value = (await editText('Repeat count or Infinite', timeline.repeat_text, input =>
                    /^(?:infinite|0*[1-9]\d*)$/i.test(input.trim()) ? undefined : 'Enter a positive integer or Infinite.'))?.trim();
                if (value && value !== timeline.repeat_text) await update('repeat', value);
            },
        },
        choice('playback_direction', 'Direction', directions),
        choice('fill', 'Fill mode', fills),
        choice('playback_behavior', 'Playback behavior', playback),
        {
            label: 'Easing',
            description: timeline.easing_text,
            run: async () => {
                const value = (await editText('Native easing name, four cubic Bézier coordinates, or Spring settings', timeline.easing_text,
                    input => input.trim() ? undefined : 'Enter an easing.'))?.trim();
                if (value && value !== timeline.easing_text) await update('easing', value);
            },
        },
    ]);
    await picked?.run();
}
