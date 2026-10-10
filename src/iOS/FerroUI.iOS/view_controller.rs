//! The view controller of a view: the safe area of the view and the
//! status bar.

use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{Rect, Size, Thickness};
use std::cell::Cell;
use std::rc::Rc;

/// The style of the status bar (`UIStatusBarStyle`, with its values).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(isize)]
pub enum StatusBarStyle {
    /// The style the system chooses.
    #[default]
    Default = 0,
    /// Light content, for a dark background.
    LightContent = 1,
    /// Dark content, for a light background.
    DarkContent = 3,
}

/// What a view needs of the view controller it is shown by. Unstable.
pub trait IFerroViewController {
    /// The preferred style of the status bar.
    fn preferred_status_bar_style(&self) -> StatusBarStyle;

    /// Sets the preferred style of the status bar.
    fn set_preferred_status_bar_style(&self, value: StatusBarStyle);

    /// Whether the status bar is hidden.
    fn prefers_status_bar_hidden(&self) -> bool;

    /// Hides or shows the status bar.
    fn set_prefers_status_bar_hidden(&self, value: bool);

    /// The padding that keeps content out of the areas the system covers.
    fn safe_area_padding(&self) -> Thickness;

    /// The safe area padding changed.
    fn safe_area_padding_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}

/// The safe area padding of a view of the given size whose safe area
/// layout guide has the given frame.
pub fn safe_area_padding_of(size: Size, frame: Rect) -> Thickness {
    Thickness::new(frame.left(), frame.top(), size.width - frame.right(), size.height - frame.bottom())
}

/// The state of the default view controller: what the view of the port
/// holds of it. The view controller object of UIKit forwards to it.
pub struct ViewControllerState {
    preferred_status_bar_style: Cell<Option<StatusBarStyle>>,
    prefers_status_bar_hidden: Cell<Option<bool>>,
    safe_area_padding: Cell<Thickness>,
    safe_area_padding_changed: Rc<HandlerList<dyn Fn()>>,
    /// Asks the system to read the status bar properties again.
    needs_status_bar_appearance_update: Box<dyn Fn()>,
}

impl ViewControllerState {
    /// Creates the state; `needs_status_bar_appearance_update` is called
    /// when a property of the status bar was set.
    pub fn new(needs_status_bar_appearance_update: Box<dyn Fn()>) -> Rc<Self> {
        Rc::new(Self {
            preferred_status_bar_style: Cell::new(None),
            prefers_status_bar_hidden: Cell::new(None),
            safe_area_padding: Cell::new(Thickness::default()),
            safe_area_padding_changed: Rc::new(HandlerList::new()),
            needs_status_bar_appearance_update,
        })
    }

    /// The style that was set, if one was.
    pub fn preferred_status_bar_style_override(&self) -> Option<StatusBarStyle> {
        self.preferred_status_bar_style.get()
    }

    /// Whether the status bar is hidden; the first call takes the answer of
    /// the system (`base`) and keeps it.
    pub fn prefers_status_bar_hidden_or(&self, base: impl FnOnce() -> bool) -> bool {
        match self.prefers_status_bar_hidden.get() {
            Some(value) => value,
            None => {
                let value = base();
                self.prefers_status_bar_hidden.set(Some(value));
                value
            }
        }
    }

    /// Called when the view was laid out, with its size and the frame of
    /// its safe area layout guide.
    pub fn view_did_layout_subviews(&self, size: Size, frame: Rect) {
        let safe_area = safe_area_padding_of(size, frame);
        if self.safe_area_padding.get() != safe_area {
            self.safe_area_padding.set(safe_area);
            for (_, handler) in self.safe_area_padding_changed.snapshot().iter() {
                handler();
            }
        }
    }
}

impl IFerroViewController for ViewControllerState {
    fn preferred_status_bar_style(&self) -> StatusBarStyle {
        self.preferred_status_bar_style.get().unwrap_or_default()
    }

