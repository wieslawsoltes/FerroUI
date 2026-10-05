//! Port of `Pages/DrawerPage/DrawerPageCustomizationPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/DrawerPageCustomizationPage.xaml`.

use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, parse_geometry, value_text};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, Colors, FontWeight, IBrush, SolidColorBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, CornerRadius, FerroObjectImpl, Ref,
    StyledElementImpl, Thickness, VisualImpl,
};
use ferroui_controls::primitives::{RangeBaseValueChangedEventArgs, TemplatedControlImpl};
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    Border, Button, CheckBox, ComboBox, ContentControlImpl, Control, ControlImpl, ControlImplExt, DrawerBehavior,
    DrawerLayoutBehavior, DrawerPage, DrawerPlacement, PathIcon, SelectionChangedEventArgs, StackPanel, TextBlock,
    UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

const ICON_PATHS: [&str; 3] = [
    // 0 - 3 lines (default hamburger)
    "M3 17h18a1 1 0 0 1 .117 1.993L21 19H3a1 1 0 0 1-.117-1.993L3 17h18H3Zm0-6 18-.002a1 1 0 0 1 .117 1.993l-.117.007L3 13a1 1 0 0 1-.117-1.993L3 11l18-.002L3 11Zm0-6h18a1 1 0 0 1 .117 1.993L21 7H3a1 1 0 0 1-.117-1.993L3 5h18H3Z",
    // 1 - 2 lines
    "M3,13H21V11H3M3,6V8H21V6",
    // 2 - 4 squares
    "M3,11H11V3H3M3,21H11V13H3M13,21H21V13H13M13,3V11H21V3",
];

#[repr(C)]
pub struct DrawerPageCustomizationPage {
    base: UserControl,
    is_loaded: Cell<bool>,
}

ferro_class!(DrawerPageCustomizationPage: UserControl);
ferro_impl_classes!(
    DrawerPageCustomizationPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(DrawerPageCustomizationPage {
    new: DrawerPageCustomizationPage::new,
    markup: {
        methods: [
            fn OnToggleDrawer(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_toggle_drawer(&sender, e.as_routed_event_args())
                },
            fn OnBehaviorChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_behavior_changed(&sender, e)
                    }
                },
            fn OnLayoutChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_layout_changed(&sender, e)
                    }
                },
            fn OnPlacementChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_placement_changed(&sender, e)
                    }
                },
            fn OnGestureToggled(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_gesture_toggled(&sender, e.as_routed_event_args())
                },
            fn OnDrawerLengthChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<RangeBaseValueChangedEventArgs>() {
                        this.on_drawer_length_changed(&sender, e)
                    }
                },
            fn OnDrawerBgChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_drawer_bg_changed(&sender, e)
                    }
                },
            fn OnHeaderBgChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_header_bg_changed(&sender, e)
                    }
                },
            fn OnFooterBgChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_footer_bg_changed(&sender, e)
                    }
                },
            fn OnIconChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_icon_changed(&sender, e)
                    }
                },
            fn OnBackdropChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_backdrop_changed(&sender, e)
                    }
                },
            fn OnShowHeaderToggled(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_show_header_toggled(&sender, e.as_routed_event_args())
                },
            fn OnShowFooterToggled(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_show_footer_toggled(&sender, e.as_routed_event_args())
                },
            fn OnHeaderTemplateChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_header_template_changed(&sender, e)
                    }
                },
            fn OnFooterTemplateChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_footer_template_changed(&sender, e)
                    }
                },
            fn OnMenuItemClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_menu_item_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(DrawerPageCustomizationPage, "/Pages/DrawerPage/DrawerPageCustomizationPage.xaml");

impl ControlImpl for DrawerPageCustomizationPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.is_loaded.set(true);
    }
}

/// `new SolidColorBrush(color)` as the value of a brush property.
fn solid(color: Color) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(color).into())
}

