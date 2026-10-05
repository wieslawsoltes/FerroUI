//! COM boolean conversions.

/// Converts a `bool` to the `int` boolean used by some native members.
#[inline]
pub(crate) fn as_com_bool(b: bool) -> i32 {
    if b {
        1
    } else {
        0
    }
}

/// Converts the `int` boolean used by some native members to a `bool`.
#[inline]
#[allow(dead_code)]
pub(crate) fn from_com_bool(b: i32) -> bool {
    b != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn com_bools_round_trip() {
        assert_eq!(as_com_bool(true), 1);
        assert_eq!(as_com_bool(false), 0);
        assert!(from_com_bool(1));
        assert!(from_com_bool(-7));
        assert!(!from_com_bool(0));
    }
}
