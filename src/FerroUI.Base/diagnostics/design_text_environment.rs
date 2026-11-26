//! Inherited text settings supplied by a developer preview.
use crate::utilities::{CultureInfo, Locale, current_ui_locale, set_current_ui_locale};
use crate::{AttachedProperty, FerroObject, FerroProperty, Ref, StyledPropertyOptions, TypeInfo};
use std::cell::OnceCell;

static OWNER: TypeInfo =
    TypeInfo::new("DesignTextEnvironment", None).with_module_path(module_path!());

crate::ferro_thread_local! {
    static PROPERTY: OnceCell<&'static AttachedProperty<Option<DesignTextEnvironment>>> =
        const { OnceCell::new() };
}

/// Locale and font-size multiplier scoped to a native preview subtree.
#[derive(Clone, Debug, PartialEq)]
pub struct DesignTextEnvironment {
    culture: Option<CultureInfo>,
    scale: f64,
}

crate::styled_value!(DesignTextEnvironment);

impl DesignTextEnvironment {
    /// Constructs a nonempty override for a preview subtree.
    pub fn new(locale: Option<Locale>, scale: f64) -> Option<Self> {
        (scale.is_finite() && scale > 0.0 && (locale.is_some() || scale != 1.0)).then(|| Self {
            culture: locale.map(CultureInfo::from_locale),
            scale,
        })
    }

    /// Installs the override before the preview's children are attached.
    pub fn install(&self, root: &FerroObject) {
        let property = PROPERTY.with(|slot| {
            *slot.get_or_init(|| {
                FerroProperty::register_attached_with_core(
                    &OWNER,
                    FerroObject::TYPE,
                    "DesignTextEnvironment",
                    StyledPropertyOptions::new(None).inherits(true),
                )
            })
        });
        root.set_value(property, Some(self.clone()));
    }

    /// Returns the settings inherited by the object, when its tree supplies them.
    pub fn get(target: &FerroObject) -> Option<Self> {
        target.get_value(Self::registered_property()?)
    }

    pub fn locale(&self) -> Option<&Locale> {
        self.culture.as_ref().map(CultureInfo::locale)
    }

    pub fn scale_font(&self, size: f64) -> f64 {
        size * self.scale
    }

    pub fn font_size(target: &FerroObject, size: f64) -> f64 {
        Self::get(target).map_or(size, |settings| settings.scale_font(size))
    }

    /// Use the target's formatting locale for bindings without an explicit converter culture.
    pub(crate) fn formatting_for(target: impl FnOnce() -> Option<Ref<FerroObject>>) -> CultureInfo {
        Self::registered_property()
            .and_then(|property| target()?.get_value(property)?.culture)
            .unwrap_or_else(CultureInfo::current_formatting)
    }

    /// Runs synchronous construction or input that needs the preview's ambient culture.
    /// Binding evaluation and text layout can also read the retained subtree settings.
    pub fn run<T>(environment: Option<&Self>, work: impl FnOnce() -> T) -> T {
        let _restore = environment
            .and_then(|settings| settings.culture.as_ref())
            .map(|culture| {
                let previous =
                    RestoreLocale(CultureInfo::current_formatting(), current_ui_locale());
                CultureInfo::set_current_formatting(culture.clone());
                set_current_ui_locale(culture.locale().clone());
                previous
            });
        work()
    }

    fn registered_property() -> Option<&'static AttachedProperty<Option<Self>>> {
        PROPERTY.with(|slot| slot.get().copied())
    }
}

struct RestoreLocale(CultureInfo, Locale);

impl Drop for RestoreLocale {
    fn drop(&mut self) {
        CultureInfo::set_current_formatting(self.0.clone());
        set_current_ui_locale(self.1.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::locale;

    #[test]
    fn stage_text_environment_is_inherited_by_only_the_attached_tree() {
        let root = FerroObject::new();
        let child = FerroObject::new();
        child.set_inheritance_parent(root.clone());
        let sibling = FerroObject::new();
        let original = CultureInfo::current_formatting();
        let settings = DesignTextEnvironment::new(Some(locale!("fr-CA")), 1.5).unwrap();
        settings.install(&root);
        assert_eq!(DesignTextEnvironment::font_size(&child, 20.0), 30.0);
        assert_eq!(DesignTextEnvironment::font_size(&sibling, 20.0), 20.0);
        assert_eq!(
            DesignTextEnvironment::formatting_for(|| Some(child)).name(),
            "fr-CA"
        );
        assert_eq!(
            DesignTextEnvironment::formatting_for(|| Some(sibling)),
            original
        );
        let ui_locale = current_ui_locale();
        DesignTextEnvironment::run(Some(&settings), || {
            assert_eq!(current_ui_locale(), locale!("fr-CA"));
            assert_eq!(CultureInfo::current_formatting().name(), "fr-CA");
        });
        assert_eq!(current_ui_locale(), ui_locale);
        assert_eq!(CultureInfo::current_formatting(), original);
    }
}
