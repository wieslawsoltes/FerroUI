import type { TimelineClock, TimelineProgram } from './timelineProtocol';

export function previewClock(program?: TimelineProgram): TimelineClock | undefined {
    return program?.preview?.native ? program.preview : program?.timeline ?? undefined;
}

export function canPreviewProgram(program?: TimelineProgram): boolean {
    return program?.preview?.available ?? Boolean(program?.timeline);
}