    fn set_preferred_status_bar_style(&self, value: StatusBarStyle) {
        self.preferred_status_bar_style.set(Some(value));
        (self.needs_status_bar_appearance_update)();
    }

    fn prefers_status_bar_hidden(&self) -> bool {
        // False is the default on iOS and iPadOS.
        self.prefers_status_bar_hidden.get().unwrap_or(false)
    }

    fn set_prefers_status_bar_hidden(&self, value: bool) {
        self.prefers_status_bar_hidden.set(Some(value));
        (self.needs_status_bar_appearance_update)();
    }

    fn safe_area_padding(&self) -> Thickness {
        self.safe_area_padding.get()
    }

    fn safe_area_padding_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.safe_area_padding_changed.add(handler);
        let handlers = self.safe_area_padding_changed.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
}

#[cfg(target_os = "ios")]
pub use uikit::DefaultFerroViewController;

#[cfg(target_os = "ios")]
mod uikit {
    use super::{IFerroViewController, StatusBarStyle, ViewControllerState};
    use crate::extensions::{to_rect, to_size};
    use ferroui_base::{Rect, Size};
    use objc2::rc::{Retained, Weak};
    use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
    use objc2_foundation::NSObjectProtocol;
    use objc2_ui_kit::{UIStatusBarStyle, UIViewController};
    use std::cell::OnceCell;
    use std::rc::Rc;

    /// The instance variables of the view controller.
    #[derive(Default)]
    pub struct DefaultFerroViewControllerIvars {
        state: OnceCell<Rc<ViewControllerState>>,
    }

    define_class!(
        // SAFETY: `UIViewController` may be subclassed; the overrides call
        // the superclass where UIKit asks for it, and the class does not
        // implement `Drop`.
        #[unsafe(super(UIViewController))]
        #[thread_kind = MainThreadOnly]
        #[name = "DefaultFerroViewController"]
        #[ivars = DefaultFerroViewControllerIvars]
        pub struct DefaultFerroViewController;

        impl DefaultFerroViewController {
            #[unsafe(method(viewDidLayoutSubviews))]
            fn view_did_layout_subviews(&self) {
                // SAFETY: the method of the superclass this one overrides.
                let _: () = unsafe { msg_send![super(self), viewDidLayoutSubviews] };
                let (size, frame) = match self.viewIfLoaded() {
                    Some(view) => (to_size(view.frame().size), to_rect(view.safeAreaLayoutGuide().layoutFrame())),
                    None => (Size::default(), Rect::default()),
                };
                self.state().view_did_layout_subviews(size, frame);
            }

            #[unsafe(method(prefersStatusBarHidden))]
            fn prefers_status_bar_hidden(&self) -> bool {
                self.state().prefers_status_bar_hidden_or(|| {
                    // SAFETY: the method of the superclass this one overrides.
                    unsafe { msg_send![super(self), prefersStatusBarHidden] }
                })
            }

            #[unsafe(method(preferredStatusBarStyle))]
            fn preferred_status_bar_style(&self) -> UIStatusBarStyle {
                // The value stays unset while no style was set, so that
                // "default" is kept and not the style of the application.
                match self.state().preferred_status_bar_style_override() {
                    Some(style) => UIStatusBarStyle(style as isize),
                    // SAFETY: the method of the superclass this one
                    // overrides.
                    None => unsafe { msg_send![super(self), preferredStatusBarStyle] },
                }
            }
        }

        unsafe impl NSObjectProtocol for DefaultFerroViewController {}
    );

    impl DefaultFerroViewController {
        /// Creates a view controller.
        pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
            let this = mtm.alloc::<Self>().set_ivars(DefaultFerroViewControllerIvars::default());
            // SAFETY: `init` of the superclass, on the object that was just
            // allocated and whose instance variables are set.
            let this: Retained<Self> = unsafe { msg_send![super(this), init] };

