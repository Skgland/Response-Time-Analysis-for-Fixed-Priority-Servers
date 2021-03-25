use crate::curve::Curve;
use crate::task::Task;
use crate::time::TimeUnit;
use crate::window::Window;

#[test]
fn demand_curve() {
    // Example 5. with t = 18

    let t_2 = Task::new(1, 5, 0);
    let t_3 = Task::new(2, 8, 0);

    let up_to = TimeUnit::from(18);

    let c_2 = t_2.demand_curve(up_to);
    let c_3 = t_3.demand_curve(up_to);

    let expected_c_2 = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 1),
            Window::new(5, 6),
            Window::new(10, 11),
            Window::new(15, 16),
        ])
    };

    let expected_c_3 = unsafe {
        Curve::from_windows_unchecked(vec![
            Window::new(0, 2),
            Window::new(8, 10),
            Window::new(16, 18),
        ])
    };

    assert_eq!(c_2, expected_c_2);
    assert_eq!(c_3, expected_c_3);
}

#[test]
fn aggregated_demand_curve() {
    // Example 5. with t = 18

    let t_2 = Task::new(1, 5, 0);
    let t_3 = Task::new(2, 8, 0);

    let up_to = TimeUnit::from(18);

    let result = t_2.demand_curve(up_to).aggregate(t_3.demand_curve(up_to));

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
