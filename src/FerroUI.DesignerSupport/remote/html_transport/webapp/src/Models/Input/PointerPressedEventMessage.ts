import {PointerEventMessageBase} from "./PointerEventMessageBase";
import {MouseButton} from "./MouseButton";
import {getMouseButton} from "./MouseEventHelpers";

export class PointerPressedEventMessage extends PointerEventMessageBase {
    public readonly button: MouseButton

    constructor(e: MouseEvent, scale: number) {
        super(e, scale);
        this.button = getMouseButton(e);
    }

    public toString = () : string => {
        return `pointer-pressed:${this.modifiers}:${this.x}:${this.y}:${this.button}`;
    }
}
