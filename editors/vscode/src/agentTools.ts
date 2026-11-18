import { realpath } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { agentCatalog, validate, type AgentTool } from './agentCatalog';
import { isInside } from './cargoTarget';
import type { DesignerPanel } from './designerPanel';
import type { DesignerWorkbench } from './designerWorkbench';
import type { HotReload } from './hotReload';
import type { DragOrigin, DropTarget } from './protocol';
import { applyTextEdits } from './textDocument';

interface Arguments {
    session: number;
    expected_revision: number;
    expected_version: number;
    path: string;
    line?: number;
    character?: number;
    include_source?: boolean;
    include_counterpart?: boolean;
    node?: number;
    offset?: number;
    limit?: number;
    view?: 'canvas' | 'outline' | 'properties' | 'toolbox';
    width?: number;
    height?: number;
    dark?: boolean;
    refresh?: boolean;
    reveal_source?: boolean;
    asset?: string;
    anchor?: number;
    position?: 'inside' | 'before' | 'after';
    x?: number;
    y?: number;
    label: string;
    edits: object[];
    source: string;
    action: string;
    tool: string;
    arguments: object;
}

export class AgentTools {
    constructor(private readonly workbench: DesignerWorkbench, private readonly reload: HotReload) {}

    definitions() { return Object.entries(agentCatalog).map(([name, definition]) => ({ name, ...definition })); }

    async call(name: string, input: unknown, signal: AbortSignal): Promise<unknown> {
        signal.throwIfAborted();
        if (!vscode.workspace.isTrusted) throw new Error('FerroUI agent tools require a trusted workspace.');
        const config = vscode.workspace.getConfiguration('ferroui.mcp');
        if (!config.get('enableDesigner', true)) throw new Error('The VS Code designer MCP server is disabled.');
        if (!Object.hasOwn(agentCatalog, name)) throw new Error('Unknown FerroUI tool');
        const tool = name as AgentTool;
        const definition = agentCatalog[tool];
        validate(definition.inputSchema, input);
        const args = input as Arguments;
        if (!definition.annotations.readOnlyHint && !config.get('allowWrites', false)) {
            throw new Error('Enable ferroui.mcp.allowWrites to let agents edit or control the designer.');
        }
        if (tool === 'ferroui_designs') {
            const panels = [];
            for (const panel of this.workbench.designs()) {
                if (await isWorkspaceFile(panel.document.uri)) panels.push(describe(panel));
            }
            const active = this.workbench.activeDesign?.session;
            return { active: panels.some(panel => panel.session === active) ? active : null, designs: panels };
        }
        if (tool === 'designer_open') {
            const uri = await resolveFile(args.path);
            signal.throwIfAborted();
            const position = args.line === undefined ? undefined : new vscode.Position(args.line, args.character ?? 0);
            const panel = await this.workbench.openDocument(uri, position);
            await panel.waitForSnapshot(signal).catch(error => { if (signal.aborted) throw error; });
            return describe(panel);
        }
        if (tool === 'ferroui_reload_status') return this.reload.state;
        if (tool === 'ferroui_reload_control') return this.reload.agentControl(args.action);

        const panel = this.workbench.designs().find(panel => panel.session === args.session);
        if (!panel || !await isWorkspaceFile(panel.document.uri)) throw new Error('Select an open design inside the current workspace.');
        signal.throwIfAborted();
        const document = panel.document;
        switch (tool) {
            case 'designer_document': {
                const source = args.include_source ? document.getText() : undefined;
                if (source && Buffer.byteLength(source) > 1024 * 1024) throw new Error('The authored buffer exceeds 1 MiB.');
                return { ...describe(panel), source,
                    counterpart: args.include_counterpart ? panel.snapshot?.counterpart : undefined,
                    diagnostics: vscode.languages.getDiagnostics(document.uri).map(item => ({ message: item.message, range: item.range, severity: item.severity })) };
            }
            case 'designer_outline': {
                const snapshot = await panel.waitForSnapshot(signal);
                const nodes = snapshot.nodes;
                if (args.node !== undefined) {
                    const selected = nodes.find(node => node.id === args.node);
                    if (!selected) throw new Error('Read the current outline for valid node IDs.');
                    return { revision: panel.currentToken, node: selected };
                }
                const offset = args.offset ?? 0;
                return { revision: panel.currentToken, total: nodes.length, offset,
                    nodes: nodes.slice(offset, offset + (args.limit ?? 100)).map(({ properties, addableProperties, ...node }) => node) };
            }
            case 'designer_toolbox':
                await panel.waitForSnapshot(signal);
                return { assets: panel.assets };
            case 'designer_preview_image': {
                const snapshot = await panel.waitForSnapshot(signal);
                if (!snapshot.image) throw new Error(snapshot.renderError ?? 'The native preview image is unavailable.');
                const { png, width, height } = snapshot.image;
                return { mimeType: 'image/png', data: png, width, height, revision: panel.currentToken, uri: document.uri.toString() };
            }
            case 'designer_view': {
                if (args.view) await this.workbench.revealDesign(panel, args.view);
                const { width, height, dark } = args;
                const viewport = { width: width ?? panel.dimensions.width, height: height ?? panel.dimensions.height, dark: dark ?? panel.dimensions.dark };
                if (args.refresh || width !== undefined || height !== undefined || dark !== undefined) panel.configure(viewport);
                return describe(panel);
            }
            case 'designer_select': {
                const target = { session: panel.session, token: args.expected_revision, node: args.node! };
                if (!panel.canUse(target) || !panel.nodes.some(node => node.id === target.node)) throw new Error('Read the current outline before selecting.');
                await panel.select(target, args.reveal_source);
                return describe(panel);
            }
            case 'designer_place': {
                if ((args.node === undefined) === (args.asset === undefined)) throw new Error('Specify one asset ID or one source node.');
                const origin: DragOrigin = args.node !== undefined ? { type: 'node', node: args.node } : { type: 'asset', control: args.asset! };
                const hasOutline = args.anchor !== undefined && args.position !== undefined;
                const hasPoint = args.x !== undefined && args.y !== undefined;
                if (hasOutline === hasPoint) throw new Error('Specify one outline anchor and position or one canvas point.');
                const target: DropTarget = hasOutline ? { type: 'outline', node: args.anchor!, position: args.position! }
                    : { type: 'canvas', x: args.x!, y: args.y! };
                await panel.agentEdit(args.expected_revision, { type: 'place', frame: args.expected_revision, placement: { origin, target } }, signal);
                break;
            }
            case 'designer_apply':
                await panel.agentEdit(args.expected_revision, { type: 'apply', label: args.label, edits: args.edits }, signal);
                break;
            case 'designer_apply_counterpart':
                await panel.agentEdit(args.expected_revision, { type: 'applyCounterpart', source: args.source }, signal);
                break;
            case 'designer_source': {
                if (Buffer.byteLength(args.source) > 1024 * 1024) throw new Error('The authored source exceeds 1 MiB.');
                const end = document.positionAt(document.getText().length);
                await applyTextEdits(document, args.expected_version, [{ range: { start: { line: 0, character: 0 }, end }, newText: args.source }]);
                break;
            }
            case 'designer_history':
                assertVersion(document, args.expected_version);
                await vscode.window.showTextDocument(document, { preserveFocus: false });
                signal.throwIfAborted();
                assertVersion(document, args.expected_version);
                if (vscode.window.activeTextEditor?.document !== document) throw new Error('The authored editor lost focus.');
                await vscode.commands.executeCommand(args.action);
                break;
            case 'designer_save':
                assertVersion(document, args.expected_version);
                if (!await document.save()) throw new Error('VS Code could not save the buffer.');
                break;
            case 'designer_studio_tools': return { tools: await panel.studioTools() };
            case 'designer_studio_query': return panel.studioQuery(args.tool, args.arguments, signal);
        }
        return describe(panel);
    }
}

