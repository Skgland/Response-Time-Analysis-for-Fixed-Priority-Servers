use crate::curve::Curve;
use crate::server::{Server, ServerType};
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
        server_type: ServerType::Deferrable,
    };

    let result = server.constrain_demand_curve(TimeUnit::from(18));

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
        capacity: TimeUnit::from(3),
        interval: TimeUnit::from(10),
        server_type: ServerType::Deferrable,
    };

    let servers: &[_] = &[server];

    let up_to = TimeUnit::from(16);

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
        capacity: TimeUnit::from(2),
        interval: TimeUnit::from(10),
        server_type: ServerType::Deferrable,
    };

    let higher_priority_load = Server {
        tasks: vec![
            Task::new(3, 30, 0),
            Task::new(5, 30, 5),
            Task::new(5, 30, 12),
            Task::new(3, 30, 18),
        ],
        capacity: TimeUnit::from(30),
        interval: TimeUnit::from(30),
        server_type: ServerType::Deferrable,
    };

    let up_to = TimeUnit::from(24);

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

#[test]
fn response_time() {
    // Example 9.

    let servers = &[
        Server {
            tasks: vec![Task::new(1, 4, 0)],
            capacity: TimeUnit::from(3),
            interval: TimeUnit::from(10),
            server_type: ServerType::Deferrable,
        },
        Server {
            tasks: vec![Task::new(1, 5, 0), Task::new(2, 8, 0)],
            capacity: TimeUnit::from(2),
            interval: TimeUnit::from(4),
            server_type: ServerType::Deferrable,
        },
    ];

    let c_s2 = Server::constrained_execution_curve(servers, 1, TimeUnit::from(16));

    let expected = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(1, 3),
            Window::new(5, 7),
            Window::new(9, 11),
            Window::new(13, 14),
            Window::new(15, 16),
        ])
    };

    assert_eq!(c_s2, expected);

    let t2_demand = Task::demand_curve(&servers[1].as_tasks()[0], TimeUnit::from(16));

    let expected = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 1),
            Window::new(5, 6),
            Window::new(10, 11),
            Window::new(15, 16),
        ])
    };

    assert_eq!(t2_demand, expected);

    let t2_available = Task::actual_execution_curve(servers, 1, 0, TimeUnit::from(16));

    let expected = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(1, 2),
            Window::new(5, 6),
            Window::new(10, 11),
            Window::new(15, 16),
        ])
    };

    assert_eq!(t2_available, expected);

    let swh = Server::system_wide_hyper_periode(servers);

    assert_eq!(swh, TimeUnit::from(40));

    let wcrt = Task::worst_case_response_time(servers, 1, 0);

    assert_eq!(wcrt, TimeUnit::from(3));
}

#[test]
#[ignore]
fn response_time2() {
    // Example 9. without t_3

    // TODO fix, probably not applying 7.1 correctly

    let servers = &[
        Server {
            tasks: vec![Task::new(1, 4, 0)],
            capacity: TimeUnit::from(3),
            interval: TimeUnit::from(10),
            server_type: ServerType::Deferrable,
        },
        Server {
            tasks: vec![Task::new(1, 5, 0)],
            capacity: TimeUnit::from(2),
            interval: TimeUnit::from(4),
            server_type: ServerType::Deferrable,
        },
    ];

    let t2_demand = Task::demand_curve(&servers[1].as_tasks()[0], TimeUnit::from(16));

    let expected = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 1),
            Window::new(5, 6),
            Window::new(10, 11),
            Window::new(15, 16),
        ])
    };

    assert_eq!(t2_demand, expected);

    let t2_available = Task::actual_execution_curve(servers, 1, 0, TimeUnit::from(16));

    let expected = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(1, 2),
            Window::new(5, 6),
            Window::new(10, 11),
            Window::new(15, 16),
        ])
    };

    assert_eq!(t2_available, expected);

    let swh = Server::system_wide_hyper_periode(servers);

    assert_eq!(swh, TimeUnit::from(20));

    let wcrt = Task::worst_case_response_time(servers, 1, 0);

    assert_eq!(wcrt, TimeUnit::from(3));
}

