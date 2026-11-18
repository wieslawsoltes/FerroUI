import type { DesignerPanel } from './designerPanel';
import { errorMessage, type FrameTarget, type Inspection, type InspectionTarget } from './protocol';

interface Query {
    key: string;
    data?: Inspection;
    error?: string;
}

export class SelectionInspection {
    private current?: Query;

    constructor(private readonly changed: () => void) {}

    read(panel?: DesignerPanel, frame?: FrameTarget): { data?: Inspection; message?: string } {
        if (!panel || !frame) return {};
        const node = panel.selectedNode;
        const runtime = panel.runtimeSelection;
        const target: InspectionTarget | undefined = runtime ? { type: 'runtime', id: runtime.inspector_id }
            : node ? { type: 'source', node: node.id } : undefined;
        if (!target) return {};
        const key = `${frame.session}:${frame.token}:${node?.id ?? 'runtime'}:${runtime?.inspector_id ?? ''}`;
        if (this.current?.key !== key) {
            const query: Query = { key };
            this.current = query;
            void panel.inspect(frame, target).then(data => { query.data = data; },
                error => { query.error = errorMessage(error); }).finally(() => {
                if (this.current === query) this.changed();
            });
        }
        return { data: this.current.data, message: this.current.error ?? (this.current.data ? undefined : 'Reading selection…') };
    }
}
