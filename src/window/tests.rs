use crate::curve::Curve;
use crate::window::{Demand, Supply, Window, WindowDeltaResult};

#[test]
fn aggregate_windows() {
    // Example 1. from Definition 4.
    let w1 = Window::<Demand>::new(2, 4);
    let w2 = Window::<Demand>::new(3, 6);
    let w3 = Window::<Demand>::new(2, 7);

    assert_eq!(w1.aggregate(&w2), Some(w3))
}

#[test]
fn window_delta_a() {
    // Example from figure 3. Part a
    // Partially full filled demand with partially used supply

    let w_p = Window::<Supply>::new(0, 5);
    let w_q = Window::<Demand>::new(3, 7);

    let result = Window::delta(&w_p, &w_q);

    let expected = WindowDeltaResult {
        remaining_supply: Curve::new(Window::new(0, 3)),
        overlap: Window::new(3, 5),
        remaining_demand: Window::new(5, 7),
    };

    assert_eq!(result, expected);
}

#[test]
fn window_delta_b() {
    // Example from figure 3. Part b
    // Fully full filled demand with partial used supply

    let w_p = Window::<Supply>::new(2, 8);
    let w_q = Window::<Demand>::new(0, 4);

    let result = Window::delta(&w_p, &w_q);

    let expected_remaining_supply = Curve::new(Window::new(6, 8));
    let expected_overlap = Window::new(2, 6);

    assert_eq!(result.remaining_supply, expected_remaining_supply);
    assert_eq!(result.overlap, expected_overlap);
    assert!(
        result.remaining_demand.is_empty(),
        "Expected empty remaining demand, got: {:#?}",
        result.remaining_demand
    )
}

#[test]
fn window_delta_c() {
    // Example from figure 3. Part c
    // Fully full filled demand with split remaining supply

    let w_p = Window::<Supply>::new(0, 8);
    let w_q = Window::<Demand>::new(2, 6);

    let result = Window::delta(&w_p, &w_q);

    let expected_remaining_supply =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(0, 2), Window::new(6, 8)]) };

    let expected_overlap = Window::new(2, 6);

    assert_eq!(result.remaining_supply, expected_remaining_supply);
    assert_eq!(result.overlap, expected_overlap);
    assert!(
        result.remaining_demand.is_empty(),
        "Expected empty remaining demand, got: {:#?}",
        result.remaining_demand
    )
}

#[test]
fn window_delta_d() {
    // Example from figure 3. Part d
    // Partially full filled demand with fully used supply

    let w_p = Window::<Supply>::new(2, 6);
    let w_q = Window::<Demand>::new(0, 8);

    let result = Window::delta(&w_p, &w_q);

    let expected = WindowDeltaResult {
        remaining_demand: Window::new(4, 8),
        remaining_supply: Curve::empty(),
        overlap: Window::new(2, 6),
    };

    assert_eq!(result, expected);
}
