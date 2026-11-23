import * as vscode from 'vscode';
import { webviewOptions } from './view';

export class DockView<State, Incoming extends { type: string }, Outgoing> implements vscode.WebviewViewProvider, vscode.Disposable {
    private view?: vscode.WebviewView;
    private subscriptions: vscode.Disposable[] = [];
    private ready = false;
    private state?: State;
    private description = '';

    constructor(
        private readonly root: vscode.Uri,
        private readonly html: (view: vscode.Webview, root: vscode.Uri) => string,
        private readonly visibilityChanged: () => void,
        private readonly receive: (message: Incoming) => void,
    ) {}

    get visible(): boolean { return Boolean(this.view?.visible); }

    resolveWebviewView(view: vscode.WebviewView): void {
        this.dispose();
        this.view = view;
        view.description = this.description;
        this.subscriptions = [
            view.onDidDispose(() => { this.dispose(); this.visibilityChanged(); }),
            view.onDidChangeVisibility(() => { this.visibilityChanged(); this.sendState(); }),
            view.webview.onDidReceiveMessage((message: Incoming) => {
                if (!message || this.view !== view) return;
                if (message.type === 'ready') {
                    this.ready = true;
                    this.sendState();
                } else this.receive(message);
            }),
        ];
        view.webview.options = webviewOptions(this.root);
        view.webview.html = this.html(view.webview, this.root);
        this.visibilityChanged();
    }

    show(state: State, file: string): void {
        this.state = state;
        this.description = file;
        if (this.view && this.view.description !== file) this.view.description = file;
        this.sendState();
    }

    post(message: Outgoing | { type: 'state'; state: State }): Thenable<boolean> | undefined {
        if (this.visible && this.ready) return this.view?.webview.postMessage(message);
    }

    private sendState(): void {
        if (this.state !== undefined) this.post({ type: 'state', state: this.state });
    }

    dispose(): void {
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.subscriptions = [];
        this.view = undefined;
        this.ready = false;
    }
}
