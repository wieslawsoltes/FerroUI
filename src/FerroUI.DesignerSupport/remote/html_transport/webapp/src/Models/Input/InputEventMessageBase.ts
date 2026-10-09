import {InputModifiers} from "./InputModifiers";
import {getModifiers} from "./MouseEventHelpers";

export abstract class InputEventMessageBase {
    public readonly modifiers : Array<InputModifiers>;

    protected constructor(e: MouseEvent) {
        this.modifiers = getModifiers(e);
    }
}
