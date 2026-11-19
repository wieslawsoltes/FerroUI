import type { TimelineMarker, TimelineMarkerEdit } from '../src/timelineProtocol';
import { create, element, iconButton, labelButton } from './dom';
import { timelinePercent, visibleTime, type TimelineViewport } from './timelineViewport';

const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });

export class TimelineMarkerView {
    private readonly form = element<HTMLFormElement>('timeline-marker-form');
    private readonly name = element<HTMLInputElement>('timeline-marker-name');
    private readonly add = element<HTMLButtonElement>('timeline-marker-add');
    private readonly list = element('timeline-marker-list');
    private readonly strip = element('timeline-marker-strip');
    private readonly popup = element('timeline-preview-range');
    private readonly popupButton = element<HTMLButtonElement>('timeline-range-options');
    private readonly slider = element<HTMLInputElement>('timeline-seek');
    private marks: { time: number; button: HTMLButtonElement }[] = [];
    private last = '';

    constructor(private readonly viewport: () => TimelineViewport, private readonly seek: (time: number) => void,
        private readonly edit: (edit: TimelineMarkerEdit) => void) {
        this.form.onsubmit = event => {
            event.preventDefault();
            const name = this.name.value.trim();
            if (!name || this.add.disabled) return;
            this.edit({ action: 'set', name, time: this.slider.valueAsNumber });
            this.name.value = '';
        };
    }

    show(markers: readonly TimelineMarker[], disabled: boolean): void {
        const serialized = JSON.stringify(markers);
        if (serialized !== this.last) {
            this.last = serialized;
            this.list.replaceChildren(...markers.map(marker => this.entry(marker)));
            this.marks = markers.map(({ name, time }) => {
                const button = iconButton('bookmark', `${name} at ${number.format(time)} seconds`, () => this.seek(time));
                button.classList.add('timeline-marker');
                return { time, button };
            });
            this.strip.replaceChildren(...this.marks.map(mark => mark.button));
            this.list.hidden = this.strip.hidden = markers.length === 0;
        }
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
            this.seek(time);
        };
        const remove = iconButton('trash', `Remove marker ${name}`, () => {
            this.name.focus();
            this.edit({ action: 'remove', name });
        });
        row.append(jump, create('span', `${number.format(time)} s`), remove);
        return row;
    }
}
