import { strict as assert } from 'node:assert';
import { mkdtemp, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import type { HostedFrame } from '../src/hostedProtocol';
import type { HostedAuthoredPlan, HostedSourceLocation } from '../src/hostedSources';
import type { HostedStyleCatalog, HostedStyleDefinition } from '../src/hostedStylesProtocol';
import type { NativePreview } from '../src/nativePreview';

const view = `use crate::brand::{INK, KEY};
fn overview() { rsx! { TextBlock { text: "Overview" } }; }
fn review() {
    rsx! { StackPanel {
        styles: [ferroui_view::style("TextBlock")
            .resource(KEY, INK)
            .set_resource(ferroui_controls::TextBlock::foreground_property(), KEY)],
        TextBlock { text: "Customer review" },
    } };
}`;
const colors = 'use ferroui_base::media::Color as Paint;\r\npub(crate) const KEY: &str = "Ink";\r\npub(crate) const INK: Paint = /* 🎨 */ Paint::from_rgb(37u8, 74, 112);\r\n';

export async function verifyHostedModuleColors(preview: NativePreview): Promise<void> {
    if (!preview.capabilities.hostedRustModules) return;
    const directory = await mkdtemp(join(tmpdir(), 'ferroui-hosted-rust-modules-'));
    try {
        const root = await realpath(directory);
        const manifest = join(root, 'Cargo.toml');
        const rustTarget = join(root, 'main.rs');
        const library = join(root, 'lib.rs');
        const address = pathToFileURL(join(root, 'review.rs')).toString();
        const brand = pathToFileURL(join(root, 'brand.rs')).toString();
        await Promise.all([
            writeFile(manifest, '[package]\nname = "todo-workbench"\nversion = "0.0.0"\n'),
            writeFile(rustTarget, 'fn main() {}'),
            writeFile(library, 'mod brand; mod review;'),
            writeFile(join(root, 'review.rs'), view),
            writeFile(join(root, 'brand.rs'), colors),
        ]);
        const document = await preview.inProject({ language: 'rust', uri: address, source: view, caret: { line: 3, character: 4 } },
            { workspaceRoot: root, metadata: { manifest, directories: [], package: 'todo-workbench' } });
        const render = (revision: number, draft?: string) => preview.renderHostedWorkspace({
            revision, assembly: 'todo-workbench', projectPath: '/review.rs', document,
        }, { root, rustProject: { manifest, targets: [rustTarget, library] }, localSources: draft === undefined ? [] : [{ uri: brand, source: draft }] });
        const accept = async (revision: number, draft?: string) => {
            const result = await render(revision, draft);
            assert.equal(result.error, null);
            assert.ok(result.frame);
            assert.ok(result.dependencies?.includes(brand));
            return result.frame;
        };
        const first = await accept(3170);
        const brandDraft = colors.replace('37u8', '45u8');
        const accepted = await accept(3171, brandDraft);
        assert.notEqual(accepted.image.png, first.image.png);
        assert.equal(accepted.rustScope?.active.start.line, 3);
        const { catalog } = await preview.hostedStyles<HostedStyleCatalog>(accepted, { action: 'catalog', address });
        const target = { address, node: catalog.definitions[0].node };
        const { definition } = await preview.hostedStyles<HostedStyleDefinition>(accepted, { action: 'read', target });
        const [entry] = definition.color_resources ?? [];
        assert.ok(entry);
        assert.equal(entry.color, '#FF2D4A70');
        assert.equal(entry.sourceAddress, brand);
        assert.equal(entry.shared_constant, 'INK');
        const linked = definition.setters.find(setter => setter.property === 'Foreground')?.resource;
        assert.equal(linked?.sourceAddress, brand);
        assert.deepEqual(linked?.declaration, entry.range);
        const location = await preview.hostedStyles<HostedSourceLocation>(accepted, {
            action: 'locate', address: brand, range: entry.range, source: brandDraft,
        });
        assert.equal(location.uri, brand);
        assert.equal(brandDraft.split('\r\n')[location.range.start.line].slice(location.range.start.character,
            location.range.end.character), 'Paint::from_rgb(45u8, 74, 112)');
        const plan = (frame: HostedFrame) => preview.hostedStyles<HostedAuthoredPlan>(frame, {
            action: 'plan', target, source: view, edit: { action: 'rust', edit: {
                action: 'color_resource', style: target.node, resource: entry.node, value: '#80446238', mode: 'declaration',
            } },
        });
        const edit = await plan(accepted);
        assert.equal(edit.uri, brand);
        await assert.rejects(preview.hostedStyles(accepted, { action: 'verifySource', address: brand, source: colors }), /source changed/);
        await preview.hostedStyles(accepted, { action: 'verifySource', address: brand, source: brandDraft });
        const edited = edit.patches.reduceRight((source, patch) => {
            assert.equal(source.slice(patch.start, patch.end), patch.expected);
            return source.slice(0, patch.start) + patch.replacement + source.slice(patch.end);
        }, brandDraft);
        assert.equal(edited, brandDraft.replace('from_rgb(45u8, 74, 112)', 'from_argb(128, 68u8, 98, 56)'));
        const changed = await accept(3172, edited);
        assert.notEqual(changed.image.png, accepted.image.png);
        assert.deepEqual(changed.rustScope, accepted.rustScope);
        const invalid = await render(3173, 'const INK:');
        assert.match(invalid.error ?? '', /Repair the Rust module sources/);
        assert.equal(invalid.frame, null);
        assert.ok(invalid.dependencies?.includes(brand));
        await assert.rejects(plan(changed), /current app-hosted preview frame/);
        assert.equal((await accept(3174, brandDraft)).image.png, accepted.image.png);
    } finally { await rm(directory, { recursive: true, force: true }); }
}
