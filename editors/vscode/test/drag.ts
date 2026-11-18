import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import * as vscode from 'vscode';
import { DesignerDrag } from '../src/designerDrag';
import type { DesignerPanel } from '../src/designerPanel';
import type { DragGesture, DragOrigin, DropTarget } from '../src/protocol';

export async function verifyDrag(panel: DesignerPanel): Promise<void> {
    const original = panel.document.getText();
    let active: DragGesture | undefined;
    const drag = new DesignerDrag(update => { active = update.gesture; }, () => false, async () => {});
    const begin = async (origin: DragOrigin) => {
        assert.notEqual(panel.currentToken, undefined);
        const frame = { session: panel.session, token: panel.currentToken! };
        const gesture = { id: randomUUID(), frame, origin };
        await drag.receive(panel, { type: 'beginDrag', gesture });
        assert.equal(active, gesture);
        return gesture;
    };
    const drop = ({ id, frame }: DragGesture, target: DropTarget) =>
        drag.receive(panel, { type: 'drop', id, frame, target });
    const undo = async () => {
        await vscode.window.showTextDocument(panel.document);
        await vscode.commands.executeCommand('undo');
        assert.equal(panel.document.getText(), original);
        return panel.waitForSnapshot(AbortSignal.timeout(15_000));
    };
    try {
        const snapshot = await panel.waitForSnapshot(AbortSignal.timeout(15_000));
        assert.ok(snapshot.image);
        const container = snapshot.nodes.find(node => node.label === 'StackPanel' && node.canInsert);
        assert.ok(container?.bounds);
        const bounds = container.bounds;
        const point = { type: 'canvas', x: bounds.x + bounds.width / 2, y: bounds.y + 2 } as const;
        const asset = { type: 'asset', control: 'TextBlock' } as const;

        const first = await begin(asset);
        assert.ok((await panel.previewPlacement(first.frame, { origin: first.origin, target: point })).bounds);
        await drag.receive(panel, { type: 'endDrag', id: first.id });
        await drop(first, point);
        const inserted = panel.document.getText();
        assert.match(inserted, /Text="Text"/);
        await drop(first, point);
        assert.equal(panel.document.getText(), inserted, 'A drag commits at most one source edit');

        const current = await undo();
        const save = current.nodes.find(node => node.name === 'saveButton');
        const title = current.nodes.find(node => node.properties.some(property => property.value === 'Workspace settings'));
        assert.ok(save?.canMove && title);
        await drop(await begin({ type: 'node', node: save.id }), { type: 'outline', node: title.id, position: 'before' });
        assert.ok(panel.document.getText().indexOf('saveButton') < panel.document.getText().indexOf('Workspace settings'));
        await undo();

        const cancelled = await begin(asset);
        await drag.receive(panel, { type: 'endDrag', id: cancelled.id });
        await delay(170);
        await drag.receive(panel, { type: 'endDrag', id: cancelled.id });
        await delay(130);
        assert.equal(active, undefined, 'Repeated release notifications do not extend a completed drag');
        await drop(cancelled, point);
        assert.equal(panel.document.getText(), original);
    } finally {
        drag.clear();
    }
    console.log('VS Code drag placement hints, drops on canvas and Outline, cancellation and undo passed');
}
