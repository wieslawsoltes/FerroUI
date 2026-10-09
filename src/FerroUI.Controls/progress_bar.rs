use crate::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use crate::primitives::{RangeBase, TemplateAppliedEventArgs, TemplatedControlImpl};
use crate::{Border, ControlImpl};
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt, Orientation};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, DirectProperty, FerroObject, FerroObjectExtensions,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size, StaticType,
    StyledElementImpl, StyledProperty, StyledPropertyMetadata, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_VERTICAL: &str = ":vertical";
const PC_HORIZONTAL: &str = ":horizontal";
const PC_INDETERMINATE: &str = ":indeterminate";

/// Provides calculated values for use with the [`ProgressBar`]'s control
/// theme or template.
///
/// This class is NOT intended for general use outside of control templates.
#[repr(C)]
pub struct ProgressBarTemplateSettings {
    base: FerroObject,
    container2_width: Cell<f64>,
    container_width: Cell<f64>,
    container_animation_start_position: Cell<f64>,
    container_animation_end_position: Cell<f64>,
    container2_animation_start_position: Cell<f64>,
    container2_animation_end_position: Cell<f64>,
    indeterminate_starting_offset: Cell<f64>,
    indeterminate_ending_offset: Cell<f64>,
}

ferro_class!(ProgressBarTemplateSettings: FerroObject);
ferroui_base::ferro_class_info!(ProgressBarTemplateSettings {
    new: ProgressBarTemplateSettings::new,
});
ferro_impl_classes!(ProgressBarTemplateSettings: FerroObjectImpl);

ferroui_base::ferro_properties! { impl ProgressBarTemplateSettings {
    ferro_property!(
        /// Defines the `ContainerAnimationStartPosition` property.
        pub fn container_animation_start_position_property() -> DirectProperty<ProgressBarTemplateSettings, f64> {
            FerroProperty::register_direct::<ProgressBarTemplateSettings, _>(
                "ContainerAnimationStartPosition",
                |p| p.container_animation_start_position(),
                Some(|p, o| p.set_container_animation_start_position(o)),
                0.0,
            )
        }
    );

    ferro_property!(
        /// Defines the `ContainerAnimationEndPosition` property.
        pub fn container_animation_end_position_property() -> DirectProperty<ProgressBarTemplateSettings, f64> {
            FerroProperty::register_direct::<ProgressBarTemplateSettings, _>(
                "ContainerAnimationEndPosition",
                |p| p.container_animation_end_position(),
                Some(|p, o| p.set_container_animation_end_position(o)),
                0.0,
            )
        }
    );

    ferro_property!(
        /// Defines the `Container2AnimationStartPosition` property.
        pub fn container2_animation_start_position_property() -> DirectProperty<ProgressBarTemplateSettings, f64> {
            FerroProperty::register_direct::<ProgressBarTemplateSettings, _>(
                "Container2AnimationStartPosition",
                |p| p.container2_animation_start_position(),
                Some(|p, o| p.set_container2_animation_start_position(o)),
                0.0,
            )
        }
    );

    ferro_property!(
        /// Defines the `Container2AnimationEndPosition` property.
        pub fn container2_animation_end_position_property() -> DirectProperty<ProgressBarTemplateSettings, f64> {
            FerroProperty::register_direct::<ProgressBarTemplateSettings, _>(
                "Container2AnimationEndPosition",
                |p| p.container2_animation_end_position(),
                Some(|p, o| p.set_container2_animation_end_position(o)),
                0.0,
            )
        }
    );

    ferro_property!(
        /// Defines the `Container2Width` property.
        pub fn container2_width_property() -> DirectProperty<ProgressBarTemplateSettings, f64> {
            FerroProperty::register_direct::<ProgressBarTemplateSettings, _>(
                "Container2Width",
                |p| p.container2_width(),
                Some(|p, o| p.set_container2_width(o)),
                0.0,
            )
        }
    );

    ferro_property!(
        /// Defines the `ContainerWidth` property.
        pub fn container_width_property() -> DirectProperty<ProgressBarTemplateSettings, f64> {
            FerroProperty::register_direct::<ProgressBarTemplateSettings, _>(
                "ContainerWidth",
                |p| p.container_width(),
                Some(|p, o| p.set_container_width(o)),
                0.0,
            )
        }
    );

    ferro_property!(
        /// Defines the `IndeterminateStartingOffset` property.
        pub fn indeterminate_starting_offset_property() -> DirectProperty<ProgressBarTemplateSettings, f64> {
            FerroProperty::register_direct::<ProgressBarTemplateSettings, _>(
                "IndeterminateStartingOffset",
                |p| p.indeterminate_starting_offset(),
                Some(|p, o| p.set_indeterminate_starting_offset(o)),
                0.0,
            )
        }
    );

    ferro_property!(
        /// Defines the `IndeterminateEndingOffset` property.
        pub fn indeterminate_ending_offset_property() -> DirectProperty<ProgressBarTemplateSettings, f64> {
            FerroProperty::register_direct::<ProgressBarTemplateSettings, _>(
                "IndeterminateEndingOffset",
                |p| p.indeterminate_ending_offset(),
                Some(|p, o| p.set_indeterminate_ending_offset(o)),
                0.0,
            )
        }
    );
} }

