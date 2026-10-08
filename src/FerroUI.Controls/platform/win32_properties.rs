use super::IWin32OptionsTopLevelImpl;
use crate::{TopLevel, Window};
use ferroui_base::{
    ferro_properties, ferro_static_type, AttachedProperty, FerroProperty, FerroPropertyChangedEventArgs,
    StyledPropertyOptions, Visual,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Set of Win32 specific properties and events that allow deeper customization of the
/// application per platform.
pub struct Win32Properties;

ferro_static_type!(Win32Properties);

ferro_properties! {
    impl Win32Properties {
        /// Defines the `NonClientHitTestResult` attached property: the non-client hit test
        /// result of a visual.
        pub fn non_client_hit_test_result_property() -> AttachedProperty<Win32HitTestValue> {
            FerroProperty::register_attached_with::<Win32Properties, Visual, _>(
                "NonClientHitTestResult",
                StyledPropertyOptions::new(Win32HitTestValue::Client).inherits(true),
            )
        }

        /// Defines the `WindowCornerPreference` attached property: the rounded corner
        /// preference of a window.
        // The validation of the original (`ValidateWindowCornerPreference`) rejects a number
        // outside the range of the enumeration; a `WindowCornerPreference` cannot hold one.
        pub fn window_corner_preference_property() -> AttachedProperty<WindowCornerPreference> {
            FerroProperty::register_attached::<Win32Properties, Window, _>(
                "WindowCornerPreference",
                WindowCornerPreference::Default,
            )
        }
    }
}

/// The function type of [`CustomWindowStylesCallback`].
type WindowStyles = dyn Fn(u32, u32) -> (u32, u32);

/// The function type of [`CustomWndProcHookCallback`].
type WndProcHook = dyn Fn(isize, u32, isize, isize, &mut bool) -> isize;

/// The invocation lists of the combined callbacks that are in use: a combined callback (held
/// weakly) and the callbacks it invokes, in order.
type InvocationLists<F: ?Sized> = RefCell<Vec<(Weak<F>, Vec<Rc<F>>)>>;

thread_local! {
    static WINDOW_STYLES_LISTS: InvocationLists<WindowStyles> = const { RefCell::new(Vec::new()) };
    static WND_PROC_HOOK_LISTS: InvocationLists<WndProcHook> = const { RefCell::new(Vec::new()) };
}

/// The address of a callback: its identity.
fn callback_key<F: ?Sized>(callback: &Rc<F>) -> usize {
    Rc::as_ptr(callback) as *const () as usize
}

/// Takes the invocation list of `current` out of `lists`: the callbacks a combined callback
/// invokes, or the callback itself when it is not a combined one. Lists of callbacks that are
/// no longer in use are dropped.
fn take_invocation_list<F: ?Sized>(lists: &InvocationLists<F>, current: &Rc<F>) -> Vec<Rc<F>> {
    let mut lists = lists.borrow_mut();
    lists.retain(|(combined, _)| combined.strong_count() > 0);
    let index = lists.iter().position(|(combined, _)| {
        combined.upgrade().is_some_and(|combined| callback_key(&combined) == callback_key(current))
    });
    match index {
        Some(index) => lists.remove(index).1,
        None => vec![current.clone()],
    }
}

/// The callback that stands for an invocation list: nothing for an empty list, the callback
/// itself for a list of one, otherwise a new combined callback, whose list is recorded.
fn from_invocation_list<F: ?Sized>(
    lists: &InvocationLists<F>,
    mut parts: Vec<Rc<F>>,
    combine: fn(Vec<Rc<F>>) -> Rc<F>,
) -> Option<Rc<F>> {
    match parts.len() {
        0 => None,
        1 => parts.pop(),
        _ => {
            let combined = combine(parts.clone());
            lists.borrow_mut().push((Rc::downgrade(&combined), parts));
            Some(combined)
        }
    }
}

/// `current += callback` for a delegate.
fn add_callback<F: ?Sized>(
    lists: &InvocationLists<F>,
    current: Option<Rc<F>>,
    callback: Rc<F>,
    combine: fn(Vec<Rc<F>>) -> Rc<F>,
) -> Option<Rc<F>> {
    let mut parts = match &current {
        Some(current) => take_invocation_list(lists, current),
        None => Vec::new(),
    };
    parts.push(callback);
    from_invocation_list(lists, parts, combine)
}

/// `current -= callback` for a delegate: removes the last occurrence of `callback` from the
/// invocation list of `current`.
fn remove_callback<F: ?Sized>(
    lists: &InvocationLists<F>,
    current: Option<Rc<F>>,
    callback: &Rc<F>,
    combine: fn(Vec<Rc<F>>) -> Rc<F>,
) -> Option<Rc<F>> {
    let current = current?;
    let mut parts = take_invocation_list(lists, &current);
    if let Some(index) = parts.iter().rposition(|part| callback_key(part) == callback_key(callback)) {
        parts.remove(index);
    }
    from_invocation_list(lists, parts, combine)
}

/// A callback that invokes every callback of `parts` with its arguments and returns the
/// result of the last one, as a combined delegate does.
fn combine_window_styles(parts: Vec<Rc<WindowStyles>>) -> Rc<WindowStyles> {
    Rc::new(move |style: u32, ex_style: u32| -> (u32, u32) {
        let mut result = (style, ex_style);
        for part in &parts {
            result = part(style, ex_style);
        }
        result
    })
}

/// A callback that invokes every callback of `parts` with its arguments and returns the
/// result of the last one, as a combined delegate does.
fn combine_wnd_proc_hooks(parts: Vec<Rc<WndProcHook>>) -> Rc<WndProcHook> {
    Rc::new(move |hwnd: isize, msg: u32, w_param: isize, l_param: isize, handled: &mut bool| -> isize {
        let mut result = 0;
        for part in &parts {
            result = part(hwnd, msg, w_param, l_param, handled);
        }
        result
    })
}

impl Win32Properties {
    fn static_constructor() {
        Self::window_corner_preference_property().changed().add_class_handler::<Window>(
            |window: &Window, e: &FerroPropertyChangedEventArgs<'_>| {
                let Some(platform_impl) = window.platform_impl() else { return };
                if let Some(top_level_impl) = platform_impl.as_win32_options_top_level_impl() {
                    top_level_impl.set_window_corner_preference(e.get_new_value::<WindowCornerPreference>());
                }
            },
        );
    }

    /// Runs `f` with the Win32 options of the platform implementation of `top_level`, if it
    /// has them.
    fn with_options(top_level: &TopLevel, f: impl FnOnce(&dyn IWin32OptionsTopLevelImpl)) {
        let Some(platform_impl) = top_level.platform_impl() else { return };
        if let Some(top_level_impl) = platform_impl.as_win32_options_top_level_impl() {
            f(top_level_impl);
        }
    }

    /// Adds a callback to set the window's style.
    pub fn add_window_styles_callback(top_level: &TopLevel, callback: Option<CustomWindowStylesCallback>) {
        Self::with_options(top_level, |top_level_impl| {
            // Adding null leaves the callbacks as they are.
            let Some(callback) = callback else { return };
            let combined = WINDOW_STYLES_LISTS.with(|lists| {
                add_callback(lists, top_level_impl.window_styles_callback(), callback, combine_window_styles)
            });
            top_level_impl.set_window_styles_callback(combined);
        });
    }

    /// Removes a callback that sets the window's style.
    pub fn remove_window_styles_callback(top_level: &TopLevel, callback: Option<CustomWindowStylesCallback>) {
        Self::with_options(top_level, |top_level_impl| {
            // Removing null leaves the callbacks as they are.
            let Some(callback) = callback else { return };
            let combined = WINDOW_STYLES_LISTS.with(|lists| {
                remove_callback(lists, top_level_impl.window_styles_callback(), &callback, combine_window_styles)
            });
            top_level_impl.set_window_styles_callback(combined);
        });
    }

    /// Adds a custom callback for the window procedure.
    pub fn add_wnd_proc_hook_callback(top_level: &TopLevel, callback: Option<CustomWndProcHookCallback>) {
        Self::with_options(top_level, |top_level_impl| {
            // Adding null leaves the callbacks as they are.
            let Some(callback) = callback else { return };
            let combined = WND_PROC_HOOK_LISTS.with(|lists| {
                add_callback(lists, top_level_impl.wnd_proc_hook_callback(), callback, combine_wnd_proc_hooks)
            });
            top_level_impl.set_wnd_proc_hook_callback(combined);
        });
    }

    /// Removes a custom callback for the window procedure.
    pub fn remove_wnd_proc_hook_callback(top_level: &TopLevel, callback: Option<CustomWndProcHookCallback>) {
        Self::with_options(top_level, |top_level_impl| {
            // Removing null leaves the callbacks as they are.
            let Some(callback) = callback else { return };
            let combined = WND_PROC_HOOK_LISTS.with(|lists| {
                remove_callback(lists, top_level_impl.wnd_proc_hook_callback(), &callback, combine_wnd_proc_hooks)
            });
            top_level_impl.set_wnd_proc_hook_callback(combined);
        });
    }

    /// Sets the non-client hit test result for a visual.
    pub fn set_non_client_hit_test_result(obj: &Visual, value: Win32HitTestValue) {
        obj.set_value(Self::non_client_hit_test_result_property(), value)
    }

    /// Gets the non-client hit test result for a visual.
    pub fn get_non_client_hit_test_result(obj: &Visual) -> Win32HitTestValue {
        obj.get_value(Self::non_client_hit_test_result_property())
    }

    /// Sets a hint for the corner preference of a window.
    pub fn set_window_corner_preference(window: &Window, value: WindowCornerPreference) {
        window.set_value(Self::window_corner_preference_property(), value)
    }

    /// Gets the corner preference hint of a window.
    pub fn get_window_corner_preference(window: &Window) -> WindowCornerPreference {
        window.get_value(Self::window_corner_preference_property())
    }
}