function describe(panel: DesignerPanel) {
    return { session: panel.session, uri: panel.document.uri.toString(), language: panel.language, version: panel.document.version,
        revision: panel.currentToken ?? null, selected: panel.selectedNode?.id ?? null, dirty: panel.document.isDirty,
        viewport: panel.dimensions, status: panel.status ?? panel.snapshot?.renderError ?? 'Ready', notices: panel.snapshot?.notices ?? [] };
}

function assertVersion(document: vscode.TextDocument, version: number): void {
    if (document.version !== version) throw new Error('The source buffer changed. Read its latest version.');
}

async function isWorkspaceFile(uri: vscode.Uri): Promise<boolean> {
    if (uri.scheme !== 'file') return false;
    const folder = vscode.workspace.getWorkspaceFolder(uri);
    if (!folder || folder.uri.scheme !== 'file') return false;
    try { return isInside(await realpath(folder.uri.fsPath), await realpath(uri.fsPath)); } catch { return false; }
}

async function resolveFile(input: string): Promise<vscode.Uri> {
    const paths = path.isAbsolute(input) ? [input] : (vscode.workspace.workspaceFolders ?? []).filter(folder => folder.uri.scheme === 'file')
        .map(folder => path.resolve(folder.uri.fsPath, input));
    const matches: vscode.Uri[] = [];
    for (const file of paths) {
        const uri = vscode.Uri.file(file);
        if (await isWorkspaceFile(uri)) matches.push(uri);
    }
    if (matches.length !== 1) throw new Error('Provide an unambiguous file path inside the current workspace.');
    return matches[0];
}
