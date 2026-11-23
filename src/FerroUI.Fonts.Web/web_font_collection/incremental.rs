//! Incremental Font Transfer for a face (feature `incremental`): what the text needs, fetching
//! the patches, and swapping in the extended font.
//!
//! The text is seen where it is shaped, so nothing is asked of the text stack. Patches found for
//! the code points seen so far are fetched and applied as one batch, off the UI thread where
//! there are threads and in slices where there are none, and the extended font goes through the
//! sanitizer again. A patch that fails ends the extension: the face moves on to its next `src`
//! entry, as the standard does not allow a font that failed to be extended further.

use super::*;
use ferroui_fonts_web_core::ift::{
    patch_maps_from_tables, patches_to_load, read_patch_maps, GlyphPatchJob, PatchMap, SubsetTarget, DEFAULT_FEATURES,
    IFTX_TABLE, IFT_TABLE, MAX_PATCHES,
};
use ferroui_base::media::FontFeature;
use ferroui_base::platform::ITextShaperImpl;
use ferroui_fonts_web_core::sfnt::Tag;
use std::collections::HashSet;

/// The patches of one font fetched at once. The rest follow when these are applied.
const MAX_PATCH_FETCHES: usize = 8;

/// The extension of one face.
pub(super) struct IncrementalState {
    /// The URL of the font, which the URLs of its patches are relative to.
    url: String,
    /// Tells the work of this state from that of one it replaced.
    pub(super) generation: u64,
    target: HashSet<u32>,
    features: Vec<Tag>,
    design_space: Vec<(Tag, i32, i32)>,
    /// The patches already asked for, as the map writes their URLs.
    requested: HashSet<String>,
    in_flight: usize,
    failed: bool,
    received: Vec<(String, Vec<u8>)>,
    pub(super) applying: bool,
    dirty: bool,
}

/// A batch of patches being applied on the UI thread.
pub(super) struct PatchWork {
    id: FontFaceId,
    generation: u64,
    job: GlyphPatchJob,
}

fn tag_of(name: &str) -> Option<Tag> {
    name.as_bytes().try_into().ok()
}

/// The URL of a patch. It stays with the font: same scheme, host and port, and for a font on disk
/// inside its folder, as a font from the network must not be able to name anything else.
pub(super) fn patch_url(font: &str, patch: &str) -> Option<String> {
    let Ok(base) = url::Url::parse(font) else {
        // A relative font, resolved by its fetcher: a relative patch, without climbing out.
        if url::Url::parse(patch).is_ok() || patch.starts_with('/') || patch.split('/').any(|part| part == "..") {
            return None;
        }
        let folder = font.rfind('/').map_or("", |end| &font[..=end]);
        return Some(format!("{folder}{patch}"));
    };
    let resolved = base.join(patch).ok()?;
    if resolved.scheme() != base.scheme()
        || resolved.host_str() != base.host_str()
        || resolved.port_or_known_default() != base.port_or_known_default()
    {
        return None;
    }
    if !matches!(base.scheme(), "http" | "https") {
        let folder = &base.path()[..=base.path().rfind('/')?];
        if !resolved.path().starts_with(folder) {
            return None;
        }
    }
    Some(resolved.to_string())
}

fn maps_of(typeface: &GlyphTypeface) -> Result<Vec<PatchMap>, FontDecodeError> {
    let read = |tag: Tag| typeface.platform_typeface().try_get_table(OpenTypeTag::new(u32::from_be_bytes(tag)));
    let Some(first) = read(IFT_TABLE) else {
        return Ok(Vec::new());
    };
    let second = read(IFTX_TABLE);
    patch_maps_from_tables(first.span(), second.as_ref().map(|table| table.span()))
}

