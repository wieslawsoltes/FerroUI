//! The `transform` attribute.

use crate::numbers::Scanner;
use ferroui_base::Matrix;

/// The matrix of a transform list; `None` when the list is malformed, which leaves the element
/// without a transform.
pub(crate) fn parse_transform(text: &str) -> Option<Matrix> {
    let mut scanner = Scanner::new(text);
    let mut total = Matrix::IDENTITY;
    loop {
        scanner.skip_separator();
        if scanner.peek().is_none() {
            return Some(total);
        }
        let name = function_name(&mut scanner)?;
        scanner.skip_whitespace();
        if scanner.peek() != Some(b'(') {
            return None;
        }
        scanner.advance();
        let mut arguments = [0.0; 6];
        let mut count = 0;
        loop {
            scanner.skip_separator();
            if scanner.peek() == Some(b')') {
                scanner.advance();
                break;
            }
            let value = scanner.number()?;
            *arguments.get_mut(count)? = value;
            count += 1;
        }
        // The list applies from its last item to its first, and a matrix here applies from the
        // left of a product to its right.
        total = function_matrix(name, &arguments[..count])? * total;
    }
}

fn function_name<'a>(scanner: &mut Scanner<'a>) -> Option<&'a [u8]> {
    let rest = scanner.rest();
    let length = rest.iter().take_while(|byte| byte.is_ascii_alphabetic()).count();
    if length == 0 {
        return None;
    }
    for _ in 0..length {
        scanner.advance();
    }
    Some(&rest[..length])
}

fn function_matrix(name: &[u8], arguments: &[f64]) -> Option<Matrix> {
    Some(match (name, arguments) {
        (b"matrix", &[a, b, c, d, e, f]) => Matrix::new(a, b, c, d, e, f),
        (b"translate", &[x]) => Matrix::new(1.0, 0.0, 0.0, 1.0, x, 0.0),
        (b"translate", &[x, y]) => Matrix::new(1.0, 0.0, 0.0, 1.0, x, y),
        (b"scale", &[s]) => Matrix::new(s, 0.0, 0.0, s, 0.0, 0.0),
        (b"scale", &[x, y]) => Matrix::new(x, 0.0, 0.0, y, 0.0, 0.0),
        (b"rotate", &[angle]) => rotation(angle),
        (b"rotate", &[angle, x, y]) => {
            Matrix::new(1.0, 0.0, 0.0, 1.0, -x, -y) * rotation(angle) * Matrix::new(1.0, 0.0, 0.0, 1.0, x, y)
        }
        (b"skewX", &[angle]) => Matrix::new(1.0, 0.0, angle.to_radians().tan(), 1.0, 0.0, 0.0),
        (b"skewY", &[angle]) => Matrix::new(1.0, angle.to_radians().tan(), 0.0, 1.0, 0.0, 0.0),
        _ => return None,
    })
}

fn rotation(degrees: f64) -> Matrix {
    let (sin, cos) = degrees.to_radians().sin_cos();
    Matrix::new(cos, sin, -sin, cos, 0.0, 0.0)
}
