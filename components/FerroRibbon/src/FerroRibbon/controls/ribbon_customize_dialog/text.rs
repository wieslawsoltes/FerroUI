//! Product-specific command sorting/filtering through actual native text services.
use super::*;
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "emscripten",
    target_os = "linux"
))]
use ferroui_base::utilities::CultureInfo;
use ferroui_base::utilities::{CompareOptions, ICompareRules};
use std::cmp::Ordering;
pub(super) enum CommandText {
    Rules(Rc<dyn ICompareRules>),
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "emscripten",
        target_os = "linux"
    ))]
    Native(ferroribbon_platform::SearchLabelCollator),
}
impl CommandText {
    pub fn current() -> DialogResult<Self> {
        if let Some(rules) = crate::search::current_compare_rules() {
            return Ok(Self::Rules(rules));
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        ))]
        {
            ferroribbon_platform::SearchLabelCollator::new(CultureInfo::current_formatting().name())
                .map(Self::Native)
                .map_err(|e| RibbonCustomizeDialogError::TextService(e.to_string()))
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        )))]
        {
            Err(RibbonCustomizeDialogError::TextService("Current-culture command sorting/filtering requires a native culture provider on this platform".into()))
        }
    }
    pub fn compare(&self, left: &RibbonString, right: &RibbonString) -> Ordering {
        match self {
            Self::Rules(rules) => rules.compare(
                &left.to_utf8().expect("native command label"),
                &right.to_utf8().expect("native command label"),
                CompareOptions::IGNORE_CASE,
            ),
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "emscripten",
                target_os = "linux"
            ))]
            Self::Native(native) => native
                .compare(left.as_utf16(), right.as_utf16())
                .expect("native command label comparison"),
        }
    }
    pub fn contains(&self, label: &str, query: &str) -> bool {
        match self {
            Self::Rules(rules) => rules.index_of(label, query, CompareOptions::IGNORE_CASE) >= 0,
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "emscripten",
                target_os = "linux"
            ))]
            Self::Native(native) => native
                .contains(
                    &label.encode_utf16().collect::<Vec<_>>(),
                    &query.encode_utf16().collect::<Vec<_>>(),
                )
                .expect("native command filtering"),
        }
    }
}
