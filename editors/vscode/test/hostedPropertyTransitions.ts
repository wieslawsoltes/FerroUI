import { strict as assert } from 'node:assert';
import { setTimeout } from 'node:timers/promises';
import * as vscode from 'vscode';
import { runEditorHistory } from '../src/editorHistory';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import { applyHostedSourcePatches } from '../src/hostedSourceEdit';
import type { NativePreview } from '../src/nativePreview';
import type { TransitionCommand, TransitionOwner } from '../src/propertyTransitionProtocol';
import type { StateEasing } from '../src/stateGroupProtocol';
import { applyStateDraft } from './hostedStateAuthoring';

const spring: StateEasing = { kind: 'Spring', value: { mass: 1, stiffness: 190, damping: 18, initial_velocity: 0 } };
type Source = (revision: number, source: string) => HostedSource;

export async function verifyHostedPropertyTransitions(preview: NativePreview, update: Source): Promise<void> {
    assert.equal(preview.capabilities.hostedPropertyTransitions, true);
    await verifyXaml(preview, update);
    await verifyRust(preview, (revision, text) => {
        const source = update(revision, text);
        return { ...source, projectPath: '/HostedTransitionCard.rs', document: {
            ...source.document, uri: new URL('HostedTransitionCard.rs', source.document.uri).toString(),
            language: 'rust', caret: { line: 1, character: 16 },
        } };
    });
    console.log('App Preview edited native app-registered XAML and mapped Rust property transitions with stale frame checks and one Undo');
}

async function verifyXaml(preview: NativePreview, update: Source) {
    const source = `<Window xmlns="https://github.com/ferroui" xmlns:app="using:TodoWorkbench.Design"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Width="380" Height="240">
    <!-- Keep the app preview heading 🌱 -->
    <Window.Resources>
        <Transitions x:Key="SharedMotion"/>
    </Window.Resources>
    <Window.Styles>
        <Style Selector="Border#Card:pointerover"><Setter Property="Opacity" Value="0.5"/></Style>
        <Style Selector="app|PreviewBadge#Badge:pointerover"><Setter Property="Emphasis" Value="1.6"/></Style>
    </Window.Styles>
    <Border Name="Card" Width="320" Height="130">
        <app:PreviewBadge Name="Badge" Text="Application control" VerticalAlignment="Center"/>
    </Border>
</Window>`.replaceAll('\n', '\r\n');
    const initial = await preview.renderHosted(update(4200, source));
    const badge = owner(initial, 'Badge');
    const choices = await preview.hostedPropertyTransitions(initial, badge);
    assert.ok(choices.canAdd, 'Show app-specific choices before the first transition');
    assert.ok(choices.properties?.some(choice => choice.property === 'Emphasis' && choice.typeName === 'DoubleTransition'));
    assert.ok(!(await preview.hostedPropertyTransitions(initial, owner(initial, 'Card')))
        .properties?.some(choice => choice.property === 'Emphasis'));
    await assert.rejects(preview.hostedPropertyTransitions({ ...initial, sequence: initial.sequence + 1 }, badge), /frame changed/i);
    await assert.rejects(preview.hostedPropertyTransitions(initial, { owner: 999999 }), /authored transition owner/);
    await assert.rejects(preview.planHostedPropertyTransition(initial, { ...badge, command: {
        action: 'add', property: 'Emphasis', type_name: 'ThicknessTransition',
    } }), /compatible property and transition class/);

    const added = await edit(preview, initial, source, badge, {
        action: 'add', property: 'Emphasis', timing: { duration: 1.2, delay: .1, easing: spring },
    }, 'xaml');
    assert.match(added, /DoubleTransition Property="Emphasis"/);
    assert.match(added, /DoubleTransition\.Easing>\s*<SpringEasing/);
    const first = await preview.renderHosted(update(4201, added));
    await assert.rejects(preview.hostedPropertyTransitions(initial, badge), /frame changed/i);
    const transitions = await preview.hostedPropertyTransitions(first, owner(first, 'Badge'));
    assert.equal(transitions.entries[0].durationSeconds, 1.2);
    assert.equal(transitions.entries[0].delaySeconds, .1);
    assert.ok(transitions.entries[0].editableTiming.includes('Delay'));
    const pose = await verifyNativeTransition(preview, first);
    const removed = await edit(preview, pose, added, owner(first, 'Badge'),
        { action: 'remove', transition: transitions.entries[0].id });
    const second = await preview.renderHosted(update(4202, removed));
    assert.deepEqual((await preview.hostedPropertyTransitions(second, owner(second, 'Badge'))).entries, []);

    const style = await edit(preview, second, removed, owner(second, 'Border#Card:pointerover'),
        { action: 'add', property: 'Opacity', timing: { duration: .5 } });
    assert.match(style, /<Setter Property="Transitions"><Transitions><DoubleTransition Property="Opacity"/);
    const third = await preview.renderHosted(update(4203, style));
    const standalone = owner(third, 'SharedMotion');
    const untyped = await preview.hostedPropertyTransitions(third, standalone);
    assert.ok(untyped.requiresTarget);
    assert.equal(untyped.canAdd, false);
    const target = { ...standalone, target_type: 'app:PreviewBadge' };
    const shared = await edit(preview, third, style, target, { action: 'add', property: 'Emphasis' });
    assert.match(shared, /\{x:Static app:PreviewBadge\.EmphasisProperty\}/);
    const fourth = await preview.renderHosted(update(4204, shared));
    const resource = { ...owner(fourth, 'SharedMotion'), target_type: target.target_type };
    const entries = await preview.hostedPropertyTransitions(fourth, resource);
    assert.equal(entries.entries.length, 1);
    assert.match(shared, /Keep the app preview heading 🌱/u);
}

