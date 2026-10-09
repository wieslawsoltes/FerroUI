// The tests of the input messages of the page, on the test runner and the assertions that come
// with Node: scripts/build-designer-webapp.sh bundles and runs them. An event is an object with
// the members a test sets up.
import { describe, it } from "node:test";
import * as assert from "node:assert/strict";
import { InputModifiers } from "../../src/Models/Input/InputModifiers";
import { MouseButton } from "../../src/Models/Input/MouseButton";
import { PointerMovedEventMessage } from "../../src/Models/Input/PointerMovedEventMessage";
import { PointerPressedEventMessage } from "../../src/Models/Input/PointerPressedEventMessage";
import { PointerReleasedEventMessage } from "../../src/Models/Input/PointerReleasedEventMessage";
import { ScrollEventMessage } from "../../src/Models/Input/ScrollEventMessage";
import { getModifiers, getMouseButton } from "../../src/Models/Input/MouseEventHelpers";

describe("Input event tests", () => {
    describe("Helpers", () => {
        it("getModifiers", () => {
            const event = {
                altKey: false,
                ctrlKey: true,
                shiftKey: false,
                metaKey: false,
                buttons: 1
            } as MouseEvent
            var actual = getModifiers(event)

            assert.deepEqual(actual, [InputModifiers.Control, InputModifiers.LeftMouseButton])
        })
        it("getMouseButton", () => {
            const event = {
                button: 1
            } as MouseEvent
            var actual = getMouseButton(event)

            assert.equal(actual, MouseButton.Middle)
        })
    })

    describe("Messages", () => {
        const x = .3
        const y = .42
        const modifiers = "0,1,2,3,4,5,6"

        const button = "1"

        const deltaX = -3.
        const deltaY = -3.

        const mouseEvent = {
            altKey: true,
            ctrlKey: true,
            shiftKey: true,
            metaKey: true,
            buttons: 7,
            button: 0,
            clientX: x,
            clientY: y
        } as MouseEvent

        it("PointerMovedEventMessage", () => {
            const message = new PointerMovedEventMessage(mouseEvent, 1)
            assert.equal(message.toString(), `pointer-moved:${modifiers}:${x}:${y}`)
        })
        it("PointerPressedEventMessage", () => {
            const message = new PointerPressedEventMessage(mouseEvent, 1)
            assert.equal(message.toString(), `pointer-pressed:${modifiers}:${x}:${y}:${button}`)
        })
        it("PointerReleasedEventMessage", () => {
            const message = new PointerReleasedEventMessage(mouseEvent, 1)
            assert.equal(message.toString(), `pointer-released:${modifiers}:${x}:${y}:${button}`)
        })

        it("ScrollEventMessage", () => {
            const wheelEvent = {
                altKey: true,
                ctrlKey: true,
                shiftKey: true,
                metaKey: true,
                buttons: 7,
                clientX: x,
                clientY: y,
                deltaX: -deltaX,
                deltaY: -deltaY
            } as WheelEvent
            const message = new ScrollEventMessage(wheelEvent, 1)

            assert.equal(message.toString(), `scroll:${modifiers}:${x}:${y}:${deltaX}:${deltaY}`)
        })
    })
})
