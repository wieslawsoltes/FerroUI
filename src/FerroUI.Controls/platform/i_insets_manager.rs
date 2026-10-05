use ferroui_base::media::Color;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::Thickness;
use std::cell::Cell;
use std::rc::Rc;

/// Manages the system bars and the safe area of a top-level.
pub trait IInsetsManager {
    /// Gets whether the system bars are visible.
    fn is_system_bar_visible(&self) -> Option<bool>;

    /// Sets whether the system bars are visible.
    fn set_is_system_bar_visible(&self, value: Option<bool>);

    /// Gets whether the window draws edge to edge, behind any visible
    /// system bars.
    fn display_edge_to_edge_preference(&self) -> bool;

    /// Sets whether the window draws edge to edge, behind any visible
    /// system bars.
    fn set_display_edge_to_edge_preference(&self, value: bool);

    /// Gets whether the window is currently displayed edge to edge.
    fn displays_edge_to_edge(&self) -> bool;

    /// Gets the current safe area padding.
    fn safe_area_padding(&self) -> Thickness;

    /// Gets the system bar color.
    fn system_bar_color(&self) -> Option<Color>;

    /// Sets the system bar color.
    fn set_system_bar_color(&self, value: Option<Color>);

    /// Occurs when the safe area for the current window changes.
    ///
    /// Disposing the returned value removes the handler.
    fn safe_area_changed(&self, handler: Rc<dyn Fn(&SafeAreaChangedArgs)>) -> Rc<dyn IDisposable>;
}

/// A base implementation of [`IInsetsManager`] for platform backends: keeps
/// the state and raises the safe area changed event.
#[derive(Default)]
pub struct InsetsManagerBase {
    is_system_bar_visible: Cell<Option<bool>>,
    display_edge_to_edge_preference: Cell<bool>,
    safe_area_padding: Cell<Thickness>,
    system_bar_color: Cell<Option<Color>>,
    safe_area_changed: Rc<HandlerList<dyn Fn(&SafeAreaChangedArgs)>>,
}

impl InsetsManagerBase {
    /// Creates an insets manager with no insets.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the current safe area padding.
    pub fn set_safe_area_padding(&self, value: Thickness) {
        self.safe_area_padding.set(value);
    }

    /// Raises the safe area changed event on the UI thread.
    pub fn on_safe_area_changed(&self, event_args: SafeAreaChangedArgs) {
        let handlers = self.safe_area_changed.clone();
        // The operation can only be canceled by a shutdown of the dispatcher.
        let _ = Dispatcher::ui_thread().invoke_local(move || {
            for (_, handler) in handlers.snapshot().iter() {
                handler(&event_args);
            }
        });
    }
}

impl IInsetsManager for InsetsManagerBase {
    fn is_system_bar_visible(&self) -> Option<bool> {
        self.is_system_bar_visible.get()
    }

    fn set_is_system_bar_visible(&self, value: Option<bool>) {
        self.is_system_bar_visible.set(value);
    }

    fn display_edge_to_edge_preference(&self) -> bool {
        self.display_edge_to_edge_preference.get()
    }

    fn set_display_edge_to_edge_preference(&self, value: bool) {
        self.display_edge_to_edge_preference.set(value);
    }

    fn displays_edge_to_edge(&self) -> bool {
        self.display_edge_to_edge_preference.get()
    }

    fn safe_area_padding(&self) -> Thickness {
        self.safe_area_padding.get()
    }

    fn system_bar_color(&self) -> Option<Color> {
        self.system_bar_color.get()
    }

    fn set_system_bar_color(&self, value: Option<Color>) {
        self.system_bar_color.set(value);
    }

    fn safe_area_changed(&self, handler: Rc<dyn Fn(&SafeAreaChangedArgs)>) -> Rc<dyn IDisposable> {
        let token = self.safe_area_changed.add(handler);
        let handlers = self.safe_area_changed.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
}

/// Provides data for the safe area changed event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SafeAreaChangedArgs {
    safe_area_padding: Thickness,
}

impl SafeAreaChangedArgs {
    /// Creates the event args.
    pub fn new(safe_are_padding: Thickness) -> Self {
        Self { safe_area_padding: safe_are_padding }
    }

    /// The safe area padding.
    pub fn safe_area_padding(&self) -> Thickness {
        self.safe_area_padding
    }
}

/// The theme of the system bars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SystemBarTheme {
    /// Light system bar theme, with light background and a dark foreground.
    Light = 0,

    /// Dark system bar theme, with dark background and a light foreground.
    Dark = 1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_keeps_state_and_raises_safe_area_changed() {
        let _scope = Dispatcher::unit_test_scope();
        let insets = InsetsManagerBase::new();
        assert_eq!(insets.is_system_bar_visible(), None);
        assert!(!insets.displays_edge_to_edge());

        insets.set_display_edge_to_edge_preference(true);
        insets.set_is_system_bar_visible(Some(false));
        insets.set_system_bar_color(Some(Color::new(255, 1, 2, 3)));
        assert!(insets.display_edge_to_edge_preference());
        assert!(insets.displays_edge_to_edge());
        assert_eq!(insets.is_system_bar_visible(), Some(false));
        assert_eq!(insets.system_bar_color(), Some(Color::new(255, 1, 2, 3)));

        let received = Rc::new(Cell::new(None));
        let r = received.clone();
        let subscription = insets.safe_area_changed(Rc::new(move |e| r.set(Some(e.safe_area_padding()))));

        let padding = Thickness::new(1.0, 2.0, 3.0, 4.0);
        insets.set_safe_area_padding(padding);
        insets.on_safe_area_changed(SafeAreaChangedArgs::new(padding));
        assert_eq!(insets.safe_area_padding(), padding);
        assert_eq!(received.get(), Some(padding));

        subscription.dispose();
        received.set(None);
        insets.on_safe_area_changed(SafeAreaChangedArgs::new(padding));
        assert_eq!(received.get(), None);
    }
}
