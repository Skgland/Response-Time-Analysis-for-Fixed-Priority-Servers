use crate::rta_lib::curve::Curve;
use crate::rta_lib::iterators::CurveIterator;
use crate::rta_lib::server::{Server, ServerKind};
use crate::rta_lib::task::Task;
use crate::rta_lib::time::TimeUnit;
use crate::rta_lib::window::Window;

#[test]
fn deferrable_server() {
    // Example 6. with t = 18

    let tasks = &[Task::new(1, 5, 0), Task::new(2, 8, 0)];

    let server = Server::new(
        tasks,
        TimeUnit::from(2),
        TimeUnit::from(4),
        ServerKind::Deferrable,
    );

    let result = server
        .constraint_demand_curve_iter()
        .take_while_curve(|window| window.end <= TimeUnit::from(18))
        .normalize();

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

    crate::util::assert_curve_eq(&expected_result, result);
}
