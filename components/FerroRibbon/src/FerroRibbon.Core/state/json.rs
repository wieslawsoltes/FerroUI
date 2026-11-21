//! Schema codec syntax layer. String escape units stay undecoded until a typed
//! field reads them: System.Text.Json permits malformed escapes in skipped fields.
use crate::RibbonString;
#[derive(Clone)]
pub(super) enum Node {
    Null,
    Bool(bool),
    Number(String),
    String(RibbonString),
    Array(Vec<Node>),
    Object(Vec<(RibbonString, Node)>),
}
pub(super) fn parse(input: &RibbonString) -> Result<Node, ()> {
    let mut p = Parser {
        units: input.as_utf16(),
        index: 0,
    };
    let value = p.value(0)?;
    p.space();
    if p.index != p.units.len() {
        return Err(());
    }
    Ok(value)
}
struct Parser<'a> {
    units: &'a [u16],
    index: usize,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self
            .units
            .get(self.index)
            .is_some_and(|c| matches!(c, 9 | 10 | 13 | 32))
        {
            self.index += 1;
        }
    }
    fn take(&mut self, c: u16) -> bool {
        if self.units.get(self.index) == Some(&c) {
            self.index += 1;
            true
        } else {
            false
        }
    }
    fn value(&mut self, depth: usize) -> Result<Node, ()> {
        self.space();
        let c = *self.units.get(self.index).ok_or(())?;
        match c {
            34 => Ok(Node::String(self.string()?)),
            123 => {
                if depth >= 64 {
                    return Err(());
                }
                self.index += 1;
                self.space();
                let mut rows = vec![];
                if self.take(125) {
                    return Ok(Node::Object(rows));
                }
                loop {
                    self.space();
                    let key = self.string()?;
                    self.space();
                    if !self.take(58) {
                        return Err(());
                    }
                    let value = self.value(depth + 1)?;
                    rows.push((key, value));
                    self.space();
                    if self.take(125) {
                        break;
                    }
                    if !self.take(44) {
                        return Err(());
                    }
                }
                Ok(Node::Object(rows))
            }
            91 => {
                if depth >= 64 {
                    return Err(());
                }
                self.index += 1;
                self.space();
                let mut rows = vec![];
                if self.take(93) {
                    return Ok(Node::Array(rows));
                }
                loop {
                    rows.push(self.value(depth + 1)?);
                    self.space();
                    if self.take(93) {
                        break;
                    }
                    if !self.take(44) {
                        return Err(());
                    }
                }
                Ok(Node::Array(rows))
            }
            110 => {
                self.literal("null")?;
                Ok(Node::Null)
            }
            116 => {
                self.literal("true")?;
                Ok(Node::Bool(true))
            }
            102 => {
                self.literal("false")?;
                Ok(Node::Bool(false))
            }
            45 | 48..=57 => self.number(),
            _ => Err(()),
        }
    }
    fn literal(&mut self, text: &str) -> Result<(), ()> {
        for c in text.bytes() {
            if !self.take(c as u16) {
                return Err(());
            }
        }
        Ok(())
    }
    fn digits(&mut self) -> usize {
        let start = self.index;
        while self
            .units
            .get(self.index)
            .is_some_and(|c| (48..=57).contains(c))
        {
            self.index += 1;
        }
        self.index - start
    }
    fn number(&mut self) -> Result<Node, ()> {
        let start = self.index;
        self.take(45);
        if !self.take(48) && self.digits() == 0 {
            return Err(());
        }
        if self.take(46) && self.digits() == 0 {
            return Err(());
        }
        if self.take(101) || self.take(69) {
            if !self.take(43) {
                self.take(45);
            }
            if self.digits() == 0 {
                return Err(());
            }
        }
        Ok(Node::Number(
            String::from_utf16(&self.units[start..self.index]).map_err(|_| ())?,
        ))
    }
    fn string(&mut self) -> Result<RibbonString, ()> {
        if !self.take(34) {
            return Err(());
        }
        let mut units = vec![];
        loop {
            let c = *self.units.get(self.index).ok_or(())?;
            self.index += 1;
            match c {
                34 => return Ok(RibbonString::from_utf16(units)),
                0..=31 => return Err(()),
                92 => {
                    let c = *self.units.get(self.index).ok_or(())?;
                    self.index += 1;
                    units.push(match c {
                        34 | 47 | 92 => c,
                        98 => 8,
                        102 => 12,
                        110 => 10,
                        114 => 13,
                        116 => 9,
                        117 => {
                            let mut unit = 0;
                            for _ in 0..4 {
                                let c = *self.units.get(self.index).ok_or(())?;
                                self.index += 1;
                                unit = (unit << 4)
                                    | match c {
                                        48..=57 => c - 48,
                                        65..=70 => c - 55,
                                        97..=102 => c - 87,
                                        _ => return Err(()),
                                    };
                            }
                            unit
                        }
                        _ => return Err(()),
                    });
                }
                _ => units.push(c),
            }
        }
    }
}
pub(super) fn object(value: Node) -> Result<Vec<(RibbonString, Node)>, ()> {
    if let Node::Object(v) = value {
        Ok(v)
    } else {
        Err(())
    }
}
pub(super) fn array(value: Node) -> Result<Vec<Node>, ()> {
    if let Node::Array(v) = value {
        Ok(v)
    } else {
        Err(())
    }
}
pub(super) fn string(value: Node) -> Result<Option<RibbonString>, ()> {
    match value {
        Node::Null => Ok(None),
        Node::String(s) => {
            s.to_utf8().map_err(|_| ())?;
            Ok(Some(s))
        }
        _ => Err(()),
    }
}
pub(super) fn boolean(value: Node) -> Result<bool, ()> {
    if let Node::Bool(v) = value {
        Ok(v)
    } else {
        Err(())
    }
}
pub(super) fn integer(value: Node) -> Result<i32, ()> {
    if let Node::Number(s) = value {
        s.parse().map_err(|_| ())
    } else {
        Err(())
    }
}
pub(super) fn double(value: Node) -> Result<f64, ()> {
    if let Node::Number(s) = value {
        s.parse().map_err(|_| ())
    } else {
        Err(())
    }
}
pub(super) fn render(value: &Node, indented: bool) -> String {
    let mut out = String::new();
    write(value, indented, 0, &mut out);
    out
}
fn line(out: &mut String, depth: usize) {
    out.push('\n');
    out.extend(std::iter::repeat_n(' ', depth * 2));
}
fn quoted(value: &RibbonString, out: &mut String) {
    use std::fmt::Write;
    out.push('"');
    for scalar in char::decode_utf16(value.as_utf16().iter().copied()) {
        let scalar = scalar.unwrap_or(char::REPLACEMENT_CHARACTER);
        let mut buffer = [0; 2];
        for &c in scalar.encode_utf16(&mut buffer).iter() {
            match c {
                8 => out.push_str("\\b"),
                9 => out.push_str("\\t"),
                10 => out.push_str("\\n"),
                12 => out.push_str("\\f"),
                13 => out.push_str("\\r"),
                92 => out.push_str("\\\\"),
                32..=126 if !matches!(c, 34 | 38 | 39 | 43 | 60 | 62 | 96) => {
                    out.push(char::from_u32(c as u32).unwrap())
                }
                _ => write!(out, "\\u{c:04X}").unwrap(),
            }
        }
    }
    out.push('"');
}
fn write(value: &Node, pretty: bool, depth: usize, out: &mut String) {
    match value {
        Node::Null => out.push_str("null"),
        Node::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Node::Number(s) => out.push_str(s),
        Node::String(s) => quoted(s, out),
        Node::Array(values) => {
            out.push('[');
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if pretty {
                    line(out, depth + 1);
                }
                write(v, pretty, depth + 1, out);
            }
            if pretty && !values.is_empty() {
                line(out, depth);
            }
            out.push(']');
        }
        Node::Object(values) => {
            out.push('{');
            for (i, (k, v)) in values.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if pretty {
                    line(out, depth + 1);
                }
                quoted(k, out);
                out.push(':');
                if pretty {
                    out.push(' ');
                }
                write(v, pretty, depth + 1, out);
            }
            if pretty && !values.is_empty() {
                line(out, depth);
            }
            out.push('}');
        }
    }
}
