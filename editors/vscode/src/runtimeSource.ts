import * as vscode from 'vscode';
import type { DesignerWorkbench } from './designerWorkbench';
import type { HotReload } from './hotReload';
import { revision } from './reloadProcess';
import type { RuntimeClient } from './runtimeClient';
import type { RuntimeTarget } from './runtimeConnections';
import type { RuntimeNode, RuntimeProperty, RuntimeSource } from './runtimeProtocol';
import { readRuntimeTemplates } from './runtimeTemplates';
import { asRange } from './textDocument';
import { workspaceSourceUri } from './workspacePaths';

interface AuthoredSource {
    anchor: RuntimeSource;
    document: vscode.TextDocument;
    position: vscode.Position;
}

export class RuntimeSourceEditor {
    constructor(private readonly designer: DesignerWorkbench, private readonly reload: HotReload) {}

    async read(client: RuntimeClient, node: number, signal?: AbortSignal): Promise<AuthoredSource | undefined> {
        if (!Number.isSafeInteger(node) || !await client.supports('get_source')) return;
        const anchor = await client.call<RuntimeSource | null>('get_source', { node_id: node }, signal);
        return this.resolve(anchor);
    }

    async resolve(anchor: RuntimeSource | null): Promise<AuthoredSource | undefined> {
        if (!anchor || typeof anchor.uri !== 'string') return;
        const uri = await workspaceSourceUri(vscode.Uri.parse(anchor.uri, true));
        if (!uri) return;
        const document = await vscode.workspace.openTextDocument(uri);
        if ((anchor.language === 'rust' || anchor.revision !== undefined) && !matchesRevision(document, anchor.revision)) return;
        const position = sourcePosition(document, anchor);
        return position && { anchor, document, position };
    }

    async revealTemplate(client: RuntimeClient, node: number, id: string, signal: AbortSignal, isCurrent: () => boolean) {
        const templates = await readRuntimeTemplates(client, node, signal);
        const template = templates.find(template => template.id === id);
        if (!template || !isCurrent()) throw new Error('The template changed. Open the current template catalog.');
        if (!template.source) throw new Error('The app has no recorded source for this template.');
        const source = await this.resolve(template.source);
        signal.throwIfAborted();
        if (!isCurrent()) throw new Error('The application connection changed.');
        if (!source) throw new Error('The template source is unavailable in the current workspace or revision.');
        const { document, position } = source;
        await vscode.window.showTextDocument(document, {
            selection: new vscode.Range(position, position), viewColumn: vscode.ViewColumn.Beside,
        });
        return { uri: document.uri.toString(), version: document.version, position: { line: position.line, character: position.character } };
    }

    describe(source: AuthoredSource, target: RuntimeTarget) {
        const { anchor, document, position } = source;
        return { uri: document.uri.toString(), language: anchor.language ?? 'xaml', version: document.version,
            position: { line: position.line, character: position.character }, canApply: this.canApply(source, target) };
    }

    canApply({ anchor, document }: AuthoredSource, target: RuntimeTarget): boolean {
        if (document.isDirty || document.isClosed) return false;
        if (anchor.language === 'rust') return document.languageId === 'rust' && matchesRevision(document, anchor.revision);
        if (!document.uri.fsPath.toLowerCase().endsWith('.xaml')) return false;
        const accepted = target === 'managed' ? this.reload.xamlRevision(document.uri.fsPath) : undefined;
        if (anchor.revision === undefined) return matchesRevision(document, accepted);
        return (!accepted || accepted === anchor.revision) && matchesRevision(document, anchor.revision);
    }

    async apply(client: RuntimeClient, node: RuntimeNode, name: string, value: string, signal: AbortSignal,
        runtimeTarget: RuntimeTarget, isCurrent: () => boolean, expectedVersion?: number): Promise<void> {
        const source = await this.read(client, node.node_id, signal);
        if (!source || !this.canApply(source, runtimeTarget) || (expectedVersion !== undefined && source.document.version !== expectedVersion)) {
            throw new Error('Save the source and wait for the running app to accept that revision before applying a value.');
        }
        const { anchor, document, position } = source;
        const version = document.version;
        const guard = () => {
            if (!isCurrent() || document.version !== version || !this.canApply(source, runtimeTarget)) {
                throw new Error('The runtime selection or its source revision changed. Choose the current control to retry.');
            }
        };
        const properties = await client.call<RuntimeProperty[]>('get_properties', { node_id: node.node_id, properties: [name] }, signal);
        if (!properties.some(property => property.name === name && !property.binding && !property.read_only) || value.trimStart().startsWith('{')) {
            throw new Error('Choose an editable literal value for this authored control.');
        }
        guard();
        const design = await this.designer.openDocument(document.uri, position);
        const snapshot = await design.waitForSnapshot(signal);
        const type = node.type.split(/[.:]/).at(-1);
        const matching = snapshot.nodes.filter(candidate => candidate.label.split(':').at(-1) === type
            && (anchor.language === 'rust' ? asRange(candidate.nameRange).start.isEqual(position)
                : asRange(candidate.range).start.isEqual(position) || asRange(candidate.nameRange).contains(position)));
        const target = matching.length === 1 ? matching[0] : undefined;
        const frame = design.currentToken;
        if (!target || frame === undefined) throw new Error('The running element has no matching control in the current designer scope.');
        const authored = target.properties.find(property => property.name === name);
        if ((anchor.language === 'rust' && !authored?.editable) || authored?.value.trimStart().startsWith('{')) {
            throw new Error('Choose a mapped authored literal in the designer.');
        }
        await design.agentEdit(frame, { type: 'setProperty', node: target.id, name, value }, signal, guard);
    }
}

function matchesRevision(document: vscode.TextDocument, expected: string | undefined): boolean {
    return typeof expected === 'string' && /^[0-9a-f]{16}$/.test(expected)
        && revision(Buffer.from(document.getText())) === expected;
}

export function sourcePosition(document: vscode.TextDocument, source: Omit<RuntimeSource, 'uri'>): vscode.Position | undefined {
    if (!Number.isSafeInteger(source.line) || !Number.isSafeInteger(source.column) || source.line < 1 || source.column < 1
        || source.line > document.lineCount) return;
    const line = source.line - 1;
    let character = source.column - 1;
    if (source.columnEncoding === 'unicodeScalar') {
        const scalars = Array.from(document.lineAt(line).text);
        if (character > scalars.length) return;
        character = scalars.slice(0, character).join('').length;
    }
    const position = new vscode.Position(line, character);
    if (document.validatePosition(position).isEqual(position)) return position;
}
