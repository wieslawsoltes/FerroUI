import * as vscode from 'vscode';
import { agentCatalog, type AgentTool } from './agentCatalog';
import { validate } from './agentSchema';
import type { CargoSelection } from './cargoTarget';
import type { TokenRequest } from './designTokenProtocol';
import type { DesignerPanel } from './designerPanel';
import type { DesignerWorkbench } from './designerWorkbench';
import { runEditorHistory } from './editorHistory';
import type { HotReload } from './hotReload';
import type { HostedDesign } from './hostedDesign';
import type { HostedAnimationControl, HostedKeyframeDraft } from './hostedAnimationProtocol';
import type { ProjectFile, ProjectInventory } from './projectInventory';
import type { ProjectScaffold } from './projectScaffold';
import type { RuntimeSourceTools } from './runtimeSourceTools';
import type { ScenarioRunner } from './scenarioRunner';
import { renameRsxControl } from './rsxRename';
import type { RuntimeTarget } from './runtimeConnections';
import type { StagesPanel } from './stagesPanel';
import type { PrototypeCommand, StageChange } from './stagesProtocol';
import type { TimelineControl } from './timelineProtocol';
import type { StateTransitionControl } from './stateTransitionProtocol';
import type { CanvasAction, CanvasInteraction, DesignerView, DragOrigin, DropTarget, OutlineMode, Placement } from './protocol';
import { applyTextEdits } from './textDocument';
import { isWorkspaceTool } from './workspaceCatalog';
import { isWorkspaceFile, resolveWorkspaceFile } from './workspacePaths';
import { bufferDiagnostics, WorkspaceTools } from './workspaceTools';

interface Arguments extends CanvasAction, CargoSelection {
    session: number;
    expected_revision: number;
    expected_version: number;
    expected_generation: number;
    expected_sequence: number;
    runtime?: RuntimeTarget;
    accept_baselines?: boolean;
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
    moving_anchor?: number;
    handle?: number;
    dx?: number;
    dy?: number;
    position?: 'inside' | 'before' | 'after';
    x?: number;
    y?: number;
    label: string;
    edits: object[];
    source: string;
    action: string;
    tool: string;
    arguments: object;
    selection?: number[];
    stage?: number;
    expected_epoch?: number;
    debug_configuration?: string;
    input?: CanvasInteraction;
    kind?: string;
    language?: 'xaml' | 'rsx';
    fixture?: string;
}

export class AgentTools {
    private readonly workspace = new WorkspaceTools();

    constructor(private readonly workbench: DesignerWorkbench, private readonly reload: HotReload, private readonly stages: StagesPanel,
        private readonly runtimeSource: RuntimeSourceTools, private readonly project: ProjectInventory, private readonly scaffolds: ProjectScaffold,
        private readonly scenarios: ScenarioRunner, private readonly hosted: HostedDesign) {}

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
        if (tool === 'designer_design_tokens') return this.project.designTokens(input as TokenRequest, signal);
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
        if (tool === 'ferroui_reload_history') return { records: this.reload.history.recent(args.limit ?? 50) };
        if (tool === 'ferroui_reload_control') return this.reload.agentControl(args.action, args, signal, args.debug_configuration);
        if (tool === 'ferroui_runtime_source') return this.runtimeSource.read(args.node!, signal, args.runtime);
        if (tool === 'ferroui_runtime_apply_source') {
            return this.runtimeSource.apply(args.node!, args.property, args.value!, args.expected_generation, args.expected_version, signal, args.runtime);
        }
        if (tool === 'ferroui_project_files') return this.project.list(args.folder, args.kind as ProjectFile['kind'] | undefined, args.offset ?? 0, args.limit ?? 100, signal);
        if (tool === 'ferroui_project_scaffold') return this.scaffolds.create(args.kind as 'view' | 'component', args.language!, args.path, args.folder, signal);
        if (tool === 'ferroui_scenario_run') {
            const uri = await resolveWorkspaceFile(args.path, args.folder);
            return this.scenarios.run(uri, signal, args.accept_baselines);
        }

