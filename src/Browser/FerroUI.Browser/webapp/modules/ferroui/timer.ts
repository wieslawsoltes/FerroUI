import { FerroExports } from "./ferroExports";

export class TimerHelper {
    public static runAnimationFrames(): void {
        function render(time: number) {
            FerroExports.TimerHelper?.JsExportOnAnimationFrame(time);
            self.requestAnimationFrame(render);
        }
        self.requestAnimationFrame(render);
    }
}