impl Shared {
    /// Sets up the extension of a font that was just decoded. False when it cannot be used: it
    /// needs a table-keyed patch, or nothing here can see the text set in it.
    pub(super) fn begin_incremental(&self, id: FontFaceId, font: &[u8]) -> bool {
        let Some(Some(url)) = self.with_slot(id, |slot| slot.incremental_source.clone()) else {
            return true;
        };
        match read_patch_maps(font, &self.options.limits) {
            Ok(maps) if maps.is_empty() => return true,
            Ok(_) => {}
            Err(_) => return false,
        }
        if FerroLocator::current().get_service::<dyn ITextShaperImpl>().is_none() {
            return false;
        }
        let generation = self.next_tick();
        let design_space = read_font_axes(font).map_or(Vec::new(), |axes| {
            let fixed = |value: f32| (f64::from(value) * 65536.0) as i32;
            axes.axes.iter().map(|axis| (axis.tag, fixed(axis.min), fixed(axis.max))).collect()
        });
        self.with_slot(id, |slot| {
            let mut features = DEFAULT_FEATURES.to_vec();
            for setting in slot.descriptors.feature_settings.iter().filter(|setting| setting.value != 0) {
                if !features.contains(&setting.tag) {
                    features.push(setting.tag);
                }
            }
            slot.incremental = Some(Box::new(IncrementalState {
                url,
                generation,
                target: HashSet::new(),
                features,
                design_space,
                requested: HashSet::new(),
                in_flight: 0,
                failed: false,
                received: Vec::new(),
                applying: false,
                dirty: false,
            }));
        });
        true
    }

    /// Lets the shaper report the text of incremental faces.
    pub(super) fn connect_shaper(this: &Rc<Self>) {
        let weak = Rc::downgrade(this);
        if let Some(shaper) = this.feature_shaper.borrow().as_ref() {
            shaper.set_observer(Rc::new(move |id, text, features| {
                if let Some(this) = weak.upgrade() {
                    Shared::note_text(&this, id, text, features);
                }
            }));
        }
    }

    /// Adds what a text needs to the target of a face.
    pub(super) fn note_text(this: &Rc<Self>, id: FontFaceId, text: &[u16], features: Option<&[FontFeature]>) {
        let changed = this.with_slot(id, |slot| {
            let Some(state) = slot.incremental.as_mut() else {
                return false;
            };
            let mut changed = false;
            for character in char::decode_utf16(text.iter().copied()).flatten() {
                changed |= state.target.insert(character as u32);
            }
            for tag in features.into_iter().flatten().filter(|feature| feature.value != 0) {
                if let Some(tag) = tag_of(&tag.tag).filter(|tag| !state.features.contains(tag)) {
                    state.features.push(tag);
                    changed = true;
                }
            }
            changed
        });
        if changed == Some(true) {
            Self::schedule_extension(this, id);
        }
    }

    pub(super) fn schedule_extension(this: &Rc<Self>, id: FontFaceId) {
        {
            let mut pending = this.pending_extensions.borrow_mut();
            if !pending.contains(&id) {
                pending.push(id);
            }
        }
        Self::schedule_work(this);
    }

    pub(super) fn run_extensions(this: &Rc<Self>) {
        let pending = std::mem::take(&mut *this.pending_extensions.borrow_mut());
        for id in pending {
            Self::extend(this, id);
        }
    }

    /// Asks for the patches the target needs, when none are on their way.
    fn extend(this: &Rc<Self>, id: FontFaceId) {
        let plan = this.with_slot(id, |slot| {
            let typeface = slot.glyph_typeface.clone()?;
            let state = slot.incremental.as_mut()?;
            if state.in_flight > 0 || state.applying {
                state.dirty = true;
                return None;
            }
            state.dirty = false;
            let target = SubsetTarget::new(
                state.target.iter().copied(),
                state.features.iter().copied(),
                state.design_space.clone(),
            );
            Some((typeface, target, state.requested.clone(), state.url.clone(), state.generation))
        });
        let Some(Some((typeface, target, requested, font_url, generation))) = plan else {
            return;
        };
        let wanted = maps_of(&typeface).and_then(|maps| patches_to_load(&maps, &target, |url| requested.contains(url)));
        let Ok(mut urls) = wanted else {
            return Self::fail_extension(this, id);
        };
        if urls.is_empty() {
            return;
        }
        if requested.len() + urls.len() > MAX_PATCHES {
            return Self::fail_extension(this, id);
        }
        let more = urls.len() > MAX_PATCH_FETCHES;
        urls.truncate(MAX_PATCH_FETCHES);
        let resolved: Option<Vec<String>> = urls.iter().map(|url| patch_url(&font_url, url)).collect();
        let (Some(resolved), Some(fetcher)) = (resolved, this.fetcher.borrow().clone()) else {
            return Self::fail_extension(this, id);
        };
        this.with_slot(id, |slot| {
            if let Some(state) = slot.incremental.as_mut() {
                state.requested.extend(urls.iter().cloned());
                state.in_flight = urls.len();
                state.dirty = more;
            }
        });
        for (url, location) in urls.into_iter().zip(resolved) {
            let weak = Rc::downgrade(this);
            fetcher.fetch(
                &location,
                Box::new(move |result| {
                    if let Some(this) = weak.upgrade() {
                        Shared::patch_fetched(&this, id, generation, url, result);
                    }
                }),
            );
        }
    }

