use super::*;
use ferroui_media::frames::{PixelTransferPurpose, PresentationRoute};
use rxing::{
    common::{DetectorRXingResult, HybridBinarizer, PerspectiveTransform, Quadrilateral},
    multi::qrcode::detector::MultiDetector,
    point,
    qrcode::decoder::qrcode_decoder,
    Binarizer, DecodeHints, LuminanceSource, Point,
};
use std::borrow::Cow;
struct Source<'a> {
    plane: AnalysisPlane<'a>,
    counters: FrameTransferCounters,
    inverted: bool,
}
impl Source<'_> {
    fn copy(&self, count: usize) {
        self.counters
            .cpu_transfer(
                PixelTransferPurpose::Analysis,
                count as u64,
                PresentationRoute::SharedTexture,
            )
            .expect("analysis allowed");
    }
    fn at(&self, x: usize, y: usize) -> u8 {
        let value = self.plane.data[y * self.plane.row_stride + x * self.plane.pixel_stride];
        if self.inverted {
            255 - value
        } else {
            value
        }
    }
}
impl LuminanceSource for Source<'_> {
    fn get_width(&self) -> usize {
        self.plane.width as usize
    }
    fn get_height(&self) -> usize {
        self.plane.height as usize
    }
    fn get_row(&self, y: usize) -> Option<Cow<'_, [u8]>> {
        if y >= self.get_height() {
            return None;
        }
        if self.plane.pixel_stride == 1 && !self.inverted {
            let offset = y * self.plane.row_stride;
            Some(Cow::Borrowed(
                &self.plane.data[offset..offset + self.get_width()],
            ))
        } else {
            self.copy(self.get_width());
            Some(Cow::Owned(
                (0..self.get_width()).map(|x| self.at(x, y)).collect(),
            ))
        }
    }
    fn get_column(&self, x: usize) -> Cow<'_, [u8]> {
        if x >= self.get_width() {
            return Cow::Borrowed(&[]);
        }
        self.copy(self.get_height());
        Cow::Owned((0..self.get_height()).map(|y| self.at(x, y)).collect())
    }
    fn get_matrix(&self) -> Cow<'_, [u8]> {
        let len = self.get_width() * self.get_height();
        if self.plane.pixel_stride == 1
            && self.plane.row_stride == self.get_width()
            && !self.inverted
        {
            Cow::Borrowed(&self.plane.data[..len])
        } else {
            self.copy(len);
            Cow::Owned(
                (0..self.get_height())
                    .flat_map(|y| (0..self.get_width()).map(move |x| self.at(x, y)))
                    .collect(),
            )
        }
    }
    fn invert(&mut self) {
        self.inverted = !self.inverted;
    }
    fn get_luma8_point(&self, x: usize, y: usize) -> u8 {
        self.at(x, y)
    }
}
/// Runs on the host's bounded analysis worker. The analysis token owns the
/// camera lease throughout the synchronous decoder; it never owns the camera.
pub fn decode(
    analysis: &QrAnalysis,
    plane: AnalysisPlane<'_>,
) -> Result<Vec<QrCandidate>, QrError> {
    if !analysis.is_valid() {
        return Err(QrError::Cancelled);
    }
    plane.validate(analysis.options)?;
    let (width, height) = (plane.width, plane.height);
    let source = Source {
        plane,
        counters: analysis.counters(),
        inverted: false,
    };
    let binarizer = HybridBinarizer::new(source);
    let bits = binarizer
        .get_black_matrix()
        .map_err(|_| QrError::DecodeFailure)?;
    let hints = DecodeHints::default();
    let detected = match MultiDetector::new(bits).detectMulti(&hints) {
        Ok(detected) => detected,
        Err(rxing::Exceptions::NotFoundException(_)) => return Ok(Vec::new()),
        Err(_) => return Err(QrError::DecodeFailure),
    };
    let mut output = Vec::new();
    for detection in detected {
        let Ok(result) = qrcode_decoder::decode_bitmatrix_with_hints(detection.getBits(), &hints)
        else {
            continue;
        };
        let points = detection.getPoints();
        if points.len() < 3 {
            continue;
        }
        let d = detection.getBits().width() as f32;
        let last = d - 3.5;
        let alignment = points.get(3).copied();
        let far = if alignment.is_some() { last - 3. } else { last };
        let bottom_right = alignment.unwrap_or_else(|| {
            point(
                points[2].x + points[0].x - points[1].x,
                points[2].y + points[0].y - points[1].y,
            )
        });
        let Ok(transform) = PerspectiveTransform::quadrilateralToQuadrilateral(
            Quadrilateral::new(
                point(3.5, 3.5),
                point(last, 3.5),
                point(far, far),
                point(3.5, last),
            ),
            Quadrilateral::new(points[1], points[2], bottom_right, points[0]),
        ) else {
            continue;
        };
        let corners = [point(0., 0.), point(d, 0.), point(d, d), point(0., d)].map(|p: Point| {
            let p = transform.transform_point(p);
            (p.x as f64 / width as f64, p.y as f64 / height as f64)
        });
        if !corners.iter().all(|(x, y)| x.is_finite() && y.is_finite()) {
            continue;
        }
        output.push(QrCandidate {
            frame: analysis.frame.metadata().identity,
            timestamp: analysis.frame.metadata().timestamp,
            frame_geometry: analysis.frame.metadata().into(),
            text: (!result.getText().contains('\u{fffd}')).then(|| result.getText().to_owned()),
            codewords: Some(result.getRawBytes().clone()),
            byte_segments: result.getByteSegments().clone(),
            payload_bytes: None,
            corners,
            geometry: if alignment.is_some() {
                GeometryQuality::ProjectedGrid
            } else {
                GeometryQuality::EstimatedFromFinderPatterns
            },
        });
    }
    if !analysis.is_valid() {
        return Err(QrError::Cancelled);
    }
    Ok(output)
}
