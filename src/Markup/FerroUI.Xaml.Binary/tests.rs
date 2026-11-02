use super::*;

fn row<'a>(key: &'a str, value: Value<'a>) -> Resource<'a> {
    Resource {
        key,
        value,
        line: 200,
        column: 0,
        value_line: 4,
        value_column: 30,
    }
}

#[test]
fn lexical_pool_preserves_unicode_insertion_order_and_old_generation_handles() {
    let rows = [
        row("Ångström", Value::Text("É")),
        row("Ångström2", Value::Text("Ê")),
        row("Å", Value::Text("É")),
    ];
    let image = encode(Kind::Alias, &rows).unwrap();
    assert_eq!(image, encode(Kind::Alias, &rows).unwrap());
    let batch =
        std::thread::scope(|scope| scope.spawn(|| Batch::decode(&image)).join().unwrap()).unwrap();
    let mut values = Vec::new();
    for (r, expected) in batch.records().zip(rows) {
        assert_eq!(r.key, expected.key);
        assert_eq!(
            (r.line, r.column, r.value_line, r.value_column),
            (200, 0, 4, 30)
        );
        let Payload::Text(text) = r.payload else {
            panic!()
        };
        assert_eq!(Value::Text(text.as_ref()), expected.value);
        values.push(text);
    }
    drop(image);
    drop(batch);
    assert_eq!(values[0].as_ref(), "É");
}

#[test]
fn all_truncations_corruption_and_wrong_shapes_fail_closed() {
    let rows = [row("a", Value::Argb(0x12345678))];
    let image = encode(Kind::LiteralBrush, &rows).unwrap();
    let batch = Batch::decode(&image).unwrap();
    assert!(matches!(
        batch.records().next().unwrap().payload,
        Payload::Argb(0x12345678)
    ));
    assert!(encode(Kind::Alias, &rows).is_err());
    for end in 0..image.len() {
        assert!(Batch::decode(&image[..end]).is_err());
    }
    let mut changed = image.clone();
    changed.push(0);
    assert!(Batch::decode(&changed).is_err());
    for (offset, byte) in [(4, 0), (5, 255), (6, 127), (8, 1), (10, 255), (11, 1)] {
        let mut changed = image.clone();
        changed[offset] = byte;
        assert!(Batch::decode(&changed).is_err(), "byte {offset}");
    }
}

#[test]
fn canonical_lengths_and_source_positions_are_bounded() {
    for leb in [&[128, 0][..], &[255, 255, 255, 255, 16], &[128; 6]] {
        assert!(uint(&mut &leb[..]).is_err());
    }
    for (base, delta) in [(0, 1), (i32::MAX as u32, 2)] {
        assert!(relative(base, delta).is_err());
    }
}

#[test]
fn text_factories_share_a_wire_layout_and_large_indices_round_trip() {
    let keys: Vec<_> = (0..300)
        .map(|i| format!("SystemControlBrush{i:03}"))
        .collect();
    for kind in [Kind::Alias, Kind::StaticBrush, Kind::DynamicBrush] {
        let rows: Vec<_> = keys
            .iter()
            .rev()
            .map(|key| Resource {
                key,
                value: Value::Text("Shared"),
                line: i32::MAX as u32,
                column: 0,
                value_line: 0,
                value_column: i32::MAX as u32,
            })
            .collect();
        let image = encode(kind, &rows).unwrap();
        let batch = Batch::decode(&image).unwrap();
        assert_eq!(batch.kind(), kind);
        assert_eq!(batch.records().len(), 300);
        for (actual, expected) in batch.records().zip(rows) {
            let Payload::Text(target) = actual.payload else {
                panic!("text operand required")
            };
            assert_eq!(actual.key, expected.key);
            assert_eq!(target.as_ref(), "Shared");
            assert_eq!(
                (
                    actual.line,
                    actual.column,
                    actual.value_line,
                    actual.value_column
                ),
                (i32::MAX, 0, 0, i32::MAX)
            );
        }
    }
}

#[test]
fn deferred_factory_imports_are_local_to_a_compiled_call_site() {
    let rows = [
        row("First", Value::Import(1)),
        row("Second", Value::Import(0)),
        row("Reuse", Value::Import(1)),
    ];
    let bytes = encode(Kind::DeferredFactory, &rows).unwrap();
    let batch = Batch::decode(&bytes).unwrap();
    assert_eq!(batch.kind(), Kind::DeferredFactory);
    assert!(batch.validate_imports(2).is_ok());
    assert!(batch.validate_imports(1).is_err());
    for (record, expected) in batch.records().zip([1, 0, 1]) {
        assert!(matches!(record.payload, Payload::Import(slot) if slot == expected));
    }
    assert!(encode(Kind::Alias, &rows).is_err());
    let oversized = encode(Kind::DeferredFactory, &[row("x", Value::Import(u32::MAX))]).unwrap();
    assert!(Batch::decode(&oversized)
        .unwrap()
        .validate_imports(3)
        .is_err());
}

