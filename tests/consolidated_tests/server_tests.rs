use rta_for_fps::curve::Curve;
use rta_for_fps::iterators::curve::CollectCurveExt;
use rta_for_fps::server::{ConstrainedServerDemand, Server, ServerKind};
use rta_for_fps::task::Task;
use rta_for_fps::time::TimeUnit;
use rta_for_fps::window::Window;

#[test]
fn deferrable_server() {
    // Example 6. with t = 18

    let server = Server {
        tasks: vec![Task::new(1, 5, 0), Task::new(2, 8, 0)],
        capacity: TimeUnit::from(2),
        interval: TimeUnit::from(4),
        server_type: ServerKind::Deferrable,
    };

    let result: Curve<ConstrainedServerDemand> = server
        .constraint_demand_curve_iter(TimeUnit::from(18))
        .collect_curve();

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
