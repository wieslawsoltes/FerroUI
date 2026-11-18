export function element<T extends HTMLElement>(id: string): T {
    return document.getElementById(id) as T;
}

export function create<K extends keyof HTMLElementTagNameMap>(tag: K, text?: string): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (text !== undefined) node.textContent = text;
    return node;
}

export function choices(values: readonly string[], name: string): HTMLSelectElement {
    const select = create('select');
    select.setAttribute('aria-label', name);
    for (const value of values) select.add(new Option(value, value));
    return select;
}
