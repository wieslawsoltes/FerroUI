//! Counters of the work the framework does per operation: how many property
//! changes are raised and how many of them anybody listens to, how many
//! styles are evaluated and how many match, how many resources are looked
//! up, how many bindings and text layouts are created. The performance
//! designs (`docs/porting/performance/designs`) each start by reading them.
//!
//! An addition of the port (DEVIATIONS.md, Diagnostics): upstream has no
//! counterpart. The counters exist only with the feature `perf-counters`.
//! Without it [`perf_count!`](crate::perf_count) and
//! [`perf_count_virtual!`](crate::perf_count_virtual) expand to no code, the
//! functions of this module have empty bodies and
//! [`PerfCountersSnapshot`] has no size; no type of the framework gains a
//! field either way.
//!
//! The counters are per thread (the thread of the dispatcher is the one that
//! is measured), and counting changes no behaviour. Read them around the
//! measured work:
//!
//! ```ignore
//! let before = perf_counters::snapshot();
//! // ... the work ...
//! let counted = perf_counters::snapshot().since(&before);
//! println!("{}", counted.report_per(rows, "row"));
//! ```

use std::fmt;

/// Whether the counters are compiled in (the feature `perf-counters`).
pub const ENABLED: bool = cfg!(feature = "perf-counters");

