import { canvasThemeLabels, DEFAULT_VIEWPORT, viewportTheme, withViewportTheme, type Viewport } from '../src/protocol';
import { anchorPopover, element } from './dom';

// Logical device sizes from Designer Studio that fit the preview host's output limits.
const presets = [
    ['Window', DEFAULT_VIEWPORT.width, DEFAULT_VIEWPORT.height],
    ['Compact phone', 360, 640],
    ['Phone', 390, 844],
    ['Tablet', 768, 1024],
    ['Desktop', 1280, 800],
] as const;

export class ViewportMenu {
    private readonly trigger = element<HTMLButtonElement>('viewport-button');
    private readonly popup = element('viewport-menu');
    private readonly label = element('dimensions');
    private readonly preset = element<HTMLSelectElement>('preset');
    private readonly width = element<HTMLInputElement>('width');
    private readonly height = element<HTMLInputElement>('height');
    private readonly theme = element<HTMLSelectElement>('theme');
    private readonly contrast = element<HTMLOptionElement>('theme-high-contrast');
    private viewport = DEFAULT_VIEWPORT;

    constructor(update: (viewport: Viewport) => void) {
        anchorPopover(this.trigger, this.popup);
        this.preset.replaceChildren(...presets.map(([name, w, h], index) => new Option(`${name} · ${w} × ${h}`, String(index))));
        this.preset.add(new Option('Custom', 'custom'));
        this.popup.addEventListener('beforetoggle', (event: ToggleEvent) => {
            if (event.newState === 'open') this.resetDraft();
        });
        this.preset.addEventListener('change', () => {
            const preset = presets[Number(this.preset.value)];
            if (!preset) { this.width.focus(); return; }
            this.width.valueAsNumber = preset[1];
            this.height.valueAsNumber = preset[2];
        });
        this.width.addEventListener('input', () => this.selectPreset());
        this.height.addEventListener('input', () => this.selectPreset());
        element('rotate').addEventListener('click', () => {
            [this.width.value, this.height.value] = [this.height.value, this.width.value];
            this.selectPreset();
        });
        element<HTMLFormElement>('viewport').addEventListener('submit', event => {
            event.preventDefault();
            if (!this.width.validity.valid || !this.height.validity.valid) return;
            const theme = this.theme.value;
            if (theme !== 'light' && theme !== 'dark' && theme !== 'high_contrast') return;
            const next = withViewportTheme({ ...this.viewport, width: this.width.valueAsNumber, height: this.height.valueAsNumber }, theme);
            const changed = next.width !== this.viewport.width || next.height !== this.viewport.height || theme !== viewportTheme(this.viewport);
            this.show(next);
            this.popup.hidePopover();
            if (changed) update(next);
        });
        this.show(this.viewport);
    }

    show(viewport: Viewport, highContrast?: boolean): void {
        if (highContrast !== undefined) this.contrast.hidden = this.contrast.disabled = !highContrast;
        this.viewport = viewport;
        const label = `${viewport.width} × ${viewport.height}`;
        this.label.textContent = label;
        this.trigger.title = `${label} · ${canvasThemeLabels[viewportTheme(viewport)]} · Preview size and theme`;
    }

    private resetDraft(): void {
        this.width.valueAsNumber = this.viewport.width;
        this.height.valueAsNumber = this.viewport.height;
        this.theme.value = viewportTheme(this.viewport);
        this.selectPreset();
    }

    private selectPreset(): void {
        const index = presets.findIndex(([, w, h]) => w === this.width.valueAsNumber && h === this.height.valueAsNumber);
        this.preset.value = index === -1 ? 'custom' : String(index);
    }
}
