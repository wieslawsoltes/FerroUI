//! Interactive expressive extended floating actions: size, color, direction and interruption.
use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, instantiate,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ContentControlImpl, Control, ControlImpl, Grid, TextBlock, UserControl,
};
use ferroui_themes_material::MaterialExtendedFab;
use std::cell::Cell;
use std::rc::Rc;

const ACTIONS: &[&str] = &[
    "SmallFab",
    "MediumFab",
    "LargeFab",
    "SecondaryFab",
    "TertiaryFab",
    "RtlFab",
];

#[repr(C)]
pub struct FloatingActionsPage {
    base: UserControl,
    action_count: Cell<u32>,
}
ferro_class!(FloatingActionsPage: UserControl);
ferro_impl_classes!(FloatingActionsPage: FerroObjectImpl, StyledElementImpl, VisualImpl,
    LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);
ferro_class_info!(FloatingActionsPage {
    new: FloatingActionsPage::new
});
xaml_class!(FloatingActionsPage, "/Pages/FloatingActionsPage.xaml");

impl FloatingActionsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            action_count: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let weak = this.downgrade();
        this.get_control::<Button>("ToggleExpanded")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let expanded = !this
                        .get_control::<MaterialExtendedFab>("SmallFab")
                        .is_expanded();
                    for name in ACTIONS {
                        this.get_control::<MaterialExtendedFab>(name)
                            .set_is_expanded(expanded);
                    }
                    this.get_control::<Button>("ToggleExpanded")
                        .set_content(Some(Rc::new(
                            if expanded {
                                "Collapse labels"
                            } else {
                                "Expand labels"
                            }
                            .to_owned(),
                        )));
                }
            });
        let weak = this.downgrade();
        this.get_control::<Button>("ToggleDirection")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let examples = this.get_control::<Grid>("SizeExamples");
                    examples.set_flow_direction(
                        if examples.flow_direction() == FlowDirection::LeftToRight {
                            FlowDirection::RightToLeft
                        } else {
                            FlowDirection::LeftToRight
                        },
                    );
                }
            });
        for name in ACTIONS {
            let weak = this.downgrade();
            this.get_control::<MaterialExtendedFab>(name)
                .click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        let count = this.action_count.get().saturating_add(1);
                        this.action_count.set(count);
                        this.get_control::<TextBlock>("ActionStatus")
                            .set_text(Some(&format!("Action activated · {count}")));
                    }
                });
        }
        this
    }
}

pub fn create() -> Ref<Control> {
    FloatingActionsPage::new().upcast()
}
