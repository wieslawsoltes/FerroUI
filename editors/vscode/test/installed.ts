import { strict as assert } from 'node:assert';
import { randomBytes } from 'node:crypto';
import { access, readFile } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { TableColumns } from '../src/columnEditor';
import { NativePreview } from '../src/nativePreview';
import type { DesignTraceSource, PropertyTrace } from '../src/propertyTrace';
import type { PreviewFinding } from '../src/protocol';
import type { SpacingCatalog } from '../src/spacingProtocol';
import { asRange, sourceOf } from '../src/textDocument';
import type { CurvePoint, TimelineProgram } from '../src/timelineProtocol';
import { isInside } from '../src/workspacePaths';
import { verifyHostedPreview } from './hostedPreview';
import { reviewStateSource } from './rsxStatesMcp';
import { until } from './wait';
import type { PaletteSheet, StyleCatalog } from '../src/stylesProtocol';

export async function run(): Promise<void> {
    const extension = vscode.extensions.getExtension('FerroUI.ferroui');
    assert.ok(extension);
    assert.ok(isInside(process.env.FERROUI_TEST_INSTALLED_ROOT!, extension.extensionPath));
    const { views, viewsContainers } = extension.packageJSON.contributes;
    assert.ok(viewsContainers.panel.some(({ id }: { id: string }) => id === 'ferrouiTimeline'));
    assert.ok(views.ferrouiTimeline.some(({ id }: { id: string }) => id === 'ferroui.timeline'));
    assert.ok(!views.ferrouiWorkbench.some(({ id }: { id: string }) => id === 'ferroui.timeline'));
    assert.ok(views.ferrouiWorkbench.some(({ id }: { id: string }) => id === 'ferroui.columns'));
    assert.ok(views.ferrouiWorkbench.some(({ id }: { id: string }) => id === 'ferroui.styles'));
    const readme = await readFile(path.join(extension.extensionPath, 'readme.md'), 'utf8');
    const userGuide = readme.match(/\[VS Code user guides\]\(([^)]+)\)/)?.[1];
    assert.ok(userGuide, 'The installed README links to the user guides');
    const guideUrl = new URL(userGuide);
    assert.equal(guideUrl.origin, 'https://github.com');
    assert.match(guideUrl.pathname, /^\/wieslawsoltes\/FerroUI\/blob\/[^/]+\/docs\/designer\/vscode\/README\.md$/);
    const suffix = process.platform === 'win32' ? '.exe' : '';
    assert.equal(vscode.workspace.getConfiguration('ferroui.languageServer').get('path'), '');
    assert.equal(vscode.workspace.getConfiguration('ferroui.preview').get('path'), '');
    await Promise.all([
        ...['ferroui-xaml-lsp', 'ferroui-designer-preview'].map(name => access(path.join(extension.extensionPath, 'bin', name + suffix))),
        ...['timeline.js', 'designData.js', 'hosted.js'].map(name => access(path.join(extension.extensionPath, 'dist', name))),
        access(path.join(extension.extensionPath, 'dist', 'icons', 'history.svg')),
        access(path.join(extension.extensionPath, 'dist', 'icons', 'table.svg')),
        access(path.join(extension.extensionPath, 'media', 'keyframe-editor.css')),
        access(path.join(extension.extensionPath, 'media', 'timeline-clips.css')),
        access(path.join(extension.extensionPath, 'media', 'canvas-spacing.css')),
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
        assert.ok(native.capabilities.spacingHelpers, 'The installed preview negotiates measured spacing');
        assert.ok(native.capabilities.selectionWrapping, 'The installed preview negotiates selection wrapping');
        assert.ok(native.capabilities.columnAuthoring, 'The installed preview negotiates table column editing');
        assert.ok(native.capabilities.itemTemplateAuthoring, 'The installed preview negotiates item template editing');
        assert.ok(native.capabilities.propertyTrace, 'The installed preview negotiates property value traces');
        assert.ok(native.capabilities.designFixtures, 'The installed preview recognizes named design fixtures');
        assert.ok(native.capabilities.timelineGeometryRecording, 'The installed preview supports canvas animation recording');
        assert.ok(native.capabilities.timelineKeyframeDrafts, 'The installed preview validates keyframe drafts and samples easing');
        assert.ok(native.capabilities.timelineCurves, 'The installed preview samples scalar animation curves');
        assert.ok(native.capabilities.timelineKeyframeScale, 'The installed preview scales authored key selections');
        assert.ok(native.capabilities.rsxAnimationPrograms, 'The installed preview authors whole Rust clips');
        assert.ok(native.capabilities.timelineCombinedPlayback, 'The installed preview plays combined animation programs');
        assert.ok(native.capabilities.timelineClipPlacement, 'The installed preview arranges native animation clips');
        assert.ok(native.capabilities.timelineProgramTargets, 'The installed preview retains animation resource targets');
        assert.ok(native.capabilities.stateTransitionPreview, 'The installed preview samples explicit state transitions');
        assert.ok(native.capabilities.timelineKeyframeTransfer, 'The installed preview transfers typed keyframes');
        assert.ok(native.capabilities.rsxStateAuthoring, 'The installed preview maps typed Rust state declarations');
        assert.ok(native.capabilities.rsxStateStructure, 'The installed preview authors Rust state groups');
        assert.ok(native.capabilities.hostedAnimationRestoration, 'The installed preview restores the accepted app-hosted timeline');
        assert.ok(native.capabilities.hostedPreviewRanges, 'The installed preview preserves an app-hosted work area');
        assert.ok(native.capabilities.hostedClipAuthoring, 'The installed preview plans whole app-hosted clips');
        assert.ok(native.capabilities.hostedStateAuthoring, 'The installed preview plans app-hosted visual states');
        const source = sourceOf(document, 'xaml', new vscode.Position(0, 0));
        const snapshot = await native.render(source, { width: 800, height: 520, dark: false }, 1);
        assert.ok(snapshot.image?.png, snapshot.renderError ?? undefined);
        assert.equal(Buffer.from(snapshot.image.png, 'base64').subarray(1, 4).toString(), 'PNG');
        const spacingSource = { ...source,
            source: '<StackPanel xmlns="https://github.com/ferroui" Spacing="12"><TextBlock Text="First"/><TextBlock Text="Second"/></StackPanel>' };
        const spacing = await native.render(spacingSource, { width: 640, height: 420, dark: false }, 18);
        assert.ok(spacing.image, spacing.renderError ?? undefined);
        const selection = [spacing.nodes.find(node => node.label === 'StackPanel')!.id];
        const handles = await native.studioQuery(18, 'designer_spacing_helpers', {}, selection) as SpacingCatalog;
        const gap = handles.handles.find(handle => handle.label === 'Spacing')!;
        assert.equal(gap.value, 12);
        const spacingEdit = await native.studioEdit(spacingSource, 18, 'designer_spacing_edit', {
            model: handles.model, handle: gap.id, adjustment: { value: 18.5 },
        }, selection);
        assert.equal(spacingEdit.edits.length, 1);
        const { range: gapRange, newText } = spacingEdit.edits[0];
        assert.equal(gapRange.start.line, 0);
        assert.equal(gapRange.end.line, 0);
        const changedGap = spacingSource.source.slice(0, gapRange.start.character) + newText
            + spacingSource.source.slice(gapRange.end.character);
        assert.equal(changedGap, spacingSource.source.replace('Spacing="12"', 'Spacing="18.5"'));
        const childSelection = spacing.nodes.filter(node => node.label === 'TextBlock').map(node => node.id);
        const wrap = { container: 'horizontal_stack' };
        const wrapPlan = await native.studioQuery(18, 'designer_wrap_selection', wrap, childSelection) as { model: string };
        const wrapEdit = await native.studioEdit(spacingSource, 18, 'designer_wrap_selection_edit', {
            ...wrap, expected_model: wrapPlan.model,
        }, childSelection);
        const wrappedSpacing = [...wrapEdit.edits].sort((a, b) => b.range.start.character - a.range.start.character)
            .reduce((source, { range, newText }) => source.slice(0, range.start.character) + newText
                + source.slice(range.end.character), spacingSource.source);
        assert.match(wrappedSpacing, /<StackPanel Orientation="Horizontal"><TextBlock Text="First"\/><TextBlock Text="Second"\/><\/StackPanel>/);
        assert.equal(wrapEdit.retainedSelection?.length, 1);
        const layoutDocument = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'LayoutDiagnostics.xaml'));
        const layout = await native.render(sourceOf(layoutDocument, 'xaml', new vscode.Position(0, 0)), { width: 800, height: 520, dark: false }, 19);
        assert.ok(layout.image, layout.renderError ?? undefined);
        const { findings } = await native.studioQuery(19, 'designer_preview_checks', {}) as { findings: PreviewFinding[] };
        assert.equal(findings.find(finding => finding.code === 'layoutOverflow')?.overflow?.right, 40);
        const anchor = findings.find(finding => finding.property === 'Canvas.Right');
        assert.ok(anchor?.range);
        assert.equal(layoutDocument.getText(asRange(anchor.range)), 'Canvas.Right="32"');
        const tableDocument = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace.uri, 'ColumnsWorkbench.xaml'));
        const tables = await native.render(sourceOf(tableDocument, 'xaml', new vscode.Position(0, 0)), { width: 800, height: 520, dark: false }, 20);
        assert.ok(tables.image, tables.renderError ?? undefined);
        for (const [name, expected] of [['Portfolio', ['Account', 'Owner', 'Region']], ['Status', ['Account', 'Tier', 'Active']]] as const) {
            const node = tables.nodes.find(node => node.name === name);
            assert.ok(node?.bounds);
            const model = await native.studioQuery(20, 'designer_columns', { node: node.id }) as TableColumns;
            assert.equal(model.reason, null);
            assert.deepEqual(model.columns.map(column => column.label), expected);
        }
        const templatesDocument = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace.uri, 'ItemTemplatesWorkbench.xaml'));
        const templatesSource = sourceOf(templatesDocument, 'xaml', new vscode.Position(0, 0));
        const templateViewport = { width: 660, height: 440, dark: false };
        const templates = await native.render(templatesSource, templateViewport, 21);
        assert.ok(templates.image, templates.renderError ?? undefined);
        const directory = templates.nodes.find(node => node.name === 'Directory');
        assert.ok(directory?.bounds);
        const { model } = await native.studioQuery(21, 'designer_item_templates', { node: directory.id }, [directory.id]) as { model: string };
        const item = await native.studioPresent(21, 22, 'designer_item_template', {
            action: 'preview', node: directory.id, property: 'ItemTemplate', expected_model: model,
        }, [directory.id]);
        assert.ok(item.image && item.definitionScope !== null);
        const title = templates.nodes.find(node => node.name === 'CustomerName');
        assert.ok(title);
        const { properties } = await native.inspect(templatesSource, 22, { type: 'source', node: title.id });
        assert.equal(properties.find(property => property.name === 'Text')?.effective, 'Alpine & Co.');
        await assert.rejects(native.render(
            { ...templatesSource, source: templatesSource.source.replace('</UserControl>', '</StackPanel>') }, templateViewport, 23));
        await assert.rejects(native.studioEdit(templatesSource, 22, 'designer_item_template', {
            action: 'copy_inline', node: directory.id, property: 'ItemTemplate', expected_model: model,
        }, [directory.id]), /layout changed/i);
        const repaired = await native.render(templatesSource, templateViewport, 24);
        assert.ok(repaired.image && repaired.definitionScope === item.definitionScope);
        const tracesDocument = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace.uri, 'PropertyTraceWorkbench.xaml'));
        const tracesSource = sourceOf(tracesDocument, 'xaml', new vscode.Position(0, 0));
        const traceFrame = await native.render(tracesSource, { width: 660, height: 440, dark: false }, 25);
        assert.ok(traceFrame.image, traceFrame.renderError ?? undefined);
        const styleCard = traceFrame.nodes.find(node => node.name === 'StyleCard');
        assert.ok(styleCard?.bounds);
        const trace = await native.studioQuery(25, 'designer_property_trace', { node: styleCard.id, property: 'Opacity' }) as PropertyTrace<DesignTraceSource>;
        assert.equal(trace.contributors[0].label, 'Style Border.priority');
        assert.ok(trace.contributors[0].source);
        assert.equal(tracesDocument.getText(asRange(trace.contributors[0].source.range)), '0.72');
        assert.ok(native.capabilities.styleComposition, 'The installed preview negotiates scoped style composition');
        const stylesDocument = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace.uri, 'StylePalette.xaml'));
        const stylesSource = sourceOf(stylesDocument, 'xaml', new vscode.Position(0, 0));
        const darkStyles = await native.render(stylesSource, { width: 660, height: 440, dark: true }, 26);
        assert.ok(darkStyles.image, darkStyles.renderError ?? undefined);
        const catalog = await native.studioQuery(26, 'designer_style_catalog', {}) as StyleCatalog;
        const palette = await native.studioQuery(26, 'designer_palette', { owner: catalog.palette_owner }) as PaletteSheet;
        assert.equal(palette.comparison.find(color => color.key === 'ActionSurface')?.dark?.value, '#FF537BE1');
        const apply = darkStyles.nodes.find(node => node.name === 'ApplyPalette');
        assert.ok(apply?.bounds);
        const liveColor = await native.studioQuery(26, 'designer_property_trace', { node: apply.id, property: 'Background' }) as PropertyTrace<DesignTraceSource>;
        assert.equal(liveColor.effective, '#FF537BE1');
        assert.ok(native.canvasGuides, 'The installed preview negotiates canvas guide support');
        const guide = await native.editCanvasGuides(native.canvasGuides.defaults, { action: 'add', axis: 'Vertical', position: -18.125 });
        assert.deepEqual(guide.settings.guides, [{ id: guide.selected, axis: 'Vertical', position: -18.125 }]);
        const copied = await native.editCanvasGuides(guide.settings, { action: 'duplicate', id: guide.selected!, position: 73.5 });
        assert.deepEqual(copied.settings.guides.map(line => line.position), [-18.125, 73.5]);
        assert.notEqual(copied.selected, guide.selected);
        assert.ok(native.canvasGuides.snapping);
        const { settings: snapped } = await native.editCanvasGuides(copied.settings, { action: 'configure_snap', options: {
            grid_visible: false, grid_spacing: 16, snap_to_grid: false, snap_to_guides: true, snap_to_objects: false, snap_tolerance: 6,
        } });
        const canvas = await native.render({ ...source, source: '<Canvas xmlns="https://github.com/ferroui" Background="White"/>' },
            { width: 800, height: 520, dark: false }, 17);
        assert.ok(canvas.image, canvas.renderError ?? undefined);
        const button = native.assets.find(asset => asset.title === 'Button')!;
        const adjustment = await native.canvasSnap(17, snapped, 1, { type: 'place', recording: false, placement: {
            origin: { type: 'asset', control: button.id }, target: { type: 'canvas', x: 70, y: 100 },
        } });
        assert.deepEqual(adjustment, { delta: { x: 3.5, y: 0 }, guides: [{ axis: 'Horizontal', position: 73.5 }] });
        const project = await native.projectInventory(workspace.uri.fsPath);
        assert.ok(project.files.some(file => file.kind === 'rsx' && file.path === 'views.rs'));
        const rust = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace.uri, 'animation_view.rs'));
        const declaration = rust.getText().indexOf('ferroui_view::animation(');
        assert.ok(declaration >= 0);
        const animationSource = sourceOf(rust, 'rust', rust.positionAt(declaration));
        const animated = await native.render(animationSource, { width: 640, height: 420, dark: false }, 2);
        assert.ok(animated.image, animated.renderError ?? undefined);
        assert.ok(animated.mappedRustAnimations, 'The installed native preview maps an RSX animation declaration');
        const { programs } = await native.studioQuery(2, 'designer_programs', { include_source: false }) as { programs: TimelineProgram[] };
        assert.ok(programs[0]?.sourceRange && programs[0].timeline && programs[0].preview?.available);
        const [program] = programs;
        const track = program.timeline!.tracks[0];
        const clipboard = await native.studioQuery(2, 'designer_keyframe_clipboard', { program: program.id,
            keyframes: [{ target: track.target, property: track.property, time: .5 }] }) as { format: string };
        assert.equal(clipboard.format, 'ferroui.keyframes.v1');
        const curve = await native.studioQuery(2, 'designer_animation_curve', { program: program.id,
            target: track.target, property: track.property, window: { start: 0, end: 2 } }) as { points: CurvePoint[] };
        assert.equal(curve.points.find(point => point.time === .5)?.value, .65);
        assert.ok(native.capabilities.timelineCurveVelocity);
        const velocity = await native.studioQuery(2, 'designer_animation_curve', { program: program.id,
            target: track.target, property: track.property, metric: 'velocity', window: { start: 0, end: 2 } }) as { points: CurvePoint[] };
        assert.ok(velocity.points.some(point => point.value > 0 && point.time < .5));
        const scaled = await native.studioEdit(animationSource, 2, 'designer_program', {
            action: 'scale_keyframes', program: program.id, pivot: 0, factor: .5,
            keyframes: track.keyframes.slice(1).map(({ time }) => ({ target: track.target, property: track.property, time })),
        }, []);
        assert.deepEqual(scaled.edits.map(edit => edit.newText), ['0.125', '0.5']);
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
        const headline = sourceOf(rust, 'rust', new vscode.Position(0, 0));
        const empty = await native.render(headline, { width: 640, height: 420, dark: false }, 4);
        assert.ok(empty.image && !empty.mappedRustAnimations && empty.rustAnimationOwners?.length);
        const created = await native.studioEdit(headline, 4, 'designer_program', {
            action: 'create', owner: empty.rustAnimationOwners[0], name: 'Headline reveal',
            selector: 'TextBlock', duration: .85,
        }, []);
        assert.ok(created.timelineProgramKey);
        assert.equal(created.edits.length, 1);
        const [insertion] = created.edits;
        const range = asRange(insertion.range);
        const draft = headline.source.slice(0, rust.offsetAt(range.start)) + insertion.newText + headline.source.slice(rust.offsetAt(range.end));
        const newClip = await native.render({ ...headline, source: draft }, { width: 640, height: 420, dark: false }, 5);
        assert.ok(newClip.image && newClip.mappedRustAnimations);
        const result = await native.studioQuery(5, 'designer_programs', { include_source: false }) as { programs: TimelineProgram[] };
        assert.deepEqual(result.programs.map(clip => [clip.name, clip.source_key]), [['Headline reveal', created.timelineProgramKey]]);
        const { creatableOwners } = await native.studioQuery(5, 'designer_states', {}) as { creatableOwners: number[] };
        const owner = creatableOwners[0];
        assert.ok(owner);
        const createdGroup = await native.studioEdit({ ...headline, source: draft }, 5, 'designer_state', {
            action: 'upsert', model: { name: 'Visibility', owner, states: [
                { name: 'Visible', setters: [{ target: '', property: 'Opacity', value: '1' }] },
            ], transitions: [] },
        }, []);
        assert.equal(createdGroup.edits.length, 1);
        assert.match(createdGroup.edits[0].newText, /visual_states: \[/);
        assert.match(createdGroup.edits[0].newText, /::ferroui_view::visual_state_group\("Visibility"\)/);
        const current = await native.render(animationSource, { width: 640, height: 420, dark: false }, 6);
        assert.ok(current.image);
        const placement = await native.studioEdit(animationSource, 6, 'designer_program', {
            action: 'place_clips', programs: [program.id], start: .75,
        }, []);
        assert.deepEqual(placement.edits.map(edit => edit.newText), ['.delay(ferroui_view::ms(750))']);
        const { animationQuery } = await native.listenHosted(randomBytes(32).toString('hex'));
        const capabilities = new URLSearchParams(animationQuery);
        assert.equal(capabilities.get('ferroui-design-animation'), '1', 'The installed relay negotiates native application animations');
        assert.equal(capabilities.get('ferroui-design-keyframes'), '1', 'The installed relay negotiates project codec authoring');
        assert.equal(capabilities.get('ferroui-design-curve-channels'), '1', 'The installed relay opts in to geometry channels');
        await verifyHostedPreview(native, source.uri);
        console.log(`Installed FerroUI ${extension.packageJSON.version} passed on VS Code ${vscode.version} (${process.platform}-${process.arch}).`);
    } finally {
        native.dispose();
        output.dispose();
    }
}
