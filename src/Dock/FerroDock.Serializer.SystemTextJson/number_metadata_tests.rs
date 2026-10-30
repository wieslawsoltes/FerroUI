// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::core::IDockSerializer;

const SOURCE: &str = include_str!("../../../tests/Dock/data/serializer-json-audit.numbers.tsv");
fn serializer() -> std::rc::Rc<DockSerializer> {
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(Vec::new()))).unwrap()
}

#[test]
fn original_float_wire_and_read_bits_match_for_boundaries_and_seeded_values() {
    let serializer = serializer();
    let mut seen = 0;
    for line in SOURCE.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        let kind = fields[0];
        macro_rules! floating {
            ($type:ty, $bits:ty) => {
                if kind.starts_with("write-") {
                    let bits = <$bits>::from_str_radix(fields[1], 16).unwrap();
                    let value = <$type>::from_bits(bits);
                    let text = serializer.serialize(value).unwrap();
                    assert_eq!(text, fields[2], "{kind} {}", fields[1]);
                    let read = serializer
                        .deserialize::<$type>(Some(&text))
                        .unwrap()
                        .unwrap();
                    if value.is_nan() {
                        assert!(read.is_nan());
                    } else {
                        assert_eq!(read.to_bits(), bits, "{kind} {}", fields[1]);
                    }
                } else {
                    let result = serializer.deserialize::<$type>(Some(fields[1]));
                    if fields[2] == "error" {
                        assert!(result.is_err(), "{kind} {}", fields[1]);
                    } else {
                        let expected = <$bits>::from_str_radix(fields[2], 16).unwrap();
                        assert_eq!(
                            result.unwrap().unwrap().to_bits(),
                            expected,
                            "{kind} {}",
                            fields[1]
                        );
                    }
                }
                seen += 1;
            };
        }
        match kind {
            "write-f64" | "read-f64" => {
                floating!(f64, u64);
            }
            "write-f32" | "read-f32" => {
                floating!(f32, u32);
            }
            _ => continue,
        }
    }
    assert_eq!(seen, 1118);
}

#[test]
fn original_integer_ranges_tokens_and_wire_match_all_eight_declared_types() {
    let serializer = serializer();
    let mut seen = 0;
    for line in SOURCE.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        let Some((operation, kind)) = fields[0].split_once('-') else {
            panic!("source row");
        };
        macro_rules! integer {
            ($type:ty) => {
                if operation == "write" {
                    let value = fields[1].parse::<$type>().unwrap();
                    assert_eq!(
                        serializer.serialize(value).unwrap(),
                        fields[2],
                        "{}",
                        fields[0]
                    );
                } else {
                    let result = serializer.deserialize::<$type>(Some(fields[1]));
                    if fields[2] == "error" {
                        assert!(result.is_err(), "{} {}", fields[0], fields[1]);
                    } else {
                        assert_eq!(
                            result.unwrap().unwrap().to_string(),
                            fields[2],
                            "{} {}",
                            fields[0],
                            fields[1]
                        );
                    }
                }
                seen += 1;
            };
        }
        match kind {
            "i8" => {
                integer!(i8);
            }
            "u8" => {
                integer!(u8);
            }
            "i16" => {
                integer!(i16);
            }
            "u16" => {
                integer!(u16);
            }
            "i32" => {
                integer!(i32);
            }
            "u32" => {
                integer!(u32);
            }
            "i64" => {
                integer!(i64);
            }
            "u64" => {
                integer!(u64);
            }
            _ => continue,
        }
    }
    assert_eq!(seen, 244);
}
