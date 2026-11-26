//! Scoped native input for clients that present a stage as an image.
use super::{PreviewStageBoard, PreviewStageId, PreviewStageMode, with_runtime_errors};
use ferroui_base::Ref;
use ferroui_base::diagnostics::DesignTextEnvironment;
use ferroui_controls::Control;
pub use ferroui_inspect::{
    KeyboardFocus as PreviewStageFocus, KeyboardInput as PreviewStageKeyboardInput,
};
use ferroui_inspect::{keyboard_focus, keyboard_input};
use ferroui_markup_xaml::source_overlay::SourceSnapshot;

/// An accepted native graph. Deliver input immediately, after releasing the
/// board borrow, so callbacks can use the host while handling the event.
pub struct PreviewStageKeyboardTarget {
    scope: Ref<Control>,
    sources: SourceSnapshot,
    text: Option<DesignTextEnvironment>,
}

impl PreviewStageBoard {
    pub fn keyboard_target(
        &self,
        id: PreviewStageId,
        epoch: u64,
    ) -> Result<PreviewStageKeyboardTarget, String> {
        let stage = &self.stages[self.index(id)?];
        if stage.epoch != epoch {
            return Err("The stage changed. Use its current controls.".into());
        }
        let configuration = &stage.spec.configuration;
        if configuration.mode != PreviewStageMode::Live || !configuration.auto_update {
            return Err(
                "Switch this stage to Interact and resume its preview to use keyboard input".into(),
            );
        }
        let rendered = self.current(stage)?;
        Ok(PreviewStageKeyboardTarget {
            scope: rendered.scope.clone().upcast(),
            sources: rendered.source_snapshot.clone(),
            text: DesignTextEnvironment::get(&rendered.scope),
        })
    }
}

impl PreviewStageKeyboardTarget {
    /// Route through a containing presentation plane so prototype hotspots can own focus.
    pub fn on_surface(mut self, surface: Ref<Control>) -> Result<Self, String> {
        if !surface.is_visual_ancestor_of(&self.scope) {
            return Err("The native stage is outside its presentation plane".into());
        }
        self.scope = surface;
        Ok(self)
    }

    pub fn input(
        self,
        action: PreviewStageKeyboardInput,
    ) -> Result<Option<PreviewStageFocus>, String> {
        DesignTextEnvironment::run(self.text.as_ref(), || {
            self.sources.run(|| {
                with_runtime_errors("keyboard input", || keyboard_input(&self.scope, action))
            })
        })
    }

    pub fn current_focus(&self) -> Option<PreviewStageFocus> {
        keyboard_focus(&self.scope)
    }
}
