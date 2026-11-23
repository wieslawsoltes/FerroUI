import { strict as assert } from 'node:assert';
import { writeFile } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { HostedAnimationAction, HostedAnimationState } from '../src/hostedAnimationProtocol';
import type { HostedGesture } from '../src/hostedProtocol';
import type { McpFixture } from './mcp';
import { verifyTimelineGraphMcp, verifyTimelineLayoutMcp } from './timelineGraphMcp';
import { verifyHostedTrackMcp } from './timelineTrackMcp';
import { verifyTimelineTimingMcp } from './timelineTimingMcp';
import { until } from './wait';

const preview = `<Window xmlns="https://github.com/ferroui" xmlns:app="using:TodoWorkbench.Design"
    Width="380" Height="280" Background="#F2F6FB">
    <Grid Margin="20" RowDefinitions="40,44,46,*">
        <TextBlock Text="Order note" FontSize="25" Foreground="#162C4A"/>
        <TextBox Grid.Row="1" Name="Note" FontSize="17"/>
        <app:PreviewBadge Grid.Row="2" Text="{Binding #Note.Text}" FontSize="16" Foreground="#194779"/>
        <ScrollViewer Grid.Row="3">
            <StackPanel Spacing="8">
                <Border Background="#E2EAF6" Padding="12"><TextBlock Text="Order received"/></Border>
                <Border Background="#D6E8DC" Padding="12"><TextBlock Text="Payment confirmed"/></Border>
                <Border Background="#E0E6F5" Padding="12"><TextBlock Text="Packing in progress"/></Border>
                <Border Background="#EBDFFF" Padding="12"><TextBlock Text="Ready for pickup"/></Border>
                <Border Background="#D3E7F8" Padding="12"><TextBlock Text="Delivery scheduled"/></Border>
            </StackPanel>
        </ScrollViewer>
    </Grid>
</Window>`;

const motion = `<Window xmlns="https://github.com/ferroui" Width="420" Height="260" Background="#F2F6FB">
    <Window.Styles>
        <Style Selector="Border#Ticket"><Style.Animations>
            <Animation Duration="0:0:2" FillMode="Both">
                <!--ferroui-designer-program:5469636b65742072657665616c-->
                <KeyFrame Cue="0%"><Setter Property="Opacity" Value="0.25"/><Setter Property="CornerRadius" Value="8,10,12,14"/></KeyFrame>
                <KeyFrame Cue="100%" KeySpline="0.25,0.2,0.7,1.4"><Setter Property="Opacity" Value="1"/><Setter Property="CornerRadius" Value="18,20,22,24"/></KeyFrame>
            </Animation>
        </Style.Animations></Style>
        <Style Selector="Border#Receipt"><Style.Animations>
            <Animation Delay="0:0:0.4" Duration="0:0:1" FillMode="Both">
                <!--ferroui-designer-program:526563656970742072657665616c-->
                <KeyFrame Cue="0%"><Setter Property="Opacity" Value="0.1"/></KeyFrame>
                <KeyFrame Cue="100%"><Setter Property="Opacity" Value="1"/></KeyFrame>
            </Animation>
        </Style.Animations></Style>
    </Window.Styles>
    <StackPanel Margin="24" Spacing="12">
        <Border Name="Ticket" Background="#FFFFFF" Padding="20" CornerRadius="12">
            <TextBlock Text="Order confirmed" FontSize="24" Foreground="#194779"/>
        </Border>
        <Border Name="Receipt" Background="#D6E8DC" Padding="20" CornerRadius="12">
            <TextBlock Text="Delivery scheduled" FontSize="18" Foreground="#234939"/>
        </Border>
    </StackPanel>
</Window>`;

interface Capture {
    generation: number;
    revision: number;
    sequence: number;
    pixels: string;
}