// TODO fix?
#[test]
#[ignore]
fn remarks() {
    // Example 10.
    // Demand, Intervals ,and Offsets multiplied by 2  to fit in Integers
    // as we can't handle S_1 with capacity 1.5 otherwise

    let servers = &[
        Server {
            tasks: vec![Task::new(6, 22, 0)],
            capacity: TimeUnit::from(3),
            interval: TimeUnit::from(10),
            server_type: ServerType::Deferrable,
        },
        Server {
            tasks: vec![Task::new(100, 400, 0)],
            capacity: TimeUnit::from(2),
            interval: TimeUnit::from(6),
            server_type: ServerType::Deferrable,
        },
    ];

    let swh = Server::system_wide_hyper_periode(servers);

    let task = &servers[1].as_tasks()[0];
    let j = 24;
    let arrival = task.job_arrival(j - 1);
    let execution = Task::actual_execution_curve(servers, 1, 0, swh);

    assert_eq!(arrival, TimeUnit::from(4600 * 2));

    let completed = Task::time_to_provide(&execution, j * task.demand);

    assert_eq!(completed, TimeUnit::from(4754 * 2));

    let result = Task::worst_case_response_time(servers, 1, 0);

    assert_eq!(result, TimeUnit::from(308));
}

#[test]
fn comparison() {
    // Example 11.

    let servers = &[
        Server {
            tasks: vec![Task::new(4, 10, 0)],
            capacity: TimeUnit::from(5),
            interval: TimeUnit::from(10),
            server_type: ServerType::Deferrable,
        },
        Server {
            tasks: vec![Task::new(3, 10, 0), Task::new(1, 10, 0)],
            capacity: TimeUnit::from(8),
            interval: TimeUnit::from(20),
            server_type: ServerType::Deferrable,
        },
    ];

    let up_to = TimeUnit::from(20);

    let t2_d = servers[1].as_tasks()[0].demand_curve(up_to);

    let expected_t2_d =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(0, 3), Window::new(10, 13)]) };

    assert_eq!(t2_d, expected_t2_d);

    let t3_d = servers[1].as_tasks()[1].demand_curve(up_to);

    let expected_t3_d =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(0, 1), Window::new(10, 11)]) };

    assert_eq!(t3_d, expected_t3_d);

    let s2_aggregated_demand = servers[1].aggregated_demand_curve(up_to);
    let s2_constrained_demand = servers[1].constrain_demand_curve(up_to);
    let expected_s2_demand =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(0, 4), Window::new(10, 14)]) };

    assert_eq!(s2_aggregated_demand, expected_s2_demand);
    assert_eq!(s2_constrained_demand, expected_s2_demand);

    let s2_unconstrained_execution = Server::unconstrained_execution_curve(servers, 1, up_to);

    // Note: Paper lists 6,10 and 16,20 as the unconstrained curve
    // but contradicts itself later with actual curve 4,8 and 14,18
    // though the later should lie in the former
    // and the supply 4,16 and 14,16 should be still available as
    // T_1 has only 4 demand every 10
    //
    // Therefore as we calculate the result here to be 4,10 and 14,20 this is assumed to be correct
    let expected_s2_unconstrained_execution =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(4, 10), Window::new(14, 20)]) };
    assert_eq!(
        s2_unconstrained_execution,
        expected_s2_unconstrained_execution
    );

    let s2_constrained_execution = Server::constrained_execution_curve(servers, 1, up_to);
    let expected_s2_constrained_execution =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(4, 8), Window::new(14, 18)]) };

    assert_eq!(s2_constrained_execution, expected_s2_constrained_execution);

    let t2_execution = Task::actual_execution_curve(servers, 1, 0, up_to);

    let expected_t2_execution =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(4, 7), Window::new(14, 17)]) };

    assert_eq!(t2_execution, expected_t2_execution);

    let task_2: &Task = &servers[1].as_tasks()[0];

    let expected_response_time = TimeUnit::from(7);

    let r_2_1 = Task::time_to_provide(&t2_execution, task_2.demand) - task_2.job_arrival(0);

    assert_eq!(r_2_1, expected_response_time);

    let r_2_2 = Task::time_to_provide(&t2_execution, 2 * task_2.demand) - task_2.job_arrival(1);

    assert_eq!(r_2_2, expected_response_time);

    let wcrt = Task::worst_case_response_time(servers, 1, 0);
    assert_eq!(wcrt, expected_response_time);
}
