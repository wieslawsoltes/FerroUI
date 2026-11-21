use super::RibbonKeyGesture;
pub(crate) fn install_culture_provider() {
    use ferroribbon_core::commands::RibbonGestureNumberFormat;
    use ferroui_base::utilities::CultureInfo;
    use std::rc::Rc;
    RibbonKeyGesture::set_number_format_provider(Some(Rc::new(|| {
        let format = CultureInfo::current_formatting().number_format();
        RibbonGestureNumberFormat {
            positive_sign: format.positive_sign().into(),
            negative_sign: format.negative_sign().into(),
        }
    })));
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::RibbonModifierKeys;
    use ferroui_base::utilities::{CultureInfo, NumberFormatInfo};
    #[test]
    fn gesture_reads_live_framework_current_culture_without_a_core_ui_dependency() {
        let original = CultureInfo::current_formatting();
        crate::register_types();
        CultureInfo::set_current_formatting(
            CultureInfo::invariant_culture().with_number_format(
                NumberFormatInfo::new()
                    .with_positive_sign("~")
                    .with_negative_sign("!"),
            ),
        );
        let g = RibbonKeyGesture::parse(Some(&"F~1".into())).unwrap();
        assert_eq!(g.key, Some("F1".into()));
        assert!(g.matches(Some(&"F~1".into()), RibbonModifierKeys::NONE));
        CultureInfo::set_current_formatting(CultureInfo::invariant_culture());
        assert!(!g.matches(Some(&"F~1".into()), RibbonModifierKeys::NONE));
        assert_eq!(
            RibbonKeyGesture::parse(Some(&"F~1".into())).unwrap().key,
            Some("F~1".into())
        );
        CultureInfo::set_current_formatting(original);
    }
}
