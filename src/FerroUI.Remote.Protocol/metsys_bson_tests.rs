//! Tests of the BSON serializer. The upstream project has none of its own
//! for the imported library (its protocol tests are in
//! `tests/remote_protocol_tests.rs`); these are tests of the port. The
//! expected bytes are written out from the BSON specification and from the
//! writer of the original: a document is its length (a little-endian `int32`
//! that counts itself), its elements and a zero byte; an element is its type
//! byte, its name with a terminating zero and its value.

use crate::error::Error;
use crate::guid::Guid;
use crate::metsys_bson::configuration::BsonConfiguration;
use crate::metsys_bson::{
    DateTime, Deserializer, Dictionary, Helper, ObjectId, ObjectIdGenerator, Options, Regex, RegexOptions, ScopedCode,
    Serializer, TypeHelper, Types, Value,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i32)]
enum Color {
    #[default]
    Red = 0,
    Green = 1,
    Blue = 2,
}

impl Color {
    fn from_value(value: i32) -> Option<Color> {
        [Color::Red, Color::Green, Color::Blue].into_iter().find(|c| *c as i32 == value)
    }
}

crate::bson_enum!(Color);

#[derive(Clone, Debug, Default, PartialEq)]
struct Child {
    foo: i32,
}

crate::bson_class!(Child as "Child" { "Foo" => foo: i32 });

#[derive(Clone, Debug, Default, PartialEq)]
struct Holder {
    text: Option<String>,
    line: Option<i32>,
    child: Option<Child>,
}

crate::bson_class!(Holder as "Holder" {
    "Text" => text: Option<String>,
    "Line" => line: Option<i32>,
    "Child" => child: Option<Child>,
});

#[derive(Clone, Debug, Default, PartialEq)]
struct Pair {
    width: f64,
    height: f64,
}

crate::bson_class!(Pair as "Pair" { "Width" => width: f64, "Height" => height: f64 });

#[derive(Clone, Debug, Default, PartialEq)]
struct Painted {
    color: Color,
    colors: Option<Vec<Color>>,
}

crate::bson_class!(Painted as "Painted" { "Color" => color: Color, "Colors" => colors: Option<Vec<Color>> });

#[derive(Clone, Debug, Default, PartialEq)]
struct WithCode {
    code: Option<ScopedCode>,
}

crate::bson_class!(WithCode as "WithCode" { "Code" => code: Option<ScopedCode> });

#[derive(Clone, Debug, Default, PartialEq)]
struct Configured {
    name: Option<String>,
    secret: Option<String>,
    note: Option<String>,
}

crate::bson_class!(Configured as "Configured" {
    "Name" => name: Option<String>,
    "Secret" => secret: Option<String>,
    "Note" => note: Option<String>,
});

#[derive(Clone, Debug, Default, PartialEq)]
struct Everything {
    int: i32,
    long: i64,
    flag: bool,
    text: String,
    number: f64,
    single: f32,
    id: Guid,
    time: DateTime,
    data: Option<Vec<u8>>,
    object_id: Option<ObjectId>,
    pattern: Option<Regex>,
    numbers: Option<Vec<i32>>,
    nullable: Option<i32>,
    child: Option<Child>,
    children: Option<Vec<Child>>,
    map: Option<Dictionary>,
    color: Color,
}

crate::bson_class!(Everything as "Everything" {
    "Int" => int: i32,
    "Long" => long: i64,
    "Flag" => flag: bool,
    "Text" => text: String,
    "Number" => number: f64,
    "Single" => single: f32,
    "Id" => id: Guid,
    "Time" => time: DateTime,
    "Data" => data: Option<Vec<u8>>,
    "ObjectId" => object_id: Option<ObjectId>,
    "Pattern" => pattern: Option<Regex>,
    "Numbers" => numbers: Option<Vec<i32>>,
    "Nullable" => nullable: Option<i32>,
    "Child" => child: Option<Child>,
    "Children" => children: Option<Vec<Child>>,
    "Map" => map: Option<Dictionary>,
    "Color" => color: Color,
});

const GUID: Guid = Guid::parse_const("6E3C5310-E2B1-4C3D-8688-01183AA48C5B");

