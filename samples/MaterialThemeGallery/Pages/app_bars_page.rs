//! A real scrolling surface for the six Material top app-bar variants.
use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, Vector, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, instantiate,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ContentControlImpl, Control, ControlImpl, Grid, ScrollViewer, UserControl,
};
use ferroui_themes_material::{
    MaterialAppBar, MaterialAppBarScrollBehavior as Behavior, MaterialAppBarSize as Variant,
};
use std::rc::Rc;

#[repr(C)]
pub struct AppBarsPage {
    base: UserControl,
}
ferro_class!(AppBarsPage: UserControl);
ferro_impl_classes!(AppBarsPage: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl,
    InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);
ferro_class_info!(AppBarsPage {
    new: AppBarsPage::new
});
xaml_class!(AppBarsPage, "/Pages/AppBarsPage.xaml");

impl AppBarsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.get_control::<MaterialAppBar>("DemoBar")
            .set_scroll_source(Some(this.get_control::<ScrollViewer>("LibraryScroll")));
        let weak = this.downgrade();
        this.get_control::<Button>("NextVariant")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let bar = this.get_control::<MaterialAppBar>("DemoBar");
                    let (variant, label) = match bar.size_variant() {
                        Variant::Small => (Variant::CenterAligned, "Centered"),
                        Variant::CenterAligned => (Variant::Medium, "Medium"),
                        Variant::Medium => (Variant::Large, "Large"),
                        Variant::Large => (Variant::MediumFlexible, "Medium flexible"),
                        Variant::MediumFlexible => (Variant::LargeFlexible, "Large flexible"),
                        Variant::LargeFlexible => (Variant::Small, "Small"),
                    };
                    bar.set_size_variant(variant);
                    bar.set_height_offset(0.0);
                    this.get_control::<Button>("NextVariant")
                        .set_content(Some(Rc::new(label.to_owned())));
                }
            });
        let weak = this.downgrade();
        this.get_control::<Button>("NextBehavior")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let bar = this.get_control::<MaterialAppBar>("DemoBar");
                    let (behavior, label) = match bar.scroll_behavior() {
                        Behavior::ExitUntilCollapsed => {
                            (Behavior::EnterAlways, "Reveal on reverse scroll")
                        }
                        Behavior::EnterAlways => (Behavior::Pinned, "Stay in place"),
                        Behavior::Pinned => (Behavior::ExitUntilCollapsed, "Collapse, then scroll"),
                    };
                    bar.set_scroll_behavior(behavior);
                    this.get_control::<Button>("NextBehavior")
                        .set_content(Some(Rc::new(label.to_owned())));
                }
            });
        let weak = this.downgrade();
        this.get_control::<Button>("ToggleSubtitle")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let bar = this.get_control::<MaterialAppBar>("DemoBar");
                    let show = bar.subtitle().is_none();
                    bar.set_subtitle(show.then(|| "Stories to return to".to_owned()));
                    this.get_control::<Button>("ToggleSubtitle")
                        .set_content(Some(Rc::new(
                            if show {
                                "Hide subtitle"
                            } else {
                                "Show subtitle"
                            }
                            .to_owned(),
                        )));
                }
            });
        let weak = this.downgrade();
        this.get_control::<Button>("ToggleDirection")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let surface = this.get_control::<Grid>("DemoSurface");
                    surface.set_flow_direction(
                        if surface.flow_direction() == FlowDirection::LeftToRight {
                            FlowDirection::RightToLeft
                        } else {
                            FlowDirection::LeftToRight
                        },
                    );
                }
            });
        let weak = this.downgrade();
        this.get_control::<Button>("BackToTop").click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.get_control::<ScrollViewer>("LibraryScroll")
                    .set_offset(Vector::default());
                this.get_control::<MaterialAppBar>("DemoBar").expand();
            }
        });
        let weak = this.downgrade();
        this.get_control::<Button>("ExpandBar").click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.get_control::<MaterialAppBar>("DemoBar").expand();
            }
        });
        this
    }
}
pub fn create() -> Ref<Control> {
    AppBarsPage::new().upcast()
}
