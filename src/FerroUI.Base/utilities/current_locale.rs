//! The ambient locale of a thread: the language its text is in and the culture it formats
//! with (C5 of #423, docs/idiomatic-rust-signatures.md section 3.4).
//!
//! .NET keeps the ambient culture in `CultureInfo.CurrentCulture` and
//! `CultureInfo.CurrentUICulture`, thread-local values. Here the ambient value of a thread is
//! its [`Locale`]: [`current_locale`] and [`set_current_locale`] (the language of text, font
//! matching and shaping, the identity of the culture), and [`current_ui_locale`] and
//! [`set_current_ui_locale`] (the language of the strings of the user interface). Both are
//! the unknown locale `und` (the invariant culture) until something sets them; nothing in the
//! framework sets them, the application does.
//!
//! Formatting keeps [`CultureInfo`] as its parameter (decision O3 of the plan): a culture
//! carries number and date conventions, and a culture can carry conventions of its own
//! attached to it ([`CultureInfo::with_number_format`],
//! [`CultureInfo::with_date_time_format`]), which a locale cannot hold. The thread therefore
//! keeps, beside its locale, the culture it formats with ([`CultureInfo::current_formatting`]):
//! the culture of the locale, or the culture with attached conventions that
//! [`CultureInfo::set_current_formatting`] sets (the locale is then the locale of that
//! culture). The two are one ambient value and always change together.

use std::cell::RefCell;

use icu_locale_core::Locale;

use super::CultureInfo;

/// The ambient culture of a thread: its locale, and the culture it formats with, whose
/// identity is the locale.
struct Ambient {
    locale: Locale,
    formatting: CultureInfo,
}

crate::ferro_thread_local! {
    static CURRENT: RefCell<Ambient> =
        RefCell::new(Ambient { locale: Locale::UNKNOWN, formatting: CultureInfo::invariant_culture() });
    static CURRENT_UI: RefCell<Locale> = RefCell::new(Locale::UNKNOWN);
}

/// The locale of the current thread (the identity of .NET's `CultureInfo.CurrentCulture`):
/// the language of its text. The unknown locale `und`, the invariant culture, until it is set.
pub fn current_locale() -> Locale {
    CURRENT.with(|current| current.borrow().locale.clone())
}

/// Sets the locale of the current thread. The thread formats with the culture of the
/// locale from now on ([`CultureInfo::current_formatting`]); conventions attached to the
/// culture it formatted with before are dropped.
pub fn set_current_locale(locale: Locale) {
    let formatting = CultureInfo::from(&locale);
    CURRENT.with(|current| *current.borrow_mut() = Ambient { locale, formatting });
}

/// The user interface locale of the current thread (.NET's `CultureInfo.CurrentUICulture`):
/// the language of the strings of the user interface. The unknown locale `und` until it is
/// set.
pub fn current_ui_locale() -> Locale {
    CURRENT_UI.with(|current| current.borrow().clone())
}

/// Sets the user interface locale of the current thread.
pub fn set_current_ui_locale(locale: Locale) {
    CURRENT_UI.with(|current| *current.borrow_mut() = locale);
}

impl CultureInfo {
    /// The culture the current thread formats with (the conventions of .NET's
    /// `CultureInfo.CurrentCulture`): the culture of [`current_locale`], with the
    /// conventions attached to it when it was set with
    /// [`set_current_formatting`](Self::set_current_formatting). Cloning it is cheap.
    pub fn current_formatting() -> CultureInfo {
        CURRENT.with(|current| current.borrow().formatting.clone())
    }

    /// Sets the culture the current thread formats with, the conventions attached to it
    /// included, and the locale of the thread to the locale of the culture
    /// ([`CultureInfo::locale`]). A culture whose name is not a BCP-47 language tag
    /// (`es-ES_tradnl`, `x-custom`), which has no locale of its own, leaves the locale the
    /// unknown locale `und` while the thread formats with the culture as it is.
    ///
    /// A culture without attached conventions is the same as
    /// [`set_current_locale`] with its locale.
    pub fn set_current_formatting(culture: CultureInfo) {
        let locale = culture.locale().clone();
        CURRENT.with(|current| *current.borrow_mut() = Ambient { locale, formatting: culture });
    }

