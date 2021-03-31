mod broken_assumption;
mod curve_tests;
mod server_tests;
mod system_tests;
mod task_tests;
mod window_tests;
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
            Curves did not match:\
            Expected:\
            {:#?}\
            
            Got:\
            {:#?}\
            \
            ",
                expected,
                result.collect_curve::<Curve<_>>()
            )
        }
    }
}
