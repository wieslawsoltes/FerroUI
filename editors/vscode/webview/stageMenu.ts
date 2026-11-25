import type { FromStages, StageInfo, StageModel } from '../src/stagesProtocol';
import { anchorPopover, element } from './dom';

export type StageField = Extract<FromStages, { type: 'edit' }>['field'];

export class StageMenu {
    private readonly popup = element('stage-menu');
    private readonly actions = this.popup.querySelectorAll<HTMLButtonElement>('button[data-stage-action]');
    private model?: StageModel;
    private invoker?: HTMLButtonElement;

    constructor() {
        anchorPopover(() => this.invoker, this.popup);
        this.popup.addEventListener('toggle', event => {
            const open = event.newState === 'open';
            this.invoker?.setAttribute('aria-expanded', String(open));
            if (!open) this.invoker = undefined;
        });
    }

    bind(button: HTMLButtonElement): void {
        button.popoverTargetElement = this.popup;
        button.setAttribute('aria-controls', this.popup.id);
        button.setAttribute('aria-expanded', 'false');
    }

    update(model: StageModel): void {
        this.model = model;
        for (const action of this.actions) {
            const field = action.dataset.stageAction;
            if (field === 'compare') action.hidden = !model.comparisonSizes?.length;
            if (field === 'duplicate' || field === 'moveEarlier' || field === 'moveLater') action.hidden = !model.stageArrangement;
        }
    }

    prepare(button: HTMLButtonElement, stage: StageInfo, edit: (field: StageField) => void): void {
        if (this.invoker !== button) this.invoker?.setAttribute('aria-expanded', 'false');
        this.invoker = button;
        const index = this.model?.stages.findIndex(entry => entry.id === stage.id) ?? -1;
        const hasFrame = stage.rendered_revision !== null;
        for (const action of this.actions) {
            const field = action.dataset.stageAction as StageField;
            action.disabled = field === 'export' && !hasFrame
                || field === 'compare' && (!hasFrame || !['ready', 'paused'].includes(stage.status.state))
                || field === 'moveEarlier' && index < 1
                || field === 'moveLater' && (index < 0 || !this.model?.stages[index + 1]);
            action.onclick = () => { this.popup.hidePopover(); edit(field); };
        }
    }

    close(): void { if (this.popup.matches(':popover-open')) this.popup.hidePopover(); }
}
