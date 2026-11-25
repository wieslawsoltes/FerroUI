import { strict as assert } from 'node:assert';
import { StageFrames } from '../src/stageFrames';
import type { StageImage, StageTarget } from '../src/stagesProtocol';
import { until } from './wait';

interface Capture {
    target: StageTarget;
    resolve: (image: StageImage) => void;
    reject: (error: Error) => void;
}

export async function verifyStageFrames(): Promise<void> {
    const captures: Capture[] = [];
    const displayed: StageTarget[] = [];
    const errors: unknown[] = [];
    const frames = new StageFrames(target => new Promise((resolve, reject) => captures.push({ target, resolve, reject })),
        target => displayed.push(target), error => errors.push(error));
    const image = { png: 'frame', width: 80, height: 60 };
    const card = (id: number, epoch: number, rendered_revision: number | null = 1) => ({ id, epoch, rendered_revision });
    const [desktop, compact, interactive] = [card(1, 1), card(2, 1), card(3, 1)];
    const updated = card(desktop.id, 2);
    let index = 0;
    const next = async (expected: StageTarget) => {
        await until(() => captures.length > index);
        const capture = captures[index++];
        assert.deepEqual(capture.target, { id: expected.id, epoch: expected.epoch });
        return capture;
    };
    try {
        frames.setStages([desktop, compact, card(interactive.id, 1, null)]);
        const retired = await next(desktop);
        frames.setStages([updated, compact, interactive]);
        frames.request(interactive);
        frames.request(interactive);
        frames.request(desktop);
        retired.resolve(image);

        const firstInput = await next(interactive);
        assert.equal(displayed.length, 0);
        frames.request(interactive);
        frames.request(interactive);
        firstInput.resolve(image);
        (await next(interactive)).resolve(image);

        const sibling = await next(compact);
        frames.setStages([updated, compact, interactive]);
        sibling.resolve(image);
        const invalidated = await next(updated);
        assert.deepEqual(displayed.map(target => target.id), [interactive.id, interactive.id, compact.id]);

        // A restored webview can request the same epochs. Retired replies still belong to the old view.
        frames.clear();
        frames.setStages([updated]);
        invalidated.reject(new Error('Previous webview'));
        (await next(updated)).resolve(image);
        await until(() => displayed.length === 4);
        assert.deepEqual(errors, []);
        console.log('Stage frames coalesced input, preserved an accepted sibling and ignored retired renders');
    } finally {
        frames.clear();
    }
}
