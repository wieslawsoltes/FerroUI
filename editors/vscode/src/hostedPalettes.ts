import type { HostedAnimationFrame } from './hostedAnimationProtocol';
import type { HostedPaletteCatalog, HostedPalettePlan, HostedPaletteReport, HostedPaletteTarget } from './hostedPaletteProtocol';
import type { HostedSources } from './hostedSources';
import type { Range } from './protocol';
import type { PaletteEdit } from './stylesProtocol';

export class HostedPalettes {
    constructor(private readonly sources: HostedSources) {}

    catalog(frame: HostedAnimationFrame, signal: AbortSignal) {
        return this.sources.read(frame, signal, client => client.hostedPalette<HostedPaletteCatalog>(frame, { action: 'catalog' }));
    }

    report(frame: HostedAnimationFrame, target: HostedPaletteTarget, signal: AbortSignal) {
        return this.sources.read(frame, signal, client => client.hostedPalette<HostedPaletteReport>(frame, { action: 'read', target }));
    }

    edit(frame: HostedAnimationFrame, target: HostedPaletteTarget, edit: PaletteEdit, apply: boolean, signal: AbortSignal) {
        return this.sources.edit(frame, target.address, apply, signal, (client, source) =>
            client.hostedPalette<HostedPalettePlan>(frame, { action: 'plan', target, edit, source }));
    }

    locate(frame: HostedAnimationFrame, address: string, range: Range, signal: AbortSignal) {
        return this.sources.locate(frame, address, range, signal);
    }

    references(frame: HostedAnimationFrame, target: HostedPaletteTarget, node: number, signal: AbortSignal) {
        return this.sources.read(frame, signal, client =>
            client.hostedPalette<{ uri: string; references: { label: string; range: Range; property: string; dynamic: boolean }[] }>(
                frame, { action: 'references', target, node }));
    }
}
