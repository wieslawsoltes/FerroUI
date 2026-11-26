import type { TimelineClock, TimelineProgram } from './timelineProtocol';

export function previewClock(program?: TimelineProgram): TimelineClock | undefined {
    return program?.preview?.native ? program.preview : program?.timeline ?? undefined;
}

export function canPreviewProgram(program?: TimelineProgram): boolean {
    return program?.preview?.available ?? Boolean(program?.timeline);
}

export function canPreviewRange(program?: TimelineProgram): boolean {
    return Boolean(program?.timeline && program.timeline.duration > 0 && !program.preview?.native && canPreviewProgram(program));
}

export function combinedRangeLimit(programs: readonly TimelineProgram[]): number {
    return programs.reduce((limit, program) => program.clip && canPreviewProgram(program)
        ? Math.max(limit, program.clip.end ?? Infinity) : limit, 0);
}
