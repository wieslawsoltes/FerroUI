//! Port of `Diagnostics/Controls/ThicknessEditor.cs`.

use ferroui_base::FerroObjectImplExt;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IBrush;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions, Thickness,
    VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControl, ContentControlImpl, ControlImpl};
use std::cell::Cell;
use std::rc::Rc;

/// An editor of a thickness: the four sides as numbers around the content, which is the area
/// inside the thickness, with a header and a highlight. The layout explorer nests the editors
/// of the margin, the border and the padding of a control around its content.
#[repr(C)]
pub struct ThicknessEditor {
    base: ContentControl,
    is_updating_thickness: Cell<bool>,
}

ferro_class!(ThicknessEditor: ContentControl);
ferro_impl_classes!(
    ThicknessEditor: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(ThicknessEditor { new: ThicknessEditor::new });

ferro_properties! {
    impl ThicknessEditor {
        pub fn thickness_property() -> StyledProperty<Thickness> {
            FerroProperty::register_with::<ThicknessEditor, _>(
                "Thickness",
                StyledPropertyOptions::new(Thickness::default()).default_binding_mode(BindingMode::TwoWay),
            )
        }

        pub fn header_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ThicknessEditor, _>("Header", None)
        }

        pub fn is_present_property() -> StyledProperty<bool> {
            FerroProperty::register::<ThicknessEditor, _>("IsPresent", true)
        }

        pub fn left_property() -> StyledProperty<f64> {
            FerroProperty::register::<ThicknessEditor, _>("Left", 0.0)
        }

        pub fn top_property() -> StyledProperty<f64> {
            FerroProperty::register::<ThicknessEditor, _>("Top", 0.0)
        }

        pub fn right_property() -> StyledProperty<f64> {
            FerroProperty::register::<ThicknessEditor, _>("Right", 0.0)
        }

        pub fn bottom_property() -> StyledProperty<f64> {
            FerroProperty::register::<ThicknessEditor, _>("Bottom", 0.0)
        }

        pub fn highlight_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<ThicknessEditor, _>("Highlight", None)
        }
    }
}

impl FerroObjectImpl for ThicknessEditor {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Self::thickness_property().as_property() {
            // `try { .. } finally { _isUpdatingThickness = false; }`: nothing in between can fail.
            this.is_updating_thickness.set(true);

            let value = change.get_new_value::<Thickness>();

            this.set_current_value(Self::left_property(), value.left);
            this.set_current_value(Self::top_property(), value.top);
            this.set_current_value(Self::right_property(), value.right);
            this.set_current_value(Self::bottom_property(), value.bottom);

            this.is_updating_thickness.set(false);
        } else if !this.is_updating_thickness.get()
            && (property == Self::left_property().as_property()
                || property == Self::top_property().as_property()
                || property == Self::right_property().as_property()
                || property == Self::bottom_property().as_property())
        {
            this.set_current_value(
                Self::thickness_property(),
                Thickness::new(this.left(), this.top(), this.right(), this.bottom()),
            );
        }
    }
}

impl ThicknessEditor {
    pub fn construct() -> Self {
        Self { base: ContentControl::construct(), is_updating_thickness: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn thickness(&self) -> Thickness {
        self.get_value(Self::thickness_property())
    }

    pub fn set_thickness(&self, value: Thickness) {
        self.set_value(Self::thickness_property(), value)
    }

    pub fn header(&self) -> Option<String> {
        self.get_value(Self::header_property())
    }

    pub fn set_header(&self, value: Option<String>) {
        self.set_value(Self::header_property(), value)
    }

    pub fn is_present(&self) -> bool {
        self.get_value(Self::is_present_property())
    }

    pub fn set_is_present(&self, value: bool) {
        self.set_value(Self::is_present_property(), value)
    }

    pub fn left(&self) -> f64 {
        self.get_value(Self::left_property())
    }

    pub fn set_left(&self, value: f64) {
        self.set_value(Self::left_property(), value)
    }

    pub fn top(&self) -> f64 {
        self.get_value(Self::top_property())
    }

    pub fn set_top(&self, value: f64) {
        self.set_value(Self::top_property(), value)
    }

    pub fn right(&self) -> f64 {
        self.get_value(Self::right_property())
    }

    pub fn set_right(&self, value: f64) {
        self.set_value(Self::right_property(), value)
    }

    pub fn bottom(&self) -> f64 {
        self.get_value(Self::bottom_property())
    }

    pub fn set_bottom(&self, value: f64) {
        self.set_value(Self::bottom_property(), value)
    }

    pub fn highlight(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::highlight_property())
    }

    pub fn set_highlight(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::highlight_property(), value)
    }
}
