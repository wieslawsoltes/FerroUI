import { isAnimationGraphMode, type AnimationGraphMode } from '../src/animationGraph';
import { anchorPopover, element, labelButton } from './dom';

export class AnimationGraphMenu {
    private readonly button: HTMLButtonElement;
    private readonly options: HTMLButtonElement[];
    private velocity = false;

    constructor(prefix: string, private selected: AnimationGraphMode, choose: (mode: AnimationGraphMode) => void) {
        this.button = element(`${prefix}-animation-graph`);
        const popup = element(`${prefix}-graph-modes`);
        this.options = [...popup.querySelectorAll<HTMLButtonElement>('button')];
        anchorPopover(this.button, popup);
        popup.addEventListener('toggle', event => this.button.setAttribute('aria-expanded', String(event.newState === 'open')));
        for (const item of this.options) item.onclick = () => {
            popup.hidePopover();
            this.button.focus({ preventScroll: true });
            if (isAnimationGraphMode(item.value)) choose(item.value);
        };
    }

    get mode(): AnimationGraphMode {
        return this.selected === 'velocity' && !this.velocity ? 'value' : this.selected;
    }

    get preference(): AnimationGraphMode { return this.selected; }

    apply(mode: AnimationGraphMode): boolean {
        if (this.selected === mode) return false;
        this.selected = mode;
        return true;
    }

    show(available: boolean, velocity: boolean): void {
        this.velocity = velocity;
        this.button.hidden = !available;
        this.button.setAttribute('aria-pressed', String(available && this.mode !== 'keys'));
        for (const item of this.options) {
            item.hidden = item.value === 'velocity' && !velocity;
            item.setAttribute('aria-pressed', String(item.value === this.mode));
            if (item.value === this.mode) labelButton(this.button, `Animation graph · ${item.textContent?.trim()}`);
        }
    }
}
