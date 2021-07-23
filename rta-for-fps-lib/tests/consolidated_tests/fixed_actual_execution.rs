use rta_for_fps_lib::curve::Curve;
use rta_for_fps_lib::iterators::CurveIterator;
use rta_for_fps_lib::server::Server;
use rta_for_fps_lib::server::ServerKind::Deferrable;
use rta_for_fps_lib::system::System;
use rta_for_fps_lib::task::Task;
use rta_for_fps_lib::time::TimeUnit;
use rta_for_fps_lib::window::Window;

#[test]
pub fn original() {
    let task1 = &[Task::new(1, 8, 2)];
    let task2 = &[Task::new(2, 4, 2)];
    let task3 = &[Task::new(1, 4, 2)];
    let server1 = Server::new(task1, 1.into(), 8.into(), Deferrable);
    let server2 = Server::new(task2, 2.into(), 4.into(), Deferrable);
    let server3 = Server::new(task3, 1.into(), 4.into(), Deferrable);
    let servers = [server1, server2, server3];
    let system = System::new(&servers);

    let orig_actual_curve: Curve<_> = system
        .original_actual_execution_curve_iter(2)
        .take_while_curve(|w| w.end <= TimeUnit::from(18))
        .collect_curve();

    let expected_curve: Curve<_> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(5, 6),
            Window::new(8, 9),
            Window::new(13, 14),
            Window::new(16, 17),
        ])
    };

    assert_eq!(orig_actual_curve, expected_curve);
}

#[test]
pub fn aggregated_hp_actual_execution() {
    let task1 = &[Task::new(1, 8, 2)];
    let task2 = &[Task::new(2, 4, 2)];
    let task3 = &[Task::new(1, 4, 2)];
    let server1 = Server::new(task1, 1.into(), 8.into(), Deferrable);
    let server2 = Server::new(task2, 2.into(), 4.into(), Deferrable);
    let server3 = Server::new(task3, 1.into(), 4.into(), Deferrable);
    let servers = [server1, server2, server3];
    let system = System::new(&servers);

    let aggregated_hp_execution: Curve<_> = system
        .aggregated_higher_priority_actual_execution_curve_iter(2)
        .take_while_curve(|w| w.end <= TimeUnit::from(18))
        .collect_curve();

    let expected_hp_execution = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(2, 5),
            Window::new(6, 7),
            Window::new(8, 9),
            Window::new(10, 13),
            Window::new(14, 15),
            Window::new(16, 17),
        ])
    };

    assert_eq!(aggregated_hp_execution, expected_hp_execution)
}

#[test]
pub fn fixed() {
    let task1 = &[Task::new(1, 8, 2)];
    let task2 = &[Task::new(2, 4, 2)];
    let task3 = &[Task::new(1, 4, 2)];
    let server1 = Server::new(task1, 1.into(), 8.into(), Deferrable);
    let server2 = Server::new(task2, 2.into(), 4.into(), Deferrable);
    let server3 = Server::new(task3, 1.into(), 4.into(), Deferrable);
    let servers = [server1, server2, server3];
    let system = System::new(&servers);

    let fixed_actual_curve: Curve<_> = system
        .fixed_actual_execution_curve_iter(2)
        .take_while_curve(|w| w.end <= TimeUnit::from(18))
        .collect_curve();

    let expected_curve: Curve<_> = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(5, 6),
            Window::new(9, 10),
            Window::new(13, 14),
            Window::new(17, 18),
        ])
    };

    assert_eq!(fixed_actual_curve, expected_curve);
}
