import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { HostedAuthoredPlan } from '../src/hostedSources';
import type { HostedStyleCatalog, HostedStyleDefinition } from '../src/hostedStylesProtocol';
import type { NativePreview } from '../src/nativePreview';

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
    const plan = (frame: HostedFrame, source: string, value: string, node = style) =>
        preview.hostedStyles<HostedAuthoredPlan>(frame, { action: 'plan', target: { ...target, node }, source,
            edit: { action: 'rust', edit: { action: 'setter', style, setter: size.node, value } } });
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
    assert.equal((await render(3152, original)).image.png, first.image.png);
    console.log('App Preview updated and restored native pixels from the mapped Rust style without changing its surrounding source');
}
