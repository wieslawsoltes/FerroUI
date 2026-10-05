//! Port of `Pages/ToolTipPage.xaml.cs`: the class of the document
//! `Pages/ToolTipPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{CancelRoutedEventArgs, IRoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Point, Ref};
use ferroui_controls::primitives::popup_positioning::{CustomPopupPlacement, PopupAnchor, PopupGravity};
use ferroui_controls::{ContentPage, Control, ToolTip};
use std::hash::{BuildHasher, Hasher};
use std::rc::Rc;

#[repr(C)]
pub struct ToolTipPage {
    base: ContentPage,
}

content_page_class!(ToolTipPage);
ferro_class_info!(ToolTipPage {
    new: ToolTipPage::new,
    markup: {
        methods: [
            fn ToolTipOpening(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ToolTipPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    let args = args.downcast_ref::<CancelRoutedEventArgs>().expect("the arguments of a tool tip opening event");
                    this.tool_tip_opening(&sender, args)
                },
        ],
    },
});
xaml_class!(ToolTipPage, "/Pages/ToolTipPage.xaml");

/// A non-negative random integer (`new Random().Next()`).
fn next_random() -> i32 {
    let random = std::collections::hash_map::RandomState::new().build_hasher().finish();
    (random & 0x7FFF_FFFF) as i32
}

impl ToolTipPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// # Panics
    /// Panics if the source of the event is not a control (an invalid cast
    /// in the managed original).
    fn tool_tip_opening(&self, _sender: &Option<BoxedValue>, args: &CancelRoutedEventArgs) {
        let source = args
            .source()
            .and_then(|source| source.cast::<Control>())
            .expect("the source of a tool tip opening event is a control");
        let tip: BoxedValue = Rc::new(String::from("New tip set from ToolTipOpening."));
        source.set_value(ToolTip::tip_property(), Some(tip));
    }

    pub fn custom_placement_callback(&self, placement: &mut CustomPopupPlacement) {
        let r = next_random();

        placement.set_anchor(match r % 4 {
            1 => PopupAnchor::TOP,
            2 => PopupAnchor::LEFT,
            3 => PopupAnchor::RIGHT,
            _ => PopupAnchor::BOTTOM,
        });
        placement.set_gravity(match r % 4 {
            1 => PopupGravity::TOP,
            2 => PopupGravity::LEFT,
            3 => PopupGravity::RIGHT,
            _ => PopupGravity::BOTTOM,
        });
        placement.offset = Point::new((r % 20) as f64, (r % 20) as f64);
    }
}
