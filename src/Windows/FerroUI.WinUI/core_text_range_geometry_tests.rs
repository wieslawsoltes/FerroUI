use super::*;
use ferroui_base::input::text_input::TextInputMethodClientEvents;
use ferroui_base::{Ref, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct RangeClient {
    events: TextInputMethodClientEvents,
    range: Cell<Option<TextSelection>>,
    reply: RefCell<Option<Result<TextInputMethodRangeGeometry, TextInputMethodError>>>,
}

impl TextInputMethodClient for RangeClient {
    fn events(&self) -> &TextInputMethodClientEvents {
        &self.events
    }
    fn text_view_visual(&self) -> Option<Ref<Visual>> {
        None
    }
    fn supports_preedit(&self) -> bool {
        false
    }
    fn supports_surrounding_text(&self) -> bool {
        false
    }
    fn surrounding_text(&self) -> String {
        String::new()
    }
    fn cursor_rectangle(&self) -> FerroRect {
        FerroRect::default()
    }
    fn selection(&self) -> TextSelection {
        TextSelection::default()
    }
    fn set_selection(&self, _: TextSelection) {
        panic!("geometry must not select text")
    }
    fn text_range_geometry(
        &self,
        range: TextSelection,
    ) -> Result<TextInputMethodRangeGeometry, TextInputMethodError> {
        self.range.set(Some(range));
        self.reply
            .borrow_mut()
            .take()
            .unwrap_or(Err(TextInputMethodError::Unsupported))
    }
}

#[test]
fn core_text_range_geometry_preserves_the_requested_utf16_indices_and_exact_bounds() {
    let client = RangeClient::default();
    let requested = TextSelection::new(3, 7);
    let fragment = TextInputMethodTextBounds::new(FerroRect::new(41.0, 12.0, 20.0, 9.0));
    client.reply.replace(Some(TextInputMethodRangeGeometry::new(
        requested,
        vec![fragment],
        || true,
    )));
    let geometry =
        CoreTextTextGeometry::query(&client, requested, FerroRect::new(1.0, 2.0, 0.0, 0.0))
            .unwrap();
    assert_eq!(client.range.get(), Some(requested));
    let CoreTextTextGeometry::Requested(geometry) = geometry else {
        panic!("requested range capability")
    };
    assert_eq!(geometry.fragments(), &[fragment]);
    assert!(geometry.is_current());
}

#[test]
fn core_text_range_geometry_uses_legacy_caret_only_for_an_unsupported_capability() {
    let client = RangeClient::default();
    let caret = FerroRect::new(2.0, 3.0, 0.0, 0.0);
    let result = CoreTextTextGeometry::query(&client, TextSelection::new(4, 8), caret).unwrap();
    assert!(matches!(result, CoreTextTextGeometry::LegacyCaret(value) if value == caret));
    for error in [
        TextInputMethodError::Unavailable,
        TextInputMethodError::InvalidRange,
        TextInputMethodError::StateChanged,
        TextInputMethodError::StaleConnection,
    ] {
        client.reply.replace(Some(Err(error)));
        assert!(
            matches!(CoreTextTextGeometry::query(&client, TextSelection::new(4, 8), caret), Err(actual) if actual == error)
        );
    }
    client.range.set(None);
    assert!(matches!(
        CoreTextTextGeometry::query(&client, TextSelection::new(8, 4), caret),
        Err(TextInputMethodError::InvalidRange)
    ));
    assert_eq!(
        client.range.get(),
        None,
        "invalid ranges never enter the client"
    );
}

#[test]
fn core_text_range_geometry_rejects_a_wrong_range_or_stale_client_snapshot() {
    let client = RangeClient::default();
    let fragment = TextInputMethodTextBounds::new(FerroRect::new(0.0, 0.0, 2.0, 8.0));
    for (range, current) in [
        (TextSelection::new(0, 1), true),
        (TextSelection::new(1, 2), false),
    ] {
        client.reply.replace(Some(TextInputMethodRangeGeometry::new(
            range,
            vec![fragment],
            move || current,
        )));
        assert!(matches!(
            CoreTextTextGeometry::query(&client, TextSelection::new(1, 2), FerroRect::default()),
            Err(TextInputMethodError::StateChanged)
        ));
    }
}

#[test]
fn core_text_range_geometry_composes_fragment_rotations_before_taking_screen_bounds() {
    let rotation = Matrix::new(0.6, 0.8, -0.8, 0.6, 0.0, 0.0);
    let inverse = Matrix::new(0.6, -0.8, 0.8, 0.6, 0.0, 0.0);
    let fragments = [
        TextInputMethodTextBounds::with_transform(FerroRect::new(0.0, 0.0, 2.0, 1.0), rotation),
        TextInputMethodTextBounds::with_transform(FerroRect::new(10.0, 5.0, 2.0, 1.0), rotation),
    ];
    let bounds = CoreTextLayoutBounds::from_owner_fragments(
        &fragments,
        FerroRect::new(0.0, 0.0, 100.0, 40.0),
        inverse,
        2.0,
        (10, 20),
        |point| {
            Ok(Point {
                x: point.x * 2.0,
                y: point.y * 3.0,
            })
        },
    )
    .unwrap();
    assert!((bounds.text.x - 10.0).abs() < 0.0001);
    assert!((bounds.text.y - 20.0).abs() < 0.0001);
    assert!((bounds.text.width - 48.0).abs() < 0.0001);
    assert!((bounds.text.height - 36.0).abs() < 0.0001);
    assert!(bounds.control.width > bounds.text.width);
    assert!(bounds.control.height > bounds.text.height);
}

#[test]
fn core_text_range_geometry_preserves_exact_small_carets_and_zero_advance_breaks() {
    for fragment in [
        FerroRect::new(3.0, 4.0, 0.0, 7.0),
        FerroRect::new(3.0, 4.0, 1.0, 7.0),
    ] {
        let bounds = CoreTextLayoutBounds::from_owner_fragments(
            &[TextInputMethodTextBounds::new(fragment)],
            FerroRect::new(0.0, 0.0, 100.0, 30.0),
            Matrix::IDENTITY,
            1.5,
            (-20, 30),
            Ok,
        )
        .unwrap();
        assert_eq!(bounds.text.x, -15.5);
        assert_eq!(bounds.text.y, 36.0);
        assert_eq!(bounds.text.width, fragment.width as f32 * 1.5);
        assert_eq!(bounds.text.height, 10.5);
        assert_eq!(bounds.control.width, 150.0);
        assert_eq!(bounds.control.height, 45.0);
    }
}

#[test]
fn core_text_range_geometry_rejects_invalid_fragment_native_points_and_scale() {
    let good = TextInputMethodTextBounds::new(FerroRect::new(0.0, 0.0, 1.0, 10.0));
    let control = FerroRect::new(0.0, 0.0, 100.0, 30.0);
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            CoreTextLayoutBounds::from_owner_fragments(
                &[good],
                control,
                Matrix::IDENTITY,
                scale,
                (0, 0),
                |_| panic!("invalid scale must fail before native transform")
            ),
            Err(HResult::INVALIDARG)
        );
    }
    assert_eq!(
        CoreTextLayoutBounds::from_owner_fragments(&[], control, Matrix::IDENTITY, 1.0, (0, 0), Ok),
        Err(HResult::INVALIDARG)
    );
    assert_eq!(
        CoreTextLayoutBounds::from_owner_fragments(
            &[good],
            control,
            Matrix::IDENTITY,
            1.0,
            (0, 0),
            |_| Ok(Point {
                x: f32::NAN,
                y: 0.0
            })
        ),
        Err(HResult::INVALIDARG)
    );
    assert_eq!(
        CoreTextLayoutBounds::from_owner_fragments(
            &[good],
            control,
            Matrix::new(1.0, 0.0, 0.0, 1.0, f64::MAX, 0.0),
            1.0,
            (0, 0),
            |_| panic!("unrepresentable root coordinates must fail")
        ),
        Err(HResult::INVALIDARG)
    );
}

