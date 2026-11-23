//! The font collection of declared web fonts.

use crate::feature_shaper::{to_font_features, FontFeatureShaper};
use crate::text_invalidation::invalidate_text_layouts;
use crate::{brotli_decoder_factory, FontDataStream};
use ferroui_base::media::fonts::{IFontCollection, OpenTypeTag};
use ferroui_base::media::{
    FontFamily, FontManager, FontSimulations, FontStretch, FontStyle, FontVariation, FontVariationSettings, FontWeight,
    GlyphTypeface, Typeface,
};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::Locale;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_fonts_web_core::css::{
    parse_font_palettes, parse_stylesheet, FontFaceDescriptors, FontPaletteValues, FontSource, FontStyleDescriptor,
    FontTech, FontTechs,
};
#[cfg(any(not(target_family = "wasm"), target_feature = "atomics"))]
use ferroui_fonts_web_core::decode_font_with_brotli;
use ferroui_fonts_web_core::loading::{
    check_faces, find_matching_faces, FontDisplayPhase, FontDisplayTimeline, FontFaceId, FontFaceLoadStatus,
    FontFaceSetError, FontFaceSetEvent, FontFaceSetLoadStatus, FontFaceSetState, FontFaceView,
};
use ferroui_fonts_web_core::matching::{
    family_matches, match_font_faces, narrow_for_unknown_font_size, FontMatchRequest, FontStyleRequest,
};
use ferroui_fonts_web_core::sfnt::{
    apply_font_palette, apply_metric_overrides, has_metric_overrides, make_invisible_font, SanitizeOptions,
};
use ferroui_fonts_web_core::variations::{names_of, read_font_axes, resolve_font_variations, FontAxes};
use ferroui_fonts_web_core::{FontDecodeError, FontDecoder};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::io::Read;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Separates a family from the id of one of its faces in the name a fallback typeface carries. A
/// private use character, so that it cannot be part of a declared name.
const FACE_SEPARATOR: char = '\u{E000}';
/// The work between two looks at the clock while decoding on the UI thread.
const DECODE_STEP: usize = 2 * 1024;
/// How long one slice of decoding may run on the UI thread before it yields to the page. Well
/// under the 50 ms of a long task, with room for what a single step cannot divide.
const DECODE_SLICE: Duration = Duration::from_millis(4);
/// The decoded fonts kept by default, in bytes. Over it, the least recently used face that nothing
/// else holds is let go, and fetched again when it is next needed.
const DEFAULT_MEMORY_BUDGET: usize = 128 * 1024 * 1024;
/// The most fonts fetched at the same time.
const MAX_CONCURRENT_FETCHES: usize = 8;

/// Why a font could not be fetched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontFetchError {
    NotFound,
    /// Refused by policy: an origin, a scheme or a limit.
    Forbidden(String),
    Network(String),
}

/// What a fetch ends with.
pub type FontFetchResult = Result<Vec<u8>, FontFetchError>;

/// Fetches the bytes of a font without blocking the UI thread. `completion` runs on the UI thread.
pub trait IFontFaceFetcher {
    fn fetch(&self, url: &str, completion: Box<dyn FnOnce(FontFetchResult)>);
}

struct Slot {
    id: FontFaceId,
    descriptors: FontFaceDescriptors,
    status: FontFaceLoadStatus,
    timeline: FontDisplayTimeline,
    /// The next entry of `src` to try.
    next_source: usize,
    glyph_typeface: Option<Rc<GlyphTypeface>>,
    /// Whether the font arrived in time to replace the fallback.
    usable: bool,
    /// The variation space of the font, for a variable font.
    axes: Option<Rc<FontAxes>>,
    /// The instances of the font made for requests, by what they were made for.
    instances: Vec<(InstanceKey, Rc<GlyphTypeface>)>,
    /// The size of the decoded font.
    size: usize,
    /// When the face was last used, in the ticks of the collection.
    last_used: u64,
    /// Let go under the memory budget: its status stays `loaded`, and the next use loads it.
    evicted: bool,
}

/// What an instance of a font was made for.
#[derive(Clone, Copy, PartialEq, Eq)]
struct InstanceKey {
    weight: i32,
    width: i32,
    style: i32,
}

impl InstanceKey {
    fn new(request: &FontMatchRequest) -> Self {
        let style = match request.style {
            FontStyleRequest::Normal => 0,
            FontStyleRequest::Italic => 1,
            FontStyleRequest::Oblique(angle) => 2 + (angle * 16.0) as i32,
        };
        Self { weight: request.weight as i32, width: (request.width * 8.0) as i32, style }
    }
}

struct DecodeJob {
    id: FontFaceId,
    decoder: FontDecoder<Arc<[u8]>>,
}

struct Shared {
    platform: Rc<dyn IFontManagerImpl>,
    fetcher: RefCell<Option<Rc<dyn IFontFaceFetcher>>>,
    /// What relative URLs of `src` are resolved against.
    base_uri: RefCell<Option<url::Url>>,
    active_fetches: Cell<usize>,
    /// The faces waiting for a fetch to finish, in the order they asked.
    waiting_fetches: RefCell<VecDeque<FontFaceId>>,
    faces: RefCell<Vec<Slot>>,
    set: RefCell<FontFaceSetState>,
    next_id: Cell<u32>,
    jobs: RefCell<VecDeque<DecodeJob>>,
    finished: RefCell<Vec<(FontFaceId, Result<Vec<u8>, FontDecodeError>)>>,
    /// Whether a slice of work is already scheduled on the dispatcher.
    scheduled: Cell<bool>,
    /// Off for tests, which run the work with [`WebFontCollection::run_pending_work`].
    use_dispatcher: bool,
    clock: Box<dyn Fn() -> u64>,
    options: SanitizeOptions,
    invisible: RefCell<Option<Option<Rc<GlyphTypeface>>>>,
    memory_budget: Cell<usize>,
    supported_techs: Cell<FontTechs>,
    /// The `@font-palette-values` rules declared.
    palettes: RefCell<Vec<FontPaletteValues>>,
    /// The palette chosen for a family, by the name of its rule.
    chosen_palettes: RefCell<Vec<(String, String)>>,
    use_tick: Cell<u64>,
    /// The wrapper of the shaper, once a face has declared features.
    feature_shaper: RefCell<Option<Rc<FontFeatureShaper>>>,
    listeners: RefCell<Vec<Rc<dyn Fn(&FontFaceSetEvent)>>>,
}

/// Maps the nine stretches to the percentages of `font-width`.
fn stretch_percent(stretch: FontStretch) -> f32 {
    const PERCENTS: [f32; 9] = [50.0, 62.5, 75.0, 87.5, 100.0, 112.5, 125.0, 150.0, 200.0];
    PERCENTS[(stretch as usize).clamp(1, 9) - 1]
}

