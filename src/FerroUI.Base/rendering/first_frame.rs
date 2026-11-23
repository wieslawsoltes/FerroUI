//! What a GPU backend does ahead of the first frame of a window.
//!
//! Not from upstream. A GPU backend makes the pipeline (the program) of a kind of draw when a
//! frame first draws it, and on a first start of an application, with no cache of the system to
//! take it from, that is a compile: the first frame waits for every pipeline it needs
//! (docs/porting/desktop-performance.md, sections 9.81 and 9.83). What can be done before the
//! first frame is chosen here, for every backend and platform, so that an application and a
//! measurement have one switch.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

/// What is done about the pipelines of the first frame before it is drawn.
///
/// `FERROUI_FIRST_FRAME` (`wait` or `prewarm`) decides for the process when it is set; else
/// what [`set`](Self::set) was given; else [`Prewarm`](Self::Prewarm).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FirstFramePipelines {
    /// Nothing: the render context is made by the first frame, and a pipeline when a frame first
    /// draws with it.
    Wait,
    /// The platform makes the render context when it starts, and a backend that has records of
    /// pipelines (its own, and those an application registered) asks for them at once, on its
    /// threads, while the application builds its first screen.
    #[default]
    Prewarm,
}

/// What [`FirstFramePipelines::set`] was given: 0 for nothing, else the variant and one.
static CHOSEN: AtomicU8 = AtomicU8::new(0);

impl FirstFramePipelines {
    /// What the process does: see the type.
    pub fn current() -> Self {
        static FROM_ENVIRONMENT: OnceLock<Option<FirstFramePipelines>> = OnceLock::new();
        let from_environment = FROM_ENVIRONMENT.get_or_init(|| {
            let text = std::env::var("FERROUI_FIRST_FRAME").ok()?;
            Self::parse(&text)
        });
        from_environment.unwrap_or(match CHOSEN.load(Ordering::Relaxed) {
            1 => Self::Wait,
            _ => Self::Prewarm,
        })
    }

    /// Chooses for the process, before the platform starts (an application, in its `main`). The
    /// environment still decides when it says something.
    pub fn set(value: Self) {
        CHOSEN.store(
            match value {
                Self::Wait => 1,
                Self::Prewarm => 2,
            },
            Ordering::Relaxed,
        );
    }

    /// The value `text` names (`wait`, `prewarm`; case and surrounding space do not matter).
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "wait" => Some(Self::Wait),
            "prewarm" => Some(Self::Prewarm),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FirstFramePipelines;

    #[test]
    fn the_names_of_the_environment_are_read_whatever_their_case() {
        assert_eq!(FirstFramePipelines::parse("wait"), Some(FirstFramePipelines::Wait));
        assert_eq!(FirstFramePipelines::parse(" Prewarm\n"), Some(FirstFramePipelines::Prewarm));
        assert_eq!(FirstFramePipelines::parse("1"), None);
        assert_eq!(FirstFramePipelines::default(), FirstFramePipelines::Prewarm);
    }
}
