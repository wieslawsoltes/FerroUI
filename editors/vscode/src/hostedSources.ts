import * as vscode from 'vscode';
import { pathToFileURL } from 'node:url';
import type { HostedAnimationFrame, HostedSourcePlan } from './hostedAnimationProtocol';
import { applyHostedSourcePatches } from './hostedSourceEdit';
import { previewSourceUri } from './includedSource';
import type { NativePreview } from './nativePreview';
import type { Range } from './protocol';
import { revision } from './reloadProcess';
import type { HostedSourceRequest } from './hostedStylesProtocol';
import { asRange, languageOf } from './textDocument';

type FrameAccess = <T>(frame: HostedAnimationFrame, signal: AbortSignal,
    task: (client: NativePreview, ensure: () => void) => Promise<T>) => Promise<T>;

export interface HostedSourceLocation { uri: string; range: Range }
export interface HostedAuthoredPlan extends HostedSourcePlan { uri: string }
export interface HostedIncludeRevision { uri: string; sourceRevision: string }

export class HostedSources {
    constructor(private readonly editor: vscode.TextEditor, private readonly withFrame: FrameAccess) {}

    read<T>(frame: HostedAnimationFrame, signal: AbortSignal, query: (client: NativePreview) => Promise<T>) {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const result = await query(client);
            ensure();
            return result;
        });
    }

    edit(frame: HostedAnimationFrame, address: string | undefined, apply: boolean, signal: AbortSignal,
        plan: (client: NativePreview, source: string) => Promise<HostedAuthoredPlan>) {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const primary = this.editor.document;
            const original = address === undefined
                ? { uri: pathToFileURL(primary.uri.fsPath).toString(), document: primary }
                : await this.source(client, frame, address);
            const originalVersion = original.document.version;
            const edit = await plan(client, original.document.getText());
            let target = original;
            let version = originalVersion;
            if (edit.uri !== original.uri) {
                if (!client.capabilities.hostedRustModules || languageOf(this.editor.document) !== 'rust') {
                    throw new Error('The source changed while preparing the edit.');
                }
                target = await this.source(client, frame, edit.uri);
                version = target.document.version;
                await this.request(client, frame, { action: 'verifySource', address: edit.uri, source: target.document.getText() });
            }
            ensure();
            if (edit.uri !== target.uri || target.document.version !== version || original.document.version !== originalVersion) {
                throw new Error('The source changed while preparing the edit.');
            }
            if (apply && edit.patches.length) await applyHostedSourcePatches(target.document, version, edit.patches);
            return { ...edit, applied: apply && edit.patches.length > 0, version: target.document.version };
        });
    }

    locate(frame: HostedAnimationFrame, address: string, range: Range, signal: AbortSignal) {
        return this.withFrame(frame, signal, async (client, ensure) => {
            const { uri, document } = await this.source(client, frame, address);
            const version = document.version;
            const location = await this.request<HostedSourceLocation>(client, frame, {
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

    async verifyIncludes(includes: readonly HostedIncludeRevision[]): Promise<void> {
        await Promise.all(includes.map(async include => {
            const uri = await previewSourceUri(this.editor.document.uri, include.uri);
            const open = () => vscode.workspace.textDocuments.find(document => document.uri.toString() === uri.toString());
            const disk = open()?.isDirty ? undefined : await vscode.workspace.fs.readFile(uri);
            const current = open();
            const bytes = current?.isDirty ? Buffer.from(current.getText()) : disk;
            if (!bytes) throw new Error('An included XAML buffer changed. Wait for App Preview to update.');
            const offset = bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf ? 3 : 0;
            if (revision(Buffer.from(bytes.subarray(offset))) !== include.sourceRevision) {
                throw new Error('An included XAML buffer changed. Wait for App Preview to update.');
            }
        }));
    }

    private async source(client: NativePreview, frame: HostedAnimationFrame, address: string) {
        const { uri } = await this.request<{ uri: string }>(client, frame, { action: 'source', address });
        const primary = this.editor.document.uri;
        const kind = languageOf(this.editor.document) === 'rust' ? 'rust' : 'xaml';
        const source = uri === pathToFileURL(primary.fsPath).toString() ? primary : await previewSourceUri(primary, uri, kind);
        return { uri, document: await vscode.workspace.openTextDocument(source) };
    }

    private request<T>(client: NativePreview, frame: HostedAnimationFrame, request: HostedSourceRequest): Promise<T> {
        if (client.capabilities.hostedRustStyles) return client.hostedStyles<T>(frame, request);
        if (request.action === 'verifySource') throw new Error('Update the FerroUI preview helper to edit a Rust module.');
        return client.hostedPalette<T>(frame, request);
    }
}
