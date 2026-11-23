//! The clock of a play (`AnimatedVisualPlayer.PlayAsync`) and the states of an icon
//! (`AnimatedIcon`), without a control: progress runs from 0 to 1.

use std::collections::VecDeque;

/// The clock of a player.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlayerClock {
    from: f64,
    span: f64,
    looped: bool,
    travelled: f64,
    pub(crate) progress: f64,
    playing: bool,
    paused: bool,
    last_ms: Option<f64>,
}

impl Default for PlayerClock {
    fn default() -> Self {
        Self { from: 0.0, span: 1.0, looped: false, travelled: 0.0, progress: 0.0, playing: false, paused: false, last_ms: None }
    }
}

impl PlayerClock {
    pub(crate) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(crate) fn is_running(&self) -> bool {
        self.playing && !self.paused
    }

    /// Starts a play (`PlayAsync`). A segment that ends before it starts goes through the end.
    pub(crate) fn play(&mut self, from: f64, to: f64, looped: bool) {
        let (mut from, mut to) = (from, to);
        // A segment from a progress to 0 is the same as to 1; one from 1 the same as from 0.
        if to == 0.0 && from > 0.0 {
            to = 1.0;
        }
        if to > 0.0 && from == 1.0 {
            from = 0.0;
        }
        let (from, to) = (from.clamp(0.0, 1.0), to.clamp(0.0, 1.0));
        *self = Self {
            from,
            span: if to >= from { to - from } else { to - from + 1.0 },
            looped,
            travelled: 0.0,
            progress: from,
            playing: true,
            paused: false,
            last_ms: None,
        };
    }

    pub(crate) fn pause(&mut self) {
        if self.playing {
            self.paused = true;
            self.last_ms = None;
        }
    }

    pub(crate) fn resume(&mut self) {
        if self.playing {
            self.paused = false;
            self.last_ms = None;
        }
    }

    /// Sets the progress and ends the play (`SetProgress`).
    pub(crate) fn set_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
        self.playing = false;
        self.paused = false;
        self.last_ms = None;
    }

    /// Goes back to where the play began and ends it (`Stop`). Without a play it does nothing.
    pub(crate) fn stop(&mut self) {
        if self.playing {
            self.set_progress(self.from);
        }
    }

    /// Moves the clock to `now_ms`, for an animation of `duration` seconds at `rate`. `true`
    /// when the play has just finished.
    pub(crate) fn advance(&mut self, now_ms: f64, duration: f64, rate: f64) -> bool {
        if !self.is_running() {
            return false;
        }
        let Some(last) = self.last_ms.replace(now_ms) else {
            return false;
        };
        if duration <= 0.0 || self.span <= 0.0 {
            if self.looped {
                return false;
            }
            self.progress = (self.from + self.span).min(1.0);
            self.playing = false;
            return true;
        }
        self.travelled += (now_ms - last).max(0.0) / 1000.0 / duration * rate.abs();
        if self.travelled >= self.span {
            if self.looped {
                self.travelled %= self.span;
            } else {
                self.travelled = self.span;
                self.progress = self.at(rate);
                self.playing = false;
                return true;
            }
        }
        self.progress = self.at(rate);
        false
    }

    /// The progress after `travelled`: from the start of the segment, or back from its end at a
    /// negative rate.
    fn at(&self, rate: f64) -> f64 {
        let along = if rate < 0.0 { self.span - self.travelled } else { self.travelled };
        let at = self.from + along;
        if at > 1.0 { at - 1.0 } else { at }
    }
}

/// What an icon does when its state changes (`AnimatedIcon::TransitionStates`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum IconMove {
    /// Plays from a progress, or from where it is, to a progress.
    Play { from: Option<f64>, to: f64, multiplier: f64 },
    /// Goes to a progress at once.
    Cut(f64),
}

