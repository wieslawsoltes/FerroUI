//! The insets manager of a view: the safe area and the status bar, read
//! from the view controller of the view.

use crate::view_controller::IFerroViewController;
use ferroui_base::media::Color;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::Thickness;
use ferroui_controls::platform::{IInsetsManager, InsetsManagerBase, SafeAreaChangedArgs};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The insets manager of the platform.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
pub struct InsetsManager {
    this: Weak<InsetsManager>,
    base: InsetsManagerBase,
    controller: RefCell<Option<Rc<dyn IFerroViewController>>>,
    display_edge_to_edge: Cell<bool>,
    display_edge_to_edge_changed: Rc<HandlerList<dyn Fn(bool)>>,
    // The subscription of the reference lives as long as the controller.
    controller_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl InsetsManager {
    /// Creates a manager without a controller.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            base: InsetsManagerBase::new(),
            controller: RefCell::new(None),
            display_edge_to_edge: Cell::new(true),
            display_edge_to_edge_changed: Rc::new(HandlerList::new()),
            controller_subscription: RefCell::new(None),
        })
    }

    /// Gives the manager the view controller of its view.
    #[cfg_attr(not(target_os = "ios"), allow(dead_code))]
    pub(crate) fn init_with_controller(&self, controller: Rc<dyn IFerroViewController>) {
        *self.controller.borrow_mut() = Some(controller.clone());
        let this = self.this.clone();
        let subscription = controller.safe_area_padding_changed(Rc::new(move || {
            if let Some(this) = this.upgrade() {
                this.base.on_safe_area_changed(SafeAreaChangedArgs::new(this.safe_area_padding()));
                this.raise_display_edge_to_edge_changed(this.display_edge_to_edge.get());
            }
        }));
        *self.controller_subscription.borrow_mut() = Some(subscription);
    }

    fn controller(&self) -> Option<Rc<dyn IFerroViewController>> {
        self.controller.borrow().clone()
    }

    /// The preference for drawing edge to edge changed, or the safe area
    /// did.
    pub fn display_edge_to_edge_changed(&self, handler: Rc<dyn Fn(bool)>) -> Rc<dyn IDisposable> {
        let token = self.display_edge_to_edge_changed.add(handler);
        let handlers = self.display_edge_to_edge_changed.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    fn raise_display_edge_to_edge_changed(&self, value: bool) {
        for (_, handler) in self.display_edge_to_edge_changed.snapshot().iter() {
            handler(value);
        }
    }
}

impl IInsetsManager for InsetsManager {
    fn is_system_bar_visible(&self) -> Option<bool> {
        Some(self.controller().map(|controller| controller.prefers_status_bar_hidden()) == Some(false))
    }

    fn set_is_system_bar_visible(&self, value: Option<bool>) {
        if let Some(controller) = self.controller() {
            controller.set_prefers_status_bar_hidden(value == Some(false));
        }
    }

    fn display_edge_to_edge_preference(&self) -> bool {
        self.display_edge_to_edge.get()
    }

    fn set_display_edge_to_edge_preference(&self, value: bool) {
        if self.display_edge_to_edge.get() != value {
            self.display_edge_to_edge.set(value);
            self.raise_display_edge_to_edge_changed(value);
            self.base.on_safe_area_changed(SafeAreaChangedArgs::new(self.safe_area_padding()));
        }
    }

    fn displays_edge_to_edge(&self) -> bool {
        self.display_edge_to_edge.get()
    }

    fn safe_area_padding(&self) -> Thickness {
        if self.display_edge_to_edge.get() {
            self.controller().map(|controller| controller.safe_area_padding()).unwrap_or_default()
        } else {
            Thickness::default()
        }
    }

    fn system_bar_color(&self) -> Option<Color> {
        self.base.system_bar_color()
    }

    fn set_system_bar_color(&self, value: Option<Color>) {
        self.base.set_system_bar_color(value);
    }

