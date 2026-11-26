import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import type { NativePreview } from '../src/nativePreview';
import type { Source } from '../src/protocol';
import type { PrototypeGraph, PrototypeLinkTransition, PrototypeScene, StageConfiguration, StageInput, StageContact, StageModel } from '../src/stagesProtocol';

/** Exercise the real native artboard protocol used by the player and MCP. */
export async function verifyNativePrototype(preview: NativePreview, uri: string): Promise<void> {
    const dir = uri.slice(0, uri.lastIndexOf('/') + 1);
    for (const language of ['xaml', 'rust'] as const) {
        const caret = { line: 0, character: 0 };
        const source: Source = { language, uri: `${dir}Journey.${language === 'xaml' ? 'xaml' : 'rs'}`, caret,
            source: language === 'xaml'
                ? `<StackPanel xmlns="https://github.com/ferroui" Background="#EDF3FA" Spacing="12">
                    <Button Name="Open" Content="Open account"/><TextBox Name="Search" Text="Ready"/>
                </StackPanel>`
                : `fn view() { let _ = rsx! { StackPanel { background: "#EDF3FA", spacing: 12.0,
                    Button { name: "Open", "Open account" } TextBox { name: "Search", text: "Ready" }
                } }; }`,
        };
        const dialog: Source = { language: 'xaml', uri: `${dir}AccountDialog.xaml`, caret,
            source: `<Border xmlns="https://github.com/ferroui" CornerRadius="16" Background="White" Padding="24">
                <StackPanel Spacing="12"><TextBox Name="Customer" AutomationProperties.Name="Customer" Text="Northwind"/>
                    <Button Name="Review" Content="Review changes"/></StackPanel></Border>` };
        const confirm: Source = { language: 'xaml', uri: `${dir}AccountConfirm.xaml`, caret,
            source: `<Border xmlns="https://github.com/ferroui" Background="#F6F2FC" Padding="20">
                <Button Name="Confirm" Content="Finish review"/></Border>` };
        const desktop: StageConfiguration = { name: 'Desktop', source_uri: source.uri, target: { kind: 'document' },
            environment: { width: 800, height: 520, theme: 'light', flow_direction: 'inherit' },
            layout: 'fixed', design_time_data: true, states: [], pinned: false, auto_update: true, mode: 'select' };
        const initial = await preview.syncStages(source, { version: 1, stages: [desktop, {
            ...desktop, name: 'Compact', environment: { ...desktop.environment, width: 390, height: 680 },
        }], active: 0 });
        const withDialog = await preview.editStages({ action: 'add', epoch: initial.epoch, name: 'Account dialog',
            width: 360, height: 180, dark: false, source: dialog });
        const withBoth = await preview.editStages({ action: 'add', epoch: withDialog.epoch, name: 'Confirmation',
            width: 240, height: 120, dark: false, source: confirm });
        const [base, , first, second] = withBoth.stages;
        const anchor = async (stage: typeof base, name: string) => {
            const choice = (await preview.stagePrototypeElements(stage)).find(item => item.label.endsWith(`· ${name}`));
            assert.ok(choice, name);
            return choice.anchor;
        };
        const options = { placement: { kind: 'center' }, dim_background: true, dismiss_on_outside: true };
        const graph = { flows: [{ name: 'Account setup', start: 0 }], links: [
            { name: 'Open account', source: 0, trigger: await anchor(base, 'Open'), action: { kind: 'open_overlay' as const, stage: 2, options } },
            { name: 'Review changes', source: 2, trigger: await anchor(first, 'Review'), action: { kind: 'open_overlay' as const, stage: 3, options } },
            { name: 'Finish review', source: 3, trigger: await anchor(second, 'Confirm'), action: { kind: 'close_overlay' as const } },
        ] };
        const board = await preview.editStages({ action: 'setPrototype', epoch: withBoth.epoch, graph });
        const hash = (png: string) => createHash('sha256').update(png).digest('hex');
        const comparison = hash((await preview.stageImage(first)).png);
        const start = await preview.stagePrototype(board.epoch, { action: 'start', flow: 'Account setup' });
        assert.ok(start?.frame);
        const key = (scene: PrototypeScene, input: StageInput) => preview.prototypeInput(scene.frame!, { kind: 'keyboard', input }, true);
        const press = (scene: PrototypeScene, keyText: string) => key(scene, { kind: 'key', key: keyText });
        assert.deepEqual((await key(start, { kind: 'focus' })).focus, { name: 'Open account', role: 'Button' });
        const opened = (await press(start, 'Enter')).scene;
        assert.ok(opened?.frame);
        assert.equal(opened.current, first.id);
        assert.deepEqual((await key(opened, { kind: 'focus' })).focus, { name: 'Customer', role: 'Edit' });
        await assert.rejects(press(start, 'Tab'), /current frame/i);
        const before = await preview.prototypeImage(opened.frame);
        assert.deepEqual([before.width, before.height], [800, 520]);
        await key(opened, { kind: 'text', text: 'Retained account 🪴' });
        assert.notEqual(hash((await preview.prototypeImage(opened.frame)).png), hash(before.png));
        assert.equal((await press(opened, 'Tab')).focus?.name, 'Review changes');
        assert.equal((await press(opened, 'Tab')).focus?.name, 'Customer');
        assert.equal((await press(opened, 'Shift+Tab')).focus?.name, 'Review changes');
        const nested = (await press(opened, 'Space')).scene;
        assert.ok(nested?.frame);
        assert.equal(nested.current, second.id);
        assert.equal(nested.overlays.length, 2);
        const dismissed = await press(nested, 'Escape');
        assert.equal(dismissed.scene?.current, first.id);
        assert.equal(dismissed.focus?.name, 'Review changes');
        assert.equal((await press(dismissed.scene!, 'Tab')).focus?.name, 'Customer');
        const outside = { contact: 3, device: 'mouse', x: 0.02, y: 0.02 } as const;
        const send = (phase: StageContact['phase']) => preview.prototypeInput(dismissed.scene!.frame!,
            { kind: 'pointer', input: { ...outside, phase } }, true);
        const returned = await send('down');
        await assert.rejects(send('up'), /current frame/i);
        assert.equal(returned.scene?.current, base.id);
        assert.equal(returned.focus?.name, 'Open account');
        assert.equal(hash((await preview.stageImage(first)).png), comparison);
        assert.equal((await press(returned.scene!, 'Escape')).scene, null);
        await assert.rejects(preview.prototypeImage(opened.frame), /current frame/i);
        await verifyNativeTransition(preview, board, graph);
    }
    console.log('Native prototype artboard, nested modal focus, text input, dismissal, and stale frame checks passed in XAML and RSX');
}

