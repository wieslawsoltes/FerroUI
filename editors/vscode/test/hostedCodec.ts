import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';

const originalShadow = '8 14 24 2 #60244B90';
const revisedShadow = '12 18 30 3 #88244B90';
const authored = [
    '<Window xmlns="https://github.com/ferroui" Width="380" Height="220" Background="#EDF2FA">',
    '  <!-- 🎨 Keep the designer note outside the clip -->',
    '  <Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>',
    '    <Animation Duration="0:0:2" FillMode="Both"><!--todo-workbench-review-shadow-->',
    '      <KeyFrame Cue="0%"><Setter Property="BoxShadow" Value="0 0 0 0 #00000000"/></KeyFrame>',
    `      <KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="${originalShadow}"/></KeyFrame>`,
    '    </Animation>',
    '  </Style.Animations></Style></Window.Styles>',
    '  <Border Name="ReviewCard" Width="180" Height="100" Background="#FFFFFF"/>',
    '</Window>',
].join('\r\n');

function shadow(frame: HostedFrame) {
    const program = frame.animations?.programs.find(clip => clip.authoring?.codec === 'todo-workbench.review-shadow');
    assert.ok(program?.authoring, 'The application announced its registered shadow codec');
    const track = program.authoring.tracks.find(track => track.property === 'BoxShadow');
    assert.ok(track);
    return { program, track };
}

export async function verifyHostedCodec(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const initial = await preview.renderHosted(update(14, authored));
    const { program, track } = shadow(initial);
    const key = track.keyframes.find(key => key.time === 2);
    assert.equal(program.sourceOffset, authored.indexOf('<Animation'));
    assert.equal(key?.value_kind, 'Discrete');
    assert.equal(key?.value_text, originalShadow);
    const draft = { program: program.id, target: track.target, property: track.property,
        previous: 2, time: 2, value: revisedShadow, easing: '0.25,0.1,0.25,1' };
    await assert.rejects(preview.planHostedKeyframe(initial, { ...draft, value: 'bad shadow' }), /valid BoxShadow literal/i);
    const finished = await preview.sampleHostedAnimation({ revision: initial.revision, sequence: initial.sequence,
        program: program.id, elapsed: 2, restart: true });
    assert.notEqual(initial.image.png, finished.frame.image.png);
    await assert.rejects(preview.planHostedKeyframe(initial, draft), /frame changed/i);
    const plan = await preview.planHostedKeyframe(finished.frame, draft);
    assert.equal(plan.valueKind, 'Discrete');
    assert.ok(plan.samples.length >= 16);
    assert.ok(plan.patch);
    const { start, end, expected, replacement } = plan.patch;
    assert.equal(authored.slice(start, end), expected);
    assert.ok(start >= authored.indexOf('<Animation') && end <= authored.indexOf('</Animation>') + '</Animation>'.length);
    const changed = authored.slice(0, start) + replacement + authored.slice(end);
    assert.match(changed, /<!--todo-workbench-review-shadow-->/);
    assert.match(changed, /🎨 Keep the designer note outside the clip/);
    assert.ok(!changed.replaceAll('\r\n', '').includes('\n'), 'The candidate retains the document’s CRLF line endings');
    assert.ok(changed.includes(revisedShadow));
    const revised = await preview.renderHosted(update(15, changed));
    const accepted = shadow(revised);
    assert.equal(accepted.track.keyframes.find(key => key.time === 2)?.value_text, revisedShadow);
    assert.equal(accepted.track.keyframes.find(key => key.time === 2)?.easing_text, draft.easing);
    const result = await preview.sampleHostedAnimation({ revision: revised.revision, sequence: revised.sequence,
        program: accepted.program.id, elapsed: 2, restart: true });
    assert.notEqual(result.frame.image.png, finished.frame.image.png, 'The applied key changes the native shadow frame');
    await assert.rejects(preview.planHostedKeyframe(finished.frame, draft), /frame changed/i);
    console.log('App-hosted project codec rejected invalid and stale drafts and round-tripped a native BoxShadow key with UTF-16 offsets and CRLF');
}
