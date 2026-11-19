import { strict as assert } from 'node:assert';
import { randomBytes } from 'node:crypto';
import { access } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { NativePreview } from '../src/nativePreview';
import { sourceOf } from '../src/textDocument';
import type { CurvePoint, TimelineProgram } from '../src/timelineProtocol';
import { isInside } from '../src/workspacePaths';
import { verifyHostedPreview } from './hostedPreview';
import { reviewStateSource } from './rsxStatesMcp';
import { until } from './wait';

export async function run(): Promise<void> {
    const extension = vscode.extensions.getExtension('FerroUI.ferroui');
    assert.ok(extension);
    assert.ok(isInside(process.env.FERROUI_TEST_INSTALLED_ROOT!, extension.extensionPath));
    const suffix = process.platform === 'win32' ? '.exe' : '';
    assert.equal(vscode.workspace.getConfiguration('ferroui.languageServer').get('path'), '');
    assert.equal(vscode.workspace.getConfiguration('ferroui.preview').get('path'), '');
    await Promise.all([
        ...['ferroui-xaml-lsp', 'ferroui-designer-preview'].map(name => access(path.join(extension.extensionPath, 'bin', name + suffix))),
        ...['timeline.js', 'designData.js', 'hosted.js'].map(name => access(path.join(extension.extensionPath, 'dist', name))),
        access(path.join(extension.extensionPath, 'media', 'keyframe-editor.css')),
    ]);
    const workspace = vscode.workspace.workspaceFolders![0];
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace.uri, 'SettingsView.xaml'));
    assert.equal(document.languageId, 'ferroui-xaml');
    await extension.activate();
    await vscode.window.showTextDocument(document);
    await vscode.commands.executeCommand('ferroui.openDesigner');
    await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label === 'Design · SettingsView.xaml')));
    await until(async () => {
        const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>('vscode.executeDocumentSymbolProvider', document.uri);
        return symbols?.some(symbol => symbol.name === 'UserControl');
    });
    const output = vscode.window.createOutputChannel('Installed FerroUI package audit');
    const native = await NativePreview.start(extension, output);
    try {
        assert.ok(native.capabilities.canvasInteraction);
        assert.ok(native.capabilities.designFixtures, 'The installed preview recognizes named design fixtures');
        assert.ok(native.capabilities.timelineGeometryRecording, 'The installed preview supports canvas animation recording');
        assert.ok(native.capabilities.timelineKeyframeDrafts, 'The installed preview validates keyframe drafts and samples easing');
        assert.ok(native.capabilities.timelineCurves, 'The installed preview samples scalar animation curves');
        assert.ok(native.capabilities.timelineCombinedPlayback, 'The installed preview plays combined animation programs');
        assert.ok(native.capabilities.timelineProgramTargets, 'The installed preview retains animation resource targets');
        assert.ok(native.capabilities.stateTransitionPreview, 'The installed preview samples explicit state transitions');
        assert.ok(native.capabilities.rsxStateAuthoring, 'The installed preview maps typed Rust state declarations');
        assert.ok(native.capabilities.hostedAnimationRestoration, 'The installed preview restores the accepted app-hosted timeline');
        const source = sourceOf(document, 'xaml', new vscode.Position(0, 0));
        const snapshot = await native.render(source, { width: 800, height: 520, dark: false }, 1);
        assert.ok(snapshot.image?.png, snapshot.renderError ?? undefined);
        assert.equal(Buffer.from(snapshot.image.png, 'base64').subarray(1, 4).toString(), 'PNG');
        const project = await native.projectInventory(workspace.uri.fsPath);
        assert.ok(project.files.some(file => file.kind === 'rsx' && file.path === 'views.rs'));
        const rust = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace.uri, 'animation_view.rs'));
        const declaration = rust.getText().indexOf('ferroui_view::animation(');
        assert.ok(declaration >= 0);
        const animated = await native.render(sourceOf(rust, 'rust', rust.positionAt(declaration)), { width: 640, height: 420, dark: false }, 2);
        assert.ok(animated.image, animated.renderError ?? undefined);
        assert.ok(animated.mappedRustAnimations, 'The installed native preview maps an RSX animation declaration');
        const { programs } = await native.studioQuery(2, 'designer_programs', { include_source: false }) as { programs: TimelineProgram[] };
        assert.ok(programs[0]?.sourceRange && programs[0].timeline && programs[0].preview?.available);
        const [program] = programs;
        const track = program.timeline!.tracks[0];
        const curve = await native.studioQuery(2, 'designer_animation_curve', { program: program.id,
            target: track.target, property: track.property, window: { start: 0, end: 2 } }) as { points: CurvePoint[] };
        assert.equal(curve.points.find(point => point.time === .5)?.value, .65);
        const stateSource = { uri: vscode.Uri.joinPath(workspace.uri, 'review_states.rs').toString(),
            language: 'rust' as const, source: reviewStateSource, caret: { line: 3, character: 2 } };
        const states = await native.render(stateSource, { width: 640, height: 420, dark: false }, 3);
        assert.ok(states.image, states.renderError ?? undefined);
        const { groups: [group] } = await native.studioQuery(3, 'designer_states', {}) as {
            groups: { name: string; transitions: Array<{ from: string | null; to: string; duration: number }> }[];
        };
        assert.equal(group.name, 'Review');
        const forward = group.transitions.find(route => route.from === 'Idle' && route.to === 'Ready');
        assert.ok(forward);
        assert.equal(forward.duration, .64);
        forward.duration = .8;
        const planned = await native.studioEdit(stateSource, 3, 'designer_state', { action: 'upsert', model: group }, []);
        assert.deepEqual(planned.edits.map(edit => edit.newText), ['800u64']);
        const { animationQuery } = await native.listenHosted(randomBytes(32).toString('hex'));
        const capabilities = new URLSearchParams(animationQuery);
        assert.equal(capabilities.get('ferroui-design-animation'), '1', 'The installed relay negotiates native application animations');
        assert.equal(capabilities.get('ferroui-design-keyframes'), '1', 'The installed relay negotiates project codec authoring');
        await verifyHostedPreview(native, source.uri);
        console.log(`Installed FerroUI ${extension.packageJSON.version} passed on VS Code ${vscode.version} (${process.platform}-${process.arch}).`);
    } finally {
        native.dispose();
        output.dispose();
    }
}