#[test]
fn core_text_range_geometry_rechecks_snapshot_between_native_publication_setters() {
    let current = Rc::new(Cell::new(true));
    let check = current.clone();
    let geometry = TextInputMethodRangeGeometry::new(
        TextSelection::new(1, 3),
        vec![TextInputMethodTextBounds::new(FerroRect::new(
            5.0, 8.0, 20.0, 10.0,
        ))],
        move || check.get(),
    )
    .unwrap();
    let bounds = CoreTextLayoutBounds::from_owner_fragments(
        geometry.fragments(),
        FerroRect::new(0.0, 0.0, 100.0, 30.0),
        Matrix::IDENTITY,
        1.0,
        (0, 0),
        Ok,
    )
    .unwrap();
    let writes = Cell::new(0);
    assert_eq!(
        bounds.publish(
            || geometry.is_current(),
            |_| {
                writes.set(writes.get() + 1);
                current.set(false);
                Ok(())
            },
            |_| panic!("stale range must not finish its control-bounds publication")
        ),
        Ok(false)
    );
    assert_eq!(writes.get(), 1);
}

#[test]
fn core_text_range_geometry_does_not_discard_zero_area_fragment_positions() {
    let bounds = CoreTextLayoutBounds::from_owner_fragments(
        &[
            TextInputMethodTextBounds::new(FerroRect::new(-5.0, -7.0, 0.0, 0.0)),
            TextInputMethodTextBounds::new(FerroRect::new(10.0, 12.0, 0.0, 0.0)),
        ],
        FerroRect::default(),
        Matrix::IDENTITY,
        1.0,
        (0, 0),
        Ok,
    )
    .unwrap();
    assert_eq!(
        bounds.text,
        Rect {
            x: -5.0,
            y: -7.0,
            width: 15.0,
            height: 19.0
        }
    );
}
