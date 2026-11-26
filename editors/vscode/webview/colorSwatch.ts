import { create } from './dom';

export function colorPicker(key: string, argb: string | null | undefined, set: (value: string) => void): HTMLInputElement {
    const input = create('input');
    input.type = 'color';
    input.className = 'palette-picker';
    input.value = argb ? `#${argb.slice(3)}` : '#000000';
    input.title = `Choose ${key} color`;
    input.setAttribute('aria-label', input.title);
    input.addEventListener('change', () => set(`#${argb?.slice(1, 3) ?? 'FF'}${input.value.slice(1)}`));
    return input;
}

export function swatch(argb: string | undefined): HTMLElement {
    const result = create('span');
    result.className = 'palette-swatch';
    result.setAttribute('aria-hidden', 'true');
    if (argb && /^#[0-9A-F]{8}$/i.test(argb)) result.style.backgroundColor = `#${argb.slice(3)}${argb.slice(1, 3)}`;
    return result;
}
