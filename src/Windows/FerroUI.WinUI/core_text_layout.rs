//! CoreText requires separate text and whole-control rectangles in screen
//! pixels. Transform requested shaped fragments through the complete visual
//! chain before taking their bounds, then apply rasterization and screen offset.
use crate::interop::{Point, Rect};
use ferroui_base::input::text_input::{
    TextInputMethodClient, TextInputMethodError, TextInputMethodRangeGeometry,
    TextInputMethodTextBounds, TextSelection,
};
use ferroui_base::{Matrix, Point as FerroPoint, Rect as FerroRect};
use ferroui_microcom::HResult;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CoreTextLayoutBounds {
    pub text: Rect,
    pub control: Rect,
}

/// An explicit compatibility boundary. Only clients without a range capability
/// use the legacy root-relative caret; an unavailable rich range has no reply.
pub(crate) enum CoreTextTextGeometry {
    Requested(TextInputMethodRangeGeometry),
    LegacyCaret(FerroRect),
}

impl CoreTextTextGeometry {
    pub(crate) fn query(
        client: &dyn TextInputMethodClient,
        range: TextSelection,
        legacy_caret: FerroRect,
    ) -> Result<Self, TextInputMethodError> {
        if range.start < 0 || range.end < range.start {
            return Err(TextInputMethodError::InvalidRange);
        }
        match client.text_range_geometry(range) {
            Ok(geometry) if geometry.range() == range && geometry.is_current() => {
                Ok(Self::Requested(geometry))
            }
            Ok(_) => Err(TextInputMethodError::StateChanged),
            Err(TextInputMethodError::Unsupported) => Ok(Self::LegacyCaret(legacy_caret)),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn is_current(&self) -> bool {
        match self {
            Self::Requested(geometry) => geometry.is_current(),
            Self::LegacyCaret(_) => true,
        }
    }
}

fn native_rect(rect: FerroRect) -> Result<Rect, HResult> {
    let values = [rect.x, rect.y, rect.width, rect.height];
    if values
        .into_iter()
        .any(|value| !value.is_finite() || !(value as f32).is_finite())
        || rect.width < 0.0
        || rect.height < 0.0
    {
        return Err(HResult::INVALIDARG);
    }
    Ok(Rect {
        x: rect.x as f32,
        y: rect.y as f32,
        width: rect.width as f32,
        height: rect.height as f32,
    })
}

fn screen_rect(rect: Rect, scale: f64, origin: (i32, i32)) -> Result<Rect, HResult> {
    native_rect(FerroRect::new(
        f64::from(rect.x) * scale + f64::from(origin.0),
        f64::from(rect.y) * scale + f64::from(origin.1),
        f64::from(rect.width) * scale,
        f64::from(rect.height) * scale,
    ))
}

fn native_point(point: FerroPoint) -> Result<Point, HResult> {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || !(point.x as f32).is_finite()
        || !(point.y as f32).is_finite()
    {
        return Err(HResult::INVALIDARG);
    }
    Ok(Point {
        x: point.x as f32,
        y: point.y as f32,
    })
}

fn transformed_fragment(
    fragment: &TextInputMethodTextBounds,
    owner_to_root: Matrix,
    transform_point: &mut impl FnMut(Point) -> Result<Point, HResult>,
) -> Result<FerroRect, HResult> {
    native_rect(fragment.rectangle)?;
    let mut minimum = FerroPoint::new(f64::INFINITY, f64::INFINITY);
    let mut maximum = FerroPoint::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for corner in fragment.corners() {
        let point = native_point(corner.transform(owner_to_root))?;
        let point = transform_point(point)?;
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(HResult::INVALIDARG);
        }
        minimum.x = minimum.x.min(f64::from(point.x));
        minimum.y = minimum.y.min(f64::from(point.y));
        maximum.x = maximum.x.max(f64::from(point.x));
        maximum.y = maximum.y.max(f64::from(point.y));
    }
    Ok(FerroRect::from_points(minimum, maximum))
}

impl CoreTextLayoutBounds {
    /// Transforms each shaped fragment through the complete visual chain before
    /// taking its bounds and union. Disjoint bidi/line boxes and rotations do
    /// not create an enlarged intermediate owner or root bounding rectangle.
    pub(crate) fn from_owner_fragments(
        fragments: &[TextInputMethodTextBounds],
        control: FerroRect,
        owner_to_root: Matrix,
        scale: f64,
        origin: (i32, i32),
        mut transform_point: impl FnMut(Point) -> Result<Point, HResult>,
    ) -> Result<Self, HResult> {
        if fragments.is_empty() || !scale.is_finite() || scale <= 0.0 {
            return Err(HResult::INVALIDARG);
        }
        let mut text: Option<FerroRect> = None;
        for fragment in fragments {
            let bounds = transformed_fragment(fragment, owner_to_root, &mut transform_point)?;
            text = Some(match text {
                None => bounds,
                Some(previous) => {
                    let left = previous.x.min(bounds.x);
                    let top = previous.y.min(bounds.y);
                    FerroRect::new(
                        left,
                        top,
                        previous.right().max(bounds.right()) - left,
                        previous.bottom().max(bounds.bottom()) - top,
                    )
                }
            });
        }
        let control = transformed_fragment(
            &TextInputMethodTextBounds::new(control),
            owner_to_root,
            &mut transform_point,
        )?;
        Ok(Self {
            // No caret minimum is added to exact range geometry. In particular,
            // paragraph marks keep their measured height and zero advance.
            text: screen_rect(native_rect(text.unwrap())?, scale, origin)?,
            control: screen_rect(native_rect(control)?, scale, origin)?,
        })
    }

