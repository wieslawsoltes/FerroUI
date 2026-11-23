import { anchorPopover, element } from './dom';
import type { TimelineTrackFilterState } from '../src/timelineTrackProtocol';

export class TimelineTrackFilter {
    private readonly toggle = element<HTMLButtonElement>('timeline-filter');
    private readonly popup = element('timeline-track-filter');
    private readonly search = element<HTMLInputElement>('timeline-track-search');
    private readonly selectedOnly = element<HTMLInputElement>('timeline-selected-targets');
    private selected = new Set<number>();
    private terms: string[] = [];

    constructor(private readonly update: () => void) {
        anchorPopover(this.toggle, this.popup);
        this.popup.addEventListener('toggle', event => {
            this.toggle.setAttribute('aria-expanded', String(event.newState === 'open'));
        });
        this.search.oninput = this.selectedOnly.onchange = () => this.apply();
    }

    get active(): boolean { return this.selectedOnly.checked || this.terms.length > 0; }

    get state(): TimelineTrackFilterState { return { query: this.search.value, selectedOnly: this.selectedOnly.checked }; }

    setFilter({ query, selectedOnly }: TimelineTrackFilterState): void {
        if (this.search.value === query && this.selectedOnly.checked === selectedOnly) return;
        this.search.value = query;
        this.selectedOnly.checked = selectedOnly;
        this.apply();
    }

    matches(target: number, label: string): boolean {
        return (!this.selectedOnly.checked || this.selected.has(target)) && this.terms.every(term => label.includes(term));
    }

    follow(hasTracks: boolean, selected: readonly number[]): boolean {
        this.toggle.hidden = !hasTracks;
        if (!hasTracks) this.popup.hidePopover();
        const changed = selected.length !== this.selected.size || selected.some(target => !this.selected.has(target));
        if (changed) this.selected = new Set(selected);
        return changed && this.selectedOnly.checked;
    }

    private apply(): void {
        this.terms = this.search.value.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
        this.toggle.toggleAttribute('data-filtered', this.active);
        this.update();
    }
}
