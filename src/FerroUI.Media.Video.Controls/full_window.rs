//! Full-window presentation uses the existing window overlay, without moving the
//! owning element or changing its player. The same presenter and chrome move as
//! a unit, so toggling never opens another source or starts another polling clock.
use crate::MediaPlayerElement;
use ferroui_base::{
    input::{InputElement, Key, KeyboardNavigation, KeyboardNavigationMode},
    reactive::IDisposable,
    Rect, Ref, Visual, WeakRef,
};
use ferroui_controls::{
    automation::AutomationProperties, primitives::OverlayLayer, Panel, TopLevel,
};
use std::rc::Rc;
pub(crate) struct FullWindow {
    layer: Ref<OverlayLayer>,
    host: Ref<Panel>,
    resize: Rc<dyn IDisposable>,
    previous_focus: Option<WeakRef<InputElement>>,
}
impl FullWindow {
    pub fn enter(view: &MediaPlayerElement) -> Option<Self> {
        let layer = OverlayLayer::get_overlay_layer(view)?;
        let previous_focus = TopLevel::get_top_level(Some(view))
            .and_then(|top| top.focus_manager().get_focused_element())
            .map(|element| element.downgrade());
        let host = Panel::new();
        host.set_focusable(true);
        KeyboardNavigation::set_tab_navigation(&host, KeyboardNavigationMode::Cycle);
        AutomationProperties::set_name(&host, Some("Full-window video"));
        host.set_z_index(i32::MAX);
        host.set_background(Some(ferroui_base::media::Brushes::black()));
        let weak = view.to_ref().downgrade();
        host.key_down(move |_, event| {
            if event.key == Key::Escape {
                if let Some(view) = weak.upgrade() {
                    view.set_is_full_window(false);
                    event.set_handled(true);
                }
            }
        });
        for child in view.children().snapshot().iter() {
            view.children().remove(child);
            host.children().add(child);
        }
        host.set_width(layer.bounds().width);
        host.set_height(layer.bounds().height);
        let weak_host = host.downgrade();
        let resize = layer.property_changed(move |change| {
            if change.property() == Visual::bounds_property().as_property() {
                if let Some(host) = weak_host.upgrade() {
                    let (_, bounds) = change.get_old_and_new_value::<Rect>();
                    host.set_width(bounds.width);
                    host.set_height(bounds.height);
                }
            }
        });
        layer.children().add(&host);
        // Canvas measures its children with infinity. Arrange this panel against
        // the owning layer and let subsequent parent layouts update its bounds.
        host.focus();
        Some(Self {
            layer,
            host,
            resize,
            previous_focus,
        })
    }
    pub fn panel(&self) -> Ref<Panel> {
        self.host.clone()
    }
    pub fn restore(self, view: &MediaPlayerElement) {
        self.resize.dispose();
        for child in self.host.children().snapshot().iter() {
            self.host.children().remove(child);
            view.children().add(child);
        }
        self.layer.children().remove(&self.host);
        if let Some(focus) = self.previous_focus.and_then(|weak| weak.upgrade()) {
            focus.focus();
        }
    }
}