/// A callback that customizes the window styles: receives the style and
/// the extended style and returns the pair to use.
pub type CustomWindowStylesCallback = Rc<dyn Fn(u32, u32) -> (u32, u32)>;

/// A callback that hooks the window procedure: receives the window handle,
/// the message and its two parameters, sets the flag when it handled the
/// message and returns the result of the message.
pub type CustomWndProcHookCallback = Rc<dyn Fn(isize, u32, isize, isize, &mut bool) -> isize>;

/// Represents a hit test value for a visual.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Win32HitTestValue {
    #[default]
    Nowhere = 0,
    Client = 1,
    Caption = 2,
    MinButton = 8,
    MaxButton = 9,
    Left = 10,
    Right = 11,
    Top = 12,
    TopLeft = 13,
    TopRight = 14,
    Bottom = 15,
    BottomLeft = 16,
    BottomRight = 17,
    Close = 20,
}

/// Represents the rounded corner preference for a window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WindowCornerPreference {
    /// Let the system decide when to round window corners.
    #[default]
    Default = 0,

    /// Never round window corners.
    DoNotRound = 1,

    /// Round the corners, if appropriate.
    Round = 2,

    /// Round the corners if appropriate, with a small radius.
    RoundSmall = 3,
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream project has no tests of this class.
    use super::*;
    use crate::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
    use crate::Border;
    use std::cell::Cell;

    #[test]
    fn the_attached_properties_have_the_defaults_of_the_class() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window = Window::with_impl(MockWindowingPlatform::create_window_mock());
        let child = Border::new();

        assert_eq!(Win32HitTestValue::Client, Win32Properties::get_non_client_hit_test_result(&child));
        assert_eq!(WindowCornerPreference::Default, Win32Properties::get_window_corner_preference(&window));
        assert!(Win32Properties::non_client_hit_test_result_property().inherits());

        Win32Properties::set_non_client_hit_test_result(&child, Win32HitTestValue::Caption);
        Win32Properties::set_window_corner_preference(&window, WindowCornerPreference::RoundSmall);
        assert_eq!(Win32HitTestValue::Caption, Win32Properties::get_non_client_hit_test_result(&child));
        assert_eq!(WindowCornerPreference::RoundSmall, Win32Properties::get_window_corner_preference(&window));
    }

    #[test]
    fn callbacks_combine_and_separate_as_delegates_do() {
        let lists: InvocationLists<WindowStyles> = RefCell::new(Vec::new());
        let calls = Rc::new(Cell::new(0));
        let counting = |result: (u32, u32)| -> CustomWindowStylesCallback {
            let calls = calls.clone();
            Rc::new(move |_, _| {
                calls.set(calls.get() + 1);
                result
            })
        };
        let (first, second, third) = (counting((1, 1)), counting((2, 2)), counting((3, 3)));

        // One callback is itself.
        let current = add_callback(&lists, None, first.clone(), combine_window_styles).expect("a callback");
        assert_eq!(callback_key(&first), callback_key(&current));
        assert!(lists.borrow().is_empty());

        // Several are invoked in order; the result is the one of the last.
        let current = add_callback(&lists, Some(current), second.clone(), combine_window_styles);
        let current = add_callback(&lists, current, third.clone(), combine_window_styles).expect("a callback");
        assert_eq!((3, 3), current(0, 0));
        assert_eq!(3, calls.get());
        assert_eq!(1, lists.borrow().len());

        // Removing one leaves the others; removing a callback that is not in the list changes nothing.
        let current = remove_callback(&lists, Some(current), &third, combine_window_styles).expect("a callback");
        assert_eq!((2, 2), current(0, 0));
        assert_eq!(5, calls.get());
        let current = remove_callback(&lists, Some(current), &third, combine_window_styles).expect("a callback");
        assert_eq!((2, 2), current(0, 0));

        let current = remove_callback(&lists, Some(current), &first, combine_window_styles).expect("a callback");
        assert_eq!(callback_key(&second), callback_key(&current));
        assert!(remove_callback(&lists, Some(current), &second, combine_window_styles).is_none());
        assert!(remove_callback(&lists, None, &second, combine_window_styles).is_none());
        assert!(lists.borrow().is_empty());
    }

    #[test]
    fn window_procedure_hooks_share_the_handled_flag() {
        let lists: InvocationLists<WndProcHook> = RefCell::new(Vec::new());
        let first: CustomWndProcHookCallback = Rc::new(|_, _, _, _, handled: &mut bool| {
            *handled = true;
            1
        });
        let second: CustomWndProcHookCallback = Rc::new(|_, msg, _, _, _: &mut bool| msg as isize);

        let current = add_callback(&lists, None, first, combine_wnd_proc_hooks);
        let current = add_callback(&lists, current, second, combine_wnd_proc_hooks).expect("a callback");
        let mut handled = false;
        assert_eq!(7, current(0, 7, 0, 0, &mut handled));
        assert!(handled);
    }
}
