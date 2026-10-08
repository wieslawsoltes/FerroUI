//! Port of `Pages/NavigationPage/NavigationPageScrollAwarePage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageScrollAwarePage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::boxed_text;
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, FontWeight, IBrush, SolidColorBrush, TextWrapping, TranslateTransform};
use ferroui_base::reactive::{IDisposable, IObserver};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class_info, instantiate, FerroObject, FerroObjectExtensions, Ref, Thickness, Vector};
use ferroui_controls::{
    BarLayoutBehavior, Border, ContentPage, Control, NavigationPage, ScrollViewer, StackPanel, TextBlock, UserControl,
};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Minimal `IObserver` wrapper.
struct ActionObserver<T> {
    on_next: Box<dyn Fn(T)>,
}

impl<T> ActionObserver<T> {
    fn new(on_next: impl Fn(T) + 'static) -> Self {
        Self { on_next: Box::new(on_next) }
    }
}

impl<T> IObserver<T> for ActionObserver<T> {
    fn on_next(&self, value: T) {
        (self.on_next)(value)
    }
}

#[repr(C)]
pub struct NavigationPageScrollAwarePage {
    base: UserControl,
    scroll_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    scroll_viewer: RefCell<Option<Ref<ScrollViewer>>>,
    last_scroll_y: Cell<f64>,
    current_translate_y: Cell<f64>,
    initialized: Cell<bool>,
}

user_control_class!(NavigationPageScrollAwarePage);
ferro_class_info!(NavigationPageScrollAwarePage { new: NavigationPageScrollAwarePage::new });
xaml_class!(NavigationPageScrollAwarePage, "/Pages/NavigationPage/NavigationPageScrollAwarePage.xaml");

