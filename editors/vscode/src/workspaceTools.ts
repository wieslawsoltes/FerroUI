import { createHash } from 'node:crypto';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import { runEditorHistory } from './editorHistory';
import type { Position, Range, TextEdit } from './protocol';
import { asPosition, exactRange, wireRange } from './textDocument';
import type { WorkspaceTool } from './workspaceCatalog';
import { queryLanguage, type LanguageQuery } from './workspaceLanguage';
import { localFolders, newWorkspaceFile, resolveWorkspaceFile, workspaceFolders } from './workspacePaths';

const maxSource = 1024 * 1024;
const excludedDirectories = '**/{.git,node_modules,target}/**';

interface Location { path: string; folder?: string }
interface FileEdit extends Location { expected_revision: string; source?: string; edits?: TextEdit[] }
interface FileAction extends Location {
    action: 'open' | 'save' | 'format' | 'undo' | 'redo' | 'rename' | 'delete';
    expected_revision?: string;
    new_path?: string;
    position?: Position;
}

export class WorkspaceTools {
    async call(tool: WorkspaceTool, args: unknown, signal: AbortSignal): Promise<unknown> {
        switch (tool) {
            case 'ferroui_workspace_files':
                return this.list(args as { folder?: string; glob?: string; limit?: number }, signal);
            case 'ferroui_workspace_read': {
                const request = args as Location & { range?: Range; include_source?: boolean };
                const document = await this.open(request);
                signal.throwIfAborted();
                const source = request.include_source === false ? undefined : document.getText(request.range ? exactRange(document, request.range) : undefined);
                if (source !== undefined) assertSize(source);
                return { ...describeBuffer(document), source, diagnostics: bufferDiagnostics(document.uri) };
            }
            case 'ferroui_workspace_create': {
                const request = args as Location & { source: string; open?: boolean };
                assertSize(request.source);
                const uri = await newWorkspaceFile(request.path, request.folder);
                const edit = new vscode.WorkspaceEdit();
                edit.createFile(uri, { contents: Buffer.from(request.source) });
                await apply(edit, signal);
                const document = await vscode.workspace.openTextDocument(uri);
                if (request.open) await vscode.window.showTextDocument(document);
                return describeBuffer(document);
            }
            case 'ferroui_workspace_language': {
                const request = args as Location & LanguageQuery;
                const document = await this.open(request);
                const buffer = describeBuffer(document);
                signal.throwIfAborted();
                const result = await queryLanguage(document, request);
                signal.throwIfAborted();
                requireRevision(document, buffer.revision);
                return { buffer, result };
            }
            case 'ferroui_workspace_edit':
                return this.edit((args as { files: FileEdit[] }).files, signal);
            case 'ferroui_workspace_file':
                return this.file(args as FileAction, signal);
        }
    }

    private async list(request: { folder?: string; glob?: string; limit?: number }, signal: AbortSignal) {
        const roots = localFolders().map(folder => ({ name: folder.name, uri: folder.uri.toString() }));
        const folders = workspaceFolders(request.folder);
        const limit = request.limit ?? 100;
        const cancellation = new vscode.CancellationTokenSource();
        const cancel = () => cancellation.cancel();
        signal.addEventListener('abort', cancel);
        try {
            const found = (await Promise.all(folders.map(async folder => {
                const matches = await vscode.workspace.findFiles(new vscode.RelativePattern(folder, request.glob ?? '**/*'),
                    excludedDirectories, limit + 1, cancellation.token);
                return matches.map(uri => ({ folder: folder.uri.toString(), uri: uri.toString(),
                    path: vscode.workspace.asRelativePath(uri, false) }));
            }))).flat();
            signal.throwIfAborted();
            found.sort((a, b) => a.folder.localeCompare(b.folder) || a.path.localeCompare(b.path));
            return { roots, files: found.slice(0, limit), truncated: found.length > limit };
        } finally {
            signal.removeEventListener('abort', cancel);
            cancellation.dispose();
        }
    }

    private async open(request: Location): Promise<vscode.TextDocument> {
        return vscode.workspace.openTextDocument(await resolveWorkspaceFile(request.path, request.folder));
    }

