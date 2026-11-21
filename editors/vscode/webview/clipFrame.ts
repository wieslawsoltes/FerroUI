import type { HostedAnimationFrame } from '../src/hostedAnimationProtocol';
import type { FrameTarget } from '../src/protocol';

export type ClipFrame = FrameTarget | HostedAnimationFrame;

export function sameClipFrame(left: ClipFrame | undefined, right: ClipFrame): boolean {
    if (!left) return false;
    return 'session' in right
        ? 'session' in left && left.session === right.session && left.token === right.token
        : 'revision' in left && left.revision === right.revision && left.sequence === right.sequence;
}
