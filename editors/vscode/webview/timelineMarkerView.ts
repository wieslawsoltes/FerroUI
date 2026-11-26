import type { TimelineMarker, TimelineMarkerAction } from '../src/timelineProtocol';
import { anchorPopover, create, element, iconButton, labelButton } from './dom';
import { timelinePercent, visibleTime, type TimelineViewport } from './timelineViewport';

const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });

export class TimelineMarkerView {
    private readonly form: HTMLFormElement;
    private readonly name: HTMLInputElement;
    private readonly add: HTMLButtonElement;
    private readonly list: HTMLElement;
    private readonly strip: HTMLElement;
    private readonly popup: HTMLElement;
    private readonly popupButton: HTMLButtonElement;
    private marks: { time: number; button: HTMLButtonElement }[] = [];
    private last = '';

    constructor(prefix: string, private readonly slider: HTMLInputElement, private readonly viewport: () => TimelineViewport,
        time: () => number, private readonly action: (edit: TimelineMarkerAction, time?: number) => void, range = true) {
        this.form = element(`${prefix}-marker-form`);
        this.name = element(`${prefix}-marker-name`);
        this.add = element(`${prefix}-marker-add`);
        this.list = element(`${prefix}-marker-list`);
        this.strip = element(`${prefix}-marker-strip`);
        this.popup = element(`${prefix}-${range ? 'preview-range' : 'preview-markers'}`);
        this.popupButton = element(`${prefix}-${range ? 'range-options' : 'marker-options'}`);
        if (!range) {
            anchorPopover(this.popupButton, this.popup);
            this.popup.addEventListener('toggle', event => this.popupButton.setAttribute('aria-expanded', String(event.newState === 'open')));
        }
        this.form.onsubmit = event => {
            event.preventDefault();
            const name = this.name.value.trim();
            if (!name || this.add.disabled) return;
            this.action({ action: 'set', name, time: time() });
            this.name.value = '';
        };
    }

    show(markers: readonly TimelineMarker[], disabled: boolean, available = true): void {
        const serialized = JSON.stringify(markers);
        if (serialized !== this.last) {
            this.last = serialized;
            this.list.replaceChildren(...markers.map(marker => this.entry(marker)));
            this.marks = markers.map(({ name, time }) => {
                const button = iconButton('bookmark', `${name} at ${number.format(time)} seconds`, () => this.action({ action: 'seek', name }, time));
                button.classList.add('timeline-marker');
                return { time, button };
            });
            this.strip.replaceChildren(...this.marks.map(mark => mark.button));
        }
        this.form.hidden = !available;
        this.list.hidden = this.strip.hidden = !available || markers.length === 0;
        this.name.disabled = this.add.disabled = disabled;
        for (const button of this.list.querySelectorAll('button')) button.disabled = disabled;
        for (const { button } of this.marks) button.disabled = disabled;
        this.layout();
    }

    layout(): void {
        const axis = this.viewport();
        for (const { time, button } of this.marks) {
            button.hidden = !visibleTime(axis, time);
            if (!button.hidden) button.style.left = `${timelinePercent(axis, time)}%`;
            else if (button === document.activeElement) this.slider.focus({ preventScroll: true });
        }
    }

    private entry({ name, time }: TimelineMarker): HTMLLIElement {
        const row = create('li');
        const jump = create('button', name);
        jump.type = 'button';
        labelButton(jump, `Seek to ${name} at ${number.format(time)} seconds`);
        jump.onclick = () => {
            this.popup.hidePopover();
            this.popupButton.focus({ preventScroll: true });
            this.action({ action: 'seek', name }, time);
        };
        const remove = iconButton('trash', `Remove marker ${name}`, () => {
            this.name.focus();
            this.action({ action: 'remove', name });
        });
        row.append(jump, create('span', `${number.format(time)} s`), remove);
        return row;
    }
}