impl ProgressBarTemplateSettings {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            container2_width: Cell::new(0.0),
            container_width: Cell::new(0.0),
            container_animation_start_position: Cell::new(0.0),
            container_animation_end_position: Cell::new(0.0),
            container2_animation_start_position: Cell::new(0.0),
            container2_animation_end_position: Cell::new(0.0),
            indeterminate_starting_offset: Cell::new(0.0),
            indeterminate_ending_offset: Cell::new(0.0),
        })
    }

    /// Used by the fluent theme to define the first indeterminate
    /// indicator's width.
    pub fn container_width(&self) -> f64 {
        self.container_width.get()
    }

    pub fn set_container_width(&self, value: f64) {
        self.set_and_raise_cell(Self::container_width_property(), &self.container_width, value);
    }

    /// Used by the fluent theme to define the second indeterminate
    /// indicator's width.
    pub fn container2_width(&self) -> f64 {
        self.container2_width.get()
    }

    pub fn set_container2_width(&self, value: f64) {
        self.set_and_raise_cell(Self::container2_width_property(), &self.container2_width, value);
    }

    /// Used by the fluent theme to define the first indeterminate
    /// indicator's start position when animated.
    pub fn container_animation_start_position(&self) -> f64 {
        self.container_animation_start_position.get()
    }

    pub fn set_container_animation_start_position(&self, value: f64) {
        self.set_and_raise_cell(
            Self::container_animation_start_position_property(),
            &self.container_animation_start_position,
            value,
        );
    }

    /// Used by the fluent theme to define the first indeterminate
    /// indicator's end position when animated.
    pub fn container_animation_end_position(&self) -> f64 {
        self.container_animation_end_position.get()
    }

    pub fn set_container_animation_end_position(&self, value: f64) {
        self.set_and_raise_cell(
            Self::container_animation_end_position_property(),
            &self.container_animation_end_position,
            value,
        );
    }

    /// Used by the fluent theme to define the second indeterminate
    /// indicator's start position when animated.
    pub fn container2_animation_start_position(&self) -> f64 {
        self.container2_animation_start_position.get()
    }

    pub fn set_container2_animation_start_position(&self, value: f64) {
        self.set_and_raise_cell(
            Self::container2_animation_start_position_property(),
            &self.container2_animation_start_position,
            value,
        );
    }

    /// Used by the fluent theme to define the second indeterminate
    /// indicator's end position when animated.
    pub fn container2_animation_end_position(&self) -> f64 {
        self.container2_animation_end_position.get()
    }

    pub fn set_container2_animation_end_position(&self, value: f64) {
        self.set_and_raise_cell(
            Self::container2_animation_end_position_property(),
            &self.container2_animation_end_position,
            value,
        );
    }

    /// Used by the simple theme to define the starting point of its
    /// indeterminate animation.
    pub fn indeterminate_starting_offset(&self) -> f64 {
        self.indeterminate_starting_offset.get()
    }

    pub fn set_indeterminate_starting_offset(&self, value: f64) {
        self.set_and_raise_cell(
            Self::indeterminate_starting_offset_property(),
            &self.indeterminate_starting_offset,
            value,
        );
    }

    /// Used by the simple theme to define the ending point of its
    /// indeterminate animation.
    pub fn indeterminate_ending_offset(&self) -> f64 {
        self.indeterminate_ending_offset.get()
    }

    pub fn set_indeterminate_ending_offset(&self, value: f64) {
        self.set_and_raise_cell(
            Self::indeterminate_ending_offset_property(),
            &self.indeterminate_ending_offset,
            value,
        );
    }
}

