use rta_for_fps_lib::server::Server;
use rta_for_fps_lib::server::ServerKind::Deferrable;
use rta_for_fps_lib::system::System;
use rta_for_fps_lib::task::Task;
use rta_for_fps_lib::time::TimeUnit;

#[test]
fn issue8() {
    let tasks: &[_] = &[Task::new(2, 3, 1), Task::new(1, 3, 0)];
    let servers: &[_] = &[Server::new(&tasks, 3.into(), 3.into(), Deferrable)];
    let system = System::new(&servers);
    let response_time =
        Task::worst_case_response_time(&system, 0, 1, system.system_wide_hyper_period(0));
    // For the analysis up to the SWH this is correct but in general it should be 2
    assert_eq!(response_time, TimeUnit::from(1))
}
