import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import { runEditorHistory } from '../src/editorHistory';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { HostedSourcePatch } from '../src/hostedAnimationProtocol';
import { applyHostedSourcePatches } from '../src/hostedSourceEdit';
import type { NativePreview } from '../src/nativePreview';
import type { StateGroupEdit, VisualStateGroup } from '../src/stateGroupProtocol';
import { animationFixture } from './hostedAnimation';

export async function verifyHostedStateAuthoring(preview: NativePreview, makeXaml: (revision: number, source: string) => HostedSource) {
    const xaml = animationFixture(0, '<!-- State scene 🌱 -->').replaceAll('\n', '\r\n');
    const rust = `fn separate() { rsx! { TextBlock { text: "Keep this invocation 🌱" } } }
fn render() { rsx! {
    Border { name: "ReviewCard", width: 240.0, height: 120.0, opacity: /* retain 🧭 */ 0.75, background: "#305DB6" }
}}
`.replaceAll('\n', '\r\n');
    await checkLanguage(preview, makeXaml, xaml, 187);
    await checkLanguage(preview, (revision, source) => {
        const frame = makeXaml(revision, source);
        return { ...frame, projectPath: '/StateReview.rs', document: {
            ...frame.document, uri: new URL('StateReview.rs', frame.document.uri).toString(),
            language: 'rust', caret: { line: 2, character: 8 },
        } };
    }, rust, 195);
}

