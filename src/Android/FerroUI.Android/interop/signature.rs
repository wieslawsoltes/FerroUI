//! Method signatures of the Java Native Interface, read so that the
//! arguments of a call can be checked before it is made: calling a method
//! with arguments of other types than its signature states is undefined
//! behaviour there.

/// A type of a signature, as far as a call distinguishes them: every class,
/// interface and array is a reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JavaType {
    Void,
    Boolean,
    Byte,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    Object,
}

/// The parameter types and the return type of a method signature such as
/// `(ILjava/lang/String;[F)V`; `None` when the text is not a signature.
pub(crate) fn parse_signature(signature: &str) -> Option<(Vec<JavaType>, JavaType)> {
    let rest = signature.strip_prefix('(')?;
    let (parameters, returns) = rest.split_once(')')?;

    let mut types = Vec::new();
    let mut chars = parameters.chars();
    while let Some(c) = chars.next() {
        let parsed = parse_type(c, &mut chars)?;
        if parsed == JavaType::Void {
            return None;
        }
        types.push(parsed);
    }

    let mut chars = returns.chars();
    let first = chars.next()?;
    let return_type = parse_type(first, &mut chars)?;
    if chars.next().is_some() {
        return None;
    }
    Some((types, return_type))
}

fn parse_type(first: char, rest: &mut std::str::Chars<'_>) -> Option<JavaType> {
    Some(match first {
        'V' => JavaType::Void,
        'Z' => JavaType::Boolean,
        'B' => JavaType::Byte,
        'C' => JavaType::Char,
        'S' => JavaType::Short,
        'I' => JavaType::Int,
        'J' => JavaType::Long,
        'F' => JavaType::Float,
        'D' => JavaType::Double,
        'L' => {
            // A class name ends with a semicolon and is not empty.
            let mut length = 0;
            loop {
                match rest.next()? {
                    ';' => break,
                    _ => length += 1,
                }
            }
            if length == 0 {
                return None;
            }
            JavaType::Object
        }
        '[' => {
            // An array of anything, arrays included, is a reference.
            let element = rest.next()?;
            let element = parse_type(element, rest)?;
            if element == JavaType::Void {
                return None;
            }
            JavaType::Object
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    // Not from the reference: its runtime checks a call when it binds the method.
    use super::*;

    #[test]
    fn primitive_parameters_and_the_return_type_are_read() {
        assert_eq!(
            parse_signature("(ZIJFD)V"),
            Some((
                vec![JavaType::Boolean, JavaType::Int, JavaType::Long, JavaType::Float, JavaType::Double],
                JavaType::Void
            ))
        );
        assert_eq!(parse_signature("()I"), Some((vec![], JavaType::Int)));
        assert_eq!(
            parse_signature("(BCS)J"),
            Some((vec![JavaType::Byte, JavaType::Char, JavaType::Short], JavaType::Long))
        );
    }

    #[test]
    fn classes_and_arrays_are_references() {
        assert_eq!(
            parse_signature("(Landroid/content/Context;J[I[[F[Ljava/lang/String;)Ljava/lang/Object;"),
            Some((
                vec![JavaType::Object, JavaType::Long, JavaType::Object, JavaType::Object, JavaType::Object],
                JavaType::Object
            ))
        );
        assert_eq!(parse_signature("()[I"), Some((vec![], JavaType::Object)));
    }

    #[test]
    fn what_is_not_a_signature_is_refused() {
        for text in
            ["", "I", "()", "(V)V", "(I", "(L;)V", "(Ljava/lang/String)V", "([)V", "(Q)V", "()VV", "()[V", "(I)"]
        {
            assert_eq!(parse_signature(text), None, "{text}");
        }
    }
}