export async function verifyHostedInputMcp({ invoke, content }: McpFixture, sibling: string, target: object) {
    const file = path.join(path.dirname(sibling), 'PreviewInput.xaml');
    await writeFile(file, preview);
    const uri = vscode.Uri.file(file);
    const document = await vscode.workspace.openTextDocument(uri);
    const original = document.getText();
    const version = document.version;
    const { session } = content<{ session: number }>(await invoke('designer_open', {
        path: vscode.workspace.asRelativePath(uri, false),
    }));
    const config = vscode.workspace.getConfiguration('ferroui.hostedPreview', uri);
    const originalAssembly = config.inspect<string>('assembly')?.workspaceValue;
    try {
        await config.update('assembly', 'todo-workbench', vscode.ConfigurationTarget.Workspace);
        const { generation } = content<{ generation: number }>(await invoke('designer_hosted_open', { session, ...target }));
        const image = async (previous?: Capture): Promise<Capture> => {
            let accepted: Capture | undefined;
            await until(async () => {
                const result = await invoke('designer_hosted_image', { session });
                if (result.isError) return false;
                const frame = content<Omit<Capture, 'pixels'>>(result);
                const pixels = result.content.find(item => item.type === 'image')!.data!;
                if (previous && (frame.sequence <= previous.sequence || pixels === previous.pixels)) return false;
                assert.equal(frame.generation, generation);
                accepted = { ...frame, pixels };
                return true;
            }).catch(async error => {
                const { status } = content<{ status: string }>(await invoke('designer_hosted_animations', { session }));
                throw new Error(`App Preview did not deliver the expected frame. ${status}`, { cause: error });
            });
            return accepted!;
        };
        const input = ({ generation, revision, sequence }: Capture, gesture: HostedGesture) =>
            invoke('designer_hosted_input', { session, expected_generation: generation,
                expected_revision: revision, expected_sequence: sequence, input: gesture });
        const start = await image();
        const click = { type: 'click', x: .34, y: .30 } as const;
        const rejected = await input({ ...start, generation: generation + 1 }, click);
        assert.match(rejected.content[0]?.text ?? '', /current application preview generation/);
        const outside = await input(start, { ...click, x: 1.25 });
        assert.match(outside.content[0]?.text ?? '', /matching input form/);
        assert.match((await input({ ...start, revision: start.revision + 1 }, click)).content[0]?.text ?? '', /frame changed/i);
        assert.deepEqual(content(await input(start, click)), { generation,
            frame: { revision: start.revision, sequence: start.sequence } });
        const focused = await image(start);
        assert.match((await input(start, click)).content[0]?.text ?? '', /frame changed/i);
        const send = async (gesture: HostedGesture) => {
            let before: Capture | undefined;
            await until(async () => {
                const frame = await image();
                const result = await input(frame, gesture);
                if (result.isError && /frame changed/i.test(result.content[0]?.text ?? '')) return false;
                content(result);
                before = frame;
                return true;
            });
            return image(before);
        };
        const typed = await send({ type: 'text', text: 'Leave at reception' });
        assert.notEqual(typed.pixels, focused.pixels);
        const scrolled = await send({ type: 'scroll', x: .60, y: .78, delta: -8 });
        assert.notEqual(scrolled.pixels, typed.pixels);
        const refresh = (expected_revision: number, expected_generation = generation) =>
            invoke('designer_hosted_refresh', { session, expected_generation, expected_revision });
        assert.match((await refresh(scrolled.revision, generation + 1)).content[0]?.text ?? '', /current application preview generation/);
        assert.match((await refresh(scrolled.revision + 1)).content[0]?.text ?? '', /preview revision changed/i);
        const updated = content<Pick<Capture, 'generation' | 'revision'>>(await refresh(scrolled.revision));
        assert.deepEqual([updated.generation, updated.revision], [generation, scrolled.revision + 1]);
        const refreshed = await image();
        assert.equal(refreshed.revision, updated.revision);
        assert.match((await input(scrolled, click)).content[0]?.text ?? '', /frame changed/i);
        assert.equal(document.version, version);
        assert.equal(document.getText(), original);
        console.log('VS Code MCP focused, typed, scrolled, and refreshed the native App Preview while rejecting stale requests');

        content(await invoke('designer_source', { session, expected_version: document.version, source: motion }));
        try {
            await image(refreshed);
            await verifyTimelineGraphMcp({ invoke, content }, session, document, generation);
            await verifyTimelineLayoutMcp({ invoke, content }, session, document, generation);
            await verifyHostedTrackMcp({ invoke, content }, session, document);
            await verifyTimelineTimingMcp({ invoke, content }, session, document, true);
            await verifyLayoutPlayback({ invoke, content }, session, generation);
        } finally {
            content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        }
        assert.equal(document.getText(), original);
    } finally {
        try {
            const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
                tab.label.endsWith(' · PreviewInput.xaml') || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
            await vscode.window.tabGroups.close(tabs);
        } finally {
            await config.update('assembly', originalAssembly, vscode.ConfigurationTarget.Workspace);
        }
    }
}

async function verifyLayoutPlayback({ invoke, content }: McpFixture, session: number, expected_generation: number) {
    const read = async () => content<HostedAnimationState>(await invoke('designer_hosted_animations', { session }));
    const layout = async () => content<{ layout: string }>(await invoke('designer_hosted_timeline_view', { session })).layout;
    const animate = async (arguments_: HostedAnimationAction) => {
        const { frame } = await read();
        assert.ok(frame);
        content(await invoke('designer_hosted_animation', { session, expected_generation,
            expected_revision: frame.revision, expected_sequence: frame.sequence, arguments: arguments_ }));
    };
    const program = (await read()).programs.find(program => program.clip)?.id;
    assert.ok(program);
    await animate({ action: 'scrub_all', elapsed: .45 });
    assert.equal(await layout(), 'clips');
    assert.equal((await read()).position?.clips?.length, 2);
    await animate({ action: 'play_range', program, range: { start: 0, end: .25, loop: false } });
    assert.equal(await layout(), 'tracks');
    await until(async () => !(await read()).position?.playing);
}
