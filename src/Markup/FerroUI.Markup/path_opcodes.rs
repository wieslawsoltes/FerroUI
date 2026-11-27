//! Binary operands for a compiled geometry literal. Each command has a three-bit
//! operation and inline flags. Coordinate operands preserve the original f64 bits.
use crate::compiled_operands::{number, read_number, take, ReadResult};
use ferroui_base::media::{FillRule, PathCommand, SweepDirection};
use ferroui_base::{Point, Size};

const FLAG: u8 = 8;
const LARGE_ARC: u8 = 16;
const CLOCKWISE: u8 = 32;

pub fn encode(commands: &[PathCommand]) -> Vec<u8> {
    fn point(out: &mut Vec<u8>, p: Point) {
        number(out, p.x);
        number(out, p.y);
    }
    let mut out = Vec::new();
    for &command in commands {
        let (op, flag) = match command {
            PathCommand::SetFillRule(rule) => (0, rule == FillRule::NonZero),
            PathCommand::BeginFigure(_, filled) => (1, filled),
            PathCommand::LineTo(_, stroked) => (2, stroked),
            PathCommand::QuadraticBezierTo(_, _, stroked) => (3, stroked),
            PathCommand::CubicBezierTo(_, _, _, stroked) => (4, stroked),
            PathCommand::ArcTo(_, _, _, large, sweep, stroked) => (
                5 | u8::from(large) * LARGE_ARC
                    | u8::from(sweep == SweepDirection::Clockwise) * CLOCKWISE,
                stroked,
            ),
            PathCommand::EndFigure(closed) => (6, closed),
        };
        out.push(op | u8::from(flag) * FLAG);
        match command {
            PathCommand::BeginFigure(p, _) | PathCommand::LineTo(p, _) => point(&mut out, p),
            PathCommand::QuadraticBezierTo(a, b, _) => {
                point(&mut out, a);
                point(&mut out, b);
            }
            PathCommand::CubicBezierTo(a, b, c, _) => {
                point(&mut out, a);
                point(&mut out, b);
                point(&mut out, c);
            }
            PathCommand::ArcTo(p, size, rotation, ..) => {
                point(&mut out, p);
                number(&mut out, size.width);
                number(&mut out, size.height);
                number(&mut out, rotation);
            }
            PathCommand::EndFigure(_) | PathCommand::SetFillRule(_) => {}
        }
    }
    out
}

pub fn read(input: &mut &[u8]) -> ReadResult<PathCommand> {
    fn point(input: &mut &[u8]) -> ReadResult<Point> {
        Ok(Point::new(read_number(input)?, read_number(input)?))
    }
    let [tag] = take(input)?;
    let flag = tag & FLAG != 0;
    Ok(match tag & !FLAG {
        0 => PathCommand::SetFillRule(if flag {
            FillRule::NonZero
        } else {
            FillRule::EvenOdd
        }),
        1 => PathCommand::BeginFigure(point(input)?, flag),
        2 => PathCommand::LineTo(point(input)?, flag),
        3 => PathCommand::QuadraticBezierTo(point(input)?, point(input)?, flag),
        4 => PathCommand::CubicBezierTo(point(input)?, point(input)?, point(input)?, flag),
        bits if bits & !(LARGE_ARC | CLOCKWISE) == 5 => {
            let end = point(input)?;
            let size = Size::new(read_number(input)?, read_number(input)?);
            let rotation = read_number(input)?;
            let sweep = if bits & CLOCKWISE != 0 {
                SweepDirection::Clockwise
            } else {
                SweepDirection::CounterClockwise
            };
            PathCommand::ArcTo(end, size, rotation, bits & LARGE_ARC != 0, sweep, flag)
        }
        6 => PathCommand::EndFigure(flag),
        _ => return Err("unknown path command"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_commands_preserve_the_parsers_results_and_truncation_fails() {
        let commands =
            PathCommand::record("F1 M1 2 L3.25 4 Q1 2 3 4 C5 6 7 8 9 10 A4 5 30 1 0 8 9 Z")
                .unwrap();
        for expected in commands {
            let bytes = encode(&[expected]);
            for length in 0..bytes.len() {
                assert!(read(&mut &bytes[..length]).is_err());
            }
            let mut input = bytes.as_slice();
            assert_eq!(read(&mut input).unwrap(), expected);
            assert!(input.is_empty());
        }
        assert!(read(&mut &[0x80][..]).is_err());
        let negative_zero = PathCommand::LineTo(Point::new(-0.0, 0.1), false);
        let bytes = encode(&[negative_zero]);
        let PathCommand::LineTo(p, false) = read(&mut bytes.as_slice()).unwrap() else {
            panic!()
        };
        assert_eq!(p.x.to_bits(), (-0.0f64).to_bits());
        assert_eq!(p.y.to_bits(), 0.1f64.to_bits());
    }
}