async function verifyNativeTransition(preview: NativePreview, frame: HostedFrame): Promise<HostedFrame> {
    await preview.hostedInput({ type: 'click', revision: frame.revision, sequence: frame.sequence, x: .5, y: .5 });
    const images = new Set([frame.image.png]);
    let newest = frame;
    const until = Date.now() + 3_000;
    while (Date.now() < until) {
        const current = await preview.hostedFrame();
        if (current?.revision === frame.revision) {
            newest = current;
            images.add(current.image.png);
        }
        if (images.size >= 4) return newest;
        await setTimeout(20);
    }
    assert.fail(`The app's custom property should animate across several native frames. Observed ${images.size}`);
}

async function verifyRust(preview: NativePreview, update: Source) {
    const original = `fn title() { rsx! { TextBlock { text: "Keep title 🌱" } } }
fn card() { rsx! {
    Border {
        name: "Card", width: 230.0, height: 120.0, padding: 18.0, background: "#DFEAF7",
        transitions: ferroui_view::transitions([
            ferroui_view::transition(ferroui_base::Visual::opacity_property(), ferroui_view::ms(/* retain 🧭 */ 300u64)),
        ]),
        TextBlock { text: "Renewals" }
    }
} }`.replaceAll('\n', '\r\n');
    const initial = await preview.renderHosted(update(4210, original));
    const card = owner(initial, 'Card');
    const initialList = await preview.hostedPropertyTransitions(initial, card);
    assert.deepEqual(initialList.entries.map(entry => entry.property), ['Opacity']);
    const margin = initialList.properties?.find(property => property.label === 'Margin');
    const brush = initialList.properties?.find(property => property.label === 'Background');
    assert.ok(margin && brush);
    await assert.rejects(preview.hostedPropertyTransitions(initial, { ...card, target_type: 'Border' }), /mapped Rust control/);
    const timed = await edit(preview, initial, original, card, {
        action: 'timing', transition: initialList.entries[0].id, timing: { duration: .72, easing: spring },
    }, 'rust');
    assert.match(timed, /retain 🧭 \*\/ 720u64/u);
    assert.match(timed, /SpringEasing::with_values\(1\.0, 190\.0, 18\.0, 0\.0\)/);
    const first = await preview.renderHosted(update(4211, timed));
    const added = await edit(preview, first, timed, owner(first, 'Card'),
        { action: 'add', property: margin.property, type_name: margin.typeName });
    const second = await preview.renderHosted(update(4212, added));
    const secondList = await preview.hostedPropertyTransitions(second, owner(second, 'Card'));
    const moved = await edit(preview, second, added, owner(second, 'Card'),
        { action: 'move', transition: secondList.entries[1].id, index: 0 });
    const third = await preview.renderHosted(update(4213, moved));
    const thirdList = await preview.hostedPropertyTransitions(third, owner(third, 'Card'));
    assert.deepEqual(thirdList.entries.map(entry => entry.property), ['Margin', 'Opacity']);
    const retargeted = await edit(preview, third, moved, owner(third, 'Card'),
        { action: 'retarget', transition: thirdList.entries[0].id, property: brush.property, type_name: brush.typeName });
    const fourth = await preview.renderHosted(update(4214, retargeted));
    const fourthList = await preview.hostedPropertyTransitions(fourth, owner(fourth, 'Card'));
    assert.deepEqual(fourthList.entries.map(entry => entry.property), ['Background', 'Opacity']);
    const removed = await edit(preview, fourth, retargeted, owner(fourth, 'Card'),
        { action: 'remove', transition: fourthList.entries[0].id });
    const final = await preview.renderHosted(update(4215, removed));
    const finalList = await preview.hostedPropertyTransitions(final, owner(final, 'Card'));
    assert.deepEqual(finalList.entries.map(entry => entry.property), ['Opacity']);
    assert.equal(finalList.entries[0].durationSeconds, .72);
    assert.equal(removed.split('fn card()')[0], original.split('fn card()')[0]);
}

function owner(frame: HostedFrame, label: string): TransitionOwner {
    const target = frame.animations?.propertyTransitionOwners?.find(candidate => candidate.label === label);
    assert.ok(target, `Expected a source-backed transition owner for ${label}`);
    return { owner: target.id };
}

async function edit(preview: NativePreview, frame: HostedFrame, source: string, owner: TransitionOwner,
    command: TransitionCommand, undo?: 'xaml' | 'rust') {
    const { patches } = await preview.planHostedPropertyTransition(frame, { ...owner, command });
    assert.ok(patches.length);
    const result = applyStateDraft(source, patches);
    assert.doesNotMatch(result, /(?<!\r)\n/);
    if (undo) {
        const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, `HostedTransitionUndo.${undo === 'rust' ? 'rs' : 'xaml'}`);
        await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
        const document = await vscode.workspace.openTextDocument(uri);
        await vscode.window.showTextDocument(document, { preview: false });
        try {
            const version = document.version;
            await applyHostedSourcePatches(document, version, patches);
            assert.equal(document.getText(), result);
            await assert.rejects(applyHostedSourcePatches(document, version, patches), /no longer matches/i);
            await runEditorHistory(document, 'undo', () => assert.equal(document.getText(), result), AbortSignal.timeout(5_000));
            assert.equal(document.getText(), source);
        } finally {
            await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
            await vscode.workspace.fs.delete(uri);
        }
    }
    return result;
}
