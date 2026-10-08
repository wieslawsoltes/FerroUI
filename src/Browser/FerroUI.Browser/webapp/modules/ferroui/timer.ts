import { FerroExports } from "./ferroExports";

export class TimerHelper {
    public static runAnimationFrames(): void {
        function render(time: number) {
            FerroExports.TimerHelper?.JsExportOnAnimationFrame(time);
            self.requestAnimationFrame(render);
        }
        self.requestAnimationFrame(render);
    }

    // Not in the original. The clock of the timestamps of the animation frames of this thread
    // (each worker has a time origin of its own), for a frame that is rendered out of turn.
    public static now(): number {
        return performance.now();
    }
}
