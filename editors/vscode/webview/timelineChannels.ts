import type { TimelineTrack } from '../src/timelineProtocol';
import { create, element, icon } from './dom';
import { timelinePercent, visibleTime, type TimelineViewport } from './timelineViewport';

interface PropertyRow {
    element: HTMLLIElement;
    searchLabel: string;
    times: number[];
}

interface Channel {
    target: number;
    element: HTMLLIElement;
    toggle: HTMLButtonElement;
    properties: HTMLOListElement;
    rows: PropertyRow[];
    summary: HTMLElement;
    timeMarks: Map<number, HTMLElement>;
}

export class TimelineChannels {
    private readonly tracks = element<HTMLOListElement>('timeline-tracks');
    private readonly status = element('timeline-filter-status');
    private readonly collapsed = new Set<number>();
    private channels: Channel[] = [];

    constructor(private readonly update: () => void) {}

    render(tracks: readonly TimelineTrack[], names: Record<number, string>, reset: boolean,
        renderTrack: (track: TimelineTrack, index: number, name: string) => HTMLLIElement): void {
        if (reset) this.collapsed.clear();
        const grouped = new Map<number, { track: TimelineTrack; index: number }[]>();
        tracks.forEach((track, index) => {
            const group = grouped.get(track.target);
            if (group) group.push({ track, index });
            else grouped.set(track.target, [{ track, index }]);
        });
        this.channels = [...grouped].map(([target, group]) => {
            const name = names[target] ?? `Target ${target}`;
            const channel = this.channel(target, name);
            channel.rows = group.map(({ track, index }) => ({
                element: renderTrack(track, index, name),
                searchLabel: `${name} ${track.property}`.toLocaleLowerCase(),
                times: track.keyframes.map(key => key.time),
            }));
            channel.properties.append(...channel.rows.map(row => row.element));
            return channel;
        });
        this.tracks.replaceChildren(...this.channels.map(channel => channel.element));
    }

    filter(active: boolean, matches: (target: number, label: string) => boolean): void {
        let anyVisible = false;
        for (const channel of this.channels) {
            const { target, element, toggle, rows, properties, summary, timeMarks } = channel;
            const collapsed = this.collapsed.has(target) && !active;
            for (const row of rows) row.element.hidden = !matches(target, row.searchLabel);
            element.hidden = rows.every(row => row.element.hidden);
            anyVisible ||= !element.hidden;
            toggle.setAttribute('aria-expanded', String(!collapsed));
            toggle.setAttribute('aria-disabled', String(active));
            properties.hidden = collapsed;
            summary.hidden = !collapsed;
            if (collapsed && !timeMarks.size) {
                for (const time of new Set(rows.flatMap(row => row.times))) {
                    const mark = create('span');
                    timeMarks.set(time, mark);
                    summary.append(mark);
                }
            }
        }
        this.status.hidden = !active || anyVisible;
    }

    layout(viewport: TimelineViewport): void {
        for (const { summary, timeMarks } of this.channels) {
            if (summary.hidden) continue;
            for (const [time, mark] of timeMarks) {
                mark.hidden = !visibleTime(viewport, time);
                if (!mark.hidden) mark.style.left = `${timelinePercent(viewport, time)}%`;
            }
        }
    }

    private channel(target: number, name: string): Channel {
        const row = create('li');
        row.className = 'timeline-channel';
        const header = create('div');
        header.className = 'timeline-channel-header';
        const properties = create('ol');
        properties.id = `timeline-channel-${target}`;
        properties.setAttribute('aria-label', `${name} properties`);
        const toggle = create('button');
        toggle.type = 'button';
        toggle.title = name;
        toggle.setAttribute('aria-controls', properties.id);
        toggle.append(icon('chevron-right'), create('span', name));
        toggle.onclick = () => {
            if (toggle.getAttribute('aria-disabled') === 'true') return;
            if (!this.collapsed.delete(target)) this.collapsed.add(target);
            this.update();
        };
        const summary = create('div');
        summary.className = 'timeline-summary';
        summary.setAttribute('aria-hidden', 'true');
        header.append(toggle, summary);
        row.append(header, properties);
        return { target, element: row, toggle, properties, rows: [], summary, timeMarks: new Map() };
    }
}