    /// The culture used by the current thread.
    #[deprecated(
        since = "0.5.0",
        note = "use `ferroui_base::utilities::current_locale` for the language of the thread, or `CultureInfo::current_formatting` for the culture it formats with"
    )]
    pub fn current_culture() -> CultureInfo {
        Self::current_formatting()
    }

    /// Sets the culture used by the current thread.
    #[deprecated(
        since = "0.5.0",
        note = "use `ferroui_base::utilities::set_current_locale`, or `CultureInfo::set_current_formatting` for a culture with attached conventions"
    )]
    pub fn set_current_culture(culture: CultureInfo) {
        Self::set_current_formatting(culture);
    }

    /// The user interface culture used by the current thread: the culture of
    /// [`current_ui_locale`].
    #[deprecated(since = "0.5.0", note = "use `ferroui_base::utilities::current_ui_locale`")]
    pub fn current_ui_culture() -> CultureInfo {
        CultureInfo::from(&current_ui_locale())
    }

    /// Sets the user interface culture used by the current thread: its locale, the unknown
    /// locale `und` for a culture whose name is not a BCP-47 language tag.
    #[deprecated(since = "0.5.0", note = "use `ferroui_base::utilities::set_current_ui_locale`")]
    pub fn set_current_ui_culture(culture: CultureInfo) {
        set_current_ui_locale(culture.locale().clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::{locale, DateTimeFormatInfo};

    /// Restores the ambient values of the thread when a test ends.
    struct Restore(CultureInfo, Locale);

    impl Drop for Restore {
        fn drop(&mut self) {
            CultureInfo::set_current_formatting(self.0.clone());
            set_current_ui_locale(self.1.clone());
        }
    }

    fn restore() -> Restore {
        Restore(CultureInfo::current_formatting(), current_ui_locale())
    }

    #[test]
    fn the_current_locale_is_the_unknown_locale_until_it_is_set() {
        std::thread::spawn(|| {
            assert_eq!(current_locale(), Locale::UNKNOWN);
            assert_eq!(current_ui_locale(), Locale::UNKNOWN);
            assert!(CultureInfo::current_formatting().is_invariant());
        })
        .join()
        .unwrap();
    }

    #[test]
    fn current_locale_round_trip() {
        let _restore = restore();
        for locale in [locale!("ja-JP"), locale!("zh-Hant-TW"), locale!("en-US-u-hc-h12"), Locale::UNKNOWN] {
            set_current_locale(locale.clone());
            assert_eq!(current_locale(), locale);
            // The thread formats with the culture of the locale.
            assert_eq!(CultureInfo::current_formatting(), CultureInfo::from(&locale));
        }
        set_current_ui_locale(locale!("pl-PL"));
        assert_eq!(current_ui_locale(), locale!("pl-PL"));
        // The two ambient values are independent.
        assert_eq!(current_locale(), Locale::UNKNOWN);
    }

    #[test]
    fn a_formatting_culture_keeps_its_attached_conventions_and_sets_the_locale() {
        let _restore = restore();
        let mut conventions = (*DateTimeFormatInfo::invariant_info()).clone();
        conventions.set_short_date_pattern("M/d/yyyy");
        let culture = CultureInfo::get_culture_info("en-GB").with_date_time_format(conventions);
        CultureInfo::set_current_formatting(culture);
        assert_eq!(current_locale(), locale!("en-GB"));
        assert_eq!(CultureInfo::current_formatting().date_time_format().short_date_pattern(), "M/d/yyyy");

        // Setting the locale drops the attached conventions.
        set_current_locale(locale!("en-GB"));
        assert_eq!(CultureInfo::current_formatting(), CultureInfo::get_culture_info("en-GB"));
        assert_ne!(CultureInfo::current_formatting().date_time_format().short_date_pattern(), "M/d/yyyy");

        // A culture whose name is no language tag formats as it is, with the unknown locale.
        CultureInfo::set_current_formatting(CultureInfo::get_culture_info("es-ES_tradnl"));
        assert_eq!(current_locale(), Locale::UNKNOWN);
        assert_eq!(CultureInfo::current_formatting().name(), "es-ES_tradnl");
    }

    #[test]
    #[allow(deprecated)]
    fn the_deprecated_current_cultures_are_the_ambient_locales() {
        let _restore = restore();
        CultureInfo::set_current_culture(CultureInfo::get_culture_info("de-DE"));
        assert_eq!(current_locale(), locale!("de-DE"));
        assert_eq!(CultureInfo::current_culture(), CultureInfo::get_culture_info("de-DE"));
        set_current_locale(locale!("fr-FR"));
        assert_eq!(CultureInfo::current_culture().name(), "fr-FR");

        CultureInfo::set_current_ui_culture(CultureInfo::get_culture_info("ko-KR"));
        assert_eq!(current_ui_locale(), locale!("ko-KR"));
        assert_eq!(CultureInfo::current_ui_culture().name(), "ko-KR");
        CultureInfo::set_current_ui_culture(CultureInfo::invariant_culture());
        assert_eq!(current_ui_locale(), Locale::UNKNOWN);
        assert!(CultureInfo::current_ui_culture().is_invariant());
    }
}
