import type { Webview } from 'vscode';
import type { CanvasAction, CanvasState, FromDesigner, ToDesigner } from './protocol';

type Result = Extract<FromDesigner, { type: 'canvasResult' }>;

export class CanvasRequests {
    private next = 0;
    private readonly pending = new Map<number, (result: Result) => void>();

    execute(webview: Webview, action: CanvasAction, signal: AbortSignal): Promise<CanvasState> {
        signal.throwIfAborted();
        const request = ++this.next;
        const message: ToDesigner = { type: 'canvasAction', request, action };
        return new Promise((resolve, reject) => {
            const cancel = () => finish({ type: 'canvasResult', request, error: 'The canvas request was cancelled.' });
            const finish = (result: Result) => {
                if (!this.pending.delete(request)) return;
                clearTimeout(timeout);
                signal.removeEventListener('abort', cancel);
                if (result.state) resolve(result.state);
                else reject(new Error(result.error ?? 'The canvas did not report its state.'));
            };
            const timeout = setTimeout(() => finish({ type: 'canvasResult', request, error: 'The canvas did not respond.' }), 5000);
            this.pending.set(request, finish);
            signal.addEventListener('abort', cancel, { once: true });
            void Promise.resolve(webview.postMessage(message)).then(delivered => {
                if (!delivered) finish({ type: 'canvasResult', request, error: 'The canvas is no longer available.' });
            }, cancel);
        });
    }

    receive(message: Result): void { this.pending.get(message.request)?.(message); }

    dispose(): void {
        for (const [request, finish] of this.pending) finish({ type: 'canvasResult', request, error: 'The designer was closed.' });
    }
}
