//! Value ranges used by visual text highlighters.
//! Source: WinUI TextRange.cpp and TextRangeCollection.cpp; see NOTICE.md.

use ferroui_base::collections::{FerroList, NotifyCollectionChangedEventArgs};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{FormatError, SpanStringTokenizer};
use std::cell::Cell;
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

/// A highlight operation could not be represented by the WinUI value contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextHighlightError {
    NegativeRange,
    ReversedRange,
    RangeOverflow,
    NonSolidColorBrush,
}

impl fmt::Display for TextHighlightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NegativeRange => "A highlight start or length is negative.",
            Self::ReversedRange => "A highlight range ends before it starts.",
            Self::RangeOverflow => {
                "A highlight range exceeds the signed 32-bit text coordinate space."
            }
            Self::NonSolidColorBrush => "Text highlighters require solid color brushes.",
        })
    }
}
impl std::error::Error for TextHighlightError {}

/// A copied start/length value in UTF-16 coordinates.
///
/// The signed fields preserve the source value contract. Invalid values can be
/// represented (including in markup), but renderers ignore them. Rust ranges
/// use half-open endpoints. This type is called TextRange in markup; its Rust
/// name distinguishes it from the document's live pointer range.
///
/// Cloning copies both fields. Interior mutability exists only so markup can
/// initialize a boxed value; collections always copy values on insertion/read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextHighlightRange {
    start_index: Cell<i32>,
    length: Cell<i32>,
}

impl TextHighlightRange {
    pub fn new(start_index: i32, length: i32) -> Self {
        Self {
            start_index: Cell::new(start_index),
            length: Cell::new(length),
        }
    }

    pub fn start_index(&self) -> i32 {
        self.start_index.get()
    }
    pub fn set_start_index(&self, value: i32) {
        self.start_index.set(value);
    }
    pub fn length(&self) -> i32 {
        self.length.get()
    }
    pub fn set_length(&self, value: i32) {
        self.length.set(value);
    }

    /// Converts a half-open UTF-16 range without truncating either endpoint.
    pub fn from_range(range: Range<usize>) -> Result<Self, TextHighlightError> {
        if range.end < range.start {
            return Err(TextHighlightError::ReversedRange);
        }
        let start = i32::try_from(range.start).map_err(|_| TextHighlightError::RangeOverflow)?;
        let end = i32::try_from(range.end).map_err(|_| TextHighlightError::RangeOverflow)?;
        Ok(Self::new(start, end - start))
    }

    pub fn to_range(&self) -> Result<Range<usize>, TextHighlightError> {
        let start = self.start_index();
        let length = self.length();
        if start < 0 || length < 0 {
            return Err(TextHighlightError::NegativeRange);
        }
        let end = start
            .checked_add(length)
            .ok_or(TextHighlightError::RangeOverflow)?;
        Ok(start as usize..end as usize)
    }

    pub fn range(&self) -> Option<Range<usize>> {
        self.to_range().ok()
    }

    pub fn parse(text: &str) -> Result<Self, FormatError> {
        SpanStringTokenizer::with_message(
            text,
            "A text highlight range requires a start and length.",
        )
        .scope(|tokens| Ok(Self::new(tokens.read_int32()?, tokens.read_int32()?)))
    }

    /// Markup setters must receive the same box while initializing the value.
    /// Registering this carrier does not change the collection's copied values.
    #[doc(hidden)]
    pub fn for_markup(start_index: i32, length: i32) -> Rc<Self> {
        ValueTypes::register_reference::<TextHighlightRange>();
        Rc::new(Self::new(start_index, length))
    }

    #[doc(hidden)]
    pub fn parse_for_markup(text: &str) -> Result<Rc<Self>, FormatError> {
        let value = Self::parse(text)?;
        Ok(Self::for_markup(value.start_index(), value.length()))
    }
}

impl TryFrom<Range<usize>> for TextHighlightRange {
    type Error = TextHighlightError;
    fn try_from(value: Range<usize>) -> Result<Self, Self::Error> {
        Self::from_range(value)
    }
}
impl fmt::Display for TextHighlightRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.start_index(), self.length())
    }
}

