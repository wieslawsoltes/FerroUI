import type { CanvasGuidesState, Guide, GuideAxis } from '../src/canvasGuidesProtocol';
import { CanvasRulers, guidePixel, guidePosition, type GuideProjection } from './canvasRulers';
import { create, element } from './dom';

interface Mark { axis: GuideAxis; position: number }

export class GuideOverlay {
    private readonly element = element('guide-overlay');
    private readonly content = this.element.closest<HTMLElement>('.designer-content')!;
    private readonly lines = element('guide-lines');
    private readonly ghost = element('guide-ghost');
    private readonly rulers = new CanvasRulers();
    private readonly buttons = new Map<number, HTMLButtonElement>();

    draw(state: CanvasGuidesState | undefined, view: GuideProjection | undefined, enabled: boolean, ghost?: Mark): void {
        this.element.hidden = !state || !view;
        if (!state || !view) return;
        const { settings } = state;
        const rail = rulerSize(state, view);
        this.element.style.inlineSize = `${view.width}px`;
        this.element.style.blockSize = `${view.height}px`;
        this.element.style.setProperty('--rail-size', `${rail}px`);
        this.content.style.setProperty('--canvas-ruler-size', `${rail}px`);
        this.lines.inert = !enabled;
        this.rulers.draw(view, state.ruler, rail !== 0);

        const guides = settings.guides_visible ? settings.guides : [];
        const ids = new Set(guides.map(guide => guide.id));
        for (const [id, button] of this.buttons) {
            if (!ids.has(id)) { button.remove(); this.buttons.delete(id); }
        }
        for (const guide of guides) {
            let button = this.buttons.get(guide.id);
            if (!button) {
                button = create('button');
                button.type = 'button';
                button.className = 'guide-line';
                button.dataset.guide = String(guide.id);
                const label = create('span');
                label.setAttribute('aria-hidden', 'true');
                button.append(label);
                this.lines.append(button);
                this.buttons.set(guide.id, button);
            }
            positionLine(button, guide, view, rail);
        }
        if (ghost) positionLine(this.ghost, ghost, view, rail);
        else this.ghost.hidden = true;
    }

    focus(id: number): boolean {
        const button = this.buttons.get(id);
        if (!button || button.hidden) return false;
        button.focus({ preventScroll: true });
        return document.activeElement === button;
    }
}

function positionLine(line: HTMLElement, mark: Mark, view: GuideProjection, rail: number): void {
    const pixel = guidePixel(mark.axis, mark.position, view);
    const horizontal = mark.axis === 'Horizontal';
    const edge = horizontal ? view.height : view.width;
    line.hidden = pixel < rail || pixel >= edge;
    line.dataset.axis = mark.axis;
    line.toggleAttribute('data-flip-label', pixel > edge - 32);
    line.style.setProperty('--guide-pixel', `${pixel}px`);
    const coordinate = `${horizontal ? 'Y' : 'X'} ${guidePosition(mark.position)}`;
    line.firstElementChild!.textContent = coordinate;
    if (line instanceof HTMLButtonElement) line.setAttribute('aria-label', `${mark.axis} guide, ${coordinate} logical pixels`);
}

export function rulerSize(state: CanvasGuidesState, view: GuideProjection): number {
    const size = state.ruler.thickness;
    return state.settings.rulers_visible && view.width > size && view.height > size ? size : 0;
}

export function rulerAt(view: GuideProjection, ruler: number, event: MouseEvent): GuideAxis | undefined {
    const x = event.clientX - view.left;
    const y = event.clientY - view.top;
    if (x < 0 || y < 0 || x >= view.width || y >= view.height) return;
    if (x >= ruler && y < ruler) return 'Horizontal';
    if (x < ruler && y >= ruler) return 'Vertical';
}

export function guideAt(state: CanvasGuidesState, view: GuideProjection, ruler: number, event: MouseEvent): Guide | undefined {
    const x = event.clientX - view.left;
    const y = event.clientY - view.top;
    if (!state.settings.guides_visible || x < ruler || y < ruler || x >= view.width || y >= view.height) return;
    let closest: Guide | undefined;
    let distance = 5;
    for (const guide of state.settings.guides) {
        const horizontal = guide.axis === 'Horizontal';
        const pixel = guidePixel(guide.axis, guide.position, view);
        const gap = Math.abs(pixel - (horizontal ? y : x));
        if (pixel >= ruler && pixel < (horizontal ? view.height : view.width) && gap <= distance) {
            closest = guide;
            distance = gap;
        }
    }
    return closest;
}