fn match_request(style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontMatchRequest {
    let style = match style {
        FontStyle::Normal => FontStyleRequest::Normal,
        FontStyle::Italic => FontStyleRequest::Italic,
        FontStyle::Oblique => FontStyleRequest::Oblique(14.0),
    };
    FontMatchRequest { weight: weight.0.clamp(1, 1000) as f32, width: stretch_percent(stretch), style }
}

/// Whether a font has this full name (name id 4) or PostScript name (name id 6).
fn has_local_name(typeface: &GlyphTypeface, name: &str) -> bool {
    let Some(table) = typeface.platform_typeface().try_get_table(OpenTypeTag::new(u32::from_be_bytes(*b"name"))) else {
        return false;
    };
    [4, 6].iter().any(|id| names_of(table.span(), *id).iter().any(|known| known.eq_ignore_ascii_case(name)))
}

/// The simulations a request needs from a face, as `font-synthesis: auto` allows them: a bold
/// for a weight from 600 on a lighter face, a slant for a face that is upright, and neither
/// where an axis of the font does the work.
fn simulations_for(
    base: &GlyphTypeface,
    face: &FontFaceDescriptors,
    axes: Option<&FontAxes>,
    request: &FontMatchRequest,
) -> FontSimulations {
    let has = |tag: &[u8; 4]| axes.is_some_and(|axes| axes.has(tag));
    let mut simulations = FontSimulations::None;
    let face_weight = face.weight.map_or(base.weight().0 as f32, |range| range.end);
    if request.weight >= 600.0 && face_weight < 600.0 && !has(b"wght") {
        simulations |= FontSimulations::Bold;
    }
    let upright = matches!(face.style, FontStyleDescriptor::Auto | FontStyleDescriptor::Normal)
        && base.style() == FontStyle::Normal;
    if request.style != FontStyleRequest::Normal && upright && !has(b"ital") && !has(b"slnt") {
        simulations |= FontSimulations::Oblique;
    }
    simulations
}

/// What `tech()` answers by default. Palettes are applied to the font itself, so every backend has
/// them. The color formats are not claimed: which of them a backend draws depends on the backend
/// and the version of the system, and an entry claimed wrongly would replace a plain font that
/// draws with one that does not. An application that knows its targets says so with
/// [`WebFontCollection::set_supported_techs`]. A font without `tech()` is loaded either way.
const DEFAULT_TECHS: FontTechs =
    FontTechs::NONE.with(FontTech::Variations).with(FontTech::FeaturesOpenType).with(FontTech::Palettes);

impl Shared {
    fn with_slot<R>(&self, id: FontFaceId, action: impl FnOnce(&mut Slot) -> R) -> Option<R> {
        self.faces.borrow_mut().iter_mut().find(|slot| slot.id == id).map(action)
    }

    fn set_status(&self, id: FontFaceId, status: FontFaceLoadStatus) {
        let changed = self.with_slot(id, |slot| {
            let can = slot.status.can_become(status);
            if can {
                slot.status = status;
            }
            can
        });
        if changed == Some(true) {
            self.set.borrow_mut().face_status_changed(id, status);
            self.deliver_events();
        }
    }

    fn deliver_events(&self) {
        let events = self.set.borrow_mut().take_events();
        if events.is_empty() {
            return;
        }
        let listeners: Vec<_> = self.listeners.borrow().clone();
        for event in &events {
            for listener in &listeners {
                listener(event);
            }
        }
    }

    /// Starts loading a face, once. Its timeline starts at its first use.
    fn request_load(this: &Rc<Self>, id: FontFaceId) {
        let now = (this.clock)();
        let state = this.with_slot(id, |slot| {
            slot.timeline.start(now);
            let evicted = std::mem::take(&mut slot.evicted);
            if evicted {
                slot.next_source = 0;
            }
            (slot.status == FontFaceLoadStatus::Unloaded, evicted)
        });
        if state == Some((false, true)) {
            Self::try_next_source(this, id);
        }
        if !matches!(state, Some((true, _))) {
            return;
        }
        this.set_status(id, FontFaceLoadStatus::Loading);
        this.schedule_display_timer(id, now);
        Self::try_next_source(this, id);
    }

    /// Wakes up when the phase of the face changes, for what is on the screen to follow it.
    fn schedule_display_timer(self: &Rc<Self>, id: FontFaceId, now: u64) {
        if !self.use_dispatcher {
            return;
        }
        let Some(Some(at)) = self.with_slot(id, |slot| slot.timeline.next_transition(now)) else {
            return;
        };
        let weak = Rc::downgrade(self);
        DispatcherTimer::run_once(
            move || {
                if let Some(this) = weak.upgrade() {
                    let now = (this.clock)();
                    let loading = this.with_slot(id, |slot| slot.status == FontFaceLoadStatus::Loading);
                    if loading == Some(true) {
                        this.schedule_display_timer(id, now);
                        this.fonts_changed();
                    }
                }
            },
            Duration::from_millis(at.saturating_sub(now)),
            DispatcherPriority::BACKGROUND,
        )
        .detach();
    }

    /// Goes on to the next entry of `src` this build can use, or fails the face.
    fn try_next_source(this: &Rc<Self>, id: FontFaceId) {
        loop {
            let source = this.with_slot(id, |slot| {
                let source = slot.descriptors.sources.get(slot.next_source).cloned();
                slot.next_source += 1;
                source
            });
            let Some(Some(source)) = source else {
                this.set_status(id, FontFaceLoadStatus::Error);
                this.fonts_changed();
                return;
            };
            if !source.is_supported(this.supported_techs.get()) {
                continue;
            }
            let url = match source {
                FontSource::Url { url, .. } => url,
                FontSource::Local { name } => {
                    #[cfg(all(feature = "browser", target_family = "wasm"))]
                    if crate::browser_fetcher::has_local_font(&name) {
                        let weak = Rc::downgrade(this);
                        let limit = this.options.limits.max_encoded_size;
                        crate::browser_fetcher::read_local_font(
                            &name,
                            limit,
                            Box::new(move |result| {
                                if let Some(this) = weak.upgrade() {
                                    Shared::fetched(&this, id, result);
                                }
                            }),
                        );
                        return;
                    }
                    if Self::install_local(this, id, &name) {
                        return;
                    }
                    continue;
                }
            };
            let Some(fetcher) = this.fetcher.borrow().clone() else {
                continue;
            };
            if this.active_fetches.get() >= MAX_CONCURRENT_FETCHES {
                this.with_slot(id, |slot| slot.next_source -= 1);
                this.waiting_fetches.borrow_mut().push_back(id);
                return;
            }
            this.active_fetches.set(this.active_fetches.get() + 1);
            let url = this.resolve(&url);
            let weak = Rc::downgrade(this);
            fetcher.fetch(
                &url,
                Box::new(move |result| {
                    if let Some(this) = weak.upgrade() {
                        this.active_fetches.set(this.active_fetches.get().saturating_sub(1));
                        Shared::fetched(&this, id, result);
                        let next = this.waiting_fetches.borrow_mut().pop_front();
                        if let Some(next) = next {
                            Shared::try_next_source(&this, next);
                        }
                    }
                }),
            );
            return;
        }
    }

    /// Uses an installed font for `local()`. False when there is none of that name.
    fn install_local(this: &Rc<Self>, id: FontFaceId, name: &str) -> bool {
        let Some(mut typeface) = this.find_local(name) else {
            return false;
        };
        let descriptors = this.with_slot(id, |slot| slot.descriptors.clone());
        let mut axes = None;
        let mut size = 0;
        // The usual reason to name a local font: overriding its metrics to match a web font. That
        // takes a private copy of the font, and only then.
        let palette = this.palette_of(id);
        if let Some(descriptors) = descriptors.filter(|face| has_metric_overrides(face) || palette.is_some()) {
            let mut bytes = Vec::new();
            let read = typeface.platform_typeface().try_get_stream().map(|mut stream| stream.read_to_end(&mut bytes));
            if matches!(read, Some(Ok(_))) && apply_metric_overrides(&mut bytes, &descriptors).is_ok() {
                if let Some(palette) = &palette {
                    apply_font_palette(&mut bytes, palette);
                }
                axes = read_font_axes(&bytes).map(Rc::new);
                size = bytes.len();
                let mut stream = FontDataStream::new(bytes);
                let adjusted = this
                    .platform
                    .try_create_glyph_typeface_from_stream(&mut stream, FontSimulations::None)
                    .and_then(|typeface| GlyphTypeface::try_create(typeface, FontSimulations::None));
                if let Some(adjusted) = adjusted {
                    typeface = adjusted;
                }
            }
        }
        let tick = this.next_tick();
        this.with_slot(id, |slot| {
            slot.usable = true;
            slot.glyph_typeface = Some(typeface.clone());
            slot.axes = axes;
            slot.size = size;
            slot.last_used = tick;
        });
        this.register_features(id, &typeface);
        this.set_status(id, FontFaceLoadStatus::Loaded);
        this.fonts_changed();
        true
    }

    /// The installed font whose full name or PostScript name is `name`. The platform finds fonts
    /// by family, so the family is looked for among the leading words of the name, and the name
    /// of each of its faces is then compared: no other font is ever substituted.
    fn find_local(&self, name: &str) -> Option<Rc<GlyphTypeface>> {
        let words: Vec<&str> = name.split_whitespace().collect();
        let mut families: Vec<String> = (1..=words.len()).rev().map(|count| words[..count].join(" ")).collect();
        if let Some((family, _)) = name.split_once('-') {
            families.push(family.to_owned());
        }
        for family in families {
            let Some(typefaces) = self.platform.try_get_family_typefaces(&family) else {
                continue;
            };
            for candidate in typefaces {
                let found = self
                    .platform
                    .try_create_glyph_typeface(&family, candidate.style(), candidate.weight(), candidate.stretch())
                    .and_then(|typeface| GlyphTypeface::try_create(typeface, FontSimulations::None));
                if let Some(found) = found.filter(|found| has_local_name(found, name)) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// A URL of `src`, against the base URI when it is relative.
    fn resolve(&self, url: &str) -> String {
        if url::Url::parse(url).is_ok() {
            return url.to_owned();
        }
        self.base_uri
            .borrow()
            .as_ref()
            .and_then(|base| base.join(url).ok())
            .map_or_else(|| url.to_owned(), |resolved| resolved.to_string())
    }

    fn fetched(this: &Rc<Self>, id: FontFaceId, result: FontFetchResult) {
        let Ok(bytes) = result else {
            Self::try_next_source(this, id);
            return;
        };
        let bytes: Arc<[u8]> = bytes.into();
        #[cfg(any(not(target_family = "wasm"), target_feature = "atomics"))]
        if this.use_dispatcher && Self::decode_off_thread(this, id, bytes.clone()) {
            return;
        }
        match FontDecoder::with_brotli(bytes, this.options, brotli_decoder_factory()) {
            Ok(decoder) => {
                this.jobs.borrow_mut().push_back(DecodeJob { id, decoder });
                Self::schedule_work(this);
            }
            Err(_) => Self::try_next_source(this, id),
        }
    }

    /// Decodes on a worker thread. False when no thread could be started: the decode then runs
    /// in slices on the UI thread.
    #[cfg(any(not(target_family = "wasm"), target_feature = "atomics"))]
    fn decode_off_thread(this: &Rc<Self>, id: FontFaceId, bytes: Arc<[u8]>) -> bool {
        let options = this.options;
        let weak = Rc::downgrade(this);
        crate::off_thread::run_off_thread(
            move || decode_font_with_brotli(&bytes, &options, brotli_decoder_factory()),
            move |result| {
                if let Some(this) = weak.upgrade() {
                    this.finished.borrow_mut().push((id, result));
                    // Through the queue, so that fonts that finish together swap together.
                    Shared::schedule_work(&this);
                }
            },
        )
        .is_ok()
    }

    fn schedule_work(this: &Rc<Self>) {
        if !this.use_dispatcher || this.scheduled.replace(true) {
            return;
        }
        let weak = Rc::downgrade(this);
        DispatcherTimer::run_once(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.scheduled.set(false);
                    if Shared::run_slice(&this) {
                        Shared::schedule_work(&this);
                    }
                }
            },
            Duration::ZERO,
            DispatcherPriority::BACKGROUND,
        )
        .detach();
    }

    /// One slice of decoding, then the fonts that finished. Returns whether work remains.
    fn run_slice(this: &Rc<Self>) -> bool {
        Self::run_slice_for(this, DECODE_SLICE)
    }

    /// Decodes in small steps until `duration` has passed, at least one step.
    fn run_slice_for(this: &Rc<Self>, duration: Duration) -> bool {
        let started = Instant::now();
        loop {
            let job = this.jobs.borrow_mut().pop_front();
            let Some(mut job) = job else {
                break;
            };
            match job.decoder.step(DECODE_STEP) {
                Ok(None) => this.jobs.borrow_mut().push_front(job),
                Ok(Some(font)) => this.finished.borrow_mut().push((job.id, Ok(font))),
                Err(error) => this.finished.borrow_mut().push((job.id, Err(error))),
            }
            if started.elapsed() >= duration {
                break;
            }
        }
        // Fonts that finish together are installed together, for one layout pass. A font is
        // held back while another is one slice from done only by the order of the queue.
        if this.jobs.borrow().is_empty() {
            Self::install_finished(this);
        }
        !this.jobs.borrow().is_empty()
    }

    fn install_finished(this: &Rc<Self>) {
        let finished = std::mem::take(&mut *this.finished.borrow_mut());
        if finished.is_empty() {
            return;
        }
        let now = (this.clock)();
        let mut changed = false;
        for (id, result) in finished {
            let mut size = 0;
            let mut axes = None;
            let typeface = result.ok().and_then(|mut font| {
                // A font that cannot take the overrides is used as it is.
                let _ = this.with_slot(id, |slot| apply_metric_overrides(&mut font, &slot.descriptors));
                if let Some(palette) = this.palette_of(id) {
                    apply_font_palette(&mut font, &palette);
                }
                size = font.len();
                axes = read_font_axes(&font).map(Rc::new);
                let mut stream = FontDataStream::new(font);
                let typeface =
                    this.platform.try_create_glyph_typeface_from_stream(&mut stream, FontSimulations::None)?;
                GlyphTypeface::try_create(typeface, FontSimulations::None)
            });
            let Some(typeface) = typeface else {
                Self::try_next_source(this, id);
                continue;
            };
            let tick = this.next_tick();
            this.with_slot(id, |slot| {
                slot.usable = slot.usable || slot.timeline.accepts_load(now);
                changed |= slot.usable;
                slot.glyph_typeface = Some(typeface.clone());
                slot.axes = axes;
                slot.size = size;
                slot.last_used = tick;
            });
            this.register_features(id, &typeface);
            this.set_status(id, FontFaceLoadStatus::Loaded);
        }
        if changed {
            this.fonts_changed();
        }
        this.enforce_memory_budget();
    }

    /// The palette rule chosen for the family of a face, if it applies to that family.
    fn palette_of(&self, id: FontFaceId) -> Option<FontPaletteValues> {
        let family = self.with_slot(id, |slot| slot.descriptors.family.clone())?;
        let chosen = self.chosen_palettes.borrow();
        let (_, name) = chosen.iter().find(|(known, _)| known.eq_ignore_ascii_case(&family))?;
        let palettes = self.palettes.borrow();
        let rule = palettes.iter().find(|rule| rule.name == *name)?;
        rule.families.iter().any(|known| known.eq_ignore_ascii_case(&family)).then(|| rule.clone())
    }

    fn next_tick(&self) -> u64 {
        let tick = self.use_tick.get() + 1;
        self.use_tick.set(tick);
        tick
    }

    /// Gives a typeface of a face the features the face declares.
    fn register_features(&self, id: FontFaceId, typeface: &Rc<GlyphTypeface>) {
        let features = self.with_slot(id, |slot| {
            let settings = &slot.descriptors.feature_settings;
            (!settings.is_empty()).then(|| to_font_features(settings))
        });
        let Some(Some(features)) = features else {
            return;
        };
        if self.feature_shaper.borrow().is_none() {
            *self.feature_shaper.borrow_mut() = FontFeatureShaper::install();
        }
        if let Some(shaper) = self.feature_shaper.borrow().as_ref() {
            shaper.set_features(typeface, features);
        }
    }

    /// Lets go of the least recently used faces nothing else holds, until the fonts fit.
    fn enforce_memory_budget(&self) {
        let budget = self.memory_budget.get();
        let shaper = self.feature_shaper.borrow();
        // The references the collection holds itself: the slot or its list of instances, and the
        // shaper's table for a face that declares features.
        let table = |slot: &Slot| (shaper.is_some() && !slot.descriptors.feature_settings.is_empty()) as usize;
        let is_free = |slot: &Slot| {
            let Some(typeface) = &slot.glyph_typeface else {
                return false;
            };
            let listed = slot.instances.iter().filter(|(_, instance)| Rc::ptr_eq(instance, typeface)).count();
            Rc::strong_count(typeface) <= 1 + listed + table(slot)
                && slot
                    .instances
                    .iter()
                    .filter(|(_, instance)| !Rc::ptr_eq(instance, typeface))
                    .all(|(_, instance)| Rc::strong_count(instance) <= 1 + table(slot))
        };
        let mut faces = self.faces.borrow_mut();
        loop {
            let total: usize = faces.iter().filter(|slot| slot.glyph_typeface.is_some()).map(|slot| slot.size).sum();
            if total <= budget {
                return;
            }
            let Some(slot) = faces.iter_mut().filter(|slot| is_free(slot)).min_by_key(|slot| slot.last_used) else {
                return;
            };
            if let Some(shaper) = shaper.as_ref() {
                for (_, instance) in &slot.instances {
                    shaper.remove(instance);
                }
                if let Some(typeface) = &slot.glyph_typeface {
                    shaper.remove(typeface);
                }
            }
            slot.instances.clear();
            slot.glyph_typeface = None;
            slot.evicted = true;
        }
    }

    /// The typeface of a face for a request: the instance of a variable font, or a simulated
    /// bold or oblique for a face that does not have them.
    fn instance(&self, id: FontFaceId, request: &FontMatchRequest) -> Option<Rc<GlyphTypeface>> {
        let base = self.usable_typeface(id)?;
        let tick = self.next_tick();
        let key = InstanceKey::new(request);
        let (axes, descriptors) = self.with_slot(id, |slot| {
            slot.last_used = tick;
            (slot.axes.clone(), slot.descriptors.clone())
        })?;
        let known = self.with_slot(id, |slot| {
            slot.instances.iter().find(|(known, _)| *known == key).map(|(_, instance)| instance.clone())
        })?;
        if let Some(found) = known {
            return Some(found);
        }
        let mut made = None;
        if let Some(axes) = &axes {
            let settings = resolve_font_variations(axes, &descriptors, request);
            let differs = settings
                .iter()
                .any(|setting| axes.axes.iter().any(|axis| axis.tag == setting.tag && axis.default != setting.value));
            if differs {
                let variations = settings.iter().map(|setting| FontVariation {
                    tag: OpenTypeTag::new(u32::from_be_bytes(setting.tag)),
                    value: setting.value as f64,
                });
                let settings = FontVariationSettings::new(variations);
                made = base
                    .platform_typeface()
                    .with_variation_settings(&settings)
                    .and_then(|typeface| GlyphTypeface::try_create(typeface, FontSimulations::None));
            }
        }
        if made.is_none() {
            let simulations = simulations_for(&base, &descriptors, axes.as_deref(), request);
            if simulations != FontSimulations::None {
                made = base.platform_typeface().try_get_stream().and_then(|mut stream| {
                    let typeface = self.platform.try_create_glyph_typeface_from_stream(&mut *stream, simulations)?;
                    GlyphTypeface::try_create(typeface, simulations)
                });
            }
        }
        let instance = made.unwrap_or(base);
        self.register_features(id, &instance);
        self.with_slot(id, |slot| slot.instances.push((key, instance.clone())));
        Some(instance)
    }

    /// What is on the screen has to be set again: one generation of lookups, one layout pass.
    fn fonts_changed(&self) {
        if let Some(manager) = FerroLocator::current().get_service::<FontManager>() {
            manager.invalidate_glyph_typefaces();
        }
        invalidate_text_layouts();
    }

    /// The face that stands in, invisibly, during a block period: the default font of the
    /// platform without its outlines. `None` where it cannot be made: text then shows in the
    /// fallback font at once.
    fn invisible_typeface(&self) -> Option<Rc<GlyphTypeface>> {
        if let Some(found) = self.invisible.borrow().as_ref() {
            return found.clone();
        }
        let make = || {
            let name = self.platform.get_default_font_family_name();
            let default = self.platform.try_create_glyph_typeface(
                &name,
                FontStyle::Normal,
                FontWeight::Normal,
                FontStretch::Normal,
            )?;
            let mut bytes = Vec::new();
            default.try_get_stream()?.read_to_end(&mut bytes).ok()?;
            let font = make_invisible_font(&bytes, &self.options.limits).ok()?;
            let mut stream = FontDataStream::new(font);
            let typeface = self.platform.try_create_glyph_typeface_from_stream(&mut stream, FontSimulations::None)?;
            GlyphTypeface::try_create(typeface, FontSimulations::None)
        };
        let made = make();
        *self.invisible.borrow_mut() = Some(made.clone());
        made
    }

    /// The composite face a request selects, as ids, the last rule first.
    fn composite(&self, family: &str, request: &FontMatchRequest) -> Vec<FontFaceId> {
        let faces = self.faces.borrow();
        let family_faces: Vec<&Slot> = faces.iter().filter(|slot| family_matches(&slot.descriptors, family)).collect();
        let descriptors: Vec<&FontFaceDescriptors> = family_faces.iter().map(|slot| &slot.descriptors).collect();
        let composite = match_font_faces(&descriptors, request);
        narrow_for_unknown_font_size(&descriptors, &composite).into_iter().map(|index| family_faces[index].id).collect()
    }

    fn covers(&self, id: FontFaceId, code_point: u32) -> bool {
        self.with_slot(id, |slot| slot.descriptors.unicode_range.contains(code_point)) == Some(true)
    }

    fn usable_typeface(&self, id: FontFaceId) -> Option<Rc<GlyphTypeface>> {
        self.with_slot(id, |slot| if slot.usable { slot.glyph_typeface.clone() } else { None }).flatten()
    }

    fn views<R>(&self, action: impl FnOnce(&[FontFaceView<'_>]) -> R) -> R {
        let faces = self.faces.borrow();
        let views: Vec<FontFaceView<'_>> = faces
            .iter()
            .map(|slot| FontFaceView { id: slot.id, descriptors: &slot.descriptors, status: slot.status })
            .collect();
        action(&views)
    }
}

/// A font collection of web fonts, declared as with `@font-face` and loaded on demand. It plays
/// the role of `document.fonts`: its methods are those of `FontFaceSet`.
///
/// Register it with `FontManager::add_font_collection`, and name a family as
/// `fonts:WebFonts#Family`.
pub struct WebFontCollection {
    key: url::Url,
    shared: Rc<Shared>,
}

ferroui_base::ferro_thread_local! {
    /// The collection of the application, once something has asked for it.
    static CURRENT: RefCell<Option<Rc<WebFontCollection>>> = const { RefCell::new(None) };
}

impl WebFontCollection {
    /// The collection of the application, registered with the font manager on first use.
    /// Panics when no font backend is registered.
    pub fn current() -> Rc<Self> {
        if let Some(current) = CURRENT.with(|current| current.borrow().clone()) {
            return current;
        }
        let collection = Rc::new(Self::new(None));
        FontManager::current().add_font_collection(collection.clone());
        CURRENT.with(|current| *current.borrow_mut() = Some(collection.clone()));
        collection
    }

    /// Starts declaring a face from code: see [`FontFaceBuilder`](crate::FontFaceBuilder).
    pub fn declare(&self, family: &str) -> crate::FontFaceBuilder<'_> {
        crate::FontFaceBuilder::new(self, family)
    }

    /// The key of the collection, unless another is given.
    pub const DEFAULT_KEY: &'static str = "fonts:WebFonts";

    /// A collection over the active font backend. Without a fetcher it uses
    /// [`DefaultFontFaceFetcher`](crate::DefaultFontFaceFetcher).
    pub fn new(fetcher: Option<Rc<dyn IFontFaceFetcher>>) -> Self {
        let fetcher = fetcher.or_else(|| Some(Rc::new(crate::DefaultFontFaceFetcher::default()) as _));
        let platform = FontManager::current().platform_impl().clone();
        let key = url::Url::parse(Self::DEFAULT_KEY).expect("the default key is a URL");
        let started = Instant::now();
        let clock = Box::new(move || started.elapsed().as_millis() as u64);
        Self::create(key, platform, fetcher, clock, true)
    }

    /// A collection with its backend, clock and scheduling given, for tests.
    #[doc(hidden)]
    pub fn with_parts(
        platform: Rc<dyn IFontManagerImpl>,
        fetcher: Option<Rc<dyn IFontFaceFetcher>>,
        clock: Box<dyn Fn() -> u64>,
    ) -> Self {
        let key = url::Url::parse(Self::DEFAULT_KEY).expect("the default key is a URL");
        Self::create(key, platform, fetcher, clock, false)
    }

    fn create(
        key: url::Url,
        platform: Rc<dyn IFontManagerImpl>,
        fetcher: Option<Rc<dyn IFontFaceFetcher>>,
        clock: Box<dyn Fn() -> u64>,
        use_dispatcher: bool,
    ) -> Self {
        let shared = Rc::new(Shared {
            platform,
            fetcher: RefCell::new(fetcher),
            base_uri: RefCell::new(None),
            active_fetches: Cell::new(0),
            waiting_fetches: RefCell::new(VecDeque::new()),
            faces: RefCell::new(Vec::new()),
            set: RefCell::new(FontFaceSetState::new(false)),
            next_id: Cell::new(1),
            jobs: RefCell::new(VecDeque::new()),
            finished: RefCell::new(Vec::new()),
            scheduled: Cell::new(false),
            use_dispatcher,
            clock,
            options: SanitizeOptions::default(),
            invisible: RefCell::new(None),
            memory_budget: Cell::new(DEFAULT_MEMORY_BUDGET),
            supported_techs: Cell::new(DEFAULT_TECHS),
            palettes: RefCell::new(Vec::new()),
            chosen_palettes: RefCell::new(Vec::new()),
            use_tick: Cell::new(0),
            feature_shaper: RefCell::new(None),
            listeners: RefCell::new(Vec::new()),
        });
        Self { key, shared }
    }

    /// Replaces what fetches fonts.
    pub fn set_fetcher(&self, fetcher: Rc<dyn IFontFaceFetcher>) {
        *self.shared.fetcher.borrow_mut() = Some(fetcher);
    }

    /// How many bytes of decoded fonts to keep. Over it, the least recently used face that nothing
    /// else holds is let go, and fetched again when it is next needed.
    pub fn set_memory_budget(&self, bytes: usize) {
        self.shared.memory_budget.set(bytes);
        self.shared.enforce_memory_budget();
    }

    /// What relative URLs of `src` are resolved against.
    pub fn set_base_uri(&self, base_uri: url::Url) {
        *self.shared.base_uri.borrow_mut() = Some(base_uri);
    }

    /// Declares a face. Nothing is fetched until text needs it.
    pub fn add(&self, descriptors: FontFaceDescriptors) -> Result<FontFaceId, FontFaceSetError> {
        let id = FontFaceId(self.shared.next_id.get());
        self.shared.next_id.set(id.0 + 1);
        let timeline = FontDisplayTimeline::new(descriptors.display);
        self.shared.set.borrow_mut().add(id, FontFaceLoadStatus::Unloaded, false)?;
        self.shared.faces.borrow_mut().push(Slot {
            id,
            descriptors,
            status: FontFaceLoadStatus::Unloaded,
            timeline,
            next_source: 0,
            glyph_typeface: None,
            usable: false,
            axes: None,
            instances: Vec::new(),
            size: 0,
            last_used: 0,
            evicted: false,
        });
        self.shared.fonts_changed();
        Ok(id)
    }

    /// Asks for the installed fonts through the Local Font Access API, for `local()` in the
    /// browser. The browser shows a permission prompt, so this has to come from a user gesture,
    /// and nothing asks for it otherwise. `done` says whether access was given.
    #[cfg(all(feature = "browser", target_family = "wasm"))]
    pub fn request_local_font_access(&self, done: impl FnOnce(bool) + 'static) {
        crate::browser_fetcher::request_local_fonts(Box::new(done));
    }

    /// Says which technologies of `tech()` count as supported, for an application that knows
    /// which color formats its backend and systems draw.
    pub fn set_supported_techs(&self, techs: FontTechs) {
        self.shared.supported_techs.set(techs);
    }

    /// `font-palette` for a family: the name of an `@font-palette-values` rule, or `None` for the
    /// palette of the font. The palette is written into the font, so the faces of the family are
    /// loaded again. Returns whether a rule of that name applies to the family.
    pub fn set_font_palette(&self, family: &str, palette: Option<&str>) -> bool {
        let applies =
            palette.is_none_or(|name| {
                self.shared.palettes.borrow().iter().any(|rule| {
                    rule.name == name && rule.families.iter().any(|known| known.eq_ignore_ascii_case(family))
                })
            });
        if !applies {
            return false;
        }
        {
            let mut chosen = self.shared.chosen_palettes.borrow_mut();
            chosen.retain(|(known, _)| !known.eq_ignore_ascii_case(family));
            if let Some(name) = palette {
                chosen.push((family.to_owned(), name.to_owned()));
            }
        }
        let shaper = self.shared.feature_shaper.borrow().clone();
        for slot in self.shared.faces.borrow_mut().iter_mut() {
            if !family_matches(&slot.descriptors, family) || slot.glyph_typeface.is_none() {
                continue;
            }
            if let Some(shaper) = &shaper {
                slot.instances.iter().for_each(|(_, instance)| shaper.remove(instance));
                slot.glyph_typeface.iter().for_each(|typeface| shaper.remove(typeface));
            }
            slot.instances.clear();
            slot.glyph_typeface = None;
            slot.evicted = true;
        }
        self.shared.fonts_changed();
        true
    }

    /// Declares the `@font-face` and `@font-palette-values` rules of a style sheet, in order.
    /// Returns the ids of the faces.
    pub fn add_style_sheet(&self, css: &str) -> Vec<FontFaceId> {
        for rule in parse_font_palettes(css) {
            let mut palettes = self.shared.palettes.borrow_mut();
            palettes.retain(|known| known.name != rule.name);
            palettes.push(rule);
        }
        parse_stylesheet(css).into_iter().filter_map(|face| self.add(face).ok()).collect()
    }

    /// `delete`: whether the face was in the set.
    pub fn delete(&self, id: FontFaceId) -> bool {
        let removed = self.shared.set.borrow_mut().delete(id);
        if removed {
            self.shared.faces.borrow_mut().retain(|slot| slot.id != id);
            self.shared.jobs.borrow_mut().retain(|job| job.id != id);
            self.shared.deliver_events();
            self.shared.fonts_changed();
        }
        removed
    }

    /// `clear`: removes every face.
    pub fn clear(&self) {
        self.shared.set.borrow_mut().clear();
        let members: Vec<FontFaceId> = self.shared.set.borrow().members().collect();
        self.shared.faces.borrow_mut().retain(|slot| members.contains(&slot.id));
        self.shared.jobs.borrow_mut().retain(|job| members.contains(&job.id));
        self.shared.deliver_events();
        self.shared.fonts_changed();
    }

    /// The `status` of the set.
    pub fn status(&self) -> FontFaceSetLoadStatus {
        self.shared.set.borrow().status()
    }

    /// Whether the set's `ready` promise is fulfilled.
    pub fn is_ready(&self) -> bool {
        self.shared.set.borrow().is_ready()
    }

    /// The `status` of a face.
    pub fn face_status(&self, id: FontFaceId) -> Option<FontFaceLoadStatus> {
        self.shared.with_slot(id, |slot| slot.status)
    }

    /// Calls `listener` with each `loading`, `loadingdone` and `loadingerror` event.
    pub fn add_listener(&self, listener: Rc<dyn Fn(&FontFaceSetEvent)>) {
        self.shared.listeners.borrow_mut().push(listener);
    }

    /// `check`: whether `text` can be drawn with what is loaded.
    pub fn check(&self, family: &str, request: &FontMatchRequest, text: &str) -> bool {
        self.shared.views(|views| check_faces(views, family, request, text))
    }

    /// `load`: starts loading the faces `text` needs, and returns them.
    pub fn load(&self, family: &str, request: &FontMatchRequest, text: &str) -> Vec<FontFaceId> {
        let ids = self.shared.views(|views| find_matching_faces(views, family, request, text));
        for id in &ids {
            Shared::request_load(&self.shared, *id);
        }
        ids
    }

    /// One slice of the queued work, of at least one step. Returns whether work remains.
    #[doc(hidden)]
    pub fn run_one_slice(&self, duration: Duration) -> bool {
        Shared::run_slice_for(&self.shared, duration)
    }

    /// Runs the decoding that is queued to its end, for tests and for a caller that has nothing
    /// else to do.
    #[doc(hidden)]
    pub fn run_pending_work(&self) {
        while Shared::run_slice(&self.shared) {}
    }

    fn family_for(&self, name: &str) -> FontFamily {
        FontFamily::parse(&format!("{}#{name}", self.key)).unwrap_or_else(|_| FontFamily::new(name))
    }

    fn family_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for slot in self.shared.faces.borrow().iter() {
            if !names.iter().any(|name| name.eq_ignore_ascii_case(&slot.descriptors.family)) {
                names.push(slot.descriptors.family.clone());
            }
        }
        names
    }
}

impl IFontCollection for WebFontCollection {
    fn key(&self) -> url::Url {
        self.key.clone()
    }

    fn count(&self) -> usize {
        self.family_names().len()
    }

    fn get(&self, index: usize) -> FontFamily {
        self.family_for(&self.family_names()[index])
    }

    fn font_families(&self) -> Vec<FontFamily> {
        self.family_names().iter().map(|name| self.family_for(name)).collect()
    }

    fn try_get_glyph_typeface(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        // The name of a fallback typeface, for one face of a composite.
        let request = match_request(style, weight, stretch);
        if let Some((_, id)) = family_name.split_once(FACE_SEPARATOR) {
            let id = FontFaceId(id.parse().ok()?);
            Shared::request_load(&self.shared, id);
            return self.shared.instance(id, &request);
        }
        let composite = self.shared.composite(family_name, &request);
        // The first available font: the first face that includes the space.
        let first =
            composite.iter().copied().find(|id| self.shared.covers(*id, 0x20)).or(composite.first().copied())?;
        Shared::request_load(&self.shared, first);
        if let Some(found) = self.shared.instance(first, &request) {
            return Some(found);
        }
        let now = (self.shared.clock)();
        let phase = self
            .shared
            .with_slot(first, |slot| (slot.status == FontFaceLoadStatus::Loading).then(|| slot.timeline.phase(now)));
        if phase == Some(Some(FontDisplayPhase::Block)) {
            return self.shared.invisible_typeface();
        }
        // No face to draw with: the font manager falls back to the default font.
        None
    }

    fn try_match_character(
        &self,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        family_name: Option<&str>,
        _culture: Option<&Locale>,
    ) -> Option<Typeface> {
        let family = family_name?.split(FACE_SEPARATOR).next()?;
        let code_point = u32::try_from(codepoint).ok()?;
        let composite = self.shared.composite(family, &match_request(font_style, font_weight, font_stretch));
        for id in composite.iter().copied().filter(|id| self.shared.covers(*id, code_point)) {
            Shared::request_load(&self.shared, id);
            let Some(found) = self.shared.usable_typeface(id) else {
                continue;
            };
            if found.character_to_glyph_map().try_get_glyph(codepoint).is_some() {
                let name = format!("{family}{FACE_SEPARATOR}{}", id.0);
                return Some(Typeface::with_style(self.family_for(&name), font_style, font_weight, font_stretch));
            }
        }
        None
    }

    fn try_get_family_typefaces(&self, family_name: &str) -> Option<Vec<Typeface>> {
        let faces = self.shared.faces.borrow();
        let typefaces: Vec<Typeface> = faces
            .iter()
            .filter(|slot| family_matches(&slot.descriptors, family_name))
            .filter_map(|slot| slot.glyph_typeface.as_ref())
            .map(|face| Typeface::with_style(self.family_for(family_name), face.style(), face.weight(), face.stretch()))
            .collect();
        (!typefaces.is_empty()).then_some(typefaces)
    }

    fn try_create_synthetic_glyph_typeface(
        &self,
        _glyph_typeface: &Rc<GlyphTypeface>,
        _style: FontStyle,
        _weight: FontWeight,
        _stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        None
    }

    fn try_get_nearest_match(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        self.try_get_glyph_typeface(family_name, style, weight, stretch)
    }

    fn dispose(&self) {
        self.shared.jobs.borrow_mut().clear();
        self.shared.finished.borrow_mut().clear();
        self.shared.faces.borrow_mut().clear();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_fonts_web_core::loading::FontFaceSetEventKind as Kind;
    use ferroui_fonts_web_core::woff2::testing::{encode_woff2, stored_brotli, EncodeOptions};
    use ferroui_vello::FontManagerImpl;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn font(path: &str) -> Vec<u8> {
        let full = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
        std::fs::read(&full).unwrap_or_else(|error| panic!("{}: {error}", full.display()))
    }

    fn inter() -> Vec<u8> {
        font("FerroUI.Base/test_assets/fonts/Inter-Regular.ttf")
    }

    type Pending = Rc<RefCell<Vec<(String, Box<dyn FnOnce(FontFetchResult)>)>>>;

    /// A fetcher whose fetches finish when the test says so.
    #[derive(Default)]
    struct TestFetcher {
        pending: Pending,
    }

    impl IFontFaceFetcher for TestFetcher {
        fn fetch(&self, url: &str, completion: Box<dyn FnOnce(FontFetchResult)>) {
            self.pending.borrow_mut().push((url.to_owned(), completion));
        }
    }

    struct Harness {
        collection: WebFontCollection,
        pending: Pending,
        now: Rc<Cell<u64>>,
    }

    impl Harness {
        fn new() -> Self {
            let fetcher = TestFetcher::default();
            let pending = fetcher.pending.clone();
            let now = Rc::new(Cell::new(1000));
            let clock = now.clone();
            let collection = WebFontCollection::with_parts(
                Rc::new(FontManagerImpl::new()),
                Some(Rc::new(fetcher)),
                Box::new(move || clock.get()),
            );
            Self { collection, pending, now }
        }

        fn urls(&self) -> Vec<String> {
            self.pending.borrow().iter().map(|(url, _)| url.clone()).collect()
        }

        fn finish(&self, responses: &HashMap<&str, FontFetchResult>) {
            let pending: Vec<_> = self.pending.borrow_mut().drain(..).collect();
            for (url, completion) in pending {
                let response = responses.get(url.as_str()).cloned().unwrap_or(Err(FontFetchError::NotFound));
                completion(response);
            }
            self.collection.run_pending_work();
        }

        fn lookup(&self, family: &str) -> Option<Rc<GlyphTypeface>> {
            self.collection.try_get_glyph_typeface(family, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        }
    }

    fn request() -> FontMatchRequest {
        FontMatchRequest::default()
    }

    #[test]
    fn nothing_is_fetched_until_a_text_needs_it() {
        let harness = Harness::new();
        harness.collection.add_style_sheet("@font-face { font-family: A; src: url(a.ttf) }");
        assert!(harness.urls().is_empty());
        assert_eq!(harness.collection.count(), 1);
        harness.lookup("A");
        assert_eq!(harness.urls(), ["a.ttf"]);
        harness.lookup("A");
        assert_eq!(harness.urls(), ["a.ttf"], "one fetch");
    }

    #[test]
    fn a_face_loads_and_is_found() {
        let harness = Harness::new();
        let ids = harness.collection.add_style_sheet("@font-face { font-family: A; src: url(a.ttf) }");
        assert!(!harness.collection.check("A", &request(), "x"));
        harness.lookup("A");
        assert_eq!(harness.collection.status(), FontFaceSetLoadStatus::Loading);
        harness.finish(&HashMap::from([("a.ttf", Ok(inter()))]));
        assert_eq!(harness.collection.face_status(ids[0]), Some(FontFaceLoadStatus::Loaded));
        assert_eq!(harness.collection.status(), FontFaceSetLoadStatus::Loaded);
        assert!(harness.collection.is_ready());
        assert!(harness.collection.check("A", &request(), "x"));
        assert!(harness.lookup("A").unwrap().family_name().contains("Inter"));
    }

    #[test]
    fn woff2_loads_through_the_collection() {
        let harness = Harness::new();
        harness.collection.add_style_sheet("@font-face { font-family: A; src: url(a.woff2) format(woff2) }");
        harness.lookup("A");
        let options = EncodeOptions { transform_glyf: true, transform_hmtx: true };
        let file = encode_woff2(&inter(), options, &stored_brotli);
        harness.finish(&HashMap::from([("a.woff2", Ok(file))]));
        #[cfg(feature = "woff2")]
        assert!(harness.lookup("A").is_some());
        #[cfg(not(feature = "woff2"))]
        assert!(harness
            .collection
            .load("A", &request(), "x")
            .iter()
            .all(|id| { harness.collection.face_status(*id) == Some(FontFaceLoadStatus::Error) }));
    }

    #[test]
    fn a_failed_source_falls_through_to_the_next() {
        let harness = Harness::new();
        let ids = harness
            .collection
            .add_style_sheet("@font-face { font-family: A; src: url(missing.ttf), url(broken.ttf), url(good.ttf) }");
        harness.lookup("A");
        assert_eq!(harness.urls(), ["missing.ttf"]);
        harness.finish(&HashMap::new());
        assert_eq!(harness.urls(), ["broken.ttf"]);
        harness.finish(&HashMap::from([("broken.ttf", Ok(b"not a font".to_vec()))]));
        assert_eq!(harness.urls(), ["good.ttf"]);
        harness.finish(&HashMap::from([("good.ttf", Ok(inter()))]));
        assert_eq!(harness.collection.face_status(ids[0]), Some(FontFaceLoadStatus::Loaded));
    }

    #[test]
    fn a_source_this_build_cannot_use_is_skipped() {
        let harness = Harness::new();
        harness.collection.add_style_sheet(
            "@font-face { font-family: A; src: url(a.svg) format(svg), url(b.ttf) tech(incremental), url(c.ttf) }",
        );
        harness.lookup("A");
        assert_eq!(harness.urls(), ["c.ttf"]);
    }

    #[test]
    fn when_every_source_fails_the_face_is_in_error() {
        let harness = Harness::new();
        let log = Rc::new(RefCell::new(Vec::new()));
        let sink = log.clone();
        harness.collection.add_listener(Rc::new(move |event: &FontFaceSetEvent| sink.borrow_mut().push(event.kind)));
        let ids = harness.collection.add_style_sheet("@font-face { font-family: A; src: url(a.ttf) }");
        harness.lookup("A");
        harness.finish(&HashMap::new());
        assert_eq!(harness.collection.face_status(ids[0]), Some(FontFaceLoadStatus::Error));
        assert!(harness.lookup("A").is_none());
        assert_eq!(*log.borrow(), [Kind::Loading, Kind::LoadingDone, Kind::LoadingError]);
    }

    #[test]
    fn unicode_range_loads_only_what_a_text_needs() {
        let harness = Harness::new();
        harness.collection.add_style_sheet(
            "@font-face { font-family: A; src: url(latin.ttf); unicode-range: U+0-FF }
             @font-face { font-family: A; src: url(greek.ttf); unicode-range: U+370-3FF }",
        );
        harness.collection.load("A", &request(), "abc");
        assert_eq!(harness.urls(), ["latin.ttf"]);
        harness.finish(&HashMap::from([("latin.ttf", Ok(inter()))]));
        assert!(harness.collection.check("A", &request(), "abc"));
        assert!(!harness.collection.check("A", &request(), "\u{3b1}"));
        let matched = harness.collection.try_match_character(
            0x3b1,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            Some("A"),
            None,
        );
        assert!(matched.is_none(), "not loaded yet");
        assert_eq!(harness.urls(), ["greek.ttf"], "asked for by the character");
        harness.finish(&HashMap::from([("greek.ttf", Ok(inter()))]));
        let matched = harness.collection.try_match_character(
            0x3b1,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            Some("A"),
            None,
        );
        let matched = matched.expect("the face that covers it");
        let name = matched.font_family().family_names().primary_family_name().to_owned();
        let found = harness.collection.try_get_glyph_typeface(
            &name,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
        );
        assert!(found.is_some(), "the fallback typeface resolves to that face: {name:?}");
    }

    #[test]
    fn text_is_invisible_in_the_block_period_then_falls_back() {
        let harness = Harness::new();
        harness.collection.add_style_sheet("@font-face { font-family: A; src: url(a.ttf); font-display: block }");
        let blank = harness.lookup("A").expect("the invisible stand-in");
        let glyph = blank.character_to_glyph_map().try_get_glyph('x' as i32).expect("the fallback's character map");
        assert!(blank.try_get_horizontal_glyph_advance(glyph).unwrap_or(0) > 0, "it takes room");
        harness.now.set(1000 + 3000);
        assert!(harness.lookup("A").is_none(), "after the block period the fallback shows");
    }

    #[test]
    fn a_font_after_the_failure_period_does_not_replace_the_fallback() {
        let harness = Harness::new();
        let ids = harness
            .collection
            .add_style_sheet("@font-face { font-family: A; src: url(a.ttf); font-display: fallback }");
        harness.lookup("A");
        harness.now.set(1000 + 3100);
        harness.finish(&HashMap::from([("a.ttf", Ok(inter()))]));
        assert_eq!(harness.collection.face_status(ids[0]), Some(FontFaceLoadStatus::Loaded));
        assert!(harness.lookup("A").is_none(), "the text keeps its fallback");
    }

    #[test]
    fn decoding_on_the_ui_thread_is_cut_into_slices() {
        let harness = Harness::new();
        let ids = harness.collection.add_style_sheet("@font-face { font-family: A; src: url(a.ttf) }");
        harness.lookup("A");
        let pending: Vec<_> = harness.pending.borrow_mut().drain(..).collect();
        for (_, completion) in pending {
            completion(Ok(inter()));
        }
        assert_eq!(harness.collection.face_status(ids[0]), Some(FontFaceLoadStatus::Loading), "nothing ran yet");
        let mut slices = 1;
        while harness.collection.run_one_slice(Duration::ZERO) {
            slices += 1;
            assert_eq!(harness.collection.face_status(ids[0]), Some(FontFaceLoadStatus::Loading));
        }
        assert!(slices > 16, "{slices} slices");
        assert_eq!(harness.collection.face_status(ids[0]), Some(FontFaceLoadStatus::Loaded));
    }

    #[test]
    fn metric_descriptors_reach_the_glyph_typeface() {
        let harness = Harness::new();
        harness.collection.add_style_sheet(
            "@font-face { font-family: Plain; src: url(a.ttf) }
             @font-face { font-family: Set; src: url(a.ttf); ascent-override: 50%; line-gap-override: 0% }
             @font-face { font-family: Big; src: url(a.ttf); size-adjust: 200% }",
        );
        for family in ["Plain", "Set", "Big"] {
            harness.lookup(family);
        }
        harness.finish(&HashMap::from([("a.ttf", Ok(inter()))]));
        let plain = harness.lookup("Plain").unwrap().metrics();
        let set = harness.lookup("Set").unwrap().metrics();
        assert_eq!(set.ascent.abs(), (plain.design_em_height as f64 * 0.5).round() as i32);
        assert_eq!(set.line_gap, 0);
        assert_eq!(set.descent, plain.descent, "what is not overridden stays");
        let big = harness.lookup("Big").unwrap().metrics();
        assert_eq!(big.design_em_height, plain.design_em_height / 2, "outlines and metrics scale together");
    }

    fn lookup_with(harness: &Harness, family: &str, style: FontStyle, weight: i32) -> Option<Rc<GlyphTypeface>> {
        harness.collection.try_get_glyph_typeface(family, style, FontWeight(weight), FontStretch::Normal)
    }

    #[test]
    fn a_variable_font_is_set_to_the_request() {
        let harness = Harness::new();
        harness.collection.add_style_sheet("@font-face { font-family: V; src: url(v.ttf); font-weight: 100 900 }");
        harness.lookup("V");
        let bytes = font("FerroUI.Base/test_assets/fonts/InterVariable.ttf");
        harness.finish(&HashMap::from([("v.ttf", Ok(bytes))]));
        let regular = lookup_with(&harness, "V", FontStyle::Normal, 400).unwrap();
        let bold = lookup_with(&harness, "V", FontStyle::Normal, 700).unwrap();
        assert!(!Rc::ptr_eq(&regular, &bold));
        assert_eq!(bold.font_simulations(), FontSimulations::None, "the axis does the work");
        let variation = bold.platform_typeface().variation_settings();
        let weight = variation.variations().iter().find(|axis| axis.tag.value() == u32::from_be_bytes(*b"wght"));
        assert_eq!(weight.map(|axis| axis.value), Some(700.0));
        let again = lookup_with(&harness, "V", FontStyle::Normal, 700).unwrap();
        assert!(Rc::ptr_eq(&bold, &again), "an instance is made once");
    }

    #[test]
    fn a_face_without_bold_or_italic_is_simulated() {
        let harness = Harness::new();
        harness.collection.add_style_sheet("@font-face { font-family: A; src: url(a.ttf) }");
        harness.lookup("A");
        harness.finish(&HashMap::from([("a.ttf", Ok(inter()))]));
        let regular = lookup_with(&harness, "A", FontStyle::Normal, 400).unwrap();
        assert_eq!(regular.font_simulations(), FontSimulations::None);
        let bold = lookup_with(&harness, "A", FontStyle::Normal, 700).unwrap();
        assert!(bold.font_simulations().contains(FontSimulations::Bold));
        let italic = lookup_with(&harness, "A", FontStyle::Italic, 400).unwrap();
        assert!(italic.font_simulations().contains(FontSimulations::Oblique));
    }

    #[test]
    fn over_the_budget_the_least_recently_used_face_is_let_go() {
        let harness = Harness::new();
        let ids = harness.collection.add_style_sheet(
            "@font-face { font-family: A; src: url(a.ttf) } @font-face { font-family: B; src: url(b.ttf) }",
        );
        harness.lookup("A");
        harness.lookup("B");
        harness.finish(&HashMap::from([("a.ttf", Ok(inter())), ("b.ttf", Ok(inter()))]));
        drop(harness.lookup("A"));
        let held = harness.lookup("B").unwrap();
        let one_font = inter().len() + 1024;
        harness.collection.set_memory_budget(one_font);
        assert_eq!(harness.collection.face_status(ids[0]), Some(FontFaceLoadStatus::Loaded), "the status stays");
        assert!(harness.urls().is_empty());
        assert!(harness.lookup("A").is_none(), "A was let go, and is fetched again");
        assert_eq!(harness.urls(), ["a.ttf"]);
        harness.collection.set_memory_budget(usize::MAX);
        harness.finish(&HashMap::from([("a.ttf", Ok(inter()))]));
        assert!(harness.lookup("A").is_some());
        assert!(Rc::ptr_eq(&held, &harness.lookup("B").unwrap()), "B was kept");
    }

    #[test]
    fn a_face_in_use_is_not_let_go() {
        let harness = Harness::new();
        harness.collection.add_style_sheet("@font-face { font-family: A; src: url(a.ttf) }");
        harness.lookup("A");
        harness.finish(&HashMap::from([("a.ttf", Ok(inter()))]));
        let held = harness.lookup("A").unwrap();
        harness.collection.set_memory_budget(0);
        assert!(Rc::ptr_eq(&held, &harness.lookup("A").unwrap()));
    }

    /// The full name of a font installed on this machine, if it has any.
    fn an_installed_font() -> Option<(String, String)> {
        let platform = FontManagerImpl::new();
        for family in platform.get_installed_font_family_names(false) {
            let Some(typefaces) = platform.try_get_family_typefaces(&family) else {
                continue;
            };
            for candidate in typefaces {
                let Some(found) = platform
                    .try_create_glyph_typeface(&family, candidate.style(), candidate.weight(), candidate.stretch())
                    .and_then(|typeface| GlyphTypeface::try_create(typeface, FontSimulations::None))
                else {
                    continue;
                };
                let tag = OpenTypeTag::new(u32::from_be_bytes(*b"name"));
                let Some(table) = found.platform_typeface().try_get_table(tag) else {
                    continue;
                };
                // A name that starts with the family, as the search by leading words expects.
                if let Some(full) = names_of(table.span(), 4).into_iter().find(|name| name.starts_with(&family)) {
                    return Some((family, full));
                }
            }
        }
        None
    }

    #[test]
    fn local_finds_an_installed_font_by_its_full_name() {
        let Some((_, full)) = an_installed_font() else {
            return;
        };
        let harness = Harness::new();
        let ids = harness
            .collection
            .add_style_sheet(&format!("@font-face {{ font-family: A; src: local(\"{full}\"), url(a.ttf) }}"));
        let found = harness.lookup("A").expect("the installed font");
        assert!(has_local_name(&found, &full));
        assert!(harness.urls().is_empty(), "nothing is fetched");
        assert_eq!(harness.collection.face_status(ids[0]), Some(FontFaceLoadStatus::Loaded));
    }

    #[test]
    fn a_local_font_that_is_not_installed_falls_through() {
        let harness = Harness::new();
        harness
            .collection
            .add_style_sheet("@font-face { font-family: A; src: local(\"No Such Font Anywhere\"), url(a.ttf) }");
        harness.lookup("A");
        assert_eq!(harness.urls(), ["a.ttf"]);
    }

    #[test]
    fn a_family_name_alone_does_not_match_another_face() {
        let Some((family, full)) = an_installed_font() else {
            return;
        };
        let harness = Harness::new();
        let wrong = format!("{family} No Such Style");
        assert_ne!(wrong, full);
        harness
            .collection
            .add_style_sheet(&format!("@font-face {{ font-family: A; src: local(\"{wrong}\"), url(a.ttf) }}"));
        harness.lookup("A");
        assert_eq!(harness.urls(), ["a.ttf"], "no other face of the family is substituted");
    }

    #[test]
    fn a_color_technology_is_not_claimed_unless_the_application_says_so() {
        let harness = Harness::new();
        harness.collection.add_style_sheet(
            "@font-face { font-family: A; src: url(color.ttf) tech(color-COLRv1), url(plain.ttf) }
             @font-face { font-family: B; src: url(color.ttf) tech(color-COLRv1), url(plain.ttf) }",
        );
        harness.lookup("A");
        assert_eq!(harness.urls(), ["plain.ttf"]);
        harness.pending.borrow_mut().clear();
        harness.collection.set_supported_techs(DEFAULT_TECHS.with(FontTech::ColorColrV1));
        harness.lookup("B");
        assert_eq!(harness.urls(), ["color.ttf"]);
    }

    #[test]
    fn a_palette_is_written_into_the_font_when_it_loads() {
        let harness = Harness::new();
        let bytes = font("../samples/ControlCatalog/Assets/FallbackFonts/NotoColorEmoji-Catalog.ttf");
        let has_palette = ferroui_fonts_web_core::woff2::testing::font_table(&bytes, b"CPAL").is_some();
        harness.collection.add_style_sheet(
            "@font-face { font-family: A; src: url(a.ttf) }
             @font-palette-values --mine { font-family: A; override-colors: 0 #ff0000 }",
        );
        assert!(!harness.collection.set_font_palette("A", Some("--nope")), "no such rule");
        assert!(!harness.collection.set_font_palette("B", Some("--mine")), "not for that family");
        assert!(harness.collection.set_font_palette("A", Some("--mine")));
        harness.lookup("A");
        harness.finish(&HashMap::from([("a.ttf", Ok(bytes))]));
        let found = harness.lookup("A").expect("the font");
        if has_palette {
            let table =
                found.platform_typeface().try_get_table(OpenTypeTag::new(u32::from_be_bytes(*b"CPAL"))).unwrap();
            let table = table.span();
            let records = u32::from_be_bytes([table[8], table[9], table[10], table[11]]) as usize;
            let first = u16::from_be_bytes([table[12], table[13]]) as usize;
            assert_eq!(table[records + first * 4..records + first * 4 + 4], [0, 0, 255, 255]);
        }
        assert!(harness.collection.set_font_palette("A", None));
        assert!(harness.lookup("A").is_none(), "the font is loaded again with its own palette");
        assert_eq!(harness.urls(), ["a.ttf"]);
    }

    #[test]
    fn a_face_for_some_sizes_is_not_chosen_over_one_for_any_size() {
        let harness = Harness::new();
        harness.collection.add_style_sheet(
            "@font-face { font-family: A; src: url(any.ttf) }
             @font-face { font-family: A; src: url(display.ttf); font-size: 48 200 }",
        );
        harness.lookup("A");
        assert_eq!(harness.urls(), ["any.ttf"]);
    }

    #[test]
    fn with_only_sized_faces_the_range_nearest_the_default_size_is_used() {
        let harness = Harness::new();
        harness.collection.add_style_sheet(
            "@font-face { font-family: A; src: url(caption.ttf); font-size: 6 11 }
             @font-face { font-family: A; src: url(text.ttf); font-size: 12 24 }
             @font-face { font-family: A; src: url(display.ttf); font-size: 25 200 }",
        );
        harness.lookup("A");
        assert_eq!(harness.urls(), ["text.ttf"]);
    }

    #[test]
    fn at_most_eight_fonts_are_fetched_at_once() {
        let harness = Harness::new();
        let css: String =
            (0..12).map(|index| format!("@font-face {{ font-family: F{index}; src: url(f{index}.ttf) }}")).collect();
        harness.collection.add_style_sheet(&css);
        for index in 0..12 {
            harness.lookup(&format!("F{index}"));
        }
        assert_eq!(harness.urls().len(), 8);
        harness.finish(&HashMap::new());
        assert_eq!(harness.urls(), ["f8.ttf", "f9.ttf", "f10.ttf", "f11.ttf"], "the others follow, in order");
    }

    #[test]
    fn relative_urls_use_the_base_uri() {
        let harness = Harness::new();
        harness.collection.set_base_uri(url::Url::parse("https://example.test/app/").unwrap());
        harness.collection.add_style_sheet("@font-face { font-family: A; src: url(fonts/a.woff2) }");
        harness.lookup("A");
        assert_eq!(harness.urls(), ["https://example.test/app/fonts/a.woff2"]);
    }

    #[test]
    fn a_data_url_loads_through_the_default_fetcher() {
        const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bytes = font("Headless/FerroUI.Headless/BareMinimum.ttf");
        let mut text = String::with_capacity(bytes.len() / 3 * 4 + 4);
        for chunk in bytes.chunks(3) {
            let word = (chunk[0] as u32) << 16
                | (*chunk.get(1).unwrap_or(&0) as u32) << 8
                | *chunk.get(2).unwrap_or(&0) as u32;
            for index in 0..4 {
                if index <= chunk.len() {
                    text.push(ALPHABET[(word >> (18 - index * 6)) as usize & 63] as char);
                } else {
                    text.push('=');
                }
            }
        }
        let collection = WebFontCollection::with_parts(
            Rc::new(FontManagerImpl::new()),
            Some(Rc::new(crate::DefaultFontFaceFetcher::default())),
            Box::new(|| 0),
        );
        let ids = collection
            .add_style_sheet(&format!("@font-face {{ font-family: A; src: url(data:font/ttf;base64,{text}) }}"));
        collection.load("A", &request(), "x");
        collection.run_pending_work();
        assert_eq!(collection.face_status(ids[0]), Some(FontFaceLoadStatus::Loaded));
    }

    #[test]
    fn delete_and_clear() {
        let harness = Harness::new();
        let ids = harness.collection.add_style_sheet(
            "@font-face { font-family: A; src: url(a.ttf) } @font-face { font-family: B; src: url(b.ttf) }",
        );
        assert!(harness.collection.delete(ids[0]));
        assert!(!harness.collection.delete(ids[0]));
        assert_eq!(harness.collection.count(), 1);
        harness.collection.clear();
        assert_eq!(harness.collection.count(), 0);
    }
}
