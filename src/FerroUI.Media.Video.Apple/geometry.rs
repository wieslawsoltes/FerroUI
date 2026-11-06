//! Decompose the track's encoded-pixel transform without rotating/copying pixels.
use ferroui_media::frames::{FrameRegion, FrameRotation};
use ferroui_media::{MediaError, MediaErrorKind};

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct Geometry {
    pub transform: [f64; 4], // CGAffineTransform a, b, c, d; translation normalizes away.
    pub clean: [f64; 4],     // Top-left encoded-pixel x, y, width, height.
    pub pixel_aspect: f64,
}
impl Geometry {
    pub fn decompose(
        &self,
        width: u32,
        height: u32,
    ) -> Result<(FrameRegion, FrameRotation, bool), MediaError> {
        let unsupported = || {
            MediaError::new(MediaErrorKind::Unsupported,
            "Video geometry requires an integral clean aperture, square pixels and an orthogonal track transform")
        };
        if !self.pixel_aspect.is_finite() || (self.pixel_aspect - 1.).abs() > 1e-6 {
            return Err(unsupported());
        }
        let mut crop = [0; 4];
        for (out, value) in crop.iter_mut().zip(self.clean) {
            if !value.is_finite()
                || value < 0.
                || value > f64::from(u32::MAX)
                || (value - value.round()).abs() > 1e-6
            {
                return Err(unsupported());
            }
            *out = value.round() as u32;
        }
        let [x, y, w, h] = crop;
        if w == 0
            || h == 0
            || x.checked_add(w).is_none_or(|v| v > width)
            || y.checked_add(h).is_none_or(|v| v > height)
        {
            return Err(unsupported());
        }
        // Mirror is applied after rotation by the shared frame geometry contract.
        for (matrix, rotation, mirror) in [
            ([1., 0., 0., 1.], FrameRotation::None, false),
            ([0., 1., -1., 0.], FrameRotation::Clockwise90, false),
            ([-1., 0., 0., -1.], FrameRotation::Clockwise180, false),
            ([0., -1., 1., 0.], FrameRotation::Clockwise270, false),
            ([-1., 0., 0., 1.], FrameRotation::None, true),
            ([0., 1., 1., 0.], FrameRotation::Clockwise90, true),
            ([1., 0., 0., -1.], FrameRotation::Clockwise180, true),
            ([0., -1., -1., 0.], FrameRotation::Clockwise270, true),
        ] {
            if self
                .transform
                .iter()
                .zip(matrix)
                .all(|(a, b)| a.is_finite() && (a - b).abs() <= 1e-6)
            {
                return Ok((
                    FrameRegion {
                        x,
                        y,
                        width: w,
                        height: h,
                    },
                    rotation,
                    mirror,
                ));
            }
        }
        Err(unsupported())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decomposition_matches_independent_affine_points() {
        use ferroui_media::frames::*;
        for quarter in 0..4 {
            for mirror in [false, true] {
                let angle = f64::from(quarter) * std::f64::consts::FRAC_PI_2;
                let (sin, cos) = angle.sin_cos();
                let sign = if mirror { -1. } else { 1. };
                let [a, b, c, d] = [cos * sign, sin, -sin * sign, cos];
                let g = Geometry {
                    transform: [a, b, c, d],
                    clean: [8., 4., 144., 80.],
                    pixel_aspect: 1.,
                };
                let (visible, rotation, mirrored) = g.decompose(160, 90).unwrap();
                assert_eq!(
                    (visible.x, visible.y, visible.width, visible.height),
                    (8, 4, 144, 80)
                );
                let frame = FrameMetadata {
                    identity: FrameIdentity {
                        source: 1,
                        generation: 1,
                        sequence: 1,
                    },
                    width: 160,
                    height: 90,
                    visible,
                    rotation,
                    mirrored,
                    format: FrameFormat::Bgra8,
                    color: FrameColor::Srgb,
                    alpha: FrameAlpha::Opaque,
                    top_left_origin: true,
                    timestamp: std::time::Duration::ZERO,
                    device: None,
                    device_generation: 0,
                    planes: vec![FramePlane {
                        offset: 0,
                        stride: 640,
                        length: 57600,
                    }],
                    retained_bytes: 57600,
                };
                // Transform the unit square with ordinary affine algebra and
                // normalize its bounding rectangle, independently of enum mapping.
                let min_x = a.min(0.) + c.min(0.);
                let min_y = b.min(0.) + d.min(0.);
                for (x, y) in [(0., 0.), (1., 0.), (0., 1.), (1., 1.), (0.2, 0.7)] {
                    let expected = (
                        (a * x + c * y - min_x) / (a.abs() + c.abs()),
                        (b * x + d * y - min_y) / (b.abs() + d.abs()),
                    );
                    let actual = frame.orient(x, y).unwrap();
                    assert!((actual.0 - expected.0).abs() < 1e-6);
                    assert!((actual.1 - expected.1).abs() < 1e-6);
                }
            }
        }
    }
    #[test]
    fn unsupported_geometry_is_rejected_instead_of_misrepresented() {
        let valid = Geometry {
            transform: [1., 0., 0., 1.],
            clean: [0., 0., 160., 90.],
            pixel_aspect: 1.,
        };
        for g in [
            Geometry {
                transform: [1., 0.1, 0., 1.],
                ..valid
            },
            Geometry {
                transform: [f64::NAN, 0., 0., 1.],
                ..valid
            },
            Geometry {
                clean: [0.5, 0., 159., 90.],
                ..valid
            },
            Geometry {
                clean: [10., 0., 160., 90.],
                ..valid
            },
            Geometry {
                pixel_aspect: 1.1,
                ..valid
            },
        ] {
            assert!(g.decompose(160, 90).is_err());
        }
    }
}