/// The document `{ key: value }`.
fn document(key: &str, value: Value) -> Vec<u8> {
    let mut dictionary = Dictionary::new();
    dictionary.set(key, value);
    Serializer::serialize(&dictionary).unwrap()
}

/// The value of `a` in a document read as a dictionary.
fn read_a(bytes: &[u8]) -> Value {
    let dictionary = Deserializer::deserialize::<Dictionary>(bytes, None).unwrap();
    dictionary.get("a").unwrap().clone()
}

fn bson_message(error: Error) -> String {
    match error {
        Error::Bson(e) => e.message().to_string(),
        other => panic!("expected a BsonException, got {:?}", other),
    }
}

// ----------------------------------------------------------------- elements

#[test]
fn int32_is_type_16_and_four_little_endian_bytes() {
    let bytes = document("a", Value::Int32(1));
    assert_eq!(bytes, [0x0C, 0, 0, 0, 0x10, 0x61, 0, 1, 0, 0, 0, 0]);
    assert_eq!(read_a(&bytes), Value::Int32(1));
}

#[test]
fn int64_is_type_18_and_eight_little_endian_bytes() {
    let bytes = document("a", Value::Int64(0x0102_0304_0506_0708));
    assert_eq!(bytes, [0x10, 0, 0, 0, 0x12, 0x61, 0, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
    assert_eq!(read_a(&bytes), Value::Int64(0x0102_0304_0506_0708));
}

#[test]
fn double_is_type_1_and_the_ieee_bits() {
    let bytes = document("a", Value::Double(1.0));
    assert_eq!(bytes, [0x10, 0, 0, 0, 0x01, 0x61, 0, 0, 0, 0, 0, 0, 0, 0xF0, 0x3F, 0]);
    assert_eq!(read_a(&bytes), Value::Double(1.0));
}

#[test]
fn single_is_written_as_a_double() {
    let bytes = document("a", Value::Single(1.5));
    assert_eq!(bytes, [0x10, 0, 0, 0, 0x01, 0x61, 0, 0, 0, 0, 0, 0, 0, 0xF8, 0x3F, 0]);
    assert_eq!(read_a(&bytes), Value::Double(1.5));
}

#[test]
fn string_is_type_2_with_a_length_that_counts_the_terminator() {
    let bytes = document("a", Value::String("hi".to_string()));
    assert_eq!(bytes, [0x0F, 0, 0, 0, 0x02, 0x61, 0, 3, 0, 0, 0, 0x68, 0x69, 0, 0]);
    assert_eq!(read_a(&bytes), Value::String("hi".to_string()));
}

#[test]
fn string_and_name_are_utf8() {
    let bytes = document("é", Value::String("ż".to_string()));
    assert_eq!(bytes, [0x10, 0, 0, 0, 0x02, 0xC3, 0xA9, 0, 3, 0, 0, 0, 0xC5, 0xBC, 0, 0]);
    let dictionary = Deserializer::deserialize::<Dictionary>(&bytes, None).unwrap();
    assert_eq!(dictionary.get("é"), Some(&Value::String("ż".to_string())));
}

#[test]
fn boolean_is_type_8_and_one_byte() {
    let bytes = document("a", Value::Boolean(true));
    assert_eq!(bytes, [0x09, 0, 0, 0, 0x08, 0x61, 0, 1, 0]);
    assert_eq!(read_a(&bytes), Value::Boolean(true));
    assert_eq!(document("a", Value::Boolean(false)), [0x09, 0, 0, 0, 0x08, 0x61, 0, 0, 0]);
}

#[test]
fn null_is_type_10_without_a_value() {
    let bytes = document("a", Value::Null);
    assert_eq!(bytes, [0x08, 0, 0, 0, 0x0A, 0x61, 0, 0]);
    assert_eq!(read_a(&bytes), Value::Null);
}

#[test]
fn byte_array_is_binary_of_subtype_2_with_two_lengths() {
    let bytes = document("a", Value::ByteArray(vec![1, 2, 3]));
    assert_eq!(bytes, [0x14, 0, 0, 0, 0x05, 0x61, 0, 7, 0, 0, 0, 0x02, 3, 0, 0, 0, 1, 2, 3, 0]);
    assert_eq!(read_a(&bytes), Value::ByteArray(vec![1, 2, 3]));
}

#[test]
fn guid_is_binary_of_subtype_3() {
    let bytes = document("a", Value::Guid(GUID));
    assert_eq!(
        bytes,
        [
            0x1D, 0, 0, 0, 0x05, 0x61, 0, 16, 0, 0, 0, 0x03, 0x10, 0x53, 0x3C, 0x6E, 0xB1, 0xE2, 0x3D, 0x4C, 0x86,
            0x88, 0x01, 0x18, 0x3A, 0xA4, 0x8C, 0x5B, 0
        ]
    );
    assert_eq!(read_a(&bytes), Value::Guid(GUID));
}

#[test]
fn object_id_is_type_7_and_twelve_bytes() {
    let id = ObjectId::from_string("0102030405060708090a0b0c").unwrap();
    let bytes = document("a", Value::ObjectId(id.clone()));
    assert_eq!(bytes, [0x14, 0, 0, 0, 0x07, 0x61, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0]);
    assert_eq!(read_a(&bytes), Value::ObjectId(id));
}

#[test]
fn date_time_is_type_9_and_the_milliseconds_from_the_epoch() {
    let time = DateTime::from_unix_milliseconds(1000).unwrap();
    let bytes = document("a", Value::DateTime(time));
    assert_eq!(bytes, [0x10, 0, 0, 0, 0x09, 0x61, 0, 0xE8, 0x03, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(read_a(&bytes), Value::DateTime(time));
}

#[test]
fn date_time_is_truncated_to_whole_milliseconds() {
    assert_eq!(Helper::EPOCH.unix_milliseconds(), 0);
    assert_eq!(DateTime::from_unix_ticks(15_000).unix_milliseconds(), 1);
    assert_eq!(DateTime::from_unix_ticks(-15_000).unix_milliseconds(), -1);
    let bytes = document("a", Value::DateTime(DateTime::from_unix_ticks(15_000)));
    assert_eq!(read_a(&bytes), Value::DateTime(DateTime::from_unix_ticks(10_000)));
}

#[test]
fn regex_is_type_11_with_two_names_and_always_the_unicode_option() {
    let bytes = document("a", Value::Regex(Regex::new("ab", RegexOptions::IgnoreCase)));
    assert_eq!(bytes, [0x0E, 0, 0, 0, 0x0B, 0x61, 0, 0x61, 0x62, 0, 0x69, 0x75, 0, 0]);
    // The reader starts from `Compiled` and has no letter for the unicode option.
    assert_eq!(read_a(&bytes), Value::Regex(Regex::new("ab", RegexOptions::Compiled | RegexOptions::IgnoreCase)));
}

#[test]
fn regex_options_are_written_in_the_order_of_the_original() {
    let all = RegexOptions::ECMAScript
        | RegexOptions::IgnoreCase
        | RegexOptions::CultureInvariant
        | RegexOptions::Multiline
        | RegexOptions::Singleline
        | RegexOptions::IgnorePatternWhitespace
        | RegexOptions::ExplicitCapture;
    let bytes = document("a", Value::Regex(Regex::new("", all)));
    // The empty pattern, then "eilmsuwx".
    assert_eq!(bytes, [0x12, 0, 0, 0, 0x0B, 0x61, 0, 0, 0x65, 0x69, 0x6C, 0x6D, 0x73, 0x75, 0x77, 0x78, 0, 0]);
    assert_eq!(read_a(&bytes), Value::Regex(Regex::new("", all | RegexOptions::Compiled)));
}

#[test]
fn array_is_type_4_and_a_document_with_the_indices_as_names() {
    let bytes = document("a", Value::Enumerable(vec![Value::Int32(1), Value::Int32(2)]));
    assert_eq!(
        bytes,
        [0x1B, 0, 0, 0, 0x04, 0x61, 0, 0x13, 0, 0, 0, 0x10, 0x30, 0, 1, 0, 0, 0, 0x10, 0x31, 0, 2, 0, 0, 0, 0, 0]
    );
    assert_eq!(read_a(&bytes), Value::Enumerable(vec![Value::Int32(1), Value::Int32(2)]));
}

#[test]
fn empty_array_is_a_document_of_five_bytes() {
    let bytes = document("a", Value::Enumerable(Vec::new()));
    assert_eq!(bytes, [0x0D, 0, 0, 0, 0x04, 0x61, 0, 5, 0, 0, 0, 0, 0]);
    assert_eq!(read_a(&bytes), Value::Enumerable(Vec::new()));
}

#[test]
fn nested_dictionary_is_type_3_and_a_document() {
    let mut inner = Dictionary::new();
    inner.set("b", Value::Int32(1));
    let bytes = document("a", Value::Dictionary(inner.clone()));
    assert_eq!(bytes, [0x14, 0, 0, 0, 0x03, 0x61, 0, 0x0C, 0, 0, 0, 0x10, 0x62, 0, 1, 0, 0, 0, 0, 0]);
    assert_eq!(read_a(&bytes), Value::Dictionary(inner));
}

#[test]
fn empty_dictionary_is_a_document_of_five_bytes() {
    let bytes = Serializer::serialize(&Dictionary::new()).unwrap();
    assert_eq!(bytes, [5, 0, 0, 0, 0]);
    assert!(Deserializer::deserialize::<Dictionary>(&bytes, None).unwrap().is_empty());
}

#[test]
fn enum_is_written_as_the_int32_of_its_number() {
    let bytes = document("a", Value::Enum { value: 2, underlying: Types::Int32 });
    assert_eq!(bytes, [0x0C, 0, 0, 0, 0x10, 0x61, 0, 2, 0, 0, 0, 0]);
    let bytes = document("a", Value::Enum { value: 2, underlying: Types::Int64 });
    assert_eq!(bytes, [0x10, 0, 0, 0, 0x12, 0x61, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn scoped_code_is_type_15_with_a_length_a_string_and_a_document() {
    let mut scope = Dictionary::new();
    scope.set("b", Value::Int32(1));
    let value =
        WithCode { code: Some(ScopedCode { code_string: Some("x".to_string()), scope: Value::Dictionary(scope) }) };
    let bytes = Serializer::serialize(&value).unwrap();
    assert_eq!(
        bytes,
        [
            0x21, 0, 0, 0, 0x0F, 0x43, 0x6F, 0x64, 0x65, 0, 0x16, 0, 0, 0, 0x02, 0, 0, 0, 0x78, 0, 0x0C, 0, 0, 0, 0x10,
            0x62, 0, 1, 0, 0, 0, 0, 0
        ]
    );
    // The scope is read as `object`: an instance without members.
    let read = Deserializer::deserialize::<WithCode>(&bytes, None).unwrap();
    let code = read.code.unwrap();
    assert_eq!(code.code_string.as_deref(), Some("x"));
    match code.scope {
        Value::Object(object) => assert_eq!(object.get_type().name, "Object"),
        other => panic!("expected an object, got {:?}", other),
    }
}

// ------------------------------------------------------------------ classes

#[test]
fn object_is_written_property_by_property_under_the_declared_names() {
    let bytes = Serializer::serialize(&Child { foo: 543 }).unwrap();
    assert_eq!(bytes, [0x0E, 0, 0, 0, 0x10, 0x46, 0x6F, 0x6F, 0, 0x1F, 0x02, 0, 0, 0]);
    assert_eq!(Deserializer::deserialize::<Child>(&bytes, None).unwrap(), Child { foo: 543 });
}

#[test]
fn null_properties_are_written_as_null_elements() {
    let bytes = Serializer::serialize(&Holder::default()).unwrap();
    assert_eq!(
        bytes,
        [
            0x18, 0, 0, 0, 0x0A, 0x54, 0x65, 0x78, 0x74, 0, 0x0A, 0x4C, 0x69, 0x6E, 0x65, 0, 0x0A, 0x43, 0x68, 0x69,
            0x6C, 0x64, 0, 0
        ]
    );
    assert_eq!(Deserializer::deserialize::<Holder>(&bytes, None).unwrap(), Holder::default());
}

#[test]
fn nullable_and_nested_object_properties() {
    let value = Holder { text: Some("a".to_string()), line: Some(5), child: Some(Child { foo: 1 }) };
    let bytes = Serializer::serialize(&value).unwrap();
    assert_eq!(
        bytes,
        [
            0x30, 0, 0, 0, // the document
            0x02, 0x54, 0x65, 0x78, 0x74, 0, 2, 0, 0, 0, 0x61, 0, // Text
            0x10, 0x4C, 0x69, 0x6E, 0x65, 0, 5, 0, 0, 0, // Line
            0x03, 0x43, 0x68, 0x69, 0x6C, 0x64, 0, // Child
            0x0E, 0, 0, 0, 0x10, 0x46, 0x6F, 0x6F, 0, 1, 0, 0, 0, 0, // its document
            0
        ]
    );
    assert_eq!(Deserializer::deserialize::<Holder>(&bytes, None).unwrap(), value);
}

#[test]
fn empty_nested_document_is_read_as_null() {
    let bytes = [
        0x1D, 0, 0, 0, 0x0A, 0x54, 0x65, 0x78, 0x74, 0, 0x0A, 0x4C, 0x69, 0x6E, 0x65, 0, 0x03, 0x43, 0x68, 0x69, 0x6C,
        0x64, 0, 5, 0, 0, 0, 0, 0,
    ];
    assert_eq!(Deserializer::deserialize::<Holder>(&bytes, None).unwrap(), Holder::default());
}

#[test]
fn property_names_are_matched_without_regard_to_case() {
    let bytes = [0x0E, 0, 0, 0, 0x10, 0x66, 0x4F, 0x6F, 0, 7, 0, 0, 0, 0];
    assert_eq!(Deserializer::deserialize::<Child>(&bytes, None).unwrap(), Child { foo: 7 });
    let helper = TypeHelper::get_helper_for_type(<Child as crate::metsys_bson::BsonClass>::CLASS);
    assert!(helper.find_property("FOO").is_some());
    assert!(helper.find_property("Bar").is_none());
    assert_eq!(
        TypeHelper::find_property_info(<Child as crate::metsys_bson::BsonClass>::CLASS, "Foo").unwrap().name,
        "Foo"
    );
    assert!(TypeHelper::find_property_info(<Child as crate::metsys_bson::BsonClass>::CLASS, "foo").is_err());
}

#[test]
fn members_the_class_does_not_have_are_read_and_dropped() {
    let mut sub = Dictionary::new();
    sub.set("x", Value::Int32(1));
    let mut dictionary = Dictionary::new();
    dictionary.set("Extra", Value::Int32(1));
    dictionary.set("Arr", Value::Enumerable(vec![Value::Int32(1), Value::String("two".to_string())]));
    dictionary.set("Sub", Value::Dictionary(sub));
    dictionary.set("Empty", Value::Dictionary(Dictionary::new()));
    dictionary.set("Nothing", Value::Null);
    dictionary.set("Foo", Value::Int32(7));
    dictionary.set("Bytes", Value::ByteArray(vec![1, 2]));
    dictionary.set("Text", Value::String("s".to_string()));
    let bytes = Serializer::serialize(&dictionary).unwrap();
    assert_eq!(Deserializer::deserialize::<Child>(&bytes, None).unwrap(), Child { foo: 7 });
}

#[test]
fn enums_and_arrays_of_enums() {
    let value = Painted { color: Color::Blue, colors: Some(vec![Color::Red, Color::Blue]) };
    let bytes = Serializer::serialize(&value).unwrap();
    assert_eq!(
        bytes,
        [
            0x2B, 0, 0, 0, // the document
            0x10, 0x43, 0x6F, 0x6C, 0x6F, 0x72, 0, 2, 0, 0, 0, // Color
            0x04, 0x43, 0x6F, 0x6C, 0x6F, 0x72, 0x73, 0, // Colors
            0x13, 0, 0, 0, 0x10, 0x30, 0, 0, 0, 0, 0, 0x10, 0x31, 0, 2, 0, 0, 0, 0, // its document
            0
        ]
    );
    assert_eq!(Deserializer::deserialize::<Painted>(&bytes, None).unwrap(), value);
}

#[test]
fn enum_is_read_from_an_int64_and_a_number_without_a_member_is_rejected() {
    let mut dictionary = Dictionary::new();
    dictionary.set("Color", Value::Int64(1));
    let bytes = Serializer::serialize(&dictionary).unwrap();
    assert_eq!(Deserializer::deserialize::<Painted>(&bytes, None).unwrap().color, Color::Green);

    let mut dictionary = Dictionary::new();
    dictionary.set("Color", Value::Int32(9));
    let bytes = Serializer::serialize(&dictionary).unwrap();
    assert!(matches!(Deserializer::deserialize::<Painted>(&bytes, None), Err(Error::Argument(_))));

    let mut dictionary = Dictionary::new();
    dictionary.set("Color", Value::Int64(1 << 40));
    let bytes = Serializer::serialize(&dictionary).unwrap();
    assert!(matches!(Deserializer::deserialize::<Painted>(&bytes, None), Err(Error::Overflow(_))));
}

#[test]
fn every_kind_of_property_round_trips() {
    let mut map = Dictionary::new();
    map.set("one", Value::Int32(1));
    map.set("list", Value::Enumerable(vec![Value::Boolean(true), Value::Null]));
    let value = Everything {
        int: -5,
        long: 1 << 40,
        flag: true,
        text: "text".to_string(),
        number: 0.25,
        single: 1.5,
        id: GUID,
        time: DateTime::from_unix_milliseconds(1_700_000_000_123).unwrap(),
        data: Some(vec![0, 255, 7]),
        object_id: Some(ObjectId::from_string("0102030405060708090a0b0c").unwrap()),
        pattern: Some(Regex::new("a+", RegexOptions::Compiled | RegexOptions::Multiline)),
        numbers: Some(vec![3, 2, 1]),
        nullable: Some(9),
        child: Some(Child { foo: 2 }),
        children: Some(vec![Child { foo: 3 }, Child { foo: 4 }]),
        map: Some(map),
        color: Color::Green,
    };
    let bytes = Serializer::serialize(&value).unwrap();
    assert_eq!(Deserializer::deserialize::<Everything>(&bytes, None).unwrap(), value);

    // The defaults: the null properties are null elements and stay null.
    let bytes = Serializer::serialize(&Everything::default()).unwrap();
    assert_eq!(Deserializer::deserialize::<Everything>(&bytes, None).unwrap(), Everything::default());
}

#[test]
fn a_value_of_another_type_than_the_property_is_an_invalid_cast() {
    // A binary element of the subtype of a `Guid` for a `byte[]` property.
    let mut dictionary = Dictionary::new();
    dictionary.set("Data", Value::Guid(GUID));
    let bytes = Serializer::serialize(&dictionary).unwrap();
    assert!(matches!(Deserializer::deserialize::<Everything>(&bytes, None), Err(Error::InvalidCast(_))));
}

#[test]
fn configuration_gives_aliases_and_leaves_properties_out() {
    BsonConfiguration::for_type::<Configured>(|configuration| {
        configuration.use_alias("Name", "n").ignore("Secret").ignore_if_null("Note");
    });
    let value = Configured { name: Some("a".to_string()), secret: Some("s".to_string()), note: None };
    let bytes = Serializer::serialize(&value).unwrap();
    assert_eq!(bytes, [0x0E, 0, 0, 0, 0x02, 0x6E, 0, 2, 0, 0, 0, 0x61, 0, 0]);
    let read = Deserializer::deserialize::<Configured>(&bytes, None).unwrap();
    assert_eq!(read, Configured { name: Some("a".to_string()), secret: None, note: None });

    // An ignored property is not assigned when a document has it.
    let mut dictionary = Dictionary::new();
    dictionary.set("Secret", Value::String("s".to_string()));
    dictionary.set("Note", Value::String("note".to_string()));
    let bytes = Serializer::serialize(&dictionary).unwrap();
    let read = Deserializer::deserialize::<Configured>(&bytes, None).unwrap();
    assert_eq!(read, Configured { name: None, secret: None, note: Some("note".to_string()) });
}

// ------------------------------------------------------------------ options

#[test]
fn long_integers_reads_an_int_as_a_long() {
    let bytes = document("Foo", Value::Int32(1));
    let options = Options { long_integers: true, string_dates: false };
    // A property typed `int` cannot take the boxed long.
    assert!(matches!(Deserializer::deserialize::<Child>(&bytes, Some(options)), Err(Error::InvalidCast(_))));
    // An `int` element of a dictionary is read by its element type, also as a long.
    let dictionary = Deserializer::deserialize::<Dictionary>(&bytes, Some(options)).unwrap();
    assert_eq!(dictionary.get("Foo"), Some(&Value::Int64(1)));
}

#[test]
fn string_dates_reads_a_date_as_sortable_text() {
    let bytes = document("a", Value::DateTime(DateTime::from_unix_milliseconds(1000).unwrap()));
    let options = Options { long_integers: false, string_dates: true };
    let dictionary = Deserializer::deserialize::<Dictionary>(&bytes, Some(options)).unwrap();
    assert_eq!(dictionary.get("a"), Some(&Value::String("1970-01-01T00:00:01".to_string())));
    assert_eq!(
        DateTime::from_unix_milliseconds(1_000_000_000_000).unwrap().to_sortable_string(),
        "2001-09-09T01:46:40"
    );
    assert_eq!(DateTime::from_unix_milliseconds(951_782_400_000).unwrap().to_sortable_string(), "2000-02-29T00:00:00");
}

// ---------------------------------------------------------------- the roots

#[test]
fn root_must_be_an_object_or_a_dictionary() {
    assert_eq!(bson_message(Serializer::serialize(&5i32).unwrap_err()), "Root type must be an object");
    assert_eq!(bson_message(Serializer::serialize(&vec![1i32]).unwrap_err()), "Root type must be an object");
    assert_eq!(bson_message(Serializer::serialize(&"x".to_string()).unwrap_err()), "Root type must be an object");
    assert_eq!(bson_message(Serializer::serialize(&Color::Red).unwrap_err()), "Root type must be an object");
    assert!(matches!(Serializer::serialize(&Value::Null), Err(Error::NullReference)));
}

// ---------------------------------------------------------- malformed input

/// `Pair { width: 1.0, height: 2.0 }`.
const PAIR: [u8; 36] = [
    0x24, 0, 0, 0, // the document
    0x01, 0x57, 0x69, 0x64, 0x74, 0x68, 0, 0, 0, 0, 0, 0, 0, 0xF0, 0x3F, // Width
    0x01, 0x48, 0x65, 0x69, 0x67, 0x68, 0x74, 0, 0, 0, 0, 0, 0, 0, 0, 0x40, // Height
    0,
];

#[test]
fn the_document_of_two_doubles() {
    let value = Pair { width: 1.0, height: 2.0 };
    assert_eq!(Serializer::serialize(&value).unwrap(), PAIR);
    assert_eq!(Deserializer::deserialize::<Pair>(&PAIR, None).unwrap(), value);
}

#[test]
fn truncated_document_is_an_end_of_stream() {
    // Inside the length, inside a name, inside a value, without the terminator.
    for length in [0, 2, 4, 8, 15, 20, 30, 35] {
        let result = Deserializer::deserialize::<Pair>(&PAIR[..length], None);
        assert!(matches!(result, Err(Error::EndOfStream)), "length {}: {:?}", length, result);
    }
}

#[test]
fn a_length_that_is_too_large_or_too_small_reads_past_the_end() {
    let mut bytes = PAIR;
    bytes[0] = 0x25;
    assert!(matches!(Deserializer::deserialize::<Pair>(&bytes, None), Err(Error::EndOfStream)));
    bytes[0] = 0x23;
    assert!(matches!(Deserializer::deserialize::<Pair>(&bytes, None), Err(Error::EndOfStream)));
}

#[test]
fn a_length_that_ends_after_an_element_ends_the_object_there() {
    // The reader counts what it reads against the length and does not look
    // at the byte it takes for the terminator.
    let mut bytes = PAIR;
    bytes[0] = 0x14;
    assert_eq!(Deserializer::deserialize::<Pair>(&bytes, None).unwrap(), Pair { width: 1.0, height: 0.0 });
}

#[test]
fn an_empty_document_cannot_be_read_as_an_object() {
    // An object is read member by member before the end is tested.
    assert!(matches!(Deserializer::deserialize::<Child>(&[5, 0, 0, 0, 0], None), Err(Error::EndOfStream)));
}

#[test]
fn a_double_property_is_read_whatever_the_element_type_says() {
    let mut bytes = PAIR;
    bytes[4] = 0x7F;
    assert_eq!(Deserializer::deserialize::<Pair>(&bytes, None).unwrap(), Pair { width: 1.0, height: 2.0 });
}

#[test]
fn an_int_property_rejects_an_element_type_it_cannot_convert() {
    let unknown = [0x0E, 0, 0, 0, 0x7F, 0x46, 0x6F, 0x6F, 0, 1, 0, 0, 0, 0];
    assert_eq!(
        bson_message(Deserializer::deserialize::<Child>(&unknown, None).unwrap_err()),
        "Could not create an int from 127"
    );
    let string = [0x0E, 0, 0, 0, 0x02, 0x46, 0x6F, 0x6F, 0, 1, 0, 0, 0, 0];
    assert_eq!(
        bson_message(Deserializer::deserialize::<Child>(&string, None).unwrap_err()),
        "Could not create an int from String"
    );
}

#[test]
fn an_unknown_element_type_of_an_unknown_member_takes_the_rest_of_the_document() {
    // The member is read as `object`, which reads the members that follow as
    // its own and ends the document; the object that was being read then has
    // no document left.
    let bytes = [0x12, 0, 0, 0, 0x7F, 0x5A, 0x7A, 0, 0x10, 0x46, 0x6F, 0x6F, 0, 1, 0, 0, 0, 0];
    assert!(matches!(Deserializer::deserialize::<Child>(&bytes, None), Err(Error::NullReference)));
}

#[test]
fn an_unknown_element_type_in_a_dictionary_is_a_key_that_is_not_found() {
    let bytes = [0x08, 0, 0, 0, 0x7F, 0x61, 0, 0];
    assert!(matches!(Deserializer::deserialize::<Dictionary>(&bytes, None), Err(Error::KeyNotFound(_))));
}

#[test]
fn an_unknown_binary_subtype_is_rejected() {
    let bytes = [0x10, 0, 0, 0, 0x05, 0x61, 0, 3, 0, 0, 0, 0x00, 1, 2, 3, 0];
    assert_eq!(
        bson_message(Deserializer::deserialize::<Dictionary>(&bytes, None).unwrap_err()),
        "No support for binary type: 0"
    );
}

#[test]
fn a_negative_string_length_is_an_argument_error() {
    let bytes = [0x0C, 0, 0, 0, 0x02, 0x61, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0];
    assert!(matches!(Deserializer::deserialize::<Dictionary>(&bytes, None), Err(Error::Argument(_))));
}

#[test]
fn a_duplicate_key_of_a_dictionary_is_an_argument_error() {
    let bytes = [0x13, 0, 0, 0, 0x10, 0x61, 0, 1, 0, 0, 0, 0x10, 0x61, 0, 2, 0, 0, 0, 0];
    assert!(matches!(Deserializer::deserialize::<Dictionary>(&bytes, None), Err(Error::Argument(_))));
}

// ---------------------------------------------------------------- ObjectId

#[test]
fn object_id_text_form() {
    let id = ObjectId::from_string("0102030405060708090A0B0C").unwrap();
    assert_eq!(id.value(), Some(&[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12][..]));
    assert_eq!(id.to_string(), "0102030405060708090a0b0c");
    assert_eq!(Option::<String>::from(&id), Some("0102030405060708090a0b0c".to_string()));
    assert_eq!(ObjectId::try_from("0102030405060708090a0b0c").unwrap(), id);
    assert_eq!(ObjectId::empty().to_string(), "000000000000000000000000");
    assert_eq!(ObjectId::new().to_string(), "");
    assert_eq!(ObjectId::new(), ObjectId::new());
    assert_ne!(ObjectId::new(), ObjectId::empty());
}

#[test]
fn object_id_try_parse() {
    assert!(ObjectId::try_parse(None).is_none());
    assert!(ObjectId::try_parse(Some("0102")).is_none());
    assert!(ObjectId::try_parse(Some("zz02030405060708090a0b0c")).is_none());
    assert_eq!(
        ObjectId::try_parse(Some("0102030405060708090a0b0c")),
        Some(ObjectId::from_string("0102030405060708090a0b0c").unwrap())
    );
    assert!(matches!(ObjectId::from_string("zz"), Err(Error::Format(_))));
    assert!(matches!(ObjectId::from_string("abc"), Err(Error::Argument(_))));
}

#[test]
fn generated_object_ids_have_twelve_bytes_and_a_counter() {
    let first = ObjectIdGenerator::generate();
    let second = ObjectId::new_object_id();
    assert_eq!(first.len(), 12);
    assert_eq!(second.value().unwrap().len(), 12);
    // The machine and the process are the same; the ids differ (the counter).
    assert_eq!(first[4..9], second.value().unwrap()[4..9]);
    assert_ne!(first, second.value().unwrap());
}
