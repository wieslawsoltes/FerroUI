//! Port of the shared RibbonTheme static service. Native theme variants replace
//! ElementTheme
use super::{RibbonThemeResourceError, RibbonThemeResources, to_ribbon_color};
use ferroribbon_core::theming::{RibbonChromeStyle, RibbonThemePalette, RibbonThemeStyle};
use ferroui_base::{
    AttachedProperty, BoxedValue, CornerRadius, FerroLocator, FerroProperty, LocatorExtensions,
    Ref, StyledElement,
    controls::ResourceDictionary,
    data::{BindingPriority, core::ValueTypes},
    ferro_property,
    media::{Brush, Color},
    metadata::{MarkupDelegate, MarkupInvokeError, MarkupTyped},
    platform::{ColorContrastPreference, IPlatformSettings},
    reactive::{Disposable, IDisposable},
    styling::ThemeVariant,
    utilities::EventArgs,
};
use std::{cell::RefCell, rc::Rc};

/// Rust error carrier for the source argument exceptions. Native property type
/// errors continue through the framework property engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RibbonThemeError {
    pub parameter: &'static str,
    pub empty: bool,
}
impl std::fmt::Display for RibbonThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (Parameter '{}')",
            if self.empty {
                "The value cannot be an empty string."
            } else {
                "Value cannot be null."
            },
            self.parameter
        )
    }
}
impl std::error::Error for RibbonThemeError {}
fn required<T>(value: Option<T>, parameter: &'static str) -> Result<T, RibbonThemeError> {
    value.ok_or(RibbonThemeError {
        parameter,
        empty: false,
    })
}
fn key(value: Option<&str>) -> Result<&str, RibbonThemeError> {
    let value = required(value, "key")?;
    if value.is_empty() {
        Err(RibbonThemeError {
            parameter: "key",
            empty: true,
        })
    } else {
        Ok(value)
    }
}

// The source event owns this receiver strongly; the Rust disposal token owns
// only a Weak receiver. An owner may keep its token without forming a new cycle.
type ChangedCallbackFn = dyn Fn(Option<&ferroui_base::FerroObject>, &EventArgs);
struct ChangedCallback(Box<ChangedCallbackFn>);
impl PartialEq for ChangedCallback {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class ChangedCallback {
    namespace:"FerroRibbon.Internal",this:Rc<ChangedCallback>,handles:[ChangedCallback,Rc<ChangedCallback>],
    methods:[fn Invoke(Option<BoxedValue>,EventArgs)=>|callback:&Rc<ChangedCallback>,_:Option<BoxedValue>,args:EventArgs|(callback.0)(None,&args)],
});
fn changed_callback_delegate(callback: Rc<ChangedCallback>) -> MarkupDelegate {
    MarkupDelegate::for_method(
        Some(callback),
        ChangedCallback::MARKUP,
        &ChangedCallback::MARKUP.methods[0],
    )
}

