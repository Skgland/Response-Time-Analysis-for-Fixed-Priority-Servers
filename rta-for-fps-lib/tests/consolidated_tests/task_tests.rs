use crate::rta_lib::curve::Curve;
use crate::rta_lib::iterators::curve::AggregationIterator;
use crate::rta_lib::iterators::CurveIterator;
use crate::rta_lib::task::curve_types::TaskDemand;
use crate::rta_lib::task::Task;
use crate::rta_lib::time::TimeUnit;
use crate::rta_lib::window::Window;

#[test]
fn demand_curve() {
    // Example 5. with t = 18

    let t_2 = Task::new(1, 5, 0);
    let t_3 = Task::new(2, 8, 0);

    let up_to = TimeUnit::from(18);

    let c_2 = t_2.into_iter().take_while(|window| window.end <= up_to);

    let expected_c_2 = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 1),
            Window::new(5, 6),
            Window::new(10, 11),
            Window::new(15, 16),
        ])
    };

    crate::util::assert_curve_eq(&expected_c_2, c_2);

    let c_3 = t_3.into_iter().take_while(|window| window.end <= up_to);

    let expected_c_3 = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 2),
            Window::new(8, 10),
            Window::new(16, 18),
        ])
    };

    crate::util::assert_curve_eq(&expected_c_3, c_3);
}

#[test]
fn aggregated_demand_curve() {
    // Example 5. with t = 18

    let t_2 = Task::new(1, 5, 0);
    let t_3 = Task::new(2, 8, 0);

    let up_to = TimeUnit::from(18);

    let f = |window: &Window<_>| window.end <= up_to;

    let t2_demand = t_2.into_iter().take_while(f);

    let t3_demand = t_3.into_iter().take_while(f);

    let result: Curve<TaskDemand> =
        AggregationIterator::new(vec![t2_demand, t3_demand]).collect_curve();

    let expected_result = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 3),
            Window::new(5, 6),
            Window::new(8, 11),
            Window::new(15, 18),
        ])
    };

    assert_eq!(result, expected_result);
}
