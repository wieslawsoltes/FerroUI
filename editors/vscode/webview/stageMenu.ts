import type { FromStages, StageInfo } from '../src/stagesProtocol';
import { anchorPopover, element } from './dom';

export type StageField = Extract<FromStages, { type: 'edit' }>['field'];

export class StageMenu {
    private readonly popup = element('stage-menu');
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

    prepare(button: HTMLButtonElement, stage: StageInfo, edit: (field: StageField) => void): void {
        if (this.invoker !== button) this.invoker?.setAttribute('aria-expanded', 'false');
        this.invoker = button;
        const actions = this.popup.querySelectorAll<HTMLButtonElement>('button[data-stage-action]');
        for (const action of actions) {
            const field = action.dataset.stageAction as StageField;
            action.disabled = field === 'export' && stage.rendered_revision === null;
            action.onclick = () => { this.popup.hidePopover(); edit(field); };
        }
    }

    close(): void { if (this.popup.matches(':popover-open')) this.popup.hidePopover(); }
}
