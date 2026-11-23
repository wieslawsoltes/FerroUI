//! The sources of animated visuals (`IAnimatedVisualSource`, `IAnimatedVisualSource2`,
//! `IDynamicAnimatedVisualSource` and `LottieVisualSource`).

use crate::{AnimatedVisual, AnimatedVisualError};
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

/// Makes the animated visual a player or an icon shows.
pub trait IAnimatedVisualSource {
    fn try_create_animated_visual(&self) -> Result<Rc<AnimatedVisual>, AnimatedVisualError>;
}

/// A source with named positions, as progress from 0 to 1.
pub trait IAnimatedVisualSource2: IAnimatedVisualSource {
    fn markers(&self) -> Vec<(String, f64)> {
        self.try_create_animated_visual().map_or_else(|_| Vec::new(), |visual| visual.progress_markers())
    }
}

/// A source whose visual can change: a player makes it again when it says so.
pub trait IDynamicAnimatedVisualSource: IAnimatedVisualSource {
    fn add_animated_visual_invalidated(&self, handler: Rc<dyn Fn()>);
}

impl AnimatedVisual {
    /// The markers as progress from 0 to 1, in the order of the document.
    pub fn progress_markers(&self) -> Vec<(String, f64)> {
        let length = self.out_point - self.in_point;
        self.markers
            .iter()
            .map(|marker| {
                let progress = if length > 0.0 { (marker.frame - self.in_point) / length } else { 0.0 };
                (marker.name.clone(), progress.clamp(0.0, 1.0))
            })
            .collect()
    }
}

/// A source of a Lottie document. It is read once, when it is first asked for.
#[derive(Default)]
pub struct LottieVisualSource {
    json: RefCell<Option<Rc<str>>>,
    visual: RefCell<Option<Rc<AnimatedVisual>>>,
    invalidated: RefCell<Vec<Rc<dyn Fn()>>>,
}

impl LottieVisualSource {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// A source of the document `json`.
    pub fn create_from_string(json: &str) -> Rc<Self> {
        let source = Self::new();
        source.set_source(json);
        source
    }

    /// Reads the document from `stream` (`SetSourceAsync` of a stream).
    pub fn set_source_stream(&self, mut stream: impl Read) -> std::io::Result<()> {
        let mut json = String::new();
        stream.read_to_string(&mut json)?;
        self.set_source(&json);
        Ok(())
    }

    /// Replaces the document; players of this source make their visual again.
    pub fn set_source(&self, json: &str) {
        *self.json.borrow_mut() = Some(Rc::from(json));
        self.visual.borrow_mut().take();
        let handlers = self.invalidated.borrow().clone();
        for handler in handlers {
            handler();
        }
    }
}

impl IAnimatedVisualSource for LottieVisualSource {
    fn try_create_animated_visual(&self) -> Result<Rc<AnimatedVisual>, AnimatedVisualError> {
        if let Some(visual) = self.visual.borrow().as_ref() {
            return Ok(visual.clone());
        }
        let json = self.json.borrow().clone().ok_or(AnimatedVisualError::Invalid("source"))?;
        let visual = Rc::new(AnimatedVisual::parse(&json)?);
        *self.visual.borrow_mut() = Some(visual.clone());
        Ok(visual)
    }
}

impl IAnimatedVisualSource2 for LottieVisualSource {}

impl IDynamicAnimatedVisualSource for LottieVisualSource {
    fn add_animated_visual_invalidated(&self, handler: Rc<dyn Fn()>) {
        self.invalidated.borrow_mut().push(handler);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    const DOCUMENT: &str = r#"{"fr":30,"ip":0,"op":100,"w":10,"h":10,"layers":[],
        "markers":[{"cm":"Normal","tm":25,"dr":0},{"cm":"Pressed","tm":100,"dr":0}]}"#;

    #[test]
    fn a_source_reads_its_document_once_and_gives_markers_as_progress() {
        let source = LottieVisualSource::create_from_string(DOCUMENT);
        let first = source.try_create_animated_visual().unwrap();
        let second = source.try_create_animated_visual().unwrap();
        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(source.markers(), vec![("Normal".to_owned(), 0.25), ("Pressed".to_owned(), 1.0)]);
    }

    #[test]
    fn a_new_document_invalidates_the_visual() {
        let source = LottieVisualSource::create_from_string(DOCUMENT);
        let first = source.try_create_animated_visual().unwrap();
        let told = Rc::new(Cell::new(0));
        let seen = told.clone();
        source.add_animated_visual_invalidated(Rc::new(move || seen.set(seen.get() + 1)));
        source.set_source(DOCUMENT);
        assert_eq!(told.get(), 1);
        assert!(!Rc::ptr_eq(&first, &source.try_create_animated_visual().unwrap()));
    }

    #[test]
    fn an_empty_source_or_a_bad_document_is_an_error() {
        assert!(LottieVisualSource::new().try_create_animated_visual().is_err());
        assert!(LottieVisualSource::create_from_string("not json").try_create_animated_visual().is_err());
    }
}
