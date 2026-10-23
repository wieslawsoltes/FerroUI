use super::Coordinate;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StackedValue {
    pub start: f64,
    pub end: f64,
    pub total: f64,
    pub negative_start: f64,
    pub negative_end: f64,
    pub negative_total: f64,
    pub cumulative_start: f64,
    pub cumulative_end: f64,
    pub is_negative: bool,
}

impl StackedValue {
    pub fn share(self) -> f64 {
        if self.is_negative {
            (self.negative_end - self.negative_start) / self.negative_total
        } else {
            (self.end - self.start) / self.total
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stacker {
    positions: HashMap<usize, usize>,
    stacks: Vec<HashMap<u64, StackedValue>>,
    totals: HashMap<u64, (f64, f64)>,
    max_series_id: Option<usize>,
}

fn index_key(index: f64) -> u64 {
    // C# Double.Equals treats both zeros and all NaNs as equal dictionary keys.
    if index == 0.0 {
        0
    } else if index.is_nan() {
        f64::NAN.to_bits()
    } else {
        index.to_bits()
    }
}

impl Stacker {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn max_series_id(&self) -> Option<usize> {
        self.max_series_id
    }
    pub fn max_length(&self) -> usize {
        self.stacks.iter().map(HashMap::len).max().unwrap_or(0)
    }
    pub fn get_series_stack_position(&mut self, series_id: usize) -> usize {
        self.max_series_id = Some(
            self.max_series_id
                .map_or(series_id, |old| old.max(series_id)),
        );
        if let Some(&position) = self.positions.get(&series_id) {
            return position;
        }
        let position = self.stacks.len();
        self.stacks.push(HashMap::new());
        self.positions.insert(series_id, position);
        position
    }

    /// Appends to positive/negative bar tracks and the signed area track.
    /// An unregistered stack position is rejected without indexing a missing row.
    pub fn stack_point(&mut self, point: Coordinate, position: usize) -> Option<f64> {
        if position >= self.stacks.len() {
            return None;
        }
        let key = index_key(point.secondary_value);
        let previous = (0..position)
            .rev()
            .find_map(|p| self.stacks[p].get(&key).copied())
            .unwrap_or_default();
        let current = self.stacks[position].entry(key).or_insert(StackedValue {
            start: previous.end,
            end: previous.end,
            negative_start: previous.negative_end,
            negative_end: previous.negative_end,
            cumulative_start: previous.cumulative_end,
            cumulative_end: previous.cumulative_end,
            ..StackedValue::default()
        });
        current.cumulative_end += point.primary_value;
        let total = self.totals.entry(key).or_default();
        if point.primary_value >= 0.0 {
            current.end += point.primary_value;
            total.0 += point.primary_value;
            Some(total.0)
        } else {
            current.negative_end += point.primary_value;
            total.1 += point.primary_value;
            Some(total.1)
        }
    }

    pub fn get_stack(&self, point: Coordinate, position: usize) -> StackedValue {
        let key = index_key(point.secondary_value);
        let mut result = self
            .stacks
            .get(position)
            .and_then(|row| row.get(&key))
            .copied()
            .unwrap_or_default();
        let total = self.totals.get(&key).copied().unwrap_or_default();
        result.total = total.0;
        result.negative_total = total.1;
        result.is_negative = point.primary_value < 0.0;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positive_and_negative_bars_share_separate_totals() {
        let mut s = Stacker::new();
        for id in 0..4 {
            assert_eq!(s.get_series_stack_position(id), id);
        }
        for (i, y) in [5.0, -3.0, 2.0, -4.0].into_iter().enumerate() {
            s.stack_point(Coordinate::new(0.0, y), i).unwrap();
        }
        let p = s.get_stack(Coordinate::new(0.0, 2.0), 2);
        assert_eq!((p.start, p.end, p.total), (5.0, 7.0, 7.0));
        assert_eq!((p.cumulative_start, p.cumulative_end), (2.0, 4.0));
        let n = s.get_stack(Coordinate::new(0.0, -4.0), 3);
        assert_eq!(
            (n.negative_start, n.negative_end, n.negative_total),
            (-3.0, -7.0, -7.0)
        );
        assert!((n.share() - 4.0 / 7.0).abs() < 1e-12);
    }
    #[test]
    fn missing_categories_find_previous_active_stack() {
        let mut s = Stacker::new();
        for id in [8, 3, 12] {
            s.get_series_stack_position(id);
        }
        s.stack_point(Coordinate::new(1.0, 5.0), 0);
        s.stack_point(Coordinate::new(2.0, 9.0), 1);
        s.stack_point(Coordinate::new(1.0, 4.0), 2);
        assert_eq!(s.get_stack(Coordinate::new(1.0, 4.0), 2).start, 5.0);
        assert_eq!(s.max_series_id(), Some(12));
    }
    #[test]
    fn signed_zero_coordinates_share_the_same_stack() {
        let mut s = Stacker::new();
        s.get_series_stack_position(0);
        s.get_series_stack_position(1);
        s.stack_point(Coordinate::new(-0.0, 2.0), 0);
        s.stack_point(Coordinate::new(0.0, 3.0), 1);
        assert_eq!(s.get_stack(Coordinate::new(0.0, 3.0), 1).end, 5.0);
        assert_eq!(s.stack_point(Coordinate::new(0.0, 1.0), 99), None);
    }
}
