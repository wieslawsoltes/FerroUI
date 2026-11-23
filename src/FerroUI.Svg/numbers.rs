//! The numbers and lengths of SVG attributes.

/// Reads numbers and separators from the text of an attribute.
pub(crate) struct Scanner<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Scanner<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        Self { bytes: text.as_bytes(), at: 0 }
    }

    pub(crate) fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    pub(crate) fn advance(&mut self) {
        self.at += 1;
    }

    pub(crate) fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r' | 0x0c)) {
            self.at += 1;
        }
    }

    /// Skips whitespace with at most one comma in it.
    pub(crate) fn skip_separator(&mut self) {
        self.skip_whitespace();
        if self.peek() == Some(b',') {
            self.at += 1;
            self.skip_whitespace();
        }
    }

    fn skip_digits(&mut self) -> usize {
        let start = self.at;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.at += 1;
        }
        self.at - start
    }

    /// A number, without what surrounds it. `None`, with nothing read, when there is none or it
    /// is not finite.
    pub(crate) fn number(&mut self) -> Option<f64> {
        let start = self.at;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.at += 1;
        }
        let mut digits = self.skip_digits();
        if self.peek() == Some(b'.') {
            self.at += 1;
            digits += self.skip_digits();
        }
        if digits == 0 {
            self.at = start;
            return None;
        }
        // An exponent only when digits follow: "1em" is the number 1 and a unit.
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let before = self.at;
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.skip_digits() == 0 {
                self.at = before;
            }
        }
        let value = std::str::from_utf8(&self.bytes[start..self.at]).ok().and_then(|text| text.parse::<f64>().ok());
        match value {
            Some(value) if value.is_finite() => Some(value),
            _ => {
                self.at = start;
                None
            }
        }
    }

    /// A flag of an arc: one character, which may touch what follows it.
    pub(crate) fn flag(&mut self) -> Option<bool> {
        let flag = match self.peek()? {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        self.at += 1;
        Some(flag)
    }

    /// What is left of the text.
    pub(crate) fn rest(&self) -> &'a [u8] {
        &self.bytes[self.at..]
    }
}

/// The numbers of a list separated by whitespace or commas, up to the first that is not one.
pub(crate) fn number_list(text: &str) -> Vec<f64> {
    let mut scanner = Scanner::new(text);
    let mut numbers = Vec::new();
    scanner.skip_whitespace();
    while let Some(number) = scanner.number() {
        numbers.push(number);
        scanner.skip_separator();
    }
    numbers
}

/// A length in user units; a percentage is of `reference`.
pub(crate) fn length(text: &str, reference: f64) -> Option<f64> {
    let text = text.trim();
    let mut scanner = Scanner::new(text);
    let value = scanner.number()?;
    let factor = match scanner.rest() {
        b"" | b"px" => 1.0,
        b"%" => reference / 100.0,
        b"pt" => 4.0 / 3.0,
        b"pc" => 16.0,
        b"in" => 96.0,
        b"cm" => 96.0 / 2.54,
        b"mm" => 96.0 / 25.4,
        b"Q" => 96.0 / 101.6,
        // The default font size: no text is drawn, so none other is known.
        b"em" => 16.0,
        b"ex" => 8.0,
        _ => return None,
    };
    let result = value * factor;
    result.is_finite().then_some(result)
}

/// An opacity: a number or a percentage, clamped to 0..1.
pub(crate) fn opacity(text: &str) -> Option<f64> {
    length(text, 1.0).map(|value| value.clamp(0.0, 1.0))
}