    pub(crate) fn from_root_bounds(
        caret: FerroRect,
        control: FerroRect,
        scale: f64,
        origin: (i32, i32),
        mut transform_bounds: impl FnMut(Rect) -> Result<Rect, HResult>,
    ) -> Result<Self, HResult> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(HResult::INVALIDARG);
        }
        // Preserve the adapter's caret fallback, but apply it in root DIPs
        // before the native transform. It must never inflate ControlBounds.
        native_rect(caret)?;
        let text = native_rect(FerroRect::new(
            caret.x,
            caret.y,
            caret.width.max(1.0),
            caret.height.max(16.0),
        ))?;
        let control = native_rect(control)?;
        let text = screen_rect(transform_bounds(text)?, scale, origin)?;
        let control = screen_rect(transform_bounds(control)?, scale, origin)?;
        Ok(Self { text, control })
    }

    /// Native property setters can synchronously change focus, start another
    /// composition, or cancel/supersede this request. Never finish publishing
    /// the old owner's pair after such a callback, nor after a failed setter.
    pub(crate) fn publish(
        self,
        mut is_current: impl FnMut() -> bool,
        mut set_text: impl FnMut(Rect) -> Result<(), HResult>,
        mut set_control: impl FnMut(Rect) -> Result<(), HResult>,
    ) -> Result<bool, HResult> {
        if !is_current() {
            return Ok(false);
        }
        set_text(self.text)?;
        if !is_current() {
            return Ok(false);
        }
        set_control(self.control)?;
        Ok(is_current())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::Matrix;
    use std::cell::{Cell, RefCell};

    fn transform(rect: Rect, matrix: Matrix) -> Result<Rect, HResult> {
        native_rect(
            FerroRect::new(
                f64::from(rect.x),
                f64::from(rect.y),
                f64::from(rect.width),
                f64::from(rect.height),
            )
            .transform_to_aabb(matrix),
        )
    }

    #[test]
    fn core_text_layout_transforms_the_whole_control_and_caret_before_screen_scaling() {
        let input = RefCell::new(Vec::new());
        let bounds = CoreTextLayoutBounds::from_root_bounds(
            FerroRect::new(10.0, 20.0, 0.0, 18.0),
            FerroRect::new(5.0, 8.0, 100.0, 40.0),
            1.5,
            (-200, 300),
            |rect| {
                input.borrow_mut().push(rect);
                // A nonuniform native panel transform changes both extents.
                transform(rect, Matrix::new(2.0, 0.0, 0.0, 3.0, 4.0, 6.0))
            },
        )
        .unwrap();
        assert_eq!(
            *input.borrow(),
            [
                Rect {
                    x: 10.0,
                    y: 20.0,
                    width: 1.0,
                    height: 18.0
                },
                Rect {
                    x: 5.0,
                    y: 8.0,
                    width: 100.0,
                    height: 40.0
                },
            ]
        );
        assert_eq!(
            bounds.text,
            Rect {
                x: -164.0,
                y: 399.0,
                width: 3.0,
                height: 81.0
            }
        );
        assert_eq!(
            bounds.control,
            Rect {
                x: -179.0,
                y: 345.0,
                width: 300.0,
                height: 180.0
            }
        );
    }

    #[test]
    fn core_text_layout_rotation_and_reflection_use_all_rectangle_edges() {
        let bounds = CoreTextLayoutBounds::from_root_bounds(
            FerroRect::new(10.0, 20.0, 2.0, 18.0),
            FerroRect::new(5.0, 8.0, 100.0, 40.0),
            2.0,
            (50, -30),
            |rect| transform(rect, Matrix::new(0.0, -1.0, -2.0, 0.0, 200.0, 100.0)),
        )
        .unwrap();
        assert_eq!(
            bounds.text,
            Rect {
                x: 298.0,
                y: 146.0,
                width: 72.0,
                height: 4.0
            }
        );
        assert_eq!(
            bounds.control,
            Rect {
                x: 258.0,
                y: -40.0,
                width: 160.0,
                height: 200.0
            }
        );
    }

    #[test]
    fn core_text_layout_caret_minimum_does_not_inflate_an_empty_control() {
        let bounds = CoreTextLayoutBounds::from_root_bounds(
            FerroRect::new(3.0, 4.0, 0.0, 0.0),
            FerroRect::new(2.0, 1.0, 0.0, 0.0),
            1.0,
            (0, 0),
            Ok,
        )
        .unwrap();
        assert_eq!(
            bounds.text,
            Rect {
                x: 3.0,
                y: 4.0,
                width: 1.0,
                height: 16.0
            }
        );
        assert_eq!(
            bounds.control,
            Rect {
                x: 2.0,
                y: 1.0,
                width: 0.0,
                height: 0.0
            }
        );
    }

    #[test]
    fn core_text_layout_rejects_invalid_native_geometry_instead_of_publishing_it() {
        let caret = FerroRect::new(1.0, 2.0, 1.0, 16.0);
        let control = FerroRect::new(0.0, 0.0, 100.0, 30.0);
        for scale in [0.0, -1.0, f64::INFINITY, f64::NAN] {
            assert_eq!(
                CoreTextLayoutBounds::from_root_bounds(caret, control, scale, (0, 0), |_| {
                    panic!("invalid scale must fail before the native transform")
                }),
                Err(HResult::INVALIDARG)
            );
        }
        for invalid in [
            FerroRect::new(f64::NAN, 0.0, 1.0, 16.0),
            FerroRect::new(0.0, 0.0, f64::INFINITY, 16.0),
            FerroRect::new(0.0, 0.0, -1.0, 16.0),
            FerroRect::new(f64::MAX, 0.0, 1.0, 16.0),
        ] {
            assert_eq!(
                CoreTextLayoutBounds::from_root_bounds(invalid, control, 1.0, (0, 0), Ok),
                Err(HResult::INVALIDARG)
            );
        }
        assert_eq!(
            CoreTextLayoutBounds::from_root_bounds(caret, control, 1.0, (0, 0), |_| {
                Ok(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: f32::INFINITY,
                    height: 2.0,
                })
            }),
            Err(HResult::INVALIDARG)
        );
    }

    fn bounds() -> CoreTextLayoutBounds {
        CoreTextLayoutBounds::from_root_bounds(
            FerroRect::new(10.0, 20.0, 1.0, 18.0),
            FerroRect::new(0.0, 0.0, 100.0, 40.0),
            1.0,
            (0, 0),
            Ok,
        )
        .unwrap()
    }

    #[test]
    fn core_text_layout_canceled_or_superseded_requests_publish_neither_rectangle() {
        assert_eq!(
            bounds().publish(
                || false,
                |_| panic!("a stale request must not publish text bounds"),
                |_| panic!("a stale request must not publish control bounds"),
            ),
            Ok(false)
        );
    }

    #[test]
    fn core_text_layout_reentrant_replacement_does_not_finish_the_old_bounds_pair() {
        let current = Cell::new(1);
        let writes = RefCell::new(Vec::new());
        assert_eq!(
            bounds().publish(
                || current.get() == 1,
                |rect| {
                    writes.borrow_mut().push(("old text", rect));
                    current.set(2);
                    let mut replacement = bounds();
                    replacement.control.width = 250.0;
                    assert!(
                        replacement
                            .publish(
                                || current.get() == 2,
                                |rect| {
                                    writes.borrow_mut().push(("new text", rect));
                                    Ok(())
                                },
                                |rect| {
                                    writes.borrow_mut().push(("new control", rect));
                                    Ok(())
                                },
                            )
                            .unwrap()
                    );
                    Ok(())
                },
                |_| panic!("old control bounds must not follow the replacement"),
            ),
            Ok(false)
        );
        assert_eq!(
            writes
                .borrow()
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>(),
            ["old text", "new text", "new control"]
        );
        assert_eq!(writes.borrow().last().unwrap().1.width, 250.0);
    }

    #[test]
    fn core_text_layout_native_failure_stops_publication_without_hiding_the_error() {
        assert_eq!(
            bounds().publish(
                || true,
                |_| Err(HResult::FAIL),
                |_| panic!("failed text bounds must not publish a control rectangle"),
            ),
            Err(HResult::FAIL)
        );
    }
}

#[cfg(test)]
#[path = "core_text_range_geometry_tests.rs"]
mod range_geometry_tests;
