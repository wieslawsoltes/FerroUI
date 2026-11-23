import * as vscode from 'vscode';
import { validate } from './agentSchema';
import type { HotReload } from './hotReload';
import { errorMessage } from './protocol';
import { RuntimeClient } from './runtimeClient';
import type { RuntimeConnections } from './runtimeConnections';
import { readRuntimeInspection, showLivePropertyTrace, showLiveTemplates } from './runtimeInspection';
import { runtimeViewChangeSchema } from './runtimeMcpCatalog';
import type { FromRuntime, RuntimeFrame, RuntimeInspectionResult, RuntimeNode, RuntimePresentation, RuntimeProperty,
    RuntimeSelection, RuntimeViewChange, RuntimeViewSnapshot, ToRuntime } from './runtimeProtocol';
import type { RuntimeSourceEditor } from './runtimeSource';
import type { AppRuntime } from './runtimeTargets';
import { runtimeHtml, webviewOptions } from './view';

const refreshInterval = 500;
let nextViewRevision = 0;

export class RuntimePanel implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private panel?: vscode.WebviewPanel;
    private client?: RuntimeClient;
    private target?: AppRuntime;
    private generation?: number;
    private presentation: RuntimePresentation = { hierarchy: 'visual', inspection: 'properties', mode: 'inspect', treeFilter: '', propertyFilter: '' };
    private inspection?: RuntimeInspectionResult & { revision: number };
    private inspectionRequest = 0;
    private windows: RuntimeNode[] = [];
    private windowId?: number;
    private frame?: RuntimeFrame;
    private nextFrame = 0;
    private revision = ++nextViewRevision;
    private epoch = 0;
    private timer?: ReturnType<typeof setTimeout>;
    private refreshRequested = false;
    private pending = false;
    private ready = false;

    constructor(private readonly context: vscode.ExtensionContext, private readonly connections: RuntimeConnections,
        reload: HotReload, private readonly authoredSource: RuntimeSourceEditor,
        private readonly pinned: AppRuntime | undefined, private readonly closed: () => void) {
        this.subscriptions = [
            connections.onDidChange(() => this.reset()),
            reload.onDidChangeRuns(() => this.postState()),
        ];
    }

    show(): void {
        if (!this.panel) {
            const summary = this.pinned && this.connections.describe(this.pinned);
            const title = summary ? `Live App · ${summary.label}` : 'FerroUI Live App';
            const panel = vscode.window.createWebviewPanel('ferroui.liveApp', title, vscode.ViewColumn.Beside,
                { ...webviewOptions(this.context.extensionUri), retainContextWhenHidden: true });
            panel.webview.html = runtimeHtml(panel.webview, this.context.extensionUri);
            panel.webview.onDidReceiveMessage((message: FromRuntime) => { void this.receive(message); });
            let visible = panel.visible;
            panel.onDidChangeViewState(() => {
                if (panel.visible && !visible) this.postSnapshot();
                visible = panel.visible;
                this.schedule(0);
            });
            panel.onDidDispose(() => {
                this.panel = undefined;
                this.ready = false;
                this.dispose();
                this.closed();
            });
            this.panel = panel;
            this.reset();
        } else this.panel.reveal();
    }

    view(): RuntimeViewSnapshot {
        if (!this.panel || !this.target || this.generation === undefined) {
            throw new Error('Wait for the Live App editor’s inspector connection.');
        }
        return {
            runtime: this.target, generation: this.generation, revision: this.revision, frame: this.frame?.token ?? null,
            window: this.windowId ?? null, windows: this.windows, ...this.presentation,
            selection: this.inspection ? { node: this.inspection.node, view: this.inspection.view } : null,
        };
    }

    async control(generation: number, revision: number, change: RuntimeViewChange, signal: AbortSignal): Promise<RuntimeViewSnapshot> {
        signal.throwIfAborted();
        if (generation !== this.generation || revision !== this.revision) {
            throw new Error('The Live App editor changed. Read ferroui_live_app_view again.');
        }
        await this.updateView(change, signal);
        signal.throwIfAborted();
        if (generation !== this.generation) throw new Error('The application connection changed. Read ferroui_live_app_view again.');
        return this.view();
    }

    private reset(reconnect = false, preferred = this.target): void {
        const target = this.pinned ?? (preferred && this.connections.describe(preferred) ? preferred : this.connections.defaultRuntime());
        const generation = target && this.connections.revision(target);
        const changed = target !== this.target || generation !== this.generation;
        if (!this.panel || !changed && !reconnect) { this.postState(); return; }
        this.target = target;
        this.generation = generation;
        if (changed) {
            this.windows = [];
            this.windowId = undefined;
            this.present({ mode: 'inspect' });
        }
        this.invalidateFrame();
        this.client?.dispose();
        const endpoint = target && this.connections.get(target);
        this.client = endpoint ? new RuntimeClient(endpoint) : undefined;
        this.postState();
        this.schedule(0, reconnect);
    }

    private touch(): void { this.revision = ++nextViewRevision; }

    private present(presentation: Partial<RuntimePresentation>, echo = true): void {
        this.presentation = { ...this.presentation, ...presentation };
        this.touch();
        if (echo) this.post({ type: 'presentation', presentation });
    }

    private invalidateFrame(): void {
        this.epoch++;
        this.touch();
        this.frame = undefined;
        this.inspection = undefined;
        this.post({ type: 'invalidateFrame' });
    }

    private postState(): void {
        const targets = this.connections.targets().filter(target => !this.pinned || target.runtime === this.pinned);
        const summary = targets.find(item => item.runtime === this.target);
        const options = targets.map(target => ({ runtime: target.runtime,
            label: targets.some(other => other !== target && other.label === target.label)
                ? `${target.label} · ${target.pid ?? target.runtime.split(':')[1].slice(0, 8)}` : target.label }));
        const status = summary ? `${summary.status}${summary.pid && !summary.available ? ' · Inspector unavailable' : ''}`
            : this.pinned ? 'App session ended.' : 'Run a FerroUI app with MCP enabled, or connect to a running app.';
        this.post({ type: 'state', target: summary?.runtime, targets: options, generation: summary?.generation ?? undefined,
            windows: this.windows, root: this.windowId,
            status });
    }

    private schedule(delay = refreshInterval, force = false): void {
        if (force) this.refreshRequested = true;
        clearTimeout(this.timer);
        if (this.canPoll()) this.timer = setTimeout(() => { void this.tick(); }, delay);
    }

    private canPoll(): boolean {
        return Boolean(this.panel && (this.refreshRequested || this.ready && this.panel.visible));
    }

    private async tick(): Promise<void> {
        const client = this.client;
        if (!this.canPoll() || this.pending) return;
        this.refreshRequested = false;
        this.pending = true;
        let epoch = this.epoch;
        try {
            if (!client) return;
            const windows = await client.call<RuntimeNode[]>('list_windows');
            if (epoch !== this.epoch) return;
            if (windows.length !== this.windows.length || windows.some((window, i) => window.node_id !== this.windows[i]?.node_id
                || window.name !== this.windows[i]?.name || window.type !== this.windows[i]?.type)) this.touch();
            this.windows = windows;
            const root = windows.find(window => window.node_id === this.windowId) ?? windows[0];
            if (root?.node_id !== this.windowId) {
                this.windowId = root?.node_id;
                this.present({ mode: 'inspect' });
                this.invalidateFrame();
                epoch = this.epoch;
            }
            this.postState();
            if (!root) return;
            const [tree, png] = await Promise.all([
                client.call<RuntimeNode[]>('get_tree', { node_id: root.node_id, tree: this.presentation.hierarchy, depth: 10 }),
                client.call<string>('screenshot', { node_id: root.node_id }),
            ]);
            if (epoch !== this.epoch) return;
            if (typeof png !== 'string' || !png.startsWith('iVBORw0KGgo')) throw new Error('The app inspector did not return a native PNG.');
            if (this.frame?.png === png && JSON.stringify(this.frame.tree) === JSON.stringify(tree)) return;
            this.frame = { token: ++this.nextFrame, root, tree, png };
            if (this.inspection && !findNode(this.frame, this.inspection.node)) {
                this.inspection = undefined;
                this.touch();
                this.post({ type: 'clearSelection' });
            }
            this.postFrame();
        } catch (error) {
            if (epoch === this.epoch) this.post({ type: 'problem', message: errorMessage(error) });
        } finally {
            this.pending = false;
            this.schedule(epoch === this.epoch && !this.refreshRequested ? refreshInterval : 0);
        }
    }

    private async updateView(change: RuntimeViewChange, signal?: AbortSignal, echoFilters = true): Promise<void> {
        switch (change.action) {
            case 'refresh':
                this.reset(true);
                return;
            case 'mode':
                if (change.mode !== this.presentation.mode) this.present({ mode: change.mode });
                return;
            case 'filter':
                if (change.query !== this.presentation[change.field]) this.present({ [change.field]: change.query }, echoFilters);
                return;
            case 'window':
                if (!this.windows.some(window => window.node_id === change.window)) throw new Error('Choose a current Live App window.');
                if (this.windowId === change.window) return;
                this.windowId = change.window;
                this.present({ mode: 'inspect' });
                break;
            case 'hierarchy':
                if (change.hierarchy === this.presentation.hierarchy) return;
                this.present({ hierarchy: change.hierarchy });
                break;
            case 'select': {
                const client = this.client;
                const frame = this.frame;
                if (!client || frame?.token !== change.frame || !findNode(frame, change.node)
                    || !await this.inspect(client, frame.token, { node: change.node, view: change.view }, signal)) {
                    throw new Error('The Live App frame changed or the node is outside the current hierarchy. Read the current view.');
                }
                return;
            }
        }
        this.invalidateFrame();
        this.postState();
        this.schedule(0, true);
    }

    private async receive(message: FromRuntime): Promise<void> {
        if (!message || typeof message !== 'object' || !vscode.workspace.isTrusted) return;
        if (message.type === 'ready') {
            this.ready = true;
            this.reset();
            this.postSnapshot();
            this.schedule(0);
            return;
        }
        if (message.type === 'start' || message.type === 'attach') {
            await vscode.commands.executeCommand(message.type === 'start' ? 'ferroui.hotReload.start' : 'ferroui.mcp.attach');
            if (!this.pinned) {
                this.reset(false, this.connections.describe(message.type === 'attach' ? 'attached' : 'managed')?.runtime);
            }
            return;
        }
        if (message.type === 'target') {
            if (!this.pinned && this.connections.targets().some(target => target.runtime === message.target)) {
                this.reset(false, message.target);
            }
            return;
        }
        if (message.type === 'presentation') {
            if (this.generation === undefined || message.generation !== this.generation) return;
            try {
                validate(runtimeViewChangeSchema, message.change);
                await this.updateView(message.change, undefined, false);
            } catch (error) {
                if (message.generation !== this.generation) return;
                this.postState();
                this.post({ type: 'presentation', presentation: this.presentation });
                this.post({ type: 'problem', message: errorMessage(error) });
            }
            return;
        }
        if (!('token' in message) || !this.client || !this.frame || this.frame.token !== message.token) return;
        const client = this.client;
        const epoch = this.epoch;
        const frame = this.frame;
        const runtime = this.target;
        if (!runtime) return;
        const withinScope = () => epoch === this.epoch && client === this.client;
        const current = () => withinScope() && this.frame?.token === frame.token;
        try {
            if (message.type === 'pointer') {
                const bounds = frame.root.bounds;
                if (!bounds || !inUnit(message.x) || !inUnit(message.y)) return;
                if (message.interact !== (this.presentation.mode === 'interact')) return;
                const x = message.x * bounds.width;
                const y = message.y * bounds.height;
                if (this.presentation.mode === 'interact') {
                    await client.call('click', { node_id: frame.root.node_id, x, y });
                    this.schedule(0);
                    return;
                }
                const request = ++this.inspectionRequest;
                this.touch();
                const native = await client.supports('get_element_at');
                const hits = native ? await client.call<RuntimeNode[]>('get_element_at', { node_id: frame.root.node_id, x, y })
                    : flatten(frame.tree).filter(node => {
                    const box = node.bounds_in_root;
                    return node.visible !== false && box && x >= box.x && y >= box.y && x < box.x + box.width && y < box.y + box.height;
                }).reverse();
                const picked = message.overlap && hits.length > 1
                    ? (await vscode.window.showQuickPick(hits.map(node => ({ label: node.name || node.type, description: node.type, node })),
                        { title: 'FerroUI controls at this point' }))?.node : hits[0];
                if (message.overlap && !picked) return;
                if (!current() || request !== this.inspectionRequest) return;
                await this.inspect(client, frame.token, { node: picked?.node_id ?? frame.root.node_id, view: 'properties' });
                return;
            }
            if (message.type === 'reveal') {
                const source = await this.authoredSource.read(client, message.node);
                if (!source || !current()) return;
                const { document, position } = source;
                await vscode.window.showTextDocument(document, { selection: new vscode.Range(position, position), viewColumn: vscode.ViewColumn.Beside });
                return;
            }
            const node = findNode(frame, message.node);
            if (!node) return;
            const node_id = message.node;
            if (message.type === 'templates') {
                await showLiveTemplates(client, this.authoredSource, node, withinScope);
                return;
            }
            if (message.type === 'trace') {
                if (typeof message.property !== 'string' || !message.property || message.property.length > 256) return;
                await showLivePropertyTrace(client, this.authoredSource, node_id, message.property, withinScope);
                return;
            }
            if ((message.type === 'text' || message.type === 'key' || message.type === 'scroll') && this.presentation.mode !== 'interact') return;
            if (message.type === 'text' && typeof message.text === 'string' && message.text.length <= 8192) {
                await client.call('type_text', { node_id, text: message.text });
            } else if (message.type === 'key' && typeof message.key === 'string' && message.key.length <= 64) {
                await client.call('press_key', { node_id, key: message.key });
            } else if (message.type === 'scroll' && Number.isFinite(message.x) && Number.isFinite(message.y)) {
                await client.call('scroll', { node_id, delta_x: message.x, delta_y: message.y });
            } else if (message.type === 'assign' && typeof message.property === 'string' && typeof message.value === 'string' && message.value.length <= 4096) {
                const properties = await client.call<RuntimeProperty[]>('get_properties', { node_id, properties: [message.property] });
                if (!current() || !properties.some(property => property.name === message.property && !property.read_only && !property.binding)) return;
                await client.call('set_property', { node_id, property: message.property, value: message.value });
                await this.inspect(client, frame.token, { node: node_id, view: 'properties' });
            } else if (message.type === 'assignSource' && typeof message.property === 'string' && typeof message.value === 'string' && message.value.length <= 4096) {
                await this.authoredSource.apply(client, node, message.property, message.value, AbortSignal.timeout(15_000),
                    runtime, current);
            }
            this.schedule(0);
        } catch (error) {
            if (epoch === this.epoch) this.post({ type: 'problem', message: errorMessage(error) });
        }
    }

    private async inspect(client: RuntimeClient, token: number, selection: RuntimeSelection, signal?: AbortSignal): Promise<boolean> {
        if (!this.target || client !== this.client || this.frame?.token !== token) return false;
        const request = ++this.inspectionRequest;
        this.touch();
        const inspection = await readRuntimeInspection(client, this.authoredSource, this.target, selection, signal);
        signal?.throwIfAborted();
        if (this.frame?.token !== token || request !== this.inspectionRequest) return false;
        this.inspection = { ...inspection, revision: request };
        this.present({ inspection: inspection.view });
        this.postInspection(token);
        return true;
    }

    private postInspection(token: number): void {
        if (this.inspection) {
            const { revision, ...inspection } = this.inspection;
            this.post({ type: 'inspection', token, revision, inspection });
        }
    }

    private postFrame(): void {
        if (this.frame) this.post({ type: 'frame', frame: this.frame });
    }

    private postSnapshot(): void {
        this.postState();
        this.post({ type: 'presentation', presentation: this.presentation });
        if (!this.frame) this.post({ type: 'invalidateFrame' });
        else {
            this.postFrame();
            if (this.inspection) this.postInspection(this.frame.token);
            else this.post({ type: 'clearSelection' });
        }
    }

    private post(message: ToRuntime): void {
        if (this.ready) void this.panel?.webview.postMessage(message);
    }

    dispose(): void {
        if (this.panel) { this.panel.dispose(); return; }
        this.generation = undefined;
        this.invalidateFrame();
        this.client?.dispose();
        clearTimeout(this.timer);
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}

function flatten(roots: RuntimeNode[]): RuntimeNode[] {
    return roots.flatMap(node => [node, ...flatten(node.children ?? [])]);
}

function findNode(frame: RuntimeFrame, id: number): RuntimeNode | undefined {
    return frame.root.node_id === id ? frame.root : flatten(frame.tree).find(node => node.node_id === id);
}

function inUnit(number: number): boolean { return number >= 0 && number <= 1; }
