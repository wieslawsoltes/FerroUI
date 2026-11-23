//! `ProgressRing` on the visuals WinUI ships for it. The control stays in `ferroui-controls`, which
//! draws an arc; an application that registers this crate gets a player in the template of each
//! ring instead, driven as `ProgressRing.cpp` drives its `LottiePlayer`.

use crate::{AnimatedVisualPlayer, AnimatedVisualSource, LottieVisualSource};
use ferroui_base::media::{Color, IBrush, ISolidColorBrush, Stretch};
use ferroui_base::{FerroPropertyChangedEventArgs, Ref, WeakRef};
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControl};
use ferroui_controls::{Control, Panel, ProgressRing};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const INDETERMINATE: &str = include_str!("icons/ProgressRingIndeterminate.json");
const DETERMINATE: &str = include_str!("icons/ProgressRingDeterminate.json");

/// The player of a ring and what it shows.
struct RingHost {
    ring: WeakRef<ProgressRing>,
    player: Ref<AnimatedVisualPlayer>,
    indeterminate: Ref<LottieVisualSource>,
    determinate: Ref<LottieVisualSource>,
    /// The progress the last change started from (`m_oldValue`).
    old_progress: Cell<f64>,
    token: Cell<u64>,
}

ferroui_base::ferro_thread_local! {
    static HOSTS: RefCell<Vec<Rc<RingHost>>> = RefCell::new(Vec::new());
}

/// From now on, a ring whose template is applied shows the WinUI visual.
pub(crate) fn install() {
    TemplatedControl::template_applied_event()
        .add_class_handler::<ProgressRing>(|ring, e: &TemplateAppliedEventArgs| attach(ring, e))
        .detach();
}

fn attach(ring: &ProgressRing, e: &TemplateAppliedEventArgs) {
    let ring = ring.to_ref();
    // A new template replaces the player of the old one; a ring that is gone leaves.
    HOSTS.with(|hosts| {
        hosts.borrow_mut().retain(|host| match host.ring.upgrade() {
            Some(owner) if owner.ptr_eq(&ring) => {
                owner.remove_property_changed_handler(host.token.get());
                false
            }
            Some(_) => true,
            None => false,
        });
    });
    let Some(root) = e.name_scope().find_as::<Panel>("LayoutRoot") else {
        return;
    };
    // The arc of the template is hidden, and the ring stops moving it.
    if let Some(arc) = e.name_scope().find_as::<Control>("LottiePlayer") {
        arc.set_is_visible(false);
    }
    ring.set_shown_by_visual(true);
    let player = AnimatedVisualPlayer::new();
    player.set_auto_play(false);
    player.set_stretch(Stretch::Fill);
    root.children().add(player.clone().upcast::<Control>());
    let host = Rc::new(RingHost {
        ring: ring.downgrade(),
        player,
        indeterminate: LottieVisualSource::create_from_string(INDETERMINATE),
        determinate: LottieVisualSource::create_from_string(DETERMINATE),
        old_progress: Cell::new(ring.normalized_value()),
        token: Cell::new(0),
    });
    let weak = Rc::downgrade(&host);
    let token = ring.add_property_changed_handler(Rc::new(move |change: &FerroPropertyChangedEventArgs<'_>| {
        if matches!(
            change.property().name(),
            "IsActive" | "IsIndeterminate" | "Value" | "Minimum" | "Maximum" | "Foreground" | "Background"
        ) {
            if let Some(host) = weak.upgrade() {
                host.update();
            }
        }
    }));
    host.token.set(token);
    host.update();
    HOSTS.with(|hosts| hosts.borrow_mut().push(host));
}

fn solid(brush: Option<Rc<dyn IBrush>>) -> Option<Color> {
    brush.and_then(|brush| brush.as_solid_color_brush().map(ISolidColorBrush::color))
}

