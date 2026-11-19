import type { FrameTarget, FromDesigner, ResizeGesture, ToDesigner } from '../src/protocol';

type Request = Extract<FromDesigner, { type: 'previewResize' }>;
type Result = Extract<ToDesigner, { type: 'resizeHint' }>;
type Receive = (preview: NonNullable<Result['preview']>) => void;
interface Hint { request: Request; show: Receive }

/** Keeps one native resize query in flight and the newest pointer sample ready to send. */
export class ResizeSnapHints {
    private sequence = 0;
    private pending?: Hint;
    private inFlight?: Hint;

    constructor(private readonly send: (message: FromDesigner) => void) {}

    request(frame: FrameTarget, gesture: ResizeGesture, show: Receive): void {
        this.pending = { show, request: { type: 'previewResize', request: ++this.sequence, frame, ...gesture } };
        this.flush();
    }

    receive(result: Result): void {
        const active = this.inFlight;
        if (!active || active.request.request !== result.request) return;
        this.inFlight = undefined;
        if (!this.pending && result.preview) active.show(result.preview);
        this.flush();
    }

    clear(): void { this.pending = this.inFlight = undefined; }

    private flush(): void {
        if (this.inFlight || !this.pending) return;
        this.inFlight = this.pending;
        this.pending = undefined;
        this.send(this.inFlight.request);
    }
}
