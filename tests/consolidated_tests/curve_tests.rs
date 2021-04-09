use rta_for_fps::curve::curve_types::UnspecifiedCurve;
use rta_for_fps::curve::Curve;
use rta_for_fps::iterators::curve::{AggregationIterator, CurveDeltaIterator, CurveSplitIterator};
use rta_for_fps::time::TimeUnit;
use rta_for_fps::window::{Demand, Overlap, Supply, Window};

#[test]
fn aggregate_curves() {
    // Example 2.
    let c1 = unsafe { Curve::from_windows_unchecked(vec![Window::new(0, 4)]) };
    let c2: Curve<UnspecifiedCurve<Demand>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 1),
            Window::new(5, 6),
            Window::new(10, 11),
        ])
    };
    let c3 = unsafe {
        Curve::<UnspecifiedCurve<Demand>>::from_windows_unchecked(vec![
            Window::new(0, 6),
            Window::new(10, 11),
        ])
    };

    let result = AggregationIterator::new(vec![c1.into_iter(), c2.into_iter()]);

    crate::util::assert_curve_eq(&c3, result);
}

#[test]
fn delta_curves() {
    // Example 3.
    let c_p: Curve<UnspecifiedCurve<Supply>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 5),
            Window::new(12, 15),
            Window::new(22, 24),
            Window::new(30, 35),
        ])
    };

    let c_q: Curve<UnspecifiedCurve<Demand>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(2, 4),
            Window::new(14, 17),
            Window::new(22, 24),
        ])
    };

    let expected_overlap: Curve<UnspecifiedCurve<Overlap<Supply, Demand>>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(2, 4),
            Window::new(14, 15),
            Window::new(22, 24),
            Window::new(30, 32),
        ])
    };

    let expected_remaining_supply: Curve<UnspecifiedCurve<Supply>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 2),
            Window::new(4, 5),
            Window::new(12, 14),
            Window::new(32, 35),
        ])
    };

    let result = CurveDeltaIterator::new(c_p.into_iter(), c_q.into_iter()).collect_delta();
    assert_eq!(result.remaining_supply, expected_remaining_supply);
    assert_eq!(result.overlap, expected_overlap);
    assert!(
        result.remaining_demand.is_empty(),
        "Expected empty remaining demand, got: {:#?}",
        result.remaining_demand
    );
}

#[test]
fn split_curves() {
    // Example 4.

    let c_p: Curve<UnspecifiedCurve<Supply>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(2, 4),
            Window::new(5, 6),
            Window::new(7, 23),
            Window::new(24, 26),
        ])
    };

    let t_s = TimeUnit::from(10);

    let expected = vec![
        Window::new(2, 4),
        Window::new(5, 6),
        Window::new(7, 10),
        Window::new(10, 20),
        Window::new(20, 23),
        Window::new(24, 26),
    ];

    let result: Vec<_> = CurveSplitIterator::new(c_p.into_iter(), t_s).collect();

    assert_eq!(result, expected);
}
