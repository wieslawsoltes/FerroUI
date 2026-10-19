//! `DynamicData.Diagnostics`: statistics of the change sets of a sequence (`ChangeStatistics`,
//! `ChangeSummary`, `DiagnosticOperators.CollectUpdateStats`).

use std::time::SystemTime;

/// The counts of one change set, or of all of them so far (`ChangeStatistics`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeStatistics {
    /// The number of the change set, from 0; `None` before any (`Index`, `-1` upstream).
    pub index: Option<usize>,
    pub adds: usize,
    pub updates: usize,
    pub removes: usize,
    pub refreshes: usize,
    pub moves: usize,
    /// The number of changes (`Count`).
    pub count: usize,
    /// When the statistics were made (`LastUpdated`).
    pub last_updated: SystemTime,
}

impl Default for ChangeStatistics {
    fn default() -> Self {
        Self::new(None, 0, 0, 0, 0, 0, 0)
    }
}

impl ChangeStatistics {
    pub fn new(index: Option<usize>, adds: usize, updates: usize, removes: usize, refreshes: usize, moves: usize, count: usize) -> Self {
        Self { index, adds, updates, removes, refreshes, moves, count, last_updated: SystemTime::now() }
    }
}

/// The statistics of the latest change set and of all of them (`ChangeSummary`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangeSummary {
    index: Option<usize>,
    /// The latest change set (`Latest`).
    pub latest: ChangeStatistics,
    /// Every change set so far (`Overall`).
    pub overall: ChangeStatistics,
}

impl ChangeSummary {
    /// No change set yet (`ChangeSummary.Empty`).
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn new(index: Option<usize>, latest: ChangeStatistics, overall: ChangeStatistics) -> Self {
        Self { index, latest, overall }
    }

    /// The number of the latest change set.
    pub fn index(&self) -> Option<usize> {
        self.index
    }

    /// The summary after one more change set with these counts (the accumulator of
    /// `CollectUpdateStats`).
    pub(crate) fn next(&self, adds: usize, updates: usize, removes: usize, refreshes: usize, moves: usize, count: usize) -> Self {
        let index = self.overall.index.map_or(0, |index| index + 1);
        let overall = &self.overall;
        let latest = ChangeStatistics::new(Some(index), adds, updates, removes, refreshes, moves, count);
        let overall = ChangeStatistics::new(
            Some(index),
            overall.adds + adds,
            overall.updates + updates,
            overall.removes + removes,
            overall.refreshes + refreshes,
            overall.moves + moves,
            overall.count + count,
        );
        Self::new(Some(index), latest, overall)
    }
}
