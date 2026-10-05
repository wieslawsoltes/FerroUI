//! Port of `Pages/TabbedPage/TabbedPageCustomizationPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageCustomizationPage.xaml`.

use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{parse_color, parse_geometry, value_text};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::media::{Color, IBrush, SolidColorBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    StyledElementImplExt, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    CheckBox, ComboBox, ContentControlImpl, ContentPage, Control, ControlImpl, PathIcon, SelectionChangedEventArgs,
    TabPlacement, TabbedPage, UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

const HOME_GEOMETRY: &str = "M10,20V14H14V20H19V12H22L12,3L2,12H5V20H10Z";

const SEARCH_GEOMETRY: &str = "M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z";

const SETTINGS_GEOMETRY: &str = "M12,15.5A3.5,3.5 0 0,1 8.5,12A3.5,3.5 0 0,1 12,8.5A3.5,3.5 0 0,1 15.5,12A3.5,3.5 0 0,1 12,15.5M19.43,12.97C19.47,12.65 19.5,12.33 19.5,12C19.5,11.67 19.47,11.34 19.43,11L21.54,9.37C21.73,9.22 21.78,8.95 21.66,8.73L19.66,5.27C19.54,5.05 19.27,4.96 19.05,5.05L16.56,6.05C16.04,5.66 15.5,5.32 14.87,5.07L14.5,2.42C14.46,2.18 14.25,2 14,2H10C9.75,2 9.54,2.18 9.5,2.42L9.13,5.07C8.5,5.32 7.96,5.66 7.44,6.05L4.95,5.05C4.73,4.96 4.46,5.05 4.34,5.27L2.34,8.73C2.21,8.95 2.27,9.22 2.46,9.37L4.57,11C4.53,11.34 4.5,11.67 4.5,12C4.5,12.33 4.53,12.65 4.57,12.97L2.46,14.63C2.27,14.78 2.21,15.05 2.34,15.27L4.34,18.73C4.46,18.95 4.73,19.04 4.95,18.95L7.44,17.94C7.96,18.34 8.5,18.68 9.13,18.93L9.5,21.58C9.54,21.82 9.75,22 10,22H14C14.25,22 14.46,21.82 14.5,21.58L14.87,18.93C15.5,18.67 16.04,18.34 16.56,17.94L19.05,18.95C19.27,19.04 19.54,18.95 19.66,18.73L21.66,15.27C21.78,15.05 21.73,14.78 21.54,14.63L19.43,12.97Z";

/// A brush of a color: an entry of the brush tables of the page.
fn brush(color: Color) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(color).into())
}

/// The table `BarBackgrounds`.
fn bar_backgrounds() -> [Option<Rc<dyn IBrush>>; 5] {
    [
        None,
        brush(parse_color("#1565C0")),
        brush(parse_color("#2E7D32")),
        brush(parse_color("#6A1B9A")),
        brush(parse_color("#212121")),
    ]
}

/// The table `SelectedColors`.
fn selected_colors() -> [Option<Rc<dyn IBrush>>; 5] {
    [
        None,
        brush(parse_color("#E53935")),
        brush(parse_color("#F57C00")),
        brush(parse_color("#00796B")),
        brush(parse_color("#E91E63")),
    ]
}

/// The table `UnselectedColors`.
fn unselected_colors() -> [Option<Rc<dyn IBrush>>; 4] {
    [None, brush(parse_color("#757575")), brush(parse_color("#BDBDBD")), brush(Color::from_argb(153, 255, 255, 255))]
}

/// `table[index]`.
///
/// # Panics
/// Panics if the index is not an index of the table (the index out of range exception of the
/// managed original).
fn entry<const N: usize>(table: [Option<Rc<dyn IBrush>>; N], index: i32) -> Option<Rc<dyn IBrush>> {
    let index = usize::try_from(index).ok().filter(|index| *index < N).expect("the index of a brush of the table");
    table[index].clone()
}

#[repr(C)]
pub struct TabbedPageCustomizationPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

ferro_class!(TabbedPageCustomizationPage: UserControl);
ferro_impl_classes!(
    TabbedPageCustomizationPage: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(TabbedPageCustomizationPage {
    new: TabbedPageCustomizationPage::new,
    markup: {
        methods: [
            fn OnPlacementChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_placement_changed(&sender, e)
                    }
                },
            fn OnBarBgChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_bar_bg_changed(&sender, e)
                    }
                },
            fn OnSelectedColorChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_selected_color_changed(&sender, e)
                    }
                },
            fn OnUnselectedColorChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_unselected_color_changed(&sender, e)
                    }
                },
            fn OnKeyboardChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_keyboard_changed(&sender, e.as_routed_event_args())
                },
            fn OnShowIconsChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_show_icons_changed(&sender, e.as_routed_event_args())
                },
            fn OnTabEnabledChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_tab_enabled_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TabbedPageCustomizationPage, "/Pages/TabbedPage/TabbedPageCustomizationPage.xaml");

