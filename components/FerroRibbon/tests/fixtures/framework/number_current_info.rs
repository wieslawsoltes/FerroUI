//! Framework-only behavior reproducer: custom current-culture format identity.
use ferroui_base::utilities::{CultureInfo, NumberFormatInfo};
#[test]
fn current_number_info_retains_explicit_culture_override_identity() {
    struct Restore(CultureInfo);
    impl Drop for Restore {
        fn drop(&mut self) {
            CultureInfo::set_current_formatting(self.0.clone());
        }
    }
    let _restore = Restore(CultureInfo::current_formatting());
    let culture = CultureInfo::invariant_culture()
        .with_number_format(NumberFormatInfo::new().with_number_decimal_separator(","));
    let expected = culture.number_format();
    CultureInfo::set_current_formatting(culture);
    assert!(
        std::rc::Rc::ptr_eq(
            &CultureInfo::current_formatting().number_format(),
            &expected
        ),
        "Formatting culture must retain the exact explicit number-format instance"
    );
}
