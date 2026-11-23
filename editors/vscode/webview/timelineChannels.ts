import type { TimelineTrack } from '../src/timelineProtocol';
import type { TimelineTrackGroup, TimelineTrackInfo } from '../src/timelineTrackProtocol';
import { create, element, icon, iconButton, labelButton } from './dom';
import { timelinePercent, visibleTime, type TimelineViewport } from './timelineViewport';

interface PropertyRow {
    element: HTMLLIElement;
    index: number;
    property: string;
    searchLabel: string;
    times: number[];
}

interface Channel {
    target: number;
    name: string;
    element: HTMLLIElement;
    toggle: HTMLButtonElement;
    lock: HTMLButtonElement;
    properties: HTMLOListElement;
    rows: PropertyRow[];
    summary: HTMLElement;
    timeMarks: Map<number, HTMLElement>;
}

export class TimelineChannels {
    private readonly tracks = element<HTMLOListElement>('timeline-tracks');
    private readonly status = element('timeline-filter-status');
    private readonly collapsed = new Set<number>();
    private readonly locked = new Set<number>();
    private channels: Channel[] = [];

    constructor(private readonly update: () => void) {}

    render(tracks: readonly TimelineTrack[], names: Record<number, string>, reset: boolean,
        renderTrack: (track: TimelineTrack, index: number, name: string) => HTMLLIElement): void {
        if (reset) {
            this.collapsed.clear();
            this.locked.clear();
        }
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
                index,
                property: track.property,
                searchLabel: `${name} ${track.property}`.toLocaleLowerCase(),
                times: track.keyframes.map(key => key.time),
            }));
            channel.properties.append(...channel.rows.map(row => row.element));
            return channel;
        });
        this.tracks.replaceChildren(...this.channels.map(channel => channel.element));
    }

    isLocked(target: number): boolean { return this.locked.has(target); }

    get hasLocks(): boolean { return this.channels.some(({ target }) => this.isLocked(target)); }

    view(): { groups: TimelineTrackGroup[]; tracks: TimelineTrackInfo[] } {
        const groups: TimelineTrackGroup[] = [];
        const tracks: TimelineTrackInfo[] = [];
        for (const group of this.channels) {
            const { target, name, element, properties, rows } = group;
            const visible = !element.hidden;
            groups.push({ target, name, visible, expanded: !properties.hidden, locked: this.isLocked(target),
                canCollapse: this.canCollapse(group), canLock: this.canLock(group) });
            for (const row of rows) tracks.push({ target, name, property: row.property,
                visible: visible && !properties.hidden && !row.element.hidden });
        }
        return { groups, tracks };
    }

    setCollapsed(target: number, collapsed: boolean): void {
        const group = this.channels.find(group => group.target === target);
        if (!group || !this.canCollapse(group)) throw new Error('Choose a visible target group with the track filter cleared.');
        this.collapse(group, collapsed);
    }

    setLocked(target: number, locked: boolean): void {
        const group = this.channels.find(group => group.target === target);
        if (!group || !this.canLock(group)) throw new Error('The visible target group cannot be locked or unlocked now.');
        this.lock(group, locked);
    }

    editableTracks(): number[] {
        return this.channels.flatMap(({ target, properties, rows }) => this.isLocked(target) || properties.hidden
            ? [] : rows.filter(row => !row.element.hidden).map(row => row.index));
    }

    showLocks(available: boolean, disabled: boolean): void {
        for (const { lock } of this.channels) {
            lock.hidden = !available;
            lock.disabled = disabled;
        }
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

    private canCollapse(group: Channel): boolean {
        return !group.element.hidden && group.toggle.getAttribute('aria-disabled') !== 'true';
    }

    private canLock(group: Channel): boolean {
        return !group.element.hidden && !group.lock.hidden && !group.lock.disabled;
    }

    private collapse(group: Channel, collapsed: boolean): void {
        if (collapsed === this.collapsed.has(group.target)) return;
        if (collapsed) this.collapsed.add(group.target);
        else this.collapsed.delete(group.target);
        this.update();
    }

    private lock(group: Channel, locked: boolean): void {
        if (locked === this.locked.has(group.target)) return;
        if (locked) this.locked.add(group.target);
        else this.locked.delete(group.target);
        this.showLock(group);
        this.update();
    }

    private showLock(group: Channel): void {
        const { element, lock, name, target } = group;
        const locked = this.isLocked(target);
        element.toggleAttribute('data-locked', locked);
        lock.setAttribute('aria-pressed', String(locked));
        labelButton(lock, `${locked ? 'Unlock' : 'Lock'} ${name} tracks`, locked ? 'lock' : 'unlock');
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
        toggle.className = 'timeline-channel-toggle';
        toggle.title = name;
        toggle.setAttribute('aria-controls', properties.id);
        toggle.append(icon('chevron-right'), create('span', name));
        const lock = iconButton('unlock', `Lock ${name} tracks`);
        lock.classList.add('timeline-channel-lock');
        const controls = create('div');
        controls.className = 'timeline-channel-controls';
        controls.append(toggle, lock);
        const summary = create('div');
        summary.className = 'timeline-summary';
        summary.setAttribute('aria-hidden', 'true');
        header.append(controls, summary);
        row.append(header, properties);
        const group = { target, name, element: row, toggle, lock, properties, rows: [], summary, timeMarks: new Map() };
        toggle.onclick = () => {
            if (this.canCollapse(group)) this.collapse(group, !this.collapsed.has(target));
        };
        lock.onclick = () => {
            if (this.canLock(group)) this.lock(group, !this.isLocked(target));
        };
        this.showLock(group);
        return group;
    }
}
