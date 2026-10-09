use ferroui_base::media::IPen;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Gets a hash code for a pen, optionally including the brush. A missing pen
/// hashes to `0`.
// The same function as the Skia backend's: it is neutral of the backend and
// listed for sharing in the design document.
pub fn get_hash_code(pen: Option<&dyn IPen>, include_brush: bool) -> u64 {
    let Some(pen) = pen else {
        return 0;
    };

    let mut hash = DefaultHasher::new();
    pen.line_cap().hash(&mut hash);
    pen.line_join().hash(&mut hash);
    pen.miter_limit().to_bits().hash(&mut hash);
    pen.thickness().to_bits().hash(&mut hash);

    if let Some(dash_style) = pen.dash_style() {
        dash_style.offset().to_bits().hash(&mut hash);
        if let Some(dashes) = dash_style.dashes() {
            for dash in dashes {
                dash.to_bits().hash(&mut hash);
            }
        }
    }

    if include_brush {
        // Brushes hash by identity.
        pen.brush().map(|brush| brush.reference_id()).hash(&mut hash);
    }

    hash.finish()
}