    private async edit(files: FileEdit[], signal: AbortSignal) {
        if (!files.length) throw new Error('Provide at least one file to edit.');
        const documents = await Promise.all(files.map(request => this.open(request)));
        const physical = await Promise.all(documents.map(document => realpath(document.uri.fsPath)));
        if (new Set(physical).size !== physical.length) throw new Error('Each file must appear only once in a transaction.');
        const edit = new vscode.WorkspaceEdit();
        for (const [index, document] of documents.entries()) {
            const { source, edits, expected_revision } = files[index];
            requireRevision(document, expected_revision);
            if ((source === undefined) === (edits === undefined)) throw new Error('Provide replacement source or text edits for each file.');
            const changes = source === undefined ? edits!.map(item => new vscode.TextEdit(exactRange(document, item.range), item.newText))
                : [new vscode.TextEdit(new vscode.Range(new vscode.Position(0, 0), document.positionAt(document.getText().length)), source)];
            for (const change of changes) assertSize(change.newText);
            changes.sort((left, right) => left.range.start.compareTo(right.range.start));
            for (let index = 1; index < changes.length; index++) {
                if (changes[index - 1].range.end.isAfter(changes[index].range.start)) throw new Error('Text edits must not overlap.');
            }
            edit.set(document.uri, changes);
        }
        await apply(edit, signal);
        return { files: documents.map(describeBuffer) };
    }

    private async file(request: FileAction, signal: AbortSignal) {
        const document = await this.open(request);
        const { action } = request;
        if (action !== 'open') requireRevision(document, request.expected_revision);
        signal.throwIfAborted();
        if (action === 'open') {
            const selection = request.position ? document.validatePosition(asPosition(request.position)) : undefined;
            await vscode.window.showTextDocument(document, { selection: selection && new vscode.Range(selection, selection) });
        } else if (action === 'save') {
            if (!await document.save()) throw new Error('VS Code could not save the buffer.');
        } else if (action === 'undo' || action === 'redo') {
            await runEditorHistory(document, action, () => requireRevision(document, request.expected_revision), signal);
        } else if (action === 'format') {
            const config = vscode.workspace.getConfiguration('editor', document.uri);
            const options = { tabSize: config.get('tabSize', 4), insertSpaces: config.get('insertSpaces', true) };
            const edits = await vscode.commands.executeCommand<vscode.TextEdit[]>('vscode.executeFormatDocumentProvider', document.uri, options);
            requireRevision(document, request.expected_revision);
            if (edits?.length) {
                const edit = new vscode.WorkspaceEdit();
                edit.set(document.uri, edits);
                await apply(edit, signal);
            }
        } else {
            if (document.isDirty) throw new Error('Save the current buffer before renaming or deleting it.');
            const edit = new vscode.WorkspaceEdit();
            if (action === 'rename') {
                if (request.new_path === undefined) throw new Error('Specify new_path for a rename.');
                const folder = vscode.workspace.getWorkspaceFolder(document.uri)!;
                const destination = await newWorkspaceFile(request.new_path, folder.uri.toString());
                requireRevision(document, request.expected_revision);
                edit.renameFile(document.uri, destination);
                await apply(edit, signal);
                return describeBuffer(await vscode.workspace.openTextDocument(destination));
            }
            edit.deleteFile(document.uri, { recursive: false });
            await apply(edit, signal);
            return { uri: document.uri.toString(), deleted: true };
        }
        return describeBuffer(document);
    }
}

export function bufferDiagnostics(uri: vscode.Uri) {
    return vscode.languages.getDiagnostics(uri).map(item => ({ message: item.message, range: wireRange(item.range), severity: item.severity,
        source: item.source, code: item.code }));
}

function describeBuffer(document: vscode.TextDocument) {
    return { uri: document.uri.toString(), language: document.languageId, revision: bufferRevision(document),
        version: document.version, dirty: document.isDirty, lines: document.lineCount };
}

function bufferRevision(document: vscode.TextDocument): string {
    const digest = createHash('sha256').update(document.getText()).digest('base64url');
    return `${document.version}:${digest}`;
}

function requireRevision(document: vscode.TextDocument, expected?: string): void {
    if (document.isClosed || expected === undefined || bufferRevision(document) !== expected) {
        throw new Error('Read the current workspace buffer revision before changing it.');
    }
}

function assertSize(source: string): void {
    if (Buffer.byteLength(source) > maxSource) throw new Error('The requested source exceeds 1 MiB. Read or edit a smaller range.');
}

async function apply(edit: vscode.WorkspaceEdit, signal: AbortSignal): Promise<void> {
    signal.throwIfAborted();
    if (!await vscode.workspace.applyEdit(edit)) throw new Error('VS Code could not apply the workspace edit.');
}