/// Failures from resource mutation or the source Changed invocation. A handler
/// failure follows successful mutation and stops the remaining invocation list.
#[derive(Clone, Debug, PartialEq)]
pub enum RibbonThemeApplyError {
    Resources(RibbonThemeResourceError),
    Handler(MarkupInvokeError),
}
impl std::fmt::Display for RibbonThemeApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resources(e) => e.fmt(f),
            Self::Handler(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for RibbonThemeApplyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Resources(e) => e,
            Self::Handler(e) => e,
        })
    }
}
impl From<RibbonThemeResourceError> for RibbonThemeApplyError {
    fn from(value: RibbonThemeResourceError) -> Self {
        Self::Resources(value)
    }
}
impl From<MarkupInvokeError> for RibbonThemeApplyError {
    fn from(value: MarkupInvokeError) -> Self {
        Self::Handler(value)
    }
}
ferroui_base::ferro_thread_local! {
    // Native service state remains UI-thread-affine, unlike CLR process statics.
    static RESOURCES: RefCell<Option<Ref<RibbonThemeResources>>> = const { RefCell::new(None) };
    static CHANGED: RefCell<Option<MarkupDelegate>> = const { RefCell::new(None) };
}
type Registrations = Rc<RefCell<Vec<(&'static FerroProperty, String)>>>;
/// Runtime palette, chrome and subtree theme service. Overload suffixes preserve
/// traceability where Rust cannot express the original overload set.
pub struct RibbonTheme;
ferroui_base::ferro_static_type!(RibbonTheme);
impl RibbonTheme {
    ferro_property!(for RibbonTheme;
        fn registrations_property() -> AttachedProperty<Option<Registrations>> {
            FerroProperty::register_attached::<RibbonTheme, StyledElement, _>("FerroRibbonThemeBrushRegistrations", None)
        }
    );
    pub fn ensure_resources() -> Ref<RibbonThemeResources> {
        if let Some(r) = RESOURCES.with_borrow(Clone::clone) {
            return r;
        }
        if let Some(app) = ferroui_controls::Application::current() {
            let r = Self::find_merged(&app.resources()).unwrap_or_else(|| {
                let r = RibbonThemeResources::new();
                // Source assigns its cache before Add can invoke resource observers.
                RESOURCES.with_borrow_mut(|cache| *cache = Some(r.clone()));
                app.resources().add_merged_dictionary(r.clone());
                r
            });
            RESOURCES.with_borrow_mut(|cache| *cache = Some(r.clone()));
            r
        } else {
            let r = RibbonThemeResources::new();
            RESOURCES.with_borrow_mut(|cache| *cache = Some(r.clone()));
            r
        }
    }
    fn find_merged(dictionary: &ResourceDictionary) -> Option<Ref<RibbonThemeResources>> {
        for provider in dictionary.merged_dictionaries().snapshot().iter() {
            if let Some(object) = provider.as_object() {
                if let Some(resources) = object.downcast_ref::<RibbonThemeResources>() {
                    return Some(resources.to_ref());
                }
                if let Some(nested) = object.downcast_ref::<ResourceDictionary>()
                    && let Some(resources) = Self::find_merged(nested)
                {
                    return Some(resources);
                }
            }
        }
        None
    }
    pub fn resources() -> Ref<RibbonThemeResources> {
        Self::ensure_resources()
    }
    pub fn palette() -> Rc<RibbonThemePalette> {
        Self::resources().palette()
    }
    pub fn chrome_style() -> RibbonChromeStyle {
        Self::resources().chrome_style()
    }
    pub fn style() -> RibbonThemeStyle {
        Self::resources().style()
    }
    /// Adds the source EventHandler invocation list. Null does nothing; handlers
    /// strongly retain their targets until removed, matching the source static event.
    pub fn add_changed(handler: Option<MarkupDelegate>) {
        let old = CHANGED.with_borrow_mut(|event| {
            let next = MarkupDelegate::combine(event.as_ref(), handler.as_ref());
            std::mem::replace(event, next)
        });
        drop(old);
    }
    /// Removes the last equal contiguous invocation list, including a sequence
    /// spanning additions. Delegate target equality never calls target.Equals.
    pub fn remove_changed(handler: Option<MarkupDelegate>) {
        let old = CHANGED.with_borrow_mut(|event| {
            let next = MarkupDelegate::remove(event.as_ref(), handler.as_ref());
            std::mem::replace(event, next)
        });
        drop(old);
    }
    /// Rust subscription convenience over the SAME source event. Explicit disposal
    /// removes this callback; dropping the token does not silently unsubscribe.
    pub fn changed(
        handler: impl Fn(Option<&ferroui_base::FerroObject>, &EventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        ValueTypes::register_reference::<ChangedCallback>();
        let callback = Rc::new(ChangedCallback(Box::new(handler)));
        let weak = Rc::downgrade(&callback);
        Self::add_changed(Some(changed_callback_delegate(callback)));
        Disposable::create(move || {
            if let Some(callback) = weak.upgrade() {
                Self::remove_changed(Some(changed_callback_delegate(callback)));
            }
        })
    }
    fn raise_changed() -> Result<(), MarkupInvokeError> {
        // Clone the invocation list before entering callbacks. Edits affect nested
        // and later invocations, never the current snapshot.
        if let Some(handler) = CHANGED.with_borrow(Clone::clone) {
            handler.try_invoke(&[None, Some(crate::compat::empty_event_args())])?;
        }
        Ok(())
    }
    pub fn apply_palette(
        palette: Option<Rc<RibbonThemePalette>>,
    ) -> Result<(), RibbonThemeApplyError> {
        Self::apply(palette, Self::chrome_style())
    }
    pub fn apply_accent(accent: Color) -> Result<(), RibbonThemeApplyError> {
        Self::apply_palette(Some(RibbonThemePalette::from_accent(to_ribbon_color(
            accent,
        ))))
    }
    pub fn apply_chrome_style(style: RibbonChromeStyle) -> Result<(), RibbonThemeApplyError> {
        Self::apply(Some(Self::palette()), style)
    }
    pub fn apply(
        palette: Option<Rc<RibbonThemePalette>>,
        chrome: RibbonChromeStyle,
    ) -> Result<(), RibbonThemeApplyError> {
        Self::resources().apply_palette_and_chrome(palette, chrome)?;
        Self::raise_changed()?;
        Ok(())
    }
    pub fn apply_style(style: RibbonThemeStyle) -> Result<(), RibbonThemeApplyError> {
        // Preserve source evaluation order: Resources, Palette, ChromeStyle.
        Self::resources().apply(Some(Self::palette()), Self::chrome_style(), style)?;
        Self::raise_changed()?;
        Ok(())
    }
    pub fn apply_with_style(
        palette: Option<Rc<RibbonThemePalette>>,
        chrome: RibbonChromeStyle,
        style: RibbonThemeStyle,
    ) -> Result<(), RibbonThemeApplyError> {
        Self::resources().apply(palette, chrome, style)?;
        Self::raise_changed()?;
        Ok(())
    }
    pub fn set_brush_color(key: &str, color: Color) {
        Self::set_brush_color_for_theme(key, color, Some("Light"));
    }
    pub fn set_brush_color_for_theme(key: &str, color: Color, theme: Option<&str>) {
        Self::resources().set_color(key, color, theme);
    }
    pub fn set_theme(
        root: Option<&StyledElement>,
        theme: Option<ThemeVariant>,
    ) -> Result<(), RibbonThemeError> {
        required(root, "root")?.set_value(ThemeVariant::requested_theme_variant_property(), theme);
        Ok(())
    }
    fn application_theme() -> ThemeVariant {
        if ferroui_controls::Application::current().and_then(|app| app.requested_theme_variant())
            == Some(ThemeVariant::dark())
        {
            ThemeVariant::dark()
        } else {
            ThemeVariant::light()
        }
    }
    fn theme(scope: Option<&StyledElement>) -> ThemeVariant {
        if scope
            .and_then(StyledElement::actual_theme_variant)
            .unwrap_or_else(Self::application_theme)
            == ThemeVariant::dark()
        {
            ThemeVariant::dark()
        } else {
            ThemeVariant::light()
        }
    }
    fn high_contrast() -> bool {
        FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .is_some_and(|settings| {
                settings.get_color_values().contrast_preference() == ColorContrastPreference::High
            })
    }
    pub fn get_brush(
        scope: Option<&StyledElement>,
        brush_key: Option<&str>,
    ) -> Result<Option<Ref<Brush>>, RibbonThemeError> {
        let key = key(brush_key)?;
        let theme = if Self::high_contrast() {
            "HighContrast"
        } else if Self::theme(scope) == ThemeVariant::dark() {
            "Dark"
        } else {
            "Light"
        };
        if let Some(brush) = Self::resources().get_brush(key, Some(theme)) {
            return Ok(Some(brush.upcast()));
        }
        Ok(ferroui_controls::Application::current()
            .and_then(|app| {
                app.resources()
                    .try_get_resource(&key.into(), None)
                    .flatten()
            })
            .and_then(|v| ValueTypes::as_object(&*v))
            .and_then(|object| object.cast::<Brush>()))
    }
    fn assign_brush(
        target: &StyledElement,
        property: &'static FerroProperty,
        brush: Option<Ref<Brush>>,
    ) {
        let value: BoxedValue =
            if property.property_type() == std::any::TypeId::of::<Option<BoxedValue>>() {
                Rc::new(brush.map(|b| Rc::new(b) as BoxedValue))
            } else if property.property_type() == std::any::TypeId::of::<Option<Ref<Brush>>>() {
                Rc::new(brush)
            } else {
                Rc::new(brush.map(Rc::<dyn ferroui_base::media::IBrush>::from))
            };
        target.set_value_untyped(property, &*value, BindingPriority::LocalValue);
    }
    pub fn set_theme_brush(
        target: Option<&StyledElement>,
        property: Option<&'static FerroProperty>,
        brush_key: Option<&str>,
    ) -> Result<(), RibbonThemeError> {
        let target = required(target, "target")?;
        let property = required(property, "property")?;
        Self::assign_brush(target, property, Self::get_brush(Some(target), brush_key)?);
        let registrations = target
            .get_value(Self::registrations_property())
            .unwrap_or_else(|| {
                let registrations = Rc::new(RefCell::new(Vec::new()));
                target.set_value(Self::registrations_property(), Some(registrations.clone()));
                registrations
            });
        let count = {
            let mut entries = registrations.borrow_mut();
            let key = brush_key.expect("GetBrush validated key").to_owned();
            if let Some(entry) = entries.iter_mut().find(|(p, _)| *p == property) {
                entry.1 = key;
            } else {
                entries.push((property, key));
            }
            entries.len()
        };
        if count == 1 {
            let weak = target.to_ref().downgrade();
            // The source intentionally tests Count, not whether already subscribed.
            // Dropping a native IDisposable token does not unsubscribe its handler.
            target
                .actual_theme_variant_changed(move || {
                    if let Some(target) = weak.upgrade() {
                        Self::update_theme_brushes(&target);
                    }
                })
                .detach();
        }
        Ok(())
    }
    fn update_theme_brushes(target: &StyledElement) {
        if let Some(entries) = target.get_value(Self::registrations_property()) {
            let count = entries.borrow().len();
            for i in 0..count {
                let (property, key) = entries.borrow()[i].clone();
                Self::assign_brush(
                    target,
                    property,
                    Self::get_brush(Some(target), Some(&key)).expect("registered key"),
                );
                // Modern .NET Dictionary permits value replacement in enumeration,
                // but adding an entry invalidates the enumerator at the next MoveNext.
                assert_eq!(
                    entries.borrow().len(),
                    count,
                    "Collection was modified; enumeration operation may not execute."
                );
            }
        }
    }
    pub fn get_corner_radius(
        scope: Option<&StyledElement>,
        key: &str,
        fallback: f64,
    ) -> CornerRadius {
        let variant = Self::theme(scope);
        Self::resources()
            .theme_dictionary(&variant)
            .and_then(|p| {
                p.as_object()
                    .and_then(|o| o.downcast_ref::<ResourceDictionary>())
                    .and_then(|d| d.try_get_value(&key.into()).flatten())
            })
            .and_then(|v| v.downcast_ref::<CornerRadius>().copied())
            .unwrap_or_else(|| CornerRadius::uniform(fallback))
    }
    #[doc(hidden)]
    pub fn markup_get_brush(
        scope: Option<Ref<StyledElement>>,
        key: Option<String>,
    ) -> Result<Option<Ref<Brush>>, RibbonThemeError> {
        Self::get_brush(scope.as_deref(), key.as_deref())
    }
    #[doc(hidden)]
    pub fn markup_set_theme(
        root: Option<Ref<StyledElement>>,
        theme: Option<ThemeVariant>,
    ) -> Result<(), RibbonThemeError> {
        Self::set_theme(root.as_deref(), theme)
    }
    #[doc(hidden)]
    pub fn markup_set_theme_brush(
        target: Option<Ref<StyledElement>>,
        property: Option<&'static FerroProperty>,
        key: Option<String>,
    ) -> Result<(), RibbonThemeError> {
        Self::set_theme_brush(target.as_deref(), property, key.as_deref())
    }
    #[doc(hidden)]
    pub fn markup_set_brush_color(
        key: Option<String>,
        color: Color,
    ) -> Result<(), RibbonThemeError> {
        Self::markup_set_brush_color_for_theme(key, color, Some("Light".into()))
    }
    #[doc(hidden)]
    pub fn markup_set_brush_color_for_theme(
        key: Option<String>,
        color: Color,
        theme: Option<String>,
    ) -> Result<(), RibbonThemeError> {
        // Source evaluates Resources before SetColor validates a dictionary key.
        let resources = Self::resources();
        let key = required(key, "key")?;
        resources.set_color(&key, color, theme.as_deref());
        Ok(())
    }
    #[doc(hidden)]
    pub fn markup_get_corner_radius(
        scope: Option<Ref<StyledElement>>,
        key: Option<String>,
        fallback: f64,
    ) -> Result<CornerRadius, RibbonThemeError> {
        let variant = Self::theme(scope.as_deref());
        let resources = Self::resources();
        if let Some(dictionary) = resources.theme_dictionary(&variant)
            && let Some(dictionary) = dictionary
                .as_object()
                .and_then(|o| o.downcast_ref::<ResourceDictionary>())
        {
            let key = required(key, "key")?;
            if let Some(radius) = dictionary
                .try_get_value(&key.into())
                .flatten()
                .and_then(|v| v.downcast_ref::<CornerRadius>().copied())
            {
                return Ok(radius);
            }
        }
        Ok(CornerRadius::uniform(fallback))
    }
    #[cfg(test)]
    pub(crate) fn changed_test_snapshot() -> Option<MarkupDelegate> {
        CHANGED.with_borrow(Clone::clone)
    }
    #[cfg(test)]
    pub(crate) fn reset_test_changed() {
        let old = CHANGED.with_borrow_mut(Option::take);
        drop(old);
    }
    #[cfg(test)]
    pub(crate) fn reset_test_cache() {
        RESOURCES.with_borrow_mut(|cache| *cache = None);
    }
}

ferroui_base::ferro_markup_type!(static RibbonTheme {
    namespace:"FerroRibbon.Controls",type_info:RibbonTheme,
    static_properties:[
        Resources:Ref<RibbonThemeResources> {get:RibbonTheme::resources},
        Palette:Rc<RibbonThemePalette> {get:RibbonTheme::palette},
        ChromeStyle:RibbonChromeStyle {get:RibbonTheme::chrome_style},
        Style:RibbonThemeStyle {get:RibbonTheme::style},
    ],
    events:[static Changed(Option<BoxedValue>,EventArgs) {
        add:RibbonTheme::add_changed, remove:RibbonTheme::remove_changed,
    }],
    methods:[
        static fn EnsureResources()->Ref<RibbonThemeResources> => RibbonTheme::ensure_resources,
        static try fn ApplyPalette(Option<Rc<RibbonThemePalette>>) => RibbonTheme::apply_palette,
        static try fn ApplyAccent(Color) => RibbonTheme::apply_accent,
        static try fn ApplyChromeStyle(RibbonChromeStyle) => RibbonTheme::apply_chrome_style,
        static try fn Apply(Option<Rc<RibbonThemePalette>>,RibbonChromeStyle) => RibbonTheme::apply,
        static try fn ApplyStyle(RibbonThemeStyle) => RibbonTheme::apply_style,
        static try fn Apply(Option<Rc<RibbonThemePalette>>,RibbonChromeStyle,RibbonThemeStyle) => RibbonTheme::apply_with_style,
        static try fn SetBrushColor(Option<String>,Color) => RibbonTheme::markup_set_brush_color,
        static try fn SetBrushColor(Option<String>,Color,Option<String>) => RibbonTheme::markup_set_brush_color_for_theme,
        static try fn SetTheme(Option<Ref<StyledElement>>,Option<ThemeVariant>) => RibbonTheme::markup_set_theme,
        static try fn GetBrush(Option<Ref<StyledElement>>,Option<String>)->Option<Ref<Brush>> => RibbonTheme::markup_get_brush,
        static try fn SetThemeBrush(Option<Ref<StyledElement>>,Option<&'static FerroProperty>,Option<String>) => RibbonTheme::markup_set_theme_brush,
        static try fn GetCornerRadius(Option<Ref<StyledElement>>,Option<String>,f64)->CornerRadius => RibbonTheme::markup_get_corner_radius,
        static fn ToColor(ferroribbon_core::theming::RibbonColor)->Color => super::to_color,
        static fn ToRibbonColor(Color)->ferroribbon_core::theming::RibbonColor => super::to_ribbon_color,
    ],
});
