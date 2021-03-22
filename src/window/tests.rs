use crate::curve::Curve;
use crate::window::{Window, WindowDeltaResult};

#[test]
fn aggregate_windows() {
    // Example 1. from Definition 4.
    let w1 = Window { start: 2, end: 4 };
    let w2 = Window { start: 3, end: 6 };
    let w3 = Window { start: 2, end: 7 };

    assert_eq!(w1.aggregate(&w2), Some(w3))
}

#[test]
fn window_delta_a() {
    // Example from figure 3. Part a
    // Partially full filled demand with partially used supply

    let w_p = Window { start: 0, end: 5 };
    let w_q = Window { start: 3, end: 7 };

    let result = Window::delta(&w_p, &w_q);

    let expected = WindowDeltaResult {
        remaining_supply: Curve {
            windows: vec![Window { start: 0, end: 3 }],
        },
        overlap: Window { start: 3, end: 5 },
        remaining_demand: Window { start: 5, end: 7 },
    };

    assert_eq!(result, expected);
}

#[test]
fn window_delta_b() {
    // Example from figure 3. Part b
    // Fully full filled demand with partial used supply

    let w_p = Window { start: 2, end: 8 };
    let w_q = Window { start: 0, end: 4 };

    let result = Window::delta(&w_p, &w_q);

    let expected_remaining_supply = Curve {
        windows: vec![Window { start: 6, end: 8 }],
    };
    let expected_overlap = Window { start: 2, end: 6 };

    assert_eq!(result.remaining_supply, expected_remaining_supply);
    assert_eq!(result.overlap, expected_overlap);
    assert!(result.remaining_demand.is_empty())
}

#[test]
fn window_delta_c() {
    // Example from figure 3. Part c
    // Fully full filled demand with split remaining supply

    let w_p = Window { start: 0, end: 8 };
    let w_q = Window { start: 2, end: 6 };

    let result = Window::delta(&w_p, &w_q);

    let expected_remaining_supply = Curve {
        windows: vec![Window { start: 0, end: 2 }, Window { start: 6, end: 8 }],
    };

    let expected_overlap = Window { start: 2, end: 6 };

    assert_eq!(result.remaining_supply, expected_remaining_supply);
    assert_eq!(result.overlap, expected_overlap);
    assert!(result.remaining_demand.is_empty())
}

#[test]
fn window_delta_d() {
    // Example from figure 3. Part d
    // Partially full filled demand with fully used supply

    let w_p = Window { start: 2, end: 6 };
    let w_q = Window { start: 0, end: 8 };

    let result = Window::delta(&w_p, &w_q);

    let expected = WindowDeltaResult {
        remaining_demand: Window { start: 4, end: 8 },
        remaining_supply: Curve { windows: vec![] },
        overlap: Window { start: 2, end: 6 },
    };

    assert_eq!(result, expected);
}
