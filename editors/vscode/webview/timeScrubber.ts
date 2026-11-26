export const defaultAnimationFrameRate = 30;

export class TimeScrubber {
    private frameRate = defaultAnimationFrameRate;
    private start?: number;
    private end = 0;

    constructor(readonly input: HTMLInputElement, seek: (seconds: number) => void) {
        input.addEventListener('input', () => seek(this.seconds));
        input.addEventListener('keydown', event => {
            if (event.altKey || event.ctrlKey || event.metaKey || input.disabled) return;
            const forward = event.key === 'ArrowRight' || event.key === 'ArrowUp';
            const backward = event.key === 'ArrowLeft' || event.key === 'ArrowDown';
            if (!forward && !backward) return;
            event.preventDefault();
            const previous = input.valueAsNumber;
            input.valueAsNumber = previous + (forward ? 1 : -1);
            if (input.valueAsNumber !== previous) seek(this.seconds);
        });
    }

    get seconds(): number { return this.input.valueAsNumber / this.frameRate; }
    get maximum(): number { return this.end; }

    show(seconds: number, end: number, start = 0, frameRate = defaultAnimationFrameRate): void {
        // Blink's native accessibility action may step by one input unit. Frame
        // units preserve that increment while step="any" keeps pointer input continuous.
        if (start !== this.start || end !== this.end || frameRate !== this.frameRate) {
            this.start = start;
            this.end = end;
            this.frameRate = frameRate;
            this.input.min = String(start * frameRate);
            this.input.max = String(end * frameRate);
        }
        this.input.value = String(seconds * frameRate);
    }
}
