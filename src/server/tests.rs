use crate::curve::Curve;
use crate::server::{Server, ServerType};
use crate::task::Task;
use crate::window::Window;

#[test]
fn deferrable_server() {
    // Example 6. with t = 18

    let server = Server {
        tasks: vec![Task::new(1, 5, 0), Task::new(2, 8, 0)],
        capacity: 2,
        interval: 4,
        server_type: ServerType::Deferrable,
    };

    let result = server.constrain_demand_curve(18);

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

#[test]
fn unconstrained_curve() {
    // Example 7.

    let server = Server {
        tasks: vec![Task::new(1, 4, 0)],
        capacity: 3,
        interval: 10,
        server_type: ServerType::Deferrable,
    };

    let servers: &[_] = &[server];

    let up_to = 16;

    let aggregated_result = Server::aggregated_higher_priority_demand_curve(servers, 1, up_to);

    let expected_aggregated_result = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 1),
            Window::new(4, 5),
            Window::new(8, 9),
            Window::new(12, 13),
        ])
    };

    assert_eq!(aggregated_result, expected_aggregated_result);

    let unconstrained_result = Server::unconstrained_execution_curve(servers, 1, up_to);

    let expected_unconstrained_result = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(1, 4),
            Window::new(5, 8),
            Window::new(9, 12),
            Window::new(13, 16),
        ])
    };

    assert_eq!(unconstrained_result, expected_unconstrained_result);
}

#[test]
#[allow(clippy::similar_names)]
fn executive_curve() {
    // Example 8.

    let server = Server {
        tasks: vec![
            Task::new(1, 30, 2),
            Task::new(1, 30, 5),
            Task::new(2, 30, 10),
        ],
        capacity: 2,
        interval: 10,
        server_type: ServerType::Deferrable,
    };

    let higher_priority_load = Server {
        tasks: vec![
            Task::new(3, 30, 0),
            Task::new(5, 30, 5),
            Task::new(5, 30, 12),
            Task::new(3, 30, 18),
        ],
        capacity: 30,
        interval: 30,
        server_type: ServerType::Deferrable,
    };

    let up_to = 24;

    let servers = &[higher_priority_load, server];

    // Unconstrained execution supply curve

    let uc_execution_result = Server::unconstrained_execution_curve(servers, 1, up_to);

    let expected_uc_execution = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(3, 5),
            Window::new(10, 12),
            Window::new(17, 18),
            Window::new(21, 24),
        ])
    };

    assert_eq!(
        uc_execution_result, expected_uc_execution,
        "Unconstrained Execution Supply Curve"
    );

    // Constrained demand curve

    let demand_result = Server::constrain_demand_curve(&servers[1], up_to);

    let expected_demand = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(2, 3),
            Window::new(5, 6),
            Window::new(10, 12),
        ])
    };

    assert_eq!(demand_result, expected_demand, "Constrained demand Curve");

    let c_execution_result = Server::constrained_execution_curve(servers, 1, up_to);

    let expected_c_execution = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(3, 4),
            Window::new(10, 12),
            Window::new(21, 22),
        ])
    };

    assert_eq!(
        c_execution_result, expected_c_execution,
        "Constrained Execution Curve"
    );
}
