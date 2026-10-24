//! Port of `Diagnostics/DevTools.cs`: attaches DevTools to a top level or an application and
//! opens its window.

use super::controls;
use super::views::MainWindow;
use super::{
    ClassicDesktopStyleApplicationLifetimeTopLevelGroup, DevToolsOptions, IDevToolsTopLevelGroup, KeyGestureExtesions,
    SingleViewTopLevelGroup,
};
use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType};
use ferroui_base::input::{IInputManager, InputElement, KeyEventArgs, KeyGesture};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable, ObservableExt, SerialDisposable};
use ferroui_base::{BoxedValue, FerroObject, Ref, Visual};
use ferroui_controls::primitives::PopupRoot;
use ferroui_controls::{Control, Design, TopLevel, Window};
use std::cell::RefCell;
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    // `s_open`: the DevTools windows that are open, by the group of top levels they inspect.
    static S_OPEN: RefCell<Vec<(Rc<dyn IDevToolsTopLevelGroup>, Ref<MainWindow>)>> = RefCell::new(Vec::new());
}

/// Attaches DevTools and opens its window.
pub struct DevTools;

impl DevTools {
    /// C# `Attach(TopLevel root, KeyGesture gesture)`: DevTools opens on `gesture` in `root`.
    pub fn attach_with_gesture(root: &TopLevel, gesture: KeyGesture) -> Rc<dyn IDisposable> {
        Self::attach(root, DevToolsOptions::new().with_gesture(gesture))
    }

