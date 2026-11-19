import type { HostedAnimationCatalog, HostedAnimationProgram, HostedAnimationState } from './hostedAnimationProtocol';
import { TimelineMarkers } from './timelineMarkers';
import type { TimelineMarkerAction } from './timelineProtocol';

export function hostedClockEnd(programs: readonly HostedAnimationProgram[]): number {
    return programs.reduce((end, program) => Math.max(end, program.totalDuration ?? Infinity), 0);
}

/** A native source identity can outlive its frame. Ambiguous clips use the current revision. */
export class HostedMarkers {
    private readonly authored = new TimelineMarkers();
    private frame = new TimelineMarkers();

    replaceFrame(): void { this.frame = new TimelineMarkers(); }

    state(catalog: HostedAnimationCatalog, programs: readonly HostedAnimationProgram[],
        shared: readonly HostedAnimationProgram[]): HostedAnimationState['markers'] {
        const scope = catalog.markerScope;
        if (!scope) return;
        return { programs: Object.fromEntries(programs.map(program => {
            const { store, key } = this.clock(scope, program);
            return [program.id, store.list(key, hostedClockEnd([program]))];
        })), shared: shared.length ? this.authored.list(this.clock(scope).key, hostedClockEnd(shared)) : [] };
    }

    edit(scope: string | undefined, programs: readonly HostedAnimationProgram[], program: HostedAnimationProgram | undefined,
        action: TimelineMarkerAction): number | undefined {
        if (!scope) throw new Error('Update the FerroUI preview helper to use App Preview markers.');
        if (!programs.length) throw new Error('Choose an animation with live preview targets.');
        const { store, key } = this.clock(scope, program);
        const end = hostedClockEnd(programs);
        if (action.action === 'seek') return store.time(key, end, action.name);
        store.edit(key, end, action);
    }

    private clock(scope: string, program?: HostedAnimationProgram) {
        return { store: program && !program.markerKey ? this.frame : this.authored,
            key: JSON.stringify([scope, program?.markerKey ?? program?.id]) };
    }
}
