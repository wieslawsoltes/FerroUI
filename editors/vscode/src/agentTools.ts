import * as vscode from 'vscode';
import { agentCatalog, type AgentTool } from './agentCatalog';
import { validate } from './agentSchema';
import type { CargoSelection } from './cargoTarget';
import type { DesignerPanel } from './designerPanel';
import type { DesignerWorkbench } from './designerWorkbench';
import type { HotReload } from './hotReload';
import type { CanvasAction, DesignerView, DragOrigin, DropTarget, OutlineMode, Placement } from './protocol';
import { applyTextEdits } from './textDocument';
import { isWorkspaceTool } from './workspaceCatalog';
import { isWorkspaceFile, resolveWorkspaceFile } from './workspacePaths';
import { bufferDiagnostics, WorkspaceTools } from './workspaceTools';

interface Arguments extends CanvasAction, CargoSelection {
    session: number;
    expected_revision: number;
    expected_version: number;
    path: string;
    line?: number;
    character?: number;
    index: number;
    lines?: number;
    include_source?: boolean;
    include_counterpart?: boolean;
    node?: number;
    offset?: number;
    limit?: number;
    view?: DesignerView;
    outline_mode?: OutlineMode;
    inspector_id?: number;
    width?: number;
    height?: number;
    dark?: boolean;
    refresh?: boolean;
    reveal_source?: boolean;
    asset?: string;
    project_asset?: string;
    property: string;
    event: string;
    handler?: string;
    value?: string;
    resource: number;
    dynamic?: boolean;
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
    private readonly workspace = new WorkspaceTools();

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
        if (!definition.annotations.readOnlyHint && !config.get('allowWrites', false)) {
            throw new Error('Enable ferroui.mcp.allowWrites to let agents edit files or control the designer and running application.');
        }
        if (isWorkspaceTool(tool)) return this.workspace.call(tool, input, signal);
        const args = input as Arguments;
        if (tool === 'ferroui_designs') {
            const panels = [];
            for (const panel of this.workbench.designs()) {
                if (await isWorkspaceFile(panel.document.uri)) panels.push(describe(panel));
            }
            const active = this.workbench.activeDesign?.session;
            return { active: panels.some(panel => panel.session === active) ? active : null, designs: panels };
        }
        if (tool === 'designer_open') {
            const uri = await resolveWorkspaceFile(args.path, args.folder);
            signal.throwIfAborted();
            const position = args.line === undefined ? undefined : new vscode.Position(args.line, args.character ?? 0);
            const panel = await this.workbench.openDocument(uri, position);
            await panel.waitForSnapshot(signal).catch(error => { if (signal.aborted) throw error; });
            return describe(panel);
        }
        if (tool === 'ferroui_reload_status') return this.reload.state;
        if (tool === 'ferroui_reload_targets') return this.reload.targets(args);
        if (tool === 'ferroui_reload_output') return this.reload.recentOutput(args.lines ?? 60);
        if (tool === 'ferroui_reload_control') return this.reload.agentControl(args.action, args, signal);

