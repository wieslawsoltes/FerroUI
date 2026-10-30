// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::tab_strip_mouse_wheel_scroll_helper::TabStripMouseWheelScrollHelper;
use ferroui_base::{
    input::InputElement,
    layout::Orientation,
    reactive::{Disposable, IDisposable},
};
use ferroui_controls::ScrollViewer;
use std::rc::Rc;
pub(crate) struct ScrollViewerMouseWheelHookHelper;
impl ScrollViewerMouseWheelHookHelper {
    pub(crate) fn attach(
        scroll_viewer: Option<&ScrollViewer>,
        orientation: Orientation,
    ) -> Option<Rc<dyn IDisposable>> {
        let scroll_viewer = scroll_viewer?.to_ref();
        let viewer = scroll_viewer.clone();
        let token = scroll_viewer.pointer_wheel_changed(move |_, e| {
            if e.handled() {
                return;
            }
            if TabStripMouseWheelScrollHelper::try_handle(Some(&viewer), orientation, e.delta()) {
                e.set_handled(true);
            }
        });
        Some(Disposable::create(move || {
            scroll_viewer.remove_handler(InputElement::pointer_wheel_changed_event(), token);
        }))
    }
}
