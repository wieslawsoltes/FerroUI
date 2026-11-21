import * as vscode from 'vscode';
import type { HostedAnimationFrame } from './hostedAnimationProtocol';
import type { HostedPaletteCatalog, HostedPaletteLocation, HostedPalettePlan, HostedPaletteReport, HostedPaletteTarget } from './hostedPaletteProtocol';
import { applyHostedSourcePatches } from './hostedSourceEdit';
import { previewSourceUri } from './includedSource';
import type { NativePreview } from './nativePreview';
import type { Range } from './protocol';
import type { PaletteEdit } from './stylesProtocol';
import { asRange } from './textDocument';

type FrameAccess = <T>(frame: HostedAnimationFrame, signal: AbortSignal,
    task: (client: NativePreview, ensure: () => void) => Promise<T>) => Promise<T>;

export class HostedPalettes {
    constructor(private readonly editor: vscode.TextEditor, private readonly withFrame: FrameAccess) {}

    catalog(frame: HostedAnimationFrame, signal: AbortSignal): Promise<HostedPaletteCatalog> {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const catalog = await client.hostedPalette<HostedPaletteCatalog>(frame, { action: 'catalog' });
            ensure();
            return catalog;
        });
    }

    report(frame: HostedAnimationFrame, target: HostedPaletteTarget, signal: AbortSignal): Promise<HostedPaletteReport> {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const report = await client.hostedPalette<HostedPaletteReport>(frame, { action: 'read', target });
            ensure();
            return report;
        });
    }

    edit(frame: HostedAnimationFrame, target: HostedPaletteTarget, edit: PaletteEdit, apply: boolean, signal: AbortSignal) {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const { uri, document } = await this.source(client, frame, target.address);
            const version = document.version;
            const plan = await client.hostedPalette<HostedPalettePlan>(frame, { action: 'plan', target, edit, source: document.getText() });
            ensure();
            if (plan.uri !== uri || document.version !== version) throw new Error('The palette source changed while preparing the edit.');
            if (apply && plan.patches.length) await applyHostedSourcePatches(document, version, plan.patches);
            return { ...plan, applied: apply && plan.patches.length > 0, version: document.version };
        });
    }

    locate(frame: HostedAnimationFrame, address: string, range: Range, signal: AbortSignal) {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const { uri, document } = await this.source(client, frame, address);
            const version = document.version;
            const location = await client.hostedPalette<HostedPaletteLocation>(frame, {
                action: 'locate', address, range, source: document.getText(),
            });
            ensure();
            if (location.uri !== uri || document.version !== version) throw new Error('The palette declaration changed while opening its source.');
            await vscode.window.showTextDocument(document, {
                viewColumn: this.editor.viewColumn ?? vscode.ViewColumn.One,
                selection: asRange(location.range),
                preview: false,
            });
            return location;
        });
    }

    references(frame: HostedAnimationFrame, target: HostedPaletteTarget, node: number, signal: AbortSignal) {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const result = await client.hostedPalette<{ uri: string; references: { label: string; range: Range; property: string; dynamic: boolean }[] }>(
                frame, { action: 'references', target, node });
            ensure();
            return result;
        });
    }

    private async source(client: NativePreview, frame: HostedAnimationFrame, address: string) {
        const { uri } = await client.hostedPalette<{ uri: string }>(frame, { action: 'source', address });
        const source = await previewSourceUri(this.editor.document.uri, uri);
        return { uri, document: await vscode.workspace.openTextDocument(source) };
    }
}