fn white() -> Option<Rc<dyn IBrush>> {
    let white: Rc<dyn IBrush> = Brushes::white();
    Some(white)
}

impl DrawerPageCustomizationPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), is_loaded: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        Self::enable_mouse_swipe_gesture(&this.demo_drawer());
        this
    }

    fn combo(&self, name: &str) -> Ref<ComboBox> {
        self.get_control::<ComboBox>(name)
    }

    fn drawer_length_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DrawerLengthText")
    }

    fn show_header_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("ShowHeaderCheck")
    }

    fn show_footer_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("ShowFooterCheck")
    }

    fn demo_drawer(&self) -> Ref<DrawerPage> {
        self.get_control::<DrawerPage>("DemoDrawer")
    }

    fn drawer_header_border(&self) -> Ref<Border> {
        self.get_control::<Border>("DrawerHeaderBorder")
    }

    fn drawer_footer_border(&self) -> Ref<Border> {
        self.get_control::<Border>("DrawerFooterBorder")
    }

    fn detail_title_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DetailTitleText")
    }

    fn on_toggle_drawer(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        let demo_drawer = self.demo_drawer();
        demo_drawer.set_is_open(!demo_drawer.is_open());
    }

    fn on_behavior_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_drawer_behavior(match self.combo("BehaviorCombo").selected_index() {
            0 => DrawerBehavior::Auto,
            1 => DrawerBehavior::Flyout,
            2 => DrawerBehavior::Locked,
            3 => DrawerBehavior::Disabled,
            _ => DrawerBehavior::Auto,
        });
    }

    fn on_layout_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_drawer_layout_behavior(match self.combo("LayoutCombo").selected_index() {
            0 => DrawerLayoutBehavior::Overlay,
            1 => DrawerLayoutBehavior::Split,
            2 => DrawerLayoutBehavior::CompactOverlay,
            3 => DrawerLayoutBehavior::CompactInline,
            _ => DrawerLayoutBehavior::Overlay,
        });
    }

    fn on_placement_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_drawer_placement(match self.combo("PlacementCombo").selected_index() {
            1 => DrawerPlacement::Right,
            2 => DrawerPlacement::Top,
            3 => DrawerPlacement::Bottom,
            _ => DrawerPlacement::Left,
        });
    }

    fn on_gesture_toggled(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        let check = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<CheckBox>());
        if let Some(check) = check {
            self.demo_drawer().set_is_gesture_enabled(check.is_checked() == Some(true));
        }
    }

    fn on_drawer_length_changed(&self, _sender: &Option<BoxedValue>, e: &RangeBaseValueChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_drawer_length(e.new_value());
        self.drawer_length_text().set_text(Some(&(e.new_value() as i32).to_string()));
    }

    fn on_drawer_bg_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_drawer_background(match self.combo("DrawerBgCombo").selected_index() {
            1 => solid(Colors::SLATE_BLUE),
            2 => solid(Colors::DARK_CYAN),
            3 => solid(Colors::DARK_RED),
            4 => solid(Colors::DARK_GREEN),
            _ => None,
        });
    }

    fn on_header_bg_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_drawer_header_background(match self.combo("HeaderBgCombo").selected_index() {
            1 => solid(Colors::DODGER_BLUE),
            2 => solid(Colors::ORANGE),
            3 => solid(Colors::TEAL),
            4 => solid(Colors::PURPLE),
            _ => None,
        });
    }

    fn on_footer_bg_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_drawer_footer_background(match self.combo("FooterBgCombo").selected_index() {
            1 => solid(Colors::DIM_GRAY),
            2 => solid(Colors::DARK_SLATE_BLUE),
            3 => solid(Colors::DARK_OLIVE_GREEN),
            4 => solid(Colors::MAROON),
            _ => None,
        });
    }

    /// # Panics
    /// Panics if the selected index is not the index of an icon (the index out of range
    /// exception of the managed original).
    fn on_icon_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        let index = self.combo("IconCombo").selected_index();
        let path = usize::try_from(index).ok().and_then(|index| ICON_PATHS.get(index));
        let icon = PathIcon::new();
        icon.set_data(parse_geometry(path.expect("the index of an icon")));
        self.demo_drawer().set_drawer_icon(Some(Control::boxed(icon)));
    }

    fn on_backdrop_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_backdrop_brush(match self.combo("BackdropCombo").selected_index() {
            1 => solid(Color::from_argb(102, 0, 0, 0)),
            2 => solid(Color::from_argb(179, 0, 0, 0)),
            3 => solid(Color::from_argb(102, 255, 255, 255)),
            _ => None,
        });
    }

    fn on_show_header_toggled(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        if self.show_header_check().is_checked() == Some(true) {
            self.demo_drawer().set_drawer_header(Some(if self.combo("HeaderTemplateCombo").selected_index() == 0 {
                Control::boxed(self.drawer_header_border())
            } else {
                boxed_text("My Application")
            }));
        } else {
            self.demo_drawer().set_drawer_header(None);
        }
    }

    fn on_show_footer_toggled(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        if self.show_footer_check().is_checked() == Some(true) {
            self.demo_drawer().set_drawer_footer(Some(match self.combo("FooterTemplateCombo").selected_index() {
                1 => boxed_text("v12.0"),
                2 => boxed_text("FerroUI"),
                _ => Control::boxed(self.drawer_footer_border()),
            }));
        } else {
            self.demo_drawer().set_drawer_footer(None);
        }
    }

    fn on_header_template_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }

        let demo_drawer = self.demo_drawer();
        match self.combo("HeaderTemplateCombo").selected_index() {
            1 => {
                demo_drawer.set_drawer_header(Some(boxed_text("My Application")));
                demo_drawer.set_drawer_header_template(Some(FuncDataTemplate::for_type::<String>(
                    |data, _| {
                        let title = TextBlock::new();
                        title.set_text(Some(data.as_str()));
                        title.set_font_size(18.0);
                        title.set_font_weight(FontWeight::SemiBold);
                        title.set_foreground(white());

                        let subtitle = TextBlock::new();
                        subtitle.set_text(Some("Navigation"));
                        subtitle.set_font_size(12.0);
                        subtitle.set_foreground(white());
                        subtitle.set_opacity(0.7);

                        let stack = StackPanel::new();
                        stack.set_spacing(2.0);
                        stack.children().add(title);
                        stack.children().add(subtitle);

                        let border = Border::new();
                        border.set_padding(Thickness::uniform(16.0));
                        border.set_child(stack);
                        Some(border.upcast())
                    },
                    false,
                )));
            }
            2 => {
                demo_drawer.set_drawer_header(Some(boxed_text("My Application")));
                demo_drawer.set_drawer_header_template(Some(FuncDataTemplate::for_type::<String>(
                    |data, _| {
                        let initial = match data.chars().next() {
                            Some(first) => first.to_uppercase().to_string(),
                            None => String::from("?"),
                        };

                        let initial_text = TextBlock::new();
                        initial_text.set_text(Some(&initial));
                        initial_text.set_font_size(18.0);
                        initial_text.set_font_weight(FontWeight::Bold);
                        initial_text.set_foreground(white());
                        initial_text.set_horizontal_alignment(HorizontalAlignment::Center);
                        initial_text.set_vertical_alignment(VerticalAlignment::Center);

                        let avatar = Border::new();
                        avatar.set_width(40.0);
                        avatar.set_height(40.0);
                        avatar.set_corner_radius(CornerRadius::uniform(20.0));
                        avatar.set_background(solid(parse_color("#1976D2")));
                        avatar.set_child(initial_text);

                        let label = TextBlock::new();
                        label.set_text(Some(data.as_str()));
                        label.set_font_size(14.0);
                        label.set_font_weight(FontWeight::SemiBold);
                        label.set_vertical_alignment(VerticalAlignment::Center);

                        let row = StackPanel::new();
                        row.set_orientation(Orientation::Horizontal);
                        row.set_spacing(10.0);
                        row.children().add(avatar);
                        row.children().add(label);

                        let border = Border::new();
                        border.set_padding(Thickness::uniform(12.0));
                        border.set_child(row);
                        Some(border.upcast())
                    },
                    false,
                )));
            }
            _ => {
                demo_drawer.set_drawer_header(Some(Control::boxed(self.drawer_header_border())));
                demo_drawer.set_drawer_header_template(None);
            }
        }
    }

    fn on_footer_template_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }

        let demo_drawer = self.demo_drawer();
        match self.combo("FooterTemplateCombo").selected_index() {
            1 => {
                demo_drawer.set_drawer_footer(Some(boxed_text("v12.0")));
                demo_drawer.set_drawer_footer_template(Some(FuncDataTemplate::for_type::<String>(
                    |data, _| {
                        let text = TextBlock::new();
                        text.set_text(Some(data.as_str()));
                        text.set_font_size(11.0);
                        text.set_foreground(white());
                        text.set_font_weight(FontWeight::SemiBold);

                        let badge = Border::new();
                        badge.set_padding(Thickness::symmetric(8.0, 4.0));
                        badge.set_corner_radius(CornerRadius::uniform(4.0));
                        badge.set_background(solid(parse_color("#1976D2")));
                        badge.set_child(text);

                        let border = Border::new();
                        border.set_padding(Thickness::symmetric(12.0, 8.0));
                        border.set_child(badge);
                        Some(border.upcast())
                    },
                    false,
                )));
            }
            2 => {
                demo_drawer.set_drawer_footer(Some(boxed_text("FerroUI")));
                demo_drawer.set_drawer_footer_template(Some(FuncDataTemplate::for_type::<String>(
                    |data, _| {
                        let icon = PathIcon::new();
                        icon.set_width(14.0);
                        icon.set_height(14.0);
                        icon.set_data(parse_geometry(
                            "M13,9H11V7H13M13,17H11V11H13M12,2A10,10 0 0,0 2,12A10,10 0 0,0 12,22A10,10 0 0,0 22,12A10,10 0 0,0 12,2Z",
                        ));
                        icon.set_opacity(0.5);

                        let label = TextBlock::new();
                        label.set_text(Some(data.as_str()));
                        label.set_font_size(12.0);
                        label.set_opacity(0.6);
                        label.set_vertical_alignment(VerticalAlignment::Center);

                        let row = StackPanel::new();
                        row.set_orientation(Orientation::Horizontal);
                        row.set_spacing(6.0);
                        row.children().add(icon);
                        row.children().add(label);

                        let border = Border::new();
                        border.set_padding(Thickness::symmetric(14.0, 10.0));
                        border.set_child(row);
                        Some(border.upcast())
                    },
                    false,
                )));
            }
            _ => {
                demo_drawer.set_drawer_footer(Some(Control::boxed(self.drawer_footer_border())));
                demo_drawer.set_drawer_footer_template(None);
            }
        }
    }

    fn on_menu_item_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        let button = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Button>());
        let Some(button) = button else {
            return;
        };
        let item = value_text(&button.tag()).unwrap_or_else(|| String::from("Home"));
        self.detail_title_text().set_text(Some(&item));
        let demo_drawer = self.demo_drawer();
        if demo_drawer.drawer_behavior() != DrawerBehavior::Locked {
            demo_drawer.set_is_open(false);
        }
    }

    fn enable_mouse_swipe_gesture(control: &Control) {
        let recognizer = control
            .gesture_recognizers()
            .to_vec()
            .into_iter()
            .find_map(|recognizer| recognizer.cast::<SwipeGestureRecognizer>());
        if let Some(recognizer) = recognizer {
            recognizer.set_is_mouse_enabled(true);
        }
    }
}
