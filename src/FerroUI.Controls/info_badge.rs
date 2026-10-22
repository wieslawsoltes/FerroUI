// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Adapted from controls/dev/InfoBadge at microsoft-ui-xaml a71c42bd3e352060e5f5ecf3b5e1a98f8a745809.

use super::{FontIconSource, IconSource};
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::documents::TextElement;
use crate::presenters::ContentPresenter;
use crate::primitives::{
    InfoBadgeTemplateSettings,
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::{Border, Control, ControlImpl, Grid, TextBlock, Viewbox};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{
    HorizontalAlignment, Layoutable, LayoutableImpl, LayoutableImplExt, VerticalAlignment,
};
use ferroui_base::media::{IBrush, SolidColorBrush};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, CornerRadius,
    DirectProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Thickness, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Default)]
struct Parts {
    root: Option<Ref<Border>>,
    value: Option<Ref<TextBlock>>,
    icon: Option<Ref<Viewbox>>,
    icon_content: Option<Ref<ContentPresenter>>,
}

/// A non-interactive notification badge with numeric, icon and dot presentations.
///
/// `Value >= 0` takes precedence over `IconSource`. `Value == -1` shows an icon
/// when a source exists, otherwise a dot. Values below -1 are rejected by the
/// native property validator, including values supplied through bindings/XAML.
/// Icons use the live reusable source contract; the control has no second
/// visual tree or drawing backend. The default and named severity themes are
/// supplied by `FerroUI.Themes.WinUI/Controls/InfoBadge.xaml`.
#[repr(C)]
pub struct InfoBadge {
    base: TemplatedControl,
    settings: Ref<InfoBadgeTemplateSettings>,
    parts: RefCell<Parts>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    updating_parts: Cell<bool>,
    update_pending: Cell<bool>,
}
ferro_class!(InfoBadge: TemplatedControl);
ferro_class_info!(InfoBadge {
    new: InfoBadge::new,
    markup: {
        namespace: "FerroUI.Controls",
        fields: [HighContrastThemeVariant: ThemeVariant => InfoBadge::high_contrast_theme_variant],
    },
});
ferro_impl_classes!(InfoBadge:
    StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl
);

impl FerroObjectImpl for InfoBadge {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let resources = this.resources_changed(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.update_parts();
            }
        });
        let weak = this.to_ref().downgrade();
        let theme = this.actual_theme_variant_changed(move || {
            if let Some(owner) = weak.upgrade() {
                owner.update_parts();
            }
        });
        let weak = this.to_ref().downgrade();
        let settings = this.settings.property_changed(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.update_parts();
            }
        });
        this.subscriptions
            .borrow_mut()
            .extend([resources, theme, settings]);
        this.update_parts();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::value_property().as_property()
            || property == Self::icon_source_property().as_property()
        {
            this.update_display_kind();
        } else if property == Visual::bounds_property().as_property()
            || property == TemplatedControl::corner_radius_property().as_property()
        {
            this.update_corner_radius();
        } else if [
            TemplatedControl::background_property().as_property(),
            TemplatedControl::foreground_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        ]
        .contains(&property)
        {
            this.update_parts();
        }
    }
}

impl LayoutableImpl for InfoBadge {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let desired = Self::parent_measure_override(this, available_size);
        Size::new(desired.width.max(desired.height), desired.height)
    }
}

impl TemplatedControlImpl for InfoBadge {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        // A settings element must not retain an old presenter when a template
        // is replaced. Release that parent's ownership before attaching a new one.
        let old = this.parts.replace(Parts::default());
        if let Some(content) = old.icon_content {
            content.set_content(None);
        }
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        *this.parts.borrow_mut() = Parts {
            root: scope.find_as::<Border>("RootGrid"),
            value: scope.find_as::<TextBlock>("ValueTextBlock"),
            icon: scope.find_as::<Viewbox>("IconPresenter"),
            icon_content: scope.find_as::<ContentPresenter>("IconContentPresenter"),
        };
        this.update_display_kind();
        this.update_corner_radius();
    }
}

