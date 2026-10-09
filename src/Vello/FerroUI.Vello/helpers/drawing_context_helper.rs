use ferroui_base::media::IPen;

/// Tries to create the dash pattern of the pen's dash style: the on and off
/// lengths and the offset into them.
///
/// The dash lengths are relative to the pen thickness. An odd number of
/// dashes is repeated to produce an even number of on/off intervals.
pub fn try_create_dashes(pen: Option<&dyn IPen>) -> Option<(Vec<f64>, f64)> {
    let pen = pen?;
    let dash_style = pen.dash_style()?;
    let src_dashes = dash_style.dashes()?;

    if src_dashes.is_empty() {
        return None;
    }

    let count = if src_dashes.len() % 2 == 0 { src_dashes.len() } else { src_dashes.len() * 2 };
    let thickness = pen.thickness();
    let dashes: Vec<f64> = (0..count).map(|i| src_dashes[i % src_dashes.len()] * thickness).collect();

    // A pattern without a length never advances along the path.
    if dashes.iter().any(|dash| !dash.is_finite() || *dash < 0.0) || dashes.iter().sum::<f64>() <= 0.0 {
        return None;
    }

    let offset = dash_style.offset() * thickness;

    Some((dashes, offset))
}
