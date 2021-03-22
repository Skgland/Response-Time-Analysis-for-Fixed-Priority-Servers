use crate::window::Window;
use std::collections::VecDeque;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Curve {
    /// windows contains an ordered Set of non-overlapping windows
    pub(crate) windows: Vec<Window>,
}

pub struct CurveDeltaResult {
    remaining_supply: Curve,
    overlap: Curve,
    remaining_demand: Curve,
}

impl Curve {
    pub fn new(window: Window) -> Self {
        if window.is_empty() {
            // Empty windows can be ignored
            Curve::empty()
        } else {
            // A Curve with only a single has
            // the windows always ordered and non-overlapping
            Curve {
                windows: vec![window],
            }
        }
    }

    pub fn windows(&self) -> &[Window] {
        self.windows.as_slice()
    }

    pub fn empty() -> Self {
        Curve { windows: vec![] }
    }

    // Definition 3.
    pub fn capacity(&self) -> usize {
        self.windows.iter().map(|window| window.length()).sum()
    }

    // Definition 5.
    pub fn aggregate(&self, other: &Self) -> Self {
        let mut new = self.windows.clone();

        for mut window in other.windows.iter().cloned() {
            let mut index = 0;
            while index < new.len() {
                if let Some(aggregate) = new[index].aggregate(&window) {
                    // remove window that was aggregated
                    new.remove(index);
                    // replace window to be inserted by aggregated window
                    window = aggregate;
                } else {
                    index += 1;
                }
            }

            // find index where to insert new window
            let index = new
                .iter()
                .enumerate()
                .find(|(_, nw)| nw.start > window.end)
                .map(|(index, _)| index)
                .unwrap_or_else(|| new.len());

            new.insert(index, window);
        }

        for new in new[..].windows(2) {
            // assert new.is_sorted_by_key(|window|window.start) once is_sorted_by_key is stable
            match new {
                [prev, next] => {
                    // ordered and non-overlapping
                    debug_assert!(prev.end < next.start);
                    debug_assert!(!prev.overlaps(next));
                }
                _ => unreachable!(
                    "Iteration over slice windows of size 2, can't have other slice lengths10"
                ),
            }
        }

        Curve { windows: new }
    }

    pub fn is_empty(&self) -> bool {
        self.capacity() == 0
    }

    // Definition 7.
    pub fn delta(supply: &Self, demand: &Self) -> CurveDeltaResult {
        let mut demand: VecDeque<_> = demand.windows.iter().cloned().collect();
        let mut supply: VecDeque<_> = supply.windows.iter().cloned().collect();

        let mut overlap = Curve::empty();

        // get first demand window
        // if None we are done <=> Condition C^'_q(t) = {}
        'main: while let Some(demand_window) = demand.front() {
            // (1) find valid supply window
            let j = 'j: loop {
                for i in 0..supply.len() {
                    if demand_window.start < supply[i].end {
                        break 'j i;
                    }
                }
                // exhausted usable supply
                // either supply.len() == 0 <=> Condition C^'_p(t) = {}
                // or all remaining supply is before remaining demand,
                // this should not happen for schedulable scenarios
                break 'main;
            };

            let supply_window = supply
                .get(j)
                .expect("Index should have only be searched in bounds!");

            // (2) calculate delta
            let result = Window::delta(&supply_window, &demand_window);

            // (3) removed used supply and add remaining supply
            supply
                .remove(j)
                .expect("Index should have only be searched in bounds, and has yet to be removed!");

            result
                .remaining_supply
                .windows
                .into_iter()
                .rev()
                .filter(|w| !w.is_empty())
                .for_each(|window| supply.push_front(window));
            // (4) add overlap windows
            overlap = overlap.aggregate(&Curve::new(result.overlap));
            // (5) remove completed demand and add remaining demand
            demand.pop_front();
            if !result.remaining_demand.is_empty() {
                demand.push_front(result.remaining_demand)
            }
        }

        CurveDeltaResult {
            remaining_supply: Curve {
                windows: supply.into(),
            },
            overlap,
            remaining_demand: Curve {
                windows: demand.into(),
            },
        }
    }

    //TODO Definition 8. ff.
}

#[cfg(test)]
mod tests;
