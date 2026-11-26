import { open as openFile } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { FileMoveDraft, RustRenameResolution, RustSymbolRename } from './fileMoveProtocol';
import { isCompiledXamlRust } from './fileMoveRust';
import type { Position, Range } from './protocol';
import { reviewedSourceUri, type ReviewedDocument } from './reviewedSources';
import { exactRange, wireRange } from './textDocument';
import { isWorkspaceFile } from './workspacePaths';
import type { XamlServer } from './xamlServer';

export interface RenamePosition { version: number; position: Position }
export interface RenameLocation { uri: string; range: Range }
export interface RenameTarget { name: string; range: Range; rustUri: string; rustDeclaration?: string | null }
export interface RegisteredRenamePlan {
    name: string;
    newName: string;
    review: string;
    documents: ReviewedDocument[];
    unmapped: RenameLocation[];
    rustRename?: RustSymbolRename | null;
}

export async function planRegisteredType(
    server: XamlServer, document: vscode.TextDocument, request: RenamePosition, target: RenameTarget,
    newName: string, signal?: AbortSignal,
): Promise<RegisteredRenamePlan> {
    // Open the exact Rust file before planning so its editor text and offsets
    // agree with the semantic rename provider, including a UTF-8 BOM or a draft.
    const anchors = new Set([target.rustUri, target.rustDeclaration]);
    for (const anchor of anchors) {
        if (!anchor) continue;
        const uri = await reviewedSourceUri(vscode.Uri.parse(anchor, true));
        if (!uri) throw new Error('Open the workspace containing the Rust type declaration.');
        await vscode.workspace.openTextDocument(uri);
    }
    const sourceDrafts = await registeredRustDrafts();
    const params = { textDocument: { uri: document.uri.toString() }, request, newName, sourceDrafts };
    const base = await server.registeredTypeRequest<RegisteredRenamePlan>('planRegisteredTypeRename', params);
    signal?.throwIfAborted();
    if (!base.rustRename) return base;
    const resolution = await semanticRename(base.rustRename, signal);
    return server.registeredTypeRequest('completeRegisteredTypeRename', {
        ...params, sourceDrafts: await registeredRustDrafts(), review: base.review, resolution,
    });
}

export async function registeredRustDrafts(): Promise<FileMoveDraft[]> {
    const rust = vscode.workspace.textDocuments.filter(document =>
        document.uri.scheme === 'file' && document.uri.path.toLowerCase().endsWith('.rs') && !isCompiledXamlRust(document));
    const inside = await Promise.all(rust.map(document => isWorkspaceFile(document.uri)));
    const snapshots = rust.filter((document, index) => inside[index] && !document.isClosed).map(document => ({
        uri: document.uri.toString(), version: document.version, source: document.getText(),
    }));
    if (Buffer.byteLength(JSON.stringify(snapshots)) > 12 * 1024 * 1024) throw new Error('Save and close larger Rust buffers before reviewing a registered name.');
    return snapshots;
}

async function semanticRename(rename: RustSymbolRename, signal?: AbortSignal): Promise<RustRenameResolution> {
    const uri = await reviewedSourceUri(vscode.Uri.parse(rename.uri, true));
    if (!uri) throw new Error('Open the workspace containing the Rust type declaration.');
    const source = await vscode.workspace.openTextDocument(uri);
    const position = new vscode.Position(rename.position.line, rename.position.character);
    const range = new vscode.Range(position, position.translate(0, rename.oldName.length));
    if (!source.validateRange(range).isEqual(range) || source.getText(range) !== rename.oldName) {
        throw new Error('The Rust declaration changed. Review the type name again.');
    }
    const version = source.version;
    const changes = await vscode.commands.executeCommand<vscode.WorkspaceEdit | undefined>(
        'vscode.executeDocumentRenameProvider', uri, position, rename.newName,
    );
    signal?.throwIfAborted();
    if (source.isClosed || source.version !== version) throw new Error('The Rust declaration changed during analysis.');
    const documents: RustRenameResolution['documents'] = [];
    let verified = false;
    for (const [file, edits] of changes?.entries() ?? []) {
        const authored = await reviewedSourceUri(file);
        if (!authored || !file.path.toLowerCase().endsWith('.rs')) {
            const generated = file.scheme === 'file' && await isGenerated(file);
            if (generated) continue;
            throw new Error('Open the consumer packages before renaming this Rust type.');
        }
        if (await isGenerated(authored)) continue;
        const live = vscode.workspace.textDocuments.find(item => item.uri.toString() === authored.toString());
        const wire = edits.map(edit => ({ range: wireRange(edit.range), newText: edit.newText }));
        if (live) for (const edit of wire) exactRange(live, edit.range);
        if (authored.toString() === uri.toString()) {
            verified ||= edits.some(edit => edit.range.isEqual(range) && edit.newText === rename.newName);
        }
        documents.push({ uri: authored.toString(), version: live?.version ?? -1, edits: wire });
    }
    if (!verified) throw new Error(`Wait for rust-analyzer to load the Cargo package that declares ${rename.oldName}.`);
    return { rename, documents };
}

async function isGenerated(uri: vscode.Uri): Promise<boolean> {
    const open = vscode.workspace.textDocuments.find(document => document.uri.toString() === uri.toString());
    if (open) return isCompiledXamlRust(open);
    const file = await openFile(uri.fsPath, 'r');
    try {
        const marker = Buffer.from('// @generated by the Rust emitter of ferroui-markup-xaml-loader ');
        const first = Buffer.alloc(marker.length);
        const { bytesRead } = await file.read(first, 0, first.length, 0);
        return bytesRead === first.length && first.equals(marker);
    } finally { await file.close(); }
}
