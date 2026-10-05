/// The kind of content a text input expects, which lets the platform show
/// an appropriate (on-screen) keyboard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextInputContentType {
    /// Default keyboard for the user's selected input method.
    #[default]
    Normal = 0,

    /// Display a keyboard that only has alphabetic characters.
    Alpha = 1,

    /// Display a numeric keypad only capable of numbers, i.e. phone number.
    Digits = 2,

    /// Display a numeric keypad for inputting a PIN.
    Pin = 3,

    /// Display a numeric keypad capable of inputting numbers including
    /// decimal separator and sign.
    Number = 4,

    /// Display a keyboard for entering an email address.
    Email = 5,

    /// Display a keyboard for entering a URL.
    Url = 6,

    /// Display a keyboard for entering a person's name.
    Name = 7,

    /// Display a keyboard for entering sensitive data.
    Password = 8,

    /// Display a keyboard suitable for #tag and @mentions. Not available on
    /// every platform, will fallback to a suitable keyboard when not
    /// available.
    Social = 9,

    /// Display a keyboard for entering a search keyword.
    Search = 10,
}
