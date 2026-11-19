import { errorMessage } from './protocol';
import type { StudioContext } from './studioContext';
import { acceptedKeyframe } from './timelineKeyframes';
import type { KeyframeDraftPreview, TimelineKeyframeRequest, TimelineKeyframeResponse, TimelineProgram } from './timelineProtocol';

export async function authorTimelineKeyframe(context: StudioContext, message: TimelineKeyframeRequest,
    programs: () => Promise<TimelineProgram[]>): Promise<TimelineKeyframeResponse> {
    const result = { type: 'keyframeDraftResult', frame: message.frame, request: message.request } as const;
    const { panel } = context;
    const { draft } = message;
    try {
        if (!panel.supportsTimelineKeyframeDrafts) throw new Error('Open a mapped keyframe in the current Timeline.');
        if (!message.apply) return { ...result, preview: await context.read<KeyframeDraftPreview>('designer_keyframe_draft', draft) };

        const original = (await programs()).find(program => program.id === draft.program);
        if (!original?.timeline) throw new Error('Choose a typed animation program from the current document.');
        const originalVersion = panel.document.version;
        await context.edit('designer_program', { action: 'upsert_keyframe', ...draft });
        await panel.waitForSnapshot(AbortSignal.timeout(15_000));
        const token = panel.currentToken;
        const version = panel.document.version;
        const available = await programs();
        if (token === undefined || panel.currentToken !== token || version > originalVersion + 1 || panel.document.version !== version) {
            return { ...result, applied: null };
        }
        const accepted = available.find(program => original.source_key && original.source_key === program.source_key)
            ?? available.find(program => program.id === original.id && program.name === original.name);
        const tracks = accepted?.timeline?.tracks.filter(track => track.property === draft.property) ?? [];
        const track = tracks.find(track => track.target === draft.target) ?? (tracks.length === 1 ? tracks[0] : undefined);
        const keyframe = track && acceptedKeyframe(track, draft.time);
        return { ...result, applied: accepted && track && keyframe ? {
            frame: { session: panel.session, token }, version, program: accepted.id, target: track.target, keyframe,
        } : null };
    } catch (error) {
        return { ...result, error: errorMessage(error) };
    }
}
