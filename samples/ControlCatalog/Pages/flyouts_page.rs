//! Port of `Pages/FlyoutsPage.xaml.cs`: the class of the document
//! `Pages/FlyoutsPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::input::{InputElement, TappedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, Point, Ref};
use ferroui_controls::primitives::popup_positioning::{CustomPopupPlacement, PopupAnchor, PopupGravity};
use ferroui_controls::primitives::FlyoutBase;
use ferroui_controls::{ContentPage, Panel, TextBlock};
use std::cell::RefCell;
use std::hash::{BuildHasher, Hasher};
use std::rc::Rc;

#[repr(C)]
pub struct FlyoutsPage {
    base: ContentPage,
}

content_page_class!(FlyoutsPage);
ferro_class_info!(FlyoutsPage {
    new: FlyoutsPage::new,
    markup: {
        methods: [
            fn CustomPlacementCallback(Rc<RefCell<CustomPopupPlacement>>) =>
                |this: &Ref<FlyoutsPage>, placement: Rc<RefCell<CustomPopupPlacement>>| {
                    this.custom_placement_callback(&mut placement.borrow_mut())
                },
        ],
    },
});
xaml_class!(FlyoutsPage, "/Pages/FlyoutsPage.xaml");

/// A non-negative random integer (`new Random().Next()`).
fn next_random() -> i32 {
    let random = std::collections::hash_map::RandomState::new().build_hasher().finish();
    (random & 0x7FFF_FFFF) as i32
}

impl FlyoutsPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let weak = this.downgrade();
        this.attached_flyout_panel().add_handler(
            InputElement::double_tapped_event(),
            move |sender: &Interactive, e: &TappedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.afp_double_tapped(sender, e);
                }
            },
        );

        this.set_xaml_texts();
        this
    }

    fn attached_flyout_panel(&self) -> Ref<Panel> {
        self.get_control::<Panel>("AttachedFlyoutPanel")
    }

    fn button_flyout_xaml_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("ButtonFlyoutXamlText")
    }

    fn menu_flyout_xaml_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("MenuFlyoutXamlText")
    }

    fn attached_flyout_xaml_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("AttachedFlyoutXamlText")
    }

    fn shared_flyout_xaml_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("SharedFlyoutXamlText")
    }

    fn afp_double_tapped(&self, sender: &Interactive, _e: &RoutedEventArgs) {
        if let Some(p) = sender.to_ref().cast::<Panel>() {
            FlyoutBase::show_attached_flyout(&p);
        }
    }

    fn set_xaml_texts(&self) {
        let bfxt = self.button_flyout_xaml_text();
        bfxt.set_text(Some(concat!(
            "<Button Content=\"Click me!\">\n",
            "    <Button.Flyout>\n",
            "        <Flyout>\n",
            "            <Panel Width=\"100\" Height=\"100\">\n",
            "                <TextBlock Text=\"Flyout Content!\" />\n",
            "            </Panel>\n",
            "        </Flyout>\n",
            "    </Button.Flyout>\n</Button>",
        )));

        let mfxt = self.menu_flyout_xaml_text();
        mfxt.set_text(Some(concat!(
            "<Button Content=\"Click me!\">\n",
            "    <Button.Flyout>\n",
            "        <MenuFlyout>\n",
            "            <MenuItem Header=\"Item 1\">\n",
            "            <MenuItem Header=\"Item 2\">\n",
            "        </MenuFlyout>\n",
            "    </Button.Flyout>\n</Button>",
        )));

        let afxt = self.attached_flyout_xaml_text();
        afxt.set_text(Some(concat!(
            "<Panel Name=\"AttachedFlyoutPanel\">\n",
            "    <FlyoutBase.AttachedFlyout>\n",
            "        <Flyout>\n",
            "            <Panel Height=\"100\">\n",
            "                <TextBlock Text=\"Attached Flyout\" />\n",
            "            </Panel>\n",
            "        </Flyout>\n",
            "    </FlyoutBase.AttachedFlyout>\n</Panel>",
            "\n\n In DoubleTapped handler:\n",
            "FlyoutBase.ShowAttachedFlyout(AttachedFlyoutPanel);",
        )));

        let sfxt = self.shared_flyout_xaml_text();
        sfxt.set_text(Some(concat!(
            "Declare a flyout in Resources:\n",
            "<Window.Resources>\n",
            "    <Flyout x:Key=\"SharedFlyout\">\n",
            "        <Panel Width=\"100\" Height=\"100\">\n",
            "            <TextBlock Text=\"Flyout Content!\" />\n",
            "        </Panel>\n",
            "    </Flyout>\n</Window.Resources>\n\n",
            "Then attach the flyout where you want it:\n",
            "<Button Content=\"Launch Flyout here\" Flyout=\"{StaticResource SharedFlyout}\" />",
        )));
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
