// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::dock_debug_overlay_manager::DockDebugOverlayManager;
use ferroui_base::{
    input::{InputElement, Key, KeyGesture, KeyModifiers},
    interactivity::RoutingStrategies,
    reactive::IDisposable,
    Ref,
};
use ferroui_controls::{Application, TopLevel};
use std::{cell::RefCell, rc::Rc};

/// Original extension methods for toggling the dock debug overlay.
pub struct DockDebugOverlayExtensions;
ferroui_base::ferro_static_type!(DockDebugOverlayExtensions);
ferroui_base::ferro_markup_type!(static DockDebugOverlayExtensions {
    namespace: "FerroDock.Ferro.Diagnostics", type_info: DockDebugOverlayExtensions,
    methods: [
        static fn AttachDockDebugOverlay(Ref<TopLevel>, Option<KeyGesture>) -> Rc<dyn IDisposable> => DockDebugOverlayExtensions::attach_dock_debug_overlay,
        static fn AttachDockDebugOverlay(Ref<Application>, Option<KeyGesture>) => |app: Ref<Application>, gesture| DockDebugOverlayExtensions::attach_dock_debug_overlay_application(&app, gesture),
    ],
});
impl DockDebugOverlayExtensions {
    /// Attaches the source tunnel hotkey (F9 by default) and returns explicit cleanup.
    pub fn attach_dock_debug_overlay(
        top_level: Ref<TopLevel>,
        gesture: Option<KeyGesture>,
    ) -> Rc<dyn IDisposable> {
        let gesture = gesture.unwrap_or_else(|| KeyGesture::new(Key::F9, KeyModifiers::empty()));
        let manager: Rc<RefCell<Option<Rc<DockDebugOverlayManager>>>> = Rc::new(RefCell::new(None));
        let state = manager.clone();
        let owner = top_level.clone();
        let subscription = top_level.add_disposable_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                if !gesture.matches(Some(event)) {
                    return;
                }
                let current = state.borrow().clone();
                if let Some(current) = current {
                    current.dispose();
                    state.replace(None);
                } else {
                    state.replace(Some(DockDebugOverlayManager::new(owner.clone())));
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        Rc::new(ActionDisposable(Box::new(move || {
            subscription.dispose();
            let current = manager.borrow().clone();
            if let Some(current) = current {
                current.dispose();
            }
            manager.replace(None);
        })))
    }
    /// Attaches to the current desktop windows or a top-level single view, as in the source.
    /// Future windows are not subscribed automatically by this overload.
    pub fn attach_dock_debug_overlay_application(app: &Application, gesture: Option<KeyGesture>) {
        let lifetime = app.application_lifetime();
        if let Some(desktop) = lifetime
            .as_ref()
            .and_then(|lifetime| lifetime.as_classic_desktop_style_application_lifetime())
        {
            for window in desktop.windows() {
                Self::attach_dock_debug_overlay(window.upcast::<TopLevel>(), gesture);
            }
        } else if let Some(single) = lifetime
            .as_ref()
            .and_then(|lifetime| lifetime.as_single_view_application_lifetime())
        {
            if let Some(top_level) = single.main_view().and_then(|view| view.cast::<TopLevel>()) {
                Self::attach_dock_debug_overlay(top_level, gesture);
            }
        }
    }
}

struct ActionDisposable(Box<dyn Fn()>);
impl IDisposable for ActionDisposable {
    fn dispose(&self) {
        (self.0)();
    }
}
