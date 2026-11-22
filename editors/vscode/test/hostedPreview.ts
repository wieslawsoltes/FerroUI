import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import type { NativePreview } from '../src/nativePreview';
import type { HostedSource } from '../src/hostedProtocol';
import { verifyHostedAnimation, verifyHostedScalarCurve } from './hostedAnimation';
import { verifyHostedCodec } from './hostedCodec';
import { verifyHostedClips } from './hostedClips';
import { verifyHostedClipAuthoring } from './hostedClipAuthoring';
import { verifyHostedColorCurves } from './hostedColorCurves';
import { verifyHostedRelativeCurves } from './hostedRelativeCurves';
import { verifyHostedEffectCurves } from './hostedEffectCurves';
import { verifyHostedEffectObjects } from './hostedEffectObjects';
import { verifyHostedGradients } from './hostedGradients';
import { verifyHostedBoxShadows } from './hostedBoxShadows';
import { verifyHostedRegisteredEffects } from './hostedRegisteredEffects';
import { verifyHostedKeyframeGroups } from './hostedKeyframeGroups';
import { verifyHostedContinuity } from './hostedContinuity';
import { verifyHostedRustAnimation } from './hostedRustAnimation';
import { verifyHostedStateRoutes } from './hostedStateRoutes';
import { verifyHostedStateAuthoring } from './hostedStateAuthoring';
import { verifyHostedStateValues } from './hostedStateValues';
import { verifyHostedStateTrigger } from './hostedStateTriggers';
import { verifyHostedTransformCurves } from './hostedTransformCurves';
import { verifyHostedTransformList } from './hostedTransformList';
import { verifyHostedWorkspaceSources } from './hostedWorkspaceSources';
import { verifyHostedStyles } from './hostedStyles';
import { verifyHostedRustStyles } from './hostedRustStyles';
import { verifyHostedSharedColors } from './hostedSharedColors';
import { verifyHostedModuleColors } from './hostedModuleColors';
import { verifyHostedRustTemplates } from './hostedRustTemplates';
import { verifyHostedTreeTemplates } from './hostedTreeTemplates';
import { reviewStateSource } from './rsxStatesMcp';

