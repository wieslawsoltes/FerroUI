import type { PrototypeCommand, PrototypeScene, StageModel, StagesRequest } from '../src/stagesProtocol';
import { create, element, icon, iconButton } from './dom';
import { PrototypeArtboard } from './prototypeArtboard';
import type { StageCard } from './stageCard';

export class PrototypePlayer {
    private readonly controls = element('prototype-actions');
    private key?: string;
    private returnFocus?: HTMLElement;
    private frame?: number;
    readonly artboard: PrototypeArtboard;

    constructor(private readonly board: HTMLElement, private readonly send: (request: StagesRequest) => void) {
        this.artboard = new PrototypeArtboard(element('stages-prototype'), send);
        this.board.before(this.artboard.element);
    }

    update(scene: PrototypeScene | null, model: StageModel | undefined, cards: ReadonlyMap<number, StageCard>): void {
        this.frame = scene?.frame;
        this.board.classList.toggle('prototype-view', Boolean(scene) && this.frame === undefined);
        const base = scene && cards.get(scene.screen)?.stage.layout_size;
        for (const [id, { element, stage }] of cards) {
            const layer = this.frame === undefined ? scene?.overlays.find(overlay => overlay.stage === id) : undefined;
            element.hidden = !!scene && (this.frame !== undefined || (id !== scene.screen && !layer));
            element.classList.toggle('prototype-overlay', Boolean(layer));
            element.classList.toggle('dim-background', Boolean(layer?.options.dim_background));
            for (const field of ['left', 'top', 'width', 'z-index']) element.style.removeProperty(field);
            if (layer && base) {
                const size = stage.layout_size;
                const { kind, x, y } = layer.options.placement;
                const place = (size: number, parent: number, offset: number) => 100 * Math.max(0, Math.min(offset, parent - size)) / parent;
                const left = x ?? (kind.endsWith('left') ? 0 : (base.width - size.width) * (kind.endsWith('right') ? 1 : .5));
                const top = y ?? (kind.startsWith('top') ? 0 : (base.height - size.height) * (kind.startsWith('bottom') ? 1 : .5));
                element.style.left = `${place(size.width, base.width, left)}%`;
                element.style.top = `${place(size.height, base.height, top)}%`;
                element.style.width = `${Math.min(100, 100 * size.width / base.width)}%`;
                element.style.zIndex = String(1 + scene!.overlays.indexOf(layer));
            }
        }
        const current = scene && cards.get(scene.current)?.stage.configuration.name;
        this.artboard.update(scene, current || 'Prototype');
        const actions = scene ? (model?.manifest.prototype?.links ?? []).flatMap((link, index) =>
            model?.stages[link.source]?.id === scene.current && (scene.links?.includes(index) ?? true) ? [{ name: link.name, index }] : []) : [];
        const key = JSON.stringify([scene?.flow, scene?.current, scene?.can_go_back, scene?.overlays.length, scene?.animating, this.frame !== undefined, current, actions]);
        if (key === this.key) return;
        this.key = key;
        const focus = document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
        const previous = focus?.dataset.playerAction;
        if (this.controls.hidden && scene) this.returnFocus = focus;
        this.controls.hidden = !scene;
        if (!scene) {
            this.controls.replaceChildren();
            // The board reveals its toolbar in the same turn.
            const target = this.returnFocus;
            this.returnFocus = undefined;
            queueMicrotask(() => { if (target?.isConnected) target.focus({ preventScroll: true }); });
            return;
        }
        const navigation = create('div');
        navigation.className = 'tool-group';
        const button = (glyph: string, label: string, command: PrototypeCommand) => {
            const button = iconButton(glyph, label, () => this.send({ type: 'prototype', command, frame: this.frame }));
            button.dataset.playerAction = command.action;
            return button;
        };
        const back = button('arrow-left', 'Back in prototype', { action: 'back' });
        back.disabled = !scene.can_go_back;
        navigation.append(back, button('refresh', 'Restart prototype', { action: 'restart' }));
        if (scene.overlays.length) navigation.append(button('close', 'Close overlay', { action: 'closeOverlay' }));
        if (this.frame !== undefined) navigation.append(this.artboard.keyboardButton);
        const location = create('div');
        location.className = 'prototype-location';
        location.append(create('strong', scene.flow));
        if (current && current !== scene.flow) {
            const path = create('span', current);
            path.className = 'prototype-current';
            location.append(icon('chevron-right'), path);
        }
        const triggers = create('div');
        triggers.className = 'prototype-triggers';
        triggers.setAttribute('role', 'group');
        triggers.setAttribute('aria-label', 'Screen interactions');
        for (const { name, index } of actions) {
            const control = create('button', name);
            control.className = 'prototype-trigger';
            control.append(icon('arrow-right'));
            control.dataset.playerAction = String(index);
            control.disabled = scene.animating === true;
            control.onclick = () => this.send({ type: 'prototype', command: { action: 'activate', link: index }, frame: this.frame });
            triggers.append(control);
        }
        const close = button('close', 'Exit Prototype', { action: 'stop' });
        this.controls.replaceChildren(navigation, location, triggers, close);
        if (previous !== undefined) {
            const next = this.controls.querySelector<HTMLButtonElement>(`button[data-player-action="${CSS.escape(previous)}"]`);
            (next && !next.disabled ? next : close).focus({ preventScroll: true });
        } else if (this.returnFocus === focus) {
            close.focus({ preventScroll: true });
        }
    }
}
