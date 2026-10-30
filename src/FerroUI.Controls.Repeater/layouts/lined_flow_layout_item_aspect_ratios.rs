// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

/// A measured ratio and its confidence, increasing to sixteen observations.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ItemAspectRatio {
    pub aspect_ratio: f64,
    pub weight: i32,
}

struct ItemAspectRatioBlock {
    start: i32,
    values: [ItemAspectRatio; 64],
}
impl ItemAspectRatioBlock {
    fn new() -> Self {
        Self {
            start: -1,
            values: [ItemAspectRatio::default(); 64],
        }
    }
    fn clear(&mut self) {
        self.start = -1;
        self.values.fill(ItemAspectRatio::default());
    }
    fn includes(&self, index: i32) -> bool {
        self.start >= 0 && index >= self.start && index < self.start + 64
    }
}

/// Bounded, block-based ratio history. Recycling always prefers an empty
/// block, then the block whose midpoint is furthest from the new item.
pub struct LinedFlowLayoutItemAspectRatios {
    blocks: Vec<ItemAspectRatioBlock>,
}
impl Default for LinedFlowLayoutItemAspectRatios {
    fn default() -> Self {
        Self::new()
    }
}
impl LinedFlowLayoutItemAspectRatios {
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|block| block.start == -1)
    }
    pub fn has_lower_weight(&self, first: i32, last: i32, weight: i32) -> bool {
        self.blocks
            .iter()
            .filter(|block| block.start != -1)
            .any(|block| {
                block.values.iter().enumerate().any(|(i, value)| {
                    let index = block.start + i as i32;
                    index >= first && index <= last && value.weight > 0 && value.weight < weight
                })
            })
    }
    pub fn average_aspect_ratio(&self, first: i32, last: i32, outside_weight: i32) -> f64 {
        let mut ratios = 0.0;
        let mut weights = 0;
        for block in self.blocks.iter().filter(|block| block.start != -1) {
            for (i, value) in block.values.iter().enumerate() {
                let index = block.start + i as i32;
                if value.weight == outside_weight || (index >= first && index <= last) {
                    ratios += value.aspect_ratio * value.weight as f64;
                    weights += value.weight;
                }
            }
        }
        if ratios > 0.0 && weights > 0 {
            ratios / weights as f64
        } else {
            0.0
        }
    }
    pub fn get_at(&self, index: i32) -> ItemAspectRatio {
        self.blocks
            .iter()
            .find(|block| block.includes(index))
            .map_or(ItemAspectRatio::default(), |block| {
                block.values[(index - block.start) as usize]
            })
    }
    pub fn set_at(&mut self, index: i32, value: ItemAspectRatio) {
        assert!(index >= 0, "Aspect-ratio item index must be nonnegative");
        if let Some(block) = self.blocks.iter_mut().find(|block| block.includes(index)) {
            block.values[(index - block.start) as usize] = value;
            return;
        }
        if self.blocks.is_empty() {
            self.resize(1, index);
        }
        let block_index = self.empty_or_furthest(index);
        let block = &mut self.blocks[block_index];
        block.clear();
        block.start = index / 64 * 64;
        block.values[(index - block.start) as usize] = value;
    }
    pub fn clear(&mut self) {
        self.blocks.truncate(4);
        for block in &mut self.blocks {
            block.clear();
        }
    }
    pub fn resize(&mut self, size: usize, reference_index: i32) {
        let count = size.max(1).div_ceil(64);
        while self.blocks.len() < count {
            self.blocks.push(ItemAspectRatioBlock::new());
        }
        while self.blocks.len() > count {
            let index = self.empty_or_furthest(reference_index);
            self.blocks.remove(index);
        }
    }
    fn empty_or_furthest(&self, index: i32) -> usize {
        if let Some(empty) = self.blocks.iter().position(|block| block.start == -1) {
            return empty;
        }
        self.blocks
            .iter()
            .enumerate()
            .max_by_key(|(_, block)| (block.start as i64 + 31 - index as i64).abs())
            .map_or(0, |(i, _)| i)
    }
}
