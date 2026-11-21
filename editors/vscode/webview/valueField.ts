export function submitValue(field: HTMLInputElement | HTMLSelectElement, original: string, apply: (value: string) => void): void {
    let last = original;
    const commit = () => {
        if (field.value === last) return;
        last = field.value;
        apply(field.value);
    };
    field.addEventListener('change', commit);
    if (field instanceof HTMLInputElement) field.addEventListener('keydown', event => {
        if (event.key === 'Enter') { event.preventDefault(); commit(); }
        if (event.key === 'Escape') field.value = last;
    });
}
