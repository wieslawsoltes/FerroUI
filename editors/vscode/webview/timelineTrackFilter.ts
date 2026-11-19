import { element } from './dom';

export class TimelineTrackFilter {
    private readonly toggle = element<HTMLButtonElement>('timeline-filter');
    private readonly search = element<HTMLInputElement>('timeline-track-search');
    private readonly status = element('timeline-filter-status');

    constructor(private readonly tracks: HTMLOListElement, update: () => void) {
        this.toggle.onclick = () => {
            if (this.search.hidden) {
                this.search.hidden = false;
                this.toggle.setAttribute('aria-expanded', 'true');
                this.search.focus();
            } else close();
        };
        const close = () => {
            this.search.hidden = true;
            this.search.value = '';
            this.toggle.setAttribute('aria-expanded', 'false');
            this.toggle.focus();
            update();
        };
        this.search.oninput = update;
        this.search.onkeydown = event => {
            if (event.key !== 'Escape') return;
            event.preventDefault();
            close();
        };
    }

    apply(): void {
        const terms = this.search.value.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
        const rows = this.tracks.querySelectorAll<HTMLLIElement>('.timeline-track');
        let anyVisible = false;
        for (const row of rows) {
            const label = row.dataset.searchLabel ?? '';
            row.hidden = !terms.every(term => label.includes(term));
            if (!row.hidden) anyVisible = true;
        }
        this.status.hidden = !terms.length || anyVisible;
    }

    show(hasTracks: boolean): void {
        this.toggle.hidden = !hasTracks;
        this.search.hidden = !hasTracks || this.toggle.getAttribute('aria-expanded') !== 'true';
    }
}
