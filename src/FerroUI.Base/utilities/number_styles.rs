//! A counterpart of .NET's `System.Globalization.NumberStyles`.

use bitflags::bitflags;

bitflags! {
    /// The styles permitted in the text of a number that is parsed.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct NumberStyles: i32 {
        /// No style elements: decimal digits only.
        const NONE = 0x0000;

        /// White space may come first.
        const ALLOW_LEADING_WHITE = 0x0001;

        /// White space may come last.
        const ALLOW_TRAILING_WHITE = 0x0002;

        /// A sign may come first.
        const ALLOW_LEADING_SIGN = 0x0004;

        /// A sign may come last.
        const ALLOW_TRAILING_SIGN = 0x0008;

        /// One pair of parentheses may enclose the number, which is then
        /// negative.
        const ALLOW_PARENTHESES = 0x0010;

        /// A decimal point is permitted.
        const ALLOW_DECIMAL_POINT = 0x0020;

        /// Group separators are permitted.
        const ALLOW_THOUSANDS = 0x0040;

        /// An exponent (`E` or `e`, an optional sign, digits) is permitted.
        const ALLOW_EXPONENT = 0x0080;

        /// A currency symbol is permitted.
        const ALLOW_CURRENCY_SYMBOL = 0x0100;

        /// The text is a hexadecimal number.
        const ALLOW_HEX_SPECIFIER = 0x0200;

        /// The text is a binary number.
        const ALLOW_BINARY_SPECIFIER = 0x0400;

        /// Leading and trailing white space and a leading sign.
        const INTEGER = Self::ALLOW_LEADING_WHITE.bits() | Self::ALLOW_TRAILING_WHITE.bits()
            | Self::ALLOW_LEADING_SIGN.bits();

        /// Leading and trailing white space and hexadecimal digits.
        const HEX_NUMBER = Self::ALLOW_LEADING_WHITE.bits() | Self::ALLOW_TRAILING_WHITE.bits()
            | Self::ALLOW_HEX_SPECIFIER.bits();

        /// Leading and trailing white space and binary digits.
        const BINARY_NUMBER = Self::ALLOW_LEADING_WHITE.bits() | Self::ALLOW_TRAILING_WHITE.bits()
            | Self::ALLOW_BINARY_SPECIFIER.bits();

        /// White space, a leading or trailing sign, a decimal point and
        /// group separators.
        const NUMBER = Self::ALLOW_LEADING_WHITE.bits() | Self::ALLOW_TRAILING_WHITE.bits()
            | Self::ALLOW_LEADING_SIGN.bits() | Self::ALLOW_TRAILING_SIGN.bits()
            | Self::ALLOW_DECIMAL_POINT.bits() | Self::ALLOW_THOUSANDS.bits();

        /// White space, a leading sign, a decimal point and an exponent.
        const FLOAT = Self::ALLOW_LEADING_WHITE.bits() | Self::ALLOW_TRAILING_WHITE.bits()
            | Self::ALLOW_LEADING_SIGN.bits() | Self::ALLOW_DECIMAL_POINT.bits()
            | Self::ALLOW_EXPONENT.bits();

        /// Every style except the exponent and the hexadecimal and binary
        /// specifiers.
        const CURRENCY = Self::ALLOW_LEADING_WHITE.bits() | Self::ALLOW_TRAILING_WHITE.bits()
            | Self::ALLOW_LEADING_SIGN.bits() | Self::ALLOW_TRAILING_SIGN.bits()
            | Self::ALLOW_PARENTHESES.bits() | Self::ALLOW_DECIMAL_POINT.bits()
            | Self::ALLOW_THOUSANDS.bits() | Self::ALLOW_CURRENCY_SYMBOL.bits();

        /// Every style except the hexadecimal and binary specifiers.
        const ANY = Self::ALLOW_LEADING_WHITE.bits() | Self::ALLOW_TRAILING_WHITE.bits()
            | Self::ALLOW_LEADING_SIGN.bits() | Self::ALLOW_TRAILING_SIGN.bits()
            | Self::ALLOW_PARENTHESES.bits() | Self::ALLOW_DECIMAL_POINT.bits()
            | Self::ALLOW_THOUSANDS.bits() | Self::ALLOW_CURRENCY_SYMBOL.bits()
            | Self::ALLOW_EXPONENT.bits();
    }
}

crate::ferro_markup_enum!(flags NumberStyles {
    None = NumberStyles::NONE,
    AllowLeadingWhite = NumberStyles::ALLOW_LEADING_WHITE,
    AllowTrailingWhite = NumberStyles::ALLOW_TRAILING_WHITE,
    AllowLeadingSign = NumberStyles::ALLOW_LEADING_SIGN,
    AllowTrailingSign = NumberStyles::ALLOW_TRAILING_SIGN,
    AllowParentheses = NumberStyles::ALLOW_PARENTHESES,
    AllowDecimalPoint = NumberStyles::ALLOW_DECIMAL_POINT,
    AllowThousands = NumberStyles::ALLOW_THOUSANDS,
    AllowExponent = NumberStyles::ALLOW_EXPONENT,
    AllowCurrencySymbol = NumberStyles::ALLOW_CURRENCY_SYMBOL,
    AllowHexSpecifier = NumberStyles::ALLOW_HEX_SPECIFIER,
    AllowBinarySpecifier = NumberStyles::ALLOW_BINARY_SPECIFIER,
    Integer = NumberStyles::INTEGER,
    HexNumber = NumberStyles::HEX_NUMBER,
    BinaryNumber = NumberStyles::BINARY_NUMBER,
    Number = NumberStyles::NUMBER,
    Float = NumberStyles::FLOAT,
    Currency = NumberStyles::CURRENCY,
    Any = NumberStyles::ANY,
}, { namespace: "System.Globalization" });

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_the_managed_enumeration() {
        assert_eq!(NumberStyles::INTEGER.bits(), 0x0007);
        assert_eq!(NumberStyles::HEX_NUMBER.bits(), 0x0203);
        assert_eq!(NumberStyles::BINARY_NUMBER.bits(), 0x0403);
        assert_eq!(NumberStyles::NUMBER.bits(), 0x006F);
        assert_eq!(NumberStyles::FLOAT.bits(), 0x00A7);
        assert_eq!(NumberStyles::CURRENCY.bits(), 0x017F);
        assert_eq!(NumberStyles::ANY.bits(), 0x01FF);
        assert_eq!(NumberStyles::default(), NumberStyles::NONE);
    }
}
