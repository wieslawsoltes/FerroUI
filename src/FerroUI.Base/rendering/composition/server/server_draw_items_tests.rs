//! When the compositor has a fragment made of a drawing and when it draws one (`draw_kept_or`): the policy
//! every backend shares, with a context that says what it wants ([`FragmentKeeping`]).

use super::{draw_kept_or, KeptFragment, LONGEST_WAIT};
use crate::platform::FragmentKeeping;
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};

/// A context that keeps fragments as `keeping` says, the log it and the drawing write to, and what the
/// compositor keeps of one drawing.
struct Drawing {
    context: MockDrawingContextImpl,
    log: DrawingLog,
    kept: KeptFragment,
}

impl Drawing {
    fn new(keeping: Option<FragmentKeeping>) -> Self {
        let log = DrawingLog::new();
        let mut context = MockDrawingContextImpl::new(log.clone());
        context.fragment_keeping = keeping;
        Self { context, log, kept: KeptFragment::default() }
    }

    /// Draws the drawing once; what the context was asked for, in order.
    fn draw(&mut self) -> Vec<String> {
        let log = self.log.clone();
        draw_kept_or(&self.kept, &mut self.context, |_| log.push("Commands"));
        self.log.take()
    }

    /// How many times the drawing is drawn from its commands alone before a fragment is made of it.
    fn draws_before_a_fragment(&mut self) -> usize {
        let mut draws = 0;
        while self.draw() == ["Commands"] {
            draws += 1;
            assert!(draws <= 1_000, "no fragment is made");
        }
        draws
    }
}

const MADE: [&str; 3] = ["BeginFragment", "Commands", "EndFragment"];

#[test]
fn a_context_without_fragments_draws_the_commands() {
    let mut drawing = Drawing::new(None);
    for _ in 0..3 {
        assert_eq!(drawing.draw(), ["Commands"]);
    }
    assert!(!drawing.kept.is_kept());
}

#[test]
fn a_fragment_is_made_with_every_drawing_that_has_none() {
    let mut drawing = Drawing::new(Some(FragmentKeeping::Each));
    assert_eq!(drawing.draw(), MADE);
    assert_eq!(drawing.draw(), ["DrawFragment"]);
    assert_eq!(drawing.draw(), ["DrawFragment"]);
    // One the context refuses is made again at once.
    drawing.context.refuses_fragments = true;
    assert_eq!(drawing.draw(), ["RefuseFragment", "BeginFragment", "Commands", "EndFragment"]);
    drawing.context.refuses_fragments = false;
    assert_eq!(drawing.draw(), ["DrawFragment"]);
    // Another drawing.
    drawing.kept.clear();
    assert_eq!(drawing.draw(), MADE);
}

#[test]
fn a_settled_drawing_gets_its_fragment_when_it_is_drawn_again() {
    let mut drawing = Drawing::new(Some(FragmentKeeping::Settled));
    assert_eq!(drawing.draw(), ["Commands"]);
    assert!(!drawing.kept.is_kept());
    assert_eq!(drawing.draw(), MADE);
    assert!(drawing.kept.is_kept());
    assert_eq!(drawing.draw(), ["DrawFragment"]);
    // A drawing that changes with every frame never gets one.
    for _ in 0..5 {
        drawing.kept.clear();
        assert_eq!(drawing.draw(), ["Commands"]);
    }
    assert_eq!(drawing.draw(), MADE);
}

#[test]
fn a_refused_fragment_is_made_again_after_a_wait_that_doubles() {
    let mut drawing = Drawing::new(Some(FragmentKeeping::Settled));
    assert_eq!(drawing.draws_before_a_fragment(), 1);
    let mut waits = Vec::new();
    for _ in 0..8 {
        drawing.context.refuses_fragments = true;
        assert_eq!(drawing.draw(), ["RefuseFragment", "Commands"]);
        assert!(!drawing.kept.is_kept());
        drawing.context.refuses_fragments = false;
        waits.push(drawing.draws_before_a_fragment());
    }
    assert_eq!(waits, [2, 4, 8, 16, 32, 64, 64, 64]);
    assert_eq!(usize::from(LONGEST_WAIT), 64);
    // Another drawing starts again.
    drawing.kept.clear();
    assert_eq!(drawing.draws_before_a_fragment(), 1);
    assert_eq!(drawing.draw(), ["DrawFragment"]);
}

#[test]
fn a_drawing_the_context_makes_no_fragment_of_is_not_asked_for_again() {
    let mut drawing = Drawing::new(Some(FragmentKeeping::Settled));
    drawing.context.makes_no_fragments = true;
    assert_eq!(drawing.draw(), ["Commands"]);
    assert_eq!(drawing.draw(), MADE);
    for _ in 0..300 {
        assert_eq!(drawing.draw(), ["Commands"]);
    }
    // Until it is another drawing.
    drawing.kept.clear();
    drawing.context.makes_no_fragments = false;
    assert_eq!(drawing.draw(), ["Commands"]);
    assert_eq!(drawing.draw(), MADE);
    assert_eq!(drawing.draw(), ["DrawFragment"]);
}

#[test]
fn a_context_that_makes_none_now_draws_and_keeps_what_was_made() {
    let mut drawing = Drawing::new(Some(FragmentKeeping::Each));
    assert_eq!(drawing.draw(), MADE);
    drawing.context.fragment_keeping = Some(FragmentKeeping::None);
    assert_eq!(drawing.draw(), ["DrawFragment"]);
    // What it refuses now it may draw in a later frame.
    drawing.context.refuses_fragments = true;
    assert_eq!(drawing.draw(), ["RefuseFragment", "Commands"]);
    assert!(drawing.kept.is_kept());
    drawing.context.refuses_fragments = false;
    assert_eq!(drawing.draw(), ["DrawFragment"]);
    // And it makes none of a drawing without one.
    drawing.kept.clear();
    assert_eq!(drawing.draw(), ["Commands"]);
    assert!(!drawing.kept.is_kept());
}