ferro_properties! { impl InfoBadge {
    /// -1 selects the icon/dot presentation; nonnegative values are displayed in full.
    pub fn value_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<InfoBadge, _>("Value", StyledPropertyOptions::new(-1_i32).validate(|value| *value >= -1))
    }
    pub fn icon_source_property() -> StyledProperty<Option<Ref<IconSource>>> {
        FerroProperty::register::<InfoBadge, _>("IconSource", None)
    }
    pub fn template_settings_property() -> DirectProperty<InfoBadge, Ref<InfoBadgeTemplateSettings>> {
        FerroProperty::register_direct::<InfoBadge, _>("TemplateSettings", |owner| owner.template_settings(), None, InfoBadgeTemplateSettings::new())
    }
} }

impl InfoBadge {
    /// The typed native HighContrast resource key, with Dark fallback.
    ///
    /// This additive integration API lets the lightweight badge dictionary use
    /// `x:Static` without depending on the optional WinUI theme crate. Its key
    /// and inheritance match `WinUITheme::high_contrast()`.
    pub fn high_contrast_theme_variant() -> ThemeVariant {
        ThemeVariant::new("HighContrast", Some(ThemeVariant::dark()))
    }

    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<InfoBadge>(false);
        Layoutable::min_width_property().override_default_value::<InfoBadge>(4.0);
        Layoutable::min_height_property().override_default_value::<InfoBadge>(4.0);
        Layoutable::max_height_property().override_default_value::<InfoBadge>(16.0);
        Layoutable::affects_measure::<InfoBadge>(&[
            Self::value_property().as_property(),
            Self::icon_source_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        ]);
        let template = FuncControlTemplate::for_type::<InfoBadge>(|_, scope| {
            let root = Border::new();
            root.set_name(Some("RootGrid".into()));
            let grid = Grid::new();
            let text = TextBlock::new();
            text.set_name(Some("ValueTextBlock".into()));
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            text.set_vertical_alignment(VerticalAlignment::Center);
            AutomationProperties::set_accessibility_view(&text, AccessibilityView::Raw);
            grid.children()
                .add(text.register_in_name_scope(&**scope).upcast::<Control>());
            let viewbox = Viewbox::new();
            viewbox.set_name(Some("IconPresenter".into()));
            viewbox.set_horizontal_alignment(HorizontalAlignment::Center);
            viewbox.set_vertical_alignment(VerticalAlignment::Stretch);
            let content = ContentPresenter::new();
            content.set_name(Some("IconContentPresenter".into()));
            viewbox.set_child(content.register_in_name_scope(&**scope));
            grid.children()
                .add(viewbox.register_in_name_scope(&**scope).upcast::<Control>());
            root.set_child(grid);
            root.register_in_name_scope(&**scope).upcast()
        });
        TemplatedControl::template_property().override_default_value::<InfoBadge>(Some(template));
    }

    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            settings: InfoBadgeTemplateSettings::new(),
            parts: RefCell::new(Parts::default()),
            subscriptions: RefCell::new(Vec::new()),
            updating_parts: Cell::new(false),
            update_pending: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn value(&self) -> i32 {
        self.get_value(Self::value_property())
    }
    pub fn set_numeric_value(&self, value: i32) {
        self.set_value(Self::value_property(), value);
    }
    pub fn icon_source(&self) -> Option<Ref<IconSource>> {
        self.get_value(Self::icon_source_property())
    }
    pub fn set_icon_source(&self, value: Option<Ref<IconSource>>) {
        FerroObject::set_value(self, Self::icon_source_property(), value);
    }
    pub fn template_settings(&self) -> Ref<InfoBadgeTemplateSettings> {
        self.settings.clone()
    }

    fn update_display_kind(&self) {
        if self.value() < 0 {
            if let Some(source) = self.icon_source() {
                self.settings
                    .set_icon_element(Some(source.create_icon_element()));
            }
        }
        self.update_parts();
    }

    fn update_corner_radius(&self) {
        let property = TemplatedControl::corner_radius_property();
        let radius = if self.has_local_value(property) {
            self.corner_radius()
        } else {
            CornerRadius::uniform(self.bounds().height / 2.0)
        };
        self.settings.set_info_badge_corner_radius(radius);
    }

    fn resource<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        self.try_find_resource(
            &ResourceKey::from(key),
            self.actual_theme_variant().as_ref(),
        )
        .flatten()
        .and_then(|value| value.downcast_ref::<T>().cloned())
    }

    fn brush(&self, key: &str) -> Option<Rc<dyn IBrush>> {
        let value = self
            .try_find_resource(
                &ResourceKey::from(key),
                self.actual_theme_variant().as_ref(),
            )
            .flatten()?;
        let value = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IBrush>>())?;
        value.downcast_ref::<Rc<dyn IBrush>>().cloned()
    }

    fn update_parts(&self) {
        if self.updating_parts.replace(true) {
            self.update_pending.set(true);
            return;
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.updating_parts);
        loop {
            self.update_pending.set(false);
            self.update_parts_core();
            if !self.update_pending.get() {
                break;
            }
        }
    }

    fn update_parts_core(&self) {
        // Part property notifications may synchronously change the owner. Never
        // hold the parts RefCell across those callbacks.
        let parts = self.parts.borrow().clone();
        let numeric = self.value() >= 0;
        let source = self.icon_source();
        let has_icon = !numeric && source.is_some();
        let font_icon = has_icon
            && source
                .as_ref()
                .is_some_and(|source| source.cast::<FontIconSource>().is_some());
        self.pseudo_classes().set(":value", numeric);
        self.pseudo_classes().set(":dot", !numeric && !has_icon);
        self.pseudo_classes().set(":icon", has_icon && !font_icon);
        self.pseudo_classes().set(":fonticon", font_icon);
        let dark = self.actual_theme_variant().as_ref() == Some(&ThemeVariant::dark());
        let foreground = if self.is_set(TemplatedControl::foreground_property()) {
            self.foreground()
        } else {
            self.brush("InfoBadgeForeground")
                .or_else(|| self.brush("TextOnAccentFillColorPrimaryBrush"))
                .or_else(|| {
                    Some(
                        SolidColorBrush::from_uint32(if dark { 0xFF000000 } else { 0xFFFFFFFF })
                            .into(),
                    )
                })
        };
        if let Some(root) = &parts.root {
            root.set_background(if self.is_set(TemplatedControl::background_property()) {
                self.background()
            } else {
                self.brush("InfoBadgeBackground")
                    .or_else(|| self.brush("AccentFillColorDefaultBrush"))
                    .or_else(|| {
                        Some(
                            SolidColorBrush::from_uint32(if dark {
                                0xFF60CDFF
                            } else {
                                0xFF0078D4
                            })
                            .into(),
                        )
                    })
            });
            root.set_padding(self.padding());
            root.set_corner_radius(self.settings.info_badge_corner_radius());
            TextElement::set_foreground_on(root, foreground.clone());
        }
        if let Some(text) = &parts.value {
            text.set_text(Some(&self.value().to_string()));
            text.set_is_visible(numeric);
            text.set_margin(
                self.resource::<Thickness>("ValueInfoBadgeTextMargin")
                    .unwrap_or(Thickness::new(4.0, 0.0, 4.0, 2.0)),
            );
            text.set_font_size(
                self.resource::<f64>("InfoBadgeValueFontSize")
                    .unwrap_or(11.0),
            );
            text.set_foreground(foreground);
        }
        if let Some(viewbox) = &parts.icon {
            viewbox.set_is_visible(has_icon);
            viewbox.set_margin(if font_icon {
                self.resource::<Thickness>("IconInfoBadgeFontIconMargin")
                    .unwrap_or(Thickness::new(4.0, 0.0, 4.0, 2.0))
            } else {
                self.resource::<Thickness>("IconInfoBadgeIconMargin")
                    .unwrap_or(Thickness::uniform(4.0))
            });
        }
        if let Some(content) = &parts.icon_content {
            let desired = self.settings.icon_element();
            let current = content
                .content()
                .and_then(|value| Control::from_boxed(&value));
            let desired_control = desired
                .as_ref()
                .map(|icon| icon.clone().upcast::<Control>());
            if current != desired_control {
                content.set_content(desired.map(Control::boxed));
            }
        }
    }
}
