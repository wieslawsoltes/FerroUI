import type { StageFocus, StageInput } from '../src/stagesProtocol';
import { create, iconButton, labelButton } from './dom';

const navigationKeys: Record<string, string> = {
    Tab: 'Tab', Enter: 'Enter', ' ': 'Space', Backspace: 'Back', Delete: 'Delete',
    ArrowLeft: 'Left', ArrowRight: 'Right', ArrowUp: 'Up', ArrowDown: 'Down',
    Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown',
};

export class StageKeyboard {
    readonly button = iconButton('symbol-key', 'Stage keyboard input', () => this.active ? this.leave(true) : this.enter());
    private readonly capture = create('textarea');
    private readonly status = create('p');
    private active = false;
    private composing = false;

    constructor(private readonly card: HTMLElement, artboard: HTMLElement, id: number, private readonly returnTo: HTMLElement,
        private readonly send: (input: StageInput) => void) {
        const instructions = create('span', 'Tab and Shift+Tab move within the preview. Press Escape to return to the stage toolbar.');
        instructions.id = `stage-keyboard-help-${id}`;
        instructions.hidden = true;
        this.capture.className = 'stage-keyboard-capture';
        this.capture.tabIndex = -1;
        this.capture.setAttribute('aria-describedby', instructions.id);
        this.capture.spellcheck = false;
        this.capture.hidden = this.button.hidden = this.status.hidden = true;
        this.button.setAttribute('aria-pressed', 'false');
        this.status.className = 'stage-frame-status stage-keyboard-status';
        this.status.setAttribute('role', 'status');
        this.button.onpointerdown = event => { if (this.active) event.preventDefault(); };
        this.capture.onblur = () => this.leave(false);
        this.capture.onkeydown = event => this.keydown(event);
        this.capture.oninput = () => { if (!this.composing) this.flushText(); };
        this.capture.addEventListener('compositionstart', () => { this.composing = true; });
        this.capture.addEventListener('compositionend', () => {
            this.composing = false;
            queueMicrotask(() => this.flushText());
        });
        artboard.append(instructions, this.capture, this.status);
    }

    update(name: string, offered: boolean, enabled: boolean): void {
        this.button.hidden = !offered;
        this.button.disabled = !enabled;
        labelButton(this.button, `Keyboard input for ${name}`);
        this.capture.setAttribute('aria-label', `Keyboard input for ${name}`);
        if (!enabled) this.leave(true);
    }

    enter(): void {
        if (this.active || this.button.hidden || this.button.disabled) return;
        this.active = true;
        this.capture.hidden = this.status.hidden = false;
        this.card.dataset.keyboard = 'true';
        this.button.setAttribute('aria-pressed', 'true');
        this.status.textContent = 'Esc to exit';
        this.capture.focus({ preventScroll: true });
        this.send({ kind: 'focus' });
    }

    focused(focus: StageFocus | null): void {
        if (!this.active) return;
        const message = focus ? [focus.name, focus.role].filter(Boolean).join(' · ') : 'No focusable control';
        const label = `${message} · Esc to exit`;
        if (this.status.textContent !== label) this.status.textContent = label;
    }

    private leave(restore: boolean): void {
        if (!this.active) return;
        this.active = this.composing = false;
        this.capture.value = '';
        this.capture.hidden = this.status.hidden = true;
        this.button.setAttribute('aria-pressed', 'false');
        delete this.card.dataset.keyboard;
        if (restore) (this.button.disabled || this.button.hidden ? this.returnTo : this.button).focus({ preventScroll: true });
    }

    private keydown(event: KeyboardEvent): void {
        if (event.isComposing || this.composing) return;
        if (event.key === 'Escape') {
            event.preventDefault();
            event.stopPropagation();
            this.leave(true);
            return;
        }
        const command = event.ctrlKey || event.metaKey;
        const key = Object.hasOwn(navigationKeys, event.key) ? navigationKeys[event.key]
            : command && /^[azy]$/i.test(event.key) ? event.key.toUpperCase() : undefined;
        if (!key || (key === 'Tab' && (command || event.altKey))) return;
        event.preventDefault();
        event.stopPropagation();
        const modifiers = [event.ctrlKey && 'Ctrl', event.altKey && 'Alt', event.shiftKey && 'Shift', event.metaKey && 'Meta'];
        this.send({ kind: 'key', key: [...modifiers.filter(Boolean), key].join('+') });
    }

    private flushText(): void {
        const text = this.capture.value;
        this.capture.value = '';
        if (this.active && text) this.send({ kind: 'text', text });
    }
}
