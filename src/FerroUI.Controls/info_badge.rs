//! WinUI's `InfoBadge` (`controls/dev/InfoBadge/InfoBadge.cpp`, `InfoBadge.h` and `InfoBadge.idl`
//! of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use crate::primitives::{
    InfoBadgeTemplateSettings, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::{ControlImpl, FontIconSource, IconSource};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, CornerRadius,
    DirectProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Ref, Size, StyledElementImpl, StyledProperty, StyledPropertyOptions, Visual, VisualImpl,
};
use std::cell::Cell;

const PC_DOT: &str = ":dot";
const PC_ICON: &str = ":icon";
const PC_FONT_ICON: &str = ":font-icon";
const PC_VALUE: &str = ":value";

/// A small badge that draws attention to an item (WinUI's `InfoBadge`): a dot, an icon or a
/// number.
///
/// A [`value`](Self::value) of 0 or more is shown as a number and takes precedence over the
/// [`icon_source`](Self::icon_source); with the value -1 (the default) the badge shows the icon of
/// its icon source, and a dot without one. The control sets the states of WinUI as
/// pseudo-classes, which the control theme of the WinUI theme
/// (`src/FerroUI.Themes.WinUI/Controls/InfoBadge.xaml`) styles: `:dot`, `:icon`, `:font-icon`
/// (an icon of a [`FontIconSource`]) and `:value` (DisplayKindStates). The template binds the
/// number to [`value`](Self::value) and the icon and the corner radius to the
/// [`template_settings`](Self::template_settings). The Fluent and Simple themes have no template
/// for it (docs/porting/winui-controls.md, section 5.1).
///
/// Differences from WinUI (docs/porting/winui-controls.md, section 10.2): a value below -1 is
/// coerced to -1 and logged, where WinUI throws; whether `CornerRadius` is local is followed from
/// the priority of its changes.
#[repr(C)]
pub struct InfoBadge {
    base: TemplatedControl,
    settings: Ref<InfoBadgeTemplateSettings>,
    corner_radius_is_local: Cell<bool>,
}

ferro_class!(InfoBadge: TemplatedControl);
ferro_class_info!(InfoBadge {
    new: InfoBadge::new,
    markup: {
        namespace: "FerroUI.Controls",
        attributes: [PseudoClasses(":dot", ":icon", ":font-icon", ":value")],
    },
});
ferro_impl_classes!(InfoBadge: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for InfoBadge {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.on_display_kind_properties_changed();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == TemplatedControl::corner_radius_property().as_property() {
            // WinUI asks whether the badge has a local CornerRadius (ReadLocalValue); the port
            // follows the priority of the changes of the property (docs/porting/winui-controls.md,
            // section 10.2).
            this.track_local_corner_radius(change);
            this.update_corner_radius();
            return;
        }
        if !change.is_effective_value_change() {
            return;
        }
        if property == Self::value_property().as_property() || property == Self::icon_source_property().as_property() {
            this.on_display_kind_properties_changed();
        } else if property == Visual::bounds_property().as_property() {
            // InfoBadge::OnSizeChanged (the radius depends on the height only; setting it again
            // when the badge only moves changes nothing).
            this.update_corner_radius();
        }
    }
}

impl LayoutableImpl for InfoBadge {
    /// InfoBadge::MeasureOverride: a badge narrower than it is high is as wide as it is high.
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let desired = Self::parent_measure_override(this, available_size);
        if desired.width < desired.height {
            Size::new(desired.height, desired.height)
        } else {
            desired
        }
    }
}

impl TemplatedControlImpl for InfoBadge {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        this.on_display_kind_properties_changed();
    }
}

