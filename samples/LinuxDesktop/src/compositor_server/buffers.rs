//! Import committed shm into compositor-owned storage. A client's pool is never
//! referenced by the UI or render threads; release follows the checked copy.
use super::Pixels;
use smithay::{
    reexports::wayland_server::{
        protocol::{wl_buffer::WlBuffer, wl_shm},
        Resource,
    },
    utils::{Buffer, Point, Size, Transform},
    wayland::shm::with_buffer_contents,
};
use std::sync::Arc;

pub(super) fn read(buffer: &WlBuffer, revision: u64) -> Option<Arc<Pixels>> {
    let result = with_buffer_contents(buffer, |pool, length, data| {
        let width = usize::try_from(data.width).ok()?;
        let height = usize::try_from(data.height).ok()?;
        let stride = usize::try_from(data.stride).ok()?;
        let offset = usize::try_from(data.offset).ok()?;
        let row = width.checked_mul(4)?;
        let bytes = row.checked_mul(height)?;
        if bytes == 0
            || bytes > 128 * 1024 * 1024
            || stride < row
            || offset
                .checked_add(stride.checked_mul(height - 1)?)?
                .checked_add(row)?
                > length
        {
            return None;
        }
        let opaque = match data.format {
            wl_shm::Format::Argb8888 => false,
            wl_shm::Format::Xrgb8888 => true,
            _ => return None,
        };
        let mut pixels = vec![0; bytes];
        for (y, dst) in pixels.chunks_exact_mut(row).enumerate() {
            // SAFETY: every row is checked against the mapped pool, Smithay guards
            // SIGBUS on truncated pools, and the destination is private allocation.
            unsafe {
                std::ptr::copy_nonoverlapping(pool.add(offset + y * stride), dst.as_mut_ptr(), row)
            };
        }
        if cfg!(target_endian = "big") {
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.reverse();
            }
        }
        if opaque {
            for alpha in pixels[3..].iter_mut().step_by(4) {
                *alpha = 255;
            }
        }
        Some(Arc::new(Pixels {
            width: data.width,
            height: data.height,
            bytes: pixels,
            revision,
        }))
    });
    match result {
        Ok(Some(pixels)) => Some(pixels),
        Ok(None) => {
            buffer.post_error(
                wl_shm::Error::InvalidStride,
                "Unsupported or oversized shm buffer",
            );
            None
        }
        Err(_) => None,
    }
}

pub(super) fn oriented(source: &Arc<Pixels>, transform: Transform, revision: u64) -> Arc<Pixels> {
    if transform == Transform::Normal {
        return source.clone();
    }
    // wl_surface.set_buffer_transform describes the transform the client
    // applied when drawing. Undo it to produce logical surface orientation.
    let transform = transform.invert();
    let size: Size<i32, Buffer> = (source.width, source.height).into();
    let target = transform.transform_size(size);
    // Transforming pixel centers is equivalent to transforming integer sample
    // indices within a rectangle whose maximum coordinates are (w-1,h-1).
    let bounds = Size::from((size.w - 1, size.h - 1));
    let mut bytes = vec![0; source.bytes.len()];
    for y in 0..size.h {
        for x in 0..size.w {
            let point = transform.transform_point_in(Point::<i32, Buffer>::from((x, y)), &bounds);
            let dst = (point.y as usize * target.w as usize + point.x as usize) * 4;
            let src = (y as usize * size.w as usize + x as usize) * 4;
            bytes[dst..dst + 4].copy_from_slice(&source.bytes[src..src + 4]);
        }
    }
    Arc::new(Pixels {
        width: target.w,
        height: target.h,
        bytes,
        revision,
    })
}
