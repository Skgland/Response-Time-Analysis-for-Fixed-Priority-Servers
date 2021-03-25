use std::collections::HashMap;

use crate::curve::{Curve, OverlapCurve, PrimitiveCurve};

use crate::time::TimeUnit;
use crate::window::{Demand, Supply, Window};

#[test]
fn aggregate_curves() {
    // Example 2.
    let c1 = unsafe { Curve::from_windows_unchecked(vec![Window::new(0, 4)]) };
    let c2: Curve<PrimitiveCurve<Demand>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 1),
            Window::new(5, 6),
            Window::new(10, 11),
        ])
    };
    let c3 = unsafe {
        Curve::<PrimitiveCurve<Demand>>::from_windows_unchecked(vec![
            Window::new(0, 6),
            Window::new(10, 11),
        ])
    };

    assert_eq!(c1.aggregate(c2), c3);
}

#[test]
fn delta_curves() {
    // Example 3.
    let c_p = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 5),
            Window::new(12, 15),
            Window::new(22, 24),
            Window::new(30, 35),
        ])
    };

    let c_q: Curve<PrimitiveCurve<_>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(2, 4),
            Window::new(14, 17),
            Window::new(22, 24),
        ])
    };

    let expected_overlap: Curve<OverlapCurve<PrimitiveCurve<Supply>, PrimitiveCurve<Demand>>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(2, 4),
            Window::new(14, 15),
            Window::new(22, 24),
            Window::new(30, 32),
        ])
    };

    let expected_remaining_supply: Curve<PrimitiveCurve<Supply>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 2),
            Window::new(4, 5),
            Window::new(12, 14),
            Window::new(32, 35),
        ])
    };

    let result = Curve::delta(c_p, c_q);

    assert_eq!(result.remaining_supply, expected_remaining_supply);
    assert_eq!(result.overlap, expected_overlap);
    assert!(result.remaining_demand.is_empty());
}

#[test]
fn split_curves() {
    // Example 4.

    let c_p: Curve<PrimitiveCurve<Supply>> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(2, 4),
            Window::new(5, 6),
            Window::new(7, 23),
            Window::new(24, 26),
        ])
    };

    let t_s = TimeUnit::from(10);

    let expected: HashMap<usize, _> = vec![
        (0, unsafe {
            Curve::from_windows_unchecked(vec![
                Window::new(2, 4),
                Window::new(5, 6),
                Window::new(7, 10),
            ])
        }),
        (1, unsafe {
            Curve::from_windows_unchecked(vec![Window::new(10, 20)])
        }),
        (2, unsafe {
            Curve::from_windows_unchecked(vec![Window::new(20, 23), Window::new(24, 26)])
        }),
    ]
    .into_iter()
    .collect();

    let result = c_p.split(t_s);

    assert_eq!(result, expected);
}
