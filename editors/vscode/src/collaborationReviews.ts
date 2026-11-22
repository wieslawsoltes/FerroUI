import * as vscode from 'vscode';
import type { CollaborationDocument, SourceAction } from './collaborationDocument';
import { SourceChangeReview } from './sourceChangeReview';

export class CollaborationReviews implements vscode.Disposable {
    private readonly apply = new SourceChangeReview({ scheme: 'ferroui-room-review', context: 'ferroui.roomApplyReview',
        command: 'ferroui.collaboration.incoming' });
    private readonly publish = new SourceChangeReview({ scheme: 'ferroui-room-publish', context: 'ferroui.roomPublishReview',
        command: 'ferroui.collaboration.outgoing' });

    async open(room: CollaborationDocument, action: SourceAction): Promise<void> {
        const review = await room.read();
        const incoming = action === 'apply';
        if (!incoming && room.current.role === 'viewer') throw new Error('Viewers can review and apply room source to their local file.');
        if (action === 'publish' && !review.can_publish) throw new Error('Review the new room source or open Publish Reviewed Resolution.');
        const remote = room.forEditor(review.source);
        const plan = { edit: { uri: room.saved.uri, version: review.editor_version, edits: [] },
            before: incoming ? review.local : remote, after: incoming ? remote : review.local };
        const title = incoming ? `Room Source · ${room.saved.label}`
            : action === 'checkpoint_resolution' ? `Room Resolution · ${room.saved.label}` : `Publish · ${room.saved.label}`;
        await (incoming ? this.apply : this.publish).open(title, [plan], async validate => {
            await room.source(action, review.editor_version, review.revision, validate);
            return room.document.uri;
        });
    }

    async recovery(room: CollaborationDocument): Promise<void> {
        const document = room.document;
        const version = document.version;
        const before = document.getText();
        const after = await room.getRecovery();
        await this.apply.open(`Room Recovery · ${room.saved.label}`,
            [{ edit: { uri: room.saved.uri, version, edits: [] }, before, after }], async validate => {
                await room.recover(version, after, validate);
                return document.uri;
            });
    }

    dispose(): void { this.apply.dispose(); this.publish.dispose(); }
}
