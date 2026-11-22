import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import { until } from './wait';

export async function verifyHostedStateTrigger(preview: NativePreview, request: HostedSource): Promise<void> {
    const markup = (width: number, trigger: boolean) => `fn approval() {
        ferroui_view::rsx! { Window { width: 360.0, height: 190.0,
            StackPanel { spacing: 12.0,
                visual_states: [ferroui_view::visual_state_group("Review")
                    .state(ferroui_view::visual_state("Ready")
                        .set_on("Card", ferroui_base::layout::Layoutable::width_property(), 210.0))],
                Button { name: "Approve", width: 360.0, height: 44.0,
                    ${trigger ? 'state_trigger("Click", "Review", "Ready"),' : ''}
                    "Approve"
                }
                Border { name: "Card", width: ${width}.0, height: 96.0, background: "#254679",
                    horizontal_alignment: ferroui_base::layout::HorizontalAlignment::Left }
            }
        } }
    }`;
    const render = (revision: number, width: number, trigger: boolean) => preview.renderHosted({
        ...request, revision, document: { ...request.document, source: markup(width, trigger), caret: { line: 1, character: 28 } },
    });
    const click = async (frame: HostedFrame) => {
        await preview.hostedInput({ type: 'click', x: .5, y: .11, revision: frame.revision, sequence: frame.sequence });
    };
    const reference = await render(request.revision, 210, false);
    await click(reference);
    let expected = reference;
    let lastFrame = Date.now();
    await until(async () => {
        const frame = await preview.hostedFrame();
        if (frame?.revision === reference.revision && frame.sequence > expected.sequence) {
            expected = frame;
            lastFrame = Date.now();
        }
        // Wait for the button's pointer and focus visuals before comparing the card.
        return expected.sequence > reference.sequence && Date.now() - lastFrame >= 300;
    });
    const before = await render(request.revision + 1, 96, true);
    assert.ok(before.image.png !== expected.image.png, 'The authored base differs from the Ready state');
    await click(before);
    await until(async () => {
        const candidate = await preview.hostedFrame();
        return candidate?.revision === before.revision && candidate.sequence > before.sequence
            && candidate.image.png === expected.image.png;
    });
    console.log('App Preview dispatched a mapped Rust state trigger and matched the native destination frame');
}
