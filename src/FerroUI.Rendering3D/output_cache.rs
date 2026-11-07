//! The output of the CPU reference kept for the next frame (`docs/porting/media3d.md`, section
//! 3.7, step 1): a viewport whose snapshot, size, scale and options are unchanged draws the image
//! it drew before, without rasterizing again.
//!
//! A backend keeps one cache on its render thread, with the form of the image it draws (a Skia
//! image, a Vello image), so that the image is also uploaded to a GPU once rather than every frame.

use crate::cpu::{CpuRasterizer, CpuRenderOptions, Scene3DImage};
use ferroui_base::rendering::scene3d::Scene3DSnapshot;
use ferroui_base::PixelSize;
use std::sync::{Arc, Weak};

/// What an output was made for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutputKey {
    generations: (u64, u64, u64),
    size: (i32, i32),
    scale_bits: u64,
    options: CpuRenderOptions,
}

struct OutputEntry<T> {
    /// The snapshot the output was made of. A snapshot is immutable, and a viewport hands out
    /// the same one while nothing changes, so the allocation names the scene; the weak handle
    /// keeps the allocation (not the scene) alive, so its address is not reused while the entry
    /// lives.
    scene: Weak<Scene3DSnapshot>,
    key: OutputKey,
    value: T,
    last_used: u64,
}

/// The outputs of the scenes a backend drew recently, with the rasterizer that makes them.
pub struct Scene3DOutputCache<T> {
    rasterizer: CpuRasterizer,
    entries: Vec<OutputEntry<T>>,
    capacity: usize,
    clock: u64,
}

impl<T: Clone> Default for Scene3DOutputCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Scene3DOutputCache<T> {
    /// The number of outputs a cache keeps by default: the viewports of a window or two.
    pub const DEFAULT_CAPACITY: usize = 8;

    /// An empty cache of [`DEFAULT_CAPACITY`](Self::DEFAULT_CAPACITY) outputs.
    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }

    /// An empty cache of `capacity` outputs (at least one).
    pub fn with_capacity(capacity: usize) -> Self {
        Scene3DOutputCache { rasterizer: CpuRasterizer::new(), entries: Vec::new(), capacity: capacity.max(1), clock: 0 }
    }

    /// The output of `scene` at `size` pixels, `scale` and `options`: the kept one when the
    /// snapshot, its generations, the size, the scale and the options are those it was made for;
    /// otherwise the scene is rasterized and `make` turns the image into the backend's form
    /// (`None` when it cannot, which keeps nothing). A new output of a scene replaces the older
    /// one of the same scene, and the least recently used output makes room for a new one.
    pub fn get_or_render(
        &mut self,
        scene: &Arc<Scene3DSnapshot>,
        size: PixelSize,
        scale: f64,
        options: CpuRenderOptions,
        make: impl FnOnce(Scene3DImage) -> Option<T>,
    ) -> Option<T> {
        self.clock += 1;
        let key = OutputKey {
            generations: scene.generations(),
            size: (size.width, size.height),
            scale_bits: scale.to_bits(),
            options,
        };
        // Outputs of snapshots nobody holds any more are dropped.
        self.entries.retain(|entry| entry.scene.strong_count() > 0);
        let same_scene = |entry: &OutputEntry<T>| std::ptr::eq(entry.scene.as_ptr(), Arc::as_ptr(scene));
        if let Some(entry) = self.entries.iter_mut().find(|entry| same_scene(entry) && entry.key == key) {
            entry.last_used = self.clock;
            return Some(entry.value.clone());
        }
        let image = self.rasterizer.render(scene, size, scale, options);
        let value = make(image)?;
        self.entries.retain(|entry| !same_scene(entry));
        if self.entries.len() >= self.capacity {
            if let Some(oldest) =
                self.entries.iter().enumerate().min_by_key(|(_, entry)| entry.last_used).map(|(index, _)| index)
            {
                self.entries.swap_remove(oldest);
            }
        }
        self.entries.push(OutputEntry { scene: Arc::downgrade(scene), key, value: value.clone(), last_used: self.clock });
        Some(value)
    }

    /// The number of outputs kept.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no output is kept.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drops every output (a lost device, a new context).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
#[path = "output_cache_tests.rs"]
mod tests;
