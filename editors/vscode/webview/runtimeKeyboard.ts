import type { KeyboardInput } from '../src/previewKeyboardProtocol';
import type { RuntimeKeyboardRequest, RuntimeKeyboardResult } from '../src/runtimeProtocol';
import { PreviewKeyboard } from './previewKeyboard';

export class RuntimeKeyboard {
    private readonly capture: PreviewKeyboard;
    private readonly queued: KeyboardInput[] = [];
    private scope?: number;
    private request = 0;
    private pending?: number;

    constructor(surface: HTMLElement, returnTo: HTMLElement, private readonly send: (request: RuntimeKeyboardRequest) => void,
        private readonly report: (message: string) => void) {
        this.capture = new PreviewKeyboard(surface, surface, 0, returnTo, input => this.enqueue(input), { onExit: () => {
            this.queued.length = 0;
            this.pending = undefined;
        } });
    }

    get button(): HTMLButtonElement { return this.capture.button; }
    get capturing(): boolean { return this.capture.capturing; }

    leave(): void { this.capture.leave(false); }

    refreshFocus(): void {
        if (this.capturing) this.enqueue({ kind: 'focus' });
    }

    follow(scope: number | undefined, name: string, enabled: boolean): void {
        const changed = this.scope !== scope;
        this.scope = scope;
        this.capture.update(name, scope !== undefined, enabled);
        if (changed) this.capture.leave(true);
    }

    receive(result: RuntimeKeyboardResult): void {
        if (result.request !== this.pending || result.scope !== this.scope) return;
        this.pending = undefined;
        if ('error' in result) {
            this.capture.leave(true);
            this.report(result.error);
            return;
        }
        this.capture.focused(result.focus);
        this.dispatch();
    }

    private enqueue(input: KeyboardInput): void {
        this.queued.push(input);
        this.dispatch();
    }

    private dispatch(): void {
        if (this.scope === undefined || this.pending !== undefined) return;
        const input = this.queued.shift();
        if (!input) return;
        this.pending = ++this.request;
        this.send({ type: 'keyboard', scope: this.scope, request: this.request, input });
    }
}