impl RingHost {
    fn update(&self) {
        let Some(ring) = self.ring.upgrade() else {
            return;
        };
        let indeterminate = ring.is_indeterminate();
        let source = if indeterminate { &self.indeterminate } else { &self.determinate };
        // `SetLottieForegroundColor` and `SetLottieBackgroundColor`.
        if let Some(color) = solid(ring.foreground()) {
            source.set_color_property("Foreground", color);
        }
        if let Some(color) = solid(ring.background()) {
            source.set_color_property("Background", color);
        }
        let wanted: Ref<AnimatedVisualSource> = source.clone().upcast();
        let changed = !self.player.source().is_some_and(|shown| shown.ptr_eq(&wanted));
        if changed {
            self.player.stop();
            self.player.set_source(Some(wanted));
        }
        let active = ring.is_active();
        self.player.set_is_visible(active);
        if !active {
            self.player.stop();
        } else if indeterminate {
            if !self.player.is_playing() {
                drop(self.player.play_async(0.0, 1.0, true));
            }
        } else {
            // `UpdateLottieProgress`: an increase plays, a decrease is shown at once.
            let to = ring.normalized_value();
            let from = self.old_progress.get();
            if changed {
                self.player.set_progress(to);
            } else if from < to {
                drop(self.player.play_async(from, to, false));
            } else if from > to {
                self.player.set_progress(to);
            }
            self.old_progress.set(to);
        }
    }
}

#[cfg(test)]
impl RingHost {
    fn of(ring: &Ref<ProgressRing>) -> Option<Rc<RingHost>> {
        HOSTS.with(|hosts| {
            hosts.borrow().iter().find(|host| host.ring.upgrade().is_some_and(|owner| owner.ptr_eq(ring))).cloned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AnimatedVisual;
    use ferroui_base::layout::ILayoutManager;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
    use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
    use ferroui_controls::{Grid, Window};

    fn ring() -> (Ref<ProgressRing>, Ref<Window>) {
        install();
        let ring = ProgressRing::new();
        ring.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let root = Grid::new();
            root.set_name(Some("LayoutRoot".to_string()));
            root.register_in_name_scope(&**scope).upcast()
        })));
        let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(100.0, 100.0));
        window.set_content(Some(Control::boxed(&ring)));
        window.show();
        window.layout_manager().execute_initial_layout_pass();
        (ring, window)
    }

    #[test]
    fn the_documents_of_the_ring_are_read_with_the_size_of_the_originals() {
        let indeterminate = AnimatedVisual::parse(INDETERMINATE).unwrap();
        let determinate = AnimatedVisual::parse(DETERMINATE).unwrap();
        assert_eq!((indeterminate.width, indeterminate.duration_seconds()), (80.0, 2.0));
        assert_eq!((determinate.width, determinate.duration_seconds()), (32.0, 2.0));
    }

    #[test]
    fn an_indeterminate_ring_loops_and_an_inactive_one_stops() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (ring, _window) = ring();
        let host = RingHost::of(&ring).expect("the template has a player");
        assert!(host.player.is_playing());
        ring.set_is_active(false);
        assert!(!host.player.is_playing());
        assert!(!host.player.is_visible());
        ring.set_is_active(true);
        assert!(host.player.is_playing());
    }

    #[test]
    fn a_determinate_ring_plays_an_increase_and_jumps_on_a_decrease() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (ring, _window) = ring();
        let host = RingHost::of(&ring).unwrap();
        ring.set_maximum(100.0);
        ring.set_is_indeterminate(false);
        assert!(host.player.source().unwrap().ptr_eq(&host.determinate.clone().upcast::<AnimatedVisualSource>()));
        ring.set_range_value(50.0);
        assert!(host.player.is_playing(), "an increase plays");
        ring.set_range_value(20.0);
        assert!(!host.player.is_playing());
        assert!((host.player.progress() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn the_colors_of_the_ring_reach_the_source() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (ring, _window) = ring();
        let host = RingHost::of(&ring).unwrap();
        ring.set_foreground(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(1, 2, 3)))));
        use crate::IAnimatedVisualSource;
        assert!(host.indeterminate.document().color_properties().contains(&("Foreground".to_owned(), Color::from_rgb(1, 2, 3))));
    }
}
