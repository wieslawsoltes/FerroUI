import type { PrototypeImage, PrototypeInput, PrototypeScene, StageFocus, StagesRequest } from '../src/stagesProtocol';
import { create } from './dom';
import { StageContacts } from './stageContacts';
import { StageKeyboard } from './stageKeyboard';

/** The native player supplies one composited image and one accepted input target. */
export class PrototypeArtboard {
    readonly element = create('section');
    private readonly image = create('img');
    private readonly placeholder = create('p', 'Rendering prototype…');
    private readonly keyboard: StageKeyboard;
    private readonly contacts: StageContacts;
    private scene?: PrototypeScene;
    private current = true;
    private displayed?: number;
    private pending = 0;
    private title = 'Prototype';

    constructor(returnTo: HTMLElement, private readonly send: (request: StagesRequest) => void) {
        this.element.className = 'prototype-artboard preview-surface';
        this.element.hidden = this.image.hidden = true;
        this.image.draggable = false;
        this.image.alt = '';
        this.placeholder.setAttribute('role', 'status');
        this.placeholder.className = 'stage-placeholder';
        this.element.append(this.placeholder, this.image);
        this.keyboard = new StageKeyboard(this.element, this.element, 0, returnTo,
            input => this.input({ kind: 'keyboard', input }), true);
        this.contacts = new StageContacts(this.image, ({ epoch, input }) => this.input({ kind: 'pointer', input }, epoch), () => this.keyboard.enter());
    }

    get keyboardButton(): HTMLButtonElement { return this.keyboard.button; }

    update(scene: PrototypeScene | null, title: string): void {
        this.scene = scene?.frame !== undefined ? scene : undefined;
        this.title = title;
        this.element.hidden = !this.scene;
        if (!this.scene) {
            this.displayed = undefined;
            this.pending++;
            this.image.removeAttribute('src');
            this.image.hidden = true;
            this.placeholder.hidden = false;
        }
        this.element.setAttribute('aria-label', `Prototype preview · ${title}`);
        this.updateInput();
    }

    setCurrent(current: boolean): void { this.current = current; this.updateInput(); }
    cancelContacts(): void { this.contacts.cancel(); }

    focused(frame: number, focus: StageFocus | null): void {
        if (frame === this.scene?.frame) this.keyboard.focused(focus);
    }

    async present(frame: number, image: PrototypeImage): Promise<void> {
        if (this.scene?.frame !== frame) return;
        const pending = ++this.pending;
        const next = new Image();
        next.src = `data:image/png;base64,${image.png}`;
        try { await next.decode(); } catch { return; }
        if (this.scene?.frame !== frame || pending !== this.pending) return;
        this.image.src = next.src;
        this.image.alt = `${this.title} native preview`;
        this.image.hidden = false;
        this.placeholder.hidden = true;
        this.displayed = image.animating ? undefined : frame;
        this.element.setAttribute('aria-busy', String(image.animating === true));
        this.updateInput();
    }

    private updateInput(): void {
        const scene = this.scene;
        this.keyboard.update(this.title, Boolean(scene), Boolean(scene) && this.current);
        this.contacts.update(scene?.frame !== undefined && this.current && scene.frame === this.displayed
            ? { id: scene.current, epoch: scene.frame } : undefined);
    }

    private input(input: PrototypeInput, expected?: number): void {
        const frame = this.scene?.frame;
        if (this.current && frame !== undefined && (expected === undefined || expected === frame) && (this.displayed === frame
            || (input.kind === 'keyboard' && (input.input.kind === 'focus'
                || (input.input.kind === 'key' && input.input.key === 'Escape'))))) {
            this.send({ type: 'prototypeInput', frame, input });
        }
    }
}
