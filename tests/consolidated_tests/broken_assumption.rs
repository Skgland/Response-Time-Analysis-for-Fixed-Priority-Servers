use rta_for_fps::curve::curve_types::CurveType;
use rta_for_fps::curve::Curve;
use rta_for_fps::iterators::CurveIterator;
use rta_for_fps::server::{ActualServerExecution, Server, ServerKind};
use rta_for_fps::system::System;
use rta_for_fps::task::Task;
use rta_for_fps::time::TimeUnit;
use rta_for_fps::window::window_types::WindowType;
use rta_for_fps::window::Window;

#[test]
// server 2 does not guarantee its budget every period, failing the algorithms assumption
#[should_panic]
fn remarks() {
    // Example 10.
    // Demand, Intervals ,and Offsets multiplied by 2  to fit in Integers
    // as we can't handle S_1 with capacity 1.5 otherwise

    let servers = &[
        Server {
            tasks: vec![Task::new(6, 22, 0)],
            capacity: TimeUnit::from(3),
            interval: TimeUnit::from(10),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(100, 400, 0)],
            capacity: TimeUnit::from(2),
            interval: TimeUnit::from(6),
            server_type: ServerKind::Deferrable,
        },
    ];

    let system = System::new(servers);

    let server_index = 1;
    let task_index = 0;
    let swh = system.system_wide_hyper_periode(server_index);

    let task = &servers[server_index].as_tasks()[task_index];
    let j = 24;
    let arrival = task.job_arrival(j - 1);
    let execution =
        Task::actual_execution_curve_iter(&system, server_index, task_index, swh).collect_curve();

    assert_eq!(arrival, TimeUnit::from(4600 * 2));

    let completed = Task::time_to_provide(&execution, j * task.demand);

    assert_eq!(completed, TimeUnit::from(4754 * 2));

    let result = Task::worst_case_response_time(&system, 1, 0);

    assert_eq!(result, TimeUnit::from(308));
}

// In the last paragraph of Section 6.1 the paper
// mentions that a check is necessary
// that the server guarantees its budget every replenishment interval
// the following examples do not have this guarantee and
// produce incorrect results as a consequence
//
// Section 2.2 Paragraph 2 also introduces this assumption

#[test]
#[should_panic]
fn example_too_high() {
    let servers = &[
        Server {
            tasks: vec![Task::new(16, 48, 0)],
            capacity: 12.into(),
            interval: 24.into(),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(4, 12, 0)],
            capacity: 6.into(),
            interval: 12.into(),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(1, 24, 0)],
            capacity: 1.into(),
            interval: 24.into(),
            server_type: ServerKind::Deferrable,
        },
    ];

    let system = System::new(servers);

    let wcrt = rta_for_fps::task::Task::worst_case_response_time(&system, servers.len() - 1, 0);

    assert_eq!(wcrt, 19.into());
}

#[test]
#[should_panic]
fn example_too_low() {
    let servers = &[
        Server {
            tasks: vec![Task::new(16, 48, 0)],
            capacity: 12.into(),
            interval: 24.into(),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(4, 12, 0)],
            capacity: 6.into(),
            interval: 12.into(),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(10, 48, 33)],
            capacity: 10.into(),
            interval: 48.into(),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(1, 24, 0)],
            capacity: 1.into(),
            interval: 24.into(),
            server_type: ServerKind::Deferrable,
        },
    ];

    let system = System::new(servers);

    let wcrt = rta_for_fps::task::Task::worst_case_response_time(&system, servers.len() - 1, 0);

    assert_eq!(wcrt, 22.into());
}

#[test]
#[should_panic]
fn execution_overlap_too_high() {
    let servers = &[
        Server {
            tasks: vec![Task::new(16, 48, 0)],
            capacity: 12.into(),
            interval: 24.into(),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(4, 12, 0)],
            capacity: 6.into(),
            interval: 12.into(),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(1, 24, 0)],
            capacity: 1.into(),
            interval: 24.into(),
            server_type: ServerKind::Deferrable,
        },
    ];

    let system = System::new(servers);

    let s1 = system
        .actual_execution_curve_iter(0, 48.into())
        .collect_curve();
    let s2: Curve<ActualServerExecution> = system
        .actual_execution_curve_iter(1, 48.into())
        .collect_curve();
    let s3: Curve<ActualServerExecution> = system
        .actual_execution_curve_iter(2, 48.into())
        .collect_curve();

    assert!(
        curve_has_no_non_trivial_overlap(&s1, &s2),
        "Curve 1: {:#?}\n\nCurve 2: {:#?}",
        &s1,
        &s2
    );
    assert!(
        curve_has_no_non_trivial_overlap(&s1, &s3),
        "Curve 1: {:#?}\n\nCurve 3: {:#?}",
        &s1,
        &s3
    );

    assert!(
        curve_has_no_non_trivial_overlap(&s2, &s3),
        "Curve 2: {:#?}\n\nCurve 3: {:#?}",
        &s2,
        &s3
    );
}

#[test]
#[should_panic]
fn execution_overlap_too_low() {
    let servers = &[
        Server {
            tasks: vec![Task::new(16, 48, 0)],
            capacity: 12.into(),
            interval: 24.into(),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(4, 12, 0)],
            capacity: 6.into(),
            interval: 12.into(),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(10, 48, 33)],
            capacity: 10.into(),
            interval: 48.into(),
            server_type: ServerKind::Deferrable,
        },
        Server {
            tasks: vec![Task::new(1, 24, 0)],
            capacity: 1.into(),
            interval: 24.into(),
            server_type: ServerKind::Deferrable,
        },
    ];

    let system = System::new(servers);

    let s1: Curve<ActualServerExecution> = system
        .actual_execution_curve_iter(0, 48.into())
        .collect_curve();
    let s2: Curve<ActualServerExecution> = system
        .actual_execution_curve_iter(1, 48.into())
        .collect_curve();
    let s3: Curve<ActualServerExecution> = system
        .actual_execution_curve_iter(2, 48.into())
        .collect_curve();
    let s4: Curve<ActualServerExecution> = system
        .actual_execution_curve_iter(3, 48.into())
        .collect_curve();

    //TODO assert messages
    assert!(curve_has_no_non_trivial_overlap(&s1, &s2));
    assert!(curve_has_no_non_trivial_overlap(&s1, &s3));
    assert!(curve_has_no_non_trivial_overlap(&s1, &s4));

    assert!(curve_has_no_non_trivial_overlap(&s2, &s3));
    assert!(curve_has_no_non_trivial_overlap(&s2, &s4));

    assert!(curve_has_no_non_trivial_overlap(&s3, &s4));
}

pub fn curve_has_no_non_trivial_overlap<C: CurveType>(c1: &Curve<C>, c2: &Curve<C>) -> bool {
    c1.as_windows().iter().all(|window1| {
        c2.as_windows()
            .iter()
            .all(|window2| window_has_no_non_trivial_overlap(window1, window2))
    })
}

pub fn window_has_no_non_trivial_overlap<W: WindowType>(w1: &Window<W>, w2: &Window<W>) -> bool {
    (!w1.overlaps(w2)) || w1.end == w2.start || w2.end == w1.start
}