    fn safe_area_changed(&self, handler: Rc<dyn Fn(&SafeAreaChangedArgs)>) -> Rc<dyn IDisposable> {
        self.base.safe_area_changed(handler)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;
    use crate::view_controller::ViewControllerState;
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::{Rect, Size};

    fn manager() -> (Rc<InsetsManager>, Rc<ViewControllerState>) {
        let controller = ViewControllerState::new(Box::new(|| {}));
        let manager = InsetsManager::new();
        manager.init_with_controller(controller.clone());
        (manager, controller)
    }

    fn lay_out(controller: &ViewControllerState) {
        controller.view_did_layout_subviews(Size::new(393.0, 852.0), Rect::new(0.0, 59.0, 393.0, 759.0));
    }

    #[test]
    fn a_manager_without_a_controller_has_no_padding_and_no_visible_bar() {
        let manager = InsetsManager::new();

        assert_eq!(Thickness::default(), manager.safe_area_padding());
        assert_eq!(Some(false), manager.is_system_bar_visible());
        assert!(manager.display_edge_to_edge_preference());
        manager.set_is_system_bar_visible(Some(false));
        assert_eq!(Some(false), manager.is_system_bar_visible());
    }

    #[test]
    fn the_padding_is_the_one_of_the_controller_while_drawing_edge_to_edge() {
        let _scope = Dispatcher::unit_test_scope();
        let (manager, controller) = manager();
        lay_out(&controller);

        assert_eq!(Thickness::new(0.0, 59.0, 0.0, 34.0), manager.safe_area_padding());

        manager.set_display_edge_to_edge_preference(false);
        assert_eq!(Thickness::default(), manager.safe_area_padding());
        assert!(!manager.displays_edge_to_edge());
    }

    #[test]
    fn a_change_of_the_safe_area_raises_both_events() {
        let _scope = Dispatcher::unit_test_scope();
        let (manager, controller) = manager();
        let log = Rc::new(RefCell::new(Vec::new()));
        let _safe_area = manager.safe_area_changed({
            let log = log.clone();
            Rc::new(move |args: &SafeAreaChangedArgs| log.borrow_mut().push(format!("{:?}", args.safe_area_padding())))
        });
        let _edge_to_edge = manager.display_edge_to_edge_changed({
            let log = log.clone();
            Rc::new(move |value| log.borrow_mut().push(format!("edge to edge {value}")))
        });

        lay_out(&controller);

        assert_eq!(2, log.borrow().len());
        assert_eq!("edge to edge true", log.borrow()[1]);
    }

    #[test]
    fn the_preference_raises_its_event_before_the_safe_area_and_only_on_a_change() {
        let _scope = Dispatcher::unit_test_scope();
        let (manager, _) = manager();
        let log = Rc::new(RefCell::new(Vec::new()));
        let _safe_area = manager.safe_area_changed({
            let log = log.clone();
            Rc::new(move |_: &SafeAreaChangedArgs| log.borrow_mut().push("safe area".to_string()))
        });
        let _edge_to_edge = manager.display_edge_to_edge_changed({
            let log = log.clone();
            Rc::new(move |value| log.borrow_mut().push(format!("edge to edge {value}")))
        });

        manager.set_display_edge_to_edge_preference(true);
        assert!(log.borrow().is_empty());

        manager.set_display_edge_to_edge_preference(false);
        assert_eq!(vec!["edge to edge false".to_string(), "safe area".to_string()], *log.borrow());
    }

    #[test]
    fn the_system_bar_is_the_status_bar_of_the_controller() {
        let (manager, controller) = manager();

        assert_eq!(Some(true), manager.is_system_bar_visible());
        manager.set_is_system_bar_visible(Some(false));
        assert!(controller.prefers_status_bar_hidden());
        assert_eq!(Some(false), manager.is_system_bar_visible());

        // No value shows the bar, as any value but "hidden" does.
        manager.set_is_system_bar_visible(None);
        assert!(!controller.prefers_status_bar_hidden());
    }
}
