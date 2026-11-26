import type { HostedAnimationFrame } from './hostedAnimationProtocol';
import type { HostedAuthoredPlan, HostedSources } from './hostedSources';
import type { HostedStyleEdit, HostedStyleRead, HostedStyleResources, HostedStyleResult, HostedStyleTarget } from './hostedStylesProtocol';

export class HostedStyles {
    constructor(readonly sources: HostedSources) {}

    read(frame: HostedAnimationFrame, request: HostedStyleRead, signal: AbortSignal) {
        return this.sources.read(frame, signal, client => client.hostedStyles<HostedStyleResult>(frame, request));
    }

    resources(frame: HostedAnimationFrame, target: HostedStyleTarget, property: string, signal: AbortSignal) {
        return this.sources.read(frame, signal, client => client.hostedStyles<HostedStyleResources>(frame, { action: 'resources', target, property }));
    }

    edit(frame: HostedAnimationFrame, target: HostedStyleTarget, edit: HostedStyleEdit, apply: boolean, signal: AbortSignal) {
        return this.sources.edit(frame, target.address, apply, signal, (client, source) =>
            client.hostedStyles<HostedAuthoredPlan>(frame, { action: 'plan', target, edit, source }));
    }
}
