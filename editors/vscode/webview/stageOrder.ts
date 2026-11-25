import type { StageModel, StagesRequest } from '../src/stagesProtocol';
import type { StageCard } from './stageCard';
import { connectStageDrag } from './stageDrag';

type StageList = HTMLElement & { moveBefore?(card: HTMLElement, before: Element | null): void };

export function placeStageCard(root: StageList, card: HTMLElement, previous?: HTMLElement): void {
    const next = previous ? previous.nextElementSibling : root.firstElementChild;
    if (card === next) return;
    if (card.parentElement === root && root.moveBefore) root.moveBefore(card, next);
    else root.insertBefore(card, next);
}

export function connectStageOrder(root: HTMLElement, card: StageCard, board: () => StageModel | undefined,
    send: (message: StagesRequest) => void): void {
    connectStageDrag(card.heading, () => {
        const accepted = board();
        if (!accepted?.stageArrangement || accepted.stages.length < 2) return;
        const { id, epoch } = card.stage;
        return {
            current: () => board() === accepted,
            destination: hit => {
                const target = hit?.closest<HTMLElement>('.stage-card');
                return target && target.parentElement === root && target !== card.element ? target : undefined;
            },
            drop: target => {
                const index = Array.from(root.children).indexOf(target);
                if (index >= 0) send({ type: 'moveStage', target: { id, epoch }, index, epoch: accepted.epoch });
            },
        };
    });
}