            let weak = Weak::from_retained(&this);
            let state = ViewControllerState::new(Box::new(move || {
                if let Some(controller) = weak.load() {
                    controller.setNeedsStatusBarAppearanceUpdate();
                }
            }));
            let _ = this.ivars().state.set(state);
            this
        }

        fn state(&self) -> &Rc<ViewControllerState> {
            match self.ivars().state.get() {
                Some(state) => state,
                None => panic!("The view controller was not created by DefaultFerroViewController::new."),
            }
        }

        /// The view controller as the view of the port sees it.
        pub fn controller(&self) -> Rc<dyn IFerroViewController> {
            self.state().clone()
        }
    }

    const _: () = {
        // The styles of the port have the values of UIKit.
        assert!(StatusBarStyle::Default as isize == UIStatusBarStyle::Default.0);
        assert!(StatusBarStyle::LightContent as isize == UIStatusBarStyle::LightContent.0);
        assert!(StatusBarStyle::DarkContent as isize == UIStatusBarStyle::DarkContent.0);
    };
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;
    use std::cell::Cell;

    fn state() -> (Rc<ViewControllerState>, Rc<Cell<u32>>) {
        let updates = Rc::new(Cell::new(0));
        let state = ViewControllerState::new({
            let updates = updates.clone();
            Box::new(move || updates.set(updates.get() + 1))
        });
        (state, updates)
    }

    #[test]
    fn the_safe_area_padding_is_what_the_layout_guide_leaves_free() {
        // A phone in portrait: the status bar above, the home indicator
        // below.
        let padding = safe_area_padding_of(Size::new(393.0, 852.0), Rect::new(0.0, 59.0, 393.0, 759.0));
        assert_eq!(Thickness::new(0.0, 59.0, 0.0, 34.0), padding);

        // In landscape: the sensor housing at the side.
        let padding = safe_area_padding_of(Size::new(852.0, 393.0), Rect::new(59.0, 0.0, 734.0, 372.0));
        assert_eq!(Thickness::new(59.0, 0.0, 59.0, 21.0), padding);
    }

    #[test]
    fn a_layout_raises_the_change_only_when_the_padding_changed() {
        let (state, _) = state();
        let changes = Rc::new(Cell::new(0));
        let subscription = state.safe_area_padding_changed({
            let changes = changes.clone();
            Rc::new(move || changes.set(changes.get() + 1))
        });

        state.view_did_layout_subviews(Size::new(393.0, 852.0), Rect::new(0.0, 59.0, 393.0, 759.0));
        assert_eq!(1, changes.get());
        assert_eq!(Thickness::new(0.0, 59.0, 0.0, 34.0), state.safe_area_padding());

        state.view_did_layout_subviews(Size::new(393.0, 852.0), Rect::new(0.0, 59.0, 393.0, 759.0));
        assert_eq!(1, changes.get());

        subscription.dispose();
        state.view_did_layout_subviews(Size::new(852.0, 393.0), Rect::new(59.0, 0.0, 734.0, 372.0));
        assert_eq!(1, changes.get());
    }

    #[test]
    fn the_status_bar_properties_have_the_defaults_of_the_system_until_set() {
        let (state, updates) = state();

        assert_eq!(StatusBarStyle::Default, state.preferred_status_bar_style());
        assert_eq!(None, state.preferred_status_bar_style_override());
        assert!(!state.prefers_status_bar_hidden());

        state.set_preferred_status_bar_style(StatusBarStyle::LightContent);
        assert_eq!(Some(StatusBarStyle::LightContent), state.preferred_status_bar_style_override());
        state.set_prefers_status_bar_hidden(true);
        assert!(state.prefers_status_bar_hidden());
        assert_eq!(2, updates.get());
    }

    #[test]
    fn the_answer_of_the_system_for_the_hidden_status_bar_is_kept() {
        let (state, _) = state();

        assert!(state.prefers_status_bar_hidden_or(|| true));
        // The answer of the first call is the answer from then on.
        assert!(state.prefers_status_bar_hidden_or(|| false));
        assert!(state.prefers_status_bar_hidden());
    }
}