        const panel = this.workbench.designs().find(panel => panel.session === args.session);
        if (!panel || !await isWorkspaceFile(panel.document.uri)) throw new Error('Select an open design inside the current workspace.');
        signal.throwIfAborted();
        const document = panel.document;
        switch (tool) {
            case 'designer_hosted_open': return this.hosted.open(document, args, signal);
            case 'designer_hosted_fixtures': return this.hosted.fixtures(document.uri);
            case 'designer_hosted_fixture':
                return this.hosted.chooseFixture(document.uri, args.expected_generation, args.expected_revision, args.fixture ?? null, signal);
            case 'designer_hosted_animations': return this.hosted.animations(document.uri);
            case 'designer_hosted_image': return this.hosted.image(document.uri);
            case 'designer_hosted_animation':
                return this.hosted.animate(document.uri, args.expected_generation, { revision: args.expected_revision, sequence: args.expected_sequence },
                    args.arguments as HostedAnimationControl | { action: 'reveal'; program: number }, signal);
            case 'designer_hosted_keyframe_draft':
            case 'designer_hosted_keyframe':
                return this.hosted.keyframe(document.uri, args.expected_generation, { revision: args.expected_revision, sequence: args.expected_sequence },
                    args.arguments as HostedKeyframeDraft, tool === 'designer_hosted_keyframe', signal);
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
                if ([args.node, args.inspector_id, args.selection].filter(value => value !== undefined).length > 1) throw new Error('Choose source nodes or one runtime object.');
                if (args.selection && args.selection.length !== 1) return panel.inspect(frame, { type: 'selection', nodes: args.selection });
                const node = args.node ?? args.selection?.[0] ?? panel.selectedNode?.id;
                const runtime = args.inspector_id ?? (args.node === undefined && !args.selection ? panel.runtimeSelection?.inspector_id : undefined);
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
            case 'designer_canvas_input':
                await panel.interact(args.expected_revision, args.input!, signal);
                return describe(panel);
            case 'designer_select': {
                const frame = { session: panel.session, token: args.expected_revision };
                if (!panel.canUse(frame)) throw new Error('Read the current outline before selecting.');
                if ([args.node, args.inspector_id, args.selection].filter(value => value !== undefined).length !== 1) {
                    throw new Error('Choose a source node, an ordered source selection, or a runtime object.');
                }
                if (args.inspector_id !== undefined) {
                    const row = (await panel.runtimeTree(frame, 'combined')).find(row => row.inspector_id === args.inspector_id);
                    signal.throwIfAborted();
                    if (!row || !panel.canUse(frame)) throw new Error('Read the current runtime tree before selecting.');
                    await panel.selectRuntime(frame, row, args.reveal_source ?? false);
                } else if (args.selection) {
                    await panel.selectNodes(frame, args.selection);
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
            case 'designer_resize':
                await panel.agentEdit(args.expected_revision, { type: 'resize', frame: args.expected_revision,
                    node: args.node!, handle: args.handle!, dx: args.dx!, dy: args.dy! }, signal);
                break;
            case 'designer_edit_property':
                if (args.action === 'set' && args.value === undefined) throw new Error('Supply the property value.');
                await this.workbench.editPropertyForAgent(panel, args.expected_revision, args.action === 'set'
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
            case 'designer_rename_rsx_control':
                await renameRsxControl(panel, args.expected_revision, args.node!, args.value!, signal);
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
                if (args.action !== 'undo' && args.action !== 'redo') throw new Error('Choose undo or redo.');
                await runEditorHistory(document, args.action, () => assertVersion(document, args.expected_version), signal);
                break;
            case 'designer_save':
                assertVersion(document, args.expected_version);
                if (!await document.save()) throw new Error('VS Code could not save the buffer.');
                break;
            case 'designer_studio_tools': return panel.studioTools();
            case 'designer_studio_query': return panel.studioQuery(args.tool, args.arguments, args.selection, signal);
            case 'designer_studio_edit':
                await panel.studioEdit(args.expected_revision, args.tool, args.arguments, args.selection, signal);
                break;
            case 'designer_studio_present':
                await panel.studioPresent(args.expected_revision, args.tool, args.arguments, args.selection, signal);
                break;
            case 'designer_timeline_control':
                await this.workbench.revealDesign(panel, 'timeline');
                return this.workbench.controlTimeline(panel, { session: panel.session, token: args.expected_revision },
                    args.arguments as TimelineControl, signal);
            case 'designer_transition_control':
                await this.workbench.revealDesign(panel, 'timeline');
                return await this.workbench.controlStateTransition(panel, { session: panel.session, token: args.expected_revision },
                    args.arguments as StateTransitionControl, signal) ?? describe(panel);
            case 'designer_stages_open': {
                const block = panel.snapshot?.activeBlock?.start;
                return this.stages.open(document, block ? new vscode.Position(block.line, block.character) : new vscode.Position(0, 0)).wait(signal);
            }
            case 'designer_stages': return this.stages.board(document.uri).wait(signal);
            case 'designer_stage_image': {
                const image = await this.stages.board(document.uri).capture(args.stage!, args.expected_epoch!, signal);
                return { mimeType: 'image/png', data: image.png, width: image.width, height: image.height };
            }
            case 'designer_stage_hotspots': return this.stages.board(document.uri).hotspots(args.stage!, args.expected_epoch!, signal);
            case 'designer_stage_checks': return this.stages.board(document.uri).checks(args.stage!, args.expected_epoch!, signal);
            case 'designer_stage_states': return this.stages.board(document.uri).stateOptions(args.stage!, args.expected_epoch!, signal);
            case 'designer_stage_fixtures': return this.stages.board(document.uri).fixtureOptions(args.stage!, args.expected_epoch!, signal);
            case 'designer_stage_change': return this.stages.board(document.uri).applyChange({ ...args.arguments, epoch: args.expected_epoch! } as StageChange, signal);
            case 'designer_stage_prototype': return this.stages.board(document.uri).controlPrototype(args.expected_epoch!, args.arguments as PrototypeCommand, signal);
        }
        return describe(panel);
    }

    private async placement(panel: DesignerPanel, args: Arguments): Promise<Placement> {
        if ([args.node, args.asset, args.project_asset, args.selection].filter(value => value !== undefined).length !== 1) {
            throw new Error('Specify a Toolbox control, project image, source node, or selection.');
        }
        const origin: DragOrigin = args.selection ? { type: 'nodes', nodes: args.selection, anchor: args.moving_anchor ?? args.selection[0] }
            : args.node !== undefined ? { type: 'node', node: args.node }
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
        revision: panel.currentToken ?? null, selected: panel.selectedNode?.id ?? null, selection: panel.selectedIds,
        inspected: panel.runtimeSelection?.inspector_id ?? null, dirty: panel.document.isDirty,
        viewport: panel.dimensions, status: panel.status ?? snapshot?.renderError ?? 'Ready', notices: snapshot?.notices ?? [], blocks };
}

function assertVersion(document: vscode.TextDocument, version: number): void {
    if (document.version !== version) throw new Error('The source buffer changed. Read its latest version.');
}
