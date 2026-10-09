import {InputEventMessageBase} from "./InputEventMessageBase";

export abstract class PointerEventMessageBase extends InputEventMessageBase {
    public readonly x: number;
    public readonly y: number;

    protected constructor(e: MouseEvent, scale: number) {
        super(e);
        this.x = e.clientX / scale;
        this.y = e.clientY / scale;
    }
}