#[test]
fn numeric_operands_preserve_every_ieee_bit_and_reject_partial_wide_values() {
    let cases = [(-32.0_f64).to_bits(), 222.0_f64.to_bits(), (-0.0_f64).to_bits(), 0.8125_f64.to_bits(),
        f64::INFINITY.to_bits(), f64::NEG_INFINITY.to_bits(), 0x7ff8_1234_5678_9876, 1_u64];
    for bits in cases {
        let data = encode(Kind::Double, &[row("v", Value::Double(bits))]).unwrap();
        assert!(matches!(Batch::decode(&data).unwrap().records().next().unwrap().payload, Payload::Double(v) if v == bits));
    }
    for kind in [Kind::Thickness, Kind::CornerRadius] {
        let quad = [cases[0], cases[7], cases[4], cases[2]];
        let data = encode(kind, &[row("v", Value::Quad(quad))]).unwrap();
        assert!(matches!(Batch::decode(&data).unwrap().records().next().unwrap().payload, Payload::Quad(v) if v == quad));
        for end in 0..data.len() {
            assert!(Batch::decode(&data[..end]).is_err());
        }
    }
}

#[test]
fn predictable_positions_roundtrip_with_explicit_v1_and_overflow_checks() {
    let old = b"FXB\0\x01\x02\x01\x01\x00\x01A\x00\xff\x00\x00\xff\x01\x01\x00\x00";
    assert_eq!(Batch::decode(old).unwrap().records().next().unwrap().line, 1);
    let rows: Vec<_> = (0..3).map(|i| Resource {
        key: "A", value: Value::Import(i), line: i32::MAX as u32 - 2 + i, column: i,
        value_line: i32::MAX as u32 - 3 + i, value_column: i + 4,
    }).collect();
    let image = encode(Kind::DeferredFactory, &rows).unwrap();
    assert_eq!(image[4], 2);
    for (row, expected) in Batch::decode(&image).unwrap().records().zip(&rows) {
        assert_eq!([row.line, row.column, row.value_line, row.value_column],
            [expected.line as i32, expected.column as i32, expected.value_line as i32, expected.value_column as i32]);
    }
    for end in 0..image.len() {
        assert!(Batch::decode(&image[..end]).is_err());
    }
    for flags in [0, 16, 18, 255] {
        let mut bad = image.clone();
        bad[6] = flags;
        assert!(Batch::decode(&bad).is_err());
    }
    // Two rows with a consecutive-line prediction starting at i32::MAX.
    let overflow = b"FXB\0\x02\x05\x1f\x01\x02\x00\x01A\x00\x00\xff\xff\xff\xff\x07\x00\x00\x00\x00\x00";
    assert!(Batch::decode(overflow).is_err());
}

#[test]
fn front_coded_suffixes_preserve_scalar_boundaries_and_reload_handles() {
    let rows = [
        row("ZäCommonSuffix", Value::Text("ZÜCommonSuffix")),
        row("ZäCommonSuffix", Value::Text("ZàCommonSuffix")),
        row("ä", Value::Text("÷")),
    ];
    let image = encode(Kind::Alias, &rows).unwrap();
    assert_eq!(image[4], 3);
    let batch = Batch::decode(&image).unwrap();
    let records: Vec<_> = batch.records().map(|record| {
        let Payload::Text(text) = record.payload else { panic!() };
        (record.key.to_owned(), text)
    }).collect();
    drop(batch);
    for ((key, text), expected) in records.into_iter().zip(rows) {
        assert_eq!(key, expected.key);
        assert_eq!(Value::Text(text.as_ref()), expected.value);
    }
    for n in 0..image.len() {
        assert!(Batch::decode(&image[..n]).is_err());
    }
}

#[test]
fn back_copy_rejects_overlaps_and_invalid_utf8_boundaries() {
    // Previous string starts at UTF-8 "äX". A one-byte insertion after
    // copying one scalar, followed by a previously stored suffix.
    let mut image = b"FXB\0\x03\x05\x00\x02\x01\x00\x06\xc3\xa4X\x02\x03\x01Y\x01\x00\x00\x00\x00\x00".to_vec();
    assert_eq!(Batch::decode(&image).unwrap().records().next().unwrap().key, "äYX");
    image[16] = 2;
    assert!(Batch::decode(&image).is_err());
    image[14] = 0;
    assert!(Batch::decode(&image).is_err());
    image[16] = 4;
    assert!(Batch::decode(&image).is_err());
}


#[test]
fn joined_pool_cannot_split_a_scalar_between_strings() {
    for image in [
        &b"FXB\0\x01\x05\x02\x00\x00\x01\xc3\x00\x01\xa4"[..],
        &b"FXB\0\x03\x05\x00\x02\x00\x00\x02\xc3\x00\x02\xa4"[..],
        &b"FXB\0\x03\x05\x00\x01\x00\x00\x01\x00"[..],
    ] {
        assert!(Batch::decode(image).is_err());
    }
}
