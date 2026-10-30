//! Observable visual range formatting from WinUI TextHighlighter.cpp.
//! Copyright (c) Microsoft Corporation. See NOTICE.md for source and license.

use super::{TextHighlightError, TextHighlightRange, TextRangeCollection};
use ferroui_base::media::{IBrush, subscribe_invalidated};
use ferroui_base::metadata::IAddChild;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Ref, StyledProperty, StyledPropertyOptions, ferro_class, instantiate,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Paints foreground and background over its ranges without editing text.
///
/// A null brush uses the view's TextControlHighlighterForeground or
/// TextControlHighlighterBackground theme resource. As in WinUI, only solid
/// color brushes are accepted. Mutable brushes and range changes invalidate all
/// observing collections; observers never retain this object.
#[repr(C)]
pub struct TextHighlighter {
    base: FerroObject,
    ranges: TextRangeCollection,
    range_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    brush_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    invalidated: Rc<HandlerList<dyn Fn()>>,
}

ferro_class!(TextHighlighter: FerroObject);
ferroui_base::ferro_class_info!(TextHighlighter {
    new: TextHighlighter::new,
    interfaces: [Rc<dyn IAddChild<TextHighlightRange>> => TextHighlighter::as_add_child],
    markup: {
        content: Ranges,
        properties: [Ranges: TextRangeCollection { get: TextHighlighter::ranges }],
    },
});

impl FerroObjectImpl for TextHighlighter {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        *this.range_subscription.borrow_mut() = Some(this.ranges.collection_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.invalidate();
            }
        }));
        this.refresh_brush_subscriptions();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::foreground_property().as_property()
            || change.property() == Self::background_property().as_property()
        {
            this.refresh_brush_subscriptions();
            this.invalidate();
        }
    }
}

ferroui_base::ferro_properties! { impl TextHighlighter {
    pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register_with::<TextHighlighter, _>(
            "Foreground", StyledPropertyOptions::new(None).validate(valid_brush))
    }
    pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register_with::<TextHighlighter, _>(
            "Background", StyledPropertyOptions::new(None).validate(valid_brush))
    }
} }

fn valid_brush(brush: &Option<Rc<dyn IBrush>>) -> bool {
    brush
        .as_ref()
        .is_none_or(|brush| brush.as_solid_color_brush().is_some())
}

impl TextHighlighter {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            ranges: TextRangeCollection::new(),
            range_subscription: RefCell::new(None),
            brush_subscriptions: RefCell::new(Vec::new()),
            invalidated: Rc::new(HandlerList::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn ranges(&self) -> TextRangeCollection {
        self.ranges.clone()
    }
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    /// Sets a solid brush, returning a validation error without changing the
    /// previous value when supplied a different brush kind.
    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) -> Result<(), TextHighlightError> {
        self.try_set_foreground(value)
    }
    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) -> Result<(), TextHighlightError> {
        self.try_set_background(value)
    }
    pub fn try_set_foreground(
        &self,
        value: Option<Rc<dyn IBrush>>,
    ) -> Result<(), TextHighlightError> {
        if !valid_brush(&value) {
            return Err(TextHighlightError::NonSolidColorBrush);
        }
        self.set_value(Self::foreground_property(), value);
        Ok(())
    }
    pub fn try_set_background(
        &self,
        value: Option<Rc<dyn IBrush>>,
    ) -> Result<(), TextHighlightError> {
        if !valid_brush(&value) {
            return Err(TextHighlightError::NonSolidColorBrush);
        }
        self.set_value(Self::background_property(), value);
        Ok(())
    }

    pub fn add_child(&self, range: TextHighlightRange) {
        self.ranges.add(range);
    }
    pub fn as_add_child(this: Ref<Self>) -> Rc<dyn IAddChild<TextHighlightRange>> {
        Rc::new(HighlighterAddChild(this))
    }

    pub fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.invalidated.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.invalidated);
        Disposable::create(move || {
            if let Some(handlers) = weak.upgrade() {
                handlers.remove(token);
            }
        })
    }

    fn invalidate(&self) {
        for (_, handler) in self.invalidated.snapshot().iter() {
            handler();
        }
    }

    fn refresh_brush_subscriptions(&self) {
        let old = std::mem::take(&mut *self.brush_subscriptions.borrow_mut());
        for subscription in old {
            subscription.dispose();
        }
        let mut subscriptions = Vec::new();
        let mut seen = HashSet::new();
        for brush in [self.foreground(), self.background()] {
            let Some(object) = brush.as_ref().and_then(|brush| brush.as_object()) else {
                continue;
            };
            if !seen.insert(object.to_ref()) {
                continue;
            }
            let weak = self.to_ref().downgrade();
            let changed: Rc<dyn Fn()> = Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.invalidate();
                }
            });
            if let Some(subscription) = subscribe_invalidated(&brush, changed) {
                subscriptions.push(subscription);
            }
        }
        *self.brush_subscriptions.borrow_mut() = subscriptions;
    }
}

impl Drop for TextHighlighter {
    fn drop(&mut self) {
        if let Some(subscription) = self.range_subscription.get_mut().take() {
            subscription.dispose();
        }
        for subscription in self.brush_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

struct HighlighterAddChild(Ref<TextHighlighter>);
impl IAddChild<TextHighlightRange> for HighlighterAddChild {
    fn add_child(&self, value: TextHighlightRange) {
        self.0.add_child(value);
    }
    fn reference_id(&self) -> usize {
        &*self.0 as *const TextHighlighter as usize
    }
}
