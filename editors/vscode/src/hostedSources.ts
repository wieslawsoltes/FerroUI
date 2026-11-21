import * as vscode from 'vscode';
import type { HostedAnimationFrame, HostedSourcePlan } from './hostedAnimationProtocol';
import { applyHostedSourcePatches } from './hostedSourceEdit';
import { previewSourceUri } from './includedSource';
import type { NativePreview } from './nativePreview';
import type { Range } from './protocol';
import { asRange } from './textDocument';

type FrameAccess = <T>(frame: HostedAnimationFrame, signal: AbortSignal,
    task: (client: NativePreview, ensure: () => void) => Promise<T>) => Promise<T>;

export interface HostedSourceLocation { uri: string; range: Range }
export interface HostedAuthoredPlan extends HostedSourcePlan { uri: string }

export class HostedSources {
    constructor(private readonly editor: vscode.TextEditor, private readonly withFrame: FrameAccess) {}

    read<T>(frame: HostedAnimationFrame, signal: AbortSignal, query: (client: NativePreview) => Promise<T>) {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const result = await query(client);
            ensure();
            return result;
        });
    }

    edit(frame: HostedAnimationFrame, address: string, apply: boolean, signal: AbortSignal,
        plan: (client: NativePreview, source: string) => Promise<HostedAuthoredPlan>) {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const { uri, document } = await this.source(client, frame, address);
            const version = document.version;
            const edit = await plan(client, document.getText());
            ensure();
            if (edit.uri !== uri || document.version !== version) throw new Error('The XAML source changed while preparing the edit.');
            if (apply && edit.patches.length) await applyHostedSourcePatches(document, version, edit.patches);
            return { ...edit, applied: apply && edit.patches.length > 0, version: document.version };
        });
    }

    locate(frame: HostedAnimationFrame, address: string, range: Range, signal: AbortSignal) {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const { uri, document } = await this.source(client, frame, address);
            const version = document.version;
            const location = await client.hostedPalette<HostedSourceLocation>(frame, {
                action: 'locate', address, range, source: document.getText(),
            });
            ensure();
            if (location.uri !== uri || document.version !== version) throw new Error('The declaration changed while opening its source.');
            await vscode.window.showTextDocument(document, {
                viewColumn: this.editor.viewColumn ?? vscode.ViewColumn.One,
                selection: asRange(location.range), preview: false,
            });
            return location;
        });
    }

    private async source(client: NativePreview, frame: HostedAnimationFrame, address: string) {
        const { uri } = await client.hostedPalette<{ uri: string }>(frame, { action: 'source', address });
        const source = await previewSourceUri(this.editor.document.uri, uri);
        return { uri, document: await vscode.workspace.openTextDocument(source) };
    }
}
