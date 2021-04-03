mod broken_assumption;
mod curve_tests;
mod server_tests;
mod system_tests;
mod task_tests;
mod window_tests;

mod incorrect {
    use crate::broken_assumption::curve_has_no_non_trivial_overlap;
    use rta_for_fps::iterators::CurveIterator;
    use rta_for_fps::server::{Server, ServerKind};
    use rta_for_fps::system::System;
    use rta_for_fps::task::Task;

    #[test]
    #[ignore]
    fn incorrect() {
        let tasks_s1 = &[Task::new(8, 32, 8)];
        let tasks_s2 = &[Task::new(4, 16, 8)];
        let tasks_s3 = &[
            Task::new(2, 64, 0),
            Task::new(2, 64, 16),
            Task::new(2, 64, 32),
            Task::new(1, 64, 48),
        ];

        let servers = &[
            Server::new(tasks_s1, 8.into(), 16.into(), ServerKind::Deferrable),
            Server::new(tasks_s2, 4.into(), 16.into(), ServerKind::Deferrable),
            Server::new(tasks_s3, 2.into(), 16.into(), ServerKind::Deferrable),
        ];

        let system = System::new(servers);

        let swh1 = system.system_wide_hyper_periode(1);
        let swh2 = system.system_wide_hyper_periode(2);
        let aes1 = system.actual_execution_curve_iter(1, swh1);
        let aes2 = system.actual_execution_curve_iter(2, swh2);

        let aes1c = aes1.collect_curve();
        let aes2c = aes2.collect_curve();

        eprintln!("{:#?}\n\n{:#?}", aes1c, aes2c);

        let result = curve_has_no_non_trivial_overlap(&aes1c, &aes2c);
        assert!(result, "check for no non-trivial overlaps");

        let wcrt1 = Task::worst_case_response_time(&system, 1, 0, swh1);
        assert_eq!(wcrt1, 12.into());

        let wcrt2 = Task::worst_case_response_time(&system, 2, 0, swh2);
        assert_eq!(wcrt2, 6.into());
    }
}

mod util {
    use rta_for_fps::curve::curve_types::CurveType;
    use rta_for_fps::curve::Curve;
    use rta_for_fps::iterators::CurveIterator;

    /// # Panics
    /// When the Curve represents not the same Curve as the the CurveIterator
    pub fn assert_curve_eq<C: CurveType>(
        expected: &Curve<C>,
        result: impl CurveIterator<C::WindowKind, CurveKind = C> + Clone,
    ) {
        if !expected.eq_curve_iterator(result.clone()) {
            panic!(
                "\
            Curves did not match:\n\
            Expected:\n\
            {:#?}\n\
            \n\
            Got:\n\
            {:#?}\n\
            \n\
            ",
                expected,
                result.collect_curve::<Curve<_>>()
            )
        }
    }
}