/// How an icon takes a state that arrives while it is playing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimatedIconAnimationQueueBehavior {
    Cut,
    #[default]
    QueueOne,
    SpeedUpQueueOne,
}

const INFIX: &str = "To";
const START: &str = "_Start";
const END: &str = "_End";

/// The move from state `from` to state `to` for these markers.
pub(crate) fn transition(markers: &[(String, f64)], from: &str, to: &str, multiplier: f64) -> IconMove {
    let find = |name: &str| markers.iter().find(|(key, _)| key == name).map(|(_, value)| *value);
    let name = format!("{from}{INFIX}{to}");
    match (find(&format!("{name}{START}")), find(&format!("{name}{END}"))) {
        (Some(start), Some(end)) => return IconMove::Play { from: Some(start), to: end, multiplier },
        (None, Some(end)) => return IconMove::Cut(end),
        (Some(start), None) => return IconMove::Cut(start),
        (None, None) => {}
    }
    if let Some(progress) = find(&name).or_else(|| find(to)) {
        return IconMove::Cut(progress);
    }
    // No animation for this transition: one that ends in the same state gives the position.
    let fragment = format!("{INFIX}{to}{END}");
    if let Some((_, progress)) = markers.iter().find(|(key, _)| key.contains(&fragment)) {
        return IconMove::Cut(*progress);
    }
    // A state can also be a progress to animate to.
    match to.parse::<f64>() {
        Ok(progress) if progress.is_finite() => IconMove::Play { from: None, to: progress as f32 as f64, multiplier },
        _ => IconMove::Cut(0.0),
    }
}

/// The states of an icon and its queue.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct IconStates {
    pub(crate) behavior: AnimatedIconAnimationQueueBehavior,
    pub(crate) current: String,
    previous: String,
    queued: VecDeque<String>,
    queue_length: usize,
    speed_up: f64,
    is_speed_up: bool,
    pub(crate) is_playing: bool,
}

impl Default for IconStates {
    fn default() -> Self {
        Self {
            behavior: AnimatedIconAnimationQueueBehavior::default(),
            current: String::new(),
            previous: String::new(),
            queued: VecDeque::new(),
            queue_length: 4,
            speed_up: 7.0,
            is_speed_up: false,
            is_playing: false,
        }
    }
}

impl IconStates {
    fn go(&mut self, markers: &[(String, f64)], to: String, multiplier: f64) -> IconMove {
        let step = transition(markers, &self.current, &to, multiplier);
        self.previous = std::mem::replace(&mut self.current, to);
        self.queued.pop_front();
        // A cut leaves a segment that is playing as it is, as the original does.
        if matches!(step, IconMove::Play { .. }) {
            self.is_playing = true;
        }
        step
    }

    /// A state that arrived (`OnLayoutUpdatedAfterStateChanged`).
    pub(crate) fn request(&mut self, markers: &[(String, f64)], state: &str) -> Option<IconMove> {
        let state = state.to_owned();
        match self.behavior {
            AnimatedIconAnimationQueueBehavior::Cut => Some(self.go(markers, state, 1.0)),
            AnimatedIconAnimationQueueBehavior::QueueOne => {
                if !self.is_playing {
                    return Some(self.go(markers, state, 1.0));
                }
                let mut step = None;
                if self.queued.len() >= self.queue_length {
                    let next = self.queued.front().cloned().unwrap_or_default();
                    step = Some(self.go(markers, next, 1.0));
                }
                self.queued.push_back(state);
                step
            }
            AnimatedIconAnimationQueueBehavior::SpeedUpQueueOne => {
                if !self.is_playing {
                    return Some(self.go(markers, state, 1.0));
                }
                if self.queued.len() >= self.queue_length {
                    let next = self.queued.front().cloned().unwrap_or_default();
                    let step = self.go(markers, next, self.speed_up);
                    self.queued.push_back(state);
                    return Some(step);
                }
                self.queued.push_back(state);
                if self.is_speed_up {
                    return None;
                }
                self.is_speed_up = true;
                let end = format!("{}{INFIX}{}{END}", self.previous, self.current);
                markers
                    .iter()
                    .find(|(key, _)| *key == end)
                    .map(|(_, to)| IconMove::Play { from: None, to: *to, multiplier: self.speed_up })
            }
        }
    }

