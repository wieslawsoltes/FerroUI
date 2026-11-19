import type { HostedAnimationFrame, HostedKeyframeSelection } from '../src/hostedAnimationProtocol';
import type { FromHosted, ToHosted } from '../src/hostedProtocol';
import { TimelineKeyframeEditor } from './timelineKeyframeEditor';

export function editorFrame(frame: HostedAnimationFrame) {
    return { session: frame.revision, token: frame.sequence };
}

export class HostedKeyframeEditor {
    private readonly editor: TimelineKeyframeEditor;
    private frame?: HostedAnimationFrame;

    constructor(send: (message: FromHosted) => void, restoreFocus: () => void) {
        this.editor = new TimelineKeyframeEditor(request => {
            const frame = this.frame;
            if (frame?.revision === request.frame.session && frame.sequence === request.frame.token) {
                send({ type: 'keyframeDraft', frame, request: request.request, draft: request.draft, apply: request.apply });
            }
        }, undefined, undefined, restoreFocus);
    }

    open(key: HostedKeyframeSelection, focus = true) {
        this.frame = key.frame;
        this.editor.open(editorFrame(key.frame), key.version, key.program, key.target, key.property,
            key.keyframe, key.targetName, key.duration, focus);
    }

    drag(key: HostedKeyframeSelection, time: number, value: number | string) {
        this.open(key, false);
        this.editor.applyCurveDrag(editorFrame(key.frame), time, value);
    }

    follow(frame: HostedAnimationFrame | undefined, version: number, program: number | undefined) {
        this.frame = frame;
        this.editor.follow(frame && editorFrame(frame), version, program);
    }

    receive(reply: Extract<ToHosted, { type: 'keyframeReply' }>) {
        this.editor.receive({ type: 'keyframeDraftResult', frame: editorFrame(reply.frame),
            request: reply.request, preview: reply.preview, error: reply.error,
            applied: reply.applied ? null : undefined,
        });
    }

    close() { this.frame = undefined; this.editor.close(); }
}
