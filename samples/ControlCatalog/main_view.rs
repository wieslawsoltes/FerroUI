//! Port of `MainView.xaml.cs`: the class of the document `MainView.xaml`.

use crate::markup::xaml_class;
use crate::view_models::MainWindowViewModel;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::input::{InputElementImpl, TappedEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    StyledElementImplExt, TypeInfo, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Application, Control, ControlImpl, DrawerPage, NavigationPage, PageImpl, SplitViewDisplayMode, TopLevel,
};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

const WIDE_BREAKPOINT: f64 = 1008.0;
const NARROW_BREAKPOINT: f64 = 640.0;

/// The main view of the catalog: a drawer page whose drawer lists the
/// pages and whose content is the navigation page that shows them.
#[repr(C)]
pub struct MainView {
    base: DrawerPage,
    last_applied_mode: Cell<Option<SplitViewDisplayMode>>,
    updating_layout: Cell<bool>,
    size_changed_token: RefCell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(MainView: DrawerPage);
ferro_impl_classes!(MainView: FerroObjectImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, PageImpl);
ferro_class_info!(MainView {
    new: MainView::new,
    markup: {
        properties: [
            ViewModel: Rc<MainWindowViewModel> { get: |this: &Ref<MainView>| this.view_model() },
        ],
        methods: [
            fn FerroIcon_OnTapped(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<TappedEventArgs>() {
                        this.ferro_icon_on_tapped(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(MainView, "/MainView.xaml");

impl StyledElementImpl for MainView {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        MainView::TYPE
    }

    fn on_data_context_changed(this: &Self) {
        Self::parent_on_data_context_changed(this);

        if let Some(view_model) = this.try_view_model() {
            view_model.set_navigator(Some(this.nav_page().as_navigation()));

            view_model.navigate_to_item(&view_model.home_item());
        }
    }
}

impl VisualImpl for MainView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if this.data_context().is_none() {
            return;
        }

        this.update_adaptive_layout();

        let top_level = TopLevel::get_top_level(Some(this)).expect("the view is attached to a top level");

        let insets = top_level.insets_manager();
        if let Some(insets) = insets {
            let view_model = this.view_model();
            // In real life application these events should be unsubscribed to avoid memory leaks.
            view_model.set_safe_area_padding(insets.safe_area_padding());
            {
                let (view_model, insets_for_handler) = (view_model.clone(), insets.clone());
                insets.safe_area_changed(Rc::new(move |_| {
                    view_model.set_safe_area_padding(insets_for_handler.safe_area_padding());
                }));
            }

            view_model.set_display_edge_to_edge(insets.display_edge_to_edge_preference());
            view_model.set_is_system_bar_visible(insets.is_system_bar_visible().unwrap_or(true));

            let weak_view_model = Rc::downgrade(&view_model);
            view_model.property_changed().add(Rc::new(move |property_name: &str| {
                let Some(view_model) = weak_view_model.upgrade() else { return };
                let insets = insets.clone();
                let property_name = property_name.to_string();
                drop(start_async(async move {
                    if property_name == "DisplayEdgeToEdge" {
                        insets.set_display_edge_to_edge_preference(view_model.display_edge_to_edge());
                    } else if property_name == "IsSystemBarVisible" {
                        insets.set_is_system_bar_visible(Some(view_model.is_system_bar_visible()));
                    }

                    // Give the OS some time to apply new values and refresh the view model.
                    delay(Duration::from_millis(100)).await;
                    view_model.set_display_edge_to_edge(insets.display_edge_to_edge_preference());
                    view_model.set_is_system_bar_visible(insets.is_system_bar_visible().unwrap_or(true));
                }));
            }));
        }
    }
}

impl MainView {
    pub fn construct() -> Self {
        Self {
            base: DrawerPage::construct(),
            last_applied_mode: Cell::new(None),
            updating_layout: Cell::new(false),
            size_changed_token: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        {
            let view = this.downgrade();
            this.loaded(move |_, _| {
                if let Some(view) = view.upgrade() {
                    view.main_view_loaded();
                }
            });
        }
        {
            let view = this.downgrade();
            this.unloaded(move |_, _| {
                if let Some(view) = view.upgrade() {
                    view.main_view_unloaded();
                }
            });
        }
        this
    }

    /// The named element `NavPage`.
    fn nav_page(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("NavPage")
    }

    fn main_view_loaded(&self) {
        if self.data_context().is_none() {
            return;
        }

        let view = self.to_ref().downgrade();
        let token = self.size_changed(move |_, e| {
            if let Some(view) = view.upgrade() {
                if e.width_changed() {
                    view.update_adaptive_layout();
                }
            }
        });
        *self.size_changed_token.borrow_mut() = Some(token);
        self.update_adaptive_layout();

        if let Some(app) = Application::current() {
            app.set_requested_theme_variant(Some(ThemeVariant::default()));
        }
    }

    fn main_view_unloaded(&self) {
        if let Some(token) = self.size_changed_token.borrow_mut().take() {
            self.remove_handler(Control::size_changed_event(), token);
        }
        self.last_applied_mode.set(None);
    }

    fn update_adaptive_layout(&self) {
        if self.updating_layout.get() || self.data_context().is_none() {
            return;
        }

        let width = self.bounds().width;
        if width <= 0.0 {
            return;
        }

        let target_mode = if width >= WIDE_BREAKPOINT {
            SplitViewDisplayMode::Inline
        } else if width >= NARROW_BREAKPOINT {
            SplitViewDisplayMode::CompactInline
        } else {
            SplitViewDisplayMode::Overlay
        };

        if self.last_applied_mode.get() == Some(target_mode) {
            return;
        }

        self.updating_layout.set(true);
        self.last_applied_mode.set(Some(target_mode));
        let view_model = self.view_model();
        view_model.set_display_mode(target_mode);

        if target_mode == SplitViewDisplayMode::Inline {
            view_model.set_is_drawer_opened(true);
        } else if target_mode == SplitViewDisplayMode::Overlay {
            view_model.set_is_drawer_opened(false);
        }
        self.updating_layout.set(false);
    }

    fn try_view_model(&self) -> Option<Rc<MainWindowViewModel>> {
        from_markup_value::<Rc<MainWindowViewModel>>(&self.data_context())
    }

    /// `ViewModel`: the data context as the view model of the main window.
    ///
    /// # Panics
    /// Panics if the data context is not that view model (the invalid cast
    /// of the managed original).
    pub(crate) fn view_model(&self) -> Rc<MainWindowViewModel> {
        self.try_view_model().expect("the data context of the main view is a MainWindowViewModel")
    }

    fn ferro_icon_on_tapped(&self, _sender: &Option<BoxedValue>, _e: &TappedEventArgs) {
        let top_level = TopLevel::get_top_level(Some(self)).expect("the view is attached to a top level");
        let launcher = top_level.launcher();
        drop(start_async(async move {
            if let Ok(uri) = Uri::absolute("https://github.com/wieslawsoltes/FerroUI") {
                launcher.launch_uri_async(&uri).await;
            }
        }));
    }
}

#[derive(Default)]
struct DelayState {
    elapsed: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

/// `Task.Delay(duration)`: completes when a dispatcher timer of `duration`
/// has ticked.
struct Delay(Rc<DelayState>);

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.elapsed.get() {
            Poll::Ready(())
        } else {
            *self.0.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn delay(duration: Duration) -> Delay {
    let state = Rc::new(DelayState::default());
    let timer_state = state.clone();
    DispatcherTimer::run_once(
        move || {
            timer_state.elapsed.set(true);
            let waker = timer_state.waker.borrow_mut().take();
            if let Some(waker) = waker {
                waker.wake();
            }
        },
        duration,
        DispatcherPriority::DEFAULT,
    );
    Delay(state)
}
