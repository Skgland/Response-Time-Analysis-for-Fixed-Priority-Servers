use crate::curve::Curve;
use crate::server::{Server, ServerKind};
use crate::task::Task;
use crate::time::TimeUnit;
use crate::window::Window;

#[test]
fn deferrable_server() {
    // Example 6. with t = 18

    let server = Server {
        tasks: vec![Task::new(1, 5, 0), Task::new(2, 8, 0)],
        capacity: TimeUnit::from(2),
        interval: TimeUnit::from(4),
        server_type: ServerKind::Deferrable,
    };

    let result = server.constraint_demand_curve(TimeUnit::from(18));

    let expected_result = unsafe {
        // the example in the paper is confusing as
        // either (4,5) and (5,6) should not have been merged to (4,6)
        // or (15,16) and (16,18) should be merged to (15,18)
        // the latter is done here as it allows for the usage of a Curve
        Curve::from_windows_unchecked(vec![
            Window::new(0, 2),
            Window::new(4, 6),
            Window::new(8, 10),
            Window::new(12, 13),
            Window::new(15, 18),
        ])
    };

    assert_eq!(result, expected_result)
}
