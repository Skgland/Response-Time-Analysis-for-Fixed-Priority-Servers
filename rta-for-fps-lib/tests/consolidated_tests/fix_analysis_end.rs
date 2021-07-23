use rta_for_fps_lib::server::Server;
use rta_for_fps_lib::server::ServerKind::Deferrable;
use rta_for_fps_lib::system::System;
use rta_for_fps_lib::task::Task;
use rta_for_fps_lib::time::TimeUnit;

#[test]
fn analysis_with_swh() {
    let task1 = &[Task::new(2, 4, 3)];
    let task2 = &[Task::new(1, 4, 0)];

    let servers = &[
        Server::new(task1, 2.into(), 4.into(), Deferrable),
        Server::new(task2, 1.into(), 4.into(), Deferrable),
    ];

    let system = System::new(servers);

    let response_time =
        Task::fixed_worst_case_response_time(&system, 1, 0, system.system_wide_hyper_period(1));
    let expected_response_time: TimeUnit = 1.into();

    assert_eq!(response_time, expected_response_time)
}

#[test]
fn analysis_with_swh_plus_offset() {
    let task1 = &[Task::new(2, 4, 3)];
    let task2 = &[Task::new(1, 4, 0)];

    let servers = &[
        Server::new(task1, 2.into(), 4.into(), Deferrable),
        Server::new(task2, 1.into(), 4.into(), Deferrable),
    ];

    let system = System::new(servers);

    let response_time = Task::fixed_worst_case_response_time(&system, 1, 0, system.analysis_end(1));
    let expected_response_time: TimeUnit = 2.into();

    assert_eq!(response_time, expected_response_time)
}
