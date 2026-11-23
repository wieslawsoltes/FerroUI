import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import { until } from './wait';

export async function verifyHostedRustBehaviors(preview: NativePreview, request: HostedSource): Promise<void> {
    const markup = (width: number, behavior: boolean) => `use ferroui_view::behavior::{event, change_property};
fn approval() {
    ferroui_view::rsx! { Window { width: 360.0, height: 190.0,
        StackPanel { spacing: 12.0,
            ${behavior ? 'behaviors: [event("Click", Some("Approve"), [change_property(Some("Card"), "Width", "210")])],' : ''}
            Button { name: "Approve", width: 360.0, height: 44.0, "Approve" }
            Border { name: "Card", width: ${width}.0, height: 96.0, background: "#254679",
                horizontal_alignment: ferroui_base::layout::HorizontalAlignment::Left }
        }
    } }
}`;
    const render = (revision: number, width: number, behavior: boolean) => preview.renderHosted({
        ...request, revision, document: { ...request.document, source: markup(width, behavior), caret: { line: 2, character: 38 } },
    });
    const click = (frame: HostedFrame) => preview.hostedInput({
        type: 'click', x: .5, y: .11, revision: frame.revision, sequence: frame.sequence,
    });
    const reference = await render(request.revision, 210, false);
    await click(reference);
    let expected = reference;
    let settledAt = Date.now();
    await until(async () => {
        const frame = await preview.hostedFrame();
        if (frame?.revision === reference.revision && frame.sequence > expected.sequence) {
            expected = frame;
            settledAt = Date.now();
        }
        return expected.sequence > reference.sequence && Date.now() - settledAt >= 300;
    });
    const before = await render(request.revision + 1, 96, true);
    assert.notEqual(before.image.png, expected.image.png);
    await click(before);
    await until(async () => {
        const candidate = await preview.hostedFrame();
        return candidate?.revision === before.revision && candidate.sequence > before.sequence
            && candidate.image.png === expected.image.png;
    });
    console.log('App Preview resolved a mapped Rust behavior’s named source and target and matched the native action frame');
}
