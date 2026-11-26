import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { HostedAuthoredPlan } from '../src/hostedSources';
import type { HostedStyleCatalog, HostedStyleDefinition, HostedStyleResources } from '../src/hostedStylesProtocol';
import type { NativePreview } from '../src/nativePreview';
import type { RustStyleEdit } from '../src/stylesProtocol';

export async function verifyHostedRustStyles(preview: NativePreview, baseUri: string): Promise<void> {
    const file = new URL('rust_style_workbench.rs', baseUri);
    const address = file.toString();
    const original = (await readFile(file, 'utf8')).replace(/\r?\n/g, '\r\n');
    const input: HostedSource = {
        revision: 3150, assembly: 'todo-workbench', projectPath: '/rust_style_workbench.rs',
        document: { uri: address, source: original, language: 'rust', caret: { line: 4, character: 9 } },
    };
    const render = (revision: number, source: string) =>
        preview.renderHosted({ ...input, revision, document: { ...input.document, source } });
    const first = await render(3150, original);
    assert.equal(first.stylesAvailable, true);
    const { catalog } = await preview.hostedStyles<HostedStyleCatalog>(first, { action: 'catalog', address });
    assert.equal(catalog.source_kind, 'rust');
    const style = catalog.definitions.find(style => style.name === '#Review')!.node;
    const target = { address, node: style };
    const { definition } = await preview.hostedStyles<HostedStyleDefinition>(first, { action: 'read', target });
    const size = definition.setters.find(setter => setter.property === 'FontSize')!;
    const checked = (frame: HostedFrame, source: string, edit: RustStyleEdit, node = style) =>
        preview.hostedStyles<HostedAuthoredPlan>(frame, { action: 'plan', target: { ...target, node }, source,
            edit: { action: 'rust', edit } });
    const plan = (frame: HostedFrame, source: string, value: string, node = style) =>
        checked(frame, source, { action: 'setter', style, setter: size.node, value }, node);
    await assert.rejects(plan(first, original, 'invalid'));
    await assert.rejects(plan(first, original, '31.5', style + 1), /matching style/);
    const result = await plan(first, original, '31.5');
    assert.equal(result.uri, address);
    assert.equal(result.patches.length, 1);
    const [patch] = result.patches;
    assert.equal(original.slice(patch.start, patch.end), patch.expected);
    const draft = original.slice(0, patch.start) + patch.replacement + original.slice(patch.end);
    assert.equal(draft, original.replace('24.0f64', '31.5f64'));
    const second = await render(3151, draft);
    assert.notEqual(second.image.png, first.image.png);
    await assert.rejects(plan(first, original, '32'), /current app-hosted preview frame/);
    await assert.rejects(plan(second, original, '32'), /buffer changed/);
    const restored = await render(3152, original);
    assert.equal(restored.image.png, first.image.png);

    const { resources } = await preview.hostedStyles<HostedStyleResources>(restored, { action: 'resources', target, property: 'Foreground' });
    assert.deepEqual(resources, [{ key: 'ReviewInk', type_name: 'Color' }]);
    const foreground = definition.setters.find(setter => setter.property === 'Foreground')!;
    const assign = { action: 'set_resource', style, setter: foreground.node, key: 'ReviewInk' } as const;
    await assert.rejects(checked(restored, original, { ...assign, key: 'Missing' }), /mapped color resource/);
    const [link] = (await checked(restored, original, assign)).patches;
    assert.equal(original.slice(link.start, link.end), link.expected);
    const repaint = original.slice(0, link.start) + link.replacement + original.slice(link.end);
    assert.equal(repaint, original.replace('.set(ferroui_controls::TextBlock::foreground_property(), "#1C3554")',
        '.set_resource(ferroui_controls::TextBlock::foreground_property(), "ReviewInk")'));
    const linked = await render(3153, repaint);
    assert.notEqual(linked.image.png, restored.image.png);
    const resource = definition.color_resources?.[0];
    assert.ok(resource);
    const [color] = (await checked(linked, repaint, { action: 'color_resource', style, resource: resource.node, value: '#FF226F5C' })).patches;
    assert.equal(repaint.slice(color.start, color.end), color.expected);
    const changed = repaint.slice(0, color.start) + color.replacement + repaint.slice(color.end);
    assert.equal(changed, repaint.replace('34, /* palette */ 78, 124', '34, /* palette */ 111, 92'));
    assert.notEqual((await render(3154, changed)).image.png, linked.image.png);
    assert.equal((await render(3155, original)).image.png, first.image.png);

    const defaultResource = repaint.replace('.resource("ReviewInk",',
        '.theme_resource(ferroui_base::styling::ThemeVariant::default(), "ReviewInk",');
    const themed = await render(3156, defaultResource);
    assert.equal(themed.image.png, linked.image.png);
    const addDark = { action: 'add_color_resource', style, layer: 'dark', key: 'ReviewInk', value: '#FFB3D8EF' } as const;
    const [dark] = (await checked(themed, defaultResource, addDark)).patches;
    assert.equal(defaultResource.slice(dark.start, dark.end), dark.expected);
    assert.match(dark.replacement, /\.theme_resource\([^,]+::dark\(\), "ReviewInk"/);
    const darkSource = defaultResource.slice(0, dark.start) + dark.replacement + defaultResource.slice(dark.end);
    const darkFrame = await render(3157, darkSource);
    const darkStyles = await preview.hostedStyles<HostedStyleDefinition>(darkFrame, { action: 'read', target });
    const angles = darkStyles.definition.resource_comparison?.find(row => row.key === 'ReviewInk');
    assert.equal(angles?.light?.value, '#FF224E7C');
    assert.equal(angles?.dark?.value, '#FFB3D8EF');
    assert.deepEqual(angles?.high_contrast, angles?.dark);
    await assert.rejects(checked(darkFrame, darkSource, addDark), /selected layer/);
    assert.equal((await render(3158, original)).image.png, first.image.png);
    console.log('App Preview linked a typed Rust color resource, added a Dark declaration, resolved fallbacks, and restored native pixels and source');
}
