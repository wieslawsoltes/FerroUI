//! Original layout converters through the native converter contract.
use crate::converters::*;
use ferroui_base::{
    data::{
        converters::{IMultiValueConverter, IValueConverter},
        core::ValueType,
    },
    utilities::CultureInfo,
    BoxedValue, UnsetValueType,
};
use std::rc::Rc;
fn roundtrip<M: Clone + PartialEq + 'static, N: Clone + PartialEq + 'static>(
    converter: &dyn IValueConverter,
    pairs: &[(M, N)],
) {
    let culture = CultureInfo::invariant_culture();
    for (m, n) in pairs {
        let value: BoxedValue = Rc::new(m.clone());
        let result = converter
            .convert(Some(&value), ValueType::of::<N>(), None, &culture)
            .unwrap()
            .unwrap();
        assert!(result.downcast_ref::<N>().is_some_and(|v| v == n));
        let value: BoxedValue = Rc::new(n.clone());
        let result = converter
            .convert_back(Some(&value), ValueType::of::<M>(), None, &culture)
            .unwrap()
            .unwrap();
        assert!(result.downcast_ref::<M>().is_some_and(|v| v == m));
    }
    let other: BoxedValue = Rc::new("unrelated".to_owned());
    for back in [false, true] {
        let result = if back {
            converter.convert_back(Some(&other), ValueType::of::<M>(), None, &culture)
        } else {
            converter.convert(Some(&other), ValueType::of::<N>(), None, &culture)
        }
        .unwrap()
        .unwrap();
        assert!(Rc::ptr_eq(&result, &other));
        let result = if back {
            converter.convert_back(None, ValueType::of::<M>(), None, &culture)
        } else {
            converter.convert(None, ValueType::of::<N>(), None, &culture)
        }
        .unwrap()
        .unwrap();
        assert!(result.is::<UnsetValueType>());
    }
}
#[test]
fn source_orientation_and_alignment_roundtrips_preserve_passthrough_and_null() {
    use ferrodock_model::core::{Alignment as A, Orientation as M};
    use ferroui_base::layout::Orientation as N;
    use ferroui_controls::Dock;
    roundtrip(
        &*OrientationConverter::instance(),
        &[(M::Horizontal, N::Horizontal), (M::Vertical, N::Vertical)],
    );
    roundtrip(
        &*AlignmentConverter::instance(),
        &[
            (A::Left, Dock::Left),
            (A::Bottom, Dock::Bottom),
            (A::Right, Dock::Right),
            (A::Top, Dock::Top),
        ],
    );
    let unset: BoxedValue = Rc::new(A::Unset);
    assert!(AlignmentConverter::instance()
        .convert(
            Some(&unset),
            ValueType::of::<Dock>(),
            None,
            &CultureInfo::invariant_culture()
        )
        .unwrap()
        .unwrap()
        .is::<UnsetValueType>());
}
#[test]
fn source_dock_mode_all_edges_roundtrip_and_center_returns_unset() {
    use ferrodock_model::core::DockMode as M;
    use ferroui_controls::Dock as N;
    assert!(
        include_str!("../../../tests/Dock/data/numeric-enum-markup-reference.tsv")
            .lines()
            .any(|row| row == "numeric-dock-mode-converter-source-fallback\t1")
    );
    roundtrip(
        &*DockModeConverter::instance(),
        &[
            (M::Left, N::Left),
            (M::Bottom, N::Bottom),
            (M::Right, N::Right),
            (M::Top, N::Top),
        ],
    );
    for mode in [
        M::Center,
        M::from_raw(-19),
        M::from_raw(123),
        M::from_raw(i32::MIN),
        M::from_raw(i32::MAX),
    ] {
        let center: BoxedValue = Rc::new(mode);
        assert!(DockModeConverter::instance()
            .convert(
                Some(&center),
                ValueType::of::<N>(),
                None,
                &CultureInfo::invariant_culture()
            )
            .unwrap()
            .unwrap()
            .is::<UnsetValueType>());
    }
}
#[test]
fn original_split_view_display_modes_and_all_four_placements_convert_both_ways() {
    use ferrodock_model::core::{SplitViewDisplayMode as M, SplitViewPanePlacement as P};
    use ferroui_controls::{SplitViewDisplayMode as N, SplitViewPanePlacement as Q};
    roundtrip(
        &*SplitViewDisplayModeConverter::instance(),
        &[
            (M::Inline, N::Inline),
            (M::CompactInline, N::CompactInline),
            (M::Overlay, N::Overlay),
            (M::CompactOverlay, N::CompactOverlay),
        ],
    );
    roundtrip(
        &*SplitViewPanePlacementConverter::instance(),
        &[
            (P::Left, Q::Left),
            (P::Right, Q::Right),
            (P::Top, Q::Top),
            (P::Bottom, Q::Bottom),
        ],
    );
}
#[test]
fn original_either_converter_returns_first_non_null_same_box_including_false_and_zero() {
    let c = EitherNotNullConverter::instance();
    let culture = CultureInfo::invariant_culture();
    for value in [
        Rc::new(false) as BoxedValue,
        Rc::new(0_i32),
        Rc::new(String::new()),
    ] {
        let result = c
            .convert(
                &[None, Some(value.clone()), Some(Rc::new(2_i32))],
                ValueType::of::<bool>(),
                None,
                &culture,
            )
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&result, &value));
    }
    let result = c
        .convert(&[None, None], ValueType::of::<bool>(), None, &culture)
        .unwrap()
        .unwrap();
    assert_eq!(
        result
            .downcast_ref::<Vec<Option<BoxedValue>>>()
            .unwrap()
            .len(),
        2
    );
}