ferroui_base::ferro_markup_type!(struct TextHighlightRange as "TextRange" {
    namespace: "FerroUI.Controls.Documents",
    handles: [TextHighlightRange, Rc<TextHighlightRange>],
    this: Rc<TextHighlightRange>,
    parse: TextHighlightRange::parse_for_markup,
    constructors: [
        () => || TextHighlightRange::for_markup(0, 0),
        (i32, i32) => TextHighlightRange::for_markup,
    ],
    properties: [
        StartIndex: i32 { get: TextHighlightRange::start_index, set: TextHighlightRange::set_start_index },
        Length: i32 { get: TextHighlightRange::length, set: TextHighlightRange::set_length },
    ],
});

/// An observable shared collection of copied highlight values.
///
/// Unlike a live document range, an item does not follow edits. Applications
/// replace range values when their search results or annotations change.
#[derive(Clone)]
pub struct TextRangeCollection(Rc<FerroList<TextHighlightRange>>);

impl Default for TextRangeCollection {
    fn default() -> Self {
        Self::new()
    }
}
impl PartialEq for TextRangeCollection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl fmt::Debug for TextRangeCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.snapshot().iter()).finish()
    }
}
impl TextRangeCollection {
    pub fn new() -> Self {
        Self(Rc::new(FerroList::new()))
    }
    pub fn from_items(items: impl IntoIterator<Item = TextHighlightRange>) -> Self {
        Self(Rc::new(FerroList::from_items(items)))
    }
    pub fn count(&self) -> usize {
        self.0.count()
    }
    pub fn len(&self) -> usize {
        self.count()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
    pub fn clear(&self) {
        self.0.clear();
    }
    pub fn add(&self, range: TextHighlightRange) {
        self.0.add(range);
    }
    pub fn add_range(&self, ranges: impl IntoIterator<Item = TextHighlightRange>) {
        self.0.add_range(ranges);
    }
    pub fn add_span(&self, range: Range<usize>) -> Result<(), TextHighlightError> {
        self.add(TextHighlightRange::from_range(range)?);
        Ok(())
    }

    /// Like the source collection, inserting past the end appends.
    pub fn insert(&self, index: usize, range: TextHighlightRange) {
        self.0.insert(index.min(self.count()), range);
    }
    pub fn insert_range(&self, index: usize, ranges: impl IntoIterator<Item = TextHighlightRange>) {
        self.0.insert_range(index.min(self.count()), ranges);
    }
    pub fn get(&self, index: usize) -> Option<TextHighlightRange> {
        self.0.try_get(index)
    }
    pub fn set(&self, index: usize, range: TextHighlightRange) -> bool {
        if index >= self.count() {
            return false;
        }
        self.0.set(index, range);
        true
    }
    pub fn remove_at(&self, index: usize) -> Option<TextHighlightRange> {
        let old = self.get(index)?;
        self.0.remove_at(index);
        Some(old)
    }
    pub fn remove(&self, range: &TextHighlightRange) -> bool {
        self.0.remove(range)
    }
    pub fn contains(&self, range: &TextHighlightRange) -> bool {
        self.0.contains(range)
    }
    pub fn index_of(&self, range: &TextHighlightRange) -> Option<usize> {
        self.0.index_of(range)
    }

    /// Copies every value so setters on a returned item cannot mutate the list.
    pub fn snapshot(&self) -> Rc<Vec<TextHighlightRange>> {
        Rc::new(self.0.to_vec())
    }
    pub fn to_vec(&self) -> Vec<TextHighlightRange> {
        self.0.to_vec()
    }
    pub fn iter(&self) -> std::vec::IntoIter<TextHighlightRange> {
        self.to_vec().into_iter()
    }
    pub fn for_each(&self, mut f: impl FnMut(&TextHighlightRange)) {
        for range in self.snapshot().iter() {
            f(range);
        }
    }

    pub fn collection_changed(
        &self,
        handler: impl for<'a, 'b> Fn(&'a NotifyCollectionChangedEventArgs<'b, TextHighlightRange>)
        + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.0.add_collection_changed(Rc::new(handler));
        let weak = Rc::downgrade(&self.0);
        Disposable::create(move || {
            if let Some(list) = weak.upgrade() {
                list.remove_collection_changed(token);
            }
        })
    }
}

ferroui_base::ferro_markup_type!(class TextRangeCollection {
    namespace: "FerroUI.Controls.Documents",
    handles: [TextRangeCollection, Option<TextRangeCollection>],
    base: FerroList<TextHighlightRange>,
    constructors: [() => TextRangeCollection::new],
    methods: [fn Add(TextHighlightRange) => TextRangeCollection::add],
});
