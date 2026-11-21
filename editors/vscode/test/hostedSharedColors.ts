import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { HostedAuthoredPlan } from '../src/hostedSources';
import type { HostedStyleCatalog, HostedStyleDefinition } from '../src/hostedStylesProtocol';
import type { NativePreview } from '../src/nativePreview';
import type { RustColorMode } from '../src/stylesProtocol';

export async function verifyHostedSharedColors(preview: NativePreview, baseUri: string): Promise<void> {
    const file = new URL('rust_shared_colors.rs', baseUri);
    const original = (await readFile(file, 'utf8')).replace(/\r?\n/g, '\r\n');
    const line = original.split('\r\n').findIndex(line => line === '    rsx! {');
    assert.ok(line >= 0);
    const address = file.toString();
    const input: HostedSource = { assembly: 'todo-workbench', revision: 3161, projectPath: '/rust_shared_colors.rs',
        document: { uri: address, source: original, language: 'rust', caret: { line, character: 4 } } };
    const render = (revision: number, source: string) => preview.renderHosted({ ...input, revision, document: { ...input.document, source } });
    const first = await render(3161, original);
    assert.equal(first.rustScope?.blocks.length, 2);
    assert.equal(first.rustScope?.active.start.line, line);
    const { catalog } = await preview.hostedStyles<HostedStyleCatalog>(first, { action: 'catalog', address });
    const style = catalog.definitions[0].node;
    const target = { address, node: style };
    const { definition } = await preview.hostedStyles<HostedStyleDefinition>(first, { action: 'read', target });
    const entry = definition.color_resources?.find(color => color.layer === 'default');
    assert.ok(entry);
    assert.equal(entry.shared_constant, 'DEFAULT_INK');
    const plan = (frame: HostedFrame, source: string, mode?: RustColorMode) => preview.hostedStyles<HostedAuthoredPlan>(frame, {
        action: 'plan', target, source,
        edit: { action: 'rust', edit: { action: 'color_resource', style, resource: entry.node, value: '#FF207054', mode } },
    });
    await assert.rejects(plan(first, original), /shared Rust constant/);
    const [change] = (await plan(first, original, 'declaration')).patches;
    assert.equal(original.slice(change.start, change.end), change.expected);
    const updated = original.slice(0, change.start) + change.replacement + original.slice(change.end);
    assert.equal(updated, original.replace('37u8, /* brand 🧭 */ 74, 112', '32u8, /* brand 🧭 */ 112, 84'));
    const changed = await render(3162, updated);
    assert.notEqual(changed.image.png, first.image.png);
    assert.deepEqual(changed.rustScope, first.rustScope);
    await assert.rejects(plan(first, original, 'declaration'), /current app-hosted preview frame/);
    assert.equal((await render(3163, original)).image.png, first.image.png);
    console.log('App Preview updated an imported Rust color declaration and retained the active invocation and original pixels');
}
