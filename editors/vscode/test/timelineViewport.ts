import { strict as assert } from 'node:assert';
import { retainedCurveTrack } from '../webview/hostedCurves';
import { CurveScale } from '../webview/timelineCurveScale';
import { cubicEasing, easingPlot } from '../webview/easingCurve';
import { stepPreviewRange, validPreviewRange } from '../src/timelinePreviewRange';
import { fitKeyTimes, fitTimeline, minimumTimelineSpan, panTimeline, stepTimelineFrame, timelinePercent, visibleTime, zoomTimeline } from '../webview/timelineViewport';

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
    assert.equal(stepTimelineFrame(7 / 30, 1, 30, 2), 8 / 30);
    assert.equal(stepTimelineFrame(.21, -1, 30, 2), 6 / 30);
    assert.equal(stepTimelineFrame(.21, 1, 30, 2), 7 / 30);
    assert.equal(stepTimelineFrame(.21, 10, 24, .24), .24);
    assert.equal(stepTimelineFrame(.01, -10, 24, 2), 0);
    const scale = new CurveScale([{ time: 1, value: .9 }], [0, .8]);
    assert.ok(scale.percent(.9) < scale.percent(.8));
    assert.ok(scale.fromOffset(.8, 25) < .8);
    assert.ok(Number.isFinite(new CurveScale([], [-1e300, 1e300]).percent(0)));
    const close = new CurveScale([], [1e12, 1e12 + 1]);
    assert.ok(close.fromOffset(1e12, -42) > 1e12 && close.fromOffset(1e12, -42) < 1e12 + 1);
    const points = cubicEasing('0.25,-0.2,0.75,1.6');
    assert.deepEqual(points, [.25, -.2, .75, 1.6]);
    assert.equal(cubicEasing('0.25,0.25,1.1,1'), undefined);
    assert.equal(cubicEasing('0,,1,1'), undefined);
    const plot = easingPlot(points, []);
    assert.ok(Math.abs(plot.value(plot.y(1.6)) - 1.6) < 1e-12);
    assert.ok(Math.abs(plot.value(plot.y(-.2)) + .2) < 1e-12);
    const range = { start: .25, end: .75, loop: true };
    assert.equal(validPreviewRange(range, 1), true);
    assert.equal(validPreviewRange({ ...range, end: .25 }, 1), false);
    assert.deepEqual(stepPreviewRange(range, .625, 1.25), { time: .375, completed: false });
    assert.deepEqual(stepPreviewRange({ ...range, loop: false }, .625, 1.25), { time: .75, completed: true });
    verifyHostedCurveTrack();
    console.log('Timeline viewport kept the zoom pivot, aligned authored key positions, and clamped both pan edges');
}

function verifyHostedCurveTrack(): void {
    const choice = (property: string, id: number, label: string) => ({ track: { property, target: id, keyframes: [] },
        target: { id, label, typeName: 'Border' }, editable: true });
    const previous = choice('Width', 8, 'Forecast');
    const restored = choice('Width', 10, 'Forecast');
    const renamed = choice('Width', 10, 'Estimate');
    const opacity = choice('Opacity', 10, 'Forecast');
    assert.equal(retainedCurveTrack([opacity, renamed], previous, false), 1, 'Retain the property when only one target is available');
    assert.equal(retainedCurveTrack([choice('Width', 8, 'Other'), opacity, restored], previous, false), 2,
        'Find the matching label after the native source IDs have changed');
    assert.equal(retainedCurveTrack([restored, choice('Width', 8, 'Forecast')], previous, false), -1,
        'A reused source ID cannot resolve two identical labels across revisions');
    assert.equal(retainedCurveTrack([restored, previous], previous, true), 1, 'Preserve precise selection within an accepted revision');
    assert.equal(retainedCurveTrack([opacity], previous, false), -1, 'Fall back when the authored track disappears');
}