macro_rules! perf_counters {
    ($($(#[$meta:meta])* $name:ident => $label:literal,)*) => {
        /// What is counted. Each counter names, in its documentation, the
        /// function that counts it.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(usize)]
        pub enum PerfCounter {
            $($(#[$meta])* $name,)*
        }

        impl PerfCounter {
            /// Every counter, in the order of the report.
            pub const ALL: &'static [PerfCounter] = &[$(PerfCounter::$name,)*];

            /// The name of the counter in the report.
            pub const fn name(self) -> &'static str {
                match self {
                    $(PerfCounter::$name => $label,)*
                }
            }
        }
    };
}

perf_counters! {
    // --- property changes (design 02) ---
    /// `FerroObject::raise_property_changed`: every notification built and
    /// passed to the `on_property_changed` chain of the class.
    PropertyChangesRaised => "property changes raised",
    /// `FerroObject::raise_property_changed`: the ones of an effective
    /// value, which go on to the handlers of the property and the listeners
    /// of the object.
    PropertyChangesEffective => "property changes of an effective value",
    /// `PropertyChangedObservable::notify`: the property has at least one
    /// handler (the class handlers and the subscriptions to the property).
    PropertyChangesWithPropertyHandlers => "property changes with handlers of the property",
    /// `FerroObject::raise_property_changed`: the object has at least one
    /// listener of its property changes.
    PropertyChangesWithObjectListeners => "property changes with listeners of the object",

    // --- inheritance (design 03) ---
    /// `ValueStore::set_inheritance_parent`: the inheritance ancestor of an
    /// object changed.
    InheritanceAncestorChanges => "inheritance ancestor changes",
    /// `ValueStore::set_inheritance_parent`: the inherited properties of the
    /// old and the new ancestor chain that were paired.
    InheritedValuesCompared => "inherited values compared on an ancestor change",
    /// `ValueStore::set_inheritance_parent`: the ones whose old and new
    /// value differ, each of which starts a walk of the subtree.
    InheritedValuesDiffering => "inherited values differing on an ancestor change",
    /// `ValueStore::inherited_value_changed`: the objects such a walk
    /// reaches (the object itself and its inheritance descendants, once per
    /// property).
    InheritedValueWalkVisits => "objects visited by inherited value walks",
    /// `ValueStore::on_inheritance_ancestor_changed`: visits that update the
    /// nearest inherited-value holder, including stops at local holders.
    InheritanceAncestorWalkVisits => "objects visited by inheritance ancestor maintenance",
    /// `ValueStore::on_ancestor_inherited_value_changed`: visits for a change
    /// to an existing inherited effective value, including local overrides.
    InheritedEffectiveValueWalkVisits => "objects visited by inherited effective value changes",

    // --- tree attachment, styles and resources (design 04) ---
    /// `ValueStore::end_styling`: completed outermost styling batches.
    StylingPassesCompleted => "outermost styling batches completed",
    /// `ValueStore::reevaluate_effective_value`: non-deferred single-property
    /// reevaluation attempts (including restarts).
    SingleValueReevaluations => "single property reevaluations",
    /// `ValueStore::reevaluate_effective_values`: non-deferred full-store
    /// reevaluation attempts (including restarts).
    FullValueReevaluations => "full value store reevaluations",
    /// Existing effective values reset by a full-store reevaluation.
    ReevaluatedEffectiveValues => "effective values reset by full reevaluations",
    /// Frame entries visited by a full-store reevaluation, before priority
    /// and value availability checks.
    ReevaluatedFrameEntries => "frame entries visited by full reevaluations",
    /// `StyledElement::on_attached_to_logical_tree_core`: an element that
    /// was not in a logical tree enters one.
    LogicalTreeAttachments => "elements attached to a logical tree",
    /// `StyledElement::on_detached_from_logical_tree_core`: an element
    /// leaves its logical tree.
    LogicalTreeDetachments => "elements detached from a logical tree",
    /// `StyledElement::get_effective_theme`: the implicit theme was not
    /// known and was looked up as a resource.
    ImplicitThemeLookups => "implicit theme lookups",
    /// `StyledElement::apply_styles`: a style host between the element and
    /// the root was asked for its styles.
    StyleHostsWalked => "style hosts walked",
    /// `Style::try_attach_checked` and `ContainerQuery::try_attach_checked`:
    /// a style with setters (a query with children) was matched against an
    /// element.
    StylesEvaluated => "styles evaluated",
    /// The same functions: the selector (the query) matched and the style
    /// was instanced on the element.
    StylesMatched => "styles matched",
    /// `ControlTheme::try_attach_checked`: a control theme was tested
    /// against the style key of an element.
    ControlThemesEvaluated => "control themes evaluated",
    /// `ControlTheme::try_attach_checked`: it applies and was instanced.
    ControlThemesMatched => "control themes matched",
    /// `StyleBase::try_attach_instance`: a style instance was added to the
    /// value store of an element.
    StyleInstancesAttached => "style instances attached",
    /// `StyleBase::try_attach_instance`: the instance was built (its setters
    /// instanced) instead of the shared instance of the style being taken.
    StyleInstancesCreated => "style instances created",
    /// `IResourceHost::try_find_resource`: a lookup up the logical tree.
    ResourceLookups => "resource lookups",
    /// `IResourceHost::try_find_resource`: the hosts such a lookup asked
    /// (`try_get_resource` of the host and of each styling parent).
    ResourceHostsProbed => "resource hosts probed",

    // --- bindings (design 05) ---
    /// `FerroObject::bind_binding_with_anchor`: a binding was instanced on
    /// a property of an object.
    BindingsInstanced => "bindings instanced",
    /// `UntypedBindingExpressionBase::new` and `TypedBindingExpression::new`:
    /// a binding expression was created (by a binding, a setter, a template
    /// binding or a dynamic resource).
    BindingExpressionsCreated => "binding expressions created",
    /// `TemplateBindingExpression::new`.
    TemplateBindingExpressionsCreated => "template binding expressions created",
    /// `DynamicResourceExpression::new` (the XAML runtime library).
    DynamicResourceExpressionsCreated => "dynamic resource expressions created",
    /// `SetterBindingEntry::new`: a style attach made the entry of a setter
    /// whose value is a binding. The entry creates its binding expression
    /// when its value is first asked for.
    SetterBindingEntriesCreated => "setter binding entries created",
    /// `UntypedBindingExpression::start` of an expression that was not
    /// running: the expression subscribed to its source.
    BindingExpressionsStarted => "binding expressions started",
    /// `DynamicResourceExpression::on_start` (the XAML runtime library): the
    /// expression found its host, subscribed to it and looked its resource up.
    DynamicResourceExpressionsStarted => "dynamic resource expressions started",
    /// `DynamicResourceExpression::publish_value` (the XAML runtime library):
    /// a dynamic resource looked its resource up and published it.
    DynamicResourceLookups => "dynamic resource lookups",
    /// `DynamicResourceExpression` (the XAML runtime library): a notification
    /// of changed resources that was not newer than the last lookup of the
    /// expression, which did not look up again.
    DynamicResourceNotificationsSeen => "dynamic resource notifications already seen",
    /// `UntypedBindingExpressionBase::publish_value` of a running
    /// expression and `TypedBindingExpression::publish_value`.
    BindingValuesPublished => "binding values published",

    // --- text (design 06) ---
    /// `TextLayout::from_text_source`.
    TextLayoutsCreated => "text layouts created",
    /// `TextLayoutCache::get_or_create`: the layout of a plain text was the one kept for it.
    TextLayoutCacheHits => "text layout cache hits",
    /// `TextLayoutCache::get_or_create`: the layout of a plain text was made and kept.
    TextLayoutCacheMisses => "text layout cache misses",
    /// `TextLayoutCache`: a kept layout nothing held was let go for room.
    TextLayoutCacheEvictions => "text layout cache evictions",
    /// `TextLayoutCache::get_or_create`: the cache was full and was asked for the text for
    /// the first time; the layout was made for its caller alone.
    TextLayoutCacheFirstRequests => "text layout cache: first requests of a text, not kept",
    /// `TextFormatterImpl::format_line_with_cache`: a line was asked for.
    TextLinesFormatted => "text lines formatted",
    /// `TextFormatterImpl::format_line_with_cache`: the shaped runs of the
    /// line came from the text run cache.
    TextRunCacheHits => "text run cache hits",
    /// `TextFormatterImpl::format_line_with_cache`: a cache was given, did
    /// not hold the line, and the shaped runs were added to it.
    TextRunCacheMisses => "text run cache misses",
    /// `TextShaper::shape_text`: a run was shaped by the platform shaper.
    TextRunsShaped => "text runs shaped",
    /// `TextBlock::invalidate_text_layout`: a layout was dropped with its shaped runs (the text
    /// or a property the shaping depends on changed).
    TextBlockLayoutDropsShaping => "text block layout drops: shaping",
    /// `TextBlock::invalidate_text_layout_keep_cache`: a layout was dropped and its shaped runs
    /// kept (a property of the paragraph changed).
    TextBlockLayoutDropsParagraph => "text block layout drops: paragraph",
    /// `TextBlock::on_measure_invalidated`: a layout was dropped because the measure of the
    /// text block was invalidated by something else than its text.
    TextBlockLayoutDropsMeasureInvalidated => "text block layout drops: measure invalidated",
    /// `TextBlock::measure_override`: a layout was dropped for the constraint of the measure.
    TextBlockLayoutDropsMeasureConstraint => "text block layout drops: measure constraint",
    /// `TextBlock::measure_override`: a layout was dropped because an embedded control was
    /// measured again.
    TextBlockLayoutDropsEmbeddedControls => "text block layout drops: embedded controls",
    /// `TextBlock::arrange_override`: a layout was dropped because the alignment of the arrange
    /// is not the one it was created with.
    TextBlockLayoutDropsArrangeAlignment => "text block layout drops: arrange alignment",
    /// `TextBlock::arrange_override`: a layout was dropped for the constraint of the arrange.
    TextBlockLayoutDropsArrangeConstraint => "text block layout drops: arrange constraint",
    /// `TextBlock::measure_override`: the measure created the layout.
    TextBlockLayoutsCreatedByMeasure => "text block layouts created by measure",
    /// `TextBlock::arrange_override`: the arrange created the layout.
    TextBlockLayoutsCreatedByArrange => "text block layouts created by arrange",

    // --- containers (the controls crate) ---
    /// `VirtualizingStackPanel::recycle_element`: a container left the
    /// viewport, was cleared and went to the recycle pool.
    ContainersRecycled => "containers recycled",
    /// `VirtualizingStackPanel::get_recycled_element`: a container of the
    /// pool was prepared for another item.
    ContainersReused => "containers reused",
    /// `VirtualizingStackPanel::create_element`: a container was created.
    ContainersCreated => "containers created",
    /// `ContentPresenter::update_child_with`: the child of a content
    /// presenter was replaced by another control or removed.
    ContentPresenterChildrenReplaced => "content presenter children replaced",

    // --- layout and rendering ---
    /// `Layoutable::measure`: the measure of an element ran (its result was not the cached one).
    MeasuresExecuted => "measures executed",
    /// `Layoutable::arrange`: the arrange of an element ran.
    ArrangesExecuted => "arranges executed",
    /// `Interactive::build_event_route`: a routed event was raised that has no class handler
    /// and no instance handler on the thread, so its route was left empty without walking the
    /// ancestors.
    RoutedEventRoutesSkipped => "routed event routes skipped (nothing listens)",
    /// `LayoutManager::inner_layout_pass`: a round of measure and arrange passes.
    LayoutPassRounds => "layout pass rounds",
    /// `LayoutManager::invalidate_measure`: the measure of an element was invalidated while a
    /// layout pass ran (what makes a pass take another round).
    LayoutMeasureInvalidatedInPass => "measures invalidated during a layout pass",
    /// `CompositingRenderer::update_core`: the drawing of a visual was recorded.
    VisualsRecorded => "visuals recorded",
    /// `CompositingRenderer::update_core`: a recorded drawing was the one the visual had, and
    /// was not sent again.
    VisualsRecordedUnchanged => "visuals recorded unchanged",
    /// `CompositingRenderer::update_core`: a recorded drawing was given to the render data the
    /// visual had, whose server-side object was kept.
    VisualsRecordedInPlace => "visuals recorded into their render data",
    /// `Visual::attach_to_compositor_core`: a composition visual was created for a visual that
    /// entered a visual tree.
    CompositionVisualsCreated => "composition visuals created",
    /// `Visual::attach_to_compositor_core`: a visual that entered a visual tree rejoined with
    /// the composition visual it had when it left one (desktop-performance.md, section 9.32).
    CompositionVisualsKept => "composition visuals kept",
    /// `Compositor::commit`: a batch of changes was sent to the render thread.
    CompositionBatchesCommitted => "composition batches committed",
    /// `Compositor::commit`: the entries of the object stream of the batches (a server object,
    /// a marker, a handle that is sent as it is).
    CompositionBatchObjects => "composition batch objects",
    /// `Compositor::commit`: the bytes of the value stream of the batches.
    CompositionBatchValueBytes => "composition batch value bytes",
    /// `MediaContext::render`: a render pass of the UI thread (callbacks, layout, commit).
    UiRenderPasses => "ui: render passes",

    // --- the frames of the render thread (counted on the thread that draws) ---
    /// `ServerCompositionTarget::render_frame`: a frame was drawn.
    RenderFramesDrawn => "render: frames drawn",
    /// `ServerCompositionTarget::render_frame`: the pixels of the bounds of the dirty region.
    RenderDirtyBoundsPixels => "render: pixels of the dirty bounds",
    /// `ServerCompositionTarget::render_frame`: the pixels of the target.
    RenderTargetPixels => "render: pixels of the target",
    /// The update pass (`UpdateContext::pre_subgraph`): a visual was visited.
    RenderUpdateVisualsVisited => "render: visuals visited by the update pass",
    /// The update pass (`UpdateContext::post_subgraph`): a visual was dirty for render.
    RenderVisualsDirty => "render: visuals dirty for render",
    /// The update pass (`UpdateContext::add_to_dirty_region`): a rectangle was added to the
    /// dirty region.
    RenderDirtyRectsAdded => "render: dirty rectangles added",
    /// The render walk (`ServerCompositionVisual::render`): a visual was visited.
    RenderVisualsVisited => "render: visuals visited by the render walk",
    /// The render walk: a visual was drawn (it meets the clip and the dirty region).
    RenderVisualsRendered => "render: visuals rendered",
    /// The render walk: a visual with its own transform set the transform of the canvas.
    RenderVisualTransformsSet => "render: visual transforms set",
    /// The drawing context of the backend: a transform of the context reached the canvas (a
    /// transform that is replaced before the canvas is used does not).
    RenderCanvasTransformsApplied => "render: transforms applied to the canvas",
    /// The render walk: a visual pushed its clip to bounds.
    RenderVisualClipsPushed => "render: visual clips to bounds pushed",
    /// The render walk: a visual pushed its opacity.
    RenderVisualOpacitiesPushed => "render: visual opacities pushed",
    /// The render walk: a visual was drawn from its bitmap cache.
    RenderCachedVisualsDrawn => "render: visuals drawn from a cache",
    /// The replay of a drawing (`RenderDataStreamReplay`): a line.
    RenderLinesDrawn => "render: lines drawn",
    /// The replay of a drawing: a rectangle.
    RenderRectanglesDrawn => "render: rectangles drawn",
    /// The replay of a drawing: an ellipse.
    RenderEllipsesDrawn => "render: ellipses drawn",
    /// The replay of a drawing: a geometry.
    RenderGeometriesDrawn => "render: geometries drawn",
    /// The replay of a drawing: a glyph run.
    RenderGlyphRunsDrawn => "render: glyph runs drawn",
    /// The drawing context of the backend: the glyphs of the glyph runs drawn.
    RenderGlyphsDrawn => "render: glyphs drawn",
    /// The drawing context of the backend: a glyph run was drawn for the first time, from its
    /// glyphs and without a retained blob.
    RenderGlyphRunsFirstDrawn => "render: glyph runs drawn for the first time",
    /// The replay of a drawing: a bitmap.
    RenderBitmapsDrawn => "render: bitmaps drawn",
    /// The replay of a drawing: a clip was pushed.
    RenderClipsPushed => "render: clips pushed by drawings",
    /// The replay of a drawing: a geometry clip was pushed.
    RenderGeometryClipsPushed => "render: geometry clips pushed by drawings",
    /// The replay of a drawing: an opacity, a layer, an opacity mask or an effect was pushed.
    RenderLayersPushed => "render: opacities, layers, masks and effects pushed by drawings",
    /// The replay of a drawing: a transform was pushed.
    RenderTransformsPushed => "render: transforms pushed by drawings",

    // --- the pipelined frame layers of the Skia backend (desktop-performance.md, section 9.33) ---
    /// `PipelinedLayer::begin_frame`: a layer that drew its frames directly began to pipeline
    /// them.
    SkiaLayersBeganToPipeline => "skia: layers that began to pipeline their frames",
    /// `PipelinedLayer::finish_frame`: a frame was handed to the threads of the layer.
    SkiaFramesPipelined => "skia: frames pipelined",
    /// `PipelinedLayer::finish_frame`: a recorded frame was played on the recorder of the
    /// context (it had a draw only that recorder draws, or the threads were not there).
    SkiaPipelinedFramesPlayedDirectly => "skia: recorded frames played on the recorder of the context",
    /// `PipelinedLayerSurface::surface`: the layer was read while one of its frames was drawn.
    SkiaPipelinedLayerReadsInFrames => "skia: reads of a pipelined layer while a frame is drawn",
}

/// The number of counters.
#[cfg(feature = "perf-counters")]
const COUNT: usize = PerfCounter::ALL.len();

/// Counts one occurrence of `$counter` (a variant of
/// [`PerfCounter`](crate::diagnostics::perf_counters::PerfCounter)) on this
/// thread.
#[cfg(feature = "perf-counters")]
#[macro_export]
macro_rules! perf_count {
    ($counter:ident) => {
        $crate::diagnostics::perf_counters::count($crate::diagnostics::perf_counters::PerfCounter::$counter)
    };
}

/// Counts one occurrence of `$counter` (a variant of
/// [`PerfCounter`](crate::diagnostics::perf_counters::PerfCounter)) on this
/// thread. Without the feature `perf-counters` it only names the counter,
/// so that a misspelt one does not compile: there is no code.
#[cfg(not(feature = "perf-counters"))]
#[macro_export]
macro_rules! perf_count {
    ($counter:ident) => {{
        let _ = $crate::diagnostics::perf_counters::PerfCounter::$counter;
    }};
}

/// Counts one virtual call of the member `$member` (a string literal,
/// `"Class::member"`) on this thread.
#[cfg(feature = "perf-counters")]
#[macro_export]
macro_rules! perf_count_virtual {
    ($member:expr) => {
        $crate::diagnostics::perf_counters::count_virtual_call($member)
    };
}

/// Counts one virtual call of the member `$member` (a string literal,
/// `"Class::member"`) on this thread. Without the feature `perf-counters`
/// it expands to the unit value: there is no code.
#[cfg(not(feature = "perf-counters"))]
#[macro_export]
macro_rules! perf_count_virtual {
    ($member:expr) => {
        ()
    };
}

#[cfg(feature = "perf-counters")]
mod counting {
    use super::{PerfCounter, COUNT};
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    crate::ferro_thread_local! {
        static COUNTS: [Cell<u64>; COUNT] = const { [const { Cell::new(0) }; COUNT] };
        static VIRTUAL_CALLS: RefCell<HashMap<&'static str, u64>> = RefCell::new(HashMap::new());
    }

    #[inline]
    pub(super) fn count(counter: PerfCounter) {
        // A thread that is ending has no counters left: nothing is counted.
        let _ = COUNTS.try_with(|counts| {
            let cell = &counts[counter as usize];
            cell.set(cell.get() + 1);
        });
    }

    #[inline]
    pub(super) fn add(counter: PerfCounter, amount: u64) {
        let _ = COUNTS.try_with(|counts| {
            let cell = &counts[counter as usize];
            cell.set(cell.get() + amount);
        });
    }

    pub(super) fn count_virtual_call(member: &'static str) {
        let _ = VIRTUAL_CALLS.try_with(|calls| {
            // The table is never borrowed across a call of the framework;
            // a call made while it is read would not be counted.
            if let Ok(mut calls) = calls.try_borrow_mut() {
                *calls.entry(member).or_insert(0) += 1;
            }
        });
    }

    pub(super) fn counts() -> [u64; COUNT] {
        COUNTS.with(|counts| std::array::from_fn(|index| counts[index].get()))
    }

    pub(super) fn virtual_calls() -> Vec<(&'static str, u64)> {
        let mut calls: Vec<(&'static str, u64)> =
            VIRTUAL_CALLS.with(|calls| calls.borrow().iter().map(|(member, count)| (*member, *count)).collect());
        super::sort_virtual_calls(&mut calls);
        calls
    }

    pub(super) fn reset() {
        COUNTS.with(|counts| {
            for cell in counts.iter() {
                cell.set(0);
            }
        });
        VIRTUAL_CALLS.with(|calls| calls.borrow_mut().clear());
    }
}

/// The most called member first; members with the same count by name.
#[cfg(feature = "perf-counters")]
fn sort_virtual_calls(calls: &mut [(&'static str, u64)]) {
    calls.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
}

/// Counts one occurrence of `counter` on this thread. Call it through
/// [`perf_count!`](crate::perf_count).
#[cfg(feature = "perf-counters")]
#[inline]
pub fn count(counter: PerfCounter) {
    counting::count(counter)
}

/// Counts one occurrence of `counter` on this thread: nothing, since the
/// feature `perf-counters` is off.
#[cfg(not(feature = "perf-counters"))]
#[inline(always)]
pub fn count(_counter: PerfCounter) {}

/// Adds `amount` occurrences of `counter` on this thread.
#[cfg(feature = "perf-counters")]
#[inline]
pub fn add(counter: PerfCounter, amount: u64) {
    counting::add(counter, amount)
}

/// Adds `amount` occurrences of `counter` on this thread: nothing, since
/// the feature `perf-counters` is off.
#[cfg(not(feature = "perf-counters"))]
#[inline(always)]
pub fn add(_counter: PerfCounter, _amount: u64) {}

/// Counts one virtual call of `member` (`"Class::member"`) on this thread.
/// Call it through [`perf_count_virtual!`](crate::perf_count_virtual).
#[cfg(feature = "perf-counters")]
#[inline]
pub fn count_virtual_call(member: &'static str) {
    counting::count_virtual_call(member)
}

/// Counts one virtual call of `member` on this thread: nothing, since the
/// feature `perf-counters` is off.
#[cfg(not(feature = "perf-counters"))]
#[inline(always)]
pub fn count_virtual_call(_member: &'static str) {}

/// The counters of this thread at one moment, or the difference of two such
/// moments ([`since`](Self::since)). Without the feature `perf-counters` it
/// holds nothing and every count reads zero.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PerfCountersSnapshot {
    #[cfg(feature = "perf-counters")]
    counts: Option<[u64; COUNT]>,
    #[cfg(feature = "perf-counters")]
    virtual_calls: Vec<(&'static str, u64)>,
}

// Without the feature the snapshot is nothing at all.
#[cfg(not(feature = "perf-counters"))]
const _: () = assert!(std::mem::size_of::<PerfCountersSnapshot>() == 0);

/// The counters of this thread now.
pub fn snapshot() -> PerfCountersSnapshot {
    #[cfg(feature = "perf-counters")]
    {
        PerfCountersSnapshot { counts: Some(counting::counts()), virtual_calls: counting::virtual_calls() }
    }
    #[cfg(not(feature = "perf-counters"))]
    {
        PerfCountersSnapshot {}
    }
}

/// Sets every counter of this thread to zero.
pub fn reset() {
    #[cfg(feature = "perf-counters")]
    counting::reset();
}

impl PerfCountersSnapshot {
    /// The count of `counter`.
    pub fn get(&self, counter: PerfCounter) -> u64 {
        #[cfg(feature = "perf-counters")]
        {
            self.counts.map_or(0, |counts| counts[counter as usize])
        }
        #[cfg(not(feature = "perf-counters"))]
        {
            let _ = counter;
            0
        }
    }

    /// The virtual calls by member (`"Class::member"`, the class that
    /// declares the member), the most called first.
    pub fn virtual_calls(&self) -> &[(&'static str, u64)] {
        #[cfg(feature = "perf-counters")]
        {
            &self.virtual_calls
        }
        #[cfg(not(feature = "perf-counters"))]
        {
            &[]
        }
    }

    /// The number of virtual calls, of every member.
    pub fn virtual_calls_total(&self) -> u64 {
        self.virtual_calls().iter().map(|(_, count)| *count).sum()
    }

    /// What was counted between `earlier` and this snapshot.
    pub fn since(&self, earlier: &PerfCountersSnapshot) -> PerfCountersSnapshot {
        #[cfg(feature = "perf-counters")]
        {
            let counts: [u64; COUNT] = std::array::from_fn(|index| {
                let counter = PerfCounter::ALL[index];
                self.get(counter).saturating_sub(earlier.get(counter))
            });
            let mut virtual_calls: Vec<(&'static str, u64)> = self
                .virtual_calls
                .iter()
                .map(|(member, count)| {
                    let before =
                        earlier.virtual_calls.iter().find(|(other, _)| other == member).map_or(0, |(_, count)| *count);
                    (*member, count.saturating_sub(before))
                })
                .filter(|(_, count)| *count > 0)
                .collect();
            sort_virtual_calls(&mut virtual_calls);
            PerfCountersSnapshot { counts: Some(counts), virtual_calls }
        }
        #[cfg(not(feature = "perf-counters"))]
        {
            let _ = earlier;
            PerfCountersSnapshot {}
        }
    }

    /// These counts and the ones of `other` together: the sum of what was
    /// counted over several stretches of work.
    pub fn plus(&self, other: &PerfCountersSnapshot) -> PerfCountersSnapshot {
        #[cfg(feature = "perf-counters")]
        {
            let counts: [u64; COUNT] = std::array::from_fn(|index| {
                let counter = PerfCounter::ALL[index];
                self.get(counter) + other.get(counter)
            });
            let mut virtual_calls = self.virtual_calls.clone();
            for (member, count) in &other.virtual_calls {
                match virtual_calls.iter_mut().find(|(existing, _)| *existing == *member) {
                    Some((_, total)) => *total += count,
                    None => virtual_calls.push((*member, *count)),
                }
            }
            sort_virtual_calls(&mut virtual_calls);
            PerfCountersSnapshot { counts: Some(counts), virtual_calls }
        }
        #[cfg(not(feature = "perf-counters"))]
        {
            let _ = other;
            PerfCountersSnapshot {}
        }
    }

    /// The counters, one a line; then the virtual calls by member.
    pub fn report(&self) -> String {
        self.format(None)
    }

    /// The counters with, next to each, the count divided by `divisor` (the
    /// rows recycled, the frames rendered): `unit` names what it counts.
    pub fn report_per(&self, divisor: u64, unit: &str) -> String {
        self.format(Some((divisor, unit)))
    }

    fn format(&self, per: Option<(u64, &str)>) -> String {
        if !ENABLED {
            return String::from("perf counters: not compiled in (the feature `perf-counters` is off)\n");
        }

        let line = |name: &str, count: u64| match per {
            Some((divisor, unit)) if divisor > 0 => {
                format!("  {name}: {count} ({:.2} per {unit})\n", count as f64 / divisor as f64)
            }
            _ => format!("  {name}: {count}\n"),
        };
        let mut text = String::new();
        for counter in PerfCounter::ALL {
            text.push_str(&line(counter.name(), self.get(*counter)));
        }
        text.push_str(&line("virtual calls", self.virtual_calls_total()));
        for (member, count) in self.virtual_calls() {
            text.push_str(&line(&format!("  {member}"), *count));
        }
        text
    }
}

impl fmt::Display for PerfCountersSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.report())
    }
}

// Without the feature the counting macros are constant expressions of the
// unit type: there is nothing to run.
#[cfg(not(feature = "perf-counters"))]
const _: () = {
    crate::perf_count!(PropertyChangesRaised);
    crate::perf_count_virtual!("FerroObject::on_property_changed")
};
