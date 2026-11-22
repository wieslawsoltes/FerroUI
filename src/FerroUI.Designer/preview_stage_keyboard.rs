//! Scoped native input for clients that present a stage as an image.
use super::{with_runtime_errors, PreviewStageBoard, PreviewStageId, PreviewStageMode};
use ferroui_base::diagnostics::DesignTextEnvironment;
use ferroui_base::input::text_input::TextInputMethodClientRequestedEventArgs;
use ferroui_base::input::{
    FocusManager, InputElement, Key, KeyGesture, KeyModifiers, KeyboardNavigationHandler,
    NavigationDirection, NavigationMethod,
};
use ferroui_base::{Ref, Visual};
use ferroui_controls::automation::peers::ControlAutomationPeer;
use ferroui_controls::{Control, TopLevel};
use ferroui_inspect::{Inspector, Permissions};
use ferroui_markup_xaml::source_overlay::SourceSnapshot;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PreviewStageKeyboardInput {
    Focus,
    Key { key: String },
    Text { text: String },
}

#[derive(Serialize)]
pub struct PreviewStageFocus {
    pub name: String,
    pub role: String,
}

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
        DesignTextEnvironment::run(self.text.as_ref(), || self.sources.run(|| {
            with_runtime_errors("keyboard input", || {
                let top =
                    TopLevel::get_top_level(Some(&self.scope)).ok_or("Mount the stage first")?;
                self.dispatch(&top, action)?;
                Ok(self.describe_focus(&top))
            })
        }))
    }

    pub fn current_focus(&self) -> Option<PreviewStageFocus> {
        let top = TopLevel::get_top_level(Some(&self.scope))?;
        self.describe_focus(&top)
    }

    fn describe_focus(&self, top: &TopLevel) -> Option<PreviewStageFocus> {
        let control = self.focused(top)?.cast::<Control>()?;
        let peer = ControlAutomationPeer::create_peer_for_element(&control);
        let name = peer.get_name();
        Some(PreviewStageFocus {
            name: if name.is_empty() { control.name().unwrap_or_default() } else { name },
            role: format!("{:?}", peer.get_automation_control_type()),
        })
    }

    fn dispatch(&self, top: &TopLevel, action: PreviewStageKeyboardInput) -> Result<(), String> {
        let focused = self.focused(top);
        match action {
            PreviewStageKeyboardInput::Focus => {
                if focused.is_none() {
                    self.move_focus(None, KeyModifiers::NONE);
                }
            }
            PreviewStageKeyboardInput::Key { key } if !key.is_empty() && key.len() <= 64 => {
                let gesture = KeyGesture::parse(&key).map_err(|error| error.to_string())?;
                match gesture.key() {
                    Key::None => return Err("Enter a valid key gesture".into()),
                    Key::Tab => self.move_focus(focused.as_ref(), gesture.key_modifiers()),
                    Key::Space
                        if gesture
                            .key_modifiers()
                            .difference(KeyModifiers::SHIFT)
                            .is_empty()
                            && focused.as_ref().is_some_and(accepts_text) =>
                    {
                        send_input(focused, "type_text", json!({"text":" "}))?;
                    }
                    _ => send_input(focused, "press_key", json!({"key":key}))?,
                }
            }
            PreviewStageKeyboardInput::Text { text } if !text.is_empty() && text.len() <= 8192 => {
                send_input(focused, "type_text", json!({"text":text}))?;
            }
            _ => return Err("Enter a key gesture or up to 8 KiB of text".into()),
        }
        Ok(())
    }

    fn focused(&self, top: &TopLevel) -> Option<Ref<InputElement>> {
        top.focus_manager()
            .get_focused_element()
            .filter(|input| self.contains(input))
    }

    fn contains(&self, input: &Ref<InputElement>) -> bool {
        let scope = self.scope.upcast_ref::<Visual>();
        scope == input.upcast_ref::<Visual>()
            || scope.is_visual_ancestor_of(input.upcast_ref::<Visual>())
    }

    fn move_focus(&self, from: Option<&Ref<InputElement>>, modifiers: KeyModifiers) {
        let scope = self.scope.clone().upcast::<InputElement>();
        if let Some(candidate) = tab_target(&scope, from, modifiers.contains(KeyModifiers::SHIFT)) {
            candidate.focus_with(NavigationMethod::Tab, modifiers);
        }
    }
}

pub(super) fn tab_target(
    scope: &Ref<InputElement>,
    from: Option<&Ref<InputElement>>,
    backwards: bool,
) -> Option<Ref<InputElement>> {
    let direction = if backwards {
        NavigationDirection::Previous
    } else {
        NavigationDirection::Next
    };
    let in_scope = |input: &Ref<InputElement>| {
        scope.upcast_ref::<Visual>().is_visual_ancestor_of(input.upcast_ref::<Visual>())
    };
    let can_tab_to = |input: &Ref<InputElement>| {
        in_scope(input) && input.is_effectively_enabled() && input.is_effectively_visible()
            && input.focusable() && input.is_tab_stop()
    };
    let next = |input: &Ref<InputElement>| {
        KeyboardNavigationHandler::get_next(input, direction).filter(can_tab_to)
    };
    from.and_then(next).or_else(|| {
        let entry = if backwards {
            FocusManager::find_last_focusable_element_in(scope)
        } else {
            FocusManager::find_first_focusable_element_in(scope)
        }.filter(in_scope)?;
        // The focus manager's initial search can suggest a disabled element.
        // Native Tab traversal skips it in the same navigation group.
        if can_tab_to(&entry) { Some(entry) } else { next(&entry) }
    })
}

fn accepts_text(input: &Ref<InputElement>) -> bool {
    let query = TextInputMethodClientRequestedEventArgs::new();
    query.set_routed_event(Some(
        InputElement::text_input_method_client_requested_event(),
    ));
    input.raise_event(&query);
    query.client().is_some()
}

fn send_input(input: Option<Ref<InputElement>>, tool: &str, mut args: Value) -> Result<(), String> {
    let control = input
        .and_then(|input| input.cast::<Control>())
        .ok_or("Focus a control in this stage first")?;
    let inspector = Inspector::shared();
    let node = inspector
        .try_borrow_mut()
        .map_err(|_| "The inspector is busy")?
        .node_id(&control);
    args["node_id"] = json!(node);
    ferroui_inspect::execute(
        tool,
        &args,
        Permissions {
            allow_input: true,
            ..Default::default()
        },
    )?;
    Ok(())
}