export async function verifyHostedPreview(preview: NativePreview, uri: string): Promise<void> {
    const app = process.env.FERROUI_TEST_HOSTED_APP;
    if (!app) return;
    const nonce = randomBytes(32).toString('hex');
    const { port, animationQuery } = await preview.listenHosted(nonce);
    const endpoint = new URL(`tcp-bson://127.0.0.1:${port}/`);
    if (animationQuery) endpoint.search = animationQuery;
    const child = spawn(app, ['--ferroui-preview', '--transport', endpoint.toString(),
        '--session-id', nonce, '--method', 'ferroui-remote', app], { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true,
        env: { ...process.env, FERROUI_EDITOR_SOURCE: '1' },
    });
    let log = '';
    child.stdout.on('data', chunk => { log = (log + chunk).slice(-10000); });
    child.stderr.on('data', chunk => { log = (log + chunk).slice(-10000); });
    let launchError: Error | undefined;
    child.on('error', error => { launchError = error; });
    const update = (revision: number, source: string): HostedSource => ({ revision, assembly: 'todo-workbench', projectPath: '/Test.xaml',
        document: { uri, source, language: 'xaml', caret: { line: 0, character: 0 } } });
    const valid = (value: string) =>
        `<Window xmlns="https://github.com/ferroui" xmlns:app="using:TodoWorkbench.Design" Width="380" Height="220">
          <StackPanel Margin="20">
            <TextBlock Name="Message" IsVisible="False" Text="${value}"/>
            <app:PreviewBadge Text="{Binding #Message.Text}"/>
          </StackPanel>
        </Window>`;
    try {
        const first = await preview.renderHosted(update(1, valid('First version')));
        assert.equal(first.revision, 1);
        assert.equal(Buffer.from(first.image.png, 'base64').subarray(1, 4).toString(), 'PNG');
        await assert.rejects(preview.renderHosted(update(2, valid('Broken').replace('PreviewBadge', 'NonexistentControl'))));
        const third = await preview.renderHosted(update(3, valid('Updated in the editor')));
        assert.equal(third.revision, 3);
        assert.notEqual(third.image.png, first.image.png);
        await assert.rejects(preview.hostedInput({ type: 'click', x: .5, y: .5, revision: first.revision, sequence: first.sequence }), /frame changed/i);
        await preview.hostedInput({ type: 'click', x: .5, y: .5, revision: third.revision, sequence: third.sequence });
        const bound = '<Window xmlns="https://github.com/ferroui" Width="380" Height="220"><TextBlock Text="{ReflectionBinding .}"/></Window>';
        const hostedXaml = (revision: number, fixture?: string): HostedSource => ({
            ...update(revision, bound), projectPath: '/HostedPreview.xaml', fixture,
        });
        const defaults = await preview.renderHosted(hostedXaml(4));
        assert.deepEqual(defaults.fixtures, ['Loading', 'Ready']);
        const loading = await preview.renderHosted(hostedXaml(5, 'Loading'));
        const ready = await preview.renderHosted(hostedXaml(6, 'Ready'));
        assert.equal(loading.fixture, 'Loading');
        assert.notEqual(ready.image.png, loading.image.png);
        await assert.rejects(preview.renderHosted(hostedXaml(7, 'Missing')), /unavailable/);
        const hostedRsx = (revision: number, fixture?: string): HostedSource => ({
            revision, projectPath: '/app_preview.rs', fixture, assembly: 'todo-workbench',
            document: { uri: new URL('app_preview.rs', uri).toString(), language: 'rust', caret: { line: 0, character: 35 },
                source: 'fn preview() { rsx! { TextBlock { text: "Unsaved Rust view" } } }' },
        });
        const rsx = await preview.renderHosted(hostedRsx(8));
        assert.deepEqual(rsx.fixtures, ['Empty queue', 'One task']);
        const one = await preview.renderHosted(hostedRsx(9, 'One task'));
        assert.equal(one.fixture, 'One task');
        assert.notEqual(one.image.png, rsx.image.png);
        assert.notEqual((await preview.renderHosted(hostedRsx(10, 'Empty queue'))).image.png, one.image.png);
        await verifyHostedAnimation(preview, update);
        await verifyHostedCodec(preview, update);
        await verifyHostedContinuity(preview, update);
        await verifyHostedScalarCurve(preview, update);
        await verifyHostedRustAnimation(preview);
        const reviewSource: HostedSource = {
            ...hostedRsx(150), projectPath: '/app_preview/review_states.rs',
            document: { uri: new URL('review_states.rs', uri).toString(), language: 'rust',
                source: reviewStateSource, caret: { line: 3, character: 2 } },
        };
        const projection = await preview.renderHosted(reviewSource);
        assert.ok(projection.fixtures?.includes('Interactive review'));
        const review = await preview.renderHosted({ ...reviewSource, revision: 151, fixture: 'Interactive review' });
        assert.equal(review.fixture, 'Interactive review');
        assert.notEqual(review.image.png, one.image.png);
        assert.deepEqual(review.animations?.programs ?? [], []);
        await verifyHostedStateRoutes(preview, reviewSource);
        await verifyHostedClips(preview, update);
        await verifyHostedKeyframeGroups(preview, update);
        await verifyHostedClipAuthoring(preview, update);
        await verifyHostedStateAuthoring(preview, update);
        await verifyHostedColorCurves(preview, update);
        await verifyHostedRelativeCurves(preview, update);
        await verifyHostedWorkspaceSources(preview, update);
        await verifyHostedStyles(preview, update);
        await verifyHostedRustStyles(preview, uri);
        await verifyHostedSharedColors(preview, uri);
        await verifyHostedModuleColors(preview);
        await verifyHostedRustTemplates(preview);
        await verifyHostedTreeTemplates(preview);
        await verifyHostedEffectCurves(preview, update);
        await verifyHostedEffectObjects(preview, update);
        await verifyHostedRegisteredEffects(preview, update);
        await verifyHostedStateValues(preview, update);
        await verifyHostedStateTrigger(preview, { ...hostedRsx(4110), projectPath: '/ApprovalStateTrigger.rs' });
        await verifyHostedTransformCurves(preview, update);
        await verifyHostedBoxShadows(preview, update);
        await verifyHostedTransformList(preview, update);
        await verifyHostedGradients(preview, update);
        assert.equal(child.exitCode, null);
        console.log('App-hosted design rendered a custom control, native bindings and registered XAML/Rust fixtures, and fenced invalid updates and stale input');
    } catch (error) {
        throw new Error(`App-hosted preview failed. ${String(error)}\nHost status: ${child.exitCode ?? child.signalCode ?? 'running'}\n${launchError?.message ?? log}`, { cause: error });
    } finally { child.kill(); }
}
