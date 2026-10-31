//! Native context/provider adapters for the shared original System.TextInfo.
use super::{i_culture_data_provider::find_culture_data, CultureInfo};
#[cfg(test)]
use crate::Utf16Text;
pub use ferroui_system::{TextCasingError, TextCasingService, TextInfo, TextUppercaseCallback};
use std::rc::Rc;
/// The conventions of `culture`: the data the registered provider has
/// for the culture or the nearest of its parents, else the invariant
/// data.
pub(crate) fn provided_for_culture(culture: &CultureInfo) -> Option<Rc<TextInfo>> {
    find_culture_data(culture, |provider, name| provider.get_text_info(name))
}
pub(crate) fn fallback_for_culture(culture: &CultureInfo) -> Rc<TextInfo> {
    if culture.is_invariant() {
        TextInfo::invariant_info()
    } else {
        Rc::new(TextInfo::for_language(culture.data_name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_list_separator_is_a_comma() {
        assert_eq!(
            ",",
            CultureInfo::invariant_culture()
                .text_info()
                .list_separator()
        );
        assert_eq!(",", TextInfo::invariant_info().list_separator());
    }

    #[test]
    fn culture_without_data_uses_the_invariant_list_separator() {
        assert_eq!(
            ",",
            CultureInfo::get_culture_info("pl-PL")
                .text_info()
                .list_separator()
        );
    }

    #[test]
    fn list_separator_can_be_changed() {
        let info = TextInfo::new().with_list_separator(";");
        assert_eq!(";", info.list_separator());
    }
}

#[cfg(test)]
mod exact_casing_tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn empty_and_null_precede_callbacks_and_original_units_and_errors_are_retained() {
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let info = TextInfo::new().with_uppercase_callback(Rc::new(move |text| {
            count.set(count.get() + 1);
            if count.get() == 1 {
                Ok(text.clone())
            } else {
                Err(TextCasingError::Failed("body".into()))
            }
        }));
        assert_eq!(
            info.try_to_upper_text(None),
            Err(TextCasingError::NullArgument)
        );
        let empty = Utf16Text::default();
        assert!(info.try_to_upper_text(Some(&empty)).unwrap().ptr_eq(&empty));
        assert_eq!(calls.get(), 0);
        let text = Utf16Text::from_units(vec![0xd800, 65, 0xdfff]);
        assert!(info.try_to_upper_text(Some(&text)).unwrap().ptr_eq(&text));
        assert_eq!(
            info.try_to_upper_text(Some(&text)),
            Err(TextCasingError::Failed("body".into()))
        );
        assert!(
            TextInfo::for_language("en-US")
                == TextInfo::for_language("en-US").with_list_separator(";")
        );
        assert!(TextInfo::for_language("en-US") != TextInfo::for_language("tr"));
    }
    #[test]
    fn fallback_text_info_is_shared_across_culture_aliases_without_freezing_explicit_providers() {
        use crate::{FerroLocator, LocatorExtensions};
        struct Provider {
            info: Rc<TextInfo>,
            calls: Cell<usize>,
        }
        impl super::super::ICultureDataProvider for Provider {
            fn get_text_info(&self, _: &str) -> Option<Rc<TextInfo>> {
                self.calls.set(self.calls.get() + 1);
                Some(self.info.clone())
            }
        }
        let culture = CultureInfo::get_culture_info("tr");
        let alias = culture.clone();
        let original = culture.text_info();
        assert!(Rc::ptr_eq(&original, &alias.text_info()));
        let provided = Rc::new(TextInfo::for_language("en-US"));
        let provider = Rc::new(Provider {
            info: provided.clone(),
            calls: Cell::new(0),
        });
        {
            let scope = FerroLocator::enter_scope();
            FerroLocator::current_mutable()
                .bind::<dyn super::super::ICultureDataProvider>()
                .to_constant(provider.clone() as Rc<dyn super::super::ICultureDataProvider>);
            assert!(Rc::ptr_eq(&provided, &culture.text_info()));
            assert!(Rc::ptr_eq(&provided, &alias.text_info()));
            assert_eq!(provider.calls.get(), 2);
            scope.dispose();
        }
        assert!(Rc::ptr_eq(&original, &culture.text_info()));
    }
}