ferro_properties! { impl InfoBadge {
    ferro_property!(
        /// Defines the `Value` property: the number the badge shows when it is 0 or more; -1 (the
        /// default) shows the icon or the dot. A value below -1 is coerced to -1.
        pub fn value_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<InfoBadge, _>(
                "Value",
                StyledPropertyOptions::new(-1_i32).coerce(InfoBadge::coerce_value),
            )
        }
    );

    ferro_property!(
        /// Defines the `IconSource` property: the icon the badge shows when its value is -1.
        pub fn icon_source_property() -> StyledProperty<Option<Ref<IconSource>>> {
            FerroProperty::register::<InfoBadge, _>("IconSource", None)
        }
    );

    ferro_property!(
        /// Defines the `TemplateSettings` property: the values the template binds to (read-only).
        pub fn template_settings_property() -> DirectProperty<InfoBadge, Ref<InfoBadgeTemplateSettings>> {
            FerroProperty::register_direct::<InfoBadge, _>(
                "TemplateSettings",
                |owner| owner.template_settings(),
                None,
                InfoBadgeTemplateSettings::new(),
            )
        }
    );
} }

impl InfoBadge {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            settings: InfoBadgeTemplateSettings::new(),
            corner_radius_is_local: Cell::new(false),
        }
    }

    /// Creates an info badge.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the number the badge shows (-1: the icon or the dot).
    pub fn value(&self) -> i32 {
        self.get_value(Self::value_property())
    }

    /// Sets the number the badge shows (-1: the icon or the dot). `set_value` is the setter of
    /// properties of the object, so the setter of `Value` has this name.
    pub fn set_numeric_value(&self, value: i32) {
        FerroObject::set_value(self, Self::value_property(), value);
    }

    /// Gets the icon source.
    pub fn icon_source(&self) -> Option<Ref<IconSource>> {
        self.get_value(Self::icon_source_property())
    }

    /// Sets the icon source.
    pub fn set_icon_source(&self, value: Option<Ref<IconSource>>) {
        FerroObject::set_value(self, Self::icon_source_property(), value);
    }

    /// Gets the values the template binds to; each badge has its own.
    pub fn template_settings(&self) -> Ref<InfoBadgeTemplateSettings> {
        self.settings.clone()
    }

    /// WinUI throws for a value below -1 (InfoBadge::OnPropertyChanged); a value from markup or
    /// from a binding must not end the application, so the value is coerced to -1 and logged.
    fn coerce_value(_instance: &FerroObject, value: i32) -> i32 {
        if value < -1 {
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                logger.log(None, &format!("InfoBadge.Value must be -1 or more; {value} is coerced to -1."));
            }
            -1
        } else {
            value
        }
    }

    /// InfoBadge::OnDisplayKindPropertiesChanged: the number, the icon of the icon source (in the
    /// template settings) or the dot.
    fn on_display_kind_properties_changed(&self) {
        let classes = self.pseudo_classes();
        let (value, icon, font_icon) = if self.value() >= 0 {
            (true, false, false)
        } else if let Some(source) = self.icon_source() {
            self.settings.set_icon_element(source.create_icon_element());
            let font_icon = source.cast::<FontIconSource>().is_some();
            (false, !font_icon, font_icon)
        } else {
            (false, false, false)
        };
        classes.set(PC_VALUE, value);
        classes.set(PC_ICON, icon);
        classes.set(PC_FONT_ICON, font_icon);
        classes.set(PC_DOT, !value && !icon && !font_icon);
    }

    /// InfoBadge::OnSizeChanged: half the height of the badge, unless it has a local corner
    /// radius, which is then the radius of the template.
    fn update_corner_radius(&self) {
        let radius = if self.corner_radius_is_local.get() {
            self.corner_radius()
        } else {
            CornerRadius::uniform(self.bounds().height / 2.0)
        };
        self.settings.set_info_badge_corner_radius(radius);
    }

    /// Whether `CornerRadius` has a local value, from the priority of its changes: a change at
    /// the local priority (a value or a binding set on the badge) makes it local; a change of the
    /// effective value at a lower priority (the local value was cleared, a style sets it) makes it
    /// not local; a change of an animation leaves it as it is. A local value equal to the value it
    /// replaces raises no change and is not seen (owner decision O5: no `has_local_value`).
    fn track_local_corner_radius(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        match change.priority() {
            BindingPriority::LocalValue => self.corner_radius_is_local.set(true),
            BindingPriority::Animation => {}
            _ if change.is_effective_value_change() => self.corner_radius_is_local.set(false),
            _ => {}
        }
    }
}