async function verifyNativeTransition(preview: NativePreview, board: StageModel, graph: PrototypeGraph): Promise<void> {
    assert.ok(preview.capabilities.prototypeTransitions);
    const transitions: PrototypeLinkTransition[] = [
        { link: 0, transition: { effect: 'slide_up', duration_ms: 4000, easing: 'linear' } },
    ];
    const edited = await preview.editStages({ action: 'setPrototype', epoch: board.epoch, graph, transitions });
    assert.equal(edited.manifest.version, 4);
    assert.deepEqual(edited.manifest.prototype_transitions, transitions);
    await assert.rejects(preview.editStages({ action: 'setPrototype', epoch: edited.epoch, graph, transitions: [...transitions, ...transitions] }),
        /distinct prototype interaction/i);
    const start = await preview.stagePrototype(edited.epoch, { action: 'start', flow: 'Account setup' });
    assert.ok(start?.frame);
    const opened = await preview.stagePrototype(edited.epoch, { action: 'activate', link: 0 }, start.frame);
    assert.ok(opened?.frame && opened.animating);
    assert.ok((await preview.prototypeImage(opened.frame)).animating);
    await assert.rejects(preview.prototypeInput(opened.frame, { kind: 'keyboard', input: { kind: 'key', key: 'Tab' } }, true), /transition/i);
    await assert.rejects(preview.prototypeImage(start.frame), /current frame/i);
    const quiet = await preview.stagePrototype(edited.epoch, { action: 'setReducedMotion', reducedMotion: true }, opened.frame);
    assert.equal(quiet?.frame, opened.frame);
    assert.equal(quiet?.animating, false);
    assert.equal((await preview.prototypeImage(opened.frame)).animating, false);
    assert.equal((await preview.prototypeInput(opened.frame, { kind: 'keyboard', input: { kind: 'focus' } }, true)).focus?.name, 'Customer');
    await preview.stagePrototype(edited.epoch, { action: 'setReducedMotion', reducedMotion: false }, opened.frame);
    const closed = await preview.stagePrototype(edited.epoch, { action: 'closeOverlay' }, opened.frame);
    assert.equal(closed?.current, edited.stages[0].id);
    assert.ok(closed?.frame && closed.animating);
    const returned = await preview.stagePrototype(edited.epoch, { action: 'setReducedMotion', reducedMotion: true }, closed.frame);
    assert.equal(returned?.animating, false);
    assert.equal((await preview.prototypeInput(closed.frame, { kind: 'keyboard', input: { kind: 'focus' } }, true)).focus?.name, 'Open account');
    await preview.stagePrototype(edited.epoch, { action: 'stop' }, closed.frame);
    console.log('Prototype transition authoring, reduced motion, focus restoration, and accepted frame checks passed');
}
