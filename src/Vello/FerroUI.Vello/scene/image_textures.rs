//! What the sinks over a GPU keep of the textures of images between frames.
//!
//! The pixels of an image are uploaded to a texture when the image is first
//! drawn. A texture that a render did not draw is kept for a while: content
//! that scrolls out of view and back, or is drawn now and then, is not
//! uploaded again, which on a phone in a browser is the slow part of a frame
//! that reveals images (a copy of the pixels in the module, and the upload).

use std::collections::HashMap;

/// The renders a texture that is no longer drawn is always kept for.
pub(crate) const IMAGE_TEXTURE_LIFETIME: u64 = 16;

/// The renders a texture that is no longer drawn is kept for at most, while
/// the textures fit [`IMAGE_TEXTURE_BUDGET`].
pub(crate) const IMAGE_TEXTURE_IDLE_LIFETIME: u64 = 3600;

/// The bytes of the textures of images the renders keep beyond those they
/// drew in the last [`IMAGE_TEXTURE_LIFETIME`] renders: those drawn the
/// longest ago are released first.
pub(crate) const IMAGE_TEXTURE_BUDGET: u64 = 128 * 1024 * 1024;

/// The bytes of a texture of RGBA pixels of 8 bits a channel.
pub(crate) fn rgba_bytes(width: u32, height: u32) -> u64 {
    u64::from(width) * u64::from(height) * 4
}

/// Releases the textures of `textures` that render `render_count` does not
/// keep: one drawn in the last [`IMAGE_TEXTURE_LIFETIME`] renders is kept;
/// one drawn before is kept while it was drawn in the last
/// [`IMAGE_TEXTURE_IDLE_LIFETIME`] renders and all the textures, the older
/// ones released first, fit [`IMAGE_TEXTURE_BUDGET`]. `usage` gives the
/// render a texture was last drawn in and its bytes; `release` takes each
/// texture that is not kept.
pub(crate) fn release_unused_textures<T>(
    textures: &mut HashMap<u64, T>,
    render_count: u64,
    usage: impl Fn(&T) -> (u64, u64),
    mut release: impl FnMut(T),
) {
    let mut total = 0;
    let mut idle = Vec::new();
    for (id, texture) in textures.iter() {
        let (last_used, bytes) = usage(texture);
        total += bytes;
        if render_count - last_used >= IMAGE_TEXTURE_LIFETIME {
            idle.push((last_used, bytes, *id));
        }
    }
    idle.sort_unstable();

    for (last_used, bytes, id) in idle {
        if total <= IMAGE_TEXTURE_BUDGET && render_count - last_used < IMAGE_TEXTURE_IDLE_LIFETIME {
            break;
        }
        if let Some(texture) = textures.remove(&id) {
            total -= bytes;
            release(texture);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(textures: &mut HashMap<u64, (u64, u64)>, render_count: u64) -> Vec<u64> {
        let mut released = Vec::new();
        let ids: HashMap<(u64, u64), u64> = textures.iter().map(|(id, usage)| (*usage, *id)).collect();
        release_unused_textures(textures, render_count, |usage| *usage, |usage| released.push(ids[&usage]));
        released.sort_unstable();
        released
    }

    #[test]
    fn a_texture_that_left_the_scene_is_kept_while_the_textures_fit_the_budget() {
        let mut textures = HashMap::from([(1, (10, 5_000_000)), (2, (100, 5_000_000))]);
        assert!(release(&mut textures, 100).is_empty());
        assert_eq!(textures.len(), 2);
    }

    #[test]
    fn the_textures_drawn_the_longest_ago_are_released_first_beyond_the_budget() {
        let mb = 1024 * 1024;
        let mut textures = HashMap::from([(1, (10, 60 * mb)), (2, (20, 60 * mb)), (3, (30, 60 * mb))]);
        assert_eq!(release(&mut textures, 100), vec![1]);
        assert_eq!(textures.len(), 2);
    }

    #[test]
    fn a_texture_drawn_in_the_last_renders_is_kept_beyond_the_budget() {
        let mb = 1024 * 1024;
        let mut textures = HashMap::from([(1, (95, 100 * mb)), (2, (99, 100 * mb))]);
        assert!(release(&mut textures, 100).is_empty());
    }

    #[test]
    fn a_texture_not_drawn_for_long_is_released_within_the_budget() {
        let mut textures = HashMap::from([(1, (1, 1000)), (2, (IMAGE_TEXTURE_IDLE_LIFETIME, 1000))]);
        assert_eq!(release(&mut textures, IMAGE_TEXTURE_IDLE_LIFETIME + 1), vec![1]);
    }
}