        const panel = this.workbench.designs().find(panel => panel.session === args.session);
        if (!panel || !await isWorkspaceFile(panel.document.uri)) throw new Error('Select an open design inside the current workspace.');
        signal.throwIfAborted();
        const document = panel.document;
        switch (tool) {
            case 'designer_document': {
                if (args.include_counterpart) {
                    await panel.waitForSnapshot(signal).catch(error => { if (signal.aborted) throw error; });
                }
                const source = args.include_source ? document.getText() : undefined;
                if (source && Buffer.byteLength(source) > 1024 * 1024) throw new Error('The authored buffer exceeds 1 MiB.');
                return { ...describe(panel), source,
                    counterpart: args.include_counterpart ? panel.snapshot?.counterpart : undefined,
                    diagnostics: bufferDiagnostics(document.uri) };
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
            case 'designer_inspect': {
                await panel.waitForSnapshot(signal);
                const frame = { session: panel.session, token: panel.currentToken! };
                if (args.node !== undefined && args.inspector_id !== undefined) throw new Error('Choose one source or runtime object.');
                const node = args.node ?? panel.selectedNode?.id;
                const runtime = args.inspector_id ?? (args.node === undefined ? panel.runtimeSelection?.inspector_id : undefined);
                if (runtime === undefined && node === undefined) throw new Error('Select a control or provide its node ID.');
                const result = await panel.inspect(frame, runtime === undefined ? { type: 'source', node: node! } : { type: 'runtime', id: runtime });
                return { revision: frame.token, ...result };
            }
            case 'designer_resources': {
                await panel.waitForSnapshot(signal);
                const frame = { session: panel.session, token: panel.currentToken! };
                return { revision: frame.token, resources: await panel.resources(frame, args.node ?? panel.selectedNode?.id) };
            }
            case 'designer_assets': {
                const assets = await this.workbench.projectAssets(panel);
                const offset = args.offset ?? 0;
                return { assets: assets.slice(offset, offset + (args.limit ?? 100)), offset, total: assets.length };
            }
            case 'designer_preview_image': {
                const snapshot = await panel.waitForSnapshot(signal);
                if (!snapshot.image) throw new Error(snapshot.renderError ?? 'The native preview image is unavailable.');
                const { png, width, height } = snapshot.image;
                return { mimeType: 'image/png', data: png, width, height, revision: panel.currentToken, uri: document.uri.toString() };
            }
            case 'designer_wait':
                await panel.waitForSnapshot(signal);
                return describe(panel);
            case 'designer_block':
                await panel.selectBlock({ session: panel.session, token: args.expected_revision }, args.index, args.reveal_source);
                await panel.waitForSnapshot(signal);
                return describe(panel);
            case 'designer_view': {
                if (args.outline_mode) this.workbench.setOutlineMode(panel, args.outline_mode);
                if (args.view) await this.workbench.revealDesign(panel, args.view);
                const { width, height, dark } = args;
                const viewport = { width: width ?? panel.dimensions.width, height: height ?? panel.dimensions.height, dark: dark ?? panel.dimensions.dark };
                if (args.refresh || width !== undefined || height !== undefined || dark !== undefined) panel.configure(viewport);
                return describe(panel);
            }
            case 'designer_canvas':
                await this.workbench.revealDesign(panel, 'canvas');
                return panel.controlCanvas(args, signal);
            case 'designer_select': {
                const frame = { session: panel.session, token: args.expected_revision };
                if (!panel.canUse(frame)) throw new Error('Read the current outline before selecting.');
                if ((args.node === undefined) === (args.inspector_id === undefined)) throw new Error('Choose one source or runtime object.');
                if (args.inspector_id !== undefined) {
                    const row = (await panel.runtimeTree(frame, 'combined')).find(row => row.inspector_id === args.inspector_id);
                    signal.throwIfAborted();
                    if (!row || !panel.canUse(frame)) throw new Error('Read the current runtime tree before selecting.');
                    await panel.selectRuntime(frame, row, args.reveal_source ?? false);
                } else {
                    if (!panel.nodes.some(node => node.id === args.node)) throw new Error('Read the current source outline before selecting.');
                    await panel.select({ ...frame, node: args.node! }, args.reveal_source);
                }
                return describe(panel);
            }
            case 'designer_placement_hint': case 'designer_place': {
                const placement = await this.placement(panel, args);
                signal.throwIfAborted();
                if (tool === 'designer_placement_hint') {
                    return panel.previewPlacement({ session: panel.session, token: args.expected_revision }, placement);
                }
                await panel.agentEdit(args.expected_revision, { type: 'place', frame: args.expected_revision, placement }, signal);
                break;
            }
            case 'designer_remove':
                await panel.agentEdit(args.expected_revision, { type: 'removeNode', node: args.node! }, signal);
                break;
            case 'designer_edit_property':
                if (args.action === 'set' && args.value === undefined) throw new Error('Supply the property value.');
                await panel.agentEdit(args.expected_revision, args.action === 'set'
                    ? { type: 'setProperty', node: args.node!, name: args.property, value: args.value! }
                    : { type: 'resetProperty', node: args.node!, name: args.property }, signal);
                break;
            case 'designer_set_event':
                if (args.action === 'set' && !args.handler) throw new Error('Supply a handler name or expression.');
                await panel.agentEdit(args.expected_revision, { type: 'setEvent', node: args.node!, name: args.event,
                    value: args.action === 'set' ? args.handler! : null }, signal);
                break;
            case 'designer_apply_resource':
                await panel.agentEdit(args.expected_revision, { type: 'applyResource', frame: args.expected_revision,
                    resource: args.resource, node: args.node!, property: args.property, dynamic: args.dynamic ?? false }, signal);
                break;
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

    private async placement(panel: DesignerPanel, args: Arguments): Promise<Placement> {
        if ([args.node, args.asset, args.project_asset].filter(value => value !== undefined).length !== 1) {
            throw new Error('Specify one Toolbox control, project image, or source node.');
        }
        const origin: DragOrigin = args.node !== undefined ? { type: 'node', node: args.node }
            : args.project_asset !== undefined ? await this.workbench.projectImage(panel, args.project_asset)
                : { type: 'asset', control: args.asset! };
        const hasOutline = args.anchor !== undefined && args.position !== undefined;
        const hasPoint = args.x !== undefined && args.y !== undefined;
        if (hasOutline === hasPoint) throw new Error('Specify one outline anchor and position or one canvas point.');
        const target: DropTarget = hasOutline ? { type: 'outline', node: args.anchor!, position: args.position! }
            : { type: 'canvas', x: args.x!, y: args.y! };
        return { origin, target };
    }
}

function describe(panel: DesignerPanel) {
    const snapshot = panel.snapshot;
    const active = snapshot?.activeBlock?.start;
    const blocks = snapshot?.blocks.map((range, index) => ({ index, range,
        active: range.start.line === active?.line && range.start.character === active.character })) ?? [];
    return { session: panel.session, uri: panel.document.uri.toString(), language: panel.language, version: panel.document.version,
        revision: panel.currentToken ?? null, selected: panel.selectedNode?.id ?? null,
        inspected: panel.runtimeSelection?.inspector_id ?? null, dirty: panel.document.isDirty,
        viewport: panel.dimensions, status: panel.status ?? snapshot?.renderError ?? 'Ready', notices: snapshot?.notices ?? [], blocks };
}

function assertVersion(document: vscode.TextDocument, version: number): void {
    if (document.version !== version) throw new Error('The source buffer changed. Read its latest version.');
}
