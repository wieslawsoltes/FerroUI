//! `Guid` keeps its text form, its order and its bits on top of `uuid::Uuid`.

use crate::Guid;
use uuid::{Uuid, Variant, Version};

#[test]
fn formats_the_hyphenated_form_in_lower_case() {
    let guid = Guid::from_u128(0x6E3C5310_E2B1_4C3D_8688_01183AA48C5B);
    assert_eq!(guid.to_string(), "6e3c5310-e2b1-4c3d-8688-01183aa48c5b");
    assert_eq!(Guid::EMPTY.to_string(), "00000000-0000-0000-0000-000000000000");
    assert_eq!(Guid::default(), Guid::EMPTY);
}

#[test]
fn new_identifiers_are_random_of_version_4() {
    let first = Guid::new_guid();
    let second = Guid::new_guid();
    assert_ne!(first, second);
    let uuid = Uuid::from(first);
    assert_eq!(uuid.get_version(), Some(Version::Random));
    assert_eq!(uuid.get_variant(), Variant::RFC4122);
}

#[test]
fn converts_to_and_from_uuid_and_keeps_the_order_of_its_bits() {
    let value = 0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF_u128;
    let guid = Guid::from_u128(value);
    assert_eq!(guid.as_u128(), value);
    assert_eq!(Guid::from(Uuid::from(guid)), guid);
    assert!(Guid::from_u128(1) < Guid::from_u128(1 << 64));
}
