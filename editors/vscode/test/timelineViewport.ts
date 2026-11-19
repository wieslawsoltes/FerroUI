import { strict as assert } from 'node:assert';
import { CurveScale } from '../webview/timelineCurveScale';
import { fitKeyTimes, fitTimeline, minimumTimelineSpan, panTimeline, timelinePercent, visibleTime, zoomTimeline } from '../webview/timelineViewport';

export function verifyTimelineViewport(): void {
    const full = fitTimeline(8);
    const zoomed = zoomTimeline(full, 4, 4);
    assert.deepEqual(zoomed, { duration: 8, start: 3, span: 2 });
    assert.equal(timelinePercent(zoomed, 4), timelinePercent(full, 4));
    assert.equal(timelinePercent(zoomed, 4.5), 75);

    const right = panTimeline(zoomed, 10);
    assert.deepEqual(right, { duration: 8, start: 6, span: 2 });
    assert.equal(visibleTime(right, 8), true);
    assert.equal(visibleTime(right, 5.99), false);
    assert.equal(panTimeline(zoomed, -1).start, 0);
    assert.equal(zoomTimeline(full, 500, 0).span, minimumTimelineSpan);
    assert.deepEqual(zoomTimeline(zoomed, .1, 4), full);
    assert.deepEqual(zoomTimeline(fitTimeline(0), 4, 0), fitTimeline(0));
    const selection = fitKeyTimes(8, [6.5, 7]);
    assert.ok(selection.span < 1 && visibleTime(selection, 6.5) && visibleTime(selection, 7));
    assert.equal(fitKeyTimes(8, [0]).start, 0);
    assert.equal(fitKeyTimes(8, [8]).start, 8 - fitKeyTimes(8, [8]).span);
    const scale = new CurveScale([{ time: 1, value: .9 }], [0, .8]);
    assert.ok(scale.percent(.9) < scale.percent(.8));
    assert.ok(scale.fromOffset(.8, 25) < .8);
    assert.ok(Number.isFinite(new CurveScale([], [-1e300, 1e300]).percent(0)));
    const close = new CurveScale([], [1e12, 1e12 + 1]);
    assert.ok(close.fromOffset(1e12, -42) > 1e12 && close.fromOffset(1e12, -42) < 1e12 + 1);
    console.log('Timeline viewport kept the zoom pivot, aligned authored key positions, and clamped both pan edges');
}
