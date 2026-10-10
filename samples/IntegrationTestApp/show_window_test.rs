//! Port of `ShowWindowTest.xaml.cs`: the class of the document `ShowWindowTest.xaml` and the
//! border of its content that remembers the size it was measured with.

use crate::markup::xaml_class;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElement, InputElementImpl, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, TappedEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, ElementRef,
    FerroObjectImpl, FerroProperty, PixelSize, Ref, Size, StyledElementImpl, StyledProperty, Thickness, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Border, ContentControlImpl, ControlImpl, Grid, TextBox, TopLevelImpl, TopLevelImplExt, Window, WindowBaseImpl,
    WindowImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// A border that remembers the size it was last measured with.
#[repr(C)]
pub struct MeasureBorder {
    base: Border,
}

ferro_class!(MeasureBorder: Border);
ferro_impl_classes!(
    MeasureBorder: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(MeasureBorder { new: MeasureBorder::new });

impl LayoutableImpl for MeasureBorder {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.set_measured_with(available_size);

        Self::parent_measure_override(this, available_size)
    }
}

ferro_properties! {
    impl MeasureBorder {
        pub fn measured_with_property() -> StyledProperty<Size> {
            FerroProperty::register::<MeasureBorder, _>("MeasuredWith", Size::default())
        }
    }
}

