import { strict as assert } from 'node:assert';
import { fitTimeline, minimumTimelineSpan, panTimeline, timelinePercent, visibleTime, zoomTimeline } from '../webview/timelineViewport';

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
    console.log('Timeline viewport kept the zoom pivot, aligned authored key positions, and clamped both pan edges');
}