    fn patch_fetched(this: &Rc<Self>, id: FontFaceId, generation: u64, url: String, result: FontFetchResult) {
        let ready = this.with_slot(id, |slot| {
            let state = slot.incremental.as_mut().filter(|state| state.generation == generation)?;
            state.in_flight = state.in_flight.saturating_sub(1);
            match result {
                Ok(bytes) => state.received.push((url, bytes)),
                Err(_) => state.failed = true,
            }
            (state.in_flight == 0).then(|| std::mem::take(&mut state.failed))
        });
        match ready {
            Some(Some(true)) => Self::fail_extension(this, id),
            Some(Some(false)) => Self::apply_patches(this, id),
            _ => {}
        }
    }

    fn apply_patches(this: &Rc<Self>, id: FontFaceId) {
        let taken = this.with_slot(id, |slot| {
            let typeface = slot.glyph_typeface.clone()?;
            let state = slot.incremental.as_mut()?;
            state.applying = true;
            Some((typeface, std::mem::take(&mut state.received), state.generation))
        });
        let Some(Some((typeface, patches, generation))) = taken else {
            return;
        };
        let limits = this.options.limits;
        let job = (|| {
            // The font in use stays in use until the extended one is ready.
            let mut font = Vec::new();
            typeface.platform_typeface().try_get_stream()?.read_to_end(&mut font).ok()?;
            let maps = read_patch_maps(&font, &limits).ok()?;
            GlyphPatchJob::new(font, maps, patches, brotli_decoder_factory()?, limits).ok()
        })();
        let Some(job) = job else {
            return Self::fail_extension(this, id);
        };
        #[cfg(any(not(target_family = "wasm"), target_feature = "atomics"))]
        if this.use_dispatcher {
            let options = this.options_for(id);
            let weak = Rc::downgrade(this);
            let started = crate::off_thread::run_off_thread(
                move || {
                    let mut job = job;
                    let font = loop {
                        if let Some(font) = job.step(usize::MAX)? {
                            break font;
                        }
                    };
                    decode_font_with_brotli(&font, &options, brotli_decoder_factory())
                },
                move |result| {
                    if let Some(this) = weak.upgrade() {
                        this.finished.borrow_mut().push((id, result, generation));
                        Shared::schedule_work(&this);
                    }
                },
            );
            if started.is_err() {
                Self::fail_extension(this, id);
            }
            return;
        }
        this.patch_jobs.borrow_mut().push_back(PatchWork { id, generation, job });
        Self::schedule_work(this);
    }

    /// Runs queued patches until `duration` has passed. Each step is small.
    pub(super) fn step_patch_jobs(this: &Rc<Self>, started: Instant, duration: Duration) {
        loop {
            let work = this.patch_jobs.borrow_mut().pop_front();
            let Some(mut work) = work else {
                return;
            };
            match work.job.step(DECODE_STEP) {
                Ok(None) => this.patch_jobs.borrow_mut().push_front(work),
                Ok(Some(font)) => {
                    let options = this.options_for(work.id);
                    match FontDecoder::with_brotli(Arc::<[u8]>::from(font), options, brotli_decoder_factory()) {
                        Ok(decoder) => this.jobs.borrow_mut().push_back(DecodeJob {
                            id: work.id,
                            decoder,
                            generation: work.generation,
                        }),
                        Err(error) => this.finished.borrow_mut().push((work.id, Err(error), work.generation)),
                    }
                }
                Err(error) => this.finished.borrow_mut().push((work.id, Err(error), work.generation)),
            }
            if started.elapsed() >= duration {
                return;
            }
        }
    }

