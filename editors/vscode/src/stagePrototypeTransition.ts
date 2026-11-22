import { prototypeEffects, type PrototypeEasing, type PrototypeTransition } from './stagesProtocol';
import { choose, editText } from './studioContext';

const effects = Object.entries(prototypeEffects) as Array<[PrototypeTransition['effect'], string]>;
const easings: Array<{ label: string; value: PrototypeEasing }> = [
    { label: 'Linear', value: 'linear' }, { label: 'Ease in', value: 'ease_in' },
    { label: 'Ease out', value: 'ease_out' }, { label: 'Ease in and out', value: 'ease_in_out' },
];

export async function editPrototypeTransition(previous: PrototypeTransition | undefined): Promise<PrototypeTransition | null | undefined> {
    const effect = await choose('Prototype transition', [
        { label: 'Instant', value: null, picked: previous === undefined },
        ...effects.map(([value, label]) => ({ label, value, picked: value === previous?.effect })),
    ]);
    if (!effect) return;
    if (!effect.value) return null;
    const duration = await editText('Duration (ms)', String(previous?.duration_ms ?? 250), input => {
        if (!/^\d+$/.test(input.trim()) || Number(input) < 1 || Number(input) > 10_000) {
            return 'Enter a whole duration from 1 to 10,000 milliseconds.';
        }
    });
    if (!duration) return;
    const easing = await choose('Transition easing', easings.map(item => ({
        ...item, picked: item.value === (previous?.easing ?? 'ease_in_out'),
    })));
    if (easing) return { effect: effect.value, duration_ms: Number(duration), easing: easing.value };
}
