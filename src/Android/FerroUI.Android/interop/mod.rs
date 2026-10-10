//! The boundary to the system: the Java Native Interface ([`java`]), the
//! functions of the NDK ([`ndk`]) and the native methods of the Java layer
//! ([`natives`]). Everything that is `unsafe` for Java or the NDK is in
//! these modules (`docs/porting/android-platform.md`, section 2).

pub(crate) mod signature;

#[cfg(target_os = "android")]
pub mod java;
#[cfg(target_os = "android")]
pub(crate) mod natives;
#[cfg(target_os = "android")]
pub(crate) mod ndk;

/// The text of a panic.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "a panic without a message".to_string()
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference.
    use super::*;

    #[test]
    fn the_message_of_a_panic_is_its_text() {
        let panic = std::panic::catch_unwind(|| panic!("number {}", 7)).unwrap_err();
        assert_eq!(panic_message(&*panic), "number 7");
        let panic = std::panic::catch_unwind(|| panic!("plain")).unwrap_err();
        assert_eq!(panic_message(&*panic), "plain");
        let panic = std::panic::catch_unwind(|| std::panic::panic_any(5)).unwrap_err();
        assert_eq!(panic_message(&*panic), "a panic without a message");
    }
}
