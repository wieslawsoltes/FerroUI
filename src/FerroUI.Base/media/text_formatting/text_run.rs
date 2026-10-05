use std::any::Any;
use std::rc::Rc;

use crate::media::text_formatting::{DrawableTextRun, TextEndOfLine, TextRunProperties};
use crate::utilities::ReadOnlyMemory;

/// The default length of a text run in the text source (one position).
pub const DEFAULT_TEXT_SOURCE_LENGTH: i32 = 1;

/// Represents a portion of a text line object.
///
/// Upstream `TextRun` is an abstract class whose subclasses are tested with
/// `is`/`as`. Here it is a trait: runs are handled as `Rc<dyn TextRun>` and
/// recovered with [`dyn TextRun::downcast_ref`] / [`dyn TextRun::downcast_rc`],
/// or with the `as_*` hooks for the open base classes (`DrawableTextRun`,
/// `TextEndOfLine`).
pub trait TextRun: 'static {
    /// Gets the text source length (UTF-16 code units).
    fn length(&self) -> i32 {
        DEFAULT_TEXT_SOURCE_LENGTH
    }

    /// Gets the text run's text.
    fn text(&self) -> ReadOnlyMemory<u16> {
        ReadOnlyMemory::empty()
    }

    /// Gets the text run's text as a slice. Runs with text override this
    /// together with [`TextRun::text`]; hot paths read the slice.
    fn text_span(&self) -> &[u16] {
        &[]
    }

    /// A set of properties shared by every character in the run.
    fn properties(&self) -> Option<&Rc<dyn TextRunProperties>> {
        None
    }

    /// The run as a drawable run (C# `run is DrawableTextRun`).
    fn as_drawable(&self) -> Option<&dyn DrawableTextRun> {
        None
    }

    /// The run as an end of line marker (C# `run is TextEndOfLine`).
    fn as_text_end_of_line(&self) -> Option<&TextEndOfLine> {
        None
    }

    /// The run as [`Any`], for downcasting to a concrete run type.
    fn as_any(&self) -> &dyn Any;

    /// The run handle as [`Any`], for downcasting to a concrete run handle.
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any>;
}

impl dyn TextRun {
    /// C# `run as T` for a concrete run type.
    #[inline]
    pub fn downcast_ref<T: TextRun>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    /// C# `run is T` for a concrete run type.
    #[inline]
    pub fn is<T: TextRun>(&self) -> bool {
        self.as_any().is::<T>()
    }

    /// C# `run as T` for a concrete run type, keeping the handle.
    pub fn downcast_rc<T: TextRun>(self: Rc<Self>) -> Option<Rc<T>> {
        if self.as_any().is::<T>() {
            self.into_any_rc().downcast::<T>().ok()
        } else {
            None
        }
    }
}

/// Implements the two `Any` accessors of [`TextRun`] inside an `impl TextRun for T` block.
#[macro_export]
#[doc(hidden)]
macro_rules! __text_run_any {
    () => {
        #[inline]
        fn as_any(&self) -> &dyn ::std::any::Any {
            self
        }

        #[inline]
        fn into_any_rc(self: ::std::rc::Rc<Self>) -> ::std::rc::Rc<dyn ::std::any::Any> {
            self
        }
    };
}

pub use crate::__text_run_any as text_run_any;
