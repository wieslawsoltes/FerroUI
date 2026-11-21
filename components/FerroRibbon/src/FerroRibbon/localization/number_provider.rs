//! Native data for the existing FerroUI service; registered in its owning scope.
use ferroui_base::{
    FerroLocator, LocatorExtensions,
    utilities::{ICultureDataProvider, Locale, NumberFormatInfo},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Default)]
struct NativeNumberProvider {
    numbers: RefCell<HashMap<String, Rc<NumberFormatInfo>>>,
}
impl ICultureDataProvider for NativeNumberProvider {
    fn get_number_format_by_locale(&self, locale: &Locale) -> Option<Rc<NumberFormatInfo>> {
        self.for_name(&locale.to_string())
    }

    // FerroUI still dispatches legacy culture names through this method.
    fn get_number_format(&self, name: &str) -> Option<Rc<NumberFormatInfo>> {
        self.for_name(name)
    }
}
impl NativeNumberProvider {
    fn for_name(&self, name: &str) -> Option<Rc<NumberFormatInfo>> {
        let key = name.to_ascii_lowercase();
        if let Some(value) = self.numbers.borrow().get(&key) {
            return Some(value.clone());
        }
        // This trait cannot return an error. Do not hide a native failure behind
        // its None/invariant fallback; the Result API is exposed by Platform.
        let value = Rc::new(
            ferroribbon_platform::native_number_format(name)
                .unwrap_or_else(|error| panic!("FerroRibbon number data for {name:?}: {error}")),
        );
        let old = self.numbers.borrow_mut().insert(key, value.clone());
        drop(old);
        Some(value)
    }
}
pub(super) fn install() {
    if FerroLocator::current()
        .get_service::<dyn ICultureDataProvider>()
        .is_none()
    {
        FerroLocator::current_mutable()
            .bind::<dyn ICultureDataProvider>()
            .to_constant(Rc::new(NativeNumberProvider::default()) as Rc<dyn ICultureDataProvider>);
    }
}