    /// The segment that was playing ended (`OnAnimationCompleted`).
    pub(crate) fn completed(&mut self, markers: &[(String, f64)]) -> Option<IconMove> {
        self.is_playing = false;
        let multiplier = match self.behavior {
            AnimatedIconAnimationQueueBehavior::Cut => return None,
            AnimatedIconAnimationQueueBehavior::QueueOne => 1.0,
            AnimatedIconAnimationQueueBehavior::SpeedUpQueueOne => {
                if self.queued.len() > 1 && self.is_speed_up {
                    self.speed_up
                } else {
                    1.0
                }
            }
        };
        if self.queued.is_empty() {
            return None;
        }
        let next = self.queued.front().cloned().unwrap_or_default();
        Some(self.go(markers, next, multiplier))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markers(names: &[(&str, f64)]) -> Vec<(String, f64)> {
        names.iter().map(|(name, progress)| ((*name).to_owned(), *progress)).collect()
    }

    #[test]
    fn a_play_runs_from_its_start_to_its_end_and_finishes() {
        let mut clock = PlayerClock::default();
        clock.play(0.25, 0.75, false);
        assert!(!clock.advance(0.0, 2.0, 1.0), "the first tick sets the clock");
        assert!(!clock.advance(500.0, 2.0, 1.0));
        assert!((clock.progress - 0.5).abs() < 1e-9);
        assert!(clock.advance(1000.0, 2.0, 1.0));
        assert_eq!(clock.progress, 0.75);
        assert!(!clock.is_playing());
    }

    #[test]
    fn a_looped_play_wraps_and_a_segment_can_go_through_the_end() {
        let mut clock = PlayerClock::default();
        clock.play(0.0, 1.0, true);
        clock.advance(0.0, 1.0, 1.0);
        assert!(!clock.advance(1250.0, 1.0, 1.0));
        assert!((clock.progress - 0.25).abs() < 1e-9);
        clock.play(0.75, 0.25, false);
        clock.advance(0.0, 1.0, 1.0);
        clock.advance(400.0, 1.0, 1.0);
        assert!((clock.progress - 0.15).abs() < 1e-9, "{}", clock.progress);
    }

    #[test]
    fn the_ends_of_a_segment_are_adjusted_as_the_player_does() {
        let mut clock = PlayerClock::default();
        clock.play(0.5, 0.0, false);
        clock.advance(0.0, 1.0, 1.0);
        assert!(clock.advance(600.0, 1.0, 1.0));
        assert_eq!(clock.progress, 1.0);
        clock.play(1.0, 0.5, false);
        assert_eq!(clock.progress, 0.0);
    }

    #[test]
    fn pause_resume_stop_and_set_progress() {
        let mut clock = PlayerClock::default();
        clock.play(0.2, 1.0, false);
        clock.advance(0.0, 1.0, 1.0);
        clock.advance(100.0, 1.0, 1.0);
        clock.pause();
        assert!(clock.is_playing() && !clock.is_running());
        let held = clock.progress;
        clock.advance(900.0, 1.0, 1.0);
        assert_eq!(clock.progress, held);
        clock.resume();
        clock.advance(1000.0, 1.0, 1.0);
        clock.advance(1100.0, 1.0, 1.0);
        assert!(clock.progress > held);
        clock.stop();
        assert_eq!((clock.progress, clock.is_playing()), (0.2, false));
        clock.set_progress(2.0);
        assert_eq!(clock.progress, 1.0);
    }

    #[test]
    fn a_negative_rate_plays_from_the_end() {
        let mut clock = PlayerClock::default();
        clock.play(0.0, 1.0, false);
        clock.advance(0.0, 1.0, -1.0);
        clock.advance(250.0, 1.0, -1.0);
        assert!((clock.progress - 0.75).abs() < 1e-9);
    }

    #[test]
    fn a_transition_takes_the_first_of_the_rules_that_applies() {
        let all = markers(&[("AToB_Start", 0.1), ("AToB_End", 0.2), ("CToD_End", 0.3), ("EToF_Start", 0.4), ("GToH", 0.5), ("Z", 0.6), ("QToR_End", 0.7)]);
        assert_eq!(transition(&all, "A", "B", 1.0), IconMove::Play { from: Some(0.1), to: 0.2, multiplier: 1.0 });
        assert_eq!(transition(&all, "C", "D", 1.0), IconMove::Cut(0.3));
        assert_eq!(transition(&all, "E", "F", 1.0), IconMove::Cut(0.4));
        assert_eq!(transition(&all, "G", "H", 1.0), IconMove::Cut(0.5));
        assert_eq!(transition(&all, "X", "Z", 1.0), IconMove::Cut(0.6));
        // Another transition that ends in the state gives the position.
        assert_eq!(transition(&all, "X", "R", 1.0), IconMove::Cut(0.7));
        assert_eq!(transition(&all, "X", "0.5", 2.0), IconMove::Play { from: None, to: 0.5, multiplier: 2.0 });
        assert_eq!(transition(&all, "X", "Nothing", 1.0), IconMove::Cut(0.0));
    }

    #[test]
    fn a_state_during_a_play_speeds_it_up_and_is_played_next() {
        let all = markers(&[("NToP_Start", 0.0), ("NToP_End", 0.2), ("PToQ_Start", 0.2), ("PToQ_End", 0.4)]);
        let mut states = IconStates {
            behavior: AnimatedIconAnimationQueueBehavior::SpeedUpQueueOne,
            current: "N".to_owned(),
            ..IconStates::default()
        };
        assert_eq!(states.request(&all, "P"), Some(IconMove::Play { from: Some(0.0), to: 0.2, multiplier: 1.0 }));
        assert!(states.is_playing);
        // The rest of the running segment at seven times the speed.
        assert_eq!(states.request(&all, "Q"), Some(IconMove::Play { from: None, to: 0.2, multiplier: 7.0 }));
        assert_eq!(states.completed(&all), Some(IconMove::Play { from: Some(0.2), to: 0.4, multiplier: 1.0 }));
        assert_eq!(states.current, "Q");
        assert_eq!(states.completed(&all), None);
    }

    #[test]
    fn by_default_a_state_during_a_play_waits_for_it() {
        let all = markers(&[("NToP_Start", 0.0), ("NToP_End", 0.2), ("PToQ_Start", 0.2), ("PToQ_End", 0.4)]);
        let mut states = IconStates { current: "N".to_owned(), ..IconStates::default() };
        assert_eq!(states.behavior, AnimatedIconAnimationQueueBehavior::QueueOne);
        states.request(&all, "P");
        assert_eq!(states.request(&all, "Q"), None);
        assert_eq!(states.completed(&all), Some(IconMove::Play { from: Some(0.2), to: 0.4, multiplier: 1.0 }));
    }

    #[test]
    fn the_cut_behavior_goes_to_each_state_at_once() {
        let all = markers(&[("AToB_Start", 0.0), ("AToB_End", 0.5), ("BToC_Start", 0.5), ("BToC_End", 1.0)]);
        let mut states = IconStates { behavior: AnimatedIconAnimationQueueBehavior::Cut, current: "A".to_owned(), ..IconStates::default() };
        states.request(&all, "B");
        assert_eq!(states.request(&all, "C"), Some(IconMove::Play { from: Some(0.5), to: 1.0, multiplier: 1.0 }));
    }
}