impl MeasureBorder {
    pub fn construct() -> Self {
        Self { base: Border::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn measured_with(&self) -> Size {
        self.get_value(Self::measured_with_property())
    }

    pub fn set_measured_with(&self, value: Size) {
        self.set_value(Self::measured_with_property(), value)
    }
}

#[repr(C)]
pub struct ShowWindowTest {
    base: Window,
    timer: RefCell<Option<Rc<DispatcherTimer>>>,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    order_text_box: RefCell<Option<Ref<TextBox>>>,
    mouse_move_count: Cell<i32>,
    mouse_release_count: Cell<i32>,
    double_click_count: Cell<i32>,
    mouse_down_count: Cell<i32>,
}

ferro_class!(ShowWindowTest: Window);
ferro_impl_classes!(
    ShowWindowTest: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(ShowWindowTest {
    new: ShowWindowTest::new,
    markup: {
        methods: [
            fn AddToWidth_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ShowWindowTest>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.add_to_width_click(&sender, e.as_routed_event_args())
                },
            fn AddToHeight_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ShowWindowTest>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.add_to_height_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ShowWindowTest, "/ShowWindowTest.xaml");

impl TopLevelImpl for ShowWindowTest {
    fn on_opened(this: &Self) {
        Self::parent_on_opened(this);
        // `PlatformImpl!.DesktopScaling`: an opened window has its platform implementation.
        let scaling = this.desktop_scaling();
        this.current_position().set_text(Some(&this.position().to_string()));
        let working_area = this.screens().screen_from_visual(this).map(|screen| screen.working_area().to_string());
        this.current_screen_rect().set_text(Some(&working_area.unwrap_or_default()));
        this.current_scaling().set_text(Some(&scaling.to_string()));

        if let Some(owner) = this.owner() {
            let owner = owner.cast::<Window>().expect("the owner of the window is a window");
            let frame_size = owner.frame_size().expect("the owner has a frame size");
            this.current_owner_rect()
                .set_text(Some(&format!("{}, {}", owner.position(), PixelSize::from_size(frame_size, scaling))));
        }
    }

    fn on_closed(this: &Self) {
        Self::parent_on_closed(this);
        if let Some(timer) = this.timer.borrow().as_ref() {
            timer.stop();
        }
    }
}

impl ShowWindowTest {
    pub fn construct() -> Self {
        Self {
            base: Window::construct(PlatformManager::create_window()),
            timer: RefCell::new(None),
            order_text_box: RefCell::new(None),
            mouse_move_count: Cell::new(0),
            mouse_release_count: Cell::new(0),
            double_click_count: Cell::new(0),
            mouse_down_count: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        // `DataContext = this;` in the managed original, where the collector frees a window
        // that holds itself. Here the window is its data context as an element reference,
        // which holds it weakly: the bindings of the document read the window through it.
        ValueTypes::register_element_ref::<ShowWindowTest>();
        this.set_data_context(Some(Rc::new(ElementRef::of(&this)) as BoxedValue));

        // The window holds the handlers of its events: the handlers hold the window weakly.
        {
            let weak = this.downgrade();
            this.position_changed(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.current_position().set_text(Some(&this.position().to_string()));
                }
            });
        }

        {
            let weak = this.downgrade();
            this.add_handler(InputElement::pointer_moved_event(), move |sender, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_pointer_moved(sender, e);
                }
            });
        }
        {
            let weak = this.downgrade();
            this.add_handler(InputElement::pointer_pressed_event(), move |sender, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_pointer_pressed(sender, e);
                }
            });
        }
        {
            let weak = this.downgrade();
            this.add_handler(InputElement::pointer_released_event(), move |sender, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_pointer_released(sender, e);
                }
            });
        }
        {
            let weak = this.downgrade();
            this.add_handler(InputElement::pointer_exited_event(), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.reset_counters();
                }
            });
        }
        {
            let weak = this.downgrade();
            this.add_handler(InputElement::double_tapped_event(), move |sender, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_double_tapped(sender, e);
                }
            });
        }

        if cfg!(target_os = "macos") {
            *this.order_text_box.borrow_mut() = Some(this.current_order());
            let timer = DispatcherTimer::new();
            timer.set_interval(Duration::from_millis(250));
            let weak = this.downgrade();
            timer.tick(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.timer_on_tick();
                }
            });
            timer.start();
            *this.timer.borrow_mut() = Some(timer);
        }
        this
    }

    fn current_position(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("CurrentPosition")
    }

    fn current_owner_rect(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("CurrentOwnerRect")
    }

    fn current_screen_rect(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("CurrentScreenRect")
    }

    fn current_scaling(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("CurrentScaling")
    }

    fn current_order(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("CurrentOrder")
    }

    fn timer_on_tick(&self) {
        #[cfg(target_os = "macos")]
        {
            let order_text_box = self.order_text_box.borrow().clone().expect("the text box of the order");
            order_text_box.set_text(Some(&crate::MacOSIntegration::get_ordered_index(self).to_string()));
        }
    }

    fn add_to_width_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.set_width(self.bounds().width + 10.0);
    }

    fn add_to_height_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.set_height(self.bounds().height + 10.0);
    }

    fn on_pointer_moved(&self, _sender: &ferroui_base::interactivity::Interactive, _e: &PointerEventArgs) {
        self.mouse_move_count.set(self.mouse_move_count.get() + 1);
        self.update_counter_displays();
    }

    fn on_pointer_pressed(&self, _sender: &ferroui_base::interactivity::Interactive, _e: &PointerPressedEventArgs) {
        self.mouse_down_count.set(self.mouse_down_count.get() + 1);
        self.update_counter_displays();
    }

    fn on_pointer_released(&self, _sender: &ferroui_base::interactivity::Interactive, _e: &PointerReleasedEventArgs) {
        self.mouse_release_count.set(self.mouse_release_count.get() + 1);
        self.update_counter_displays();
    }

    pub fn reset_counters(&self) {
        self.mouse_move_count.set(0);
        self.mouse_release_count.set(0);
        self.double_click_count.set(0);
        self.mouse_down_count.set(0);
        self.update_counter_displays();
    }

    fn on_double_tapped(&self, _sender: &ferroui_base::interactivity::Interactive, _e: &TappedEventArgs) {
        self.double_click_count.set(self.double_click_count.get() + 1);
        self.update_counter_displays();
    }

    fn update_counter_displays(&self) {
        let mouse_move_count_text_box = self.find_control::<TextBox>("MouseMoveCount");
        let mouse_down_count_text_box = self.find_control::<TextBox>("MouseDownCount");
        let mouse_release_count_text_box = self.find_control::<TextBox>("MouseReleaseCount");
        let double_click_count_text_box = self.find_control::<TextBox>("DoubleClickCount");

        if let Some(mouse_move_count_text_box) = mouse_move_count_text_box {
            mouse_move_count_text_box.set_text(Some(&self.mouse_move_count.get().to_string()));
        }

        if let Some(mouse_down_count_text_box) = mouse_down_count_text_box {
            mouse_down_count_text_box.set_text(Some(&self.mouse_down_count.get().to_string()));
        }

        if let Some(mouse_release_count_text_box) = mouse_release_count_text_box {
            mouse_release_count_text_box.set_text(Some(&self.mouse_release_count.get().to_string()));
        }

        if let Some(double_click_count_text_box) = double_click_count_text_box {
            double_click_count_text_box.set_text(Some(&self.double_click_count.get().to_string()));
        }
    }

    pub fn show_title_area_control(&self) {
        let Some(title_area_control) = self.find_control::<Grid>("TitleAreaControl") else {
            return;
        };
        title_area_control.set_is_visible(true);

        let title_bar_height = if self.extend_client_area_title_bar_height_hint() > 0.0 {
            self.extend_client_area_title_bar_height_hint()
        } else {
            30.0
        };
        title_area_control.set_margin(Thickness::new(110.0, -title_bar_height, 8.0, 0.0));
        title_area_control.set_height(title_bar_height);
    }
}
