//! The test application and the host of the behaviors (two borders in a panel in a window).

use ferroui_base::{FerroObject, ObjectType, Rect, Ref};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Control, Panel, Window};
use ferroui_xaml_interactivity::Interaction;

/// Starts the application of a test.
pub(crate) fn start_app() -> UnitTestApplicationScope {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

/// The object as the root class (the items of the behavior collections).
pub(crate) fn object<T: ObjectType>(value: &Ref<T>) -> Ref<FerroObject> {
    value.clone().upcast()
}

/// Whether running `f` panics (the invalid operations of the reference are panics here).
#[allow(dead_code)]
pub(crate) fn panics(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err()
}

/// A border (the associated object) and another border in a panel in a window.
pub(crate) struct Host {
    pub(crate) window: Ref<Window>,
    pub(crate) panel: Ref<Panel>,
    pub(crate) border: Ref<Border>,
    pub(crate) other: Ref<Border>,
}

impl Host {
    /// Creates the host; the window is not shown yet.
    pub(crate) fn new() -> Self {
        let border = Border::new();
        let other = Border::new();
        let panel = Panel::new();
        panel.children().add(&border);
        panel.children().add(&other);
        let window = Window::new();
        let content: Ref<Control> = panel.clone().upcast();
        window.set_content(Some(Control::boxed(content)));
        Self { window, panel, border, other }
    }

    /// Adds a behavior to the border (`<Interaction.Behaviors>` of markup) and shows the window.
    pub(crate) fn show_with(&self, behavior: Ref<FerroObject>) {
        Interaction::get_behaviors(&self.border).add(behavior);
        self.window.show();
    }

    /// Removes the border from the visual tree.
    pub(crate) fn detach_border(&self) {
        self.panel.children().remove(&self.border);
    }

    /// Sets the size of the bounds of the border (what the layout does).
    pub(crate) fn resize(&self, width: f64, height: f64) {
        self.border.set_bounds(Rect::new(0.0, 0.0, width, height));
    }

    /// Sets the size of the bounds of the other border.
    pub(crate) fn resize_other(&self, width: f64, height: f64) {
        self.other.set_bounds(Rect::new(0.0, 0.0, width, height));
    }

    /// Whether the border has the class `name`.
    pub(crate) fn has(&self, name: &str) -> bool {
        self.border.classes().contains(name)
    }

    /// Whether the other border has the class `name`.
    pub(crate) fn other_has(&self, name: &str) -> bool {
        self.other.classes().contains(name)
    }
}
