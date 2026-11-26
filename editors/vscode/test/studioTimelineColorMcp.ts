import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { CurvePoint, TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';

const scene = `<Border xmlns="https://github.com/ferroui" Name="Accent" Background="Black" Width="180" Height="100">
  <Border.Styles><Style Selector="Border#Accent"><Style.Animations>
    <Animation Duration="0:0:2" FillMode="Both">
      <KeyFrame Cue="0%"><Setter Property="Background" Value="Black"/></KeyFrame>
      <KeyFrame Cue="100%"><Setter Property="Background" Value="#80FF8040"/></KeyFrame>
    </Animation>
  </Style.Animations></Style></Border.Styles>
</Border>`;

export async function verifyTimelineColorMcp({ invoke, content }: McpFixture, session: number, document: vscode.TextDocument) {
    const original = document.getText();
    content(await invoke('designer_source', { session, expected_version: document.version, source: scene }));
    content(await invoke('designer_wait', { session }));
    const query = async <Result>(tool: string, args: object) => content<Result>(await invoke('designer_studio_query', {
        session, tool, arguments: args,
    }));
    const { programs: [program] } = await query<{ programs: TimelineProgram[] }>('designer_programs', { include_source: false });
    const track = program.timeline?.tracks.find(track => track.property === 'Background');
    assert.ok(track);
    assert.deepEqual(track.curve_channels, ['red', 'green', 'blue', 'alpha']);
    const { points } = await query<{ points: CurvePoint[] }>('designer_animation_curve', {
        program: program.id, target: program.targets[0], property: 'Background', channel: 'green', window: { start: 0, end: 2 },
    });
    assert.equal(Math.round(points.find(point => point.time === 1)!.value * 255), 92);
    assert.equal(document.getText(), scene);
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    content(await invoke('designer_wait', { session }));
    assert.equal(document.getText(), original);
    console.log('VS Code and Studio MCP discovered RGBA channels and sampled a solid brush without editing XAML');
}
