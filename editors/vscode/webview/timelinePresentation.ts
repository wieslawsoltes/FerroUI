import type { TimelinePresentationReply, TimelinePresentationRequest } from '../src/timelinePresentationProtocol';
import type { TimelineTrackCommand, TimelineTrackView } from '../src/timelineTrackProtocol';
import type { TimelineSnapChange, TimelineSnapState, TimelineWorkAreaScope } from '../src/timelineTimingProtocol';
import { changeTimelineViewport, readTimelineViewport, type TimelineRulerView } from './timelineViewportRequest';
import type { TimelineWorkArea } from './timelineWorkArea';

export interface WorkAreaView extends TimelineWorkAreaScope {
    area: TimelineWorkArea;
    clock: 'authored' | 'elapsed';
}

export interface TimelinePresentationView {
    ruler(): TimelineRulerView | undefined;
    tracks(): TimelineTrackView | undefined;
    changeTracks(command: TimelineTrackCommand): void;
    workArea(): WorkAreaView | undefined;
    snap?(): TimelineSnapState | undefined;
    changeSnap?(command: TimelineSnapChange): void;
}

export function answerTimelinePresentation({ request, change }: TimelinePresentationRequest<unknown>,
    view?: TimelinePresentationView): TimelinePresentationReply {
    const reply = { type: 'timelinePresentationReply', request } as const;
    try {
        if (change && !view) throw new Error('Read the current Timeline revision and program again.');
        if (change?.kind === 'viewport') changeTimelineViewport(change.viewport, view?.ruler());
        if (change?.kind === 'tracks') view?.changeTracks(change.track);
        if (change?.kind === 'work_area') {
            const current = view?.workArea();
            const command = change.work_area;
            if (!current || current.program !== command.program || current.scope !== command.scope) {
                throw new Error('Read the work area for the visible Timeline clock again.');
            }
            current.area.change(command);
        }
        if (change?.kind === 'snap') {
            if (!view?.changeSnap) throw new Error('Open the keyframe snapping control.');
            view.changeSnap(change.snap);
        }
        const current = view?.workArea();
        const work_area = current?.area.state;
        return { ...reply, presentation: { viewport: readTimelineViewport(view?.ruler()), track_view: view?.tracks(),
            work_area: current && work_area && { ...work_area, scope: current.scope, program: current.program, clock: current.clock },
            frame_snap: view?.snap?.() } };
    } catch (error) {
        return { ...reply, error: error instanceof Error ? error.message : String(error) };
    }
}
