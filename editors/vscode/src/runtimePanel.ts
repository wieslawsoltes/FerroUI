import * as vscode from 'vscode';
import type { HotReload } from './hotReload';
import { errorMessage } from './protocol';
import { RuntimeClient } from './runtimeClient';
import type { RuntimeConnections, RuntimeTarget } from './runtimeConnections';
import { readRuntimeDiagnostics } from './runtimeDiagnostics';
import type { FromRuntime, RuntimeFrame, RuntimeHierarchy, RuntimeInspection, RuntimeNode, RuntimeProperty, ToRuntime } from './runtimeProtocol';
import type { RuntimeSourceEditor } from './runtimeSource';
import { runtimeHtml, webviewOptions } from './view';

const refreshInterval = 500;

export class RuntimePanel implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private panel?: vscode.WebviewPanel;
    private client?: RuntimeClient;
    private target: RuntimeTarget = 'managed';
    private hierarchy: RuntimeHierarchy = 'visual';
    private windows: RuntimeNode[] = [];
    private windowId?: number;
    private frame?: RuntimeFrame;
    private revision = 0;
    private epoch = 0;
    private timer?: ReturnType<typeof setTimeout>;
    private pending = false;
    private ready = false;

    constructor(private readonly context: vscode.ExtensionContext, private readonly connections: RuntimeConnections,
        private readonly reload: HotReload, private readonly authoredSource: RuntimeSourceEditor) {
        this.subscriptions = [
            vscode.commands.registerCommand('ferroui.openLiveApp', () => this.show()),
            connections.onDidChange(() => this.reset()),
        ];
    }

    private show(): void {
        if (!this.panel) {
            const panel = vscode.window.createWebviewPanel('ferroui.liveApp', 'FerroUI Live App', vscode.ViewColumn.Beside,
                { ...webviewOptions(this.context.extensionUri), retainContextWhenHidden: true });
            panel.webview.html = runtimeHtml(panel.webview, this.context.extensionUri);
            panel.webview.onDidReceiveMessage((message: FromRuntime) => { void this.receive(message); });
            panel.onDidChangeViewState(() => this.schedule(0));
            panel.onDidDispose(() => {
                this.panel = undefined;
                this.ready = false;
                this.reset();
            });
            this.panel = panel;
        } else this.panel.reveal();
    }

    private reset(): void {
        let endpoint = this.connections.get(this.target);
        if (!endpoint) {
            const alternate = this.target === 'managed' ? 'attached' : 'managed';
            endpoint = this.connections.get(alternate);
            if (endpoint) this.target = alternate;
        }
        if (this.panel && endpoint && endpoint === this.client?.endpoint) return;
        this.invalidate();
        this.client?.dispose();
        this.client = this.panel && endpoint ? new RuntimeClient(endpoint) : undefined;
        this.windows = [];
        this.windowId = undefined;
        this.postState();
        this.schedule(0);
    }

    private invalidate(): void {
        this.epoch++;
        this.frame = undefined;
    }

    private postState(): void {
        const targets = (['managed', 'attached'] as const).filter(target => this.connections.get(target));
        this.post({ type: 'state', target: this.client ? this.target : undefined, targets, windows: this.windows, root: this.windowId,
            status: this.client ? this.target === 'managed' ? `${this.reload.state.target ?? 'Managed application'} · ${this.reload.state.status}` : 'Attached application'
                : 'Run a FerroUI app with MCP enabled, or connect to a running app.' });
    }

    private schedule(delay = refreshInterval): void {
        clearTimeout(this.timer);
        if (this.ready && this.panel?.visible) this.timer = setTimeout(() => { void this.tick(); }, delay);
    }

    private async tick(): Promise<void> {
        const client = this.client;
        if (!this.ready || !this.panel?.visible || this.pending) return;
        this.pending = true;
        const epoch = this.epoch;
        try {
            if (!client) return;
            const windows = await client.call<RuntimeNode[]>('list_windows');
            if (epoch !== this.epoch) return;
            this.windows = windows;
            const root = windows.find(window => window.node_id === this.windowId) ?? windows[0];
            if (root?.node_id !== this.windowId) this.frame = undefined;
            this.windowId = root?.node_id;
            this.postState();
            if (!root) return;
            const [tree, png] = await Promise.all([
                client.call<RuntimeNode[]>('get_tree', { node_id: root.node_id, tree: this.hierarchy, depth: 10 }),
                client.call<string>('screenshot', { node_id: root.node_id }),
            ]);
            if (epoch !== this.epoch) return;
            if (this.frame?.png === png && JSON.stringify(this.frame.tree) === JSON.stringify(tree)) return;
            this.frame = { token: ++this.revision, root, tree, png };
            this.post({ type: 'frame', frame: this.frame });
        } catch (error) {
            if (epoch === this.epoch) this.post({ type: 'problem', message: errorMessage(error) });
        } finally {
            this.pending = false;
            this.schedule(epoch === this.epoch ? refreshInterval : 0);
        }
    }

    private async receive(message: FromRuntime): Promise<void> {
        if (!message || typeof message !== 'object' || !vscode.workspace.isTrusted) return;
        if (message.type === 'ready') {
            this.ready = true;
            this.reset();
            return;
        }
        if (message.type === 'start' || message.type === 'attach') {
            await vscode.commands.executeCommand(message.type === 'start' ? 'ferroui.hotReload.start' : 'ferroui.mcp.attach');
            if (message.type === 'attach' && this.connections.get('attached')) { this.target = 'attached'; this.reset(); }
            return;
        }
        if (message.type === 'target') {
            if (message.target === 'managed' || message.target === 'attached') {
                this.target = message.target;
                this.reset();
            }
            return;
        }
        if (message.type === 'refresh') {
            if (this.client) {
                this.client.dispose();
                this.client = undefined;
            }
            this.reset();
            return;
        }
        if (message.type === 'hierarchy' || message.type === 'window') {
            if (message.type === 'hierarchy') {
                if (!['visual', 'logical', 'automation'].includes(message.tree)) return;
                this.hierarchy = message.tree;
            } else {
                if (!this.windows.some(window => window.node_id === message.id)) return;
                this.windowId = message.id;
            }
            this.invalidate();
            this.postState();
            this.schedule(0);
            return;
        }
        if (!('token' in message) || !this.client || !this.frame || this.frame.token !== message.token) return;
        const client = this.client;
        const epoch = this.epoch;
        const frame = this.frame;
        try {
            const nodes = flatten(frame.tree);
            if (message.type === 'pointer') {
                const bounds = frame.root.bounds;
                if (!bounds || !inUnit(message.x) || !inUnit(message.y)) return;
                const x = message.x * bounds.width;
                const y = message.y * bounds.height;
                if (message.interact) {
                    await client.call('click', { node_id: frame.root.node_id, x, y });
                    this.schedule(0);
                    return;
                }
                const native = await client.supports('get_element_at');
                const hits = native ? await client.call<RuntimeNode[]>('get_element_at', { node_id: frame.root.node_id, x, y })
                    : nodes.filter(node => {
                    const box = node.bounds_in_root;
                    return node.visible !== false && box && x >= box.x && y >= box.y && x < box.x + box.width && y < box.y + box.height;
                }).reverse();
                const picked = message.overlap && hits.length > 1
                    ? (await vscode.window.showQuickPick(hits.map(node => ({ label: node.name || node.type, description: node.type, node })),
                        { title: 'FerroUI controls at this point' }))?.node : hits[0];
                if (message.overlap && !picked) return;
                if (epoch !== this.epoch || this.frame?.token !== frame.token) return;
                await this.inspect(client, epoch, frame.token, picked?.node_id ?? frame.root.node_id, 'properties');
                return;
            }
            if (message.type === 'reveal') {
                const source = await this.authoredSource.read(client, message.node);
                if (!source || epoch !== this.epoch || this.frame?.token !== frame.token) return;
                const { document, position } = source;
                await vscode.window.showTextDocument(document, { selection: new vscode.Range(position, position), viewColumn: vscode.ViewColumn.Beside });
                return;
            }
            const node = nodes.find(node => node.node_id === message.node);
            if (!node) return;
            const node_id = message.node;
            if (message.type === 'select') await this.inspect(client, epoch, frame.token, node_id, message.view);
            else if (message.type === 'text' && typeof message.text === 'string' && message.text.length <= 8192) {
                await client.call('type_text', { node_id, text: message.text });
            } else if (message.type === 'key' && typeof message.key === 'string' && message.key.length <= 64) {
                await client.call('press_key', { node_id, key: message.key });
            } else if (message.type === 'scroll' && Number.isFinite(message.x) && Number.isFinite(message.y)) {
                await client.call('scroll', { node_id, delta_x: message.x, delta_y: message.y });
            } else if (message.type === 'assign' && typeof message.property === 'string' && typeof message.value === 'string' && message.value.length <= 4096) {
                const properties = await client.call<RuntimeProperty[]>('get_properties', { node_id, properties: [message.property] });
                if (epoch !== this.epoch || !properties.some(property => property.name === message.property && !property.read_only && !property.binding)) return;
                await client.call('set_property', { node_id, property: message.property, value: message.value });
                await this.inspect(client, epoch, frame.token, node_id, 'properties');
            } else if (message.type === 'assignSource' && typeof message.property === 'string' && typeof message.value === 'string' && message.value.length <= 4096) {
                await this.authoredSource.apply(client, node, message.property, message.value, AbortSignal.timeout(15_000),
                    this.target, () => epoch === this.epoch && this.frame?.token === frame.token);
            }
            if (message.type !== 'select') this.schedule(0);
        } catch (error) {
            if (epoch === this.epoch) this.post({ type: 'problem', message: errorMessage(error) });
        }
    }

    private async inspect(client: RuntimeClient, epoch: number, token: number, node: number, view: RuntimeInspection): Promise<void> {
        let data: unknown;
        switch (view) {
            case 'properties': data = await client.call('get_properties', { node_id: node }); break;
            case 'styles': data = await client.call('get_styles', { node_id: node }); break;
            case 'resources': data = await client.call('get_resources', { node_id: node }); break;
            case 'diagnostics': data = await readRuntimeDiagnostics(client); break;
            default: return;
        }
        const source = await this.authoredSource.read(client, node);
        const canApply = view === 'properties' && source && this.authoredSource.canApply(source, this.target);
        if (epoch === this.epoch && this.frame?.token === token) this.post({ type: 'inspection', token, node, view, data, canReveal: Boolean(source), canApply: Boolean(canApply) });
    }

    private post(message: ToRuntime): void {
        if (this.ready) void this.panel?.webview.postMessage(message);
    }

    dispose(): void {
        this.panel?.dispose();
        this.client?.dispose();
        clearTimeout(this.timer);
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}

function flatten(roots: RuntimeNode[]): RuntimeNode[] {
    return roots.flatMap(node => [node, ...flatten(node.children ?? [])]);
}

function inUnit(number: number): boolean { return number >= 0 && number <= 1; }
