import type { KeyboardFocus, KeyboardInput } from '../src/previewKeyboardProtocol';
import { create, iconButton, labelButton } from './dom';

const navigationKeys: Record<string, string> = {
    Tab: 'Tab', Enter: 'Enter', ' ': 'Space', Backspace: 'Back', Delete: 'Delete',
    ArrowLeft: 'Left', ArrowRight: 'Right', ArrowUp: 'Up', ArrowDown: 'Down',
    Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown', Escape: 'Escape',
};

interface CaptureOptions { nativeEscape?: boolean; onExit?: () => void }

export class PreviewKeyboard {
    readonly button = iconButton('keyboard', 'Preview keyboard input', () => this.active ? this.leave(true) : this.enter());
    private readonly capture = create('textarea');
    private readonly status = create('p');
    private readonly announcement = create('p');
    private active = false;
    private composing = false;

    constructor(private readonly surface: HTMLElement, artboard: HTMLElement, id: number, private readonly returnTo: HTMLElement,
        private readonly send: (input: KeyboardInput) => void, private readonly options: CaptureOptions = {}) {
        const instructions = create('span', options.nativeEscape
            ? 'Tab and Shift+Tab move within the active view. Escape closes an overlay or exits the prototype.'
            : 'Tab and Shift+Tab move within the preview. Press Escape to return to the preview toolbar.');
        instructions.id = `preview-keyboard-help-${id}`;
        instructions.hidden = true;
        this.capture.className = 'preview-keyboard-capture';
        this.capture.tabIndex = -1;
        this.button.setAttribute('aria-describedby', instructions.id);
        this.capture.setAttribute('aria-describedby', instructions.id);
        this.capture.spellcheck = false;
        this.capture.hidden = this.button.hidden = this.status.hidden = true;
        this.button.setAttribute('aria-pressed', 'false');
        this.status.className = 'preview-keyboard-status';
        this.status.setAttribute('aria-hidden', 'true');
        this.announcement.className = 'preview-keyboard-announcement';
        this.announcement.setAttribute('role', 'status');
        this.announcement.setAttribute('aria-atomic', 'true');
        this.button.onpointerdown = event => { if (this.active) event.preventDefault(); };
        this.capture.onblur = () => this.leave(false);
        this.capture.onkeydown = event => this.keydown(event);
        this.capture.oninput = () => { if (!this.composing) this.flushText(); };
        this.capture.addEventListener('compositionstart', () => { this.composing = true; });
        this.capture.addEventListener('compositionend', () => {
            this.composing = false;
            queueMicrotask(() => this.flushText());
        });
        artboard.append(instructions, this.capture, this.status, this.announcement);
    }

    update(name: string, offered: boolean, enabled: boolean): void {
        this.button.hidden = !offered;
        this.button.disabled = !enabled;
        labelButton(this.button, `Keyboard input for ${name}`);
        this.capture.setAttribute('aria-label', `Keyboard input for ${name}`);
        if (!enabled) this.leave(true);
    }

    get capturing(): boolean { return this.active; }

    enter(): void {
        if (this.active || this.button.hidden || this.button.disabled) return;
        this.active = true;
        this.capture.hidden = this.status.hidden = false;
        this.surface.dataset.keyboard = 'true';
        this.button.setAttribute('aria-pressed', 'true');
        this.status.textContent = this.shortcut;
        this.capture.focus({ preventScroll: true });
        this.send({ kind: 'focus' });
    }

    focused(focus: KeyboardFocus | null): void {
        if (!this.active) return;
        const parts = focus ? [focus.name, focus.role === 'Edit' ? 'Text field' : focus.role].filter(Boolean)
            : ['No focusable control'];
        const label = [...parts, this.shortcut].join(' · ');
        if (this.status.textContent !== label) this.status.textContent = label;
        const spoken = parts.join(', ');
        if (this.announcement.textContent !== spoken) this.announcement.textContent = spoken;
    }

    private get shortcut(): string { return this.options.nativeEscape ? 'Esc to go back' : 'Esc to exit'; }

    leave(restore: boolean): void {
        if (!this.active) return;
        this.active = this.composing = false;
        this.capture.value = '';
        this.capture.hidden = this.status.hidden = true;
        this.announcement.textContent = '';
        this.button.setAttribute('aria-pressed', 'false');
        delete this.surface.dataset.keyboard;
        this.options.onExit?.();
        if (restore) (this.button.disabled || this.button.hidden ? this.returnTo : this.button).focus({ preventScroll: true });
    }

    private keydown(event: KeyboardEvent): void {
        if (event.isComposing || this.composing) return;
        const key = previewInputKey(event);
        if (!key) return;
        event.preventDefault();
        event.stopPropagation();
        if (key === 'Escape' && !this.options.nativeEscape) this.leave(true);
        else this.send({ kind: 'key', key });
    }

    private flushText(): void {
        const text = this.capture.value;
        this.capture.value = '';
        if (this.active && text) this.send({ kind: 'text', text });
    }
}

export function previewInputKey({ key, ctrlKey, altKey, shiftKey, metaKey }: Pick<KeyboardEvent,
    'key' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>): string | undefined {
    // Keep the VoiceOver modifier and workbench navigation available during capture.
    if (ctrlKey && altKey) return;
    const command = ctrlKey || metaKey;
    if (key === 'Tab' && (command || altKey)) return;
    if (key === 'Escape' && (command || altKey || shiftKey)) return;
    const native = Object.hasOwn(navigationKeys, key) ? navigationKeys[key]
        : command && /^[azy]$/i.test(key) ? key.toUpperCase() : undefined;
    if (!native) return;
    return [ctrlKey && 'Ctrl', altKey && 'Alt', shiftKey && 'Shift', metaKey && 'Meta', native].filter(Boolean).join('+');
}