impl StyledElementImpl for TabbedPageCustomizationPage {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_logical_tree(this, e);

        this.update_bar_background_brush();
        this.update_selected_tab_foreground_brush();
        this.update_unselected_tab_foreground_brush();
    }
}

impl TabbedPageCustomizationPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        this
    }

    fn combo(&self, name: &str) -> Ref<ComboBox> {
        self.get_control::<ComboBox>(name)
    }

    fn check(&self, name: &str) -> Ref<CheckBox> {
        self.get_control::<CheckBox>(name)
    }

    /// The field `DemoTabs`: null until `InitializeComponent()` has returned.
    fn demo_tabs(&self) -> Option<Ref<TabbedPage>> {
        self.component_initialized.get().then(|| self.get_control::<TabbedPage>("DemoTabs"))
    }

    fn on_placement_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(demo_tabs) = self.demo_tabs() else {
            return;
        };
        demo_tabs.set_tab_placement(match self.combo("PlacementCombo").selected_index() {
            1 => TabPlacement::Bottom,
            2 => TabPlacement::Left,
            3 => TabPlacement::Right,
            _ => TabPlacement::Top,
        });
    }

    fn on_bar_bg_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if self.is_initialized() {
            self.update_bar_background_brush();
        }
    }

    fn update_bar_background_brush(&self) {
        let brush = entry(bar_backgrounds(), self.combo("BarBgCombo").selected_index());
        self.resources().set("CustomBarBackgroundBrush", brush.map(|brush| Rc::new(brush) as BoxedValue));
    }

    fn on_selected_color_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if self.is_initialized() {
            self.update_selected_tab_foreground_brush();
        }
    }

    fn update_selected_tab_foreground_brush(&self) {
        let value = match entry(selected_colors(), self.combo("SelectedColorCombo").selected_index()) {
            Some(brush) => Some(Rc::new(brush) as BoxedValue),
            None => self.find_resource(&ResourceKey::from("TabbedPageTabItemHeaderForegroundSelected")),
        };
        self.resources().set("CustomSelectedTabForegroundBrush", value);
    }

    fn on_unselected_color_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if self.is_initialized() {
            self.update_unselected_tab_foreground_brush();
        }
    }

    fn update_unselected_tab_foreground_brush(&self) {
        let value = match entry(unselected_colors(), self.combo("UnselectedColorCombo").selected_index()) {
            Some(brush) => Some(Rc::new(brush) as BoxedValue),
            None => self.find_resource(&ResourceKey::from("TabbedPageTabItemHeaderForegroundUnselected")),
        };
        self.resources().set("CustomUnselectedTabForegroundBrush", value);
    }

    fn on_keyboard_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(demo_tabs) = self.demo_tabs() {
            demo_tabs.set_is_keyboard_navigation_enabled(self.check("KeyboardCheck").is_checked() == Some(true));
        }
    }

    fn on_show_icons_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let show = self.check("ShowIconsCheck").is_checked() == Some(true);
        let icon = |geometry: &str| {
            show.then(|| {
                let icon = PathIcon::new();
                icon.set_data(parse_geometry(geometry));
                Control::boxed(icon)
            })
        };
        self.get_control::<ContentPage>("HomePage").set_icon(icon(HOME_GEOMETRY));
        self.get_control::<ContentPage>("SearchPage").set_icon(icon(SEARCH_GEOMETRY));
        self.get_control::<ContentPage>("SettingsPage").set_icon(icon(SETTINGS_GEOMETRY));
    }

    /// # Panics
    /// Panics if the tag of the check box is not the index of a page (the index out of range
    /// exception of the managed original).
    fn on_tab_enabled_changed(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let cb = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<CheckBox>());
        let Some(cb) = cb else {
            return;
        };
        let Some(index) = value_text(&cb.tag()).and_then(|tag| tag.trim().parse::<i32>().ok()) else {
            return;
        };

        let pages = self.get_control::<TabbedPage>("DemoTabs").pages();
        if let Some(pages) = pages {
            let index = usize::try_from(index).expect("the index of a page");
            if let Some(page) = pages.get(index).cast::<ContentPage>() {
                TabbedPage::set_is_tab_enabled(&page, cb.is_checked() == Some(true));
            }
        }
    }
}