/// A control used to indicate the progress of an operation.
#[repr(C)]
pub struct ProgressBar {
    base: RangeBase,
    percentage: Cell<f64>,
    indicator: RefCell<Option<Ref<Border>>>,
    track_size_changed_listener: RefCell<Option<Rc<dyn IDisposable>>>,
    template_settings: Ref<ProgressBarTemplateSettings>,
}

ferro_class!(ProgressBar: RangeBase);
// The `markup:` part is declared here and not generated: `TemplateSettings` is a plain property
// (a property of the class that is not a registered one), which the bindings of the control
// themes read (`{Binding $parent[ProgressBar].TemplateSettings.ContainerWidth}`).
ferroui_base::ferro_class_info!(ProgressBar {
    new: ProgressBar::new,
    markup: {
        properties: [
            TemplateSettings: Ref<ProgressBarTemplateSettings> { get: ProgressBar::template_settings },
        ],
        attributes: [
            TemplatePart("PART_Indicator", type(Ref<Border>), IsRequired = true),
            PseudoClasses(":vertical", ":horizontal", ":indeterminate"),
        ],
    },
});
ferro_impl_classes!(ProgressBar: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl);

impl ControlImpl for ProgressBar {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ProgressBarAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for ProgressBar {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(Some(this.is_indeterminate()), Some(this.orientation()));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == RangeBase::value_property().as_property()
            || property == RangeBase::minimum_property().as_property()
            || property == RangeBase::maximum_property().as_property()
            || property == Self::is_indeterminate_property().as_property()
            || property == Self::orientation_property().as_property()
        {
            this.update_indicator();
        }

        if property == Self::is_indeterminate_property().as_property() {
            this.update_pseudo_classes(Some(change.get_new_value::<bool>()), None);
        } else if property == Self::orientation_property().as_property() {
            this.update_pseudo_classes(None, Some(change.get_new_value::<Orientation>()));
        }
    }
}

impl LayoutableImpl for ProgressBar {
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let result = Self::parent_arrange_override(this, final_size);
        this.update_indicator();
        result
    }
}

impl TemplatedControlImpl for ProgressBar {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        // Dispose any previous track size listener.
        let previous = this.track_size_changed_listener.borrow_mut().take();
        if let Some(previous) = previous {
            previous.dispose();
        }

        let indicator = e.name_scope().get_as::<Border>("PART_Indicator");
        *this.indicator.borrow_mut() = Some(indicator.clone());

        // Listen to size changes of the indicator's track (parent) and update
        // the indicator there.
        let listener = indicator.parent().map(|parent| {
            let weak = this.to_ref().downgrade();
            parent.get_property_changed_observable(Visual::bounds_property().as_property()).subscribe(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.update_indicator();
                }
            })
        });
        *this.track_size_changed_listener.borrow_mut() = listener;

        this.update_indicator();
    }
}

impl ProgressBar {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] =
        &[TemplatePartAttribute::required("PART_Indicator", <Border as StaticType>::TYPE)];

    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute =
        PseudoClassesAttribute::new(&[PC_VERTICAL, PC_HORIZONTAL, PC_INDETERMINATE]);
}

ferroui_base::ferro_properties! { impl ProgressBar {
    ferro_property!(
        /// Defines the `IsIndeterminate` property.
        pub fn is_indeterminate_property() -> StyledProperty<bool> {
            FerroProperty::register::<ProgressBar, _>("IsIndeterminate", false)
        }
    );

    ferro_property!(
        /// Defines the `ShowProgressText` property.
        pub fn show_progress_text_property() -> StyledProperty<bool> {
            FerroProperty::register::<ProgressBar, _>("ShowProgressText", false)
        }
    );

    ferro_property!(
        /// Defines the `ProgressTextFormat` property.
        pub fn progress_text_format_property() -> StyledProperty<String> {
            FerroProperty::register::<ProgressBar, _>("ProgressTextFormat", "{1:0}%".to_string())
        }
    );

    ferro_property!(
        /// Defines the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            FerroProperty::register::<ProgressBar, _>("Orientation", Orientation::Horizontal)
        }
    );

    ferro_property!(
        /// Defines the `Percentage` property.
        pub fn percentage_property() -> DirectProperty<ProgressBar, f64> {
            FerroProperty::register_direct::<ProgressBar, _>("Percentage", |o| o.percentage(), None, 0.0)
        }
    );
} }

