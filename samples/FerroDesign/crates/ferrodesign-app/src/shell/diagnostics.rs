//! Constant-size read-only sampling for browser performance measurements.
//! Unlike state_json, this does not traverse chrome, selection, text layouts,
//! prototype state or the document, and cannot create a render or edit.
use super::*;

impl EditorShell {
    pub fn frame_state_json(&self) -> String {
        let editor = self.editor.borrow();
        let viewport = self.canvas.viewport.get();
        let stats = self.canvas.render_stats();
        let projection = self.layer_projection.borrow().stats();
        let mut state = json!({
            "renderedFrames":self.canvas.frames.get(),"sceneFrames":self.scene_frames.get(),
            "documentRevision":editor.document.revision,
            "zoom":viewport.zoom,"pan":{"x":viewport.pan.x,"y":viewport.pan.y},
            "visibleNodes":stats.visible_nodes,"culledNodes":stats.culled_nodes,
            "cachedNodes":stats.cached_nodes,"invalidNodes":stats.invalid_nodes,
            "preparedNodes":stats.prepared_nodes,"rebuiltVisuals":stats.rebuilt_visuals,
            "fullPrepares":stats.full_prepares,"incrementalPrepares":stats.incremental_prepares,
            "rootBoundsTests":stats.root_bounds_tests,"rootCandidates":stats.root_candidates,
            "rootQueryReused":stats.root_query_reused,
            "totalFullPrepares":stats.total_full_prepares,"totalIncrementalPrepares":stats.total_incremental_prepares,
            "totalPreparedNodes":stats.total_prepared_nodes,"totalRebuiltVisuals":stats.total_rebuilt_visuals,
            "layerProjectionBuilds":projection.builds,"layerProjectionReuses":projection.reuses,
        });
        #[cfg(target_os = "emscripten")]
        {
            let presented = ferroui_browser::rendering::RenderStatistics::current();
            state["presentedFrames"] = json!(presented.frames);
            state["presentedFrameThread"] = json!(presented.frame_thread);
            state["presentedFrameKind"] = json!(presented.frame_kind);
        }
        #[cfg(not(target_os = "emscripten"))]
        { state["presentedFrames"] = serde_json::Value::Null; }
        state.to_string()
    }
}
