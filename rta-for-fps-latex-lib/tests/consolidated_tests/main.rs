use rta_for_fps_latex_lib::DemandCurveDataPoints;
use rta_for_fps_lib::curve::AggregateExt;
use rta_for_fps_lib::iterators::{CurveIterator, ReclassifyIterator};
use rta_for_fps_lib::task::curve_types::TaskDemand;
use rta_for_fps_lib::task::Task;
use rta_for_fps_lib::time::TimeUnit;

#[test]
fn figure_4_t1() {
    let t_1 = Task::new(1, 5, 0);
    let t_1_curve = t_1
        .into_iter()
        .take_while(|window| window.end <= TimeUnit::from(50));

    let graph_data = DemandCurveDataPoints::new(t_1_curve.collect_curve()).to_string();

    assert_eq!(
        graph_data,
        "\
    x,y\n\
    0,0\n\
    1,1\n\
    5,1\n\
    6,2\n\
    10,2\n\
    11,3\n\
    15,3\n\
    16,4\n\
    20,4\n\
    21,5\n\
    25,5\n\
    26,6\n\
    30,6\n\
    31,7\n\
    35,7\n\
    36,8\n\
    40,8\n\
    41,9\n\
    45,9\n\
    46,10\n\
    "
    )
}

#[test]
fn figure_4_t2() {
    let t_2 = Task::new(2, 8, 0);
    let t_2_curve = t_2
        .into_iter()
        .take_while(|window| window.end <= TimeUnit::from(50));

    let graph_data = DemandCurveDataPoints::new(t_2_curve.collect_curve()).to_string();

    assert_eq!(
        graph_data,
        "\
    x,y\n\
    0,0\n\
    2,2\n\
    8,2\n\
    10,4\n\
    16,4\n\
    18,6\n\
    24,6\n\
    26,8\n\
    32,8\n\
    34,10\n\
    40,10\n\
    42,12\n\
    48,12\n\
    50,14\n\
    "
    )
}

#[test]
fn figure_4_aggregate() {
    let t_1 = Task::new(1, 5, 0);
    let t_2 = Task::new(2, 8, 0);

    let aggregated_curve = [t_1, t_2]
        .iter()
        .map(|task| {
            task.into_iter()
                .take_while(|window| window.end <= TimeUnit::from(50))
        })
        .aggregate::<ReclassifyIterator<_, TaskDemand>>();
    let graph_data = DemandCurveDataPoints::new(aggregated_curve.collect_curve()).to_string();
    assert_eq!(
        graph_data,
        "\
    x,y\n\
    0,0\n\
    3,3\n\
    5,3\n\
    6,4\n\
    8,4\n\
    11,7\n\
    15,7\n\
    18,10\n\
    20,10\n\
    21,11\n\
    24,11\n\
    27,14\n\
    30,14\n\
    31,15\n\
    32,15\n\
    34,17\n\
    35,17\n\
    36,18\n\
    40,18\n\
    43,21\n\
    45,21\n\
    46,22\n\
    48,22\n\
    50,24\n\
    "
    )
}
