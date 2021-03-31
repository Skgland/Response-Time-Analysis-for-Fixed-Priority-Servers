use rta_for_fps::curve::Curve;
use rta_for_fps::iterators::curve::CollectCurveExt;
use rta_for_fps::server::{
    ActualServerExecution, AvailableServerExecution, HigherPriorityServerDemand, Server, ServerKind,
};
use rta_for_fps::system::System;
use rta_for_fps::task::{ActualTaskExecution, Task, TaskDemand};
use rta_for_fps::time::TimeUnit;
use rta_for_fps::window::Window;

#[test]
fn unconstrained_curve() {
    // Example 7.

    let server = Server {
        tasks: vec![Task::new(1, 4, 0)],
        capacity: TimeUnit::from(3),
        interval: TimeUnit::from(10),
        server_type: ServerKind::Deferrable,
    };

    let servers = &[server];

    let system = System::new(servers);

    let up_to = TimeUnit::from(16);

    let aggregated_result: Curve<HigherPriorityServerDemand> = system
        .aggregated_higher_priority_demand_curve_iter(1, up_to)
        .collect_curve();

    let expected_aggregated_result = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 1),
            Window::new(4, 5),
            Window::new(8, 9),
            Window::new(12, 13),
        ])
    };

    assert_eq!(aggregated_result, expected_aggregated_result);

    let unconstrained_result: Curve<AvailableServerExecution> = system
        .available_server_execution_curve_iter(1, up_to)
        .collect_curve();

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
        server_type: ServerKind::Deferrable,
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
        server_type: ServerKind::Deferrable,
    };

    let up_to = TimeUnit::from(24);

    let servers = &[higher_priority_load, server];

    let system = System::new(servers);

    // Unconstrained execution supply curve

    let uc_execution_result: Curve<AvailableServerExecution> = system
        .available_server_execution_curve_iter(1, up_to)
        .collect_curve();

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

    let demand_result = system.as_servers()[1].constraint_demand_curve(up_to);

    let expected_demand = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(2, 3),
            Window::new(5, 6),
            Window::new(10, 12),
        ])
    };

    assert_eq!(demand_result, expected_demand, "Constrained demand Curve");

    let c_execution_result: Curve<ActualServerExecution> =
        system.actual_execution_curve_iter(1, up_to).collect_curve();

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
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(1, 5, 0), Task::new(2, 8, 0)],
            capacity: TimeUnit::from(2),
            interval: TimeUnit::from(4),
            server_type: ServerKind::Deferrable,
        },
    ];

    let system = System::new(servers);

    let server_index = 1;
    let task_index = 0;

    let c_s2: Curve<ActualServerExecution> = system
        .actual_execution_curve_iter(server_index, TimeUnit::from(16))
        .collect_curve();

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

    let t2_demand: Curve<TaskDemand> = Task::demand_curve_iter(
        &servers[server_index].as_tasks()[task_index],
        TimeUnit::from(16),
    )
    .collect_curve();

    let expected = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 1),
            Window::new(5, 6),
            Window::new(10, 11),
            Window::new(15, 16),
        ])
    };

    assert_eq!(t2_demand, expected);

    let t2_available: Curve<ActualTaskExecution> =
        Task::actual_execution_curve_iter(&system, server_index, task_index, TimeUnit::from(16))
            .collect_curve();

    let expected = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(1, 2),
            Window::new(5, 6),
            Window::new(10, 11),
            Window::new(15, 16),
        ])
    };

    assert_eq!(t2_available, expected);

    let swh = system.system_wide_hyper_periode(server_index);

    assert_eq!(swh, TimeUnit::from(40));

    let wcrt = Task::worst_case_response_time(&system, server_index, task_index);

    assert_eq!(wcrt, TimeUnit::from(3));
}

#[test]
fn comparison() {
    // Example 11.

    let servers = &[
        Server {
            tasks: vec![Task::new(4, 10, 0)],
            capacity: TimeUnit::from(5),
            interval: TimeUnit::from(10),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(3, 10, 0), Task::new(1, 10, 0)],
            capacity: TimeUnit::from(8),
            interval: TimeUnit::from(20),
            server_type: ServerKind::Deferrable,
        },
    ];

    let system = System::new(servers);

    let up_to = TimeUnit::from(20);

    let t2_d: Curve<TaskDemand> = servers[1].as_tasks()[0]
        .demand_curve_iter(up_to)
        .collect_curve();

    let expected_t2_d =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(0, 3), Window::new(10, 13)]) };

    assert_eq!(t2_d, expected_t2_d);

    let t3_d: Curve<TaskDemand> = servers[1].as_tasks()[1]
        .demand_curve_iter(up_to)
        .collect_curve();

    let expected_t3_d =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(0, 1), Window::new(10, 11)]) };

    assert_eq!(t3_d, expected_t3_d);

    let s2_aggregated_demand = servers[1].aggregated_demand_curve(up_to);
    let s2_constrained_demand = servers[1].constraint_demand_curve(up_to);
    let expected_s2_demand =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(0, 4), Window::new(10, 14)]) };

    assert_eq!(s2_aggregated_demand, expected_s2_demand);
    assert_eq!(s2_constrained_demand, expected_s2_demand.reclassify());

    let s2_unconstrained_execution: Curve<AvailableServerExecution> = system
        .available_server_execution_curve_iter(1, up_to)
        .collect_curve();

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

    let s2_constrained_execution: Curve<ActualServerExecution> =
        system.actual_execution_curve_iter(1, up_to).collect_curve();
    let expected_s2_constrained_execution =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(4, 8), Window::new(14, 18)]) };

    assert_eq!(s2_constrained_execution, expected_s2_constrained_execution);

    let t2_execution = Task::actual_execution_curve_iter(&system, 1, 0, up_to).collect_curve();

    let expected_t2_execution =
        unsafe { Curve::from_windows_unchecked(vec![Window::new(4, 7), Window::new(14, 17)]) };

    assert_eq!(t2_execution, expected_t2_execution);

    let task_2: &Task = &servers[1].as_tasks()[0];

    let expected_response_time = TimeUnit::from(7);

    let r_2_1 = Task::time_to_provide(&t2_execution, task_2.demand) - task_2.job_arrival(0);

    assert_eq!(r_2_1, expected_response_time);

    let r_2_2 = Task::time_to_provide(&t2_execution, 2 * task_2.demand) - task_2.job_arrival(1);

    assert_eq!(r_2_2, expected_response_time);

    let wcrt = Task::worst_case_response_time(&system, 1, 0);
    assert_eq!(wcrt, expected_response_time);
}