async function checkLanguage(preview: NativePreview, buildSource: (revision: number, source: string) => HostedSource,
    original: string, revision: number) {
    const initialSource = buildSource(revision, original);
    const language = initialSource.document.language;
    const initial = await preview.renderHosted(initialSource);
    const owner = initial.animations?.stateAuthoring?.owners.find(candidate => candidate.label === 'ReviewCard');
    assert.ok(owner?.canCreate, 'Expose authored live owners before the first state group');
    assert.deepEqual(initial.animations?.stateAuthoring?.groups, []);
    const plan = (frame: HostedFrame, command: StateGroupEdit) => preview.planHostedStateGroup(frame, command);
    const model = (frame: HostedFrame) => {
        const match = frame.animations?.stateAuthoring?.groups.find(group => group.model.name === 'Delivery');
        assert.ok(match, 'Find the accepted state group');
        return match.model;
    };
    const apply = async (frame: HostedFrame, text: string, command: StateGroupEdit, undo = false) => {
        const draft = await plan(frame, command);
        assert.ok(draft.patches.length);
        const updated = applyStateDraft(text, draft.patches);
        assert.doesNotMatch(updated, /(?<!\r)\n/);
        if (undo) await verifyUndo(text, draft.patches, updated, language);
        return updated;
    };
    const first: VisualStateGroup = { owner: owner.id, name: 'Delivery', states: [{ name: 'Waiting', setters: [] }], transitions: [] };
    await assert.rejects(plan(initial, { action: 'upsert', model: { ...first, owner: 999999 } }), /authored state owner/i);
    const added = await apply(initial, original, { action: 'upsert', model: first }, true);
    assert.match(added, /🌱/u);
    if (language === 'rust') {
        assert.match(added, /\/\* retain 🧭 \*\//u);
        assert.equal(added.split('fn render()')[0], original.split('fn render()')[0]);
    }
    const frame = await preview.renderHosted(buildSource(++revision, added));
    assert.deepEqual(model(frame).states, first.states);
    await assert.rejects(plan(initial, { action: 'remove', owner: owner.id, group: first.name }), /frame changed/i);
    const routeModel: VisualStateGroup = {
        ...model(frame),
        states: [
            { name: 'Waiting', setters: [{ target: '', property: 'Opacity', value: '0.2' }] },
            { name: 'Ready', setters: [{ target: '', property: 'Opacity', value: '1' }] },
        ],
        transitions: [
            { from: 'Waiting', to: 'Ready', duration: .64, easing: { kind: 'Linear' } },
            { from: null, to: 'Ready', duration: .8, easing: { kind: 'Named', value: 'CubicEaseIn' } },
        ],
    };
    const authored = await apply(frame, added, { action: 'upsert', model: routeModel });
    const accepted = await preview.renderHosted(buildSource(++revision, authored));
    assert.equal(model(accepted).transitions.length, 2);
    assert.ok(accepted.animations!.programs.every(program => program.stateRoute), 'State-owned programs only appear as states and routes');
    const ready = accepted.animations!.programs.find(program => program.stateRoute?.endpoint && program.stateRoute.to === 'Ready');
    const waiting = accepted.animations!.programs.find(program => program.stateRoute?.endpoint && program.stateRoute.to === 'Waiting');
    const route = accepted.animations!.programs.find(program => program.stateRoute?.from === 'Waiting');
    const defaultRoute = accepted.animations!.programs.find(program => program.stateRoute && !program.stateRoute.endpoint
        && program.stateRoute.from === null && program.stateRoute.to === 'Ready');
    assert.ok(ready && waiting && route && defaultRoute);
    assert.notEqual(ready.id, defaultRoute.id, 'An endpoint has an identity distinct from its default transition');
    const sample = (latest: HostedFrame, program: number, elapsed: number) => preview.sampleHostedAnimation({
        revision: latest.revision, sequence: latest.sequence, program, elapsed, restart: true,
    });
    const base = await sample(accepted, waiting.id, 0);
    const endpoint = await sample(base.frame, ready.id, 0);
    assert.notEqual(base.frame.image.png, endpoint.frame.image.png);
    assert.equal(endpoint.completed, true);
    const middle = await sample(endpoint.frame, route.id, .32);
    assert.notEqual(middle.frame.image.png, base.frame.image.png);
    assert.notEqual(middle.frame.image.png, endpoint.frame.image.png);

    const edit = await preview.planHostedState(middle.frame, { program: route.id, duration: .7, easing: 'LinearEasing' });
    const retimed = applyStateDraft(authored, edit.patches);
    const adjusted = await preview.renderHosted({ ...buildSource(++revision, retimed), restoreAnimation: {
        frame: { revision: middle.frame.revision, sequence: middle.frame.sequence },
        presentation: { selection: route.id, overrides: {}, position: { program: route.id, elapsed: .32, completed: false, playing: false } },
    } });
    assert.equal(adjusted.animationPresentation?.position?.elapsed, .32);
    assert.equal(model(adjusted).transitions.find(transition => transition.from === 'Waiting')?.duration, .7);
    assert.notEqual(adjusted.image.png, middle.frame.image.png);

    const current = model(adjusted);
    const trimmed = await apply(adjusted, retimed, { action: 'upsert', model: {
        ...current, states: [{ name: 'Ready', setters: [] }], transitions: [],
    } });
    const removed = await preview.renderHosted(buildSource(++revision, trimmed));
    assert.deepEqual(model(removed).states, [{ name: 'Ready', setters: [] }]);
    assert.equal(removed.animations?.programs.filter(program => Boolean(program.stateRoute)).length, 0);
    const deleted = await apply(removed, trimmed, { action: 'remove', owner: model(removed).owner, group: current.name });
    const cleared = await preview.renderHosted(buildSource(++revision, deleted));
    assert.deepEqual(cleared.animations?.stateAuthoring?.groups, []);
    console.log(`App Preview edited ${language} state groups, sampled native endpoints and routes, and restored an accepted pose after editing`);
}

function applyStateDraft(source: string, patches: HostedSourcePatch[]): string {
    let next = source.length;
    return patches.reduceRight((text, { start, end, expected, replacement }) => {
        assert.ok(start <= end && end <= next);
        assert.equal(source.slice(start, end), expected);
        next = start;
        return text.slice(0, start) + replacement + text.slice(end);
    }, source);
}

async function verifyUndo(source: string, patches: HostedSourcePatch[], expected: string, language: 'xaml' | 'rust'): Promise<void> {
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, `HostedStateGroupUndo.${language === 'rust' ? 'rs' : 'xaml'}`);
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document, { preview: false });
    try {
        await applyHostedSourcePatches(document, document.version, patches);
        assert.equal(document.getText(), expected);
        await runEditorHistory(document, 'undo', () => assert.equal(document.getText(), expected), AbortSignal.timeout(5_000));
        assert.equal(document.getText(), source);
    } finally {
        await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
        await vscode.workspace.fs.delete(uri);
    }
}
