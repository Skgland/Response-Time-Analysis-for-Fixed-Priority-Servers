use crate::curve::Curve;
use crate::window::Window;

#[test]
fn aggregate_curves() {
    let c1 = Curve {
        windows: vec![Window { start: 0, end: 4 }],
    };
    let c2 = Curve {
        windows: vec![
            Window { start: 0, end: 1 },
            Window { start: 5, end: 6 },
            Window { start: 10, end: 11 },
        ],
    };
    let c3 = Curve {
        windows: vec![Window { start: 0, end: 6 }, Window { start: 10, end: 11 }],
    };

    assert_eq!(c1.aggregate(&c2), c3);
}

#[test]
fn delta_curves() {
    // Example 3.
    let c_p = Curve {
        windows: vec![
            Window::new(0, 5),
            Window::new(12, 15),
            Window::new(22, 24),
            Window::new(30, 35),
        ],
    };

    let c_q = Curve {
        windows: vec![Window::new(2, 4), Window::new(14, 17), Window::new(22, 24)],
    };

    let expected_overlap = Curve {
        windows: vec![
            Window::new(2, 4),
            Window::new(14, 15),
            Window::new(22, 24),
            Window::new(30, 32),
        ],
    };

    let expected_remaining_supply = Curve {
        windows: vec![
            Window::new(0, 2),
            Window::new(4, 5),
            Window::new(12, 14),
            Window::new(32, 35),
        ],
    };

    let result = Curve::delta(&c_p, &c_q);

    assert_eq!(result.remaining_supply, expected_remaining_supply);
    assert_eq!(result.overlap, expected_overlap);
    assert!(result.remaining_demand.is_empty());
}
