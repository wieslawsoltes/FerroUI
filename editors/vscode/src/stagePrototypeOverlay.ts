import type { OverlayOptions } from './stagesProtocol';
import { choose, editText } from './studioContext';

const positions = [
    ['Center', 'center'], ['Top left', 'top_left'], ['Top center', 'top_center'], ['Top right', 'top_right'],
    ['Center left', 'center_left'], ['Center right', 'center_right'], ['Bottom left', 'bottom_left'],
    ['Bottom center', 'bottom_center'], ['Bottom right', 'bottom_right'], ['Enter coordinates…', 'manual'],
] as const;

export async function editOverlayOptions(previous?: OverlayOptions): Promise<OverlayOptions | undefined> {
    const selected = await choose('Overlay placement', positions.map(([label, value]) => ({
        label, value, picked: (previous?.placement.kind ?? 'center') === value,
    })));
    if (!selected) return;
    const placement: OverlayOptions['placement'] = { kind: selected.value };
    if (selected.value === 'manual') {
        const x = await coordinate('Overlay horizontal offset', previous?.placement.x);
        if (x === undefined) return;
        const y = await coordinate('Overlay vertical offset', previous?.placement.y);
        if (y === undefined) return;
        Object.assign(placement, { x, y });
    }
    const behavior = await choose('Overlay backdrop and dismissal', [
        { label: 'Dim and dismiss on outside click', dim_background: true, dismiss_on_outside: true },
        { label: 'Dim and keep open on outside click', dim_background: true, dismiss_on_outside: false },
        { label: 'Clear and dismiss on outside click', dim_background: false, dismiss_on_outside: true },
        { label: 'Clear and keep open on outside click', dim_background: false, dismiss_on_outside: false },
    ].map(item => ({ ...item, picked: item.dim_background === (previous?.dim_background ?? true)
        && item.dismiss_on_outside === (previous?.dismiss_on_outside ?? false) })));
    if (behavior) return { placement, dim_background: behavior.dim_background, dismiss_on_outside: behavior.dismiss_on_outside };
}

async function coordinate(prompt: string, previous = 0): Promise<number | undefined> {
    const input = await editText(prompt, String(previous));
    if (input === undefined) return;
    const value = Number(input);
    if (!input.trim() || !Number.isSafeInteger(value) || Math.abs(value) > 16_384) {
        throw new Error('Enter a whole number between -16384 and 16384 logical units.');
    }
    return value;
}
