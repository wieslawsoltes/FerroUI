import * as vscode from 'vscode';
import type { Range } from './protocol';
import { exactRange } from './textDocument';
import { resolveWorkspaceFile, workspaceFolders } from './workspacePaths';
import { xamlLanguage, type XamlServer } from './xamlServer';

export interface LibraryDefinition {
    kind: string;
    name: string;
    range: Range;
    key?: string;
    targetType?: string;
}

export interface ProjectDefinition {
    folder: string;
    path: string;
    definition: LibraryDefinition;
}

interface LibraryPage {
    entries: ProjectDefinition[];
    total: number;
    offset: number;
}

export class ProjectLibrary {
    constructor(private readonly server: XamlServer) {}

    get onDidChange() { return this.server.onProjectLibraryChange; }

    async list(reference: string | undefined, query: string, offset: number, limit: number, signal: AbortSignal) {
        const folders = workspaceFolders(reference);
        if (folders.length > 1) throw new Error('Choose one workspace folder to search the Library.');
        const folder = folders[0]?.uri.toString();
        if (!folder) return { entries: [], total: 0, offset };
        const page = await this.server.projectLibraryRequest<LibraryPage>('projectLibrary', { folder, query, offset, limit }, signal);
        signal.throwIfAborted();
        return page;
    }

    async resolve(entry: ProjectDefinition, signal: AbortSignal) {
        signal.throwIfAborted();
        const uri = await resolveWorkspaceFile(entry.path, entry.folder);
        const document = await vscode.workspace.openTextDocument(uri);
        if (document.languageId !== xamlLanguage) throw new Error('The definition must resolve to a FerroUI XAML document.');
        const version = document.version;
        await this.server.projectLibraryRequest('resolveProjectLibrary', { ...entry, uri: uri.toString(), version }, signal);
        const ensure = () => {
            signal.throwIfAborted();
            if (document.isClosed || document.version !== version) throw new Error('The Library source changed before navigation.');
        };
        ensure();
        return { document, range: exactRange(document, entry.definition.range), ensure };
    }
}
