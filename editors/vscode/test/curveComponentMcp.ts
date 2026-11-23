import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { McpFixture } from './mcp';
import type { TimelineProgram } from '../src/timelineProtocol';

const xaml = `<Window xmlns="https://github.com/ferroui" Width="420" Height="260">
    <Window.Styles>
        <Style Selector="Border#ReviewCard"><Style.Animations>
            <Animation Duration="0:0:2" FillMode="Both">
                <KeyFrame Cue="25%">
                    <!-- keep the corner review 🧭 -->
                    <Setter Property="CornerRadius" Value = '4,8,16,24'/>
                    <Setter Property="Opacity" Value="0&#46;8"/>
                </KeyFrame>
                <KeyFrame Cue="100%"><Setter Property="CornerRadius" Value="16,20,28,38"/></KeyFrame>
            </Animation>
        </Style.Animations></Style>
    </Window.Styles>
    <Border Name="ReviewCard" Width="270" Height="150" Background="#305F9C"/>
</Window>
`;

const rust = `use ferroui_view::{prelude::*, animation as clip};
use ferroui_base::{CornerRadius as Radius, animation::FillMode::Both as Hold};
use ferroui_controls::Border as Card;

fn header() { rsx! { TextBlock { text: "Keep this view 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [style("Border#ReviewCard").animation(
            clip(ms(2000))
                .key_frame(0.25, |key| key.set(Card::corner_radius_property(),
                    Radius::top_bottom(/* retain 🧭 */ 8.0f64, 24.0)))
                .key_frame(1.0, |key| key.set(Card::corner_radius_property(),
                    Radius::new(16.0, 20.0, 28.0, 38.0)))
                .fill_mode(Hold)
        )],
        Border { name: "ReviewCard", width: 270.0, height: 150.0, background: "#305F9C" }
    }
} }
`;

export async function verifyCurveComponentMcp({ invoke, content }: McpFixture): Promise<void> {
    for (const [path, text] of [['CurveComponent.xaml', xaml], ['curve_component.rs', rust]]) {
        const original = text.replaceAll('\n', '\r\n');
        const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
        await vscode.workspace.fs.writeFile(uri, Buffer.from(original));
        const document = await vscode.workspace.openTextDocument(uri);
        const { session } = content<{ session: number }>(await invoke('designer_open', {
            path, ...(path.endsWith('.rs') ? { line: document.positionAt(original.lastIndexOf('rsx!') + 6).line, character: 17 } : {}),
        }));
        const frame = async () => {
            assert.equal(content<{ status: string }>(await invoke('designer_wait', { session })).status, 'Ready');
            return content<{ revision: number }>(await invoke('designer_outline', { session }));
        };
        const query = (tool: string, args: object) => invoke('designer_studio_query', { session, tool, arguments: args });
        const tracks = async () => {
            const { programs } = content<{ programs: TimelineProgram[] }>(await query('designer_programs',
                { include_source: false, include_keyframe_text: true }));
            const program = programs.find(candidate => candidate.timeline?.tracks.some(track => track.property === 'CornerRadius'));
            assert.ok(program?.timeline);
            return { program: program.id, track: program.timeline.tracks.find(track => track.property === 'CornerRadius')! };
        };
        try {
            const { revision } = await frame();
            const { program, track } = await tracks();
            const key = track.keyframes[0];
            assert.equal(key.curve_values?.find(value => value.channel === 'top_right')?.value, 8);
            const location = { program, target: track.target, property: track.property, previous: key.time, time: key.time, easing: key.easing_text };
            const component = { channel: 'top_right', value: 30 };
            const draft = { ...location, components: [component, { channel: 'bottom_left', value: 42 }] };
            assert.equal(content<{ valueKind: string }>(await query('designer_keyframe_draft', { ...location, component })).valueKind, 'CornerRadius');
            assert.ok((await query('designer_keyframe_draft', { ...draft, value: '8' })).isError);
            assert.ok((await query('designer_keyframe_draft', { ...draft, component })).isError);
            assert.ok((await query('designer_keyframe_draft', { ...draft, components: [component, component] })).isError);
            assert.ok((await query('designer_keyframe_draft', { ...draft, components: [component, { channel: 'height', value: 30 }] })).isError);
            assert.equal(content<{ valueKind: string }>(await query('designer_keyframe_draft', draft)).valueKind, 'CornerRadius');
            assert.equal(document.getText(), original);
            const commit = (revision: number) => invoke('designer_studio_edit', { session, expected_revision: revision,
                tool: 'designer_program', arguments: { action: 'upsert_keyframe', ...draft } });
            content(await commit(revision));
            await frame();
            const edited = (await tracks()).track;
            const next = edited.keyframes[0].curve_values;
            assert.deepEqual(next?.map(value => value.value), path.endsWith('.rs') ? [8, 30, 24, 42] : [4, 30, 16, 42]);
            assert.deepEqual(edited.keyframes[1].curve_values, track.keyframes[1].curve_values);
            assert.doesNotMatch(document.getText(), /(?<!\r)\n/);
            if (path.endsWith('.rs')) assert.equal(document.getText(), original.replace(
                'Radius::top_bottom(/* retain 🧭 */ 8.0f64, 24.0)',
                'Radius::new(/* retain 🧭 */ 8.0f64, 30.0, 24.0, 42.0)',
            ));
            else assert.equal(document.getText(), original.replace("'4,8,16,24'", "'4,30,16,42'"));
            assert.ok((await commit(revision)).isError);
            content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
            assert.equal(document.getText(), original);
            if (path.endsWith('.xaml')) {
                const { revision: accepted } = await frame();
                const current = await tracks();
                const key = current.track.keyframes[1];
                content(await invoke('designer_studio_edit', { session, expected_revision: accepted,
                    tool: 'designer_program', arguments: {
                        action: 'upsert_keyframe', program: current.program, target: current.track.target,
                        property: current.track.property, previous: key.time, time: 1.75, easing: '0.2,0.3,0.8,1',
                        component: { channel: 'top_right', value: 30 },
                    } }));
                assert.equal(document.getText(), original.replace(
                    'Cue="100%"><Setter Property="CornerRadius" Value="16,20,28,38"',
                    'Cue="87.5%" KeySpline="0.2,0.3,0.8,1"><Setter Property="CornerRadius" Value="16,30,28,38"',
                ));
                content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
                assert.equal(document.getText(), original);
            }
        } finally {
            const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
                .filter(tab => tab.label === path || tab.label === `Design · ${path}`);
            await vscode.window.tabGroups.close(tabs);
            await vscode.workspace.fs.delete(uri);
        }
    }
    console.log('MCP patched XAML setters and a mapped Rust key, rejected stale and invalid edits, and verified exact Undo');
}
