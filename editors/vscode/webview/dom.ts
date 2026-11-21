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

export function icon(glyph: string): HTMLElement {
    const node = create('span');
    node.className = `codicon codicon-${glyph}`;
    node.setAttribute('aria-hidden', 'true');
    return node;
}

export function iconButton(glyph: string, label: string, invoke?: () => void): HTMLButtonElement {
    const button = create('button');
    button.type = 'button';
    button.className = 'tool';
    button.append(icon(glyph));
    labelButton(button, label);
    if (invoke) button.onclick = invoke;
    return button;
}

export function labelButton(button: HTMLButtonElement, label: string, glyph?: string): void {
    if (button.title !== label) {
        button.title = label;
        button.setAttribute('aria-label', label);
    }
    if (glyph) {
        const node = button.querySelector('.codicon');
        if (node) node.className = `codicon codicon-${glyph}`;
    }
}

export function anchorPopover(trigger: HTMLElement | (() => HTMLElement | undefined), popup: HTMLElement): void {
    const position = () => {
        const button = typeof trigger === 'function' ? trigger() : trigger;
        if (!button?.isConnected) return;
        const edge = button.getBoundingClientRect();
        const below = window.innerHeight - edge.bottom;
        const above = below < 235 && edge.top > below;
        popup.style.setProperty('--anchor-right', `${edge.right}px`);
        popup.style.setProperty('--anchor-bottom', `${edge.bottom}px`);
        popup.style.setProperty('--anchor-top', `${edge.top}px`);
        popup.style.setProperty('--popover-room', `${Math.max(0, (above ? edge.top : below) - 14)}px`);
        popup.toggleAttribute('data-opens-above', above);
    };
    popup.addEventListener('beforetoggle', event => { if (event.newState === 'open') position(); });
    window.addEventListener('resize', () => { if (popup.matches(':popover-open')) position(); });
    window.addEventListener('scroll', () => { if (popup.matches(':popover-open')) position(); }, { capture: true, passive: true });
}
