use crate::curve::Curve;

// Definition 1.
// Not Copy to prevent accidental errors due to implicit copy
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Window {
    pub start: usize,
    pub end: usize,
}

impl Window {
    pub fn new(start: usize, end: usize) -> Self {
        Window { start, end }
    }

    pub fn empty() -> Self {
        Window { start: 0, end: 0 }
    }

    // Ω Definition 2.
    pub fn overlaps(&self, other: &Self) -> bool {
        return !(self.end < other.start || other.end < self.start);
    }

    // ⊕ Definition 4.
    pub fn aggregate(&self, other: &Self) -> Option<Self> {
        self.overlaps(other).then(|| {
            let start = usize::min(self.start, other.start);
            let end = start + self.length() + other.length();
            Window { start, end }
        })
    }

    // Definition 1.
    pub fn length(&self) -> usize {
        if self.end > self.start {
            self.end - self.start
        } else {
            0
        }
    }

    pub fn is_empty(&self) -> bool {
        self.length() == 0
    }

    pub fn delta(supply: &Self, demand: &Self) -> WindowDeltaResult {
        if supply.end < demand.start {
            WindowDeltaResult {
                remaining_supply: Curve::new(supply.clone()),
                overlap: Window::empty(),
                remaining_demand: demand.clone(),
            }
        } else {
            let overlap_start = usize::max(supply.start, demand.start);
            let overlap_end =
                overlap_start + usize::min(demand.length(), supply.end - overlap_start);
            let overlap = Window {
                start: overlap_start,
                end: overlap_end,
            };

            let remaining_demand = Window {
                start: demand.start + overlap.length(),
                end: demand.end,
            };

            let remaining_head_demand = Window {
                start: supply.start,
                end: overlap.start,
            };
            let remaining_tail_demand = Window {
                start: overlap.end,
                end: supply.end,
            };

            WindowDeltaResult {
                remaining_demand,
                remaining_supply: Curve::new(remaining_head_demand)
                    .aggregate(&Curve::new(remaining_tail_demand)),
                overlap,
            }
        }
    }
}

// deriving Eq for testing
#[derive(Debug, Eq, PartialEq)]
pub struct WindowDeltaResult {
    pub remaining_supply: Curve,
    pub overlap: Window,
    pub remaining_demand: Window,
}

#[cfg(test)]
mod tests;
