import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import { applyHostedSourcePatches } from '../src/hostedSourceEdit';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import { asRange } from '../src/textDocument';

const companionClip = `styles: [ferroui_view::style("Border#ReviewCard")
                .animation(ferroui_view::animation(ferroui_view::ms(600))
                    .key_frame(0.0, |k| k.set(ferroui_base::Visual::opacity_property(), 0.2))
                    .key_frame(1.0, |k| k.set(ferroui_base::Visual::opacity_property(), 1.0)))],`
    .replaceAll('\n', '\r\n');

export async function verifyHostedStateRoutes(preview: NativePreview, review: HostedSource): Promise<void> {
    const source = review.document.source.replaceAll('"Card"', '"ReviewCard"').replaceAll('"Idle"', '"Queued"')
        .replace('StackPanel {', `StackPanel { width: 516.0, height: 340.0,\r\n            ${companionClip}`)
        .replace('width: 100.0, height: 120.0', 'width: 100.0, height: 120.0, horizontal_alignment: ferroui_base::layout::HorizontalAlignment::Left');
    const update = (revision: number, text: string, options?: Partial<HostedSource>) =>
        preview.renderHosted({ ...review, revision, document: { ...review.document, source: text }, ...options });
    const routeIn = (frame: HostedFrame) => {
        const route = frame.animations?.programs.find(program => program.stateRoute?.from === 'Queued' && program.stateRoute.to === 'Ready');
        assert.ok(route?.stateRoute, 'Find the mapped Queued → Ready state route');
        return { ...route, stateRoute: route.stateRoute };
    };
    const seek = (frame: HostedFrame, program?: number, elapsed = 0, restart = true) =>
        preview.sampleHostedAnimation({ revision: frame.revision, sequence: frame.sequence, program, elapsed, restart });
    const initial = await update(152, source);
    assert.equal(initial.animations?.programs.filter(program => Boolean(program.stateRoute)).length, 3);
    const route = routeIn(initial);
    const clip = initial.animations?.programs.find(program => !program.stateRoute);
    assert.ok(clip);
    const ordinary = await seek(initial, clip.id, .3);
    const start = await seek(ordinary.frame, route.id);
    const middle = await seek(start.frame, route.id, .32, false);
    const end = await seek(middle.frame, route.id, .64);
    assert.equal(end.completed, true);
    assert.notEqual(start.frame.image.png, middle.frame.image.png);
    assert.notEqual(middle.frame.image.png, end.frame.image.png);

    const draft = { program: route.id, duration: .8, easing: 'LinearEasing' };
    await assert.rejects(preview.planHostedState(middle.frame, draft), /frame changed/i);
    await assert.rejects(preview.planHostedState(end.frame, { ...draft, duration: .0001 }), /millisecond/i);
    const plan = await preview.planHostedState(end.frame, draft);
    assert.equal(plan.patches.length, 2);
    const edited = await verifyUndo(source, route.stateRoute.sourceRange, plan.patches);
    const retained = { program: route.id, elapsed: .32, playing: false, completed: false };
    const restoreAnimation = {
        frame: { revision: end.frame.revision, sequence: end.frame.sequence },
        presentation: { selection: route.id, position: retained, overrides: {} },
    };
    const current = await update(153, edited, { restoreAnimation });
    const next = routeIn(current);
    assert.equal(next.duration, .8);
    assert.equal(next.stateRoute.easing, 'LinearEasing');
    assert.deepEqual(current.animationPresentation?.position, { ...retained, program: next.id });
    const reference = await update(154, source.replace('width: 100.0, height:', 'width: 176.0, height:'));
    assert.equal(current.image.png, reference.image.png, 'The replacement first frame is already posed at 32/80 of 120 → 260');
    const referenceMiddle = await update(155, source.replace('width: 100.0, height:', 'width: 137.5, height:'));
    assert.equal(middle.frame.image.png, referenceMiddle.image.png, 'Match the native CubicEaseIn interpolation');

    const fixture = await update(156, source, { fixture: 'Interactive review' });
    const fixtureRoute = routeIn(fixture);
    const sampled = await seek(fixture, fixtureRoute.id, .32);
    assert.notEqual(fixture.image.png, sampled.frame.image.png);
    const stopped = await seek(sampled.frame);
    assert.equal(stopped.frame.image.png, fixture.image.png, 'The fixture’s owned pose returns when the Designer layer is disposed');
    console.log('App Preview sampled Rust routes beside ordinary clips, edited and undid the route, and retained its native pose through a reload');
}

async function verifyUndo(source: string, range: import('../src/protocol').Range,
    patches: import('../src/hostedAnimationProtocol').HostedSourcePatch[]): Promise<string> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const uri = vscode.Uri.joinPath(root, 'HostedStateUndo.rs');
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document, { preview: false });
    try {
        assert.match(document.getText(asRange(range)), /^transition\(Some\("Queued"\)/);
        assert.equal(document.eol, vscode.EndOfLine.CRLF);
        await applyHostedSourcePatches(document, document.version, patches);
        const edited = document.getText();
        assert.equal(edited, source.replace('640u64', '800u64').replace('"CubicEaseIn"', '"LinearEasing"'));
        await vscode.commands.executeCommand('undo');
        assert.equal(document.getText(), source, 'One Undo restores exact CRLF, comments and surrounding Rust');
        return edited;
    } finally {
        await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
        await vscode.workspace.fs.delete(uri);
    }
}