    /// Whether a result is the extension this face is waiting for. A result of a state that was
    /// replaced is `None`: it is dropped.
    pub(super) fn take_extension(&self, id: FontFaceId, generation: u64) -> Option<bool> {
        if generation == 0 {
            return Some(false);
        }
        self.with_slot(id, |slot| {
            let state = slot.incremental.as_mut().filter(|state| state.generation == generation)?;
            state.applying = false;
            Some(true)
        })
        .flatten()
    }

    /// Lets go of the typefaces made from the font before it was extended.
    pub(super) fn release_instances(&self, id: FontFaceId) {
        let shaper = self.feature_shaper.borrow().clone();
        self.with_slot(id, |slot| {
            if let Some(shaper) = &shaper {
                slot.instances.iter().for_each(|(_, instance)| shaper.remove(instance));
                slot.glyph_typeface.iter().for_each(|typeface| shaper.remove(typeface));
            }
            slot.instances.clear();
        });
    }

    /// The face can no longer be extended: what it shows would have holes, so it is given up for
    /// the next `src` entry, and without one the text falls back.
    pub(super) fn fail_extension(this: &Rc<Self>, id: FontFaceId) {
        this.release_instances(id);
        this.with_slot(id, |slot| {
            slot.incremental = None;
            slot.glyph_typeface = None;
            slot.usable = false;
            slot.size = 0;
        });
        this.fonts_changed();
        Self::try_next_source(this, id);
    }

    /// Whether a face still needs a patch for a text.
    pub(super) fn needs_patches(&self, id: FontFaceId, text: &str) -> bool {
        let found = self.with_slot(id, |slot| {
            let state = slot.incremental.as_ref()?;
            let target = SubsetTarget::new(
                text.chars().map(u32::from),
                state.features.iter().copied(),
                state.design_space.clone(),
            );
            Some((slot.glyph_typeface.clone()?, target))
        });
        let Some(Some((typeface, target))) = found else {
            return false;
        };
        let wanted = maps_of(&typeface).and_then(|maps| patches_to_load(&maps, &target, |_| false));
        wanted.map_or(true, |urls| !urls.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::patch_url;

    #[test]
    fn a_patch_stays_with_its_font() {
        let font = "https://fonts.example/a/font.ttf";
        assert_eq!(patch_url(font, "p/04").as_deref(), Some("https://fonts.example/a/p/04"));
        assert_eq!(patch_url(font, "/b/04").as_deref(), Some("https://fonts.example/b/04"));
        for other in [
            "https://other.example/p/04",
            "//other.example/p/04",
            "http://fonts.example/p/04",
            "https://fonts.example:8443/p/04",
            "file:///etc/passwd",
        ] {
            assert_eq!(patch_url(font, other), None, "{other}");
        }
    }

    #[test]
    fn a_patch_of_a_font_on_disk_stays_in_its_folder() {
        let font = "file:///fonts/a/font.ttf";
        assert_eq!(patch_url(font, "p/04").as_deref(), Some("file:///fonts/a/p/04"));
        assert_eq!(patch_url(font, "../secret"), None);
        assert_eq!(patch_url(font, "/etc/passwd"), None);
    }

    #[test]
    fn a_relative_font_takes_only_relative_patches() {
        assert_eq!(patch_url("fonts/a.ttf", "p/04").as_deref(), Some("fonts/p/04"));
        assert_eq!(patch_url("a.ttf", "p/04").as_deref(), Some("p/04"));
        for other in ["../x", "/x", "https://other.example/x", "file:///x"] {
            assert_eq!(patch_url("fonts/a.ttf", other), None, "{other}");
        }
    }
}
