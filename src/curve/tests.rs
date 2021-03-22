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
