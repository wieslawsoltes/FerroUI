import { fitStageCamera, type StageCamera, type StageManifest, type StageModel, type StageTarget } from './stagesProtocol';

export interface SavedStageView {
    manifest: StageManifest;
    cameras?: (StageCamera | null)[];
}

export class StageViewState {
    private cameras = new Map<number, { camera: StageCamera; revision: number }>();

    reconcile(model: StageModel, replaced?: SavedStageView): void {
        // Imported manifests allocate new native IDs. The stored camera array is
        // ordered with its own accepted manifest and is restored together with it.
        const restored = replaced?.cameras?.length === model.stages.length ? replaced.cameras : undefined;
        const previous = this.cameras;
        this.cameras = new Map();
        if (!model.cameraZooms) return;
        for (const [index, stage] of model.stages.entries()) {
            const prior = previous.get(stage.id);
            const camera = replaced ? restored?.[index] : prior?.camera;
            stage.camera = validCamera(camera, model.cameraZooms) ? camera : fitStageCamera;
            stage.camera_revision = replaced ? 0 : prior?.revision ?? 0;
            this.cameras.set(stage.id, { camera: stage.camera, revision: stage.camera_revision });
        }
    }

    setCamera(model: StageModel, target: StageTarget, camera: StageCamera, scrollRevision?: number) {
        if (!target || !Number.isSafeInteger(target.id) || !Number.isSafeInteger(target.epoch)) throw new Error('Provide the stage ID and epoch.');
        const stage = model.stages.find(stage => stage.id === target.id && stage.epoch === target.epoch);
        if (!stage) throw new Error('Read the current stage ID and epoch.');
        if (!model.cameraZooms) throw new Error('The preview helper does not support stage zoom.');
        if (!validCamera(camera, model.cameraZooms)) throw new Error('Choose a supported stage zoom and finite, nonnegative scroll offsets.');
        if (scrollRevision !== undefined && (stage.camera_revision !== scrollRevision || stage.camera?.zoom_percent !== camera.zoom_percent)) return;
        stage.camera = { zoom_percent: camera.zoom_percent, x: camera.x, y: camera.y };
        stage.camera_revision = scrollRevision ?? (stage.camera_revision ?? 0) + 1;
        const state = { camera: stage.camera, revision: stage.camera_revision };
        this.cameras.set(stage.id, state);
        return state;
    }

    snapshot(model: StageModel): SavedStageView {
        return { manifest: model.manifest, cameras: model.stages.map(stage => stage.camera ?? null) };
    }
}

function validCamera(value: StageCamera | null | undefined, zooms: readonly number[]): value is StageCamera {
    return !!value && Number.isFinite(value.x) && Number.isFinite(value.y) && value.x >= 0 && value.y >= 0
        && (value.zoom_percent === null ? value.x === 0 && value.y === 0 : zooms.includes(value.zoom_percent));
}