impl NavigationPageScrollAwarePage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            scroll_subscription: RefCell::new(None),
            scroll_viewer: RefCell::new(None),
            last_scroll_y: Cell::new(0.0),
            current_translate_y: Cell::new(0.0),
            initialized: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers of the events of the control itself hold it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        let weak = this.downgrade();
        this.unloaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_unloaded(sender, e);
            }
        });
        this
    }

    fn demo_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    /// `Dispatcher.UIThread.Post(() => AttachScrollWatcher(_scrollViewer), DispatcherPriority.Loaded)`:
    /// the field is read when the action runs.
    fn post_attach_scroll_watcher(&self) {
        let this = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                let scroll_viewer = this.scroll_viewer.borrow().clone();
                this.attach_scroll_watcher(scroll_viewer);
            },
            DispatcherPriority::LOADED,
        );
    }

    /// `async void`.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            if self.scroll_viewer.borrow().is_some() {
                self.post_attach_scroll_watcher();
            }
            return;
        }

        self.initialized.set(true);

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_content(Some(Control::boxed(Self::build_long_content())));
        *self.scroll_viewer.borrow_mut() = Some(scroll_viewer.clone());

        let root_page = ContentPage::new();
        root_page.set_header(Some(boxed_text("Scroll to Hide Bar")));
        root_page.set_content(Some(Control::boxed(scroll_viewer)));
        root_page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        root_page.set_vertical_content_alignment(VerticalAlignment::Stretch);

        NavigationPage::set_bar_layout_behavior(&root_page, Some(BarLayoutBehavior::Overlay));
        let this = self.to_ref();
        drop(start_async(async move {
            if this.demo_nav().push_async_with_transition(root_page, None).await.is_err() {
                return;
            }

            this.post_attach_scroll_watcher();
        }));
    }

    fn on_unloaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.detach_scroll_watcher();
    }

    /// `DemoNav.GetVisualDescendants().OfType<Border>().FirstOrDefault(b => b.Name == "PART_NavigationBar")`.
    fn find_navigation_bar(&self) -> Option<Ref<Border>> {
        self.demo_nav()
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<Border>())
            .find(|border| border.name().as_deref() == Some("PART_NavigationBar"))
    }

    fn attach_scroll_watcher(&self, sv: Option<Ref<ScrollViewer>>) {
        let Some(sv) = sv else {
            return;
        };

        self.detach_scroll_watcher();

        let Some(nav_bar) = self.find_navigation_bar() else {
            return;
        };
        let transform = TranslateTransform::new();
        nav_bar.set_render_transform(Some(transform.clone().into()));

        self.last_scroll_y.set(0.0);
        self.current_translate_y.set(0.0);

        // The scroll viewer is a field of the control and holds the observer: the observer
        // holds the control weakly.
        let weak = self.to_ref().downgrade();
        let object: &FerroObject = &sv;
        let subscription = FerroObjectExtensions::get_observable(object, ScrollViewer::offset_property()).subscribe(
            Rc::new(ActionObserver::new(move |offset: Vector| {
                let Some(this) = weak.upgrade() else {
                    return;
                };

                let y = offset.y;
                let delta = y - this.last_scroll_y.get();
                this.last_scroll_y.set(y);

                let bar_height = this.demo_nav().bar_height();

                if y <= bar_height {
                    this.current_translate_y.set((-y).clamp(-bar_height, 0.0));
                } else if delta > 0.0 {
                    this.current_translate_y.set((this.current_translate_y.get() - delta).clamp(-bar_height, 0.0));
                } else if delta < 0.0 {
                    this.current_translate_y.set((this.current_translate_y.get() - delta).clamp(-bar_height, 0.0));
                }

                transform.set_y(this.current_translate_y.get());
            })),
        );
        *self.scroll_subscription.borrow_mut() = Some(subscription);
    }

    fn detach_scroll_watcher(&self) {
        let subscription = self.scroll_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        // `navBar?.RenderTransform is TranslateTransform t`.
        let transform = self.find_navigation_bar().and_then(|nav_bar| nav_bar.render_transform()).and_then(|transform| {
            transform.as_object().and_then(|object| object.to_ref().cast::<TranslateTransform>())
        });
        if let Some(t) = transform {
            t.set_y(0.0);
        }
    }

    fn build_long_content() -> Ref<Control> {
        let items: [&str; 20] = [
            "Scroll down to hide the navigation bar.",
            "Scroll back up to reveal it again.",
            "The bar tracks the scroll position for a smooth effect.",
            "BarLayoutBehavior.Overlay lets content extend behind the bar.",
            "GetObservable(ScrollViewer.OffsetProperty) drives the scroll detection.",
            "TranslateTransform.Y is clamped between -BarHeight and 0.",
            "Bar always reveals fully when scrolled back to the top.",
            "Cleanup disposes the observable subscription on Unload.",
            "Keep scrolling\u{2026}",
            "Item 10",
            "Item 11",
            "Item 12",
            "Item 13",
            "Item 14",
            "Item 15",
            "Item 16",
            "Item 17",
            "Item 18",
            "Item 19",
            "Item 20, now try scrolling back up!",
        ];

        let stack = StackPanel::new();
        stack.set_spacing(1.0);
        for (i, item) in items.iter().enumerate() {
            let title = TextBlock::new();
            title.set_text(Some(&format!("Item {}", i + 1)));
            title.set_font_size(12.0);
            title.set_font_weight(FontWeight::SemiBold);
            title.set_opacity(0.4);

            let text = TextBlock::new();
            text.set_text(Some(item));
            text.set_font_size(14.0);
            text.set_text_wrapping(TextWrapping::Wrap);

            let child = StackPanel::new();
            child.set_spacing(4.0);
            child.children().add(title);
            child.children().add(text);

            let background: Rc<dyn IBrush> = if i % 2 == 0 {
                SolidColorBrush::with_color(Color::from_argb(12, 128, 128, 128)).into()
            } else {
                Brushes::transparent()
            };

            let border = Border::new();
            border.set_background(Some(background));
            border.set_padding(Thickness::symmetric(16.0, 20.0));
            border.set_margin(if i == 0 { Thickness::new(0.0, 52.0, 0.0, 0.0) } else { Thickness::uniform(0.0) });
            border.set_child(child);
            stack.children().add(border);
        }

        stack.upcast()
    }
}
