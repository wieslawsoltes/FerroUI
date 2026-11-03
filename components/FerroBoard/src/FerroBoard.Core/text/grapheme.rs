// Adapted from .NET TextSegmentationUtility.cs at 60629d14374c56f1cb51819049ad1fa529307f8d.
// Copyright (c) .NET Foundation and Contributors. MIT license:
// reference/dotnet-runtime/LICENSE.TXT. See docs/DEPENDENCIES.md for runtime provenance.
use super::text_properties::{GraphemeClass, properties};
use GraphemeClass::*;
struct Processor<'a> {
    input: &'a [u16],
    offset: usize,
    length: usize,
    class: GraphemeClass,
}
impl Processor<'_> {
    fn advance(&mut self) {
        self.offset += self.length;
        let (scalar, length) = match self.input.get(self.offset).copied() {
            None => (0xfffd, 0),
            Some(high) if (0xd800..=0xdbff).contains(&high) => {
                match self.input.get(self.offset + 1).copied() {
                    Some(low) if (0xdc00..=0xdfff).contains(&low) => (
                        0x10000 + ((high as u32 - 0xd800) << 10) + (low as u32 - 0xdc00),
                        2,
                    ),
                    _ => (0xfffd, 1),
                }
            }
            Some(low) if (0xdc00..=0xdfff).contains(&low) => (0xfffd, 1),
            Some(c) => (c as u32, 1),
        };
        self.length = length;
        self.class = properties(scalar).1;
    }
}
pub(crate) fn first_length(input: &[u16]) -> usize {
    let mut p = Processor {
        input,
        offset: 0,
        length: 0,
        class: Other,
    };
    p.advance();
    while p.class == Prepend {
        p.advance();
    }
    if p.offset > 0 && matches!(p.class, Control | Cr | Lf) {
        return p.offset;
    }
    let mut previous = p.class;
    p.advance();
    loop {
        match previous {
            Cr => {
                if p.class != Lf {
                    return p.offset;
                }
                p.advance();
                return p.offset;
            }
            Control | Lf => return p.offset,
            L => {
                if matches!(p.class, L | V | Lv | Lvt) {
                    previous = p.class;
                    p.advance();
                    continue;
                }
                break;
            }
            Lv | V => {
                if matches!(p.class, V | T) {
                    previous = p.class;
                    p.advance();
                    continue;
                }
                break;
            }
            Lvt | T => {
                if p.class == T {
                    previous = T;
                    p.advance();
                    continue;
                }
                break;
            }
            ExtendedPictograph => {
                while p.class == Extend {
                    p.advance();
                }
                if p.class != Zwj {
                    break;
                }
                p.advance();
                if p.class != ExtendedPictograph {
                    break;
                }
                p.advance();
                continue;
            }
            RegionalIndicator => {
                if p.class == RegionalIndicator {
                    p.advance();
                }
                break;
            }
            _ => break,
        }
    }
    while matches!(p.class, Extend | Zwj | SpacingMark) {
        p.advance();
    }
    p.offset
}