impl ProgressBar {
    fn static_constructor() {
        RangeBase::value_property().override_metadata::<ProgressBar>(
            StyledPropertyMetadata::new(None).with_default_binding_mode(BindingMode::OneWay),
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: RangeBase::construct(),
            percentage: Cell::new(0.0),
            indicator: RefCell::new(None),
            track_size_changed_listener: RefCell::new(None),
            template_settings: ProgressBarTemplateSettings::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The overall percentage complete of the progress.
    ///
    /// This read-only property is automatically calculated using the current
    /// `Value` and the effective range (`Maximum` - `Minimum`).
    pub fn percentage(&self) -> f64 {
        self.percentage.get()
    }

    fn set_percentage(&self, value: f64) {
        self.set_and_raise_cell(Self::percentage_property(), &self.percentage, value);
    }

    /// The template settings of the progress bar.
    pub fn template_settings(&self) -> Ref<ProgressBarTemplateSettings> {
        self.template_settings.clone()
    }

    /// Whether the progress bar shows the actual value or a generic,
    /// continuous progress indicator (indeterminate state).
    pub fn is_indeterminate(&self) -> bool {
        self.get_value(Self::is_indeterminate_property())
    }

    pub fn set_is_indeterminate(&self, value: bool) {
        self.set_value(Self::is_indeterminate_property(), value)
    }

    /// Whether progress text will be shown.
    pub fn show_progress_text(&self) -> bool {
        self.get_value(Self::show_progress_text_property())
    }

    pub fn set_show_progress_text(&self, value: bool) {
        self.set_value(Self::show_progress_text_property(), value)
    }

    /// The format string applied to the internally calculated progress text
    /// before it is shown.
    pub fn progress_text_format(&self) -> String {
        self.get_value(Self::progress_text_format_property())
    }

    pub fn set_progress_text_format(&self, value: impl Into<String>) {
        self.set_value(Self::progress_text_format_property(), value.into())
    }

    /// The orientation of the progress bar.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    fn update_indicator(&self) {
        let Some(indicator) = self.indicator.borrow().clone() else {
            return;
        };

        // Gets the size of the parent indicator container.
        let bar_size = indicator.visual_parent().map_or_else(|| self.bounds().size(), |parent| parent.bounds().size());

        if self.is_indeterminate() {
            // Pulled from ModernWPF.

            let dim = if self.orientation() == Orientation::Horizontal { bar_size.width } else { bar_size.height };
            let bar_indicator_width = dim * 0.4; // Indicator width at 40% of the progress bar
            let bar_indicator_width2 = dim * 0.6; // Indicator width at 60% of the progress bar

            let settings = &self.template_settings;

            settings.set_container_width(bar_indicator_width);
            settings.set_container2_width(bar_indicator_width2);

            settings.set_container_animation_start_position(bar_indicator_width * -1.8); // Position at -180%
            settings.set_container_animation_end_position(bar_indicator_width * 3.0); // Position at 300%

            settings.set_container2_animation_start_position(bar_indicator_width2 * -1.5); // Position at -150%
            settings.set_container2_animation_end_position(bar_indicator_width2 * 1.66); // Position at 166%

            settings.set_indeterminate_starting_offset(-dim);
            settings.set_indeterminate_ending_offset(dim);
        } else {
            let (minimum, maximum) = (self.minimum(), self.maximum());
            // The smallest positive value: the range is empty only when the
            // bounds are equal.
            let percent =
                if (maximum - minimum).abs() < 5e-324 { 1.0 } else { (self.value() - minimum) / (maximum - minimum) };

            // When the orientation changed, the indicator's width or height
            // should be set to NaN. The indicator size calculation should
            // consider the padding of the progress bar.
            let margin = indicator.margin();
            if self.orientation() == Orientation::Horizontal {
                let width = (bar_size.width - margin.left - margin.right) * percent;
                indicator.set_width(if width > 0.0 { width } else { 0.0 });
                indicator.set_height(f64::NAN);
            } else {
                indicator.set_width(f64::NAN);
                let height = (bar_size.height - margin.top - margin.bottom) * percent;
                indicator.set_height(if height > 0.0 { height } else { 0.0 });
            }

            self.set_percentage(percent * 100.0);
        }
    }

    fn update_pseudo_classes(&self, is_indeterminate: Option<bool>, o: Option<Orientation>) {
        if let Some(is_indeterminate) = is_indeterminate {
            self.pseudo_classes().set(PC_INDETERMINATE, is_indeterminate);
        }

        let Some(o) = o else {
            return;
        };
        self.pseudo_classes().set(PC_VERTICAL, o == Orientation::Vertical);
        self.pseudo_classes().set(PC_HORIZONTAL, o == Orientation::Horizontal);
    }
}
