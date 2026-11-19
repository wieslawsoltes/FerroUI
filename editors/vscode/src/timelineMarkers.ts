import type { DesignerPanel } from './designerPanel';
import type { TimelineMarker, TimelineMarkerEdit, TimelineProgram } from './timelineProtocol';

/** Markers belong to the current design and authored clip, outside the source transaction. */
export class TimelineMarkers {
    private readonly designs = new WeakMap<DesignerPanel, Map<string, Map<string, number>>>();

    list(panel: DesignerPanel, program: TimelineProgram): TimelineMarker[] {
        const markers = this.designs.get(panel)?.get(this.key(panel, program));
        const duration = program.timeline?.duration;
        if (!markers || duration === undefined) return [];
        return [...markers].filter(([, time]) => time <= duration).map(([name, time]) => ({ name, time }))
            .sort((left, right) => left.time - right.time || left.name.localeCompare(right.name));
    }

    edit(panel: DesignerPanel, program: TimelineProgram, edit: TimelineMarkerEdit): void {
        const name = typeof edit.name === 'string' ? edit.name.trim() : '';
        if (!name || name.length > 48 || /[\u0000-\u001f\u007f]/.test(name)) throw new Error('Enter a marker name of up to 48 characters.');
        const duration = program.timeline?.duration;
        if (edit.action === 'set' && (duration === undefined || !Number.isFinite(edit.time) || edit.time < 0 || edit.time > duration)) {
            throw new Error('Choose a marker time within the clip.');
        }
        const key = this.key(panel, program);
        const store = this.designs.get(panel) ?? new Map<string, Map<string, number>>();
        const markers = store.get(key) ?? new Map<string, number>();
        if (edit.action === 'remove') {
            if (!markers.delete(name)) throw new Error('Choose a marker from this clip.');
        } else if (edit.action === 'set') {
            if (markers.size >= 64 && !markers.has(name)) throw new Error('Remove a marker before adding another.');
            markers.set(name, edit.time);
        } else throw new Error('Choose a marker action.');
        this.designs.set(panel, store);
        store.set(key, markers);
    }

    private key(panel: DesignerPanel, program: TimelineProgram): string {
        const block = panel.snapshot?.activeBlock?.start;
        return JSON.stringify([block?.line, block?.character, program.source_key ?? [program.id, program.owner, program.selector]]);
    }
}
