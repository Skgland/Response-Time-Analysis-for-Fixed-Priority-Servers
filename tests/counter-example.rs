use rta_for_fps::curve::curve_types::CurveType;
use rta_for_fps::curve::Curve;
use rta_for_fps::server::{Server, ServerKind};
use rta_for_fps::task::Task;
use rta_for_fps::window::window_types::WindowType;
use rta_for_fps::window::Window;

// In the last paragraph of Section 6.1 the paper
// mentions that a check is necessary
// that the server guarantees its budget every replenishment interval
// these counter examples do not have this guarantee and
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

    let wcrt = rta_for_fps::task::Task::worst_case_response_time(servers, servers.len() - 1, 0);

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

    let wcrt = rta_for_fps::task::Task::worst_case_response_time(servers, servers.len() - 1, 0);

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

    let s1 = rta_for_fps::server::Server::actual_execution_curve(servers, 0, 48.into());
    let s2 = rta_for_fps::server::Server::actual_execution_curve(servers, 1, 48.into());
    let s3 = rta_for_fps::server::Server::actual_execution_curve(servers, 2, 48.into());

    assert!(curve_has_no_non_trivial_overlap(&s1, &s2));
    assert!(curve_has_no_non_trivial_overlap(&s1, &s3));

    assert!(curve_has_no_non_trivial_overlap(&s2, &s3));
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

    let s1 = rta_for_fps::server::Server::actual_execution_curve(servers, 0, 48.into());
    let s2 = rta_for_fps::server::Server::actual_execution_curve(servers, 1, 48.into());
    let s3 = rta_for_fps::server::Server::actual_execution_curve(servers, 2, 48.into());
    let s4 = rta_for_fps::server::Server::actual_execution_curve(servers, 3, 48.into());

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
