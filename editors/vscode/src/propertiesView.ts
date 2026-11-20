import * as vscode from 'vscode';
import type { FrameTarget, FromProperties, NodeOperation, NodeTarget, PropertiesState, PropertyAuthoring, Range, ToProperties } from './protocol';
import { errorMessage } from './protocol';
import { propertiesHtml, webviewOptions } from './view';

export const PROPERTIES_VIEW = 'ferroui.properties';

export class PropertiesView implements vscode.WebviewViewProvider, vscode.Disposable {
    private view?: vscode.WebviewView;
    private subscriptions: vscode.Disposable[] = [];
    private ready = false;
    private state: PropertiesState = {};

    constructor(
        private readonly root: vscode.Uri,
        private readonly visibilityChanged: () => void,
        private readonly edit: (target: NodeTarget, operation: NodeOperation) => Promise<void>,
        private readonly openToolbox: () => void,
        private readonly reveal: (frame: FrameTarget, range: Range) => Promise<void>,
        private readonly author: (target: NodeTarget, authoring: PropertyAuthoring) => Promise<void>,
    ) {}

    get visible(): boolean { return this.view?.visible ?? false; }

    resolveWebviewView(view: vscode.WebviewView): void {
        this.dispose();
        this.view = view;
        this.subscriptions = [
            view.onDidDispose(() => { this.dispose(); this.visibilityChanged(); }),
            view.onDidChangeVisibility(() => { this.visibilityChanged(); this.sendState(); }),
            view.webview.onDidReceiveMessage((message: FromProperties) => { void this.receive(view, message); }),
        ];
        view.webview.options = webviewOptions(this.root);
        view.webview.html = propertiesHtml(view.webview, this.root);
        this.visibilityChanged();
    }

    show(state: PropertiesState, file: string): void {
        this.state = state;
        if (this.view && this.view.description !== file) this.view.description = file;
        this.sendState();
    }

    private async receive(view: vscode.WebviewView, message: FromProperties): Promise<void> {
        if (this.view !== view || !message) return;
        if (message.type === 'ready') {
            this.ready = true;
            this.sendState();
        } else if (message.type === 'toolbox') {
            this.openToolbox();
        } else if (message.type === 'reveal') {
            await this.reveal(message.frame, message.range);
        } else if (message.type === 'edit' || message.type === 'author') {
            let error: string | undefined;
            try {
                if (message.type === 'author') await this.author(message.target, message.authoring);
                else await this.edit(message.target, message.operation);
            } catch (failure) {
                error = errorMessage(failure);
            }
            if (this.view === view) this.post({ type: 'edit', request: message.request, error });
        }
    }

    private sendState(): void {
        if (this.ready && this.visible) this.post({ type: 'state', state: this.state });
    }

    private post(message: ToProperties): void {
        void this.view?.webview.postMessage(message);
    }

    dispose(): void {
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.subscriptions = [];
        this.view = undefined;
        this.ready = false;
    }
}