    /// C# `Attach(TopLevel root, DevToolsOptions options)`: DevTools opens on the gesture of
    /// `options` in `root`. Disposing the result detaches it.
    pub fn attach(root: &TopLevel, options: Rc<DevToolsOptions>) -> Rc<dyn IDisposable> {
        // The handler is added to the root itself: it holds the root weakly.
        let weak_root = root.to_ref().downgrade();

        root.add_disposable_handler_with(
            InputElement::key_down_event(),
            move |_, e: &KeyEventArgs| {
                if options.gesture().matches(Some(e)) {
                    if let Some(root) = weak_root.upgrade() {
                        Self::open(&root, &options);
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        )
    }

    /// C# `Open(TopLevel root, DevToolsOptions options)`: opens DevTools for `root`. Disposing
    /// the result closes its window.
    pub fn open(root: &TopLevel, options: &Rc<DevToolsOptions>) -> Rc<dyn IDisposable> {
        Self::open_core(
            Rc::new(SingleViewTopLevelGroup::new(root.to_ref())),
            options,
            root.to_ref().cast::<Window>(),
            None,
        )
    }

    /// C# `Open(IDevToolsTopLevelGroup group, DevToolsOptions options)`.
    pub fn open_group(group: Rc<dyn IDevToolsTopLevelGroup>, options: &Rc<DevToolsOptions>) -> Rc<dyn IDisposable> {
        Self::open_core(group, options, None, None)
    }

    /// C# `Attach(Application application, DevToolsOptions options)`: DevTools opens on the
    /// gesture of `options` in any window of `application`. Disposing the result detaches it and
    /// closes the window it opened.
    ///
    /// # Panics
    /// When the application does not have a classic desktop lifetime.
    pub fn attach_application(
        application: &ferroui_controls::Application,
        options: Rc<DevToolsOptions>,
    ) -> Rc<dyn IDisposable> {
        let opened_disposable = Rc::new(SerialDisposable::new());
        let result = Rc::new(CompositeDisposable::with_capacity(2));
        result.add(opened_disposable.clone());

        // Skip if call on Design Mode
        if !Design::is_design_mode() {
            let life_time = application
                .application_lifetime()
                .filter(|lifetime| lifetime.as_classic_desktop_style_application_lifetime().is_some());

            let Some(life_time) = life_time else {
                panic!(
                    "DevTools can only attach to applications that support IClassicDesktopStyleApplicationLifetime. (Parameter 'application')"
                );
            };

            if let Some(input_manager) = application.input_manager() {
                let input_manager: Rc<dyn IInputManager> = input_manager;
                // The application owns its input manager: the handler holds the application
                // weakly.
                let weak_application = application.to_ref().downgrade();
                result.add(input_manager.pre_process().subscribe_fn(move |e: Rc<dyn IRawInputEventArgs>| {
                    let owner = life_time
                        .as_classic_desktop_style_application_lifetime()
                        .and_then(|lifetime| lifetime.main_window());

                    if let Some(key_event_args) = e.downcast_ref::<RawKeyEventArgs>() {
                        if key_event_args.type_() == RawKeyEventType::KeyUp
                            && options.gesture().matches_raw(key_event_args)
                        {
                            opened_disposable.set(Some(Self::open_core(
                                Rc::new(ClassicDesktopStyleApplicationLifetimeTopLevelGroup::new(life_time.clone())),
                                &options,
                                owner,
                                weak_application.upgrade(),
                            )));
                            e.set_handled(true);
                        }
                    }
                }));
            }
        }

        result
    }

    fn open_core(
        top_level_group: Rc<dyn IDevToolsTopLevelGroup>,
        options: &Rc<DevToolsOptions>,
        owner: Option<Ref<Window>>,
        app: Option<Ref<ferroui_controls::Application>>,
    ) -> Rc<dyn IDisposable> {
        let focused_control = owner
            .as_ref()
            .and_then(|owner| owner.focus_manager().get_focused_element())
            .and_then(|element| element.cast::<Control>());

        let group = top_level_group.as_any();
        let root: Ref<FerroObject> = if group.is::<ClassicDesktopStyleApplicationLifetimeTopLevelGroup>() {
            let application = app
                .or_else(ferroui_controls::Application::current)
                .expect("an application is running when DevTools opens for its windows");
            controls::Application::new(top_level_group.clone(), application).upcast()
        } else if group.is::<SingleViewTopLevelGroup>() {
            top_level_group.items().into_iter().next().expect("a single view group has its top level").upcast()
        } else {
            controls::TopLevelGroup::new(top_level_group.clone()).upcast()
        };

        // If single static toplevel is already visible in another devtools window, focus it.
        let main_window = S_OPEN.with(|open| {
            open.borrow().iter().find(|(group, _)| **group == *top_level_group).map(|(_, window)| window.clone())
        });
        if let Some(main_window) = main_window {
            main_window.activate();
            main_window.selected_control(focused_control);
            return Disposable::empty();
        }

        let items = top_level_group.items();
        if items.len() == 1 && top_level_group.observable_items().is_none() {
            let single_top_level = items[0].clone();

            let main_window = S_OPEN.with(|open| {
                open.borrow()
                    .iter()
                    .find(|(group, _)| group.items().contains(&single_top_level))
                    .map(|(_, window)| window.clone())
            });
            if let Some(main_window) = main_window {
                main_window.activate();
                main_window.selected_control(focused_control);
                return Disposable::empty();
            }
        }

        let window = MainWindow::new();
        window.set_root(Some(root));
        window.set_width(options.size().width);
        window.set_height(options.size().height);
        window.set_tag(Some(Rc::new(top_level_group.clone()) as BoxedValue));
        window.set_options(options);
        window.selected_control(focused_control);

        // `window.Closed += DevToolsClosed`; the handler removes itself.
        let closed_subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let handler_subscription = closed_subscription.clone();
        let weak_window = window.downgrade();
        let closed = window.closed(move || {
            if let Some(window) = weak_window.upgrade() {
                Self::dev_tools_closed(&window, &handler_subscription);
            }
        });
        *closed_subscription.borrow_mut() = Some(closed);

        S_OPEN.with(|open| open.borrow_mut().push((top_level_group, window.clone())));
        match owner.filter(|_| options.show_as_child_window()) {
            Some(owner) => window.show_with_owner(&owner),
            None => window.show(),
        }

        Disposable::create(move || window.close())
    }

    fn dev_tools_closed(window: &Ref<MainWindow>, subscription: &RefCell<Option<Rc<dyn IDisposable>>>) {
        let own = subscription.borrow_mut().take();
        if let Some(own) = own {
            own.dispose();
        }

        let group = window.tag().and_then(|tag| tag.downcast_ref::<Rc<dyn IDevToolsTopLevelGroup>>().cloned());
        if let Some(group) = group {
            S_OPEN.with(|open| open.borrow_mut().retain(|(open_group, _)| **open_group != *group));
        }
    }

    /// C# `DoesBelongToDevTool(this Visual v)`: whether `v` is in a DevTools window, or in a
    /// popup of one. A visual that is in no top level belongs to DevTools, as upstream.
    pub fn does_belong_to_dev_tool(v: &Visual) -> bool {
        let mut top_level = TopLevel::get_top_level(Some(v));

        while let Some(current) = top_level {
            if current.is::<MainWindow>() {
                break;
            }
            match current.cast::<PopupRoot>() {
                Some(pr) => top_level = pr.parent_top_level(),
                None => return false,
            }
        }
        true
    }
}

/// The extension method of `DevTools` on visuals.
pub trait DevToolsVisualExtensions {
    /// Whether the visual is in a DevTools window, or in a popup of one
    /// ([`DevTools::does_belong_to_dev_tool`]).
    fn does_belong_to_dev_tool(&self) -> bool;
}

impl DevToolsVisualExtensions for Visual {
    fn does_belong_to_dev_tool(&self) -> bool {
        DevTools::does_belong_to_dev_tool(self)
    }
}
